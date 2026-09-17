#!/usr/bin/env python3
"""Require named native transaction tests to be discovered AND pass on each OS."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess


REQUIRED = {
    "real_godot_install_and_failed_update_preserve_approved_resources",
    "real_godot_install_with_v3_catalog",
    "real_godot_animation_verification_and_script_failure_protocol",
    "real_godot_failed_blue_update_restores_red_native_texture_without_reimport",
    "retained_alias_revisions_install_and_roll_back_without_production_jobs",
}


def verify_listing(output):
    names = set(re.findall(r"^([\w:]+): test$", output, re.MULTILINE))
    if not REQUIRED <= names:
        raise RuntimeError(f"Native transaction tests missing: {sorted(REQUIRED - names)}")
    return names


def verify_execution(output):
    names = set(re.findall(r"^test ([\w:]+) \.\.\. ok$", output, re.MULTILINE))
    summary = re.search(r"test result: ok\. (\d+) passed; 0 failed; 0 ignored;", output)
    if not REQUIRED <= names or summary is None or int(summary[1]) != len(names):
        raise RuntimeError("Native transaction gate requires every named test to pass; "
                           "zero, skipped, missing or failed tests are not acceptance")
    return names


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--godot", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = args.output.absolute()
    root.mkdir(parents=True, exist_ok=False)
    repo = Path(__file__).resolve().parent.parent
    env = dict(os.environ, FORGE_GODOT_PATH=str(args.godot.absolute()), CARGO_TERM_COLOR="never")
    report = {"ok": False, "requiredTests": sorted(REQUIRED), "godot": str(args.godot.absolute())}
    summary = root / "summary.json"
    summary.write_text(json.dumps(report, indent=2), encoding="utf-8")

    def run(label, command, timeout=1200):
        try:
            result = subprocess.run(command, cwd=repo, env=env, stdout=subprocess.PIPE,
                                    stderr=subprocess.STDOUT, text=True, encoding="utf-8",
                                    timeout=timeout)
        except subprocess.TimeoutExpired as error:
            output = error.stdout or b""
            (root / f"{label}.log").write_bytes(output if isinstance(output, bytes) else output.encode())
            raise
        (root / f"{label}.log").write_text(result.stdout, encoding="utf-8")
        print(result.stdout, end="", flush=True)
        result.check_returncode()
        return result.stdout

    report["gitCommit"] = run("commit", ["git", "rev-parse", "HEAD"], 30).strip()
    report["dirty"] = bool(run("status", ["git", "status", "--porcelain"], 30).strip())
    report["rustc"] = run("rustc", ["rustc", "--version"], 30).strip()
    report["godotVersion"] = run("godot-version", [str(args.godot.absolute()), "--version"], 30).strip()
    cargo = ["cargo", "test", "--locked", "-p", "core", "--test", "godot_install_transaction_tests", "--"]
    listed = verify_listing(run("discovery", [*cargo, "--ignored", "--list", "--format=terse"]))
    passed = verify_execution(run("execution", [*cargo, "--ignored", "--test-threads=1",
                                                "--format=pretty", "--color=never"]))
    if listed != passed:
        raise RuntimeError(f"Discovered and executed test sets differ: {listed ^ passed}")
    report.update(ok=True, discoveredTests=sorted(listed), passedTests=sorted(passed))
    summary.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"ok": True, "nativeTransactionTests": len(passed), "summary": str(summary)}))


if __name__ == "__main__":
    main()
