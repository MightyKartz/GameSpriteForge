#!/usr/bin/env python3
"""Offline CLI contracts learned from Sword, using only synthetic local assets.

Requires a built Forge CLI and optionally Godot 4.6.x. No Provider requests or
consumer projects are used. --output must name a new directory; it retains full
command evidence while a compact summary describes the verified contracts.
"""

import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import zlib


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def inventory(path):
    if not path.exists():
        return {}
    return {str(item.relative_to(path)): digest(item) for item in sorted(path.rglob("*")) if item.is_file()}


def png(path, width=64, height=64, sheet=False):
    """RGBA PNG with a solid subject, soft alpha, and invisible RGB evidence."""
    def chunk(kind, value):
        return struct.pack(">I", len(value)) + kind + value + struct.pack(">I", zlib.crc32(kind + value))

    rows = bytearray()
    for y in range(height):
        rows.append(0)
        for x in range(width):
            cell_x = x % 64 if sheet else x
            if 24 <= cell_x < 40 and 20 <= y < 52:
                color = (190, 90, 40, 255)
            elif cell_x == 23 and 20 <= y < 52:
                color = (190, 90, 40, 8)
            elif not sheet and x == 0 and y == 30:
                color = (40, 90, 200, 1)
            else:
                color = (77, 12, 90, 0)
            rows.extend(color)
    path.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
                     + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b""))


class Check:
    def __init__(self, forge, root, godot):
        self.forge, self.root, self.godot = forge, root, godot
        self.env = dict(os.environ, FORGE_JOB_STORE=str(root / "jobs"), FORGE_PLAN_STORE=str(root / "plans"),
                        FORGE_CACHE_STORE=str(root / "cache"), FORGE_REAL_PROVIDER_MAX_REQUESTS="0")
        self.env.pop("FORGE_REAL_PROVIDER_ACCEPT", None)
        if godot:
            self.env["FORGE_GODOT_PATH"] = str(godot)
        self.commands = []
        self.cases = []
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
            raise AssertionError(f"{label}: expected one JSON envelope, got exit {result.returncode}: {result.stderr}") from error
        assert (result.returncode != 0) == fail, (label, result.returncode, envelope, result.stderr)
        assert envelope.get("ok") is (not fail), (label, envelope)
        return envelope.get("error") if fail else envelope["data"]

    def stores(self):
        return {name: inventory(self.root / name) for name in ("plans", "jobs")}

    def prepare(self, label, operation, request):
        path = self.root / "specs" / f"{label}.json"
        save(path, request)
        plan = self.call(label + "-plan", "plan", operation, "--request", path)
        assert plan["estimate"]["providerRequestEstimate"] == plan["estimate"]["maximumProviderRequests"] == 0
        job = self.call(label + "-execute", "plan", "execute", "--token", plan["token"], "--wait")
        assert job["lifecycle_state"] == "succeeded", (label, job)
        pack = Path(next(item["path"] for item in job["artifacts"] if item["kind"] == "gsfpack"))
        self.call(label + "-validate", "pack", "validate", "--path", pack)
        return job, pack

    def reject_source_plan(self, label, operation, request):
        path = self.root / "specs" / f"{label}.json"
        save(path, request)
        before = self.stores()
        self.call(label, "plan", operation, "--request", path, fail=True)
        assert self.stores() == before, f"{label}: rejected source created or changed a plan/job"

    def inspect_sources(self, source):
        before = inventory(source.parent)
        stores = self.stores()
        report = self.call("source-inspect", "source", "inspect", "--path", source, "--frame-width", 32, "--frame-height", 32)
        assert report["sha256"] == digest(source)
        assert (report["width"], report["height"], report["pngColorType"]) == (64, 64, 6)
        assert report["hasAlphaChannel"] and report["alpha"]["transparentPixelsWithRgb"] > 0
        assert report["grid"]["divisible"] and len(report["grid"]["cells"]) == 4
        assert any(cell["touchesEdgeAlpha1"] for cell in report["grid"]["cells"])
        assert report["visualReview"] != "approved"
        uneven = self.call("source-grid-remainder", "source", "inspect", "--path", source, "--frame-width", 30, "--frame-height", 32)
        assert uneven["grid"]["divisible"] is False and uneven["grid"]["remainderX"] == 4
        previews = self.root / "inspection-previews"
        self.call("source-previews", "source", "inspect", "--path", source, "--preview-dir", previews)
        preview_inventory = inventory(previews)
        assert len(preview_inventory) == 2
        self.call("source-preview-no-overwrite", "source", "inspect", "--path", source, "--preview-dir", previews, fail=True)
        assert inventory(previews) == preview_inventory
        assert inventory(source.parent) == before and self.stores() == stores
        self.cases.append("source inspection measures pixels/grid and preserves sources, stores and existing previews")

    def install(self, label, pack, project, key=None):
        arguments = ["godot", "plan-install", "--pack", pack, "--project", project]
        if key is not None:
            arguments.extend(["--asset-key", key])
        plan = self.call(label + "-plan", *arguments)
        pending = json.loads((self.root / "plans" / f"{plan['token']}.pending.json").read_text())
        expected_key = key or json.loads((pack / "forgepack.json").read_text())["id"]
        assert pending["operation"]["request"]["target"] == f"addons/forge_assets/{expected_key}", pending
        job = self.call(label + "-execute", "plan", "execute", "--token", plan["token"], "--wait")
        assert job["lifecycle_state"] == "succeeded", job
        target = project / "addons/forge_assets" / expected_key
        assert (target / ".forge-install.json").is_file()
        before = inventory(project)
        verification = self.call(label + "-verify", "godot", "verify-install", "--project", project, "--asset-key", expected_key)
        assert verification["readOnly"] is True and verification["verifiedFiles"] > 0
        assert verification["cacheCheck"]["status"] == "verified", verification
        assert inventory(project) == before, "verify-install modified the project or Godot cache"
        return job, target, expected_key

    def receipt(self, label, prepare_job, install_job=None, review=None):
        receipt_path = self.root / "delivery" / f"{label}.json"
        receipt_path.parent.mkdir(exist_ok=True)
        arguments = ["receipt", "export", "--job", prepare_job["job_id"], "--out", receipt_path]
        if install_job:
            arguments.extend(["--install-job", install_job["job_id"]])
        if review:
            arguments.extend(["--review", review])
        self.call(label + "-export", *arguments)
        content = json.loads(receipt_path.read_text())
        assert content["schemaVersion"] and content["prepare"]["job"]["job_id"] == prepare_job["job_id"]
        assert content["pack"]["files"] and content["pack"]["sha256"]
        assert content["prepare"]["execution"] is not None, "new CLI Job is missing execution identity"
        receipt_sha = digest(receipt_path)
        self.call(label + "-verify", "receipt", "verify", "--path", receipt_path, "--expected-sha256", receipt_sha)
        self.call(label + "-wrong-receipt-sha", "receipt", "verify", "--path", receipt_path, "--expected-sha256", "0" * 64, fail=True)
        before = receipt_path.read_bytes()
        self.call(label + "-no-overwrite", *arguments, fail=True)
        assert receipt_path.read_bytes() == before
        return receipt_path

    def run(self):
        doctor = self.call("doctor", "doctor")
        inputs = self.root / "specs" / "inputs"
        inputs.mkdir(parents=True)
        source = inputs / "jade.png"
        png(source)
        self.inspect_sources(source)
        other = inputs / "stone.png"
        png(other)
        static = {"schemaVersion": "1", "kind": "prop_set", "id": "stable-jade", "name": "灵玉与石头",
                  "license": "CC0-1.0", "sampling": "linear", "canvasSize": 64,
                  "items": [{"id": "jade", "name": "灵玉", "path": "inputs/jade.png"},
                            {"id": "stone", "name": "石头", "path": "inputs/stone.png"}],
                  "sourceLocks": [{"path": "inputs/jade.png", "sha256": digest(source)},
                                  {"path": "inputs/stone.png", "sha256": digest(other)}]}
        wrong = copy.deepcopy(static)
        wrong["sourceLocks"][0]["sha256"] = "0" * 64
        self.reject_source_plan("static-wrong-source-hash", "prepare-static", wrong)
        missing = copy.deepcopy(static)
        missing["sourceLocks"].pop()
        self.reject_source_plan("static-incomplete-source-locks", "prepare-static", missing)
        duplicate = copy.deepcopy(static)
        duplicate["sourceLocks"].append(copy.deepcopy(duplicate["sourceLocks"][0]))
        self.reject_source_plan("static-duplicate-source-lock", "prepare-static", duplicate)
        malformed = copy.deepcopy(static)
        malformed["sourceLocks"][0]["sha256"] = "bad-hash"
        self.reject_source_plan("static-malformed-source-lock", "prepare-static", malformed)
        uppercase = copy.deepcopy(static)
        uppercase["sourceLocks"][0]["sha256"] = uppercase["sourceLocks"][0]["sha256"].upper()
        static_job, static_pack = self.prepare("static", "prepare-static", uppercase)
        self.cases.append("approved static source hashes resolve relative to request and reject mismatched/incomplete/duplicate locks before planning")

        sheet = inputs / "idle-sheet.png"
        png(sheet, width=126, sheet=True)
        animation = {"schemaVersion": "1", "input": {"kind": "sprite_sheet", "path": "inputs/idle-sheet.png", "split": {
            "mode": "fixed_grid", "frameWidth": 64, "frameHeight": 64, "columns": 2, "rows": 1, "sourcePaddingRightPx": 2}},
            "metadata": {"name": "One-shot local fixture", "animation": "cast", "fps": 8, "loop": False, "frameDurationsMs": [70, 150]},
            "rendering": {"textureFilter": "linear", "pixelSnap": False},
            "normalize": {"mode": "preserve_source", "margin": 0, "marginBottom": 0, "alphaThreshold": 0,
                          "manualAnchor": {"x": 32, "y": 52, "lockedByUser": True}},
            "quality": {"requireGameReady": True},
            "sourceLocks": [{"path": "inputs/idle-sheet.png", "sha256": digest(sheet)}]}
        wrong_animation = copy.deepcopy(animation)
        wrong_animation["sourceLocks"][0]["sha256"] = "0" * 64
        self.reject_source_plan("animation-wrong-source-hash", "prepare-asset", wrong_animation)
        animation_job, animation_pack = self.prepare("animation", "prepare-asset", animation)
        manifest = json.loads((animation_pack / "assets/manifest.json").read_text())
        assert manifest["animations"][0]["loop"] is False
        assert manifest["animations"][0]["frameDurationsMs"] == [70, 150]
        quality = json.loads((animation_pack / "quality-report.json").read_text())
        assert "trim_loop_range" not in json.dumps(quality["recommendations"])
        self.cases.append("locked whole-sheet preprocessing preserves timing and non-loop semantics")

        character = copy.deepcopy(animation)
        character["schemaVersion"] = "2"
        character["metadata"] = {"name": "Shared source character", "defaultAnimation": "idle"}
        character_input = character.pop("input")
        character["animations"] = [{"name": "idle", "input": character_input, "fps": 8, "loop": True},
                                   {"name": "cast", "input": copy.deepcopy(character_input), "fps": 8, "loop": False}]
        character_wrong = copy.deepcopy(character)
        character_wrong["sourceLocks"][0]["sha256"] = "0" * 64
        self.reject_source_plan("character-wrong-source-hash", "prepare-character", character_wrong)
        self.prepare("character-shared-source", "prepare-character", character)
        self.cases.append("multi-action source locks cover reused files once")

        changed_path = self.root / "specs" / "changed-after-plan.json"
        save(changed_path, static)
        change_plan = self.call("changed-after-plan-plan", "plan", "prepare-static", "--request", changed_path)
        original = source.read_bytes()
        source.write_bytes(original + b"changed after approval")
        before = self.stores()
        self.call("changed-after-plan-refused", "plan", "execute", "--token", change_plan["token"], "--wait", fail=True)
        assert self.stores() == before
        source.write_bytes(original)
        self.cases.append("source changes after planning fail before token consumption or Job creation")

        project = self.root / "godot-project"
        project.mkdir()
        (project / "project.godot").write_text('config_version=5\n[application]\nconfig/name="Sword feedback synthetic QA"\n[rendering]\nrenderer/rendering_method="gl_compatibility"\n')
        copied_pack = self.root / "delivery" / "同名素材.gsfpack"
        copied_pack.parent.mkdir()
        shutil.copytree(static_pack, copied_pack)
        before = self.stores()
        self.call("illegal-stable-key", "godot", "plan-install", "--pack", copied_pack, "--project", project, "--asset-key", "../escape", fail=True)
        assert self.stores() == before
        install_job = None
        if self.godot:
            _, default_target, _ = self.install("pack-id-default", copied_pack, project)
            default_before = inventory(default_target)
            install_job, target, key = self.install("explicit-stable-key", copied_pack, project, "jade-second")
            assert inventory(default_target) == default_before and target != default_target
            self.cases.append("same-named Pack installs use stable Pack id or explicit asset key without target collisions")
            resource = next(target.rglob("*.tscn"))
            resource_bytes = resource.read_bytes()
            resource.write_bytes(resource_bytes + b"\n# unreviewed resource change\n")
            before = inventory(project)
            self.call("installed-resource-tamper", "godot", "verify-install", "--project", project, "--asset-key", key, fail=True)
            assert inventory(project) == before
            resource.write_bytes(resource_bytes)
            texture = next((target / "items").glob("*.png"))
            texture_bytes = texture.read_bytes()
            texture.write_bytes(texture_bytes + b"unreviewed texture bytes")
            self.call("installed-texture-tamper", "godot", "verify-install", "--project", project, "--asset-key", key, fail=True)
            texture.write_bytes(texture_bytes)
            self.cases.append("read-only installation verification rejects changed native resources and textures")
            snapshot = json.loads((target / ".forge-install.json").read_text())
            assert snapshot["schemaVersion"] == "2" and snapshot["cacheFiles"] and snapshot["importFiles"]
            cache = project / next(item["path"] for item in snapshot["cacheFiles"] if item["path"].endswith(".ctex"))
            cache_bytes = cache.read_bytes()
            cache.write_bytes(cache_bytes + b"stale imported cache")
            before = inventory(project)
            self.call("installed-cache-tamper", "godot", "verify-install", "--project", project, "--asset-key", key, fail=True)
            assert inventory(project) == before, "cache audit tried to repair a changed cache"
            cache.write_bytes(cache_bytes)
            sidecar = target / snapshot["importFiles"][0]["path"]
            sidecar_bytes = sidecar.read_bytes()
            sidecar.write_bytes(sidecar_bytes + b"\n# changed import settings\n")
            self.call("installed-import-routing-tamper", "godot", "verify-install", "--project", project, "--asset-key", key, fail=True)
            sidecar.write_bytes(sidecar_bytes)
            self.cases.append("Godot imported cache bytes and import routing are verified without rebuilding the cache")

        review = self.root / "review.json"
        pack_hash = next(item["sha256"] for item in static_job["artifacts"] if item["kind"] == "gsfpack")
        save(review, {"schemaVersion": "1", "packSha256": pack_hash, "status": "pending", "reviewer": "Synthetic test fixture",
                      "note": "Synthetic structural QA is not human visual approval."})
        static_receipt = self.receipt("static-receipt", static_job, install_job, review)
        animation_receipt = self.receipt("animation-receipt", animation_job)
        animation_receipt_content = json.loads(animation_receipt.read_text())
        assert "source_transform" in json.dumps(animation_receipt_content["prepare"]["reports"]), "receipt omitted source-transform evidence"
        assert json.loads(static_receipt.read_text())["review"]["status"] == "pending"
        self.cases.append("receipts retain execution, quality/source-transform and separate pending review evidence")

        execution_path = Path(animation_job["job_dir"]) / "execution-provenance.json"
        retained_execution = self.root / "retained-execution-provenance.json"
        execution_path.rename(retained_execution)
        legacy_receipt = self.root / "delivery" / "legacy-receipt.json"
        before = self.stores()
        self.call("legacy-receipt-export", "receipt", "export", "--job", animation_job["job_id"], "--out", legacy_receipt)
        assert self.stores() == before, "receipt export backfilled a legacy Job's missing producer"
        legacy_content = json.loads(legacy_receipt.read_text())
        assert legacy_content["prepare"]["execution"] is None and legacy_content["exporter"]
        verified_legacy = self.call("legacy-receipt-verify", "receipt", "verify", "--path", legacy_receipt)
        assert verified_legacy["producerKnown"] is False
        retained_execution.rename(execution_path)
        self.cases.append("legacy Jobs retain unknown producer identity without borrowing the exporter identity or modifying the Job")

        changed_report = copy.deepcopy(animation_receipt_content)
        changed_report["prepare"]["reports"][0]["text"] += " "
        changed_report_path = self.root / "delivery" / "changed-report-receipt.json"
        save(changed_report_path, changed_report)
        self.call("receipt-embedded-report-tamper", "receipt", "verify", "--path", changed_report_path, fail=True)
        self.cases.append("embedded report bytes are verified independently of the original Job files")

        portable_animation = self.root / "delivery" / "animation.gsfpack"
        shutil.copytree(animation_pack, portable_animation)
        archived_jobs = self.root / "removed-job-store"
        (self.root / "jobs").rename(archived_jobs)
        before = inventory(self.root / "delivery")
        self.call("receipt-without-job-store", "receipt", "verify", "--path", animation_receipt, "--pack", portable_animation)
        self.call("static-receipt-without-job-store", "receipt", "verify", "--path", static_receipt, "--pack", copied_pack,
                  *(["--project", project] if install_job else []))
        assert not (self.root / "jobs").exists() and inventory(self.root / "delivery") == before
        self.cases.append("portable receipts verify with a Pack override after the original Job store is removed")

        pack_texture = portable_animation / "assets/sprite_sheet.png"
        texture_bytes = pack_texture.read_bytes()
        pack_texture.write_bytes(texture_bytes + b"unreviewed Pack mutation")
        self.call("receipt-pack-tamper", "receipt", "verify", "--path", animation_receipt, "--pack", portable_animation, fail=True)
        pack_texture.write_bytes(texture_bytes)
        self.cases.append("receipt verification rejects changed Pack bytes")
        if install_job:
            project_before = inventory(project)
            relocated_project = self.root / "relocated-project"
            shutil.copytree(project, relocated_project)
            self.call("receipt-project-override", "receipt", "verify", "--path", static_receipt, "--pack", copied_pack, "--project", relocated_project)
            assert inventory(project) == project_before
            self.cases.append("receipt verification supports a relocated project without mutating the original")
            shutil.rmtree(relocated_project / ".godot")
            before = inventory(relocated_project)
            uncached = self.call("receipt-project-without-cache", "receipt", "verify", "--path", static_receipt, "--pack", copied_pack, "--project", relocated_project)
            assert uncached["installationCacheCheck"]["status"] == "not_materialized" and uncached["installationCacheCheck"]["materialized"] is False
            assert inventory(relocated_project) == before
            self.cases.append("copied projects without cache explicitly require import before native loading and audit creates no cache")
        summary = {"ok": True, "forge": str(self.forge), "forgeSha256": digest(self.forge), "doctor": doctor,
                   "godot": str(self.godot) if self.godot else None, "providerRequests": 0,
                   "source": "synthetic local PNGs only; no Sword files or consumer pins", "cases": self.cases,
                   "commands": self.commands, "godotChecksSkipped": self.godot is None}
        save(self.root / "summary.json", summary)
        print(json.dumps({key: value for key, value in summary.items() if key not in ("commands", "doctor")}, ensure_ascii=False, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--forge", type=Path, required=True)
    parser.add_argument("--godot", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    forge = Path(os.path.abspath(args.forge))
    godot = Path(os.path.abspath(args.godot)) if args.godot else None
    if args.output:
        root = args.output.resolve()
        root.mkdir(parents=True, exist_ok=False)
        Check(forge, root, godot).run()
    else:
        with tempfile.TemporaryDirectory(prefix="forge-sword-feedback-") as temporary:
            Check(forge, Path(temporary), godot).run()


if __name__ == "__main__":
    main()
