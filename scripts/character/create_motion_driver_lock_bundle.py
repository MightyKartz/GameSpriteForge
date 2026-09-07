#!/usr/bin/env python3
"""Bind geometry-locked first frames to exact motion-driver prompts."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.write_text(
        json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--experiment-root", type=Path, required=True)
    args = parser.parse_args()
    root = args.experiment_root.resolve()
    first_frame_index_path = root / "first-frame-locks.json"
    first_frame_index = json.loads(first_frame_index_path.read_text(encoding="utf-8"))
    locks: dict[str, Any] = {}

    for direction in ("right", "down", "up", "left"):
        first_frame_lock_path = Path(first_frame_index["locks"][direction]["path"])
        first_frame_lock = json.loads(first_frame_lock_path.read_text(encoding="utf-8"))
        direction_root = root / "motion-driver-locks" / direction
        prompt_path = direction_root / "prompt.zh-CN.txt"
        lock_path = direction_root / "motion-driver-lock.json"
        lock = {
            "schemaVersion": "1",
            "profile": "geometry-locked-motion-driver@1.0.0",
            "status": "ready-not-submitted",
            "direction": direction,
            "animation": f"walk_{direction}",
            "firstFrameLock": {
                "path": str(first_frame_lock_path),
                "sha256": sha256(first_frame_lock_path),
            },
            "providerInput": first_frame_lock["providerInput"],
            "prompt": {
                "path": str(prompt_path),
                "sha256": sha256(prompt_path),
                "language": "zh-CN",
            },
            "requestContract": {
                "mode": "image-to-video",
                "preferredValidatedModel": "Vidu Q2",
                "durationSeconds": 5,
                "aspectRatio": "1:1",
                "audio": False,
                "camera": "fixed",
                "backgroundRgb": [255, 0, 255],
                "submitOnlyAfterExplicitCostConfirmation": True,
            },
            "geometryGate": {
                "requiredBeforeSubmission": True,
                "firstFrameGatePassed": first_frame_lock["gate"]["passed"],
                "stableScaleDistancePx": first_frame_lock["measurement"][
                    "stableScaleBand"
                ]["distancePx"],
                "footPlaneY": first_frame_lock["measurement"]["footPlaneY"],
                "foregroundCenterX": first_frame_lock["measurement"][
                    "foregroundCenterX"
                ],
            },
            "providerRequestCountThisOperation": 0,
        }
        write_json(lock_path, lock)
        locks[direction] = {"path": str(lock_path), "sha256": sha256(lock_path)}

    index_path = root / "motion-driver-locks.json"
    write_json(
        index_path,
        {
            "schemaVersion": "1",
            "profile": "geometry-locked-motion-driver-index@1.0.0",
            "status": "ready-not-submitted",
            "firstFrameLockIndex": {
                "path": str(first_frame_index_path),
                "sha256": sha256(first_frame_index_path),
            },
            "locks": locks,
            "providerRequestCountThisOperation": 0,
        },
    )
    print(
        json.dumps(
            {
                "index": str(index_path),
                "sha256": sha256(index_path),
                "directions": list(locks),
                "status": "ready-not-submitted",
                "providerRequestCountThisOperation": 0,
            },
            ensure_ascii=False,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
