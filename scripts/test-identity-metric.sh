#!/bin/sh
set -eu

unset FORGE_REAL_PROVIDER_ACCEPT || true
unset FORGE_REAL_PROVIDER_MAX_REQUESTS || true
unset FORGE_REAL_PROVIDER_MAX_COST_TICKS || true

CALIBRATION_DIR="docs/qa/artifacts/forge-identity-metric-calibration-2026-08-11"
MANIFEST="$CALIBRATION_DIR/calibration.json"
REPORT="$CALIBRATION_DIR/identity-metric-report.json"

test -f "$MANIFEST"
cargo test -p core quality::identity
cargo check -p core
cargo check -p core --features identity-metric-v2
cargo run -q -p core --example evaluate_identity_metric --features identity-metric-v2 -- \
  --manifest "$MANIFEST" \
  --output "$REPORT"

python3 - "$REPORT" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as handle:
    report = json.load(handle)

calibration = report["calibration"]
assert calibration["passCount"] == 4, calibration
assert calibration["failCount"] == 5, calibration
assert calibration["grayCount"] == 1, calibration
baseline = report["baselinePhash"]
composite = report["composite"]
assert composite["separation"] > baseline["separation"], (baseline, composite)
assert composite["rocAuc"] > baseline["rocAuc"], (baseline, composite)
assert composite["rocAuc"] >= 0.99, composite
assert any(
    item["metric"] == "regional_palette_emd" and item["rocAuc"] >= 0.99
    for item in report["ablation"]
), report["ablation"]
PY
