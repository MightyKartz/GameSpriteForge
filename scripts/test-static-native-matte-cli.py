#!/usr/bin/env python3
"""Offline CLI coverage for native static canvases and explicit single-PNG matting."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import zlib


def rgb_png():
    def chunk(kind, payload):
        return (struct.pack(">I", len(payload)) + kind + payload
                + struct.pack(">I", zlib.crc32(kind + payload) & 0xffffffff))
    rows = bytearray()
    for y in range(11):
        rows.append(0)
        for x in range(23):
            rows.extend((255, 255, 200) if (x, y) == (3, 7)
                        else (70, 80, 90) if (x, y) == (19, 2) else (255, 255, 255))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 23, 11, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b""))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check(binary, root):
    specs = root / "requests with spaces"
    sources = specs / "sources"
    sources.mkdir(parents=True)
    original = sources / "original.png"
    original.write_bytes(rgb_png())
    original_hash = sha(original)
    env = os.environ.copy()
    env.update(FORGE_JOB_STORE=str(root / "jobs"), FORGE_PLAN_STORE=str(root / "plans"))

    def call(args, expected_code=0):
        result = subprocess.run([str(binary), *map(str, args), "--json"], cwd=root,
                                env=env, capture_output=True, text=True, timeout=90)
        assert result.returncode == expected_code, (result.returncode, result.stdout, result.stderr)
        value = json.loads(result.stdout)
        assert value["ok"] is (expected_code == 0), value
        return value.get("data") if expected_code == 0 else value

    matte_request = {
        "schemaVersion": "1", "input": "sources/original.png", "output": "derived/matte.png",
        "parameters": {"keyMode": "manual", "manualKeyColor": "#FFFFFF", "threshold": 10,
                       "softness": 100, "despillStrength": 0.0, "haloPixels": 0},
    }
    matte_path = specs / "matte-request.json"
    matte_path.write_text(json.dumps(matte_request), encoding="utf-8")
    report = call(["source", "matte", "--request", matte_path])
    matte = specs / "derived/matte.png"
    assert matte.is_file() and not (root / "derived/matte.png").exists()
    assert (report["width"], report["height"]) == (23, 11)
    assert report["sourceSha256"] == original_hash and report["outputSha256"] == sha(matte)
    assert report["partialAlphaPixels"] == 1 and report["visiblePixels"] == 2
    assert report["resolvedKeyColor"] == "#FFFFFF" and report["parameters"] == matte_request["parameters"]
    assert report["transparentRgbCleared"] and report["canvasPreserved"] and report["visualReviewRequired"]
    assert report["providerRequestCount"] == 0 and not report["providerRequestOccurred"]
    assert sha(original) == original_hash
    assert not (root / "jobs").exists() and not (root / "plans").exists()
    matte_hash = sha(matte)
    call(["source", "matte", "--request", matte_path], 1)
    assert sha(matte) == matte_hash
    matte_request["output"] = "sources/original.png"
    matte_path.write_text(json.dumps(matte_request), encoding="utf-8")
    call(["source", "matte", "--request", matte_path], 1)
    assert sha(original) == original_hash

    request = {
        "schemaVersion": "1", "kind": "prop_set", "id": "native-rectangles", "name": "Native rectangles",
        "license": "CC0-1.0", "sampling": "linear", "canvasPolicy": "preserve_source",
        "items": [{"id": "original", "name": "Original", "path": "sources/original.png"},
                  {"id": "matte", "name": "Matte", "path": "derived/matte.png"}],
    }

    def execute(request):
        request_path = specs / (request["id"] + ".json")
        request_path.write_text(json.dumps(request), encoding="utf-8")
        plan = call(["plan", "prepare-static", "--request", request_path])
        assert plan["estimate"]["providerRequestEstimate"] == plan["estimate"]["maximumProviderRequests"] == 0
        job = call(["plan", "execute", "--token", plan["token"], "--wait"])
        assert job["lifecycle_state"] == "succeeded", job
        pack = Path(next(a["path"] for a in job["artifacts"] if a["kind"] == "gsfpack"))
        call(["pack", "validate", "--path", pack])
        return pack, job

    native_pack, native_job = execute(request)
    assert sha(native_pack / "assets/items/original.png") == original_hash
    assert sha(native_pack / "assets/items/matte.png") == matte_hash
    native_helper = json.loads((native_pack / "assets/godot_import.json").read_text())
    assert (native_helper["frameWidth"], native_helper["frameHeight"]) == (23, 11)
    assert native_helper["anchor"] == {"type": "custom", "x": 0.0, "y": 0.0}

    # The existing JSON form continues its square normalization behavior.
    request.pop("canvasPolicy")
    request["canvasSize"] = 64
    request["id"] = "legacy-normalization"
    request["items"] = [request["items"][1]]
    legacy_pack, _ = execute(request)
    helper = json.loads((legacy_pack / "assets/godot_import.json").read_text())
    assert (helper["frameWidth"], helper["frameHeight"]) == (64, 64)
    assert helper["anchor"] == {"type": "feet", "x": 32.0, "y": 60.0}
    assert sha(original) == original_hash and sha(matte) == matte_hash
    summary = {"ok": True, "checks": ["request_relative_matte_paths", "new_output_only", "source_unchanged",
               "soft_alpha_and_hash_evidence", "matte_without_job_stores", "native_rgb_rgba_rectangles",
               "native_png_bytes_and_origin", "legacy_normalization", "zero_provider_estimates"],
               "nativeJobId": native_job["job_id"], "nativePack": str(native_pack)}
    (root / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--forge", type=Path, required=True)
    parser.add_argument("--output", type=Path, help="New directory in which to retain test evidence")
    args = parser.parse_args()
    binary = args.forge.resolve(strict=True)
    if args.output:
        args.output.mkdir(parents=True, exist_ok=False)
        check(binary, args.output.resolve())
    else:
        with tempfile.TemporaryDirectory(prefix="forge-native-matte-") as tmp:
            check(binary, Path(tmp))
