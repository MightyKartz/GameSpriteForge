#!/usr/bin/env python3
"""Fail-closed acceptance and source fixture integrity for the M0 experiment."""
import json
import subprocess
import tempfile
import unittest
from argparse import Namespace
from pathlib import Path
from unittest.mock import patch

from PIL import Image

import agent_resource_baseline as baseline


class BaselineTests(unittest.TestCase):
    def test_native_success_requires_exact_count_and_clean_completion(self):
        for code, stdout, stderr in [
            (0, "", ""),
            (0, "RESOURCE_TASK_PASS:4\n", ""),
            (1, "RESOURCE_TASK_PASS:5\n", ""),
            (0, "RESOURCE_TASK_PASS:5\n", "SCRIPT ERROR: parse failed"),
            (0, "SCRIPT ERROR: bad load\nRESOURCE_TASK_PASS:5\n", ""),
            (0, "RESOURCE_TASK_PASS:5\n", "FAIL:pixels:synthetic:idle:0"),
        ]:
            with self.subTest(code=code, stdout=stdout, stderr=stderr):
                with self.assertRaises(ValueError):
                    baseline.require_native(subprocess.CompletedProcess([], code, stdout, stderr), 5)
        baseline.require_native(subprocess.CompletedProcess([], 0, "Godot\nRESOURCE_TASK_PASS:5\n", ""), 5)

    def test_independent_frames_match_declared_input_layout_and_preserve_soft_edges(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            recipes, contract = baseline.make_inputs(root)
            for character in contract["characters"]:
                recipe = recipes[character["id"]][1]
                self.assertNotEqual(recipe["metadata"]["defaultAnimation"], character["actions"][0]["name"])
                for expected, action in zip(character["actions"], recipe["animations"]):
                    source = action["input"]
                    sheet = None
                    if source["kind"] == "sprite_sheet":
                        sheet = Image.open(root / "requests" / source["path"]).convert("RGBA")
                    for i, frame_path in enumerate(expected["frames"]):
                        frame = Image.open(root / frame_path).convert("RGBA")
                        self.assertEqual(frame.size, tuple(character["size"]))
                        if sheet:
                            spec = source["split"]
                            x = i % spec["columns"] * spec["frameWidth"]
                            y = i // spec["columns"] * spec["frameHeight"]
                            actual = sheet.crop((x, y, x + frame.width, y + frame.height))
                        else:
                            actual = Image.open(root / "requests" / source["paths"][i]).convert("RGBA")
                        self.assertEqual(frame.tobytes(), actual.tobytes())
            edge = Image.open(root / "inputs/synthetic-attack-2.png").convert("RGBA")
            self.assertEqual(edge.getpixel((22, 20)), (120, 50, 210, 1))
            self.assertEqual(edge.getpixel((56, 30))[3], 128)
            for index in range(3):
                pixels = [Image.open(root / f"inputs/synthetic-{action}-{index}.png").tobytes()
                          for action in ["idle", "walk", "attack"]]
                self.assertEqual(len(set(pixels)), 3, "Swapping actions must change pixels")
            self.assertFalse(contract["characters"][0]["actions"][-1]["loop"])
            self.assertGreater(len(set(contract["characters"][0]["actions"][0]["durationsMs"])), 1)

    def test_negative_control_restores_contract_even_after_timeout_or_false_pass(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            contract = {"characters": [{"actions": [{"durationsMs": [70, 150]}]}]}
            baseline.save(root / "acceptance.json", contract)
            original = (root / "acceptance.json").read_bytes()
            for outcome in [subprocess.TimeoutExpired("godot", 1),
                            subprocess.CompletedProcess([], 0, "", ""),
                            subprocess.CompletedProcess([], 1, "", "FAIL:duration:synthetic:idle:0\nSCRIPT ERROR: crashed")]:
                def invoke(label, argv):
                    changed = json.loads((root / "acceptance.json").read_text())
                    self.assertEqual(changed["characters"][0]["actions"][0]["durationsMs"][0], 80)
                    if isinstance(outcome, Exception):
                        raise outcome
                    return outcome
                with self.assertRaises((ValueError, subprocess.TimeoutExpired)):
                    baseline.check_negative_duration(root, contract, invoke, [])
                self.assertEqual((root / "acceptance.json").read_bytes(), original)
                self.assertEqual(contract["characters"][0]["actions"][0]["durationsMs"][0], 70)

    def test_early_failure_still_records_source_integrity(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "attempt"
            executable = Path(directory) / "stub"
            executable.write_text("unit-test identity only")
            calls = 0
            def invoke(*args, **kwargs):
                nonlocal calls
                calls += 1
                if calls == 1:
                    return subprocess.CompletedProcess([], 0, '{"ok":true,"data":{}}', "")
                if calls == 2:
                    return subprocess.CompletedProcess([], 0, "unit-test Godot stub", "")
                (root / "inputs/prop.png").unlink()
                raise OSError("simulated early command failure")
            with patch.object(baseline.subprocess, "run", side_effect=invoke):
                with self.assertRaises(OSError):
                    baseline.run(Namespace(output=root, forge=executable, godot=executable))
            report = json.loads((root / "report.json").read_text())
            self.assertFalse(report["ok"])
            self.assertFalse(report["sourcesUnchanged"])
            self.assertEqual(report["error"], "simulated early command failure")

    def test_existing_output_is_never_reused(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            sentinel = root / "report.json"
            sentinel.write_text("previous failed observation")
            with self.assertRaises(FileExistsError):
                baseline.run(Namespace(output=root, forge=root / "forge", godot=root / "godot"))
            self.assertEqual(sentinel.read_text(), "previous failed observation")


if __name__ == "__main__":
    unittest.main()
