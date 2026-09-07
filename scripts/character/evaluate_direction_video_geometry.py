#!/usr/bin/env python3
"""Evaluate existing directional animation frames against one visual-scale gate.

The stable signal is measured over every approved frame: dominant scarf
component centroid to current foot plane. Full-body bounds are reported only
as diagnostics because gait articulation legitimately changes them.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import statistics
from pathlib import Path
from typing import Any

from PIL import Image

from normalize_direction_first_frames import measure


STABLE_SCALE_RELATIVE_TOLERANCE = 0.02


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--approved-export-root", type=Path, required=True)
    parser.add_argument("--direction-geometry-lock", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    root = args.approved_export_root.resolve()
    geometry_lock = args.direction_geometry_lock.resolve()
    direction_results: dict[str, Any] = {}
    medians: list[float] = []

    for direction in ("right", "down", "up"):
        asset_root = root / direction / f"walk-{direction}-approved-recovery"
        frame_paths = sorted((asset_root / "frames").glob("frame_*.png"))
        if not frame_paths:
            raise SystemExit(f"no approved frames found for {direction}")
        frames: list[dict[str, Any]] = []
        for frame_path in frame_paths:
            measurement = measure(Image.open(frame_path).convert("RGBA"))
            frames.append(
                {
                    "path": str(frame_path),
                    "sha256": sha256(frame_path),
                    "stableScaleDistancePx": round(
                        measurement.stable_scale_distance_px, 6
                    ),
                    "fullBodyBoundingBoxDiagnostic": list(measurement.bbox),
                    "fullBodyHeightPxDiagnostic": measurement.bbox[3]
                    - measurement.bbox[1],
                    "footPlaneYDiagnostic": measurement.foot_y,
                    "scarfCentroidY": round(measurement.scarf_centroid_y, 6),
                }
            )
        stable_values = [frame["stableScaleDistancePx"] for frame in frames]
        full_body_heights = [frame["fullBodyHeightPxDiagnostic"] for frame in frames]
        median_stable = statistics.median(stable_values)
        medians.append(median_stable)
        manifest_path = asset_root / "manifest.json"
        direction_results[direction] = {
            "approvedManifest": {
                "path": str(manifest_path),
                "sha256": sha256(manifest_path),
            },
            "frameCount": len(frames),
            "stableScaleBand": {
                "landmarks": "dominant-scarf-centroid-to-current-foot-plane",
                "medianDistancePx": round(median_stable, 6),
                "minDistancePx": round(min(stable_values), 6),
                "maxDistancePx": round(max(stable_values), 6),
                "peakToPeakPx": round(max(stable_values) - min(stable_values), 6),
            },
            "fullBodyBoundsDiagnostic": {
                "medianHeightPx": round(statistics.median(full_body_heights), 6),
                "minHeightPx": min(full_body_heights),
                "maxHeightPx": max(full_body_heights),
            },
            "frames": frames,
        }

    target = statistics.median(medians)
    all_passed = True
    for direction, result in direction_results.items():
        median_value = result["stableScaleBand"]["medianDistancePx"]
        relative_error = abs(median_value - target) / target
        passed = relative_error <= STABLE_SCALE_RELATIVE_TOLERANCE
        result["sharedVisualScaleGate"] = {
            "targetMedianDistancePx": round(target, 6),
            "relativeError": round(relative_error, 9),
            "tolerance": STABLE_SCALE_RELATIVE_TOLERANCE,
            "passed": passed,
        }
        all_passed = all_passed and passed

    right_down_delta = abs(
        direction_results["right"]["stableScaleBand"]["medianDistancePx"]
        - direction_results["down"]["stableScaleBand"]["medianDistancePx"]
    )
    right_up_delta = abs(
        direction_results["right"]["stableScaleBand"]["medianDistancePx"]
        - direction_results["up"]["stableScaleBand"]["medianDistancePx"]
    )
    report = {
        "schemaVersion": "1",
        "profile": "existing-direction-video-geometry-gate@1.0.0",
        "status": "evaluated",
        "directionGeometryLock": {
            "path": str(geometry_lock),
            "sha256": sha256(geometry_lock),
        },
        "policy": {
            "stableScaleLandmarks": "dominant-scarf-centroid-to-current-foot-plane",
            "comparisonStatistic": "median over all approved frames per direction",
            "sharedVisualScaleRelativeTolerance": STABLE_SCALE_RELATIVE_TOLERANCE,
            "fullBodyBoundsRole": "diagnostic-only",
            "approvedFramePixelsModified": False,
        },
        "directions": direction_results,
        "comparison": {
            "targetMedianDistancePx": round(target, 6),
            "rightDownMedianDeltaPx": round(right_down_delta, 6),
            "rightDownMedianRelativeDelta": round(right_down_delta / target, 9),
            "rightUpMedianDeltaPx": round(right_up_delta, 6),
            "rightUpMedianRelativeDelta": round(right_up_delta / target, 9),
            "allExistingDirectionsPassSharedVisualScaleGate": all_passed,
        },
        "verdict": (
            "all-existing-directions-reusable-with-one-runtime-scale"
            if all_passed
            else "partial-reuse-only-regenerate-failing-directions-from-first-frame-locks"
        ),
        "recommendedAction": (
            "Reuse all three approved videos with one runtime scale."
            if all_passed
            else "Right and down are compatible at one runtime scale; up is not. Regenerate up from the normalized up FirstFrameLock before promoting a unified directional Character Pack. For strict shared lineage, regenerate every direction from its normalized FirstFrameLock."
        ),
        "providerRequestCountThisOperation": 0,
    }
    write_json(args.output.resolve(), report)
    print(
        json.dumps(
            {
                "output": str(args.output.resolve()),
                "verdict": report["verdict"],
                "targetMedianDistancePx": report["comparison"][
                    "targetMedianDistancePx"
                ],
                "directionGate": {
                    direction: value["sharedVisualScaleGate"]["passed"]
                    for direction, value in direction_results.items()
                },
                "providerRequestCountThisOperation": 0,
            },
            ensure_ascii=False,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
