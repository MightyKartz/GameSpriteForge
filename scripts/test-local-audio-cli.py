#!/usr/bin/env python3
"""Verify local WAV -> audio Pack -> Godot and portable receipts, offline.

Uses only synthetic audio and a new isolated project. This verifies technical
delivery, never listening quality, seamlessness, model output or licensing.
"""
import argparse
import copy
import hashlib
import json
import math
import os
import shutil
import struct
import subprocess
import tempfile
import wave
from pathlib import Path


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n")


def inventory(path):
    if not path.exists():
        return {}
    return {str(p.relative_to(path)): digest(p) for p in sorted(path.rglob("*")) if p.is_file()}


def wav(path, rate, channels):
    data = bytearray()
    for frame in range(rate):
        for channel in range(channels):
            data.extend(struct.pack("<h", round(6000 * math.sin(2 * math.pi * (440 + channel * 110) * frame / rate))))
    with wave.open(str(path), "wb") as stream:
        stream.setparams((channels, 2, rate, rate, "NONE", "not compressed"))
        stream.writeframes(data)


class Check:
    def __init__(self, forge, godot, root):
        self.forge, self.godot, self.root = forge, godot, root
        self.env = dict(os.environ, FORGE_JOB_STORE=str(root / "jobs"), FORGE_PLAN_STORE=str(root / "plans"),
                        FORGE_CACHE_STORE=str(root / "cache"), FORGE_REAL_PROVIDER_MAX_REQUESTS="0",
                        FORGE_GODOT_PATH=str(godot))
        self.env.pop("FORGE_REAL_PROVIDER_ACCEPT", None)
        self.commands = []
        (root / "commands").mkdir()

    def call(self, label, *arguments, fail=False):
        result = subprocess.run([str(self.forge), *map(str, arguments), "--json"], cwd=self.root,
                                env=self.env, capture_output=True, text=True, timeout=120)
        prefix = self.root / "commands" / f"{len(self.commands):03d}-{label}"
        prefix.with_suffix(".stdout.json").write_text(result.stdout)
        prefix.with_suffix(".stderr.log").write_text(result.stderr)
        self.commands.append({"label": label, "arguments": list(map(str, arguments)), "exitCode": result.returncode})
        try:
            envelope = json.loads(result.stdout)
        except json.JSONDecodeError as error:
            raise AssertionError(f"{label}: expected a JSON envelope, exit {result.returncode}: {result.stderr}") from error
        assert (result.returncode != 0) == fail and envelope.get("ok") is (not fail), (label, envelope, result.stderr)
        return envelope.get("error") if fail else envelope["data"]

    def stores(self):
        return {name: inventory(self.root / name) for name in ("plans", "jobs")}

    def successful_audio_job(self, job):
        assert job["lifecycle_state"] == "succeeded", job
        assert job["source_kind"] == "import_audio" and job["operation_kind"] == "prepare_audio", job
        report = self.call("audio-job-report", "job", "report", "--id", job["job_id"])
        assert report["providerRequestOccurred"] is False and report["providerRequestCount"] == 0, report
        return Path(next(a["path"] for a in job["artifacts"] if a["kind"] == "gsfpack"))

    def run(self):
        doctor = self.call("doctor", "doctor")
        assert "local_audio_import" in doctor["capabilities"], doctor
        specs = self.root / "specs"
        sources = specs / "inputs"
        sources.mkdir(parents=True)
        cue, ambience = sources / "cue.wav", sources / "ambience.wav"
        wav(cue, 22050, 1)
        wav(ambience, 48000, 2)
        original_sources, before_stores = inventory(sources), self.stores()
        inspected = self.call("audio-inspect", "audio", "inspect", "--path", cue)
        assert inspected["sha256"] == digest(cue) and inspected["sampleRate"] == 22050
        assert inspected["channels"] == 1 and inspected["durationSeconds"] == 1.0
        assert inventory(sources) == original_sources and self.stores() == before_stores
        origin = {"tool": "Forge synthetic WAV fixture", "model": "none", "prompt": "synthetic sine wave",
                  "seed": "fixed", "license": "Synthetic fixture assertion; not verified by Forge"}
        request = {
            "schemaVersion": "1", "id": "audio-fixture", "name": "Synthetic audio fixture",
            "items": [
                {"id": "cue", "name": "One shot", "path": "inputs/cue.wav", "role": "sfx", "loop": False,
                 "trimStartSeconds": 0.1, "trimEndSeconds": 0.6, "fadeInMs": 20, "fadeOutMs": 30,
                 "gainDb": -6.0, "origin": origin},
                {"id": "ambience", "name": "Loop intent", "path": "inputs/ambience.wav", "role": "ambience",
                 "loop": True, "trimEndSeconds": 0.8, "crossfadeMs": 20, "gainDb": -3.0},
            ],
            "sourceLocks": [{"path": "inputs/cue.wav", "sha256": digest(cue)},
                            {"path": "inputs/ambience.wav", "sha256": digest(ambience)}],
        }
        request_path = specs / "audio.json"
        save(request_path, request)
        bad = copy.deepcopy(request)
        bad["sourceLocks"][0]["sha256"] = "0" * 64
        bad_path = specs / "bad-lock.json"
        save(bad_path, bad)
        before = self.stores()
        self.call("reject-source-lock", "plan", "prepare-audio", "--request", bad_path, fail=True)
        assert self.stores() == before
        plan = self.call("audio-plan", "plan", "prepare-audio", "--request", request_path)
        assert plan["estimate"]["providerRequestEstimate"] == plan["estimate"]["maximumProviderRequests"] == 0
        original = cue.read_bytes()
        cue.write_bytes(original[:-2] + bytes((original[-2] ^ 1, original[-1])))
        before = self.stores()
        self.call("reject-changed-source", "plan", "execute", "--token", plan["token"], "--wait", fail=True)
        assert self.stores() == before, "changed input consumed the token or created a Job"
        cue.write_bytes(original)
        planned_job = self.call("audio-execute", "plan", "execute", "--token", plan["token"], "--wait")
        self.successful_audio_job(planned_job)
        before = self.stores()
        self.call("reject-reused-token", "plan", "execute", "--token", plan["token"], "--wait", fail=True)
        assert self.stores() == before
        job = self.call("audio-import", "audio", "import", "--request", request_path, "--wait")
        pack = self.successful_audio_job(job)
        self.call("pack-validate", "pack", "validate", "--path", pack)
        summary = self.call("pack-inspect", "asset", "inspect", "--pack", pack)
        assert summary["assetType"] == "audio_set", summary
        metadata = json.loads((pack / "forgepack.json").read_text())
        manifest = json.loads((pack / "assets/manifest.json").read_text())
        quality = json.loads((pack / "quality-report.json").read_text())
        assert metadata["schemaVersion"] == "4.0.0" and metadata["provenance"]["kind"] == "local_audio_import"
        assert metadata["provenance"]["generationPerformed"] is False and metadata["provenance"]["licenseVerified"] is False
        assert json.loads((pack / "assets/godot_import.json").read_text()) == manifest
        for field in ("generationPerformed", "originVerified", "licenseVerified", "listeningReviewPerformed", "seamlessLoopVerified"):
            assert quality[field] is False, (field, quality)
        assert quality["providerRequestOccurred"] is False and quality["providerRequestCount"] == 0
        by_id = {item["id"]: item for item in manifest["items"]}
        for item_id, frames, looping in (("cue", 24000, False), ("ambience", 37440, True)):
            item = by_id[item_id]
            assert item["audio"]["sampleRate"] == 48000 and item["audio"]["channels"] == 2
            assert item["audio"]["bitsPerSample"] == 16 and item["audio"]["frameCount"] == frames
            assert item["loop"] is looping and item["audio"]["clippedSampleCount"] == 0
            assert item["source"]["sha256"] == original_sources[f"{item_id}.wav"] == digest(pack / item["source"]["path"])
            # Rust canonicalization may retain the Windows verbatim prefix.
            assert Path(item["source"]["originalPath"]).samefile(sources / f"{item_id}.wav")
        assert by_id["cue"]["source"]["origin"] == origin and "origin" not in by_id["ambience"]["source"]
        assert 0.02 < by_id["cue"]["audio"]["peakAmplitude"] < 0.11
        assert not list(pack.rglob("*.png")) and not list(pack.rglob("*.gif"))
        assert inventory(sources) == original_sources

        game = self.root / "game"
        game.mkdir()
        (game / "project.godot").write_text('config_version=5\n[application]\nconfig/name="Forge synthetic audio QA"\n')
        install_plan = self.call("godot-plan", "godot", "plan-install", "--pack", pack, "--project", game)
        installed = self.call("godot-install", "plan", "execute", "--token", install_plan["token"], "--wait")
        assert installed["lifecycle_state"] == "succeeded" and installed["operation_kind"] == "install_godot", installed
        target = game / "addons/forge_assets/audio-fixture"
        usage = json.loads((target / "forge_usage.json").read_text())
        assert usage["nodeType"] == "AudioStreamWAV" and usage["listeningReview"] == "not_assessed", usage
        for item_id in by_id:
            assert usage["audioPaths"][item_id] == f"res://addons/forge_assets/audio-fixture/streams/{item_id}.res"
            assert (target / "streams" / f"{item_id}.res").read_bytes()[:4] in (b"RSRC", b"RSCC")
            assert digest(target / "sources" / f"{item_id}.wav") == by_id[item_id]["sha256"]
        verification_path = Path(next(a["path"] for a in installed["artifacts"] if a["kind"] == "godot_install_verification"))
        verification = json.loads(verification_path.read_text())
        assert verification["nativeLoadVerified"] is True
        native = next(phase for phase in verification["phases"] if phase["phase"] == "verify")["audioStreams"]
        assert {item["id"] for item in native} == set(by_id)
        for item in native:
            assert item["sampleRate"] == 48000 and item["channels"] == 2
            assert item["frameCount"] == by_id[item["id"]]["audio"]["frameCount"]
            assert item["loop"] is by_id[item["id"]]["loop"] and item["loopBegin"] == 0
            assert item["loopEnd"] == (item["frameCount"] if item["loop"] else 0)
        before = inventory(game)
        audit = self.call("install-audit", "godot", "verify-install", "--project", game, "--asset-key", "audio-fixture")
        assert audit["readOnly"] is True and audit["cacheCheck"]["status"] == "verified"
        assert audit["cacheCheck"]["materialized"] is True and inventory(game) == before
        snapshot = json.loads((target / ".forge-install.json").read_text())
        assert snapshot["schemaVersion"] == "2"
        samples = [game / f["path"] for f in snapshot["cacheFiles"] if f["path"].endswith(".sample")]
        assert len(samples) == 2 and len(snapshot["importFiles"]) == 2, snapshot
        for label, path in (("native-resource", target / "streams/cue.res"), ("cache", samples[0])):
            original = path.read_bytes()
            path.write_bytes(original + b"tampered fixture")
            before = inventory(game)
            self.call(f"reject-{label}-tamper", "godot", "verify-install", "--project", game, "--asset-key", "audio-fixture", fail=True)
            assert inventory(game) == before
            path.write_bytes(original)

        delivery = self.root / "delivery"
        # FFmpeg's high-rate extensible WAV header is unsupported by Godot 4.6;
        # the complete CLI route must deliver the canonical PCM header instead.
        high_request = copy.deepcopy(request)
        high_request.update(id="audio-high-rate", sampleRate=96000)
        high_path = specs / "high-rate.json"
        save(high_path, high_request)
        high_job = self.call("high-rate-import", "audio", "import", "--request", high_path, "--wait")
        high_pack = self.successful_audio_job(high_job)
        high_manifest = json.loads((high_pack / "assets/manifest.json").read_text())
        for item in high_manifest["items"]:
            assert item["audio"]["sampleRate"] == 96000
            assert (high_pack / item["path"]).read_bytes()[20:22] == b"\x01\x00"
        high_plan = self.call("high-rate-install-plan", "godot", "plan-install", "--pack", high_pack,
                              "--project", game, "--asset-key", "audio-high-rate")
        high_install = self.call("high-rate-install", "plan", "execute", "--token", high_plan["token"], "--wait")
        assert high_install["lifecycle_state"] == "succeeded", high_install
        self.call("high-rate-audit", "godot", "verify-install", "--project", game, "--asset-key", "audio-high-rate")
        receipt = delivery / "audio-receipt.json"
        delivery.mkdir()
        self.call("receipt-export", "receipt", "export", "--job", job["job_id"], "--install-job", installed["job_id"], "--out", receipt)
        receipt_data = json.loads(receipt.read_text())
        assert receipt_data["prepare"]["execution"] is not None
        receipt_sha = digest(receipt)
        self.call("receipt-verify", "receipt", "verify", "--path", receipt, "--expected-sha256", receipt_sha)
        retained_pack = delivery / "PortableAudio.gsfpack"
        shutil.copytree(pack, retained_pack)
        (self.root / "jobs").rename(self.root / "archived-job-store")
        before_delivery, before_game = inventory(delivery), inventory(game)
        portable = self.call("receipt-portable", "receipt", "verify", "--path", receipt,
                             "--pack", retained_pack, "--project", game, "--expected-sha256", receipt_sha)
        assert portable["verified"] is True and not (self.root / "jobs").exists(), portable
        assert inventory(delivery) == before_delivery and inventory(game) == before_game
        mutated = retained_pack / by_id["cue"]["path"]
        original = mutated.read_bytes()
        mutated.write_bytes(original[:-2] + bytes((original[-2] ^ 1, original[-1])))
        self.call("reject-pack-tamper", "receipt", "verify", "--path", receipt, "--pack", retained_pack, "--project", game, fail=True)
        self.call("reject-pack-validation-tamper", "pack", "validate", "--path", retained_pack, fail=True)
        mutated.write_bytes(original)
        self.call("reject-receipt-hash", "receipt", "verify", "--path", receipt, "--pack", retained_pack,
                  "--project", game, "--expected-sha256", "0" * 64, fail=True)
        result = {"ok": True, "forge": str(self.forge), "forgeSha256": digest(self.forge), "doctor": doctor,
                  "godot": str(self.godot), "godotVersion": subprocess.check_output([str(self.godot), "--version"], text=True).strip(),
                  "source": "synthetic local WAVs only", "providerRequests": 0, "generationPerformed": False,
                  "listeningReviewPerformed": False, "nativeAudio": native, "cacheSamples": len(samples),
                  "receiptSha256": receipt_sha, "portableReceiptVerified": True, "commands": self.commands}
        save(self.root / "summary.json", result)
        print(f"PASS local audio CLI: {self.root}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--forge", required=True, type=Path)
    parser.add_argument("--godot", required=True, type=Path)
    parser.add_argument("--output", type=Path, help="New directory retaining synthetic fixtures and command evidence")
    args = parser.parse_args()
    forge, godot = Path(os.path.abspath(args.forge)), Path(os.path.abspath(args.godot))
    if args.output:
        root = args.output.resolve()
        root.mkdir(parents=True, exist_ok=False)
        Check(forge, godot, root).run()
    else:
        with tempfile.TemporaryDirectory(prefix="forge-audio-cli-") as directory:
            Check(forge, godot, Path(directory)).run()


if __name__ == "__main__":
    main()
