#!/usr/bin/env python3
"""Offline black-box tests for image locks; all PNGs and projects are synthetic."""
import argparse
import binascii
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import zlib


def png_bytes():
    def chunk(kind, payload):
        return (struct.pack(">I", len(payload)) + kind + payload
                + struct.pack(">I", binascii.crc32(kind + payload) & 0xffffffff))

    rows = []
    for y in range(4):
        rows.append(b"\0" + b"".join(
            bytes((100, 80, 50, 255 if (x, y) == (1, 1) else 0))
            for x in range(4)))
    return (b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", struct.pack(">IIBBBBB", 4, 4, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(b"".join(rows))) + chunk(b"IEND", b""))


def inventory(root):
    return {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in root.rglob("*") if p.is_file()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--forge", type=Path, required=True)
    args = parser.parse_args()
    binary = args.forge.resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix="forge-image-contract-") as tmp:
        workspace = Path(tmp)
        root = workspace / "project with spaces"
        (root / "game" / "items").mkdir(parents=True)
        (root / "tools").mkdir()
        image = root / "game/items/portrait.png"
        image.write_bytes(png_bytes())
        lock = {"schemaVersion": 1, "images": [{
            "id": "portrait", "path": "game/items/portrait.png",
            "width": 4, "height": 4,
            "sha256": hashlib.sha256(image.read_bytes()).hexdigest(),
            "requiresAlpha": True,
            "sourceManifest": "unverified-source.json",
        }], "expectedCounts": {"images": 1}}
        lock_path = root / "tools/asset-lock.json"

        def save_lock():
            lock_path.write_text(json.dumps(lock), encoding="utf-8")

        env = os.environ.copy()
        env["FORGE_JOB_STORE"] = str(root / "must-not-create-jobs")
        env["FORGE_PLAN_STORE"] = str(root / "must-not-create-plans")
        command = [str(binary), "asset", "verify-images", "--root", str(root),
                   "--lock", "tools/asset-lock.json", "--scan", "game", "--json"]

        def invoke(expected_code=0, extra=()):
            before = inventory(root)
            result = subprocess.run(command + list(extra), cwd=workspace, env=env,
                                    capture_output=True, encoding="utf-8", timeout=30)
            assert result.returncode == expected_code, (result.returncode, result.stdout, result.stderr)
            value = json.loads(result.stdout)  # Also rejects multiple JSON/stdout messages.
            assert value["schemaVersion"] == "1", value
            assert inventory(root) == before, "verification mutated consumer files"
            assert not (root / "must-not-create-jobs").exists()
            assert not (root / "must-not-create-plans").exists()
            return value

        save_lock()
        success = invoke(extra=("--scan", "game/items"))
        assert success["ok"] is True
        assert success["data"]["scope"] == "image_contract_only"
        assert success["data"]["scannedImageCount"] == 1
        assert success["data"]["notCheckedFields"] == ["images[].sourceManifest"]

        # Same count is insufficient: replacing an expected filename must report both sides.
        image.rename(image.with_name("unlocked.PNG"))
        drift = invoke(1)
        assert drift["ok"] is False
        assert drift["error"]["code"] == "image_contract_failed"
        assert {issue["code"] for issue in drift["data"]["issues"]} == {"missing_image", "unexpected_image"}
        image.with_name("unlocked.PNG").rename(image)

        # Aggregate content and dimension drift; don't stop at the first discrepancy.
        lock["images"][0]["sha256"] = "0" * 64
        lock["images"][0]["width"] = 5
        save_lock()
        drift = invoke(1)
        assert {issue["code"] for issue in drift["data"]["issues"]} == {"sha256_mismatch", "dimensions_mismatch"}

        # Malformed input uses one standard error envelope, without partial-success data.
        lock["images"][0]["path"] = "../outside.png"
        save_lock()
        bad_input = invoke(1)
        assert bad_input["ok"] is False
        assert bad_input["error"]["code"] == "invalid_image_contract"
        assert "data" not in bad_input

        result = subprocess.run([str(binary), "asset", "verify-images", "--root", str(root),
                                 "--lock", str(lock_path), "--json"],
                                cwd=workspace, env=env, capture_output=True, encoding="utf-8", timeout=30)
        assert result.returncode != 0 and "--scan" in result.stderr

    print(json.dumps({"ok": True, "checks": [
        "success_and_overlap", "root_relative_paths_from_different_cwd",
        "image_only_scope", "ignored_metadata_reported", "missing_and_unlocked_same_count",
        "aggregate_content_drift", "invalid_path_error_envelope", "scan_is_required",
        "read_only_without_job_or_plan_stores"], "providerRequests": 0}))


if __name__ == "__main__":
    main()
