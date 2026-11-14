"""Guard the isolation command before downloading/building an image."""
import importlib.util
from pathlib import Path
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


if __name__ == "__main__":
    unittest.main()
