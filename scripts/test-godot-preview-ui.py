#!/usr/bin/env python3
"""Exercise real preview controls in a copied Godot project; input stays untouched."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess


HARNESS = r'''extends SceneTree
var failures: Array[String] = []
var checks := 0
var buttons := {}
var choices: Array[OptionButton] = []

func check(condition: bool, label: String) -> void:
	checks += 1
	if not condition:
		failures.append(label)
		printerr("FAIL preview UI: " + label)

func find_controls(node: Node) -> void:
	if node is OptionButton:
		choices.append(node)
	elif node is Button:
		buttons[node.text] = node
	for child in node.get_children():
		find_controls(child)

func _initialize() -> void:
	call_deferred("run_checks")

func run_checks() -> void:
	var packed = load("res://preview.tscn") as PackedScene
	check(packed != null, "preview scene loads")
	if packed == null:
		finish()
		return
	var preview = packed.instantiate()
	root.add_child(preview)
	await process_frame
	await process_frame
	preview.set_process(false)
	var player = preview.player
	check(player != null and player.has_method("state"), "preview owns real player")
	if player == null:
		finish()
		return
	player.set_process(false)
	find_controls(preview)
	check(buttons.has("Play") and buttons.has("Pause") and buttons.has("Reset pose"), "all action controls exist")
	check(choices.size() == 2, "clip and speed selectors exist")
	check(preview.timeline != null, "timeline control exists")
	if not failures.is_empty():
		finish()
		return
	var clip_select: OptionButton
	var speed_select: OptionButton
	for choice in choices:
		if choice.item_count > 0 and choice.get_item_text(0).ends_with("x"):
			speed_select = choice
		else:
			clip_select = choice
	check(clip_select != null and clip_select.item_count > 0 and speed_select != null, "selectors contain real clips and rates")
	if clip_select == null or clip_select.item_count == 0 or speed_select == null:
		finish()
		return
	check(not player.state()["playing"], "preview starts paused")
	buttons["Play"].pressed.emit()
	check(player.state()["playing"], "Play signal starts player")
	player.advance(0.1)
	buttons["Pause"].pressed.emit()
	var paused: Dictionary = player.state().duplicate(true)
	check(not paused["playing"] and paused["positionSeconds"] > 0.0, "Pause signal preserves current progress")
	player.advance(0.2)
	check(player.state() == paused, "paused player no longer advances")
	var clip_index := mini(1, clip_select.item_count - 1)
	clip_select.select(clip_index)
	clip_select.item_selected.emit(clip_index)
	var selected: Dictionary = player.state()
	check(selected["clip"] == clip_select.get_item_text(clip_index), "clip selection changes active clip")
	check(not selected["playing"] and not selected["finished"] and selected["positionSeconds"] == 0.0, "clip selection seeks start and pauses")
	var duration := float(selected["durationSeconds"])
	check(duration > 0.0, "selected clip has duration")
	check(absf(preview.timeline.max_value - duration) < 0.00001, "clip selection updates timeline range")
	var seek_time := duration * 0.25
	preview.timeline.set_value_no_signal(seek_time)
	preview.timeline.value_changed.emit(seek_time)
	check(absf(player.state()["positionSeconds"] - seek_time) < 0.00001 and not player.state()["playing"], "timeline signal seeks paused player")
	var double_index := -1
	for index in speed_select.item_count:
		if is_equal_approx(float(speed_select.get_item_text(index).trim_suffix("x")), 2.0):
			double_index = index
	check(double_index >= 0, "double speed option exists")
	if double_index >= 0:
		speed_select.select(double_index)
		speed_select.item_selected.emit(double_index)
		check(player.state()["speed"] == 2.0, "speed selection changes real player rate")
		buttons["Play"].pressed.emit()
		player.advance(duration * 0.125)
		check(absf(player.state()["positionSeconds"] - duration * 0.5) < 0.00001, "Play resumes timeline at selected speed")
	buttons["Reset pose"].pressed.emit()
	var reset: Dictionary = player.state()
	check(not reset["playing"] and not reset["finished"] and reset["positionSeconds"] == 0.0, "Reset signal restores stopped initial state")
	check(preview.timeline.value == 0.0 and preview.status.text.contains("Paused"), "Reset updates visible timeline and status")
	buttons["Play"].pressed.emit()
	check(player.state()["playing"] and player.state()["clip"] == clip_select.get_item_text(clip_select.selected), "Play after reset uses selected clip")
	buttons["Pause"].pressed.emit()
	check(not player.state()["playing"] and preview.status.text.contains("Paused"), "Pause updates player and displayed status")
	preview.queue_free()
	finish()

func finish() -> void:
	if failures.is_empty():
		print("FORGE_PREVIEW_UI_PASS " + JSON.stringify({"ok":true,"checks":checks}))
		quit(0)
	else:
		printerr(JSON.stringify({"ok":false,"failures":failures}))
		quit(1)
'''


def inventory(root):
    return {str(path.relative_to(root)): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in root.rglob("*") if path.is_file()}


def run(args):
    source = args.project.resolve(strict=True)
    output = args.output.resolve()
    godot = args.godot.resolve(strict=True)
    assert (source / "preview.tscn").is_file(), "project must contain the generated preview.tscn"
    assert source != output and source not in output.parents and output not in source.parents, "output must be separate from input project"
    before = inventory(source)
    output.mkdir(parents=True, exist_ok=False)
    project = output / "project"
    shutil.copytree(source, project, ignore=shutil.ignore_patterns(".git", ".godot"))
    (project / "preview_ui_test.gd").write_text(HARNESS, encoding="utf-8")
    flags = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
    try:
        # Only rebuild Godot's cache inside the copy; never repeat Forge installation.
        imported = subprocess.run([str(godot), "--headless", "--path", str(project), "--editor", "--import"],
                                  capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=60, creationflags=flags)
        (output / "import-stdout.txt").write_text(imported.stdout, encoding="utf-8")
        (output / "import-stderr.txt").write_text(imported.stderr, encoding="utf-8")
        assert imported.returncode == 0 and "SCRIPT ERROR" not in imported.stderr, (imported.returncode, imported.stdout, imported.stderr)
        result = subprocess.run([str(godot), "--headless", "--path", str(project), "--script", "res://preview_ui_test.gd"],
                                capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=45, creationflags=flags)
        (output / "stdout.txt").write_text(result.stdout, encoding="utf-8")
        (output / "stderr.txt").write_text(result.stderr, encoding="utf-8")
        prefix = "FORGE_PREVIEW_UI_PASS "
        markers = [line[len(prefix):] for line in result.stdout.splitlines() if line.startswith(prefix)]
        assert result.returncode == 0 and len(markers) == 1 and "SCRIPT ERROR" not in result.stderr, (result.returncode, result.stdout, result.stderr)
        summary = json.loads(markers[0])
        assert summary["ok"] and summary["checks"] >= 20, summary
        summary.update(inputProject=str(source), copiedProject=str(project), inputFilesUnchanged=True,
                       scope="native_preview_control_signals_and_player_state")
        (output / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
        print(json.dumps(summary))
    finally:
        assert inventory(source) == before, "input preview project changed during isolated UI check"


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project", type=Path, required=True)
    parser.add_argument("--godot", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True, help="New QA directory retaining copied project and evidence")
    run(parser.parse_args())
