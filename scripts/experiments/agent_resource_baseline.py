#!/usr/bin/env python3
"""M0 resource-task inventory using a selected CLI and isolated native Godot.

This is a technical observation, not a productivity benchmark or artistic review.
Creates reusable inputs, engine-independent acceptance.json, and Forge requests.
Raw logs and stores remain under a NEW --output directory; no Provider calls.
"""
import argparse
import copy
import hashlib
import json
import math
import os
from pathlib import Path
import shutil
import struct
import subprocess
import time
import wave

from PIL import Image, ImageDraw

REPO = Path(__file__).resolve().parents[2]
CHECKER = Path(__file__).with_name("check-agent-resources.gd")


def save(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def require_native(result, count):
    require(result.returncode == 0
            and f"RESOURCE_TASK_PASS:{count}" in result.stdout.splitlines()
            and "SCRIPT ERROR" not in result.stdout + result.stderr
            and "FAIL:" not in result.stdout + result.stderr,
            "Native resource acceptance failed; see command logs")


def make_inputs(root):
    inputs = root / "inputs"
    inputs.mkdir()
    requests = root / "requests"
    requests.mkdir()
    contract = {"schemaVersion": 1, "characters": [], "static": [], "audio": []}
    recipes = {}
    for name, size, anchor, actions in [
        ("synthetic", 64, [32, 52], [("idle", [70, 150, 230], True),
                                     ("walk", [80, 120, 160], True),
                                     ("attack", [60, 240, 100], False)]),
        ("thunder", 627, [313, 580], [("idle", [200] * 4, True),
                                     ("move", [143] * 4, True)]),
    ]:
        expected = {"id": name, "size": [size, size], "anchor": anchor, "actions": []}
        recipe = {"schemaVersion": "2", "metadata": {"name": name, "defaultAnimation": actions[-1][0]},
                  "normalize": {"mode": "preserve_source", "margin": 0, "marginBottom": 0,
                                "alphaThreshold": 0, "manualAnchor": {
                                    "x": anchor[0], "y": anchor[1], "lockedByUser": True}},
                  "rendering": {"textureFilter": "nearest", "pixelSnap": True},
                  # Explicit prototype from the start; never downgrade a failed gate.
                  "quality": {"requireGameReady": False}, "animations": []}
        for action, durations, loop in actions:
            frames = []
            if name == "thunder":
                source = REPO / f"docs/media/showcase/thunder/sources/thunder-{action}.png"
                target = inputs / source.name
                shutil.copyfile(source, target)
                image = Image.open(target).convert("RGBA")
                require(image.size == (1254, 1254), "Public thunder source dimensions changed")
                frames = [image.crop((i % 2 * size, i // 2 * size,
                                      (i % 2 + 1) * size, (i // 2 + 1) * size)) for i in range(4)]
            else:
                for index in range(3):
                    frame = Image.new("RGBA", (size, size))
                    draw = ImageDraw.Draw(frame)
                    draw.rectangle((23, 15, 39, 51), fill=(150, 80, 190, 255))
                    reach = 5 + index * (6 if action == "attack" else 2)
                    draw.line((39, 30, 39 + reach, 30), fill=(40, 190, 220, 128), width=2)
                    frame.putpixel((22, 20), (120, 50, 210, 1))
                    frame.putpixel((30, 25), (20 + index * 40, 110, 60, 255))
                    frames.append(frame)
            paths = []
            for index, frame in enumerate(frames):
                path = inputs / f"{name}-{action}-{index}.png"
                frame.save(path)
                paths.append(path)
            if name == "thunder" or action == "walk":
                if name != "thunder":
                    target = inputs / f"{name}-{action}-sheet.png"
                    sheet = Image.new("RGBA", (size * len(frames), size))
                    for index, frame in enumerate(frames):
                        sheet.paste(frame, (index * size, 0))
                    sheet.save(target)
                columns, rows = (2, 2) if name == "thunder" else (3, 1)
                source_input = {"kind": "sprite_sheet", "path": f"../inputs/{target.name}",
                                "split": {"mode": "fixed_grid", "frameWidth": size,
                                          "frameHeight": size, "columns": columns, "rows": rows}}
            else:
                source_input = {"kind": "png_sequence", "paths": [f"../inputs/{p.name}" for p in paths]}
            recipe["animations"].append({"name": action, "input": source_input, "fps": 10,
                                          "loop": loop, "frameDurationsMs": durations,
                                          "matting": {"mode": "preserve_alpha"}})
            expected["actions"].append({"name": action, "durationsMs": durations, "loop": loop,
                                         "frames": [str(p.relative_to(root)) for p in paths]})
        recipes[name] = ("prepare-character", recipe)
        contract["characters"].append(expected)

    static_path = inputs / "prop.png"
    shutil.copyfile(inputs / "synthetic-idle-0.png", static_path)
    contract["static"].append({"id": "prop", "source": "inputs/prop.png", "size": [64, 64]})
    recipes["props"] = ("prepare-static", {
        "schemaVersion": "1", "kind": "prop_set", "id": "props", "name": "M0 prop",
        "license": "MIT synthetic fixture", "sampling": "nearest", "canvasPolicy": "preserve_source",
        "items": [{"id": "prop", "name": "prop", "path": "../inputs/prop.png"}]})
    for name, role, loop, seconds in [("music", "music", True, 1.0), ("cue", "sfx", False, 0.2)]:
        path = inputs / f"{name}.wav"
        with wave.open(str(path), "wb") as stream:
            stream.setparams((1, 2, 22050, 0, "NONE", "not compressed"))
            stream.writeframes(b"".join(struct.pack("<h", round(3000 * math.sin(2 * math.pi * 440 * i / 22050)))
                                       for i in range(round(22050 * seconds))))
        contract["audio"].append({"id": name, "sampleRate": 22050, "channels": 1,
                                   "frames": round(22050 * seconds), "loop": loop,
                                   "source": f"inputs/{name}.wav", "role": role})
    recipes["audio"] = ("prepare-audio", {
        "schemaVersion": "1", "id": "audio", "name": "Synthetic music and SFX roles",
        "sampleRate": 22050, "channels": 1,
        "items": [{"id": a["id"], "path": "../" + a["source"], "role": a["role"], "loop": a["loop"]}
                  for a in contract["audio"]]})
    for name, (_, recipe) in recipes.items():
        save(requests / f"{name}.json", recipe)
    save(root / "acceptance.json", contract)
    return recipes, contract


def run(args):
    root = args.output.absolute()
    root.mkdir(parents=True, exist_ok=False)
    (root / "logs").mkdir()
    forge, godot = args.forge.absolute(), args.godot.absolute()
    env = dict(os.environ, FORGE_JOB_STORE=str(root / "jobs"), FORGE_PLAN_STORE=str(root / "plans"),
               FORGE_CACHE_STORE=str(root / "cache"), FORGE_CONFIG_DIR=str(root / "config"),
               FORGE_GODOT_PATH=str(godot), FORGE_REAL_PROVIDER_MAX_REQUESTS="0")
    env.pop("FORGE_REAL_PROVIDER_ACCEPT", None)
    report = {"schemaVersion": 1, "ok": False, "calls": [], "results": {}, "rejections": {},
              "visualReview": "not_assessed", "listeningReview": "not_assessed",
              "humanSeconds": None, "modelCost": None, "productivityDecision": "insufficient_evidence"}

    def command(label, argv):
        started = time.perf_counter()
        row = {"label": label, "argv": list(map(str, argv))}
        report["calls"].append(row)
        try:
            p = subprocess.run(row["argv"], cwd=root, env=env, capture_output=True, text=True, timeout=240)
            row["exitCode"] = p.returncode
            save(root / "logs" / f"{len(report['calls']):03d}-{label}.json",
                 {"stdout": p.stdout, "stderr": p.stderr, "exitCode": p.returncode})
            return p
        except subprocess.TimeoutExpired as error:
            row["timedOut"] = True
            def decoded(value):
                return value.decode("utf-8", errors="replace") if isinstance(value, bytes) else value
            save(root / "logs" / f"{len(report['calls']):03d}-{label}.json",
                 {"stdout": decoded(error.stdout), "stderr": decoded(error.stderr), "timedOut": True})
            raise
        finally:
            row["seconds"] = round(time.perf_counter() - started, 4)

    def cli(label, *argv, reject=False):
        p = command(label, [forge, *argv, "--json"])
        value = json.loads(p.stdout)
        require(value.get("ok") is (not reject) and (p.returncode != 0) == reject,
                f"Unexpected CLI result: {label}")
        return value["error"] if reject else value["data"]

    def execute(label, kind, path):
        plan = cli(label + "-plan", "plan", kind, "--request", path)
        require(plan["estimate"]["providerRequestEstimate"] == plan["estimate"]["maximumProviderRequests"] == 0,
                "Offline plan must have zero Provider requests")
        job = cli(label + "-execute", "plan", "execute", "--token", plan["token"], "--wait")
        require(job["lifecycle_state"] == "succeeded", f"Job failed: {label}")
        return job

    try:
        report["runnerSha256"] = digest(Path(__file__))
        report["checkerSha256"] = digest(CHECKER)
        report["forgeSha256"] = digest(forge)
        report["doctor"] = cli("doctor", "doctor")
        p = command("godot-version", [godot, "--version"])
        require(p.returncode == 0, "Godot version query failed")
        report["godotVersion"] = p.stdout.strip()
        recipes, contract = make_inputs(root)
        hashes = {p.name: digest(p) for p in (root / "inputs").iterdir()}
        report["sourceSha256"] = hashes
        report["acceptanceSha256"] = digest(root / "acceptance.json")

        # Observe error actionability without prescribing current error wording.
        for name in ["duration-count", "missing-frame"]:
            bad = copy.deepcopy(recipes["synthetic"][1])
            action = bad["animations"][2]
            if name == "duration-count":
                action["frameDurationsMs"] = [60, 240]
            else:
                action["input"]["paths"][1] = "../inputs/missing.png"
            path = root / "requests" / f"bad-{name}.json"
            save(path, bad)
            error = cli(name, "plan", "prepare-character", "--request", path, reject=True)
            report["rejections"][name] = error

        game = root / "game"
        game.mkdir()
        (game / "project.godot").write_text('config_version=5\n[application]\nconfig/name="Agent resource M0"\n', encoding="utf-8")
        for name, (kind, _) in recipes.items():
            job = execute(name, kind, root / "requests" / f"{name}.json")
            quality = cli(name + "-report", "job", "report", "--id", job["job_id"])
            require(quality["providerRequestOccurred"] is False and quality["providerRequestCount"] == 0,
                    "Local preparation unexpectedly reports Provider usage")
            pack = Path(next(a["path"] for a in job["artifacts"] if a["kind"] == "gsfpack"))
            cli(name + "-validate", "pack", "validate", "--path", pack)
            manifest = json.loads((pack / "assets/manifest.json").read_text())
            if name in ("synthetic", "thunder"):
                expected = next(c for c in contract["characters"] if c["id"] == name)
                # Compare actual exported frame PNGs with independent source crops.
                for action in manifest["animations"]:
                    expected_action = next(a for a in expected["actions"] if a["name"] == action["name"])
                    require(action["frameDurationsMs"] == expected_action["durationsMs"], "Pack timing mismatch")
                outputs = sorted((pack / "assets/frames").glob("*.png"))
                # Export order follows manifest order (default action is first).
                sources = [root / p for action in manifest["animations"]
                           for a in expected["actions"] if a["name"] == action["name"] for p in a["frames"]]
                require(len(outputs) == len(sources), "Missing or unexpected exported frames")
                for output, source in zip(outputs, sources):
                    require(Image.open(output).convert("RGBA").tobytes() == Image.open(source).convert("RGBA").tobytes(),
                            "Exported RGBA differs from source: " + source.name)
            install_path = root / "requests" / f"install-{name}.json"
            save(install_path, {"schemaVersion": "1", "packPath": str(pack), "projectPath": str(game),
                                "target": f"addons/forge_assets/{name}", "assetKey": name, "providerRefs": []})
            installed = execute(name + "-install", "install-godot", install_path)
            report["results"][name] = {"jobId": job["job_id"], "installJobId": installed["job_id"],
                                        "packManifestSha256": digest(pack / "forgepack.json"),
                                        "qualityReport": quality, "nativeAcceptance": False}

        # The adapter contains Forge paths. Acceptance values remain independent.
        native_contract = copy.deepcopy(contract)
        for character in native_contract["characters"]:
            character["scene"] = f"res://addons/forge_assets/{character['id']}/forge_animated_sprite.tscn"
            for action in character["actions"]:
                action["frames"] = [str(root / p) for p in action["frames"]]
        for item in native_contract["static"]:
            item["texture"] = f"res://addons/forge_assets/props/items/{item['id']}.png"
            item["source"] = str(root / item["source"])
        for item in native_contract["audio"]:
            item["stream"] = f"res://addons/forge_assets/audio/streams/{item['id']}.res"
        save(game / "acceptance.json", native_contract)
        shutil.copyfile(CHECKER, game / "verify.gd")
        argv = [godot, "--headless", "--path", game, "--script", "res://verify.gd"]
        native = command("native-acceptance", argv)
        report["nativeFailures"] = [line.removeprefix("FAIL:") for line in native.stderr.splitlines()
                                    if line.startswith("FAIL:")]
        report["sourcesUnchanged"] = hashes == {p.name: digest(p) for p in (root / "inputs").iterdir()}
        require(report["sourcesUnchanged"], "Source inputs changed")
        require_native(native, 5)
        for result in report["results"].values():
            result["nativeAcceptance"] = True
        native_contract["characters"][0]["actions"][0]["durationsMs"][0] += 10
        save(game / "acceptance.json", native_contract)
        negative = command("native-negative-control", argv)
        require(negative.returncode != 0 and "FAIL:duration:synthetic:idle:0" in negative.stderr,
                "Native checker did not reject incorrect expected timing")
        report["negativeControl"] = "wrong_expected_duration_rejected"
        native_contract["characters"][0]["actions"][0]["durationsMs"][0] -= 10
        save(game / "acceptance.json", native_contract)
        require(hashes == {p.name: digest(p) for p in (root / "inputs").iterdir()}, "Source inputs changed")
        report["ok"] = True
    except Exception as error:
        report["error"] = str(error)
        raise
    finally:
        save(root / "report.json", report)
    return report


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--forge", type=Path, required=True)
    parser.add_argument("--godot", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = run(args)
    print(json.dumps({"ok": result["ok"], "report": str(args.output.absolute() / "report.json")}))
