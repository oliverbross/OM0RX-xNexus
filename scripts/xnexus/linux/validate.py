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


def resource_profile(name, phase):
    if name == "standard":
        return {"name": name, "memory": "6g", "jobs": 2,
                "admission_physical_bytes": 10 * 1024 ** 3,
                "admission_commit_bytes": None, "guard_commit": False}
    if ((name == "desktop-reduced" and phase == "desktop")
            or (name == "contract-reduced" and phase in ("contract", "seed"))):
        return {"name": name, "memory": "4g", "jobs": 1,
                "admission_physical_bytes": 8 * 1024 ** 3,
                "admission_commit_bytes": 6 * 1024 ** 3, "guard_commit": True}
    if ((name == "desktop-low-headroom" and phase == "desktop")
            or (name == "contract-low-headroom" and phase in ("contract", "seed"))):
        return {"name": name, "memory": "4g", "jobs": 1,
                "admission_physical_bytes": 7 * 1024 ** 3,
                "admission_commit_bytes": 8 * 1024 ** 3, "guard_commit": True}
    raise ValueError("Resource profile does not match its phase")


def check_admission(profile, physical, commit=None):
    if physical < profile["admission_physical_bytes"]:
        raise ValueError(f"Validation held: host has {physical / 1024**3:.2f} GiB physical free; "
                         f"require {profile['admission_physical_bytes'] / 1024**3:g} GiB")
    required = profile["admission_commit_bytes"]
    if required is not None and (commit is None or commit < required):
        value = "unknown" if commit is None else f"{commit / 1024**3:.2f} GiB"
        raise ValueError(f"Validation held: available Windows commit {value}; "
                         f"require {required / 1024**3:g} GiB")


def run_args(image, phase, job, volume, cidfile=None, profile_name="standard"):
    if not re.fullmatch(r"sha256:[0-9a-f]{64}", image):
        raise ValueError("Use the immutable image ID from the image phase receipt")
    if phase not in ("fetch", "contract", "desktop", "seed"):
        raise ValueError("Unknown validation phase")
    profile = resource_profile(profile_name, phase)
    profile_args = (["--env", f"XNEXUS_RESOURCE_PROFILE={profile_name}"]
                    if profile_name != "standard" else [])
    cid_args = ["--cidfile", str(cidfile)] if cidfile else []
    return ["docker", "run", "--rm", *cid_args, "--platform", "linux/amd64", "--pull", "never",
            "--cpus", "2", "--memory", profile["memory"], "--memory-swap", profile["memory"], "--pids-limit", "256", *profile_args,
            "--env", "NODE_OPTIONS=--max-old-space-size=" + ("2560" if profile_name in ("contract-reduced", "contract-low-headroom") else "4096"),
            "--cap-drop", "ALL", "--security-opt", "no-new-privileges",
            "--network", "bridge" if phase == "fetch" else "none",
            "--mount", f"type=volume,src={volume},dst=/work",
            "--mount", f"type=bind,src={job / 'input'},dst=/input,readonly",
            "--mount", f"type=bind,src={job / 'evidence'},dst=/evidence",
            "--mount", f"type=bind,src={TOOLS},dst=/tools,readonly",
            image, "bash", "/tools/validate.sh", phase]


def host_headroom():
    """Physical RAM and Windows system commit availability, not address space."""
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
        return {"physical": status.available_physical, "commit": status.available_pagefile}
    for line in Path("/proc/meminfo").read_text().splitlines():
        if line.startswith("MemAvailable:"):
            return {"physical": int(line.split()[1]) * 1024, "commit": None}
    raise OSError("No supported host memory headroom source")


def seed_mounts(command, image, donor_volume, destination_volume, donor_input):
    if (donor_volume == destination_volume
            or not re.fullmatch(r"xnexus-linux-[0-9a-f]{16}-[0-9a-f]{12}", donor_volume)):
        raise ValueError("Distinct exact donor cache identity required")
    result = list(command)
    index = result.index(image)
    result[index:index] = ["--mount", f"type=volume,src={donor_volume},dst=/donor,readonly",
                          "--mount", f"type=bind,src={donor_input},dst=/donor-input,readonly"]
    return result


def available_memory():
    return host_headroom()["physical"]


def available_commit():
    value = host_headroom()["commit"]
    if value is None:
        raise OSError("Windows available commit measurement is required")
    return value


def owns_container(details, cid, image, phase, volume):
    return (details.get("Id") == cid
            and details.get("Config", {}).get("Image") == image
            and details.get("Config", {}).get("Cmd") == ["bash", "/tools/validate.sh", phase]
            and any(m.get("Type") == "volume" and m.get("Name") == volume
                    and m.get("Destination") == "/work" for m in details.get("Mounts", [])))


def memory_guard(done, cidfile, image, phase, volume, receipt,
                 read_memory=available_memory, read_json=None, stop=None, read_commit=None):
    """Stop only our proven container below either enabled 2 GiB headroom floor."""
    read_json = read_json or (lambda cid: json.loads(capture(["docker", "inspect", cid]))[0])
    stop = stop or (lambda cid: subprocess.run(["docker", "stop", "--time", "10", cid],
                                             capture_output=True, text=True, timeout=30).returncode)
    while not done.is_set():
        try:
            free = read_memory()
            receipt["minimum_observed_host_free_bytes"] = min(
                free, receipt.get("minimum_observed_host_free_bytes", free))
            commit = read_commit() if read_commit else None
            if commit is not None:
                receipt["minimum_observed_host_commit_bytes"] = min(
                    commit, receipt.get("minimum_observed_host_commit_bytes", commit))
            reason = ("host_memory_below_2_gib" if free < 2 * 1024 ** 3 else
                      "host_commit_below_2_gib" if commit is not None and commit < 2 * 1024 ** 3 else None)
            if reason and cidfile.exists():
                cid = cidfile.read_text().strip()
                if not re.fullmatch(r"[0-9a-f]{64}", cid):
                    raise ValueError("Invalid owned-container receipt")
                if not owns_container(read_json(cid), cid, image, phase, volume):
                    raise ValueError("Container identity mismatch; refused to stop it")
                receipt["interruption"] = {"reason": reason, "available_bytes": free,
                                           "available_commit_bytes": commit, "container_id": cid}
                print(f"Stopping owned Linux validation container: {reason}.", flush=True)
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
    parser.add_argument("--phase", choices=("image", "fetch", "contract", "desktop", "seed"), default="contract")
    parser.add_argument("--resource-profile", choices=("standard", "desktop-reduced", "contract-reduced", "desktop-low-headroom", "contract-low-headroom"), default="standard")
    parser.add_argument("--donor-job", type=Path, help="Retained successful donor job; seed only")
    parser.add_argument("--donor-receipt", type=Path, help="Exact successful desktop receipt; seed only")
    parser.add_argument("--execute", action="store_true", help="Without this flag, only print the plan")
    args = parser.parse_args()
    repo, job = args.repo.resolve(), args.job.resolve()
    try:
        profile = resource_profile(args.resource_profile, args.phase)
    except ValueError as error:
        parser.error(str(error))
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
            "job": str(job), "cache_volume": volume, "cpus": 2, "memory": profile["memory"],
            "resource_profile": profile,
            "network": "downloads only; contract/desktop none",
            "scope": "Linux source validation, no package/install/device/GUI acceptance"}
    print(json.dumps(plan, indent=2), flush=True)
    if not args.execute:
        return 0
    if args.phase == "seed" and (not args.donor_job or not args.donor_receipt or args.resource_profile not in ("contract-reduced", "contract-low-headroom")):
        parser.error("Seed requires donor job, desktop receipt and explicit bounded contract profile")
    if args.phase != "seed" and (args.donor_job or args.donor_receipt):
        parser.error("Donor arguments are only valid for seed")
    headroom = None
    if args.phase in ("contract", "desktop", "seed"):
        headroom = host_headroom()
        try:
            check_admission(profile, headroom["physical"], headroom["commit"])
        except ValueError as error:
            parser.error(str(error))
    job.mkdir(parents=True, exist_ok=True)
    (job / "input").mkdir(exist_ok=True)
    (job / "evidence").mkdir(exist_ok=True)
    stamp = str(time.time_ns())
    receipt = {**plan, "admission_headroom": headroom, "tool_hashes": {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                                     for p in TOOLS.iterdir() if p.is_file()}}
    receipt["harness_commit"] = capture(["git", "-C", str(TOOLS), "rev-parse", "HEAD"])
    receipt["harness_worktree_dirty"] = bool(capture(["git", "-C", str(TOOLS), "status", "--porcelain"]))
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
        cmd = run_args(image, args.phase, job, volume, cidfile, args.resource_profile)
        if args.phase == "seed":
            from seed_cache import prepare_plan
            donor_job = args.donor_job.resolve()
            donor_receipt = args.donor_receipt.resolve()
            donor = json.loads(donor_receipt.read_text())
            donor_archive = donor_job / "input" / "source.tar"
            if (donor_job / "input" / "source.sha").read_text().strip() != donor["source"]:
                raise ValueError("Donor job source label differs from receipt")
            verify_archive(repo, donor["source"], donor_archive)
            seed = prepare_plan(archive, donor_archive, donor, args.source, image, volume)
            receipt["donor_volume_inspect"] = json.loads(capture(["docker", "volume", "inspect", seed["donor_volume"]]))
            seed["donor_receipt_sha256"] = hashlib.sha256(donor_receipt.read_bytes()).hexdigest()
            seed["donor_archive_path"] = str(donor_archive)
            plan_path = job / "input" / "seed-plan.json"
            if plan_path.exists():
                raise ValueError("Seed plan already exists; preserve partial evidence and choose a fresh job")
            plan_path.write_text(json.dumps(seed, indent=2) + "\n")
            receipt["seed_plan"] = seed
            cmd = seed_mounts(cmd, image, seed["donor_volume"], volume, donor_job / "input")
    receipt["command"] = cmd
    receipt_path = job / "evidence" / f"{stamp}-{args.phase}.json"
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n")
    with (job / "evidence" / f"{stamp}-{args.phase}.log").open("w", encoding="utf-8") as log:
        process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
        done = threading.Event()
        guard = None
        if args.phase in ("contract", "desktop", "seed"):
            guard = threading.Thread(target=memory_guard,
                args=(done, cidfile, image, args.phase, volume, receipt),
                kwargs={"read_commit": available_commit if profile["guard_commit"] else None}, daemon=True)
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
    if args.phase == "seed":
        try:
            verify_archive(repo, donor["source"], donor_archive)
            if hashlib.sha256(donor_receipt.read_bytes()).hexdigest() != seed["donor_receipt_sha256"]:
                raise ValueError("Donor receipt changed during seed")
        except Exception as error:
            receipt["donor_integrity_error"] = str(error)
            receipt["exit_code"], receipt["result"] = 3, "donor_integrity_failed"
    if args.phase == "image" and receipt["exit_code"] == 0:
        receipt["image"] = (job / "image.id").read_text().strip()
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt["exit_code"]


if __name__ == "__main__":
    sys.exit(main())
