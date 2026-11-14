"""Guard the isolation command before downloading/building an image."""
import importlib.util
import io
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import threading
import unittest

spec = importlib.util.spec_from_file_location("validate", Path(__file__).with_name("validate.py"))
validate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(validate)
source_spec = importlib.util.spec_from_file_location("verify_source", Path(__file__).with_name("verify_source.py"))
verify_source = importlib.util.module_from_spec(source_spec)
source_spec.loader.exec_module(verify_source)


class ContainerIsolationTests(unittest.TestCase):
    def args(self, phase="contract"):
        return validate.run_args("sha256:" + "a" * 64, phase, Path("C:/evidence/job"), "xnexus-test")

    def test_checks_have_no_external_network_or_devices(self):
        for phase in ("contract", "desktop"):
            args = self.args(phase)
            self.assertEqual(args[args.index("--network") + 1], "none")
            self.assertEqual(args[args.index("--cpus") + 1], "2")
            self.assertEqual(args[args.index("--memory") + 1], "6g")
            self.assertNotIn("--privileged", args)
            self.assertNotIn("--device", args)
            self.assertNotIn("--publish", args)
            self.assertNotIn("--network=host", args)
            self.assertIn("NODE_OPTIONS=--max-old-space-size=4096", args)

    def test_reduced_profile_is_desktop_only_with_bounded_resources(self):
        args = validate.run_args("sha256:" + "a" * 64, "desktop", Path("job"), "test",
                                 profile_name="desktop-reduced")
        for flag, value in (("--memory", "4g"), ("--memory-swap", "4g"),
                            ("--cpus", "2"), ("--network", "none")):
            self.assertEqual(args[args.index(flag) + 1], value)
        self.assertIn("XNEXUS_RESOURCE_PROFILE=desktop-reduced", args)
        self.assertEqual(validate.resource_profile("desktop-reduced", "desktop")["jobs"], 1)
        for phase in ("fetch", "contract", "image"):
            with self.assertRaises(ValueError):
                validate.resource_profile("desktop-reduced", phase)

    def test_reduced_admission_requires_both_physical_and_windows_commit(self):
        gib = 1024 ** 3
        profile = validate.resource_profile("desktop-reduced", "desktop")
        validate.check_admission(profile, 8 * gib, 6 * gib)
        for physical, commit in ((8 * gib - 1, 6 * gib), (8 * gib, 6 * gib - 1),
                                 (20 * gib, None)):
            with self.assertRaises(ValueError):
                validate.check_admission(profile, physical, commit)
        standard = validate.resource_profile("standard", "desktop")
        with self.assertRaises(ValueError):
            validate.check_admission(standard, 10 * gib - 1)
        validate.check_admission(standard, 10 * gib)

    def test_reduced_contract_and_seed_are_bounded_and_offline(self):
        for phase in ("contract", "seed"):
            args = validate.run_args("sha256:" + "a" * 64, phase, Path("job"), "new-volume",
                                     profile_name="contract-reduced")
            for flag, value in (("--memory", "4g"), ("--memory-swap", "4g"),
                                ("--cpus", "2"), ("--network", "none")):
                self.assertEqual(args[args.index(flag) + 1], value)
            self.assertIn("NODE_OPTIONS=--max-old-space-size=2560", args)
            profile = validate.resource_profile("contract-reduced", phase)
            self.assertEqual(profile["jobs"], 1)
            self.assertTrue(profile["guard_commit"])
            validate.check_admission(profile, 8 * 1024**3, 6 * 1024**3)
            for physical, commit in ((8 * 1024**3 - 1, 6 * 1024**3), (8 * 1024**3, None)):
                with self.assertRaises(ValueError):
                    validate.check_admission(profile, physical, commit)
        for phase in ("desktop", "fetch", "image"):
            with self.assertRaises(ValueError):
                validate.resource_profile("contract-reduced", phase)

    def test_seed_mounts_donor_readonly_and_never_overlays_new_source(self):
        image = "sha256:" + "a" * 64
        donor, new = "xnexus-linux-" + "b" * 16 + "-" + "a" * 12, "xnexus-linux-" + "c" * 16 + "-" + "a" * 12
        command = validate.run_args(image, "seed", Path("job"), new, profile_name="contract-reduced")
        args = validate.seed_mounts(command, image, donor, new, Path("donor/input"))
        self.assertIn(f"type=volume,src={donor},dst=/donor,readonly", args)
        self.assertIn(f"type=bind,src={Path('donor/input')},dst=/donor-input,readonly", args)
        self.assertIn(f"type=volume,src={new},dst=/work", args)
        self.assertEqual(args[args.index("--network") + 1], "none")
        for invalid in (new, "unrelated-volume"):
            with self.assertRaises(ValueError):
                validate.seed_mounts(command, image, invalid, new, Path("donor/input"))

    def test_commit_pressure_stops_only_owned_container_even_when_ram_is_ample(self):
        cid, image, volume = "b" * 64, "sha256:" + "a" * 64, "xnexus-test"
        details = {"Id": cid, "Config": {"Image": image,
                   "Cmd": ["bash", "/tools/validate.sh", "desktop"]},
                   "Mounts": [{"Type": "volume", "Name": volume, "Destination": "/work"}]}
        with tempfile.TemporaryDirectory() as root:
            cidfile = Path(root) / "owned.cid"
            cidfile.write_text(cid)
            for own in (True, False):
                stopped, receipt = [], {}
                actual = details if own else {**details, "Id": "c" * 64}
                validate.memory_guard(threading.Event(), cidfile, image, "desktop", volume, receipt,
                    read_memory=lambda: 12 * 1024 ** 3, read_commit=lambda: 2 * 1024 ** 3 - 1,
                    read_json=lambda _: actual, stop=lambda item: stopped.append(item) or 0)
                self.assertEqual(stopped, [cid] if own else [])
                self.assertEqual(receipt["minimum_observed_host_commit_bytes"], 2 * 1024 ** 3 - 1)
                if own:
                    self.assertEqual(receipt["interruption"]["reason"], "host_commit_below_2_gib")
                else:
                    self.assertIn("memory_guard_error", receipt)
                receipt["process_exit_code"] = 0
                self.assertNotEqual(validate.validation_result(receipt), 0)

    def test_commit_probe_failure_cannot_claim_protected_pass(self):
        def broken():
            raise OSError("commit unavailable")
        receipt = {"process_exit_code": 0}
        validate.memory_guard(threading.Event(), Path("unused"), "image", "desktop", "volume", receipt,
            read_memory=lambda: 12 * 1024 ** 3, read_commit=broken,
            stop=lambda _: self.fail("Unknown memory must not stop an unproven owner"))
        self.assertIn("memory_guard_error", receipt)
        self.assertNotEqual(validate.validation_result(receipt), 0)

    def test_only_fetch_phase_has_download_network(self):
        args = self.args("fetch")
        self.assertEqual(args[args.index("--network") + 1], "bridge")
        with self.assertRaises(ValueError):
            self.args("publish")

    def test_mutable_image_tags_are_rejected(self):
        with self.assertRaises(ValueError):
            validate.run_args("ubuntu:latest", "contract", Path("job"), "test")
        self.assertIn("sha256:" + "a" * 64, self.args())

    def test_source_and_tools_are_readonly_no_host_profile_mount(self):
        mounts = [self.args()[i + 1] for i, arg in enumerate(self.args()) if arg == "--mount"]
        self.assertEqual(len(mounts), 4)
        self.assertTrue(next(m for m in mounts if "dst=/input" in m).endswith(",readonly"))
        self.assertTrue(next(m for m in mounts if "dst=/tools" in m).endswith(",readonly"))
        self.assertFalse(any("docker.sock" in m or "dst=/root" in m for m in mounts))

    def test_low_memory_stops_only_the_verified_owned_container(self):
        cid, image, volume = "b" * 64, "sha256:" + "a" * 64, "xnexus-test"
        details = {"Id": cid, "Config": {"Image": image,
                   "Cmd": ["bash", "/tools/validate.sh", "contract"]},
                   "Mounts": [{"Type": "volume", "Name": volume, "Destination": "/work"}]}
        with tempfile.TemporaryDirectory() as root:
            cidfile = Path(root) / "owned.cid"
            cidfile.write_text(cid)
            stopped, receipt = [], {}
            validate.memory_guard(threading.Event(), cidfile, image, "contract", volume, receipt,
                read_memory=lambda: 1024 ** 3, read_json=lambda _: details,
                stop=lambda owned: stopped.append(owned) or 0)
            self.assertEqual(stopped, [cid])
            self.assertEqual(receipt["interruption"]["reason"], "host_memory_below_2_gib")
            # The same low-memory condition must refuse a different owner.
            for key in ("command", "image", "id", "volume"):
                wrong = {**details, "Config": dict(details["Config"]), "Mounts": list(details["Mounts"])}
                if key == "command": wrong["Config"]["Cmd"] = ["postgres"]
                if key == "image": wrong["Config"]["Image"] = "other"
                if key == "id": wrong["Id"] = "c" * 64
                if key == "volume": wrong["Mounts"] = []
                stopped, receipt = [], {}
                validate.memory_guard(threading.Event(), cidfile, image, "contract", volume, receipt,
                    read_memory=lambda: 1024 ** 3, read_json=lambda _: wrong,
                    stop=lambda owned: stopped.append(owned) or 0)
                self.assertEqual(stopped, [], key)
                self.assertIn("memory_guard_error", receipt)

    def test_sufficient_memory_does_not_stop_container(self):
        done, receipt = threading.Event(), {}
        def memory():
            done.set()
            return 3 * 1024 ** 3
        validate.memory_guard(done, Path("unused.cid"), "image", "contract", "volume", receipt,
            read_memory=memory, stop=lambda _: self.fail("Must not stop with adequate memory"))
        self.assertNotIn("interruption", receipt)
        self.assertEqual(receipt["minimum_observed_host_free_bytes"], 3 * 1024 ** 3)

    def test_guard_failure_or_interruption_cannot_pass_even_with_zero_command_exit(self):
        for evidence in ({"memory_guard_error": "identity mismatch"},
                         {"interruption": {"reason": "host_memory_below_2_gib", "stop_exit_code": 0}}):
            receipt = {"process_exit_code": 0, **evidence}
            self.assertNotEqual(validate.validation_result(receipt), 0)
            self.assertNotEqual(receipt["result"], "passed")
        receipt = {"process_exit_code": 0}
        self.assertEqual(validate.validation_result(receipt), 0)
        self.assertEqual(receipt["result"], "passed")


class SourceProvenanceTests(unittest.TestCase):
    def test_retained_archive_must_match_exact_commit_not_its_label(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            repo = root / "repo"
            repo.mkdir()
            def git(*args):
                return subprocess.check_output(["git", "-C", str(repo), *args], stderr=subprocess.PIPE).decode().strip()
            git("init")
            (repo / "tracked.txt").write_text("committed bytes\n")
            git("add", "tracked.txt")
            git("-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
                "-c", "core.hooksPath=/nonexistent", "commit", "-m", "Fixture")
            source = git("rev-parse", "HEAD")
            archive = root / "source.tar"
            git("archive", "--format=tar", "--output", str(archive), source)
            (root / "source.sha").write_text(source + "\n")
            validate.verify_archive(repo, source, archive)
            altered = archive.read_bytes()[:-1] + b"X"
            archive.write_bytes(altered)
            with self.assertRaisesRegex(ValueError, "differ from exact git commit"):
                validate.verify_archive(repo, source, archive)
            self.assertEqual(archive.read_bytes(), altered)
            self.assertEqual((root / "source.sha").read_text().strip(), source)

    def fixture(self, root, symlink=False):
        archive, source = root / "source.tar", root / "cache"
        source.mkdir()
        (source / "src").mkdir()
        (source / "src" / "tracked.txt").write_bytes(b"source bytes\n")
        with tarfile.open(archive, "w") as entries:
            directory = tarfile.TarInfo("src")
            directory.type = tarfile.DIRTYPE
            directory.mode = 0o755
            entries.addfile(directory)
            file = tarfile.TarInfo("src/tracked.txt")
            file.size = 13
            entries.addfile(file, io.BytesIO(b"source bytes\n"))
            if symlink:
                link = tarfile.TarInfo("source-link")
                link.type, link.linkname = tarfile.SYMTYPE, "src/tracked.txt"
                entries.addfile(link)
                try:
                    os.symlink("src/tracked.txt", source / "source-link")
                except OSError as error:
                    self.skipTest(f"Host cannot create fixture symlink: {error}")
        return archive, source

    def test_generated_build_outputs_allowed_but_tracked_edits_preserved_and_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            archive, source = self.fixture(Path(temporary))
            self.assertEqual(verify_source.verify(archive, source), 2)
            (source / "target").mkdir()
            (source / "target" / "generated.bin").write_bytes(b"build output")
            self.assertEqual(verify_source.verify(archive, source), 2)
            tracked = source / "src" / "tracked.txt"
            tracked.write_bytes(b"edited bytes\n") # same length, different content
            with self.assertRaisesRegex(ValueError, "Tracked source changed"):
                verify_source.verify(archive, source)
            self.assertEqual(tracked.read_bytes(), b"edited bytes\n")

    def test_symlink_target_changes_are_rejected_without_following_link(self):
        with tempfile.TemporaryDirectory() as temporary:
            archive, source = self.fixture(Path(temporary), symlink=True)
            self.assertEqual(verify_source.verify(archive, source), 3)
            link = source / "source-link"
            link.unlink()
            os.symlink("outside-or-missing", link)
            with self.assertRaisesRegex(ValueError, "Tracked source changed"):
                verify_source.verify(archive, source)
            self.assertEqual(os.readlink(link), "outside-or-missing")


@unittest.skipUnless(os.environ.get("XNEXUS_TOOLING_FIXTURE") == "disposable-container",
                     "Phase fixture runs only in its explicit disposable test container")
class PhaseProvenanceTests(unittest.TestCase):
    def test_actual_phase_checks_before_and_after_but_allows_generated_outputs(self):
        # This fixture never runs real compiler/tests. Its stub tools exercise
        # the real shell phase wrapper, including its post-command EXIT trap.
        self.assertEqual(os.name, "posix")
        self.assertFalse(Path("/work/source").exists())
        Path("/input").mkdir()
        Path("/evidence").mkdir()
        with tempfile.TemporaryDirectory() as temporary:
            archive, _ = SourceProvenanceTests().fixture(Path(temporary))
            Path("/input/source.tar").write_bytes(archive.read_bytes())
            stubs = Path(temporary) / "tools"
            stubs.mkdir()
            script = ("#!/bin/bash\n"
                      "echo invoked >> /evidence/stub-invocations\n"
                      "echo $CARGO_BUILD_JOBS,$CMAKE_BUILD_PARALLEL_LEVEL >> /evidence/stub-jobs\n"
                      "if [[ $0 == */npm && $* == '--prefix ui run build' ]]; then\n"
                      "  if [[ $FIXTURE_CHANGE == tracked ]]; then\n"
                      "    printf 'edited bytes\\n' > /work/source/src/tracked.txt\n"
                      "  else mkdir -p /work/source/target; printf output > /work/source/target/generated.bin; fi\n"
                      "fi\n")
            for tool in ("node", "npm", "cargo", "rustc", "uname", "gfortran", "cmake"):
                path = stubs / tool
                path.write_text(script)
                path.chmod(0o755)
            env = {**os.environ, "PATH": f"{stubs}:{os.environ['PATH']}", "FIXTURE_CHANGE": "generated"}
            def phase():
                return subprocess.run(["bash", "/tools/validate.sh", "contract"], env=env,
                                      capture_output=True, text=True, timeout=15)
            valid = phase()
            self.assertEqual(valid.returncode, 0, valid.stderr)
            self.assertIn("contract passed", valid.stdout)
            self.assertTrue(Path("/work/source/target/generated.bin").exists())
            self.assertEqual(set(Path("/evidence/stub-jobs").read_text().splitlines()), {"2,2"})
            Path("/evidence/stub-jobs").write_text("")
            reduced = subprocess.run(["bash", "/tools/validate.sh", "desktop"],
                env={**env, "XNEXUS_RESOURCE_PROFILE": "desktop-reduced"},
                capture_output=True, text=True, timeout=15)
            self.assertEqual(reduced.returncode, 0, reduced.stderr)
            self.assertEqual(set(Path("/evidence/stub-jobs").read_text().splitlines()), {"1,1"})
            Path("/evidence/stub-jobs").write_text("")
            reduced_contract = subprocess.run(["bash", "/tools/validate.sh", "contract"],
                env={**env, "XNEXUS_RESOURCE_PROFILE": "contract-reduced"},
                capture_output=True, text=True, timeout=15)
            self.assertEqual(reduced_contract.returncode, 0, reduced_contract.stderr)
            self.assertEqual(set(Path("/evidence/stub-jobs").read_text().splitlines()), {"1,1"})
            rejected = subprocess.run(["bash", "/tools/validate.sh", "contract"],
                env={**env, "XNEXUS_RESOURCE_PROFILE": "desktop-reduced"},
                capture_output=True, text=True, timeout=15)
            self.assertNotEqual(rejected.returncode, 0)
            env["FIXTURE_CHANGE"] = "tracked"
            mutated = phase()
            self.assertNotEqual(mutated.returncode, 0)
            self.assertNotIn("contract passed", mutated.stdout)
            self.assertIn("Tracked source changed during validation", mutated.stderr)
            tracked = Path("/work/source/src/tracked.txt")
            self.assertEqual(tracked.read_bytes(), b"edited bytes\n")
            calls = Path("/evidence/stub-invocations").read_bytes()
            rejected_before = phase()
            self.assertNotEqual(rejected_before.returncode, 0)
            self.assertEqual(Path("/evidence/stub-invocations").read_bytes(), calls)
            self.assertEqual(tracked.read_bytes(), b"edited bytes\n")


if __name__ == "__main__":
    unittest.main()
