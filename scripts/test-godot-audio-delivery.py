#!/usr/bin/env python3
"""Exercise native Godot audio delivery with isolated, synthetic PCM16 WAVs."""
import argparse
import hashlib
import json
import math
import re
import shutil
import struct
import subprocess
import tempfile
import wave
from pathlib import Path


ERROR = re.compile(r"SCRIPT ERROR|(?:^|\n)ERROR:|(?:^|\n)FAIL ")
ROOT = Path(__file__).resolve().parents[1]


def write_wav(path, rate, channels, frames, frequency=440, extensible=False):
    path.parent.mkdir(parents=True, exist_ok=True)
    samples = bytearray()
    for frame in range(frames):
        for channel in range(channels):
            sample = round(6000 * math.sin(2 * math.pi * (frequency + channel * 110) * frame / rate))
            samples.extend(struct.pack("<h", sample))
    if extensible:
        # PCM16 WAVE_FORMAT_EXTENSIBLE, matching FFmpeg's high-rate output:
        # cbSize=22, validBits=16, front-left/front-right mask and PCM subtype.
        pcm_subtype = bytes.fromhex("0100000000001000800000aa00389b71")
        channel_mask = 3 if channels == 2 else 4
        fmt = struct.pack("<HHIIHHHHI", 0xFFFE, channels, rate, rate * channels * 2,
                          channels * 2, 16, 22, 16, channel_mask) + pcm_subtype
        chunks = b"fmt " + struct.pack("<I", len(fmt)) + fmt + b"data" + struct.pack("<I", len(samples)) + samples
        path.write_bytes(b"RIFF" + struct.pack("<I", 4 + len(chunks)) + b"WAVE" + chunks)
        return
    with wave.open(str(path), "wb") as stream:
        stream.setparams((channels, 2, rate, frames, "NONE", "not compressed"))
        stream.writeframes(samples)


def exercise(godot, output):
    output.mkdir(parents=True, exist_ok=False)
    project = output / "project"
    project.mkdir()
    (project / "project.godot").write_text('[application]\nconfig/name="Forge Audio Fixture"\n')
    for name in ("install_forge_pack.gd", "import_forge_pack.gd"):
        shutil.copy2(ROOT / "scripts/godot" / name, project / name)
    pack = output / "SyntheticAudio.gsfpack"
    items = []
    for item_id, rate, channels, frames, looping, extensible in (
        ("cue", 22050, 1, 11025, False, False),
        ("ambience", 48000, 2, 24000, True, False),
        ("high_rate", 96000, 2, 48000, True, False),
    ):
        path = pack / "assets/audio" / f"{item_id}.wav"
        write_wav(path, rate, channels, frames, extensible=extensible)
        items.append({
            "id": item_id, "name": item_id, "path": f"assets/audio/{item_id}.wav",
            "loop": looping, "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "audio": {"sampleRate": rate, "channels": channels, "frameCount": frames,
                      "durationSeconds": frames / rate, "bitsPerSample": 16},
        })
    helper = {"schemaVersion": "4.0.0", "assetType": "audio_set", "items": items}
    (pack / "assets/godot_import.json").write_text(json.dumps(helper))
    (pack / "forgepack.json").write_text(json.dumps({
        "schemaVersion": "4.0.0", "assetType": "audio_set", "name": "Synthetic Audio",
    }))
    target = project / "audio"
    (target / "sources").mkdir(parents=True)
    for item in items:
        shutil.copy2(pack / item["path"], target / "sources" / f'{item["id"]}.wav')

    count = 0

    def run(label, script="install_forge_pack.gd", arguments=None, fails=False, expected_error=None):
        nonlocal count
        count += 1
        command = [str(godot), "--headless", "--path", str(project), "--script", f"res://{script}"]
        if arguments is not None:
            command += ["--", *map(str, arguments)]
        result = subprocess.run(command, capture_output=True, text=True, timeout=30)
        text = result.stdout + result.stderr
        (output / f"{count:02d}-{label}.log").write_text(text)
        completions = [json.loads(line.removeprefix("FORGE_INSTALL_RESULT "))
                       for line in result.stdout.splitlines() if line.startswith("FORGE_INSTALL_RESULT ")]
        if fails:
            assert result.returncode != 0 and not completions, (label, text)
            assert "FAIL Forge Godot install:" in text and "SCRIPT ERROR" not in text, (label, text)
            if expected_error:
                assert expected_error in text, (label, text)
        else:
            assert result.returncode == 0 and not ERROR.search(text), (label, text)
            if script == "install_forge_pack.gd":
                assert len(completions) == 1 and completions[0]["status"] == "succeeded", (label, text)
                assert completions[0]["assetType"] == "audio_set", (label, text)
        return completions, result.stdout

    run("install", arguments=[pack, "audio"])
    results, _ = run("verify", arguments=[pack, "audio", "--verify"])
    assert len(results[0]["audioStreams"]) == 3
    # Load saved binary resources independently, rather than trusting the
    # installer's own completion evidence or WAV import cache.
    (project / "inspect.gd").write_text('''extends SceneTree
func _initialize() -> void:
    var result := []
    for id in ["cue", "ambience", "high_rate"]:
        var stream := ResourceLoader.load("res://audio/streams/" + id + ".res") as AudioStreamWAV
        if stream == null:
            quit(1)
            return
        result.append({"id": id, "rate": stream.mix_rate, "stereo": stream.stereo,
            "format": stream.format, "bytes": stream.data.size(), "duration": stream.get_length(),
            "loop": stream.loop_mode, "begin": stream.loop_begin, "end": stream.loop_end})
    print("AUDIO_INSPECT " + JSON.stringify(result))
    quit(0)
''')
    _, stdout = run("inspect", "inspect.gd")
    actual = json.loads(next(line.removeprefix("AUDIO_INSPECT ") for line in stdout.splitlines()
                             if line.startswith("AUDIO_INSPECT ")))
    assert actual == [
        {"id": "cue", "rate": 22050, "stereo": False, "format": 1, "bytes": 22050,
         "duration": 0.5, "loop": 0, "begin": 0, "end": 0},
        {"id": "ambience", "rate": 48000, "stereo": True, "format": 1, "bytes": 96000,
         "duration": 0.5, "loop": 1, "begin": 0, "end": 24000},
        {"id": "high_rate", "rate": 96000, "stereo": True, "format": 1, "bytes": 192000,
         "duration": 0.5, "loop": 1, "begin": 0, "end": 48000},
    ], actual
    for item in items:
        resource = target / "streams" / f'{item["id"]}.res'
        assert resource.read_bytes()[:4] in (b"RSRC", b"RSCC")
        assert resource.stat().st_size < (pack / item["path"]).stat().st_size + 4096

    (project / "damage_loop.gd").write_text('''extends SceneTree
func _initialize() -> void:
    var path := "res://audio/streams/ambience.res"
    var stream := ResourceLoader.load(path) as AudioStreamWAV
    stream.loop_end -= 1
    quit(0 if ResourceSaver.save(stream, path) == OK else 1)
''')
    run("damage-loop", "damage_loop.gd")
    run("reject-loop", arguments=[pack, "audio", "--verify"], fails=True)
    run("restore", arguments=[pack, "audio"])
    extra = target / "streams/extra.res"
    shutil.copy2(target / "streams/cue.res", extra)
    run("reject-extra", arguments=[pack, "audio", "--verify"], fails=True)
    extra.unlink()
    write_wav(target / "sources/cue.wav", 22050, 1, 11025, frequency=880)
    run("reject-source", arguments=[pack, "audio", "--verify"], fails=True)
    (target / "sources/cue.wav").unlink()
    run("reject-missing", arguments=[pack, "audio"], fails=True)
    # Format disagreement must fail during construction before a success marker.
    write_wav(target / "sources/cue.wav", 44100, 1, 11025)
    run("reject-rate", arguments=[pack, "audio"], fails=True)
    shutil.copy2(pack / "assets/audio/cue.wav", target / "sources/cue.wav")
    # Godot 4.6 does not decode WAVE_FORMAT_EXTENSIBLE, even with PCM16 subtype.
    # The same 96 kHz samples must be delivered with the canonical classic fmt1
    # header; intentional decoder errors here must never become a success.
    write_wav(target / "sources/high_rate.wav", 96000, 2, 48000, extensible=True)
    run("reject-extensible", arguments=[pack, "audio"], fails=True,
        expected_error="Format not supported for WAVE file (not PCM)")
    shutil.copy2(pack / "assets/audio/high_rate.wav", target / "sources/high_rate.wav")
    _, smoke = run("import-smoke", "import_forge_pack.gd", [pack])
    assert "PASS Forge Godot import smoke: imported and verified 3 native audio streams" in smoke
    summary = {"status": "passed", "syntheticOnly": True, "nativeStreams": actual,
               "classicPcm16At96000Verified": True, "extensibleWavRejected": True,
               "negativeChecks": ["loop", "extra-resource", "source-content", "missing-source", "sample-rate", "unsupported-extensible"],
               "godotVersion": subprocess.check_output([str(godot), "--version"], text=True).strip()}
    (output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(f"PASS Godot audio delivery: {output}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--godot", required=True, type=Path)
    parser.add_argument("--output", type=Path, help="New directory for synthetic fixtures and logs")
    args = parser.parse_args()
    if args.output:
        exercise(args.godot.resolve(), args.output.resolve())
    else:
        with tempfile.TemporaryDirectory(prefix="forge-audio-godot-") as directory:
            exercise(args.godot.resolve(), Path(directory) / "evidence")


if __name__ == "__main__":
    main()
