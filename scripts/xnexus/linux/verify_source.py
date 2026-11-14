#!/usr/bin/env python3
"""Verify archive-tracked source without changing cached files or build outputs."""
import argparse
import os
from pathlib import Path, PurePosixPath
import stat
import tarfile


def verify(archive, root):
    root = Path(root)
    if not stat.S_ISDIR(root.lstat().st_mode):
        raise ValueError("Source root is not a real directory")
    count = 0
    with tarfile.open(archive, mode="r|*") as entries:
        for entry in entries:
            relative = PurePosixPath(entry.name)
            if relative.is_absolute() or ".." in relative.parts or not relative.parts:
                raise ValueError(f"Unsafe archive path: {entry.name}")
            path = root.joinpath(*relative.parts)
            for parent in list(path.parents)[:len(relative.parts) - 1]:
                if not stat.S_ISDIR(parent.lstat().st_mode):
                    raise ValueError(f"Tracked parent changed: {entry.name}")
            info = path.lstat()
            if entry.isdir():
                valid = stat.S_ISDIR(info.st_mode)
            elif entry.issym():
                valid = stat.S_ISLNK(info.st_mode) and os.readlink(path) == entry.linkname
            elif entry.isfile():
                valid = stat.S_ISREG(info.st_mode) and info.st_size == entry.size
                if valid:
                    with entries.extractfile(entry) as expected, path.open("rb") as actual:
                        while True:
                            block = expected.read(1024 * 1024)
                            if actual.read(len(block)) != block:
                                valid = False
                                break
                            if not block:
                                break
                if valid and os.name != "nt":
                    valid = (info.st_mode & 0o111) == (entry.mode & 0o111)
            else:
                raise ValueError(f"Unsupported tracked entry type: {entry.name}")
            if not valid:
                raise ValueError(f"Tracked source changed: {entry.name}; preserved without overwrite")
            count += 1
    if count == 0:
        raise ValueError("Empty source archive cannot establish provenance")
    return count


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path)
    parser.add_argument("root", type=Path)
    args = parser.parse_args()
    print(f"Verified {verify(args.archive, args.root)} archive-tracked entries")
