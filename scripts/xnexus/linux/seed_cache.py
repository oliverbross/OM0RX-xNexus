#!/usr/bin/env python3
"""Copy explicit caches into a fresh volume; never relabel old source as new proof."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import tarfile

from verify_source import verify

LOCKS = ("Cargo.lock", "src-tauri/Cargo.lock", "ui/package-lock.json")
MANIFESTS = ("Cargo.toml", "src-tauri/Cargo.toml", "ui/package.json")
CACHES = ("cargo/registry", "cargo/git", "target", "home/.npm/_cacache")


def file_hash(path):
    result = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            result.update(chunk)
    return result.hexdigest()


def dependency_hashes(archive):
    with tarfile.open(archive) as tar:
        members = {m.name: m for m in tar.getmembers()}
        names = set(LOCKS + MANIFESTS)
        names.update(n for n in members if n.endswith("/Cargo.toml") and
                     n.startswith(("crates/", "integrations/")))
        result = {}
        for name in sorted(names):
            item = members.get(name)
            if item is None or not item.isfile():
                raise ValueError(f"Missing regular dependency input: {name}")
            result[name] = hashlib.sha256(tar.extractfile(item).read()).hexdigest()
        return result


def volume_name(source, image):
    if not re.fullmatch(r"[0-9a-f]{40}", source) or not re.fullmatch(r"sha256:[0-9a-f]{64}", image):
        raise ValueError("Exact source and immutable image required")
    return "xnexus-linux-" + source[:16] + "-" + image.split(":")[1][:12]


def prepare_plan(archive, donor_archive, receipt, source, image, volume):
    if volume != volume_name(source, image):
        raise ValueError("Destination volume identity mismatch")
    donor_source = receipt["source"]
    donor_volume = volume_name(donor_source, image)
    if source == donor_source or volume == donor_volume:
        raise ValueError("New source requires an independent destination volume")
    if (receipt.get("phase") != "desktop" or receipt.get("result") != "passed"
            or receipt.get("exit_code") != 0 or receipt.get("process_exit_code") != 0
            or receipt.get("image") != image or receipt.get("cache_volume") != donor_volume
            or any(k in receipt for k in ("memory_guard_error", "interruption", "donor_integrity_error"))):
        raise ValueError("Donor is not an exact successful guarded desktop receipt")
    donor_hash = file_hash(donor_archive)
    if donor_hash != receipt.get("archive_sha256"):
        raise ValueError("Donor archive differs from qualified receipt")
    inputs = dependency_hashes(archive)
    donor_inputs = dependency_hashes(donor_archive)
    if inputs != donor_inputs:
        raise ValueError("Dependency lockfiles/manifests differ; seed refused")
    return {"source": source, "image": image, "volume": volume,
            "archive_sha256": file_hash(archive), "donor_source": donor_source,
            "donor_volume": donor_volume, "donor_archive_sha256": donor_hash,
            "dependency_sha256": inputs, "donor_dependency_sha256": donor_inputs,
            "cache_paths": list(CACHES), "scope": "cache reuse only; no inherited source qualification"}


def tree_manifest(root):
    """Hash bytes/link targets/modes without following links or allocating whole files."""
    root = Path(root)
    if not root.is_dir() or root.is_symlink():
        raise ValueError(f"Cache root is not a real directory: {root}")
    rows = []
    def visit(path):
        mode = path.lstat().st_mode
        row = {"path": path.relative_to(root).as_posix(), "mode": stat.S_IMODE(mode)}
        if stat.S_ISLNK(mode):
            row.update(kind="symlink", target=os.readlink(path))
        elif stat.S_ISREG(mode):
            row.update(kind="file", sha256=file_hash(path), bytes=path.stat().st_size)
        elif stat.S_ISDIR(mode):
            row["kind"] = "directory"
        else:
            raise ValueError(f"Unsupported special cache entry: {path}")
        rows.append(row)
        if row["kind"] == "directory":
            for child in sorted(path.iterdir()):
                visit(child)
    visit(root)
    return rows


def real_parents(root, relative):
    path = root
    for part in Path(relative).parts:
        path /= part
        if path.is_symlink():
            raise ValueError(f"Cache path has a symlink ancestor: {path}")


def seed(work, donor, archive, donor_archive, plan, evidence):
    work, donor, evidence = Path(work), Path(donor), Path(evidence)
    if work.resolve() == donor.resolve() or not work.is_dir() or work.is_symlink() or any(work.iterdir()):
        raise ValueError("Seed destination must be a fresh empty independent volume")
    if plan["cache_paths"] != list(CACHES):
        raise ValueError("Unreviewed cache path selection")
    if file_hash(archive) != plan["archive_sha256"] or file_hash(donor_archive) != plan["donor_archive_sha256"]:
        raise ValueError("Source archive changed before seed")
    if (donor / "archive.sha256").read_text().strip() != plan["donor_archive_sha256"]:
        raise ValueError("Donor volume belongs to another archive")
    if dependency_hashes(archive) != plan["dependency_sha256"] or dependency_hashes(donor_archive) != plan["donor_dependency_sha256"]:
        raise ValueError("Dependency provenance changed before seed")
    verify(donor_archive, donor / "source")
    # A partial copy cannot be silently reused or overwritten on retry.
    (work / "seed.pending.json").write_text(json.dumps(plan, indent=2) + "\n")
    evidence.mkdir(parents=True, exist_ok=True)
    report = {"plan": plan, "caches": {}, "status": "copying"}
    for relative in CACHES:
        real_parents(donor, relative)
        origin, destination = donor / relative, work / relative
        if not origin.exists():
            if relative == "cargo/git":
                report["caches"][relative] = {"absent": True}
                continue
            raise ValueError(f"Required donor cache missing: {relative}")
        before = tree_manifest(origin)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(origin, destination, symlinks=True, copy_function=shutil.copy2)
        after, copied = tree_manifest(origin), tree_manifest(destination)
        if before != after or before != copied:
            raise ValueError(f"Cache copy/donor integrity mismatch: {relative}")
        record = relative.replace("/", "-") + ".manifest.json"
        (evidence / record).write_text(json.dumps(before, indent=2) + "\n")
        report["caches"][relative] = {"entries": len(before), "manifest": record,
                                     "sha256": file_hash(evidence / record)}
    verify(donor_archive, donor / "source")
    for relative, copied in report["caches"].items():
        if copied.get("absent"):
            if (donor / relative).exists():
                raise ValueError("Donor cache appeared during seed")
        elif tree_manifest(donor / relative) != json.loads((evidence / copied["manifest"]).read_text()):
            raise ValueError(f"Donor cache changed after copying: {relative}")
    if file_hash(archive) != plan["archive_sha256"] or file_hash(donor_archive) != plan["donor_archive_sha256"]:
        raise ValueError("Source archive changed during seed")
    report["status"] = "copied-and-verified-not-qualified"
    (evidence / "seed-copy.json").write_text(json.dumps(report, indent=2) + "\n")
    (work / "seed.json").write_text(json.dumps(plan, indent=2) + "\n")
    (work / "seed.pending.json").unlink()


def check_compiled(path):
    artifacts = []
    finished = False
    for line in Path(path).read_text().splitlines():
        item = json.loads(line)
        if item.get("reason") == "build-finished":
            finished = item.get("success") is True
        target = item.get("target", {})
        source = target.get("src_path", "").replace("\\", "/")
        if (item.get("reason") == "compiler-artifact" and source == "/work/source/src-tauri/src/lib.rs"
                and item.get("fresh") is False
                and set(target.get("kind", [])) & {"lib", "rlib", "staticlib", "cdylib"}):
            artifacts.append(item)
    if not finished or not artifacts:
        raise ValueError("No successful non-fresh actual src-tauri/src/lib.rs compilation; do not claim updated desktop validation")
    result = {"scope": "actual app library rechecked from current source; not runtime qualification",
              "cargo_json_sha256": file_hash(path), "nonfresh_app_artifacts": artifacts}
    Path(path).with_suffix(".verified.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check-compiled", type=Path)
    args = parser.parse_args()
    if args.check_compiled:
        check_compiled(args.check_compiled)
    else:
        seed(Path("/work"), Path("/donor"), Path("/input/source.tar"), Path("/donor-input/source.tar"),
             json.loads(Path("/input/seed-plan.json").read_text()), Path("/evidence/seed"))
