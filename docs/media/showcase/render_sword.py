#!/usr/bin/env python3
"""Render README GIFs from a local Sword checkout without changing that project.

Requires Godot 4.6.x, FFmpeg and a graphical session. No image generation, Forge
imports, network calls, or extra Python dependencies are used.
"""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

ACTORS = ("fire_fx", "frost_fx", "lightning_fx", "ghost", "golem", "vine", "boss_slam")
FILES = ("sprite_sheet.png", "forge_sprite_frames.tres", "forge_usage.json")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sword", type=Path, required=True)
    parser.add_argument("--godot", default="godot")
    parser.add_argument("--ffmpeg", default="ffmpeg")
    parser.add_argument("--work-dir", type=Path, default=Path("target/qa/sword-showcase"))
    args = parser.parse_args()
    media = Path(__file__).resolve().parent
    args.work_dir.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix="render-", dir=args.work_dir.resolve()))
    specs = json.loads((args.sword / "asset-specs/animation-imports.json").read_text())
    inputs = {}
    hashes = {}
    for spec in specs["actors"]:
        actor = spec["actor"]
        if actor not in ACTORS:
            continue
        key = spec["asset_key"]
        relative = Path("addons/forge_assets") / key
        receipt = json.loads((args.sword / f"docs/qa/animation-imports/{actor}.json").read_text())
        usage = json.loads((args.sword / relative / "forge_usage.json").read_text())
        if receipt["pack_sha256"] != usage["packSha256"] or not receipt["validation"]["valid"]:
            raise ValueError(f"Pack evidence differs: {actor}")
        texture = (relative / "sprite_sheet.png").as_posix()
        if digest(args.sword / texture) != receipt["texture_hashes"][texture]:
            raise ValueError(f"Installed texture differs: {actor}")
        (work / relative).mkdir(parents=True)
        for name in FILES:
            source = args.sword / relative / name
            hashes[(relative / name).as_posix()] = digest(source)
            shutil.copyfile(source, work / relative / name)
        inputs[actor] = key
    if set(inputs) != set(ACTORS):
        raise ValueError("Missing requested animation assets")
    (work / "inputs.json").write_text(json.dumps(inputs))
    shutil.copyfile(media / "render_sword.gd", work / "render_sword.gd")
    (work / "project.godot").write_text('''config_version=5
[application]
config/name="Sword Asset Preview"
run/main_scene="res://main.tscn"
config/features=PackedStringArray("4.6", "GL Compatibility")
[display]
window/size/viewport_width=1040
window/size/viewport_height=440
[rendering]
renderer/rendering_method="gl_compatibility"
environment/defaults/default_clear_color=Color(0.067, 0.122, 0.106, 1)
''')
    (work / "main.tscn").write_text('''[gd_scene load_steps=2 format=3]
[ext_resource type="Script" path="res://render_sword.gd" id="1"]
[node name="SwordAssetPreview" type="Control"]
script = ExtResource("1")
''')

    def run(command, logfile):
        with (work / logfile).open("w") as output:
            subprocess.run(command, stdout=output, stderr=subprocess.STDOUT, check=True, timeout=180)

    run([args.godot, "--headless", "--path", str(work), "--editor", "--import", "--quit"], "import.log")
    results = {}
    for mode in ("spells", "enemies"):
        frames = work / mode
        frames.mkdir()
        run([args.godot, "--path", str(work), "--", mode, str(frames)], f"{mode}.log")
        gif = media / f"sword-{mode}.gif"
        palette_filter = (
            "[0:v]split[a][b];[a]palettegen=max_colors=256:stats_mode=full[p];"
            "[b][p]paletteuse=dither=bayer:bayer_scale=3:diff_mode=rectangle"
        )
        run([args.ffmpeg, "-y", "-framerate", "50", "-i", str(frames / "%04d.png"),
             "-filter_complex", palette_filter, "-loop", "0", str(gif)], f"{mode}-encode.log")
        results[mode] = {"sha256": digest(gif), "bytes": gif.stat().st_size,
                         "frames": len(list(frames.glob("*.png"))), "capture_fps": 50}
    for relative, expected in hashes.items():
        if digest(args.sword / relative) != expected:
            raise ValueError(f"Consumer resource changed during render: {relative}")
    evidence = {"inputs_sha256": hashes, "outputs": results, "consumer_resources_unchanged": True}
    (work / "render-result.json").write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps({"work_dir": str(work), **evidence}, indent=2))


if __name__ == "__main__":
    main()
