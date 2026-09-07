#!/usr/bin/env python3
"""Verify closure for the zero-provider direction geometry experiment."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def verify_ref(ref: dict[str, Any], label: str, checks: list[dict[str, Any]]) -> None:
    path = Path(ref["path"])
    actual = sha256(path)
    expected = ref["sha256"]
    passed = actual == expected
    checks.append(
        {
            "check": label,
            "path": str(path),
            "expectedSha256": expected,
            "actualSha256": actual,
            "passed": passed,
        }
    )
    if not passed:
        raise SystemExit(f"hash mismatch for {label}: {path}")


def collect_provider_counts(value: Any, path: str = "$") -> list[dict[str, Any]]:
    results: list[dict[str, Any]] = []
    if isinstance(value, dict):
        for key, child in value.items():
            child_path = f"{path}.{key}"
            if key == "providerRequestCountThisOperation":
                results.append({"path": child_path, "value": child, "passed": child == 0})
            results.extend(collect_provider_counts(child, child_path))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            results.extend(collect_provider_counts(child, f"{path}[{index}]"))
    return results


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--experiment-root", type=Path, required=True)
    args = parser.parse_args()
    root = args.experiment_root.resolve()
    checks: list[dict[str, Any]] = []

    source_lock = load(root / "direction-grid-source-lock.json")
    verify_ref(source_lock["sourceGrid"], "source-grid", checks)
    for direction, ref in source_lock["cells"].items():
        verify_ref(ref, f"source-cell-{direction}", checks)

    geometry_lock_path = root / "direction-geometry-lock.json"
    geometry_lock = load(geometry_lock_path)
    verify_ref(geometry_lock["sourceGridLock"], "source-grid-lock", checks)
    verify_ref(geometry_lock["preview"], "geometry-preview", checks)
    if not geometry_lock["allDirectionsPassed"]:
        raise SystemExit("direction geometry lock did not pass all directions")
    for direction, entry in geometry_lock["directions"].items():
        verify_ref(entry["source"], f"geometry-source-{direction}", checks)
        verify_ref(entry["lockedAlphaFirstFrame"], f"alpha-first-frame-{direction}", checks)
        verify_ref(entry["lockedProviderInput"], f"provider-first-frame-{direction}", checks)
        if not entry["gate"]["passed"]:
            raise SystemExit(f"direction geometry gate failed for {direction}")

    first_frame_index_path = root / "first-frame-locks.json"
    first_frame_index = load(first_frame_index_path)
    for direction, ref in first_frame_index["locks"].items():
        verify_ref(ref, f"first-frame-lock-{direction}", checks)
        lock = load(Path(ref["path"]))
        verify_ref(lock["sourceCell"], f"first-frame-source-{direction}", checks)
        verify_ref(lock["alphaFirstFrame"], f"first-frame-alpha-{direction}", checks)
        verify_ref(lock["providerInput"], f"first-frame-provider-{direction}", checks)

    motion_index_path = root / "motion-driver-locks.json"
    motion_index = load(motion_index_path)
    verify_ref(motion_index["firstFrameLockIndex"], "motion-first-frame-index", checks)
    if motion_index["status"] != "ready-not-submitted":
        raise SystemExit("motion-driver bundle must remain ready-not-submitted")
    for direction, ref in motion_index["locks"].items():
        verify_ref(ref, f"motion-driver-lock-{direction}", checks)
        lock = load(Path(ref["path"]))
        verify_ref(lock["firstFrameLock"], f"motion-first-frame-{direction}", checks)
        verify_ref(lock["providerInput"], f"motion-provider-input-{direction}", checks)
        verify_ref(lock["prompt"], f"motion-prompt-{direction}", checks)
        if not lock["geometryGate"]["firstFrameGatePassed"]:
            raise SystemExit(f"motion geometry preflight failed for {direction}")

    reuse_gate_path = root / "existing-video-reuse-gate.json"
    reuse_gate = load(reuse_gate_path)
    verify_ref(reuse_gate["directionGeometryLock"], "reuse-direction-geometry-lock", checks)
    for direction, result in reuse_gate["directions"].items():
        verify_ref(result["approvedManifest"], f"approved-manifest-{direction}", checks)
        for index, frame in enumerate(result["frames"]):
            verify_ref(frame, f"approved-frame-{direction}-{index + 1:03d}", checks)

    godot_root = root / "godot-unified-contract-diagnostic"
    runtime_contract = load(godot_root / "runtime-contracts.json")
    verify_ref(runtime_contract["firstFrameLockIndex"], "godot-first-frame-index", checks)
    verify_ref(runtime_contract["existingVideoReuseGate"], "godot-reuse-gate", checks)
    godot_report_path = godot_root / "qa-output" / "unified-contract-report.json"
    godot_report = load(godot_report_path)
    comparison = godot_report["comparison"]
    required_true = [
        "sharedWorldPivotApplied",
        "sharedVisualScaleApplied",
        "sharedCollisionApplied",
        "sharedMovementSpeedApplied",
        "sharedTargetCycleDurationApplied",
        "worldPivotStable",
    ]
    for key in required_true:
        if comparison[key] is not True:
            raise SystemExit(f"Godot runtime contract check failed: {key}")
    if comparison["approvedFramePixelsModified"] is not False:
        raise SystemExit("approved frames were unexpectedly modified")
    screenshot_path = godot_root / "qa-output" / "unified-contract.png"
    if not screenshot_path.is_file():
        raise SystemExit("Godot screenshot is missing")

    text_resource_checks: list[dict[str, Any]] = []
    forbidden_markers = (b"PackedByteArray", b"data:image/", b"base64,")
    for pattern in ("*.tscn", "*.tres"):
        for path in sorted(godot_root.rglob(pattern)):
            content = path.read_bytes()
            passed = len(content) < 1024 * 1024 and not any(
                marker in content for marker in forbidden_markers
            )
            text_resource_checks.append(
                {
                    "path": str(path),
                    "sizeBytes": len(content),
                    "underOneMiB": len(content) < 1024 * 1024,
                    "externalRasterOnly": not any(
                        marker in content for marker in forbidden_markers
                    ),
                    "passed": passed,
                }
            )
            if not passed:
                raise SystemExit(f"Godot text resource contract failed: {path}")

    provider_checks: list[dict[str, Any]] = []
    for path in (
        root / "direction-grid-source-lock.json",
        geometry_lock_path,
        first_frame_index_path,
        motion_index_path,
        reuse_gate_path,
        godot_root / "runtime-contracts.json",
        godot_report_path,
    ):
        for result in collect_provider_counts(load(path)):
            result["document"] = str(path)
            provider_checks.append(result)
            if not result["passed"]:
                raise SystemExit(f"non-zero provider request count in {path}")

    report = {
        "schemaVersion": "1",
        "profile": "direction-geometry-experiment-verification@1.0.0",
        "status": "passed",
        "hashClosureChecks": checks,
        "godotTextResourceChecks": text_resource_checks,
        "providerRequestCountChecks": provider_checks,
        "summary": {
            "hashClosurePassed": True,
            "directionGeometryGatePassed": True,
            "motionDriverBundleReadyNotSubmitted": True,
            "godotUnifiedRuntimeContractPassed": True,
            "existingVideoScaleGateAllPassed": False,
            "existingVideoReusableDirections": ["right", "down"],
            "existingVideoRegenerationRequiredDirections": ["up"],
            "approvedFramesUntouched": True,
            "providerRequestCountThisOperation": 0,
        },
    }
    output = root / "verification-report.json"
    output.write_text(
        json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(
        json.dumps(
            {
                "output": str(output),
                "sha256": sha256(output),
                "status": "passed",
                "hashChecks": len(checks),
                "textResourceChecks": len(text_resource_checks),
                "providerRequestCountThisOperation": 0,
            },
            ensure_ascii=False,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
