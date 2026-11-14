"""Guard the isolation command before downloading/building an image."""
import importlib.util
from pathlib import Path
import tempfile
import threading
import unittest

spec = importlib.util.spec_from_file_location("validate", Path(__file__).with_name("validate.py"))
validate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(validate)


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


if __name__ == "__main__":
    unittest.main()
