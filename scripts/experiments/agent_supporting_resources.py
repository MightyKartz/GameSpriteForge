#!/usr/bin/env python3
"""M3 synthetic Agent tasks: existing guide delivery, preview and install recovery.

No consumer projects, toolchain upgrades, Provider requests or artistic approvals.
Run with Python 3.10+ and the selected checkout's absolute CLI and Godot paths.
"""
import argparse
import copy
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import sys
import wave

SCRIPTS = Path(__file__).resolve().parents[1]


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    loaded = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(loaded)
    return loaded


audio = module("audio_fixture", SCRIPTS / "test-local-audio-cli.py")
static = module("static_fixture", SCRIPTS / "test-local-static-cli.py")
digest, inventory, save = audio.digest, audio.inventory, audio.save


def run(args):
    root = args.output.resolve()
    root.mkdir(parents=True, exist_ok=False)
    cli = audio.Check(args.forge.absolute(), args.godot.absolute(), root)
    cli.env.update(FORGE_CONFIG_DIR=str(root / "config"))
    summary = {"ok": False, "syntheticOnly": True, "visualReview": "not_assessed",
               "listeningReview": "not_assessed", "licenseConfirmation": "fixture_author_assertion_only",
               "runnerSha256": digest(Path(__file__)), "tasks": {}, "rejections": {}}
    try:
        summary["doctor"] = cli.call("doctor", "doctor")
        summary["binarySha256"] = pin = digest(Path(summary["doctor"]["cliPath"]))
        summary["godotVersion"] = subprocess.check_output([str(cli.godot), "--version"], text=True).strip()
        example = root / "local-delivery.py"
        result = subprocess.run([str(cli.forge), "guide", "local-delivery-example"], env=cli.env,
                                capture_output=True, check=True)
        example.write_bytes(result.stdout)
        summary["embeddedExampleSha256"] = digest(example)
        sources = root / "sources"
        sources.mkdir()
        for name, rate, channels in [("music", 22050, 1), ("cue", 48000, 2)]:
            audio.wav(sources / f"{name}.wav", rate, channels)
        static.png(sources / "prop.png", (150, 95, 70))
        (sources / "invalid.wav").write_bytes(bytes(64))
        (sources / "invalid.png").write_bytes(bytes(64))
        # Exercise the missing-helper failure without probing or modifying user tools.
        isolated = root / "without-tools"
        isolated.mkdir()
        payload = isolated / Path(summary["doctor"]["cliPath"]).name
        shutil.copy2(summary["doctor"]["cliPath"], payload)
        missing_env = dict(cli.env, GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS=str(isolated),
                           GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS="1")
        missing = subprocess.run([str(payload), "audio", "inspect", "--path", str(sources / "music.wav"), "--json"],
                                 env=missing_env, capture_output=True, text=True, timeout=30)
        error = json.loads(missing.stdout)["error"]
        assert missing.returncode and "ffmpeg/ffprobe" in error["message"], (missing.stdout, missing.stderr)
        summary["rejections"]["missingAudioHelpers"] = error
        payload.unlink()
        original_sources = inventory(sources)
        summary["sourceSha256"] = original_sources
        library = root / "library"
        cli.call("library", "project", "init", "--path", library, "--name", "M3 synthetic resources", "--local-assets")
        game = root / "game"
        game.mkdir()
        project_text = 'config_version=5\n[application]\nconfig/name="M3 synthetic tasks"\n[rendering]\nrenderer/rendering_method="gl_compatibility"\n'
        (game / "project.godot").write_text(project_text)
        (game / "gameplay.gd").write_text("extends Node\n# Caller-owned game logic\n")
        # An isolated sentinel is not a real consumer lock and must stay byte-identical.
        (game / "toolchain.lock").write_bytes(b"caller-owned-toolchain-pin\n")
        caller = {p: digest(game / p) for p in ["project.godot", "gameplay.gd", "toolchain.lock"]}
        prepared = {}
        for name, operation in [("music", "prepare-audio"), ("cue", "prepare-audio"), ("props", "prepare-static")]:
            if operation == "prepare-audio":
                rate, channels = (22050, 1) if name == "music" else (48000, 2)
                inspected = cli.call(name + "-inspect", "audio", "inspect", "--path", sources / f"{name}.wav")
                assert (inspected["sampleRate"], inspected["channels"], inspected["durationSeconds"]) == (rate, channels, 1.0)
                request = {"schemaVersion": "1", "id": name, "name": name, "sampleRate": rate, "channels": channels,
                           "items": [{"id": name, "path": f"sources/{name}.wav", "role": "music" if name == "music" else "sfx", "loop": name == "music"}]}
            else:
                cli.call("prop-inspect", "source", "inspect", "--path", sources / "prop.png")
                request = {"schemaVersion": "1", "id": name, "name": name, "kind": "prop_set", "license": "Synthetic test fixture",
                           "sampling": "nearest", "canvasSize": 64,
                           "items": [{"id": "prop", "name": "prop", "path": "sources/prop.png"}]}
            request["assetProject"] = {"projectPath": str(library), "assetId": name}
            request["sourceLocks"] = [{"path": i["path"], "sha256": digest(root / i["path"])} for i in request["items"]]
            path = root / f"{name}.json"
            save(path, request)
            if name in {"music", "props"}:
                bad = copy.deepcopy(request)
                bad.pop("sourceLocks")
                bad["items"].append(dict(bad["items"][0], id="broken", path="sources/invalid." + ("wav" if name == "music" else "png")))
                bad_path = root / f"{name}-invalid.json"
                save(bad_path, bad)
                before = cli.stores()
                error = cli.call(name + "-invalid", "plan", operation, "--request", bad_path, fail=True)
                assert "broken" in error["message"] and "invalid." in error["message"], error
                assert cli.stores() == before
                summary["rejections"][name] = error
            output = root / (name + "-delivery")
            command = [sys.executable, str(example), "--forge", str(cli.forge), "--expected-binary-sha256", pin,
                       "--operation", operation, "--request", str(path), "--project", str(game),
                       "--asset-key", name, "--out", str(output)]
            if name == "music":
                implicit = copy.deepcopy(request)
                implicit.pop("channels")
                save(path, implicit)
                rejected = subprocess.run(command, env=cli.env, capture_output=True, text=True, timeout=30)
                assert rejected.returncode and "explicit sampleRate and channels" in rejected.stderr and not output.exists()
                summary["rejections"]["implicitAudioConversion"] = rejected.stderr.strip()
                save(path, request)
                # Preparation succeeds; a real Godot project error makes installation fail.
                (game / "project.godot").write_text(project_text + '\n[autoload]\nBroken="*res://broken.gd"\n')
                (game / "broken.gd").write_text("extends Node\nthis is not GDScript\n")
            result = subprocess.run(command, env=cli.env, capture_output=True, text=True, timeout=480)
            (root / (name + "-delivery.log")).write_text(result.stdout + result.stderr)
            progress = json.loads((output / "progress.json").read_text())
            pack = output / "retained.gsfpack"
            assert pack.is_dir() and (output / "prepared-receipt.json").is_file()
            if name == "music":
                assert result.returncode and not progress["completed"] and progress["installJob"]
                assert "godot_project_import_failed" in json.dumps(progress), progress
                assert not (game / "addons/forge_assets/music").exists()
                old_evidence = {p: digest(output / p) for p in ["progress.json", "prepared-receipt.json"]}
                pack_before = inventory(pack)
                (game / "project.godot").write_text(project_text)
                (game / "broken.gd").unlink()
                # Reuse the retained Pack and preparation Job. No second prepare/import.
                cli.env.update(FORGE_JOB_STORE=str(output / "jobs"), FORGE_PLAN_STORE=str(output / "plans"))
                plan = cli.call("recovery-plan", "godot", "plan-install", "--pack", pack, "--project", game, "--asset-key", name)
                installed = cli.call("recovery-install", "plan", "execute", "--token", plan["token"], "--wait")
                assert installed["lifecycle_state"] == "succeeded", installed
                receipt = output / "recovered-receipt.json"
                cli.call("recovered-receipt", "receipt", "export", "--job", progress["prepareJob"], "--install-job", installed["job_id"], "--out", receipt)
                assert inventory(pack) == pack_before
                assert old_evidence == {p: digest(output / p) for p in old_evidence}
                # Exact successful preparation identity is also bound into the receipt.
                receipt_data = json.loads(receipt.read_text())
                assert receipt_data["prepare"]["job"]["job_id"] == progress["prepareJob"]
                summary["recovery"] = {"prepareJob": progress["prepareJob"], "failedInstallJob": progress["installJob"],
                                       "recoveredInstallJob": installed["job_id"], "oldEvidenceUnchanged": True, "packUnchanged": True}
            else:
                assert result.returncode == 0 and progress["completed"], (result.stdout, result.stderr)
                receipt = output / "delivery-receipt.json"
            verified = cli.call(name + "-receipt", "receipt", "verify", "--path", receipt, "--pack", pack,
                                "--project", game, "--expected-sha256", digest(receipt))
            assert verified["verified"] and verified["installationVerified"]
            cli.call(name + "-audit", "godot", "verify-install", "--project", game, "--asset-key", name, "--pack", pack)
            info = cli.call(name + "-pack", "asset", "inspect", "--pack", pack)
            if operation == "prepare-audio":
                item = info["audioItems"][0]
                assert item["audio"]["sampleRate"] == rate and item["audio"]["channels"] == channels
                assert item["audio"]["durationSeconds"] == 1 and item["loop"] is (name == "music")
                with wave.open(str(sources / f"{name}.wav"), "rb") as src, wave.open(str(pack / item["path"]), "rb") as dst:
                    assert src.readframes(src.getnframes()) == dst.readframes(dst.getnframes()), "Neutral PCM16 processing changed samples"
            history = cli.call(name + "-history", "asset", "history", "--project", library, "--id", name)
            assert len(history) == 1
            catalog = inventory(library)
            preview = root / (name + "-preview")
            cli.call(name + "-preview", "asset", "preview", "--project", library, "--id", name,
                     "--revision", history[0]["revision"], "--out", preview)
            html = (preview / "index.html").read_text()
            assert ("--domain auditory" if operation == "prepare-audio" else "--domain visual") in html
            assert ("<audio controls" if operation == "prepare-audio" else "<img") in html
            assert inventory(library) == catalog
            if operation == "prepare-audio":
                assert '"listeningReview": "not_assessed"' in json.dumps(verified)
            if operation == "prepare-audio":
                media = [digest(p) for p in (preview / "media").rglob("*.wav")]
                assert sorted(media) == sorted([digest(sources / f"{name}.wav"), digest(pack / item["path"])])
            prepared[name] = pack
            summary["tasks"][name] = {"operation": operation, "prepareJob": progress["prepareJob"],
                                      "receiptSha256": digest(receipt), "previewNoApprovalMutation": True,
                                      "providerRequests": 0, "revision": history[0]["revision"]}
        # Independent saved-resource and runtime playback check (headless, no listening claim).
        checker = SCRIPTS / "experiments/check-supporting-resources.gd"
        shutil.copyfile(checker, game / "check.gd")
        summary["checkerSha256"] = digest(checker)
        native = subprocess.run([str(cli.godot), "--headless", "--path", str(game), "--script", "res://check.gd"],
                                capture_output=True, text=True, timeout=30)
        log = native.stdout + native.stderr
        (root / "native-playback.log").write_text(log)
        assert native.returncode == 0 and "M3_NATIVE_PASS" in log and "SCRIPT ERROR" not in log and "ERROR:" not in log, log
        assert inventory(sources) == original_sources
        assert caller == {p: digest(game / p) for p in caller}
        summary.update(ok=True, sourceBytesPreserved=True, callerFilesPreserved=True, nativePlayback=True)
    except Exception as error:
        summary["error"] = str(error)
        raise
    finally:
        summary["commands"] = cli.commands
        save(root / "summary.json", summary)
    print(f"PASS M3 supporting resource tasks: {root}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for flag in ["forge", "godot", "output"]:
        parser.add_argument("--" + flag, type=Path, required=True)
    run(parser.parse_args())
