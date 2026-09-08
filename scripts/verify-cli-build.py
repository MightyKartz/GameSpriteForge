#!/usr/bin/env python3
"""Verify the executable's compiled identity, including an installed release payload."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile


REQUIRED_CAPABILITIES = {
    "local_static_import", "local_animation_import", "preserve_source_coordinates",
    "local_animation_timing", "whole_sheet_source_transform", "pack_validation", "godot_install",
}


def verify(args):
    with tempfile.TemporaryDirectory(prefix="forge-build-check-") as temporary:
        env = dict(os.environ, FORGE_JOB_STORE=str(Path(temporary) / "jobs"),
                   FORGE_PLAN_STORE=str(Path(temporary) / "plans"))
        result = subprocess.run([str(args.forge.resolve()), "doctor", "--json"],
                                env=env, capture_output=True, text=True, timeout=30)
        assert result.returncode == 0, result.stderr
        envelope = json.loads(result.stdout)
        assert envelope["ok"], envelope.get("error")
        data = envelope["data"]
    build = data["build"]
    assert data["cliVersion"] == args.version, (data["cliVersion"], args.version)
    assert REQUIRED_CAPABILITIES <= set(data["capabilities"]), data["capabilities"]
    if args.commit:
        assert build["gitCommit"] == args.commit, (build["gitCommit"], args.commit)
    if args.release:
        assert build["dirty"] is False, build
        assert build["profile"] == "release", build
        assert build["target"] == "aarch64-apple-darwin", build
        assert build["features"] == [], build
    if args.build_info:
        manifest = json.loads(args.build_info.read_text())
        assert manifest["version"] == data["cliVersion"], manifest
        assert manifest["commit"] == build["gitCommit"], (manifest, build)
        assert manifest["target"] == build["target"], (manifest, build)
        payload = args.build_info.resolve().parent
        for key, binary in [("ffmpegPath", "ffmpeg"), ("ffprobePath", "ffprobe")]:
            assert data[key] and Path(data[key]).resolve() == payload / "bin" / binary, data[key]
    return {"ok": True, "cliVersion": data["cliVersion"], "build": build,
            "capabilities": data["capabilities"], "packagedPayloadChecked": bool(args.build_info)}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--forge", type=Path, required=True)
    parser.add_argument("--version", required=True, help="Version without the tag's v prefix")
    parser.add_argument("--commit", help="Expected full source commit")
    parser.add_argument("--release", action="store_true", help="Require a clean default macOS ARM64 release build")
    parser.add_argument("--build-info", type=Path, help="Installed payload BUILD_INFO.json")
    print(json.dumps(verify(parser.parse_args()), indent=2))
