#!/usr/bin/env python3
"""Run the standalone SpriteFrames clock example without writing consumer projects."""
import argparse
import re
import shutil
import subprocess
import tempfile
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--godot", required=True)
    args = parser.parse_args()
    source = Path(__file__).resolve().parents[1] / "examples/godot/forge-external-clock"
    with tempfile.TemporaryDirectory(prefix="forge-clock-") as directory:
        project = Path(directory) / "project"
        shutil.copytree(source, project, ignore=shutil.ignore_patterns(".godot", "*.uid"))
        result = subprocess.run([args.godot, "--headless", "--path", str(project), "--script",
                                 "res://verify_external_clock.gd"], capture_output=True, text=True, timeout=30)
        output = result.stdout + result.stderr
        print(output, end="")
        if result.returncode or re.search(r"(?:SCRIPT ERROR|(?:^|\n)ERROR:|(?:^|\n)FAIL )", output):
            raise SystemExit("Godot external-clock verification failed")
        if "PASS forge external clock:" not in output:
            raise SystemExit("Godot verification returned without a completion marker")


if __name__ == "__main__":
    main()
