#!/usr/bin/env python3
"""Offline policy unit tests; mocked subprocess only, never invokes cargo/network."""
import importlib.util
import json
import os
import tempfile
from pathlib import Path
import unittest
from unittest.mock import patch
from types import SimpleNamespace

spec = importlib.util.spec_from_file_location("notices", Path(__file__).with_name("gen-rust-licenses.py"))
notices = importlib.util.module_from_spec(spec)
spec.loader.exec_module(notices)


class OfflinePolicy(unittest.TestCase):
    def test_isolated_home_changes_only_cargo_subprocess_environment(self):
        before = dict(os.environ)
        with tempfile.TemporaryDirectory() as home, patch.object(notices, "CARGO_HOME_OVERRIDE", home), patch.object(notices.subprocess, "run", return_value=SimpleNamespace(returncode=0, stdout="", stderr="")) as run:
            notices.cargo("metadata")
            self.assertEqual(run.call_args.kwargs["env"]["CARGO_HOME"], home)
            self.assertEqual(dict(os.environ), before)

    def test_cli_isolated_home_is_explicit_offline_and_never_changes_parent_environment(self):
        before = dict(os.environ)
        with tempfile.TemporaryDirectory() as home:
            def fail():
                self.assertEqual(notices.CARGO_HOME_OVERRIDE, home)
                self.assertTrue(notices.OFFLINE)
                raise notices.Stop("expected test stop")
            with patch.object(notices, "generate", side_effect=fail), patch("builtins.print"):
                self.assertEqual(notices.main(["generator", "--offline", "--cargo-home", home]), 2)
            self.assertEqual(dict(os.environ), before)
            with patch.object(notices, "generate") as generate, patch("builtins.print"):
                for flags in [["--cargo-home"], ["--cargo-home", home], ["--offline", "--cargo-home", "relative"], ["--offline", "--cargo-home", home, "--cargo-home", home]]:
                    self.assertEqual(notices.main(["generator", *flags]), 2)
                generate.assert_not_called()

    def test_metadata_queries_every_declared_target_and_unions_distinct_packages(self):
        def reply(*args):
            self.assertEqual(args[0], "metadata")
            target = args[args.index("--filter-platform") + 1]
            return json.dumps({"packages": [{"name": target, "version": "1", "source": "registry"}]})
        with patch.object(notices, "cargo", side_effect=reply) as run:
            packages = notices.desktop_packages()
        self.assertEqual({name for name, _ in packages}, set(notices.TARGETS))
        self.assertEqual(run.call_count, 4)
        self.assertEqual([call.args[-1] for call in run.call_args_list], notices.TARGETS)

    def test_normal_tree_coverage_uses_the_same_four_targets(self):
        def reply(*args):
            self.assertEqual(args[0], "tree")
            self.assertEqual(args[args.index("-e") + 1], "normal")
            return args[args.index("--target") + 1] + " v1\n"
        with patch.object(notices, "cargo", side_effect=reply) as run:
            self.assertEqual({name for name, _ in notices.shipped_crates()}, set(notices.TARGETS))
        self.assertEqual(run.call_count, 4)

    def test_missing_required_target_metadata_fails_without_partial_union(self):
        with patch.object(notices, "cargo", side_effect=[json.dumps({"packages": []}), notices.Stop("required target cache missing")]) as run:
            with self.assertRaisesRegex(notices.Stop, "required target cache missing"):
                notices.desktop_packages()
            self.assertEqual(run.call_count, 2)

    def test_conflicting_cross_target_package_identity_is_not_silently_overwritten(self):
        first = {"name": "shared", "version": "1", "source": "registry-a"}
        second = {**first, "source": "registry-b"}
        with patch.object(notices, "cargo", side_effect=[json.dumps({"packages": [first]}), json.dumps({"packages": [second]})]):
            with self.assertRaisesRegex(notices.Stop, "conflicting package metadata"):
                notices.desktop_packages()

    def test_offline_applies_to_every_cargo_resolution_call(self):
        with patch.object(notices, "OFFLINE", True), patch.object(notices.subprocess, "run", return_value=SimpleNamespace(returncode=0, stdout="", stderr="")) as run:
            notices.cargo("metadata", "--format-version", "1")
            notices.cargo("tree", "--target", "x86_64-pc-windows-gnu")
            self.assertEqual(run.call_count, 2)
            for call in run.call_args_list:
                self.assertIn("--offline", call.args[0])
                self.assertIn("--locked", call.args[0])

    def test_missing_cached_input_is_an_error_without_retry(self):
        with patch.object(notices, "OFFLINE", True), patch.object(notices.subprocess, "run", return_value=SimpleNamespace(returncode=101, stdout="", stderr="offline dependency missing")) as run:
            with self.assertRaisesRegex(notices.Stop, "offline dependency missing"):
                notices.cargo("metadata")
            self.assertEqual(run.call_count, 1)

    def test_cli_preserves_check_mode_and_propagates_offline(self):
        def fail():
            self.assertTrue(notices.OFFLINE)
            raise notices.Stop("deliberate missing cache")
        with patch.object(notices, "generate", side_effect=fail), patch("builtins.print"):
            self.assertEqual(notices.main(["generator", "--check", "--offline"]), 2)
        with patch.object(notices, "generate") as generate, patch("builtins.print"):
            self.assertEqual(notices.main(["generator", "--offline", "--offline"]), 2)
            generate.assert_not_called()


if __name__ == "__main__":
    unittest.main()
