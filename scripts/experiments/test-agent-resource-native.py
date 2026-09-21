#!/usr/bin/env python3
"""Exercise the M0 acceptance oracle with tiny native resources, not Forge output.

This tests the checker, not Forge delivery or Godot PNG import settings.
"""
import argparse
import copy
import json
from pathlib import Path
import shutil
import subprocess

import agent_resource_baseline as baseline

FIXTURE = '''extends SceneTree
func _initialize() -> void:
	var mode = OS.get_cmdline_user_args()[0]
	var image = Image.create(2, 2, false, Image.FORMAT_RGBA8)
	image.fill(Color8(150, 80, 190, 255))
	image.set_pixel(0, 0, Color8(120, 50, 210, 1))
	assert(image.save_png("res://source.png") == OK)
	var texture = ImageTexture.create_from_image(image)
	assert(ResourceSaver.save(texture, "res://texture.tres") == OK)
	var frames = SpriteFrames.new()
	frames.remove_animation("default")
	frames.add_animation("idle")
	frames.set_animation_speed("idle", 1000.0)
	frames.set_animation_loop("idle", true)
	frames.add_frame("idle", texture, 70.0)
	frames.add_frame("idle", texture, 230.0)
	var player = Node2D.new()
	player.set_script(load("res://player.gd"))
	var sprite = AnimatedSprite2D.new()
	sprite.name = "AnimatedSprite2D"
	sprite.sprite_frames = frames
	sprite.animation = "idle"
	sprite.centered = false
	sprite.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	sprite.position = Vector2(-1, -2)
	if mode != "missing-sprite":
		player.add_child(sprite)
		sprite.owner = player
	else:
		sprite.free()
	if mode == "missing-frames":
		sprite.sprite_frames = null
	var scene = PackedScene.new()
	assert(scene.pack(player) == OK)
	assert(ResourceSaver.save(scene, "res://character.tscn") == OK)
	player.free()
	var audio = AudioStreamWAV.new()
	audio.format = AudioStreamWAV.FORMAT_16_BITS
	audio.mix_rate = 22050
	audio.stereo = false
	var pcm = PackedByteArray()
	pcm.resize(64)
	audio.data = pcm
	audio.loop_mode = AudioStreamWAV.LOOP_FORWARD
	audio.loop_begin = 0
	audio.loop_end = 32
	if mode == "bad-loop-range":
		audio.loop_begin = 10
		audio.loop_end = 11
	assert(ResourceSaver.save(audio, "res://audio.res") == OK)
	if mode == "bad-visible-pixel":
		image.set_pixel(0, 0, Color8(1, 2, 3, 1))
		assert(image.save_png("res://source.png") == OK)
	print("FIXTURE_READY")
	quit(0)
'''


def run(args):
    root = args.output.absolute()
    root.mkdir(parents=True, exist_ok=False)
    (root / "project.godot").write_text('config_version=5\n[application]\nconfig/name="M0 oracle tests"\n')
    (root / "fixture.gd").write_text(FIXTURE)
    shutil.copyfile(baseline.REPO / "scripts/godot/runtime/layered-player-v1.gd", root / "player.gd")
    shutil.copyfile(baseline.CHECKER, root / "verify.gd")
    contract = {
        "schemaVersion": 1,
        "characters": [{"id": "synthetic", "scene": "res://character.tscn", "size": [2, 2],
                        "anchor": [1, 2], "actions": [{"name": "idle", "durationsMs": [70, 230],
                        "loop": True, "frames": [str(root / "source.png")] * 2}]}],
        "static": [{"id": "prop", "size": [2, 2], "texture": "res://texture.tres",
                    "source": str(root / "source.png")}],
        "audio": [{"id": "music", "stream": "res://audio.res", "sampleRate": 22050,
                   "channels": 1, "frames": 32, "loop": True}],
    }
    report = {"ok": False, "checkerSha256": baseline.digest(baseline.CHECKER), "cases": []}
    godot = str(args.godot.absolute())
    try:
        version = subprocess.run([godot, "--version"], check=True, capture_output=True, text=True, timeout=30)
        report["godotVersion"] = version.stdout.strip()
        for case, failure in [
            ("valid", None),
            ("bad-character-size", "size:synthetic:idle:0"),
            ("bad-static-size", "static-size:prop"),
            ("bad-loop-range", "audio-loop-range:music"),
            ("bad-duration", "duration:synthetic:idle:0"),
            ("bad-visible-pixel", "pixels:synthetic:idle:0"),
            ("missing-sprite", "sprite:synthetic"),
            ("missing-frames", "sprite-frames:synthetic"),
        ]:
            result = subprocess.run([godot, "--headless", "--path", str(root), "--script",
                                     "res://fixture.gd", "--", case], capture_output=True, text=True, timeout=30)
            baseline.save(root / f"{case}-fixture.json", {"exitCode": result.returncode,
                          "stdout": result.stdout, "stderr": result.stderr})
            baseline.require(result.returncode == 0 and "FIXTURE_READY" in result.stdout.splitlines()
                             and "SCRIPT ERROR" not in result.stdout + result.stderr, "Fixture failed: " + case)
            expected = copy.deepcopy(contract)
            if case == "bad-character-size":
                expected["characters"][0]["size"] = [4, 1]
            if case == "bad-static-size":
                expected["static"][0]["size"] = [4, 1]
            if case == "bad-duration":
                expected["characters"][0]["actions"][0]["durationsMs"][0] += 10
            baseline.save(root / "acceptance.json", expected)
            result = subprocess.run([godot, "--headless", "--path", str(root), "--script",
                                     "res://verify.gd"], capture_output=True, text=True, timeout=30)
            report["cases"].append({"case": case, "exitCode": result.returncode,
                                    "stdout": result.stdout, "stderr": result.stderr})
            if failure is None:
                baseline.require_native(result, 3)
            else:
                baseline.require(result.returncode == 1 and "FAIL:" + failure in result.stderr.splitlines()
                                 and "SCRIPT ERROR" not in result.stdout + result.stderr,
                                 "Missing deterministic rejection: " + case)
        report["ok"] = True
    finally:
        baseline.save(root / "report.json", report)
    print(json.dumps({"ok": True, "cases": len(report["cases"]), "report": str(root / "report.json")}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--godot", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    run(parser.parse_args())
