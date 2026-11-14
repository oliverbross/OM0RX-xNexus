#!/usr/bin/env python3
"""Prepare/execute source-pinned Linux checks. Default is a read-only plan."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import time

TOOLS = Path(__file__).resolve().parent


def capture(args):
    return subprocess.check_output(args, text=True).strip()


def run_args(image, phase, job, volume):
    if not re.fullmatch(r"sha256:[0-9a-f]{64}", image):
        raise ValueError("Use the immutable image ID from the image phase receipt")
    if phase not in ("fetch", "contract", "desktop"):
        raise ValueError("Unknown validation phase")
    return ["docker", "run", "--rm", "--platform", "linux/amd64", "--pull", "never",
            "--cpus", "2", "--memory", "6g", "--memory-swap", "6g", "--pids-limit", "256",
            "--cap-drop", "ALL", "--security-opt", "no-new-privileges",
            "--network", "bridge" if phase == "fetch" else "none",
            "--mount", f"type=volume,src={volume},dst=/work",
            "--mount", f"type=bind,src={job / 'input'},dst=/input,readonly",
            "--mount", f"type=bind,src={job / 'evidence'},dst=/evidence",
            "--mount", f"type=bind,src={TOOLS},dst=/tools,readonly",
            image, "bash", "/tools/validate.sh", phase]


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
        receipt["archive_sha256"] = hashlib.sha256(archive.read_bytes()).hexdigest()
        receipt["image"] = image
        receipt["image_inspect"] = json.loads(capture(["docker", "image", "inspect", image]))
        cmd = run_args(image, args.phase, job, volume)
    receipt["command"] = cmd
    receipt_path = job / "evidence" / f"{stamp}-{args.phase}.json"
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n")
    with (job / "evidence" / f"{stamp}-{args.phase}.log").open("w", encoding="utf-8") as log:
        process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
        for line in process.stdout:
            print(line, end="", flush=True)
            log.write(line)
            log.flush()
        receipt["exit_code"] = process.wait()
    if args.phase == "image" and receipt["exit_code"] == 0:
        receipt["image"] = (job / "image.id").read_text().strip()
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt["exit_code"]


if __name__ == "__main__":
    sys.exit(main())
