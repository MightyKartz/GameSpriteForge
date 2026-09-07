#!/usr/bin/env python3
"""Offline integrity regressions for the directional reuse experiment helper.

The fixture is intentionally small: its PNGs are only identity payloads for
source/approval closure. Actual Pack decoding and Godot rendering are covered by
the experiment's real local install and runtime checks, not by these tests.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import prepare_directional_reuse as reuse


PNG = base64.b64decode(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aF1sAAAAASUVORK5CYII="
)


def put_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding="utf-8")


def read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def hash_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class DirectionalReuseTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="forge-directional-reuse-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.sources = self.root / "sources"
        self.gate = self.root / "geometry" / "reuse-gate.json"
        lock = self.gate.parent / "lock.json"
        put_json(lock, {"fixture": "geometry authority"})
        gate = {"directionGeometryLock": {"path": str(lock), "sha256": hash_file(lock)},
                "directions": {}}
        self.exports: dict[str, Path] = {}
        self.packs: dict[str, Path] = {}

        reference_scale = 737.589513
        for direction, stable in (("right", reference_scale), ("down", 737.228160), ("up", 784.751588)):
            export = self.sources / direction / f"walk-{direction}-approved-recovery"
            pack = export / f"{direction}.gsfpack"
            self.exports[direction], self.packs[direction] = export, pack
            frames = []
            for index in (1, 2):
                frame = export / "frames" / f"frame_{index:03d}.png"
                packed = pack / "assets/frames" / frame.name
                for path in (frame, packed):
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_bytes(PNG)
                frames.append({"path": str(frame), "sha256": hash_file(frame),
                               "stableScaleDistancePx": stable})
            durations = [166, 208] if direction == "right" else [125, 125]
            animation = {"name": f"walk_{direction}", "frames": [0, 1],
                         "frameDurationsMs": durations, "fps": 2000 / sum(durations), "loop": True}
            manifest = {"animations": [animation], "anchor": {"x": 7.0, "y": 13.0}}
            for path in (export / "manifest.json", pack / "assets/manifest.json"):
                put_json(path, manifest)
            approval = {"status": "approved", "checks": {
                "motionAcceptable": True, "placementAcceptable": True,
                "edgeCleanupAcceptable": True, "footAlternationAcceptable": True,
                "pivotAcceptable": True, "collisionAcceptable": True,
            }, "frameSha256": [frame["sha256"] for frame in frames],
                "frameDurationsMs": durations}
            approval_path = pack / "quality/animation-human-review.json"
            original_review = self.root / "original-evidence" / direction / "review.json"
            for path in (approval_path, original_review):
                put_json(path, approval)
            replay = original_review.parent / "replay.json"
            put_json(replay, {"fixture": "native replay"})
            lineage = pack / "quality/animation-review-lineage.json"
            put_json(lineage, {"originalApprovalSha256": hash_file(original_review)})
            certification = {"frameSha256": approval["frameSha256"], "frameDurationsMs": durations,
                             "approvedReviewPath": str(original_review),
                             "approvedReviewSha256": hash_file(original_review),
                             "recoveredReplaySummaryPath": str(replay),
                             "recoveredReplaySummarySha256": hash_file(replay)}
            put_json(pack / "quality/recovery-certification.json", certification)
            metadata = {"animationHumanReviewSha256": hash_file(approval_path),
                        "animationReviewLineageSha256": hash_file(lineage),
                        "approvedFrameSha256": approval["frameSha256"],
                        "runtimeContract": {"playbackSpeedScale": 2.0,
                                            "sourceCycleDurationMs": sum(durations),
                                            "targetCycleDurationMs": sum(durations) / 2}}
            put_json(pack / "forgepack.json", {"animations": [animation],
                                               "source": {"metadata": metadata}})
            gate["directions"][direction] = {
                "approvedManifest": {"path": str(export / "manifest.json"),
                                     "sha256": hash_file(export / "manifest.json")},
                "frames": frames, "frameCount": 2,
                "stableScaleBand": {"medianDistancePx": stable},
                "sharedVisualScaleGate": {"passed": direction != "up",
                                          "relativeError": abs(stable - reference_scale) / reference_scale},
            }
        put_json(self.gate, gate)

    def test_native_timing_survives_historical_runtime_acceleration(self) -> None:
        entries, snapshots = reuse.inspect_sources(self.sources, self.gate)
        contract = reuse.build_contract(entries)
        right = contract["directions"]["right"]
        self.assertEqual(right["frameDurationsMs"], [166, 208])
        self.assertEqual(right["sourceCycleDurationMs"], 374)
        self.assertEqual(right["playbackSpeedScale"], 1.0)
        self.assertEqual(right["sourceNativePivotPx"], {"x": 7.0, "y": 13.0})
        self.assertEqual(sum(entry["frameCount"] for entry in entries.values()), 6)
        self.assertEqual(right["calibrationScale"], 1.0)
        self.assertEqual(contract["directions"]["down"]["calibrationScale"], 1.0)
        self.assertAlmostEqual(contract["directions"]["up"]["calibrationScale"], 0.9399, delta=0.00001)
        self.assertFalse(contract["directions"]["up"]["stableScaleGatePassed"])
        self.assertEqual(contract["visualReview"], "pending")
        self.assertFalse(contract["productionPromoted"])
        for path, snapshot in snapshots.items():
            self.assertEqual(reuse.tree_snapshot(Path(path)), snapshot)

    def test_source_frame_tampering_is_rejected(self) -> None:
        frame = self.exports["right"] / "frames/frame_001.png"
        frame.write_bytes(frame.read_bytes() + b"changed after approval")
        with self.assertRaisesRegex(ValueError, "source hash mismatch"):
            reuse.inspect_sources(self.sources, self.gate)

    def test_pack_frame_tampering_is_rejected(self) -> None:
        frame = self.packs["right"] / "assets/frames/frame_002.png"
        frame.write_bytes(b"replacement")
        with self.assertRaisesRegex(ValueError, "source hash mismatch"):
            reuse.inspect_sources(self.sources, self.gate)

    def test_hash_valid_approval_with_wrong_durations_is_rejected(self) -> None:
        pack = self.packs["right"]
        path = pack / "quality/animation-human-review.json"
        approval = read_json(path)
        approval["frameDurationsMs"] = [187, 187]  # Same total, different actual cadence.
        put_json(path, approval)
        document = read_json(pack / "forgepack.json")
        document["source"]["metadata"]["animationHumanReviewSha256"] = hash_file(path)
        put_json(pack / "forgepack.json", document)
        with self.assertRaisesRegex(ValueError, "approved frame timing mismatch"):
            reuse.inspect_sources(self.sources, self.gate)

    def test_existing_and_protected_output_locations_are_rejected(self) -> None:
        existing = self.root / "already-created"
        existing.mkdir()
        with self.assertRaisesRegex(ValueError, "already exists"):
            reuse.check_output_location(existing, [self.sources])
        with self.assertRaisesRegex(ValueError, "overlaps protected source"):
            reuse.check_output_location(self.sources / "new-candidate", [self.sources])
        # Also protect a not-yet-created source underneath a proposed output root.
        with self.assertRaisesRegex(ValueError, "overlaps protected source"):
            reuse.check_output_location(self.root / "future", [self.root / "future/source"])
        reuse.check_output_location(self.root / "fresh-candidate", [self.sources, self.gate.parent])

    def test_provider_plan_requires_both_explicit_zero_bounds(self) -> None:
        reuse.local_plan_only({"estimate": {"providerRequestEstimate": 0, "maximumProviderRequests": 0}})
        for estimate in ({}, {"providerRequestEstimate": 0}, {"maximumProviderRequests": 0},
                         {"providerRequestEstimate": 1, "maximumProviderRequests": 0},
                         {"providerRequestEstimate": 0, "maximumProviderRequests": 1},
                         {"providerRequestEstimate": None, "maximumProviderRequests": 0},
                         {"providerRequestEstimate": 0, "maximumProviderRequests": -1}):
            with self.subTest(estimate=estimate), self.assertRaises(ValueError):
                reuse.local_plan_only({"estimate": estimate})
        with self.assertRaises((ValueError, KeyError)):
            reuse.local_plan_only({})

    def test_unsafe_install_plan_never_reaches_execute(self) -> None:
        templates = self.root / "templates"
        templates.mkdir()
        for name in ("project.godot", "main.tscn", "directional_reuse_review.gd"):
            (templates / name).write_text("fixture\n", encoding="utf-8")
        calls = []

        def fake_cli(_binary: Path, args: list[str], _env: dict, _log: Path) -> dict:
            calls.append(args[:2])
            if args[:2] == ["pack", "validate"]:
                return {"valid": True}
            if args[:2] == ["godot", "plan-install"]:
                return {"token": "must-never-execute", "estimate": {"providerRequestEstimate": 0}}
            self.fail(f"unsafe plan reached an execution step: {args}")

        args = argparse.Namespace(source_root=self.sources, reuse_gate=self.gate,
                                  output=self.root / "candidate", forge=self.root / "unused-forge",
                                  godot=self.root / "unused-godot")
        with patch.object(reuse, "TEMPLATES", templates), patch.object(reuse, "cli", fake_cli):
            with self.assertRaisesRegex(ValueError, "explicit zero Provider bounds"):
                reuse.prepare(args)
        self.assertEqual(calls, [["pack", "validate"], ["godot", "plan-install"]])


if __name__ == "__main__":
    unittest.main()
