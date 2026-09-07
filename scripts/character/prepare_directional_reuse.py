#!/usr/bin/env python3
"""Build a local, reviewable three-direction runtime candidate from approved Packs.

No frame renderer or Provider is used. Original Packs remain immutable; the up
calibration is a single runtime scale for the whole clip. This is a scoped
experiment helper, not a new default Forge character workflow.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import shutil
import statistics
import subprocess
from typing import Any

DIRECTIONS = ("right", "down", "up")
TEMPLATES = Path(__file__).resolve().parent / "godot"


def load(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def ref(path: Path) -> dict:
    return {"path": str(path.resolve()), "sha256": digest(path)}


def checked(path: Path, expected: str) -> dict:
    value = ref(path)
    require(value["sha256"] == expected, f"source hash mismatch: {path}")
    return value


def tree_snapshot(root: Path) -> dict:
    return {str(p.relative_to(root)): digest(p) for p in sorted(root.rglob("*")) if p.is_file()}


def check_output_location(output: Path, sources: list[Path]) -> None:
    require(not output.exists(), "output already exists; use a fresh candidate directory")
    for source in sources:
        require(not output.is_relative_to(source) and not source.is_relative_to(output),
                f"output overlaps protected source: {source}")


def inspect_sources(source_root: Path, gate_path: Path) -> tuple[dict, dict]:
    gate = load(gate_path)
    lock_ref = gate["directionGeometryLock"]
    checked(Path(lock_ref["path"]), lock_ref["sha256"])
    require(set(gate["directions"]) == set(DIRECTIONS), "requires exactly right/down/up")
    entries, snapshots = {}, {}
    for direction in DIRECTIONS:
        root = source_root / direction / f"walk-{direction}-approved-recovery"
        packs = list(root.glob("*.gsfpack"))
        require(len(packs) == 1 and packs[0].is_dir(), f"one directory Pack required: {root}")
        pack = packs[0]
        measurement = gate["directions"][direction]
        manifest_ref = checked(root / "manifest.json", measurement["approvedManifest"]["sha256"])
        checked(pack / "assets/manifest.json", manifest_ref["sha256"])
        manifest = load(root / "manifest.json")
        require(len(manifest["animations"]) == 1, "one animation per source Pack required")
        animation = manifest["animations"][0]
        frames = sorted((root / "frames").glob("frame_*.png"))
        expected = measurement["frames"]
        require(len(frames) == len(expected) == measurement["frameCount"], "frame coverage mismatch")
        require(animation["name"] == f"walk_{direction}" and animation["loop"] is True,
                "source animation name or loop mismatch")
        require(animation["frames"] == list(range(len(frames))), "source frame ordering changed")
        durations = animation["frameDurationsMs"]
        require(len(durations) == len(frames) and all(type(x) is int and x > 0 for x in durations),
                "positive native frame durations required")
        frame_refs = [checked(p, item["sha256"]) for p, item in zip(frames, expected)]
        hashes = [item["sha256"] for item in frame_refs]
        for p, sha in zip(frames, hashes):
            checked(pack / "assets/frames" / p.name, sha)
        document = load(pack / "forgepack.json")
        metadata = document["source"]["metadata"]
        approval_path = pack / "quality/animation-human-review.json"
        approval_ref = checked(approval_path, metadata["animationHumanReviewSha256"])
        lineage_ref = checked(pack / "quality/animation-review-lineage.json",
                              metadata["animationReviewLineageSha256"])
        approval = load(approval_path)
        certification = load(pack / "quality/recovery-certification.json")
        require(approval["status"] == "approved" and len(approval["checks"]) >= 6
                and all(v is True for v in approval["checks"].values()), "source approval incomplete")
        for item in (approval, certification):
            require(item["frameSha256"] == hashes, "approved frame identity mismatch")
            require(item["frameDurationsMs"] == durations, "approved frame timing mismatch")
        require(metadata["approvedFrameSha256"] == hashes, "Pack source chain mismatch")
        require(document["animations"] == manifest["animations"], "Pack animation differs from manifest")
        external_refs = [checked(Path(certification[key + "Path"]), certification[key + "Sha256"])
                         for key in ("approvedReview", "recoveredReplaySummary")]
        stable_values = [x["stableScaleDistancePx"] for x in expected]
        median = statistics.median(stable_values)
        require(math.isclose(median, measurement["stableScaleBand"]["medianDistancePx"], abs_tol=1e-6),
                "geometry median does not match frame measurements")
        entries[direction] = {
            "pack": str(pack), "packManifest": ref(pack / "forgepack.json"),
            "manifest": manifest_ref, "sourceApproval": approval_ref, "sourceLineage": lineage_ref,
            "sourceCertification": ref(pack / "quality/recovery-certification.json"),
            "originalEvidence": external_refs, "frames": frame_refs,
            "frameCount": len(frames), "frameDurationsMs": durations,
            "sourceCycleDurationMs": sum(durations), "sourceNativePivotPx": {
                "x": manifest["anchor"]["x"], "y": manifest["anchor"]["y"]},
            "stableScaleMedianPx": median, "sourceGeometryGate": measurement["sharedVisualScaleGate"],
        }
        snapshots[str(root)] = tree_snapshot(root)
    return entries, snapshots


def build_contract(entries: dict) -> dict:
    target = entries["right"]["stableScaleMedianPx"]
    scale = target / entries["up"]["stableScaleMedianPx"]
    require(math.isfinite(scale) and 0.8 <= scale <= 1.2, "calibration outside experiment bounds")
    directions = {}
    for direction, source in entries.items():
        factor = scale if direction == "up" else 1.0
        directions[direction] = {
            key: source[key] for key in ("frameCount", "frameDurationsMs", "sourceCycleDurationMs",
                                        "sourceNativePivotPx", "stableScaleMedianPx")}
        directions[direction].update({
            "scene": f"res://addons/forge_assets/walk_{direction}/forge_animated_sprite.tscn",
            "animation": f"walk_{direction}", "calibrationScale": factor, "playbackSpeedScale": 1.0,
            "stableScaleGatePassed": source["sourceGeometryGate"]["passed"],
            "stableScaleRelativeError": source["sourceGeometryGate"]["relativeError"],
            "calibratedStableScalePx": source["stableScaleMedianPx"] * factor,
            "calibrationFitRelativeError": abs(source["stableScaleMedianPx"] * factor - target) / target,
        })
    return {
        "schemaVersion": "1", "profile": "directional-runtime-reuse-candidate@1.0.0",
        "common": {"visualScale": 0.22, "movementSpeedPixelsPerSecond": 80.0,
                   "collision": {"width": 72.0, "height": 96.0, "offsetX": 0.0, "offsetY": -48.0}},
        "directions": directions, "providerRequestCountThisOperation": 0,
        "policy": {"sourcePixelsModified": False, "nativeDurationsPreserved": True,
                   "transform": "one fixed uniform runtime scale per direction; native pivot unchanged",
                   "calibrationEvidence": "fitted scarf-centroid/foot-plane proxy, not independent anatomy validation",
                   "sourceGeometryVerdictsPreserved": True, "mirroringApplied": False},
        "visualReview": "pending", "productionPromoted": False,
    }


def cli(binary: Path, args: list[str], env: dict, log: Path) -> dict:
    result = subprocess.run([str(binary), *args, "--json"], env=env, capture_output=True, text=True, timeout=180)
    try:
        envelope = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise ValueError(f"invalid CLI JSON for {args[:2]}: {result.stderr[-2000:]}") from error
    write(log, envelope)
    require(result.returncode == 0 and envelope.get("ok") is True,
            f"CLI failed for {args[:2]}: {envelope.get('error', result.stderr[-2000:])}")
    return envelope["data"]


def local_plan_only(plan: dict) -> None:
    estimate = plan["estimate"]
    require(estimate.get("providerRequestEstimate") == 0 and estimate.get("maximumProviderRequests") == 0,
            "refusing a plan without explicit zero Provider bounds")


def prepare(args: argparse.Namespace) -> None:
    source_root, gate_path, output = args.source_root.resolve(), args.reuse_gate.resolve(), args.output.resolve()
    check_output_location(output, [source_root, gate_path.parent])
    entries, snapshots = inspect_sources(source_root, gate_path)
    contract = build_contract(entries)
    output.mkdir(parents=True)
    project = output / "godot-project"
    project.mkdir()
    for name in ("project.godot", "main.tscn", "directional_reuse_review.gd"):
        shutil.copy2(TEMPLATES / name, project / name)
    write(project / "runtime-contracts.json", contract)
    write(output / "source-lock.json", {"reuseGate": ref(gate_path), "directions": entries,
                                       "sourceSnapshots": snapshots})
    env = dict(os.environ)
    for key in ("FORGE_REAL_PROVIDER_ACCEPT", "FORGE_REAL_PROVIDER_MAX_REQUESTS", "FORGE_REAL_PROVIDER_MAX_COST_TICKS"):
        env.pop(key, None)
    env.update({"FORGE_JOB_STORE": str(output / "stores/jobs"),
                "FORGE_PLAN_STORE": str(output / "stores/plans"), "FORGE_GODOT_PATH": str(args.godot.resolve())})
    logs = output / "cli-evidence"
    install_jobs = []
    for direction, source in entries.items():
        prefix = logs / direction
        valid = cli(args.forge.resolve(), ["pack", "validate", "--path", source["pack"]], env,
                    prefix / "pack-validate.json")
        require(valid.get("valid") is True, f"invalid Pack: {direction}")
        plan = cli(args.forge.resolve(), ["godot", "plan-install", "--pack", source["pack"],
                   "--project", str(project), "--target", f"addons/forge_assets/walk_{direction}",
                   "--asset-key", f"walk_{direction}"], env, prefix / "install-plan.json")
        local_plan_only(plan)
        job = cli(args.forge.resolve(), ["plan", "execute", "--token", plan["token"], "--wait"], env,
                  prefix / "install-execute.json")
        require(job["lifecycle_state"] == "succeeded", f"install did not succeed: {direction}")
        report = cli(args.forge.resolve(), ["job", "report", "--id", job["job_id"]], env,
                     prefix / "install-report.json")
        require(report.get("providerRequestOccurred") is False and report.get("providerRequestCount") == 0,
                "install report did not confirm zero Provider requests")
        install_jobs.append({"direction": direction, "jobId": job["job_id"],
                             "providerRequestCount": report["providerRequestCount"]})
    cli(args.forge.resolve(), ["project", "inspect", "--project", str(project)], env,
        logs / "project-inspect.json")
    for path, before in snapshots.items():
        require(tree_snapshot(Path(path)) == before, f"source changed during installation: {path}")
    write(output / "preparation-report.json", {
        "status": "prepared", "project": str(project), "sourceFrameCount": sum(x["frameCount"] for x in entries.values()),
        "upCalibrationScale": contract["directions"]["up"]["calibrationScale"],
        "sourceAssetsUnchanged": True, "installJobs": install_jobs,
        "providerRequestCountThisOperation": sum(x["providerRequestCount"] for x in install_jobs),
        "runtimeContract": ref(project / "runtime-contracts.json"), "sourceLock": ref(output / "source-lock.json"),
        "visualReview": "pending", "productionPromoted": False,
    })
    print(json.dumps({"output": str(output), "status": "prepared", "frames": sum(x["frameCount"] for x in entries.values()),
                      "providerRequestCountThisOperation": 0}, ensure_ascii=False))


def verify(output: Path) -> None:
    from PIL import Image

    output = output.resolve()
    prepared = load(output / "preparation-report.json")
    checked(Path(prepared["sourceLock"]["path"]), prepared["sourceLock"]["sha256"])
    checked(Path(prepared["runtimeContract"]["path"]), prepared["runtimeContract"]["sha256"])
    lock = load(output / "source-lock.json")
    checked(Path(lock["reuseGate"]["path"]), lock["reuseGate"]["sha256"])
    for path, before in lock["sourceSnapshots"].items():
        require(tree_snapshot(Path(path)) == before, f"source mutation detected: {path}")
    project = output / "godot-project"
    texture_checks = []
    for direction, source in lock["directions"].items():
        installed = project / f"addons/forge_assets/walk_{direction}"
        for texture in sorted((Path(source["pack"]) / "assets").glob("sprite_sheet*.png")):
            destination = installed / texture.name
            texture_checks.append(checked(destination, digest(texture)))
    text_resources = []
    for path in sorted(project.rglob("*")):
        if path.suffix not in (".tres", ".tscn"):
            continue
        raw = path.read_bytes()
        require(len(raw) < 1024 * 1024 and not any(token in raw for token in
                (b"PackedByteArray", b"data:image/", b"ImageTexture.create_from_image", b"[sub_resource type=\"Image\"")),
                f"embedded raster or oversized resource: {path}")
        text_resources.append(ref(path))
    runtime_path = project / "qa-output/runtime-report.json"
    runtime = load(runtime_path)
    require(runtime.get("runtimePassed") is True, "Godot runtime verification failed")
    require(runtime.get("visualReview") == "pending", "candidate must not inherit source visual approval")
    pixel_checks = []
    for direction, source in lock["directions"].items():
        textures = runtime["nativeResources"][direction]["frameTextures"]
        require(len(textures) == source["frameCount"], "runtime texture coverage differs from source")
        for index, (texture, frame) in enumerate(zip(textures, source["frames"])):
            require(texture["frameIndex"] == index and texture["textureClass"] == "AtlasTexture",
                    "runtime frame order or texture type changed")
            resource = texture["atlasResourcePath"]
            require(resource.startswith("res://"), "external runtime texture path")
            atlas_path = (project / resource.removeprefix("res://")).resolve()
            require(atlas_path.is_relative_to(project), "runtime texture escapes project")
            region = texture["region"]
            margin = texture["margin"]
            require(all(value == 0 for value in margin.values()), "unexpected atlas frame margin")
            require(all(float(value).is_integer() for value in region.values()), "non-integer atlas region")
            x, y, width, height = (int(region[key]) for key in ("x", "y", "width", "height"))
            with Image.open(atlas_path) as atlas, Image.open(frame["path"]) as original:
                require(x >= 0 and y >= 0 and width > 0 and height > 0 and
                        x + width <= atlas.width and y + height <= atlas.height, "invalid atlas region")
                pixels = atlas.crop((x, y, x + width, y + height)).convert("RGBA")
                expected = original.convert("RGBA")
                require(pixels.size == expected.size and pixels.tobytes() == expected.tobytes(),
                        f"installed atlas pixels differ: {direction} frame {index}")
                pixel_checks.append({"direction": direction, "frameIndex": index,
                                     "rgbaSha256": hashlib.sha256(pixels.tobytes()).hexdigest(), "passed": True})
    write(output / "verification-report.json", {
        "status": "passed", "sourceAssetsUnchanged": True,
        "sourceFrameCount": sum(x["frameCount"] for x in lock["directions"].values()),
        "installedTextureChecks": texture_checks, "textResourceChecks": text_resources,
        "runtimeFramePixelChecks": pixel_checks,
        "projectFiles": [ref(project / name) for name in
                         ("project.godot", "main.tscn", "directional_reuse_review.gd")],
        "runtimeReport": ref(runtime_path), "runtimeContract": prepared["runtimeContract"],
        "providerRequestCountThisOperation": prepared["providerRequestCountThisOperation"],
        "visualReview": "pending", "productionPromoted": False,
    })
    print(json.dumps({"status": "passed", "installedTextures": len(texture_checks),
                      "textResources": len(text_resources), "visualReview": "pending"}))


def package_output(output: Path) -> None:
    import zipfile

    output = output.resolve()
    verification = load(output / "verification-report.json")
    require(verification["status"] == "passed", "verify the candidate before packaging")
    for item in [verification["runtimeReport"], verification["runtimeContract"],
                 *verification["projectFiles"], *verification["installedTextureChecks"],
                 *verification["textResourceChecks"]]:
        checked(Path(item["path"]), item["sha256"])
    archive = output / "forge-directional-reuse-review.zip"
    require(not archive.exists(), "delivery archive already exists; retain it and use a new candidate")
    launcher = output / "open-review.command"
    shutil.copy2(TEMPLATES / "open-review.command", launcher)
    launcher.chmod(0o755)
    project = output / "godot-project"
    with zipfile.ZipFile(archive, "x", zipfile.ZIP_DEFLATED, compresslevel=6) as bundle:
        bundle.write(launcher, "forge-directional-reuse-review/open-review.command")
        for path in sorted(project.rglob("*")):
            relative = path.relative_to(project)
            if path.is_file() and ".godot" not in relative.parts:
                bundle.write(path, "forge-directional-reuse-review/godot-project/" + str(relative))
    delivery = {**ref(archive), "sizeBytes": archive.stat().st_size,
                "launcher": ref(launcher), "visualReview": "pending", "productionPromoted": False}
    write(output / "delivery.json", delivery)
    print(json.dumps(delivery, ensure_ascii=False))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    build = commands.add_parser("prepare")
    build.add_argument("--source-root", type=Path, required=True)
    build.add_argument("--reuse-gate", type=Path, required=True)
    build.add_argument("--output", type=Path, required=True)
    build.add_argument("--forge", type=Path, required=True)
    build.add_argument("--godot", type=Path, required=True)
    check = commands.add_parser("verify")
    check.add_argument("--output", type=Path, required=True)
    package = commands.add_parser("package")
    package.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.command == "prepare":
            prepare(args)
        elif args.command == "verify":
            verify(args.output)
        else:
            package_output(args.output)
    except (ValueError, KeyError, OSError, subprocess.TimeoutExpired) as error:
        parser.exit(1, f"directional reuse failed: {error}\n")


if __name__ == "__main__":
    main()
