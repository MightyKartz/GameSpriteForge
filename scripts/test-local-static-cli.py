#!/usr/bin/env python3
"""Exercise local PNG -> static Pack -> Godot through the default CLI, offline."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import zlib


def png(path, color):
    def chunk(kind, payload):
        return struct.pack(">I", len(payload)) + kind + payload + struct.pack(">I", zlib.crc32(kind + payload))

    rows = bytearray()
    for y in range(64):
        rows.append(0)
        for x in range(64):
            rows.extend((*color, 48 if x == 20 else 255) if 20 <= x < 44 and 8 <= y < 56 else (0, 0, 0, 0))
    path.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 64, 64, 8, 6, 0, 0, 0))
                    + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b""))


def run(forge, root, args):
    env = os.environ.copy()
    env.update(FORGE_JOB_STORE=str(root / "jobs"), FORGE_PLAN_STORE=str(root / "plans"))
    result = subprocess.run([str(forge), *args, "--json"], cwd=root, env=env,
                            capture_output=True, text=True, timeout=90)
    value = json.loads(result.stdout)  # Fails if stdout contains diagnostics/multiple values.
    assert result.returncode == 0 and value["ok"], (args, value, result.stderr)
    return value["data"]


def check(forge, root, godot):
    specs = root / "specs"
    sources = specs / "sources"
    sources.mkdir(parents=True)
    png(sources / "jade.png", (0, 200, 80))
    png(sources / "stone.png", (150, 95, 70))
    game = root / "game"
    game.mkdir()
    (game / "project.godot").write_text('config_version=5\n[application]\nconfig/name="Local static test"\n[rendering]\nrenderer/rendering_method="gl_compatibility"\n')
    library = root / "library"
    run(forge, root, ["project", "init", "--path", str(library), "--name", "Static outputs", "--local-assets"])
    completed = []
    for kind, sampling in [("prop_set", "linear"), ("prop_set", "nearest"), ("icon_set", "linear")]:
        name = f"{kind}-{sampling}"
        request = {"assetProject": {"projectPath": "../library", "assetId": name}, "schemaVersion": "1", "kind": kind, "id": name, "name": name,
                   "license": "CC0-1.0", "sampling": sampling, "canvasSize": 64,
                   "items": [{"id": "jade_blade", "name": "Jade blade", "path": "sources/jade.png"},
                             {"id": "stone", "name": "Stone", "path": "sources/stone.png"}]}
        request_path = specs / f"{name}.json"
        request_path.write_text(json.dumps(request))
        plan = run(forge, root, ["plan", "prepare-static", "--request", str(request_path)])
        job = run(forge, root, ["plan", "execute", "--token", plan["token"], "--wait"])
        assert job["lifecycle_state"] == "succeeded"
        history = run(forge, root, ["asset", "history", "--project", str(library), "--id", name])
        assert len(history) == 1 and history[0]["status"] == "available"
        publication = next(a for a in job["artifacts"] if a["kind"] == "asset_publication_request")
        run(forge, root, ["asset", "recover", "--input", publication["path"]])
        assert len(run(forge, root, ["asset", "history", "--project", str(library), "--id", name])) == 1
        report = run(forge, root, ["job", "report", "--id", job["job_id"]])
        assert report["providerRequestOccurred"] is False and report["providerRequestCount"] == 0
        pack = Path(next(a["path"] for a in job["artifacts"] if a["kind"] == "gsfpack"))
        run(forge, root, ["pack", "validate", "--path", str(pack)])
        info = run(forge, root, ["asset", "inspect", "--pack", str(pack)])
        assert info["assetType"] == kind and [i["id"] for i in info["items"]] == ["jade_blade", "stone"]
        metadata = json.loads((pack / "forgepack.json").read_text())
        assert metadata["source"]["kind"] == "import_frames"
        assert metadata["items"][0]["provenance"]["sha256"] == hashlib.sha256((sources / "jade.png").read_bytes()).hexdigest()
        assert metadata["source"]["metadata"]["providerRequestCount"] == 0
        revision = history[0]["revision"]
        retained = run(forge, root, ["asset", "retain", "--project", str(library), "--id", name, "--revision", revision])
        assert retained["retained"]
        asset_lock = game / ".forge/resources.lock.json"
        run(forge, root, ["asset", "lock", "--project", str(library), "--id", name, "--revision", revision, "--out", str(asset_lock)])
        if godot:
            plan = run(forge, root, ["godot", "plan-install", "--library", str(library), "--asset-id", name, "--asset-lock", str(asset_lock), "--project", str(game), "--asset-key", name])
            assert "whole_pack" in json.dumps(plan) and "jade_blade" in json.dumps(plan) and "stone" in json.dumps(plan)
            installed = run(forge, root, ["plan", "execute", "--token", plan["token"], "--wait"])
            assert installed["lifecycle_state"] == "succeeded"
            associations = run(forge, root, ["asset", "installations", "--project", str(library), "--id", name])
            assert associations[-1]["revision"] == revision
            usage = json.loads((game / "addons/forge_assets" / name / "forge_usage.json").read_text())
            assert usage["rendering"]["textureFilter"] == sampling
            assert usage["anchor"] == {"type": "feet" if kind == "prop_set" else "center", "x": 32, "y": 60 if kind == "prop_set" else 32}
            assert usage["texturePaths"]["jade_blade"] == f"res://addons/forge_assets/{name}/items/jade_blade.png"
        completed.append({"kind": kind, "sampling": sampling, "jobId": job["job_id"], "pack": str(pack), "providerRequests": 0})
    if godot:
        (game / "verify.gd").write_text('''extends SceneTree
func _initialize() -> void:
\tfor sampling in ["linear", "nearest"]:
\t\tvar scene = load("res://addons/forge_assets/prop_set-%s/scenes/jade_blade.tscn" % sampling).instantiate()
\t\tvar sprite = scene.get_node("Sprite2D")
\t\tassert(not sprite.centered and sprite.position == Vector2(-32, -60))
\t\tassert(sprite.texture_filter == (CanvasItem.TEXTURE_FILTER_LINEAR if sampling == "linear" else CanvasItem.TEXTURE_FILTER_NEAREST))
\t\tscene.free()
\tassert(load("res://addons/forge_assets/icon_set-linear/items/jade_blade.png") is Texture2D)
\tprint("PASS local static CLI Godot resources")
\tquit(0)
''')
        result = subprocess.run([str(godot), "--headless", "--path", str(game), "--script", "res://verify.gd", "--quit-after", "120"], capture_output=True, text=True, timeout=60)
        assert result.returncode == 0 and "PASS local static CLI Godot resources" in result.stdout, (result.stdout, result.stderr)
    summary = {"ok": True, "forge": str(forge), "godot": str(godot) if godot else None,
               "cases": completed, "source": "deterministic local test PNGs; no Provider calls"}
    (root / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--forge", type=Path, required=True)
    parser.add_argument("--godot", type=Path)
    parser.add_argument("--output", type=Path, help="Optional new directory retaining test evidence")
    args = parser.parse_args()
    if args.godot:
        args.godot = args.godot.resolve()
        # The installation Job must use the same engine as the resource check.
        os.environ["FORGE_GODOT_PATH"] = str(args.godot)
    if args.output:
        args.output.mkdir(parents=True, exist_ok=False)
        check(args.forge.resolve(), args.output.resolve(), args.godot)
    else:
        with tempfile.TemporaryDirectory(prefix="forge-local-static-") as temporary:
            check(args.forge.resolve(), Path(temporary), args.godot)
