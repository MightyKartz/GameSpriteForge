#!/usr/bin/env python3
"""Offline layered intake, native install/preview, playback and optional consumer-rig acceptance."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
from PIL import Image, ImageChops, ImageDraw


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def cli(forge, root, *args, ok=True):
    env = os.environ.copy()
    env.update(FORGE_JOB_STORE=str(root / "jobs"), FORGE_PLAN_STORE=str(root / "plans"))
    process = subprocess.run([str(forge), *map(str, args), "--json"], cwd=root, env=env,
                             capture_output=True, text=True, encoding="utf-8", timeout=180)
    value = json.loads(process.stdout)
    assert (process.returncode == 0 and value["ok"]) == ok, (args, value, process.stderr)
    return value.get("data") if ok else value


def transform(position=(0, 0), rotation=0, scale=(1, 1), opacity=1):
    return {"position": list(position), "rotationDegrees": rotation, "scale": list(scale), "opacity": opacity}


def check(forge, godot, root, rig_path=None, consumer=None):
    source = root / "sources"
    source.mkdir()
    watched = []
    if rig_path:
        assert consumer, "--consumer is required with --rig"
        rig = json.loads(rig_path.read_text(encoding="utf-8-sig"))
        width, height = rig["canvas_size"]
        inputs = [(item["id"], consumer / "game" / item["texture"].removeprefix("res://"), item["pivot"], item["sha256"]) for item in rig["layers"]]
        reference = consumer / rig["source"]
        watched = [rig_path, reference, consumer / "tools/asset-lock.json", consumer / "tools/toolchain.json", *[item[1] for item in inputs]]
        before = {str(path): sha(path) for path in watched}
        name = "consumer-layered-acceptance"
    else:
        width, height = 40, 24
        inputs = []
        for index, color in enumerate([(210, 40, 30, 255), (30, 120, 240, 128), (60, 60, 60, 255)]):
            path = source / f"layer_{index}.png"
            image = Image.new("RGBA", (width, height))
            ImageDraw.Draw(image).rectangle((4 + index * 3, 5, 23 + index * 3, 19), fill=color)
            image.save(path)
            inputs.append((f"layer_{index}", path, [11, 13], sha(path)))
        name = "layered-fixture"
    layers = [{"id": name, "name": name, "path": str(path), "sha256": digest,
               "pivot": pivot, "transform": transform(), "blend": "normal"} for name, path, pivot, digest in inputs]
    request = {"schemaVersion": "1", "id": name, "name": name, "license": "proprietary" if rig_path else "CC0-1.0",
               "canvas": {"width": width, "height": height, "origin": [0, 0]}, "sampling": "linear" if rig_path else "nearest", "layers": layers}
    if not rig_path:
        request["clips"] = [{"id": "move", "durationMs": 1000, "loop": False, "tracks": [{"layerId": layers[0]["id"], "keyframes": [
            {"timeMs": 0, "transform": transform()}, {"timeMs": 1000, "transform": transform((8, 4), 20, (1.2, 0.8), 0.25)}]}]},
            {"id": "cycle", "durationMs": 1000, "loop": True, "tracks": [{"layerId": layers[1]["id"], "keyframes": [
                {"timeMs": 0, "transform": transform()}, {"timeMs": 1000, "transform": transform()}]}]}]
        request["defaultClip"] = "move"
    request_path = root / "request.json"
    request_path.write_text(json.dumps(request), encoding="utf-8")
    pack = root / f"{name}.gsfpack"
    cli(forge, root, "asset", "prepare-layered", "--request", request_path, "--output", pack)
    cli(forge, root, "pack", "validate", "--path", pack)
    summary = cli(forge, root, "asset", "inspect", "--pack", pack)
    assert summary["assetType"] == "layered" and len(summary["layered"]["layers"]) == len(layers)
    composite = Image.new("RGBA", (width, height))
    source_composite = Image.new("RGBA", (width, height))
    for name, path, pivot, digest in inputs:
        output = pack / "assets/layers" / (name + ".png")
        assert sha(output) == digest == sha(path)
        composite = Image.alpha_composite(composite, Image.open(output).convert("RGBA"))
        source_composite = Image.alpha_composite(source_composite, Image.open(path).convert("RGBA"))
    assert composite.tobytes() == source_composite.tobytes()
    composite.save(root / "rest-composite.png")
    reference_match = None
    if rig_path:
        original = Image.open(reference).convert("RGBA")
        reference_match = composite.tobytes() == original.tobytes()
        assert reference_match, "static layered composition must equal the consumer's original RGBA pixels"
    preview = cli(forge, root, "godot", "preview", "--pack", pack, "--output", root / "preview", "--godot", godot)
    project = Path(preview["project"])
    audit = cli(forge, root, "godot", "verify-install", "--project", project, "--asset-key", request["id"], "--pack", pack)
    assert audit["verifiedTextures"] == len(layers)
    if not rig_path:
        (project / "playback_test.gd").write_text(PLAYBACK_TEST, encoding="utf-8")
        result = subprocess.run([str(godot), "--headless", "--path", str(project), "--script", "res://playback_test.gd", "--quit-after", "120"], capture_output=True, text=True, encoding="utf-8", timeout=40)
        (root / "playback.log").write_text(result.stdout + result.stderr, encoding="utf-8")
        assert result.returncode == 0 and "FORGE_PLAYBACK_PASSED" in result.stdout and "SCRIPT ERROR:" not in result.stderr, (result.stdout, result.stderr)
    if watched:
        assert {str(path): sha(path) for path in watched} == before, "consumer input/locks changed"
    doctor = cli(forge, root, "doctor")
    result = {"ok": True, "launcherPath": str(forge), "launcherSha256": sha(forge),
              "cliPath": doctor["cliPath"], "cliSha256": sha(Path(doctor["cliPath"])), "build": doctor["build"], "layers": len(layers),
              "canvas": [width, height], "copiedBytesExact": True, "orderedCompositeExact": True,
              "originalImagePixelsExact": reference_match, "nativeLoad": "passed", "playback": "passed" if not rig_path else "not_run",
              "consumerUnchanged": True if watched else None, "providerRequestCount": 0, "preview": preview, "audit": audit}
    (root / "summary.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(json.dumps(result, indent=2))


PLAYBACK_TEST = '''extends SceneTree
func _initialize() -> void:
\tvar player = load("res://addons/forge_assets/preview/layered.tscn").instantiate()
\troot.add_child(player)
\tplayer.set_process(false)
\tassert(player.play("move"))
\tplayer.pause()
\tplayer.advance(0.7)
\tassert(is_equal_approx(player.position_seconds, 0.0))
\tassert(player.seek(0.5))
\tvar layer = player.get_node("Layers/layer_0")
\tassert(layer.position.is_equal_approx(Vector2(15, 15)))
\tassert(is_equal_approx(layer.rotation_degrees, 10.0))
\tassert(layer.scale.is_equal_approx(Vector2(1.1, 0.9)))
\tassert(is_equal_approx(layer.modulate.a, 0.625))
\tassert(player.set_speed(2.0))
\tassert(player.play("move", false))
\tplayer.advance(0.25)
\tassert(player.finished and not player.playing)
\tassert(is_equal_approx(player.position_seconds, 1.0))
\tassert(not player.seek(NAN) and not player.set_speed(-1.0))
\tassert(player.play("cycle"))
\tplayer.advance(0.75)
\tassert(is_equal_approx(player.position_seconds, 0.5) and not player.finished)
\tassert(layer.position.is_equal_approx(Vector2(11, 13)))
\tplayer.reset_pose()
\tassert(not player.playing and not player.finished)
\tplayer.free()
\tprint("FORGE_PLAYBACK_PASSED")
\tquit(0)
'''


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--forge", type=Path, required=True)
    parser.add_argument("--godot", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--rig", type=Path)
    parser.add_argument("--consumer", type=Path)
    args = parser.parse_args()
    if args.output:
        args.output.mkdir(parents=True, exist_ok=False)
        check(args.forge.resolve(), args.godot.resolve(), args.output.resolve(), args.rig, args.consumer)
    else:
        with tempfile.TemporaryDirectory(prefix="forge-layered-") as root:
            check(args.forge.resolve(), args.godot.resolve(), Path(root), args.rig, args.consumer)
