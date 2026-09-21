#!/usr/bin/env python3
"""Reject false native passes, stale files and misleading partial-pair averages."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest


spec = importlib.util.spec_from_file_location("comparison", Path(__file__).with_name("compare-static-delivery.py"))
comparison = importlib.util.module_from_spec(spec)
spec.loader.exec_module(comparison)


class ComparisonTests(unittest.TestCase):
    def test_exit_zero_without_native_completion_is_not_acceptance(self):
        for stdout, stderr in [("", ""), ("DELIVERY_CHECK_PASSED:8\n", "SCRIPT ERROR: parse failed"),
                               ("SCRIPT ERROR: failed\nDELIVERY_CHECK_PASSED:8\n", "")]:
            with self.assertRaises(ValueError):
                comparison.require_acceptance(subprocess.CompletedProcess([], 0, stdout, stderr), 8)

    def test_wrong_count_and_failure_after_marker_are_rejected(self):
        for code, count in [(0, 7), (1, 8)]:
            with self.assertRaises(ValueError):
                comparison.require_acceptance(subprocess.CompletedProcess([], code, f"DELIVERY_CHECK_PASSED:{count}\n", ""), 8)

    def test_exact_completion_is_accepted(self):
        comparison.require_acceptance(subprocess.CompletedProcess([], 0, "Godot header\nDELIVERY_CHECK_PASSED:8\n", ""), 8)

    def test_missing_stale_and_changed_files_are_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "item.png"
            source.write_bytes(b"original")
            target = root / "installed"
            target.mkdir()
            with self.assertRaises(ValueError):
                comparison.check_inventory(target, [source])
            (target / source.name).write_bytes(b"changed")
            with self.assertRaises(ValueError):
                comparison.check_inventory(target, [source])
            (target / source.name).write_bytes(source.read_bytes())
            (target / "stale.png").write_bytes(b"old")
            with self.assertRaises(ValueError):
                comparison.check_inventory(target, [source])
            (target / "stale.png").unlink()
            (target / "nested").mkdir()
            (target / "nested/stale.png").write_bytes(b"old")
            with self.assertRaises(ValueError):
                comparison.check_inventory(target, [source])
            (target / "nested/stale.png").unlink()
            comparison.check_inventory(target, [source])

    def test_failed_pairs_remain_visible_and_do_not_become_fast_successes(self):
        rows = [{"trial": 1, "arm": "native", "phase": "first", "passed": True, "seconds": 2},
                {"trial": 1, "arm": "forge", "phase": "first", "passed": False, "seconds": 0.1},
                {"trial": 2, "arm": "native", "phase": "first", "passed": True, "seconds": 3},
                {"trial": 2, "arm": "forge", "phase": "first", "passed": True, "seconds": 9}]
        result = comparison.summarize(rows)["first"]
        self.assertEqual(result, {"completePassingPairs": 1, "observedPairs": 2,
                                  "medianSeconds": {"native": 3, "forge": 9}, "failedAttempts": 1})

    def test_missing_arm_or_empty_observations_have_no_comparison(self):
        for rows in [[], [{"trial": 1, "arm": "forge", "phase": "first", "passed": True, "seconds": 3}]]:
            result = comparison.summarize(rows)["first"]
            self.assertEqual(result["completePassingPairs"], 0)
            self.assertEqual(result["medianSeconds"], {"native": None, "forge": None})


if __name__ == "__main__":
    unittest.main()
