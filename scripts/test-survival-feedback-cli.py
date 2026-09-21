#!/usr/bin/env python3
"""Synthetic Survival feedback contracts; never reads consumer assets or pins.

--godot adds native delivery/recovery and receipt verification without JobStore.
--storage-root probes an explicitly selected directory, including external disks.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import runpy
import struct
import subprocess
import sys
import tempfile
import zlib
from unittest.mock import patch


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def png(path):
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xffffffff)
    pixels = b"".join(b"\0" + b"".join(bytes((140, 80, 20, 255) if 8 <= x < 24 and 8 <= y < 24 else (0, 0, 0, 0)) for x in range(32)) for y in range(32))
    path.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 32, 32, 8, 6, 0, 0, 0))
                     + chunk(b"IDAT", zlib.compress(pixels)) + chunk(b"IEND", b""))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--forge", required=True, type=Path)
    parser.add_argument("--godot", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--storage-root", type=Path)
    parser.add_argument("--expect-storage-unsupported", action="store_true")
    args = parser.parse_args()
    root = args.output.absolute() if args.output else Path(tempfile.mkdtemp(prefix="forge-survival-test-"))
    if args.output:
        root.mkdir(parents=True, exist_ok=False)
    forge = args.forge.absolute()
    env = dict(os.environ, FORGE_JOB_STORE=str(root / "jobs"), FORGE_PLAN_STORE=str(root / "plans"),
               FORGE_CONFIG_DIR=str(root / "config"), FORGE_REAL_PROVIDER_MAX_REQUESTS="0")
    if args.godot:
        env["FORGE_GODOT_PATH"] = str(args.godot.resolve())
    commands = []

    def call(*arguments):
        proc = subprocess.run([str(forge), *map(str, arguments), "--json"], env=env, capture_output=True, text=True, timeout=120)
        result = json.loads(proc.stdout)
        commands.append({"arguments": list(map(str, arguments)), "exit": proc.returncode, "result": result})
        assert proc.returncode == 0 and result["ok"], (arguments, result, proc.stderr)
        return result["data"]

    doctor = call("doctor")
    pin = digest(Path(doctor["cliPath"]))
    scratch_parent = args.storage_root.resolve() if args.storage_root else root
    # Leave a caller-owned sentinel beside the isolated probe and check its bytes.
    with tempfile.TemporaryDirectory(prefix="forge-storage-contract-", dir=scratch_parent) as d:
        probe_root = Path(d)
        (probe_root / "keep").write_bytes(b"sentinel")
        probe = call("storage", "check", "--path", probe_root)
        assert probe["supported"] is (not args.expect_storage_unsupported), probe
        assert probe["temporaryFilesRemoved"]
        assert [p.name for p in probe_root.iterdir()] == ["keep"]
        assert (probe_root / "keep").read_bytes() == b"sentinel"

    example = root / "local-delivery.py"
    guide = subprocess.run([str(forge), "guide", "local-delivery-example"], env=env, capture_output=True, check=True)
    example.write_bytes(guide.stdout)
    compile(guide.stdout, str(example), "exec")
    source = root / "source.png"
    png(source)
    request = root / "request.json"
    spec = {"schemaVersion": "1", "kind": "icon_set", "id": "synthetic", "name": "Synthetic",
            "license": "CC0-1.0", "sampling": "linear", "canvasSize": 64,
            "items": [{"id": "item", "name": "Item", "path": "source.png"}],
            "sourceLocks": [{"path": "source.png", "sha256": digest(source)}]}
    request.write_text(json.dumps(spec), encoding="utf-8")
    project = root / "game"
    project.mkdir()
    (project / "project.godot").write_text('config_version=5\n[application]\nconfig/name="Synthetic delivery"\n[rendering]\nrenderer/rendering_method="gl_compatibility"\n', encoding="utf-8")

    def run_example(out, expected_pin=pin, selected_project=project):
        return subprocess.run([sys.executable, str(example), "--forge", str(forge), "--expected-binary-sha256", expected_pin,
                               "--request", str(request), "--project", str(selected_project), "--asset-key", "synthetic",
                               "--out", str(out), "--operation", "prepare-static"], env=env, capture_output=True, text=True, timeout=360)

    wrong = root / "wrong-pin"
    result = run_example(wrong, "0" * 64)
    assert result.returncode and "hash mismatch" in result.stderr and not wrong.exists(), result.stderr
    unlocked = root / "unlocked"
    request.write_text(json.dumps({k: v for k, v in spec.items() if k != "sourceLocks"}), encoding="utf-8")
    result = run_example(unlocked)
    assert result.returncode and "sourceLocks" in result.stderr and not unlocked.exists(), result.stderr
    request.write_text(json.dumps(spec), encoding="utf-8")
    assert not (project / ".forge").exists()

    # Simulate another writer claiming the output while the initial doctor runs.
    # Preflight must not write progress into a directory this invocation never owned.
    contested = root / "contested-output"
    module = runpy.run_path(str(example))
    def concurrent_creator(command, **kwargs):
        if command[1] == "doctor":
            contested.mkdir()
            (contested / "progress.json").write_bytes(b"other writer evidence")
            data = doctor
        else:
            assert command[1:3] == ["storage", "check"], command
            data = {"supported": True}
        return subprocess.CompletedProcess(command, 0, json.dumps({"ok": True, "data": data}), "")
    params = argparse.Namespace(forge=str(forge), request=str(request), project=str(project),
                                out=str(contested), expected_binary_sha256=pin, asset_key="synthetic",
                                operation="prepare-static", timeout=30)
    with patch("subprocess.run", side_effect=concurrent_creator):
        try:
            module["deliver"](params)
        except FileExistsError:
            pass
        else:
            raise AssertionError("Concurrent output creator must win without being overwritten")
    assert (contested / "progress.json").read_bytes() == b"other writer evidence"

    cases = ["storage_probe_cleanup", "pinned_binary_rejection", "reviewed_source_locks_required", "embedded_example", "concurrent_output_preserved"]
    if args.expect_storage_unsupported:
        with tempfile.TemporaryDirectory(prefix="forge-unsupported-project-", dir=scratch_parent) as d:
            unsupported = Path(d)
            (unsupported / "project.godot").write_bytes((project / "project.godot").read_bytes())
            out = root / "unsupported-delivery"
            result = run_example(out, selected_project=unsupported)
            assert result.returncode and "Unsupported output/project filesystem" in result.stderr, result.stderr
            assert not out.exists() and not (unsupported / ".forge").exists()
        cases.append("unsupported_project_rejected_before_production")
    if args.godot:
        out = root / "delivered"
        result = run_example(out)
        assert result.returncode == 0, (result.stdout, result.stderr)
        progress = json.loads((out / "progress.json").read_text())
        assert progress["completed"] and progress["visualReview"] == "not_recorded"
        reports = [c["response"]["data"] for c in progress["commands"] if c["arguments"][:2] == ["job", "report"]]
        assert all(r["providerRequestCount"] == 0 for r in reports)
        receipt = out / "delivery-receipt.json"
        receipt_hash = digest(receipt)
        result = run_example(out)
        assert result.returncode and "must be new" in result.stderr
        assert digest(receipt) == receipt_hash
        shutil.rmtree(out / "jobs")  # Only this synthetic test's freshly-created store.
        verification = call("receipt", "verify", "--path", receipt, "--pack", out / "retained.gsfpack",
                            "--project", project, "--expected-sha256", receipt_hash)
        assert verification["verified"] and verification["installationVerified"]
        assert verification["installationCacheCheck"]["status"] == "verified"
        cases.extend(["native_delivery", "zero_provider", "existing_output_preserved", "receipt_without_job_store"])

        broken = root / "broken-game"
        broken.mkdir()
        (broken / "project.godot").write_text((project / "project.godot").read_text() + '\n[autoload]\nBroken="*res://broken.gd"\n')
        (broken / "broken.gd").write_text("extends Node\nthis is not valid GDScript\n")
        failed = root / "failed-delivery"
        result = run_example(failed, selected_project=broken)
        assert result.returncode, result.stdout
        progress = json.loads((failed / "progress.json").read_text())
        assert not progress["completed"] and progress["prepareJob"] and progress["installJob"]
        assert (failed / "retained.gsfpack").is_dir() and (failed / "prepared-receipt.json").is_file()
        assert not (broken / "addons/forge_assets/synthetic").exists()
        reports = [c["response"]["data"] for c in progress["commands"] if c["arguments"][:2] == ["job", "report"]]
        failure = reports[-1]["job"]
        assert failure["error_code"] == "godot_project_import_failed", failure
        assert "godot.import.stderr.log" in failure["error_summary"]
        assert "prepare_install_plan" in failure["next_actions"]
        cases.append("failed_project_retains_preparation_and_rolls_back_install")

    summary = {"build": doctor["build"], "binarySha256": pin, "cases": cases,
               "storage": probe, "nativeGodot": bool(args.godot), "commands": commands}
    (root / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"passed": cases, "output": str(root)}))


if __name__ == "__main__":
    main()
