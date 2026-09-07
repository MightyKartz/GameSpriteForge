#!/usr/bin/env python3
"""Create geometry-locked directional first frames without model calls.

This tool performs one whole-image uniform resize and translation per neutral
direction still. It never edits animation frames and never applies a
non-uniform deformation. The stable scale signal is the dominant scarf
component centroid to the planted foot plane; full-body bounds remain
diagnostic because silhouette height changes with direction.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import statistics
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from PIL import Image, ImageDraw, ImageFont


CANVAS_SIZE = (512, 512)
TARGET_CENTER_X = 256.0
TARGET_FOOT_Y = 480.0
ALPHA_THRESHOLD = 48
STABLE_SCALE_TOLERANCE = 0.015
FOOT_TOLERANCE_PX = 1.0
CENTER_TOLERANCE_PX = 1.0

DIRECTIONS = {
    "down": "front_idle.png",
    "up": "back_idle.png",
    "right": "right_idle.png",
    "left": "left_idle.png",
}


@dataclass(frozen=True)
class Measurement:
    bbox: tuple[int, int, int, int]
    scarf_bbox: tuple[int, int, int, int]
    scarf_centroid_y: float
    foot_y: float
    center_x: float
    stable_scale_distance_px: float

    def json(self) -> dict[str, Any]:
        left, top, right, bottom = self.bbox
        return {
            "alphaBoundingBox": list(self.bbox),
            "fullBodyWidthPxDiagnostic": right - left,
            "fullBodyHeightPxDiagnostic": bottom - top,
            "scarfComponentBoundingBox": list(self.scarf_bbox),
            "scarfCentroidY": round(self.scarf_centroid_y, 6),
            "footPlaneY": round(self.foot_y, 6),
            "foregroundCenterX": round(self.center_x, 6),
            "stableScaleBand": {
                "landmarks": "dominant-scarf-centroid-to-planted-foot-plane",
                "distancePx": round(self.stable_scale_distance_px, 6),
            },
        }


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def alpha_bbox(image: Image.Image) -> tuple[int, int, int, int]:
    alpha = image.convert("RGBA").getchannel("A")
    mask = alpha.point(lambda value: 255 if value >= ALPHA_THRESHOLD else 0)
    bbox = mask.getbbox()
    if bbox is None:
        raise ValueError("direction still has no foreground alpha")
    return bbox


def is_scarf_pixel(pixel: tuple[int, int, int, int]) -> bool:
    red, green, blue, alpha = pixel
    return (
        alpha >= ALPHA_THRESHOLD
        and red > 105
        and green > 48
        and blue < 70
        and red > green * 1.15
        and green > blue * 1.5
    )


def dominant_scarf_component(
    image: Image.Image, bbox: tuple[int, int, int, int]
) -> list[tuple[int, int]]:
    rgba = image.convert("RGBA")
    left, top, right, bottom = bbox
    search_bottom = int(top + 0.62 * (bottom - top))
    candidates = {
        (x, y)
        for y in range(top, search_bottom)
        for x in range(left, right)
        if is_scarf_pixel(rgba.getpixel((x, y)))
    }
    components: list[list[tuple[int, int]]] = []
    while candidates:
        seed = candidates.pop()
        stack = [seed]
        component = [seed]
        while stack:
            x, y = stack.pop()
            for neighbor in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
                if neighbor in candidates:
                    candidates.remove(neighbor)
                    stack.append(neighbor)
                    component.append(neighbor)
        components.append(component)
    if not components:
        raise ValueError("direction still has no dominant scarf component")
    return max(components, key=len)


def measure(image: Image.Image) -> Measurement:
    bbox = alpha_bbox(image)
    component = dominant_scarf_component(image, bbox)
    xs = [point[0] for point in component]
    ys = [point[1] for point in component]
    scarf_bbox = (min(xs), min(ys), max(xs) + 1, max(ys) + 1)
    scarf_centroid_y = sum(ys) / len(ys)
    left, _, right, bottom = bbox
    foot_y = float(bottom - 1)
    center_x = (left + right - 1) / 2.0
    return Measurement(
        bbox=bbox,
        scarf_bbox=scarf_bbox,
        scarf_centroid_y=scarf_centroid_y,
        foot_y=foot_y,
        center_x=center_x,
        stable_scale_distance_px=foot_y - scarf_centroid_y,
    )


def normalize(
    image: Image.Image, source: Measurement, target_distance: float
) -> tuple[Image.Image, float, tuple[int, int], tuple[float, float]]:
    scale = target_distance / source.stable_scale_distance_px
    crop = image.convert("RGBA").crop(source.bbox)
    width = max(1, round(crop.width * scale))
    height = max(1, round(crop.height * scale))
    premultiplied = crop.convert("RGBa")
    resized = premultiplied.resize((width, height), Image.Resampling.LANCZOS).convert("RGBA")
    resized_bbox = alpha_bbox(resized)
    resized_center_x = (resized_bbox[0] + resized_bbox[2] - 1) / 2.0
    resized_foot_y = resized_bbox[3] - 1
    translation_x = round(TARGET_CENTER_X - resized_center_x)
    translation_y = round(TARGET_FOOT_Y - resized_foot_y)
    canvas = Image.new("RGBA", CANVAS_SIZE, (0, 0, 0, 0))
    canvas.alpha_composite(resized, (translation_x, translation_y))
    source_canvas_translation = (
        translation_x - source.bbox[0] * scale,
        translation_y - source.bbox[1] * scale,
    )
    return canvas, scale, (translation_x, translation_y), source_canvas_translation


def magenta_provider_input(image: Image.Image) -> Image.Image:
    background = Image.new("RGB", CANVAS_SIZE, (255, 0, 255))
    foreground = image.convert("RGBA")
    background.paste(foreground.convert("RGB"), (0, 0), foreground.getchannel("A"))
    return background


def checkerboard(size: tuple[int, int]) -> Image.Image:
    output = Image.new("RGB", size, (25, 31, 40))
    draw = ImageDraw.Draw(output)
    tile = 24
    for y in range(0, size[1], tile):
        for x in range(0, size[0], tile):
            if (x // tile + y // tile) % 2 == 0:
                draw.rectangle((x, y, x + tile - 1, y + tile - 1), fill=(38, 47, 58))
    return output


def contact_sheet(
    originals: dict[str, Image.Image], normalized: dict[str, Image.Image], output: Path
) -> None:
    directions = ["down", "up", "right", "left"]
    margin_y = 34
    sheet = checkerboard((2048, 2 * (512 + margin_y)))
    draw = ImageDraw.Draw(sheet)
    font = ImageFont.load_default()
    for row, group in enumerate((originals, normalized)):
        row_y = row * (512 + margin_y) + margin_y
        for column, direction in enumerate(directions):
            x = column * 512
            sheet.paste(group[direction], (x, row_y), group[direction])
            draw.line((x, row_y + int(TARGET_FOOT_Y), x + 511, row_y + int(TARGET_FOOT_Y)), fill=(71, 195, 255), width=2)
            label = ("SOURCE  " if row == 0 else "LOCKED  ") + direction
            draw.text((x + 12, row_y - 25), label, font=font, fill=(235, 242, 249))
    output.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(output, format="PNG", optimize=False)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--source-grid", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    args = parser.parse_args()
    output_root = args.output_root.resolve()
    if output_root.exists() and any(output_root.iterdir()):
        raise SystemExit(f"refusing to overwrite non-empty output root: {output_root}")
    output_root.mkdir(parents=True, exist_ok=True)

    source_root = args.source_root.resolve()
    source_grid = args.source_grid.resolve()
    originals: dict[str, Image.Image] = {}
    source_paths: dict[str, Path] = {}
    source_measurements: dict[str, Measurement] = {}
    for direction, filename in DIRECTIONS.items():
        path = source_root / filename
        image = Image.open(path).convert("RGBA")
        if image.size != CANVAS_SIZE:
            raise SystemExit(f"{path} must be exactly {CANVAS_SIZE}")
        source_paths[direction] = path
        originals[direction] = image
        source_measurements[direction] = measure(image)

    target_distance = statistics.median(
        measurement.stable_scale_distance_px
        for measurement in source_measurements.values()
    )
    normalized: dict[str, Image.Image] = {}
    entries: dict[str, Any] = {}
    for direction in ["down", "up", "right", "left"]:
        source = source_measurements[direction]
        image, uniform_scale, crop_paste_offset, source_canvas_translation = normalize(
            originals[direction], source, target_distance
        )
        measurement = measure(image)
        direction_root = output_root / "first-frame-locks" / direction
        alpha_path = direction_root / f"{direction}_idle_geometry_locked.png"
        provider_path = direction_root / f"{direction}_idle_geometry_locked_magenta.png"
        direction_root.mkdir(parents=True, exist_ok=True)
        image.save(alpha_path, format="PNG", optimize=False)
        magenta_provider_input(image).save(provider_path, format="PNG", optimize=False)
        stable_error = abs(measurement.stable_scale_distance_px - target_distance) / target_distance
        foot_error = abs(measurement.foot_y - TARGET_FOOT_Y)
        center_error = abs(measurement.center_x - TARGET_CENTER_X)
        passed = (
            stable_error <= STABLE_SCALE_TOLERANCE
            and foot_error <= FOOT_TOLERANCE_PX
            and center_error <= CENTER_TOLERANCE_PX
        )
        entries[direction] = {
            "source": {
                "path": str(source_paths[direction]),
                "sha256": sha256(source_paths[direction]),
                "measurement": source.json(),
            },
            "transform": {
                "scope": "single-pre-video-direction-still",
                "uniformScale": round(uniform_scale, 9),
                "sourceCanvasAffineTranslationPx": [
                    round(source_canvas_translation[0], 6),
                    round(source_canvas_translation[1], 6),
                ],
                "resizedCropPasteOffsetPx": list(crop_paste_offset),
                "nonUniformScale": False,
                "rotationDegrees": 0,
                "redrawApplied": False,
                "animationFramesModified": False,
                "resampling": "Pillow LANCZOS on premultiplied RGBA",
            },
            "lockedAlphaFirstFrame": {
                "path": str(alpha_path),
                "sha256": sha256(alpha_path),
            },
            "lockedProviderInput": {
                "path": str(provider_path),
                "sha256": sha256(provider_path),
                "backgroundRgb": [255, 0, 255],
            },
            "measurement": measurement.json(),
            "gate": {
                "stableScaleRelativeError": round(stable_error, 9),
                "footPlaneErrorPx": round(foot_error, 6),
                "centerXErrorPx": round(center_error, 6),
                "passed": passed,
            },
        }
        if not passed:
            raise SystemExit(f"normalized first frame failed geometry gate: {direction}")
        normalized[direction] = image

    preview_path = output_root / "direction-geometry-source-vs-locked.png"
    contact_sheet(originals, normalized, preview_path)
    source_lock_path = output_root / "direction-grid-source-lock.json"
    source_lock = {
        "schemaVersion": "1",
        "profile": "direction-grid-source-lock@1.0.0",
        "status": "locked",
        "sourceGrid": {"path": str(source_grid), "sha256": sha256(source_grid)},
        "cells": {
            direction: {
                "path": str(path),
                "sha256": sha256(path),
                "canvas": list(CANVAS_SIZE),
            }
            for direction, path in source_paths.items()
        },
        "providerRequestCountThisOperation": 0,
    }
    write_json(source_lock_path, source_lock)

    geometry_lock_path = output_root / "direction-geometry-lock.json"
    geometry_lock = {
        "schemaVersion": "1",
        "profile": "direction-geometry-lock@1.0.0",
        "status": "locked",
        "sourceGridLock": {
            "path": str(source_lock_path),
            "sha256": sha256(source_lock_path),
        },
        "policy": {
            "canvas": list(CANVAS_SIZE),
            "targetFootPlaneY": TARGET_FOOT_Y,
            "targetCenterX": TARGET_CENTER_X,
            "stableScaleLandmarks": "dominant-scarf-centroid-to-planted-foot-plane",
            "targetStableScaleDistancePx": round(target_distance, 6),
            "stableScaleRelativeTolerance": STABLE_SCALE_TOLERANCE,
            "footPlaneTolerancePx": FOOT_TOLERANCE_PX,
            "centerXTolerancePx": CENTER_TOLERANCE_PX,
            "fullBodyBoundsRole": "diagnostic-only",
            "allowedTransform": "one whole-still uniform scale plus translation before video acquisition",
            "forbiddenTransforms": [
                "non-uniform-scale",
                "warp",
                "redraw",
                "per-frame-animation-transform",
            ],
        },
        "directions": entries,
        "preview": {"path": str(preview_path), "sha256": sha256(preview_path)},
        "allDirectionsPassed": all(entry["gate"]["passed"] for entry in entries.values()),
        "providerRequestCountThisOperation": 0,
    }
    write_json(geometry_lock_path, geometry_lock)
    geometry_lock_sha256 = sha256(geometry_lock_path)

    first_frame_locks: dict[str, Any] = {}
    for direction in ["down", "up", "right", "left"]:
        entry = entries[direction]
        lock_path = output_root / "first-frame-locks" / direction / "first-frame-lock.json"
        lock = {
            "schemaVersion": "1",
            "profile": "direction-first-frame-lock@1.0.0",
            "status": "locked",
            "direction": direction,
            "directionGeometryLockPath": str(geometry_lock_path),
            "directionGeometryLockSha256": geometry_lock_sha256,
            "sourceCell": entry["source"],
            "alphaFirstFrame": entry["lockedAlphaFirstFrame"],
            "providerInput": entry["lockedProviderInput"],
            "measurement": entry["measurement"],
            "transform": entry["transform"],
            "gate": entry["gate"],
            "providerRequestCountThisOperation": 0,
        }
        write_json(lock_path, lock)
        first_frame_locks[direction] = {
            "path": str(lock_path),
            "sha256": sha256(lock_path),
        }

    index_path = output_root / "first-frame-locks.json"
    write_json(
        index_path,
        {
            "schemaVersion": "1",
            "profile": "direction-first-frame-lock-index@1.0.0",
            "directionGeometryLockPath": str(geometry_lock_path),
            "directionGeometryLockSha256": geometry_lock_sha256,
            "locks": first_frame_locks,
            "providerRequestCountThisOperation": 0,
        },
    )
    print(
        json.dumps(
            {
                "directionGeometryLock": str(geometry_lock_path),
                "directionGeometryLockSha256": geometry_lock_sha256,
                "firstFrameLockIndex": str(index_path),
                "firstFrameLockIndexSha256": sha256(index_path),
                "targetStableScaleDistancePx": round(target_distance, 6),
                "allDirectionsPassed": True,
                "providerRequestCountThisOperation": 0,
            },
            ensure_ascii=False,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
