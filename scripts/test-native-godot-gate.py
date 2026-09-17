#!/usr/bin/env python3
"""Regression checks for false-positive native test acceptance."""
import importlib.util
from pathlib import Path
import unittest


spec = importlib.util.spec_from_file_location(
    "native_gate", Path(__file__).with_name("test-native-godot-transactions.py"))
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)


class NativeGateTests(unittest.TestCase):
    def test_zero_discovery_is_rejected(self):
        with self.assertRaises(RuntimeError):
            gate.verify_listing("0 tests, 0 benchmarks\n")

    def test_zero_execution_is_rejected(self):
        with self.assertRaises(RuntimeError):
            gate.verify_execution("test result: ok. 0 passed; 0 failed; 0 ignored;\n")

    def test_missing_named_test_is_rejected(self):
        with self.assertRaises(RuntimeError):
            gate.verify_listing("\n".join(f"{name}: test" for name in sorted(gate.REQUIRED)[1:]))

    def test_skipped_or_failed_test_is_rejected(self):
        for outcome in ["ignored", "FAILED"]:
            with self.subTest(outcome=outcome), self.assertRaises(RuntimeError):
                gate.verify_execution("\n".join(f"test {name} ... {outcome}" for name in gate.REQUIRED)
                                      + "\ntest result: ok. 5 passed; 0 failed; 0 ignored;")

    def test_summary_alone_is_not_evidence(self):
        with self.assertRaises(RuntimeError):
            gate.verify_execution("test result: ok. 5 passed; 0 failed; 0 ignored;")

    def test_named_success_is_accepted(self):
        self.assertEqual(gate.verify_listing("\n".join(f"{name}: test" for name in gate.REQUIRED)), gate.REQUIRED)
        output = "\n".join(f"test {name} ... ok" for name in gate.REQUIRED)
        self.assertEqual(gate.verify_execution(output + "\ntest result: ok. 5 passed; 0 failed; 0 ignored;"), gate.REQUIRED)


if __name__ == "__main__":
    unittest.main()
