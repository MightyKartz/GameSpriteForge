#!/usr/bin/env python3
"""Reproduce the v0.4.0 local PNG/WAV -> Pack -> native Godot showcase.

Requires macOS, Python 3, the public Forge launcher, Godot 4.6.x, bundled FFmpeg
with h264_videotoolbox, and a graphical session. No Python packages, downloads,
models or credentials are used. Logs, requests, Packs and receipts are retained
under a new --work-dir; the intermediate AVI is removed after successful encoding.
"""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import wave

ITEMS = ("potion", "crystal", "key", "crate", "barrel", "campfire")
NAMES = ("Healing potion", "Mana crystal", "Brass key", "Supply crate", "Travel barrel", "Campfire")


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def save(path, data):
    Path(path).write_text(json.dumps(data, indent=2) + "\n")


def chime(path):
    """Original deterministic C-major chime; no recorded or model-generated input."""
    rate = 48000
    values = []
    for i in range(int(rate * 1.8)):
        time = i / rate
        value = 0.0
        for onset, frequency in ((0.0, 523.251), (0.12, 659.255), (0.24, 783.991)):
            age = time - onset
            if age >= 0:
                envelope = min(age / 0.009, 1.0) * math.exp(-age * 4.2)
                envelope *= min((1.8 - time) / 0.09, 1.0)
                value += envelope * (math.sin(math.tau * frequency * age)
                                     + 0.18 * math.sin(math.tau * frequency * 2 * age)) * 0.19
        values.append(round(max(-1.0, min(1.0, value)) * 32767))
    with wave.open(str(path), "wb") as output:
        output.setparams((1, 2, rate, len(values), "NONE", "not compressed"))
        output.writeframes(struct.pack("<" + "h" * len(values), *values))
    return [max(abs(x) for x in values[i:i + 1080]) / 32767 for i in range(0, len(values), 1080)]


class Showcase:
    def __init__(self, args):
        self.args = args
        self.media = Path(__file__).resolve().parent
        self.root = args.work_dir.absolute()
        if self.root.exists():
            raise ValueError("--work-dir must not exist; choose a new isolated directory")
        self.root.mkdir(parents=True)
        for name in ("logs", "specs", "sources", "catalog-inputs", "delivery", "empty-tools", "game"):
            (self.root / name).mkdir()
        self.env = os.environ.copy()
        self.env.update(FORGE_JOB_STORE=str(self.root / "jobs"), FORGE_PLAN_STORE=str(self.root / "plans"),
                        FORGE_GODOT_PATH=str(args.godot),
                        GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS=str(self.root / "empty-tools"),
                        GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS="1")
        self.calls = []
        self.completed = []

    def command(self, label, command, timeout=180):
        result = subprocess.run([str(x) for x in command], cwd=self.root, env=self.env,
                                capture_output=True, text=True, timeout=timeout)
        (self.root / "logs" / (label + ".stdout")).write_text(result.stdout)
        (self.root / "logs" / (label + ".stderr")).write_text(result.stderr)
        if result.returncode:
            raise RuntimeError(f"{label} failed ({result.returncode}); see {self.root / 'logs'}\n{result.stderr[-2000:]}")
        return result.stdout

    def call(self, label, *args):
        value = json.loads(self.command(label, [self.args.forge, *args, "--json"]))
        assert value["ok"], value
        self.calls.append({"label": label, "command": [str(x) for x in args], "ok": True})
        return value["data"]

    def prepare(self, label, request, operation):
        request_path = self.root / "specs" / (label + ".json")
        save(request_path, request)
        plan = self.call(label + "-plan", "plan", operation, "--request", request_path)
        assert plan["estimate"]["providerRequestEstimate"] == plan["estimate"]["maximumProviderRequests"] == 0
        job = self.call(label + "-execute", "plan", "execute", "--token", plan["token"], "--wait")
        assert job["lifecycle_state"] == "succeeded", job
        report = self.call(label + "-report", "job", "report", "--id", job["job_id"])
        assert report["providerRequestOccurred"] is False and report["providerRequestCount"] == 0
        pack = Path(next(a["path"] for a in job["artifacts"] if a["kind"] == "gsfpack"))
        validation = self.call(label + "-validate", "pack", "validate", "--path", pack)
        assert validation["valid"], validation
        info = self.call(label + "-inspect", "asset", "inspect", "--pack", pack)
        asset_id = request["assetProject"]["assetId"]
        history = self.call(label + "-history", "asset", "history", "--project", self.root / "library", "--id", asset_id)
        revision = next(h["revision"] for h in history if h["revision"] not in {x["revision"] for x in self.completed})
        retained = self.call(label + "-retain", "asset", "retain", "--project", self.root / "library",
                             "--id", asset_id, "--revision", revision)
        assert retained["retained"], retained
        result = {"label": label, "assetId": asset_id, "jobId": job["job_id"], "revision": revision,
                  "pack": str(pack), "packValid": True, "assetType": info["assetType"],
                  "providerRequestCount": 0, "report": report}
        self.completed.append(result)
        print(f"Prepared and validated {label}", flush=True)
        return result

    def install(self, item):
        asset_id = item["assetId"]
        library, game = self.root / "library", self.root / "game"
        lock = game / ".forge" / (asset_id + ".lock.json")
        self.call(asset_id + "-lock", "asset", "lock", "--project", library, "--id", asset_id,
                  "--revision", item["revision"], "--out", lock)
        plan = self.call(asset_id + "-install-plan", "godot", "plan-install", "--library", library,
                         "--asset-id", asset_id, "--asset-lock", lock, "--project", game,
                         "--asset-key", asset_id, "--target", "addons/forge_assets/" + asset_id)
        installed = self.call(asset_id + "-install", "plan", "execute", "--token", plan["token"], "--wait")
        assert installed["lifecycle_state"] == "succeeded", installed
        verification = self.call(asset_id + "-verify", "godot", "verify-install", "--project", game, "--asset-key", asset_id)
        assert verification["verifiedFiles"] > 0 and verification["cacheCheck"]["status"] == "verified", verification
        receipt = self.root / "delivery" / (asset_id + "-receipt.json")
        self.call(asset_id + "-receipt", "receipt", "export", "--job", item["jobId"],
                  "--install-job", installed["job_id"], "--out", receipt)
        retained_pack = self.root / "delivery" / (asset_id + ".gsfpack")
        shutil.copytree(item["pack"], retained_pack)
        item.update(installJobId=installed["job_id"], nativeInstallVerified=True,
                    receiptSha256=digest(receipt), portablePack=str(retained_pack),
                    packSha256=verification["packSha256"], installedFilesVerified=verification["verifiedFiles"])

    def check_media(self, mp4):
        ffprobe = Path(self.args.ffmpeg).absolute().with_name("ffprobe")
        probe = json.loads(self.command("media-probe", [ffprobe, "-v", "error", "-show_streams", "-show_format", "-of", "json", mp4]))
        video = next(s for s in probe["streams"] if s["codec_type"] == "video")
        audio = next(s for s in probe["streams"] if s["codec_type"] == "audio")
        assert (video["width"], video["height"], video["avg_frame_rate"], int(video["nb_frames"])) == (1200, 700, "30/1", 216), video
        assert audio["channels"] == 2 and int(audio["sample_rate"]) == 48000, audio
        assert 7.0 <= float(probe["format"]["duration"]) <= 7.4, probe
        windows = []
        for index, start in enumerate((0.65, 2.85, 5.05)):
            label = "audio-cue-" + str(index + 1)
            self.command(label, [self.args.ffmpeg, "-ss", str(start), "-t", "1.8", "-i", mp4,
                                 "-af", "volumedetect", "-vn", "-f", "null", "-"])
            log = (self.root / "logs" / (label + ".stderr")).read_text()
            samples = int(re.findall(r"n_samples: (\d+)", log)[-1])
            peak = float(re.findall(r"max_volume: ([-\d.]+) dB", log)[-1])
            mean = float(re.findall(r"mean_volume: ([-\d.]+) dB", log)[-1])
            assert samples > 0 and peak > -60.0 and mean > -60.0, log
            windows.append({"startSeconds": start, "durationSeconds": 1.8, "samples": samples,
                            "peakDb": peak, "meanDb": mean, "nonSilent": True})
        gif = json.loads(self.command("gif-probe", [ffprobe, "-v", "error", "-show_streams", "-show_format", "-of", "json", mp4.with_suffix(".gif")]))
        assert len(gif["streams"]) == 1 and gif["streams"][0]["codec_name"] == "gif", gif
        assert (gif["streams"][0]["width"], gif["streams"][0]["height"]) == (900, 525), gif
        assert 7.1 <= float(gif["format"]["duration"]) <= 7.3, gif
        return probe, {"videoCodec": video["codec_name"], "audioCodec": audio["codec_name"],
                       "sampleRate": int(audio["sample_rate"]), "channels": audio["channels"],
                       "cueWindows": windows, "gifHasAudio": False, "gifDurationSeconds": float(gif["format"]["duration"])}

    def run(self):
        doctor = self.call("doctor", "doctor")
        required = {"local_static_import", "local_audio_import", "audio_godot_delivery",
                    "project_asset_catalog_v3", "project_asset_output_registration", "delivery_receipts"}
        assert required.issubset(doctor["capabilities"]), doctor
        self.command("forge-version", [self.args.forge, "--version"])
        godot_version = self.command("godot-version", [self.args.godot, "--version"]).strip()
        assert godot_version.startswith("4.6."), godot_version
        source_hashes = {}
        for item in ITEMS:
            source = self.media.parent / "sprites" / (item + ".png")
            source_hashes["../sprites/" + source.name] = digest(source)
            shutil.copyfile(source, self.root / "sources" / source.name)
        waveform = chime(self.root / "sources" / "synthetic-chime.wav")
        source_hashes["generated/synthetic-chime.wav"] = digest(self.root / "sources" / "synthetic-chime.wav")
        self.call("library-init", "project", "init", "--path", self.root / "library",
                  "--name", "Forge v0.4.0 · local asset showcase", "--local-assets")
        for item in ("potion", "crystal"):
            shutil.copyfile(self.root / "sources" / (item + ".png"), self.root / "catalog-inputs" / (item + ".png"))
        shutil.copyfile(self.root / "sources" / "synthetic-chime.wav", self.root / "catalog-inputs" / "synthetic-chime.wav")
        historical = self.media.parent / "sword-spells.gif"
        source_hashes["../sword-spells.gif"] = digest(historical)
        shutil.copyfile(historical, self.root / "catalog-inputs" / "historical-spells.gif")
        intake_path = self.root / "specs" / "intake.json"
        scan = self.call("library-scan", "asset", "scan", "--project", self.root / "library",
                         "--root", self.root / "catalog-inputs", "--out", intake_path)
        assert not scan["issues"], scan
        for entry in scan["items"]:
            filename = Path(entry["path"]).name
            entry["assetId"] = "source-" + Path(filename).stem
            entry["tags"] = ["showcase", "source"]
            if filename == "historical-spells.gif":
                entry.update(name="Historical spells · GIF preview", purpose="Historical animation preview",
                             origin={"notes": "Existing public Sword GIF, registered unchanged. Historical import; no fresh animation delivery."})
            elif filename.endswith(".wav"):
                entry.update(name="Synthetic chime · WAV source", purpose="Synthetic sound effect",
                             origin={"tool": "Python math + wave", "model": "none", "notes": "Original deterministic sine-wave chime; no human listening review performed."})
            else:
                entry.update(name=Path(filename).stem.title() + " · PNG source", purpose="Public demonstration source",
                             origin={"notes": "Existing public intermediate PNG from the historical v0.2.1 showcase; bytes unchanged."})
        save(intake_path, {"schemaVersion": scan["schemaVersion"], "items": scan["items"]})
        self.call("library-register", "asset", "register", "--project", self.root / "library", "--input", intake_path)
        (self.root / "game" / "project.godot").write_text(PROJECT)
        # Godot's import pass checks the configured main scene during installation.
        (self.root / "game" / "main.tscn").write_text('[gd_scene format=3]\n[node name="PreparingAssets" type="Control"]\n')
        base = {"schemaVersion": "1", "kind": "prop_set", "id": "forest_props", "name": "Forest props",
                "license": "MIT; public repository demonstration assets", "sampling": "linear",
                "assetProject": {"projectPath": "../library", "assetId": "forest_props"},
                "items": [{"id": item, "name": name, "path": "../sources/" + item + ".png"} for item, name in zip(ITEMS, NAMES)],
                "sourceLocks": [{"path": "../sources/" + item + ".png", "sha256": source_hashes["../sprites/" + item + ".png"]} for item in ITEMS]}
        first = self.prepare("props-128", {**base, "canvasSize": 128}, "prepare-static")
        second = self.prepare("props-256", {**base, "canvasSize": 256}, "prepare-static")
        audio = self.prepare("synthetic-audio", {"schemaVersion": "1", "id": "synthetic_audio", "name": "Synthetic chime",
            "assetProject": {"projectPath": "../library", "assetId": "synthetic_audio"}, "sampleRate": 48000, "channels": 2,
            "items": [{"id": "chime", "name": "Synthetic C-major chime", "path": "../sources/synthetic-chime.wav",
                       "role": "sfx", "loop": False, "gainDb": -3, "fadeInMs": 5, "fadeOutMs": 60,
                       "origin": {"tool": "Python standard library math + wave", "model": "none",
                                  "prompt": "Original deterministic three-note sine-wave chime; synthetic showcase audio, not AI music",
                                  "seed": "deterministic", "license": "MIT; authored demonstration fixture, assertion not verified by Forge"}}],
            "sourceLocks": [{"path": "../sources/synthetic-chime.wav", "sha256": source_hashes["generated/synthetic-chime.wav"]}]}, "prepare-audio")
        for item, name, purpose, tags in ((second, "Forest props · 128 / 256 px", "Godot scene props", ("static", "two-revisions")),
                                         (audio, "Synthetic chime · audio Pack", "Synthetic sound effect", ("audio", "synthetic"))):
            self.call(item["assetId"] + "-annotate", "asset", "annotate", "--project", self.root / "library", "--id", item["assetId"],
                      "--name", name, "--purpose", purpose, "--tag", tags[0], "--tag", tags[1])
            self.install(item)
        # Prove portable receipts without the original Job store at its recorded location.
        (self.root / "jobs").rename(self.root / "jobs-retained")
        try:
            for item in (second, audio):
                receipt = self.root / "delivery" / (item["assetId"] + "-receipt.json")
                verified = self.call(item["assetId"] + "-portable-receipt", "receipt", "verify", "--path", receipt,
                                     "--pack", item["portablePack"], "--project", self.root / "game", "--expected-sha256", item["receiptSha256"])
                assert verified["verified"], verified
                item["portableReceiptVerified"] = True
                item["reviewVerification"] = {key: verified[key] for key in ("visualReview", "listeningReview") if key in verified}
        finally:
            (self.root / "jobs-retained").rename(self.root / "jobs")
        preview = self.call("library-preview", "asset", "preview", "--project", self.root / "library", "--limit", "12", "--out", self.root / "gallery")
        self.call("props-comparison", "asset", "preview", "--project", self.root / "library", "--id", "forest_props", "--out", self.root / "comparison")
        self.call("library-audit", "project", "verify-assets", "--project", self.root / "library")
        print("PREVIEW_INDEX " + str(self.root / "gallery" / "index.html"), flush=True)
        save(self.root / "game" / "showcase-inputs.json", {"waveform": waveform})
        shutil.copyfile(self.media / "render.gd", self.root / "game" / "render.gd")
        (self.root / "game" / "main.tscn").write_text(SCENE)
        self.command("godot-import", [self.args.godot, "--headless", "--path", self.root / "game", "--editor", "--import", "--quit"])
        movie = self.root / "native-capture.avi"
        self.command("godot-capture", [self.args.godot, "--path", self.root / "game", "--write-movie", movie,
                                      "--fixed-fps", "30", "--quit-after", "216"], timeout=240)
        capture_log = (self.root / "logs" / "godot-capture.stdout").read_text()
        assert "NATIVE_SHOWCASE_READY" in capture_log and capture_log.count("NATIVE_CHIME_PLAY") == 3, capture_log
        assert "SCRIPT ERROR" not in (self.root / "logs" / "godot-capture.stderr").read_text()
        encoded = self.root / "encoded"
        encoded.mkdir()
        mp4 = encoded / "native-delivery.mp4"
        self.command("mp4-encode", [self.args.ffmpeg, "-y", "-i", movie, "-c:v", "h264_videotoolbox", "-b:v", "5M", "-pix_fmt", "yuv420p",
                                    "-c:a", "aac", "-b:a", "160k", "-movflags", "+faststart", mp4])
        self.command("gif-encode", [self.args.ffmpeg, "-y", "-i", mp4, "-filter_complex",
            "fps=15,scale=900:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=192:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=3",
            "-an", "-loop", "0", encoded / "native-delivery.gif"])
        self.command("cover", [self.args.ffmpeg, "-y", "-ss", "1.2", "-i", mp4, "-frames:v", "1", encoded / "native-delivery.png"])
        ffprobe = Path(self.args.ffmpeg).absolute().with_name("ffprobe")
        probe, media_checks = self.check_media(mp4)
        for relative, expected in source_hashes.items():
            actual = self.root / "sources" / "synthetic-chime.wav" if relative.startswith("generated/") else self.media / relative
            assert digest(actual) == expected, relative
        outputs = {name: {"sha256": digest(encoded / name), "bytes": (encoded / name).stat().st_size}
                   for name in ("native-delivery.mp4", "native-delivery.gif", "native-delivery.png")}
        evidence = {"schemaVersion": 1, "purpose": "Fresh v0.4.0 static and audio native delivery showcase",
            "forge": {"version": doctor["cliVersion"], "build": doctor["build"], "launcherSha256": digest(self.args.forge),
                      "payloadBinarySha256": digest(Path(self.args.forge).resolve()),
                      "ffmpegSha256": digest(self.args.ffmpeg), "ffprobeSha256": digest(ffprobe)},
            "godot": {"version": godot_version, "binarySha256": digest(self.args.godot)}, "sources": source_hashes,
            "imports": self.completed, "outputs": outputs, "mediaChecks": media_checks,
            "recipeSha256": {name: digest(self.media / name) for name in ("render.py", "render.gd")},
            "capture": {"width": 1200, "height": 700, "fps": 30, "frames": 216,
                "durationSeconds": float(probe["format"]["duration"]), "audio": "Godot Movie Maker captured the installed AudioStreamWAV played three times; AAC encode",
                "presentation": "Authored native Godot asset demonstration, not Forge UI or gameplay; static prop scenes remain static",
                "gif": "Silent 15 FPS palette-quantized preview"},
            "review": {"humanVisualReviewPerformed": False, "humanListeningReviewPerformed": False, "licenseVerified": False,
                       "notes": "Structural validation, native load, receipt verification and signal checks do not establish human approval."},
            "gallery": {"registeredHistoricalGif": True, "freshAnimationImport": False, "propRevisions": 2, "reviewAssertionsWritten": 0},
            "sourceBytesUnchanged": True, "networkOrProviderRequests": 0}
        save(self.root / "run-evidence.json", {**evidence, "commands": self.calls, "ffprobe": probe})
        # Published evidence excludes absolute local paths and arbitrary raw report fields.
        public = {**evidence, "imports": [{key: value for key, value in item.items() if key not in ("pack", "portablePack", "report")}
                                           for item in self.completed]}
        text = json.dumps(public, indent=2) + "\n"
        assert "/Users/" not in text and str(self.root) not in text
        (encoded / "provenance.json").write_text(text)
        # Existing published media survive any failed preparation, capture or check.
        # Each validated candidate replaces its destination atomically.
        for name in (*outputs, "provenance.json"):
            os.replace(encoded / name, self.media / name)
        # Only the task-owned intermediate is removed, after successful encoding/verification.
        movie.unlink()
        print(json.dumps({"workDir": str(self.root), "preview": str(self.root / "gallery" / "index.html"), "outputs": outputs}, indent=2))


PROJECT = '''config_version=5
[application]
config/name="Forge · Native asset showcase"
run/main_scene="res://main.tscn"
config/features=PackedStringArray("4.6", "GL Compatibility")
[display]
window/size/viewport_width=1200
window/size/viewport_height=700
[rendering]
renderer/rendering_method="gl_compatibility"
environment/defaults/default_clear_color=Color(0.055, 0.09, 0.078, 1)
[editor]
movie_writer/mjpeg_quality=0.85
movie_writer/mix_rate=48000
movie_writer/speaker_mode=0
'''
SCENE = '''[gd_scene load_steps=2 format=3]
[ext_resource type="Script" path="res://render.gd" id="1"]
[node name="NativeAssetShowcase" type="Control"]
script = ExtResource("1")
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--forge", type=Path, required=True, help="Absolute public launcher; do not resolve its symlink")
    parser.add_argument("--godot", type=Path, required=True)
    parser.add_argument("--ffmpeg", type=Path, required=True, help="FFprobe must be beside this executable")
    parser.add_argument("--work-dir", type=Path, required=True)
    args = parser.parse_args()
    for name in ("forge", "godot", "ffmpeg"):
        value = getattr(args, name)
        if not value.is_absolute() or not value.is_file():
            parser.error("--" + name + " must name an existing absolute executable")
    Showcase(args).run()


if __name__ == "__main__":
    main()
