#!/usr/bin/env python3
"""Prepare/execute source-pinned Linux checks. Default is a read-only plan."""
import argparse
import ctypes
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import threading
import time

TOOLS = Path(__file__).resolve().parent


def capture(args):
    return subprocess.check_output(args, text=True).strip()


def stream_hash(stream):
    digest = hashlib.sha256()
    for block in iter(lambda: stream.read(1024 * 1024), b""):
        digest.update(block)
    return digest.hexdigest()


def verify_archive(repo, source, archive):
    """Compare actual retained bytes with a freshly streamed committed archive."""
    with archive.open("rb") as stream:
        actual = stream_hash(stream)
    with subprocess.Popen(["git", "-C", str(repo), "archive", "--format=tar", source],
                          stdout=subprocess.PIPE) as process:
        expected = stream_hash(process.stdout)
        if process.wait() != 0:
            raise ValueError("Cannot verify committed source archive")
    if actual != expected:
        raise ValueError("Source archive bytes differ from exact git commit; preserved without overwrite")
    return actual


def validation_result(receipt):
    """A zero command exit alone cannot establish guarded validation."""
    process_exit = receipt["process_exit_code"]
    if "interruption" in receipt or "memory_guard_error" in receipt:
        receipt["result"] = "interrupted" if "interruption" in receipt else "guard_failed"
        return process_exit if process_exit != 0 else 3
    receipt["result"] = "passed" if process_exit == 0 else "failed"
    return process_exit


def run_args(image, phase, job, volume, cidfile=None):
    if not re.fullmatch(r"sha256:[0-9a-f]{64}", image):
        raise ValueError("Use the immutable image ID from the image phase receipt")
    if phase not in ("fetch", "contract", "desktop"):
        raise ValueError("Unknown validation phase")
    cid_args = ["--cidfile", str(cidfile)] if cidfile else []
    return ["docker", "run", "--rm", *cid_args, "--platform", "linux/amd64", "--pull", "never",
            "--cpus", "2", "--memory", "6g", "--memory-swap", "6g", "--pids-limit", "256",
            "--env", "NODE_OPTIONS=--max-old-space-size=4096",
            "--cap-drop", "ALL", "--security-opt", "no-new-privileges",
            "--network", "bridge" if phase == "fetch" else "none",
            "--mount", f"type=volume,src={volume},dst=/work",
            "--mount", f"type=bind,src={job / 'input'},dst=/input,readonly",
            "--mount", f"type=bind,src={job / 'evidence'},dst=/evidence",
            "--mount", f"type=bind,src={TOOLS},dst=/tools,readonly",
            image, "bash", "/tools/validate.sh", phase]


def available_memory():
    """Physical host headroom, not the Docker container's memory limit."""
    if sys.platform == "win32":
        class MemoryStatus(ctypes.Structure):
            _fields_ = [("length", ctypes.c_uint32), ("load", ctypes.c_uint32)] + [
                (name, ctypes.c_uint64) for name in (
                    "total_physical", "available_physical", "total_pagefile",
                    "available_pagefile", "total_virtual", "available_virtual", "extended")]
        status = MemoryStatus()
        status.length = ctypes.sizeof(status)
        if not ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(status)):
            raise OSError("Cannot read host memory headroom")
        return status.available_physical
    for line in Path("/proc/meminfo").read_text().splitlines():
        if line.startswith("MemAvailable:"):
            return int(line.split()[1]) * 1024
    raise OSError("No supported host memory headroom source")


def owns_container(details, cid, image, phase, volume):
    return (details.get("Id") == cid
            and details.get("Config", {}).get("Image") == image
            and details.get("Config", {}).get("Cmd") == ["bash", "/tools/validate.sh", phase]
            and any(m.get("Type") == "volume" and m.get("Name") == volume
                    and m.get("Destination") == "/work" for m in details.get("Mounts", [])))


def memory_guard(done, cidfile, image, phase, volume, receipt,
                 read_memory=available_memory, read_json=None, stop=None):
    """Stop only our proven container if host available RAM falls below 2 GiB."""
    read_json = read_json or (lambda cid: json.loads(capture(["docker", "inspect", cid]))[0])
    stop = stop or (lambda cid: subprocess.run(["docker", "stop", "--time", "10", cid],
                                             capture_output=True, text=True, timeout=30).returncode)
    while not done.is_set():
        try:
            free = read_memory()
            receipt["minimum_observed_host_free_bytes"] = min(
                free, receipt.get("minimum_observed_host_free_bytes", free))
            if free < 2 * 1024 ** 3 and cidfile.exists():
                cid = cidfile.read_text().strip()
                if not re.fullmatch(r"[0-9a-f]{64}", cid):
                    raise ValueError("Invalid owned-container receipt")
                if not owns_container(read_json(cid), cid, image, phase, volume):
                    raise ValueError("Container identity mismatch; refused to stop it")
                receipt["interruption"] = {"reason": "host_memory_below_2_gib",
                                           "available_bytes": free, "container_id": cid}
                print("Stopping owned Linux validation container: host RAM below 2 GiB.", flush=True)
                receipt["interruption"]["stop_exit_code"] = stop(cid)
                return
        except Exception as error:
            receipt["memory_guard_error"] = str(error)
            print(f"Host memory guard: {error}", flush=True)
            return
        done.wait(10)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--source", required=True, help="Exact 40-character committed source SHA")
    parser.add_argument("--job", type=Path, required=True, help="Dedicated evidence directory outside source")
    parser.add_argument("--phase", choices=("image", "fetch", "contract", "desktop"), default="contract")
    parser.add_argument("--execute", action="store_true", help="Without this flag, only print the plan")
    args = parser.parse_args()
    repo, job = args.repo.resolve(), args.job.resolve()
    if not re.fullmatch(r"[0-9a-f]{40}", args.source):
        parser.error("--source must be a full lowercase commit SHA")
    resolved = capture(["git", "-C", str(repo), "rev-parse", f"{args.source}^{{commit}}"])
    if resolved != args.source:
        parser.error("source identity mismatch")
    remote = capture(["git", "-C", str(repo), "remote", "get-url", "origin"])
    if remote not in ("https://github.com/oliverbross/OM0RX-xNexus.git", "git@github.com:oliverbross/OM0RX-xNexus.git"):
        parser.error("Only the OM0RX-xNexus fork is accepted")
    volume = "xnexus-linux-" + args.source[:16]
    plan = {"source": args.source, "phase": args.phase, "execute": args.execute,
            "job": str(job), "cache_volume": volume, "cpus": 2, "memory": "6g",
            "network": "downloads only; contract/desktop none",
            "scope": "Linux source validation, no package/install/device/GUI acceptance"}
    print(json.dumps(plan, indent=2), flush=True)
    if not args.execute:
        return 0
    if args.phase in ("contract", "desktop"):
        headroom = available_memory()
        if headroom < 10 * 1024 ** 3:
            parser.error(f"Validation held: host has {headroom / 1024**3:.2f} GiB free; require 10 GiB before start")
    job.mkdir(parents=True, exist_ok=True)
    (job / "input").mkdir(exist_ok=True)
    (job / "evidence").mkdir(exist_ok=True)
    stamp = str(time.time_ns())
    receipt = {**plan, "tool_hashes": {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                                     for p in TOOLS.iterdir() if p.is_file()}}
    receipt["docker"] = capture(["docker", "version", "--format", "{{json .}}"])
    if args.phase == "image":
        image_file = job / "image.id"
        cmd = ["docker", "build", "--platform", "linux/amd64", "--iidfile", str(image_file), str(TOOLS)]
    else:
        image = (job / "image.id").read_text().strip()
        if not re.fullmatch(r"sha256:[0-9a-f]{64}", image):
            raise ValueError("Image receipt lacks an immutable image ID")
        volume += "-" + image.split(":")[1][:12]
        receipt["cache_volume"] = volume
        archive = job / "input" / "source.tar"
        # git archive exports committed tracked bytes only, never local secrets or outputs.
        if not archive.exists():
            subprocess.run(["git", "-C", str(repo), "archive", "--format=tar", "--output", str(archive), args.source], check=True)
            (job / "input" / "source.sha").write_text(args.source + "\n")
        if (job / "input" / "source.sha").read_text().strip() != args.source:
            raise ValueError("Job directory belongs to another source; preserve it and choose a new directory")
        receipt["archive_sha256"] = verify_archive(repo, args.source, archive)
        receipt["image"] = image
        receipt["image_inspect"] = json.loads(capture(["docker", "image", "inspect", image]))
        cidfile = job / "evidence" / f"{stamp}-{args.phase}.cid"
        cmd = run_args(image, args.phase, job, volume, cidfile)
    receipt["command"] = cmd
    receipt_path = job / "evidence" / f"{stamp}-{args.phase}.json"
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n")
    with (job / "evidence" / f"{stamp}-{args.phase}.log").open("w", encoding="utf-8") as log:
        process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
        done = threading.Event()
        guard = None
        if args.phase in ("contract", "desktop"):
            guard = threading.Thread(target=memory_guard,
                args=(done, cidfile, image, args.phase, volume, receipt), daemon=True)
            guard.start()
        for line in process.stdout:
            print(line, end="", flush=True)
            log.write(line)
            log.flush()
        receipt["process_exit_code"] = process.wait()
        done.set()
        if guard:
            guard.join(timeout=35)
            if guard.is_alive():
                receipt["memory_guard_error"] = "Memory guard did not finish; protected validation is unproven"
    receipt["exit_code"] = validation_result(receipt)
    if args.phase == "image" and receipt["exit_code"] == 0:
        receipt["image"] = (job / "image.id").read_text().strip()
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt["exit_code"]


if __name__ == "__main__":
    sys.exit(main())
