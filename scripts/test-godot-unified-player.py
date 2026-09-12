#!/usr/bin/env python3
"""Run the current unified controller in an isolated native Godot project."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


HARNESS = r'''extends SceneTree

var failures: Array[String] = []
var checks := 0

func check(condition: bool, label: String) -> void:
	checks += 1
	if not condition:
		failures.append(label)
		printerr("FAIL unified controller: " + label)

func close(a: float, b: float) -> bool:
	return absf(a - b) < 0.00001

func _initialize() -> void:
	call_deferred("run_checks")

func run_checks() -> void:
	var script = load("res://player.gd")
	check(script != null, "controller loads")
	if script == null:
		finish()
		return
	var layered = script.new()
	var layers := Node2D.new()
	layers.name = "Layers"
	var body := Node2D.new()
	body.name = "body"
	layers.add_child(body)
	layered.add_child(layers)
	root.add_child(layered)
	layered.set_process(false)
	var events: Array[String] = []
	layered.completed.connect(func(clip: String): events.append(clip))
	check(layered.clips() == PackedStringArray(["once", "looping"]), "layered clip discovery")
	layered.reset_pose()
	check(body.position.is_equal_approx(Vector2(15, 19)), "reset restores original pivot plus position")
	check(close(body.rotation_degrees, 20) and body.scale.is_equal_approx(Vector2(0.8, 1.1)) and close(body.modulate.a, 0.75), "reset restores original rotation scale opacity")
	check(not layered.state()["playing"] and not layered.state()["finished"], "reset is stopped unfinished")
	check(layered.play() and layered.state()["clip"] == "once", "layered reset then parameterless play selects default clip")
	check(layered.play("once"), "play known layered clip")
	layered.seek(0.25)
	check(body.position.is_equal_approx(Vector2(15, 14.5)), "layered seek interpolates source coordinates around pivot")
	check(close(body.rotation_degrees, 22.5) and body.scale.is_equal_approx(Vector2(1.25, 1.25)) and close(body.modulate.a, 0.875), "layered linear transform interpolation")
	var before: Dictionary = layered.state().duplicate(true)
	check(not layered.play("missing") and layered.state() == before, "unknown clip leaves state untouched")
	check(not layered.seek(-1.0) and not layered.seek(NAN) and not layered.seek(INF) and layered.state() == before, "invalid seek leaves state untouched")
	check(not layered.set_speed(-1.0) and not layered.set_speed(NAN) and not layered.set_speed(INF) and layered.state() == before, "invalid speed leaves state untouched")
	layered.play("once")
	check(layered.set_speed(2.0), "valid speed")
	layered.advance(0.2)
	check(close(layered.state()["positionSeconds"], 0.4), "double speed advancement")
	layered.pause()
	layered.advance(0.4)
	check(close(layered.state()["positionSeconds"], 0.4) and not layered.state()["playing"], "pause prevents advancement")
	layered.seek(0.25)
	check(not layered.state()["playing"], "seek preserves pause")
	layered.play("once", false)
	layered.advance(0.375)
	check(layered.state()["finished"] and not layered.state()["playing"] and close(layered.state()["positionSeconds"], 1.0), "nonloop clamps and finishes")
	check(events == ["once"], "completion emitted exactly once")
	layered.advance(1.0)
	check(events.size() == 1, "finished controller does not emit twice")
	layered.play("once")
	layered.seek(99.0)
	check(layered.state()["finished"] and not layered.state()["playing"] and close(layered.state()["positionSeconds"], 1.0), "seek past end stops and clamps")
	layered.advance(0.1)
	check(events.size() == 1, "seeking end emits no completion signal")
	layered.play("looping")
	layered.set_speed(1.0)
	layered.advance(2.25)
	check(close(layered.state()["positionSeconds"], 0.25) and layered.state()["playing"] and not layered.state()["finished"], "loop wraps without finishing")
	check(events.size() == 1, "loop emits no completion")
	layered.set_speed(0.0)
	layered.advance(100.0)
	check(close(layered.state()["positionSeconds"], 0.25), "zero speed holds pose")
	before = layered.state().duplicate(true)
	layered.advance(NAN)
	layered.advance(INF)
	layered.advance(-1.0)
	check(layered.state() == before, "invalid delta does not corrupt state")
	layered.set_speed(1e308)
	before = layered.state().duplicate(true)
	layered.advance(2.0)
	check(layered.state() == before and is_finite(layered.state()["positionSeconds"]), "overflowing time delta is ignored")
	layered.queue_free()

	# Unequal frame durations must be retained by the shared controller.
	var sprite_frames := SpriteFrames.new()
	var texture := ImageTexture.create_from_image(Image.create(1, 1, false, Image.FORMAT_RGBA8))
	for clip in ["once", "looping"]:
		sprite_frames.add_animation(clip)
		sprite_frames.set_animation_speed(clip, 10.0)
		sprite_frames.set_animation_loop(clip, clip == "looping")
		for duration in [1.0, 3.0, 2.0]:
			sprite_frames.add_frame(clip, texture, duration)
	var sequence = script.new()
	var sprite := AnimatedSprite2D.new()
	sprite.name = "AnimatedSprite2D"
	sprite.sprite_frames = sprite_frames
	sprite.animation = "once"
	sequence.add_child(sprite)
	check(sequence.state()["clip"] == "once" and close(sequence.state()["durationSeconds"], 0.6), "first pre-ready state initializes a consistent snapshot")
	root.add_child(sequence)
	sequence.set_process(false)
	var frame_events: Array[String] = []
	sequence.completed.connect(func(clip: String): frame_events.append(clip))
	check(close(sequence.duration_seconds(), 0.6), "unequal frame durations sum to true duration")
	sequence.play("once")
	sequence.seek(0.25)
	check(sprite.frame == 1 and close(sprite.frame_progress, 0.5), "sequence seek honors long middle frame")
	sequence.seek(0.45)
	check(sprite.frame == 2 and close(sprite.frame_progress, 0.25), "sequence seek preserves final-frame progress")
	sequence.play("once")
	sequence.advance(0.6 - 0.000001)
	check(not sequence.state()["finished"] and sequence.state()["playing"] and frame_events.is_empty(), "one microsecond before end does not finish early")
	sequence.play("once")
	sequence.set_speed(2.0)
	sequence.advance(0.3)
	check(sequence.state()["finished"] and not sequence.state()["playing"] and sprite.frame == 2, "sequence completion reaches final frame")
	check(frame_events == ["once"], "sequence completion emits once")
	sequence.advance(1.0)
	check(frame_events.size() == 1, "sequence completion does not repeat")
	sequence.play("looping")
	sequence.set_speed(1.0)
	sequence.advance(0.85)
	check(sprite.frame == 1 and close(sprite.frame_progress, 0.5) and not sequence.state()["finished"], "sequence loop respects unequal durations")
	sequence.pause()
	before = sequence.state().duplicate(true)
	sequence.advance(1.0)
	check(sequence.state() == before, "sequence pause holds state")
	sequence.reset_pose()
	check(sprite.frame == 0 and close(sprite.frame_progress, 0.0) and not sequence.state()["playing"], "sequence reset returns first frame")
	check(sequence.play() and sequence.state()["clip"] == "looping", "sequence reset then parameterless play retains selected clip")
	sequence.play("once")
	sequence.seek(1.0)
	check(sequence.state()["finished"] and not sequence.state()["playing"] and frame_events.size() == 1, "sequence seek-end stops without signal")
	sequence.queue_free()
	finish()

func finish() -> void:
	if failures.is_empty():
		print("FORGE_UNIFIED_PLAYER_PASS " + JSON.stringify({"checks":checks,"ok":true}))
		quit(0)
	else:
		printerr(JSON.stringify({"ok":false,"failures":failures}))
		quit(1)
'''


def manifest():
    start = {"position": [0, 0], "rotationDegrees": 0, "scale": [1, 1], "opacity": 1}
    end = {"position": [20, 10], "rotationDegrees": 90, "scale": [2, 2], "opacity": 0.5}
    original = {"position": [5, 7], "rotationDegrees": 20, "scale": [0.8, 1.1], "opacity": 0.75}
    return {
        "schemaVersion": "1", "assetType": "layered", "id": "controller-test", "name": "Controller test",
        "canvas": {"width": 80, "height": 40, "origin": [0, 0]}, "sampling": "linear",
        "layers": [{"id": "body", "name": "Body", "texture": "assets/layers/body.png", "sha256": "0" * 64,
                    "pivot": [10, 12], "transform": original, "blend": "normal"}],
        "clips": [{"id": name, "durationMs": 1000, "loop": loop,
                   "tracks": [{"layerId": "body", "keyframes": [{"timeMs": 0, "transform": start},
                                                                    {"timeMs": 1000, "transform": end}]}]}
                  for name, loop in [("once", False), ("looping", True)]], "defaultClip": "once",
    }


def run(godot, source, root):
    # This tests controller behavior directly, without claiming Pack/texture verification.
    shutil.copyfile(source, root / "player.gd")
    (root / "manifest.json").write_text(json.dumps(manifest()), encoding="utf-8")
    (root / "test.gd").write_text(HARNESS, encoding="utf-8")
    (root / "project.godot").write_text(
        'config_version=5\n[application]\nconfig/name="Forge controller test"\n'
        '[rendering]\nrenderer/rendering_method="gl_compatibility"\n', encoding="utf-8")
    result = subprocess.run([str(godot), "--headless", "--path", str(root), "--script", "res://test.gd"],
                            capture_output=True, text=True, timeout=45,
                            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
    (root / "stdout.txt").write_text(result.stdout, encoding="utf-8")
    (root / "stderr.txt").write_text(result.stderr, encoding="utf-8")
    prefix = "FORGE_UNIFIED_PLAYER_PASS "
    lines = [line[len(prefix):] for line in result.stdout.splitlines() if line.startswith(prefix)]
    assert result.returncode == 0 and len(lines) == 1, (result.returncode, result.stdout, result.stderr)
    assert "SCRIPT ERROR" not in result.stderr and "Parse Error" not in result.stderr, result.stderr
    report = json.loads(lines[0])
    assert report["ok"] and report["checks"] >= 30, report
    print(json.dumps({"ok": True, "nativeControllerChecks": report["checks"], "godot": str(godot)}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--godot", type=Path, required=True)
    parser.add_argument("--controller", type=Path,
                        default=Path(__file__).resolve().parent / "godot/forge_layered_player.gd")
    parser.add_argument("--output", type=Path, help="New directory retaining the isolated project and logs")
    args = parser.parse_args()
    if args.output:
        args.output.mkdir(parents=True, exist_ok=False)
        run(args.godot.resolve(strict=True), args.controller.resolve(strict=True), args.output.resolve())
    else:
        with tempfile.TemporaryDirectory(prefix="forge-unified-player-") as temporary:
            run(args.godot.resolve(strict=True), args.controller.resolve(strict=True), Path(temporary))
