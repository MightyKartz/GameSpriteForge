#!/usr/bin/env python3
"""Render layered assets with Godot's real compatibility renderer and a standalone PCK.

Test artifacts are written to a new --output directory. --project is optional: without it this
runs the synthetic blend/transform oracle alone. A supplied preview project and
optional original artwork are copied into this isolated test, never modified.
The PCK is a self-contained resource pack, not a published or signed application.
Python uses only the standard library; Godot saves PNG and raw RGBA for comparison.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import time
import zlib


RENDER_SCRIPT = r'''extends SceneTree

var output := ""
var images := {}

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	output = OS.get_cmdline_user_args()[0]
	if DisplayServer.get_name() == "headless":
		push_error("A real rendering display is required; headless screenshots are not accepted")
		quit(2)
		return
	var config: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://qa-config.json"))
	var fixture: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://synthetic/manifest.json"))
	await _capture(load("res://synthetic/layered.tscn").instantiate(), "synthetic-initial", fixture, "", 0.0, true)
	await _capture(_reference(fixture, "res://synthetic", -1.0), "synthetic-reference", fixture)
	await _capture(load("res://synthetic/layered.tscn").instantiate(), "synthetic-midpoint", fixture, "move", 0.5)
	var expected := _reference(fixture, "res://synthetic", -1.0)
	# Independent known midpoint oracle, not sampled through the production player.
	var moving := expected.get_node("overlay_normal") as Node2D
	moving.position = Vector2(24.0, 34.5)
	moving.rotation_degrees = 30.0
	moving.scale = Vector2(1.1, 0.9)
	moving.modulate.a = 0.6
	await _capture(expected, "synthetic-midpoint-reference", fixture)
	var probes := _multiply_probes()
	await _capture(probes["node"], "multiply-probes", {"canvas": {"width": probes["cases"].size() * 4, "height": 4}})
	if config.get("consumer", false):
		var manifest: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://consumer/manifest.json"))
		await _capture(load("res://consumer/layered.tscn").instantiate(), "consumer-initial", manifest, "", 0.0, true)
		await _capture(_reference(manifest, "res://consumer", -1.0), "consumer-reference", manifest)
		if not manifest.get("clips", []).is_empty():
			var clip: Dictionary = manifest["clips"][0]
			var midpoint := float(clip["durationMs"]) / 2000.0
			await _capture(load("res://consumer/layered.tscn").instantiate(), "consumer-midpoint", manifest, String(clip["id"]), midpoint)
			await _capture(_reference(manifest, "res://consumer", midpoint * 1000.0), "consumer-midpoint-reference", manifest)
		if config.get("original", false):
			var original := Sprite2D.new()
			original.centered = false
			original.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST if manifest["sampling"] == "nearest" else CanvasItem.TEXTURE_FILTER_LINEAR
			original.texture = load("res://consumer/original.png")
			await _capture(original, "consumer-original", manifest)
	var report := {"displayDriver": DisplayServer.get_name(), "adapter": RenderingServer.get_video_adapter_name(),
		"renderingMethod": ProjectSettings.get_setting("rendering/renderer/rendering_method"), "images": images, "multiplyCases": probes["cases"]}
	var file := FileAccess.open(output.path_join("render.json"), FileAccess.WRITE)
	file.store_string(JSON.stringify(report, "  "))
	file.close()
	print("RENDER_QA_COMPLETE")
	quit(0)

func _capture(node: Node2D, label: String, manifest: Dictionary, clip: String = "", seconds: float = 0.0, reset: bool = false) -> void:
	var viewport := SubViewport.new()
	viewport.size = Vector2i(int(manifest["canvas"]["width"]), int(manifest["canvas"]["height"]))
	viewport.transparent_bg = true
	viewport.disable_3d = true
	viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	root.add_child(viewport)
	viewport.add_child(node)
	node.set_process(false)
	if reset:
		node.call("reset_pose")
	elif not clip.is_empty():
		if not node.call("play", clip):
			push_error("Player could not select clip " + clip)
			quit(3)
			return
		node.call("seek", seconds)
		node.call("pause")
	await process_frame
	await RenderingServer.frame_post_draw
	await process_frame
	await RenderingServer.frame_post_draw
	var image := viewport.get_texture().get_image()
	image.convert(Image.FORMAT_RGBA8)
	if image.is_empty() or image.save_png(output.path_join(label + ".png")) != OK:
		push_error("Failed to capture " + label)
		quit(4)
		return
	var raw := FileAccess.open(output.path_join(label + ".rgba"), FileAccess.WRITE)
	raw.store_buffer(image.get_data())
	raw.close()
	images[label] = {"width": image.get_width(), "height": image.get_height(), "format": "rgba8"}
	viewport.queue_free()
	await process_frame

func _reference(manifest: Dictionary, base: String, time_ms: float) -> Node2D:
	var reference := Node2D.new()
	var values := {}
	for layer in manifest["layers"]:
		values[String(layer["id"])] = layer["transform"]
	if time_ms >= 0.0 and not manifest.get("clips", []).is_empty():
		for track in manifest["clips"][0]["tracks"]:
			values[String(track["layerId"])] = _interpolate(track["keyframes"], time_ms)
	for layer in manifest["layers"]:
		var transform: Dictionary = values[String(layer["id"])]
		var pivot := Vector2(float(layer["pivot"][0]), float(layer["pivot"][1]))
		var node := Node2D.new()
		node.name = String(layer["id"])
		node.position = pivot + Vector2(float(transform["position"][0]), float(transform["position"][1]))
		node.rotation_degrees = float(transform["rotationDegrees"])
		node.scale = Vector2(float(transform["scale"][0]), float(transform["scale"][1]))
		node.modulate.a = float(transform["opacity"])
		var sprite := Sprite2D.new()
		sprite.centered = false
		sprite.position = -pivot
		sprite.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST if manifest["sampling"] == "nearest" else CanvasItem.TEXTURE_FILTER_LINEAR
		sprite.texture = load(base.path_join("layers").path_join(String(layer["texture"]).get_file()))
		if layer["blend"] == "multiply":
			var material := ShaderMaterial.new()
			material.shader = load(base.path_join("forge_alpha_multiply.gdshader"))
			sprite.material = material
		else:
			var material := CanvasItemMaterial.new()
			material.blend_mode = CanvasItemMaterial.BLEND_MODE_ADD if layer["blend"] == "add" else CanvasItemMaterial.BLEND_MODE_MIX
			sprite.material = material
		node.add_child(sprite)
		reference.add_child(node)
	return reference

func _multiply_probes() -> Dictionary:
	var node := Node2D.new()
	var cases := []
	for destination_alpha in [0, 128, 255]:
		for setting in [[-1, 1.0, [180,100,60]], [0, 1.0, [0,0,0]], [0, 1.0, [255,255,255]],
			[128, 1.0, [180,100,60]], [255, 1.0, [180,100,60]], [128, 0.25, [180,100,60]],
			[255, 0.5, [180,100,60]], [255, 1.0, [255,255,255]], [0, 1.0, [180,100,60]]]:
			var location := Vector2(cases.size() * 4, 0)
			var background := _color_sprite(Color(40.0/255.0,70.0/255.0,100.0/255.0,float(destination_alpha)/255.0))
			background.position = location
			node.add_child(background)
			if setting[0] >= 0:
				var tint := Node2D.new()
				tint.position = location
				tint.modulate.a = setting[1]
				var rgb: Array = setting[2]
				var foreground := _color_sprite(Color(float(rgb[0])/255.0,float(rgb[1])/255.0,float(rgb[2])/255.0,float(setting[0])/255.0))
				var material := ShaderMaterial.new()
				material.shader = load("res://synthetic/forge_alpha_multiply.gdshader")
				foreground.material = material
				tint.add_child(foreground)
				node.add_child(tint)
			cases.append({"destinationAlpha":destination_alpha,"sourceAlpha":setting[0],"opacity":setting[1],"sourceRGB":setting[2]})
	return {"node":node,"cases":cases}

func _color_sprite(color: Color) -> Sprite2D:
	var image := Image.create(4,4,false,Image.FORMAT_RGBA8)
	image.fill(color)
	var sprite := Sprite2D.new()
	sprite.centered = false
	sprite.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	sprite.texture = ImageTexture.create_from_image(image)
	return sprite

func _interpolate(keys: Array, time_ms: float) -> Dictionary:
	for index in range(1, keys.size()):
		if time_ms <= float(keys[index]["timeMs"]):
			var previous: Dictionary = keys[index - 1]
			var next: Dictionary = keys[index]
			var fraction := (time_ms - float(previous["timeMs"])) / float(next["timeMs"] - previous["timeMs"])
			var result := {}
			for name in ["rotationDegrees", "opacity"]:
				result[name] = float(previous["transform"][name]) * (1.0 - fraction) + float(next["transform"][name]) * fraction
			for name in ["position", "scale"]:
				result[name] = []
				for axis in range(2):
					result[name].append(float(previous["transform"][name][axis]) * (1.0 - fraction) + float(next["transform"][name][axis]) * fraction)
			return result
	return keys[-1]["transform"]
'''

PACK_SCRIPT = r'''extends SceneTree
func _initialize() -> void:
	var output: String = OS.get_cmdline_user_args()[0]
	var packer := PCKPacker.new()
	if packer.pck_start(output) != OK:
		quit(2)
		return
	var paths: Array[String] = []
	_collect("res://", paths)
	_collect("res://.godot/imported", paths)
	paths.sort()
	for path in paths:
		if packer.add_file(path, ProjectSettings.globalize_path(path)) != OK:
			push_error("PCK add failed: " + path)
			quit(3)
			return
	if packer.flush() != OK:
		quit(4)
		return
	print("PCK_QA_COMPLETE ", paths.size())
	quit(0)

func _collect(folder: String, paths: Array[String]) -> void:
	var directory := DirAccess.open(folder)
	if directory == null:
		return
	for name in directory.get_files():
		if name != "pack.gd" and not name.ends_with(".uid"):
			paths.append(folder.path_join(name))
	for name in directory.get_directories():
		if name != ".godot":
			_collect(folder.path_join(name), paths)
'''


def write_text(path, text):
    path.write_text(text, encoding="utf-8", newline="\n")


def write_json(path, value):
    write_text(path, json.dumps(value, indent=2, ensure_ascii=False))


def png(path, width, height, pixel):
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    rows = b"".join(b"\0" + b"".join(bytes(pixel(x, y)) for x in range(width)) for y in range(height))
    path.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b""))


def synthetic(project):
    base = project / "synthetic"
    (base / "layers").mkdir(parents=True)
    runtime = Path(__file__).resolve().parents[1] / "scripts/godot/runtime/layered-player-v1.gd"
    shutil.copy2(runtime, base / "forge_layered_player.gd")
    shutil.copy2(runtime.with_name("alpha-multiply-v1.gdshader"), base / "forge_alpha_multiply.gdshader")
    identity = {"position": [0, 0], "rotationDegrees": 0, "scale": [1, 1], "opacity": 1}
    layers = []
    for index, (name, blend) in enumerate([("background", "normal"), ("overlay_normal", "normal"), ("overlay_add", "add"), ("overlay_multiply", "multiply")]):
        source = base / "layers" / (name + ".png")
        x0 = (index - 1) * 50 + 8
        def pixel(x, y, i=index, start=x0):
            if i == 0:
                return (40, 70, 100, 255)
            return (180, 100, 60, 128) if start <= x < start + 24 and 20 <= y < 44 else (0, 0, 0, 0)
        png(source, 160, 72, pixel)
        layers.append({"id": name, "name": name, "texture": "assets/layers/" + source.name,
                       "sha256": hashlib.sha256(source.read_bytes()).hexdigest(), "pivot": [20 if index < 2 else x0 + 12, 32],
                       "transform": identity.copy(), "blend": blend})
    manifest = {"schemaVersion": "1", "assetType": "layered", "id": "render_fixture", "name": "Render fixture",
                "canvas": {"width": 160, "height": 72, "origin": [0, 0]}, "sampling": "nearest", "layers": layers,
                "defaultClip": "move", "clips": [{"id": "move", "durationMs": 1000, "loop": False, "tracks": [{
                    "layerId": "overlay_normal", "keyframes": [{"timeMs": 0, "transform": identity},
                    {"timeMs": 1000, "transform": {"position": [8, 5], "rotationDegrees": 60, "scale": [1.2, 0.8], "opacity": 0.2}}]}]}]}
    write_json(base / "manifest.json", manifest)
    scene = ['[gd_scene load_steps=10 format=3]', '[ext_resource type="Script" path="forge_layered_player.gd" id="player"]', '[ext_resource type="Shader" path="forge_alpha_multiply.gdshader" id="shader"]']
    for index, layer in enumerate(layers):
        scene.append(f'[ext_resource type="Texture2D" path="layers/{layer["id"]}.png" id="tex_{index}"]')
    for name, mode in [("normal", 0), ("add", 1)]:
        scene.append(f'[sub_resource type="CanvasItemMaterial" id="{name}"]\nblend_mode = {mode}')
    scene.append('[sub_resource type="ShaderMaterial" id="multiply"]\nshader = ExtResource("shader")')
    scene.append('[node name="Fixture" type="Node2D"]\nscript = ExtResource("player")\n[node name="Layers" type="Node2D" parent="."]')
    for index, layer in enumerate(layers):
        x, y = layer["pivot"]
        scene.append(f'[node name="{layer["id"]}" type="Node2D" parent="Layers"]\nposition = Vector2({x}, {y})\n'
                     f'[node name="Sprite" type="Sprite2D" parent="Layers/{layer["id"]}"]\nposition = Vector2({-x}, {-y})\n'
                     f'texture_filter = 1\ncentered = false\ntexture = ExtResource("tex_{index}")\nmaterial = SubResource("{layer["blend"]}")')
    write_text(base / "layered.tscn", "\n\n".join(scene) + "\n")


def safe_file(path):
    if path.is_symlink() or getattr(path, "is_junction", lambda: False)() or not path.is_file():
        raise ValueError(f"Expected regular input file: {path}")
    return path


def copy_consumer(source, project, reference):
    asset = source / "addons/forge_assets/preview"
    target = project / "consumer"
    (target / "layers").mkdir(parents=True)
    manifest = json.loads(safe_file(asset / "manifest.json").read_text(encoding="utf-8"))
    hashes = {}
    for name in ["manifest.json", "layered.tscn", "forge_layered_player.gd"]:
        path = safe_file(asset / name)
        shutil.copy2(path, target / name)
        hashes[name] = hashlib.sha256(path.read_bytes()).hexdigest()
    if (asset / "forge_alpha_multiply.gdshader").exists():
        path = safe_file(asset / "forge_alpha_multiply.gdshader")
        shutil.copy2(path, target / path.name)
        hashes[path.name] = hashlib.sha256(path.read_bytes()).hexdigest()
    for layer in manifest["layers"]:
        name = Path(layer["texture"]).name
        if name != layer["id"] + ".png" or "/" in name or "\\" in name:
            raise ValueError("Unsafe layer path")
        path = safe_file(asset / "layers" / name)
        data = path.read_bytes()
        digest = hashlib.sha256(data).hexdigest()
        if digest != layer["sha256"]:
            raise ValueError(f"Source layer hash mismatch: {name}")
        shutil.copy2(path, target / "layers" / name)
        hashes["layers/" + name] = digest
    if reference:
        shutil.copy2(safe_file(reference), target / "original.png")
        hashes["original.png"] = hashlib.sha256(reference.read_bytes()).hexdigest()
    return hashes


def run(command, label, output, deadline, marker=None):
    options = {}
    if os.name == "nt":
        startup = subprocess.STARTUPINFO()
        startup.dwFlags |= subprocess.STARTF_USESHOWWINDOW
        startup.wShowWindow = subprocess.SW_HIDE
        options.update(startupinfo=startup, creationflags=subprocess.CREATE_NO_WINDOW)
    remaining = deadline - time.monotonic()
    if remaining <= 0:
        raise TimeoutError("Render QA deadline exceeded")
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, **options)
    try:
        data, _ = process.communicate(timeout=remaining)
    except subprocess.TimeoutExpired:
        process.kill()
        data, _ = process.communicate()
        (output / (label + ".log")).write_bytes(data)
        raise TimeoutError(f"{label} exceeded render QA deadline; Godot was terminated")
    text = data.decode("utf-8", errors="replace")
    write_text(output / (label + ".log"), text)
    if process.returncode or re.search(r"SCRIPT ERROR|(?:^|\n)ERROR:", text) or (marker and marker not in text):
        raise RuntimeError(f"{label} failed; see {output / (label + '.log')}")


def compare(folder, actual, expected, threshold=2):
    a = (folder / (actual + ".rgba")).read_bytes()
    b = (folder / (expected + ".rgba")).read_bytes()
    if len(a) != len(b) or not a:
        raise ValueError(f"Image byte lengths differ: {actual}, {expected}")
    maximum = 0
    outside = 0
    changed = 0
    for i in range(0, len(a), 4):
        # RGB where both alphas are zero is invisible and not a visual mismatch.
        delta = max(abs(a[i + c] - b[i + c]) for c in (range(4) if a[i+3] or b[i+3] else (3,)))
        maximum = max(maximum, delta)
        changed += delta != 0
        outside += delta > threshold
    return {"actual": actual, "expected": expected, "maxChannelDelta": maximum,
            "changedPixels": changed, "pixelsAboveTolerance": outside, "tolerance": threshold, "passed": outside == 0}


def multiply_proof(folder, report):
    data = (folder / "multiply-probes.rgba").read_bytes()
    width = report["images"]["multiply-probes"]["width"]
    baselines, results = {}, []
    for index, case in enumerate(report["multiplyCases"]):
        start = (2 * width + index * 4 + 2) * 4
        actual = list(data[start:start + 4])
        if case["sourceAlpha"] < 0:
            baselines[case["destinationAlpha"]] = actual
            continue
        base = baselines[case["destinationAlpha"]]
        alpha = case["sourceAlpha"] / 255 * case["opacity"]
        expected = [round(base[c] * (1 - alpha + case["sourceRGB"][c] / 255 * alpha)) for c in range(3)] + [base[3]]
        maximum = max(abs(a - b) for a, b in zip(actual, expected))
        results.append({**case, "actual": actual, "expected": expected, "maxChannelDelta": maximum,
                        "destinationAlphaPreserved": actual[3] == base[3], "passed": maximum <= 2 and actual[3] == base[3]})
    return results


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project", type=Path)
    parser.add_argument("--godot", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--reference", type=Path)
    parser.add_argument("--timeout", type=float, default=240)
    args = parser.parse_args()
    if args.reference and not args.project:
        parser.error("--reference requires --project")
    output = args.output.resolve()
    if args.project and output.is_relative_to(args.project.resolve()):
        parser.error("output cannot be inside the supplied source project")
    output.mkdir(parents=True, exist_ok=False)
    project = output / "project"
    project.mkdir()
    native, packed = output / "native", output / "packed"
    native.mkdir()
    packed.mkdir()
    deadline = time.monotonic() + args.timeout
    synthetic(project)
    hashes = copy_consumer(args.project.resolve(), project, args.reference.resolve() if args.reference else None) if args.project else {}
    write_text(project / "project.godot", 'config_version=5\n[application]\nconfig/name="Forge layered GPU QA"\n[display]\nwindow/size/viewport_width=64\nwindow/size/viewport_height=64\nwindow/size/window_width_override=64\nwindow/size/window_height_override=64\n[rendering]\nrenderer/rendering_method="gl_compatibility"\nrenderer/rendering_method.mobile="gl_compatibility"\ntextures/default_filters/use_nearest_mipmap_filter=false\n')
    write_json(project / "qa-config.json", {"consumer": bool(args.project), "original": bool(args.reference)})
    write_text(project / "render.gd", RENDER_SCRIPT)
    write_text(project / "pack.gd", PACK_SCRIPT)
    godot = str(args.godot.resolve())
    run([godot, "--headless", "--editor", "--path", str(project), "--import", "--quit"], "import", output, deadline)
    render_args = ["--rendering-method", "gl_compatibility", "--rendering-driver", "opengl3", "--position", "-20000,-20000", "--resolution", "64x64", "--script", "res://render.gd"]
    run([godot, "--path", str(project), *render_args, "--", str(native)], "native-render", output, deadline, "RENDER_QA_COMPLETE")
    pck = output / "layered-render-qa.pck"
    run([godot, "--headless", "--path", str(project), "--script", "res://pack.gd", "--", str(pck)], "pack", output, deadline, "PCK_QA_COMPLETE")
    # Launch without --path in an empty working project context. All resource
    # reads must resolve from the PCK, including imported GPU texture data.
    hidden_project = output / "project-hidden-during-pck"
    if project.parent != output or hidden_project.parent != output or hidden_project.exists():
        raise ValueError("Unexpected isolated project rename target")
    project.rename(hidden_project)
    try:
        run([godot, "--main-pack", str(pck), *render_args, "--", str(packed)], "packed-render", output, deadline, "RENDER_QA_COMPLETE")
    finally:
        hidden_project.rename(project)
    comparisons = []
    for folder in [native, packed]:
        report = json.loads((folder / "render.json").read_text())
        if report["displayDriver"] == "headless" or not report["adapter"] or "dummy" in report["adapter"].lower():
            raise RuntimeError("No effective GPU/compatibility renderer evidence")
        pairs = [("synthetic-initial", "synthetic-reference"), ("synthetic-midpoint", "synthetic-midpoint-reference")]
        if args.project:
            pairs.append(("consumer-initial", "consumer-reference"))
            if "consumer-midpoint" in report["images"]:
                pairs.append(("consumer-midpoint", "consumer-midpoint-reference"))
            if args.reference:
                pairs.append(("consumer-initial", "consumer-original"))
        for actual, reference in pairs:
            comparison = compare(folder, actual, reference)
            comparison["run"] = folder.name
            comparisons.append(comparison)
    native_report = json.loads((native / "render.json").read_text())
    packaged_report = json.loads((packed / "render.json").read_text())
    resource_parity = all((native / (name + ".rgba")).read_bytes() == (packed / (name + ".rgba")).read_bytes() for name in native_report["images"])
    initial = (native / "synthetic-initial.rgba").read_bytes()
    colors = [list(initial[(32 * 160 + x) * 4:(32 * 160 + x) * 4 + 4]) for x in [20, 70, 120]]
    blends_distinct = len({tuple(color) for color in colors}) == 3 and all(color[3] > 0 for color in colors)
    movement = compare(native, "synthetic-initial", "synthetic-midpoint", threshold=0)
    source_unchanged = True
    if args.project:
        asset = args.project.resolve() / "addons/forge_assets/preview"
        for name, digest in hashes.items():
            source = args.reference.resolve() if name == "original.png" else asset / name
            source_unchanged &= hashlib.sha256(source.read_bytes()).hexdigest() == digest
    probes = {"native": multiply_proof(native, native_report), "packed": multiply_proof(packed, packaged_report)}
    probe_success = all(case["passed"] for cases in probes.values() for case in cases)
    result = {"ok": all(c["passed"] for c in comparisons) and resource_parity and blends_distinct and movement["changedPixels"] > 0 and source_unchanged and probe_success,
              "scope": "native_gpu_and_self_contained_pck_render", "native": native_report, "packaged": packaged_report,
              "comparisons": comparisons, "nativePckPixelParity": resource_parity,
              "sourceProjectHiddenDuringPck": True,
              "alphaAwareMultiplyCases": probes, "alphaAwareMultiplyPassed": probe_success,
              "blendSampleColors": dict(zip(["normal", "add", "multiply"], colors)), "blendModesVisuallyDistinct": blends_distinct,
              "syntheticMotionChangedPixels": movement["changedPixels"], "sourceFilesUnchanged": source_unchanged,
              "sourceHashes": hashes, "pckSha256": hashlib.sha256(pck.read_bytes()).hexdigest(),
              "limitations": ["This is a PCK resource-pack render test using the supplied Godot executable, not an EXE export or distribution approval.",
                              "Tolerant native/reference comparisons allow at most 2 channel levels; native/PCK screenshot bytes must match exactly."]}
    write_json(output / "result.json", result)
    print(json.dumps({"ok": result["ok"], "result": str(output / "result.json"), "adapter": native_report["adapter"], "comparisons": len(comparisons), "nativePckPixelParity": resource_parity}))
    raise SystemExit(0 if result["ok"] else 1)


if __name__ == "__main__":
    main()
