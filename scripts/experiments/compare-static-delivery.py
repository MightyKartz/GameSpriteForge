#!/usr/bin/env python3
"""Measure ready-PNG native import versus Forge delivery in NEW isolated projects.

No model calls, consumer modifications, or productivity claims. See
docs/qa/asset-delivery-comparison.md for the shared contract and exclusions.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import statistics
import subprocess
import sys
import time


ROOT = Path(__file__).resolve().parents[2]
CHECKER = Path(__file__).with_name("check-static-delivery.gd")
TARGET = Path("addons/forge_assets/pilot/items")
PROJECT = 'config_version=5\n[application]\nconfig/name="Delivery comparison"\n[rendering]\nrenderer/rendering_method="gl_compatibility"\n'


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def require_acceptance(result, count):
    marker = "DELIVERY_CHECK_PASSED:" + str(count)
    if result.returncode or marker not in result.stdout.splitlines() or "SCRIPT ERROR" in result.stdout + result.stderr:
        raise ValueError("Native acceptance failed; see retained command log")


def check_inventory(target, items):
    if {p.relative_to(target).as_posix() for p in target.rglob("*.png")} != {p.name for p in items}:
        raise ValueError("Installed PNG inventory differs (missing or stale files)")
    for source in items:
        if digest(target / source.name) != digest(source):
            raise ValueError("Installed PNG bytes differ: " + source.name)


def summarize(records):
    summary = {}
    for phase in ("first", "update"):
        pairs = {}
        for row in records:
            if row["phase"] == phase:
                pairs.setdefault(row["trial"], {})[row["arm"]] = row
        valid = [p for p in pairs.values() if set(p) == {"native", "forge"}
                 and all(r["passed"] for r in p.values())]
        summary[phase] = {
            "completePassingPairs": len(valid), "observedPairs": len(pairs),
            "medianSeconds": {arm: statistics.median(p[arm]["seconds"] for p in valid) if valid else None
                              for arm in ("native", "forge")},
            "failedAttempts": sum(not r["passed"] for p in pairs.values() for r in p.values()),
        }
    return summary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--forge", type=Path, required=True)
    parser.add_argument("--godot", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--trials", type=int, default=3)
    args = parser.parse_args()
    if not 1 <= args.trials <= 20:
        parser.error("trials must be between 1 and 20")
    output = args.output.absolute()
    output.mkdir(parents=True, exist_ok=False)
    forge, godot = args.forge.absolute(), args.godot.absolute()
    sources = sorted((ROOT / "docs/media/showcase/sprites").glob("*.png"))
    if len(sources) != 8:
        raise ValueError("Expected the eight public showcase PNGs")
    source_hashes = {p.name: digest(p) for p in sources}
    env = dict(os.environ, FORGE_GODOT_PATH=str(godot), FORGE_REAL_PROVIDER_MAX_REQUESTS="0",
               FORGE_CONFIG_DIR=str(output / "config"), FORGE_JOB_STORE=str(output / "jobs"),
               FORGE_PLAN_STORE=str(output / "plans"))
    env.pop("FORGE_REAL_PROVIDER_ACCEPT", None)
    logs = output / "logs"
    logs.mkdir()
    calls = []

    def run(command, cwd=None):
        index = len(calls) + 1
        started = time.perf_counter()
        try:
            result = subprocess.run(list(map(str, command)), env=env, cwd=cwd, capture_output=True,
                                    encoding="utf-8", timeout=600)
            record = {"argv": list(map(str, command)), "exit": result.returncode,
                      "stdout": result.stdout, "stderr": result.stderr}
        except subprocess.TimeoutExpired as error:
            record = {"argv": list(map(str, command)), "exit": None, "error": str(error)}
            result = None
        record["seconds"] = time.perf_counter() - started
        write_json(logs / f"{index:03d}.json", record)
        calls.append({"log": f"logs/{index:03d}.json", "exit": record["exit"], "seconds": record["seconds"]})
        if result is None:
            raise TimeoutError("Command timed out; see " + calls[-1]["log"])
        return result

    def checked(command, cwd=None):
        result = run(command, cwd)
        if result.returncode:
            raise ValueError("Command failed; see " + calls[-1]["log"])
        return result.stdout

    doctor = json.loads(checked([forge, "doctor", "--json"]))
    if doctor.get("ok") is not True:
        raise ValueError("Forge doctor failed")
    binary_hash = digest(doctor["data"]["cliPath"])
    example = output / "local-delivery.py"
    example.write_text(checked([forge, "guide", "local-delivery-example"]), encoding="utf-8")
    report = {
        "schemaVersion": 1, "kind": "ready_png_execution_pilot", "passed": False,
        "platform": platform.platform(), "forge": doctor["data"], "binarySha256": binary_hash,
        "godot": {"version": checked([godot, "--version"]).strip(), "sha256": digest(godot)},
        "sourceHashes": source_hashes, "runnerSha256": digest(__file__), "checkerSha256": digest(CHECKER),
        "exampleSha256": digest(example), "records": [], "commands": calls,
        "humanSeconds": None, "modelCost": None, "maintenanceSeconds": None,
        "productDecision": "insufficient_evidence",
        "limitations": ["public prepared artwork, not real game iterations", "no human or model-cost measurement",
                        "tool installation and script authoring excluded", "no rollback or cross-machine test",
                        "different additional safety/evidence guarantees", "OS caches retained; arm order alternates"],
    }
    write_json(output / "report.json", report)

    def native_check(project, items, manifest):
        check_inventory(project / TARGET, items)
        write_json(manifest, [{"source": str(p), "resource": "res://" + (TARGET / p.name).as_posix()} for p in items])
        result = run([godot, "--headless", "--path", project, "--script", project / "check.gd", "--", manifest])
        require_acceptance(result, len(items))

    try:
        for trial in range(1, args.trials + 1):
            order = ["native", "forge"] if trial % 2 else ["forge", "native"]
            for arm in order:
                arm_root = output / f"trial-{trial}" / arm
                project = arm_root / "game"
                project.mkdir(parents=True)
                (project / "project.godot").write_text(PROJECT, encoding="utf-8")
                shutil.copyfile(CHECKER, project / "check.gd")
                for phase in ("first", "update"):
                    work = arm_root / phase
                    work.mkdir()
                    inputs = work / "sources"
                    inputs.mkdir()
                    for i, source in enumerate(sources if phase == "first" else sources[:-1]):
                        # A different existing image replaces slot zero; the last item is removed.
                        shutil.copyfile(sources[1] if phase == "update" and i == 0 else source, inputs / source.name)
                    items = sorted(inputs.glob("*.png"))
                    request = work / "request.json"
                    write_json(request, {"schemaVersion": "1", "id": "pilot", "kind": "icon_set",
                                         "name": "Public showcase pilot", "license": "private", "sampling": "linear",
                                         "canvasPolicy": "preserve_source",
                                         "items": [{"id": p.stem, "name": p.stem, "path": str(p)} for p in items],
                                         "sourceLocks": [{"path": str(p), "sha256": digest(p)} for p in items]})
                    # Hash selection is for a technical experiment, not an assertion of art approval.
                    row = {"trial": trial, "arm": arm, "armOrder": order, "phase": phase, "passed": False}
                    started = time.perf_counter()
                    try:
                        if arm == "native":
                            target = project / TARGET
                            if target.exists():
                                shutil.rmtree(target)  # Only this runner's disposable owned directory.
                            target.mkdir(parents=True)
                            for p in items:
                                shutil.copyfile(p, target / p.name)
                            checked([godot, "--headless", "--path", project, "--import"])
                        else:
                            checked([sys.executable, example, "--forge", forge, "--expected-binary-sha256", binary_hash,
                                     "--request", request, "--project", project, "--asset-key", "pilot",
                                     "--out", work / "delivery", "--operation", "prepare-static"])
                        native_check(project, items, work / "acceptance.json")
                        row["passed"] = True
                    except (ValueError, TimeoutError, OSError) as error:
                        row["error"] = str(error)
                    row["seconds"] = time.perf_counter() - started
                    report["records"].append(row)
                    write_json(output / "report.json", report)
                    print(json.dumps(row), flush=True)

        # A native negative control must fail even when file-copy checks are bypassed.
        project = output / "trial-1/native/game"
        items = sorted((output / "trial-1/native/update/sources").glob("*.png"))
        manifest = output / "negative-control.json"
        write_json(manifest, [{"source": str(sources[-1]), "resource": "res://" + (TARGET / items[0].name).as_posix()}])
        result = run([godot, "--headless", "--path", project, "--script", project / "check.gd", "--", manifest])
        report["negativeControlPassed"] = result.returncode != 0 and "DELIVERY_CHECK_FAILED:" in result.stderr
        report["sourcesUnchanged"] = source_hashes == {p.name: digest(p) for p in sources}
        report["summary"] = summarize(report["records"])
        report["passed"] = all(r["passed"] for r in report["records"]) and report["negativeControlPassed"] and report["sourcesUnchanged"]
    finally:
        write_json(output / "report.json", report)
    print(json.dumps({"report": str(output / "report.json"), "passed": report["passed"], "summary": report.get("summary")}))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
