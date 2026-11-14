"""Tiny disposable files only: no container, compiler or retained cache writes."""
import copy
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest import mock

import seed_cache as seed


class CacheSeedTests(unittest.TestCase):
    def fixture(self, root):
        donor, work = root / "donor", root / "new"
        donor.mkdir()
        work.mkdir()
        (donor / "source").mkdir()
        old, new = root / "old.tar", root / "new.tar"
        files = {p: b"same dependency input\n" for p in seed.LOCKS + seed.MANIFESTS}
        files["crates/example/Cargo.toml"] = b"same path dependency\n"
        for archive, content in ((old, b"old source\n"), (new, b"new source\n")):
            with tarfile.open(archive, "w") as tar:
                for name, value in {**files, "src-tauri/src/lib.rs": content}.items():
                    entry = tarfile.TarInfo(name)
                    entry.size = len(value)
                    tar.addfile(entry, io.BytesIO(value))
                    if archive == old:
                        path = donor / "source" / name
                        path.parent.mkdir(parents=True, exist_ok=True)
                        path.write_bytes(value)
        (donor / "archive.sha256").write_text(seed.file_hash(old))
        for relative in seed.CACHES:
            path = donor / relative
            path.mkdir(parents=True, exist_ok=True)
            (path / "cached.bin").write_bytes(b"retained dependency artifact")
        (donor / "source/ui/dist").mkdir(parents=True)
        (donor / "source/ui/dist/old.js").write_bytes(b"must not copy")
        (donor / "home/credentials").write_bytes(b"must not copy")
        image = "sha256:" + "a" * 64
        source, old_source = "1" * 40, "2" * 40
        receipt = {"source": old_source, "phase": "desktop", "result": "passed",
                   "process_exit_code": 0, "exit_code": 0, "image": image,
                   "cache_volume": seed.volume_name(old_source, image), "archive_sha256": seed.file_hash(old)}
        plan = seed.prepare_plan(new, old, receipt, source, image, seed.volume_name(source, image))
        return donor, work, new, old, receipt, plan

    def test_only_reviewed_cache_paths_copied_and_donor_unchanged(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            donor, work, new, old, _, plan = self.fixture(root)
            before = seed.tree_manifest(donor)
            seed.seed(work, donor, new, old, plan, root / "evidence")
            self.assertEqual(seed.tree_manifest(donor), before)
            for relative in seed.CACHES:
                self.assertEqual(seed.tree_manifest(donor / relative), seed.tree_manifest(work / relative))
            for relative in ("source", "archive.sha256", "home/credentials", "seed.pending.json"):
                self.assertFalse((work / relative).exists())
            self.assertTrue((work / "seed.json").is_file())
            self.assertEqual(len(plan["dependency_sha256"]), 9)

    def test_nonempty_or_same_destination_refused_without_changes(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            donor, work, new, old, _, plan = self.fixture(root)
            (work / "unique").write_bytes(b"preserve")
            for target in (work, donor):
                before = seed.tree_manifest(target)
                with self.assertRaisesRegex(ValueError, "fresh empty"):
                    seed.seed(target, donor, new, old, plan, root / "evidence")
                self.assertEqual(seed.tree_manifest(target), before)

    def test_mismatched_receipt_archive_image_or_manifest_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            _, _, new, old, receipt, plan = self.fixture(root)
            for changes in ({"image": "sha256:" + "b" * 64}, {"exit_code": 1},
                            {"memory_guard_error": "unproven"}, {"archive_sha256": "bad"},
                            {"cache_volume": "foreign"}, {"result": "failed"}):
                with self.assertRaises(ValueError):
                    seed.prepare_plan(new, old, {**receipt, **changes}, plan["source"], plan["image"], plan["volume"])
            with tarfile.open(new, "a") as archive:
                item = tarfile.TarInfo("Cargo.lock")
                item.size = 7
                archive.addfile(item, io.BytesIO(b"changed"))
            with self.assertRaisesRegex(ValueError, "Dependency"):
                seed.prepare_plan(new, old, receipt, plan["source"], plan["image"], plan["volume"])

    def test_tracked_donor_mutation_or_wrong_volume_archive_refused(self):
        for kind in ("source", "archive"):
            with tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                donor, work, new, old, _, plan = self.fixture(root)
                path = donor / ("source/src-tauri/src/lib.rs" if kind == "source" else "archive.sha256")
                path.write_text("changed")
                with self.assertRaises(ValueError):
                    seed.seed(work, donor, new, old, plan, root / "evidence")
                self.assertEqual(list(work.iterdir()), [])

    def test_changed_copy_is_detected_and_partial_evidence_preserved(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            donor, work, new, old, _, plan = self.fixture(root)
            original = seed.shutil.copytree
            def corrupt(*args, **kwargs):
                result = original(*args, **kwargs)
                (Path(args[1]) / "cached.bin").write_text("corrupted")
                return result
            with mock.patch.object(seed.shutil, "copytree", side_effect=corrupt):
                with self.assertRaisesRegex(ValueError, "integrity mismatch"):
                    seed.seed(work, donor, new, old, plan, root / "evidence")
            self.assertTrue((work / "seed.pending.json").exists())
            self.assertFalse((work / "seed.json").exists())
            self.assertEqual((donor / "cargo/registry/cached.bin").read_bytes(), b"retained dependency artifact")

    def test_unreviewed_copy_path_and_missing_required_cache_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            donor, work, new, old, _, plan = self.fixture(root)
            bad = copy.deepcopy(plan)
            bad["cache_paths"].append("source")
            with self.assertRaisesRegex(ValueError, "Unreviewed"):
                seed.seed(work, donor, new, old, bad, root / "evidence")
            (donor / "target").rename(donor / "target-preserved")
            with self.assertRaisesRegex(ValueError, "missing"):
                seed.seed(work, donor, new, old, plan, root / "evidence")
            self.assertTrue((work / "seed.pending.json").exists())


class AppCompilationTests(unittest.TestCase):
    def test_nonfresh_real_library_required_with_successful_build(self):
        artifact = {"reason": "compiler-artifact", "fresh": False,
                    "target": {"src_path": "/work/source/src-tauri/src/lib.rs", "kind": ["staticlib", "cdylib", "rlib"]}}
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "cargo.jsonl"
            for changed in ({"fresh": True}, {"target": {"src_path": "/work/source/other/lib.rs", "kind": ["lib"]}},
                            {"target": {"src_path": "/work/source/src-tauri/src/lib.rs", "kind": ["bin"]}}):
                path.write_text(json.dumps({**artifact, **changed}) + '\n{"reason":"build-finished","success":true}\n')
                with self.assertRaisesRegex(ValueError, "non-fresh"):
                    seed.check_compiled(path)
            path.write_text(json.dumps(artifact) + '\n{"reason":"build-finished","success":false}\n')
            with self.assertRaises(ValueError):
                seed.check_compiled(path)
            path.write_text(json.dumps(artifact) + '\n{"reason":"build-finished","success":true}\n')
            result = seed.check_compiled(path)
            self.assertEqual(len(result["nonfresh_app_artifacts"]), 1)
            self.assertEqual(result["cargo_json_sha256"], seed.file_hash(path))


if __name__ == "__main__":
    unittest.main()
