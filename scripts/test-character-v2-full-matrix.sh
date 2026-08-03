#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/forge-character-v2-full.XXXXXX")"
trap 'rm -rf "${TEST_ROOT}"' EXIT

command -v jq >/dev/null 2>&1
GODOT="${FORGE_GODOT_PATH:-/Applications/Godot.app/Contents/MacOS/Godot}"
if [ ! -x "${GODOT}" ]; then
  GODOT="$(command -v godot || command -v godot4 || true)"
fi
test -x "${GODOT}"
export FORGE_GODOT_PATH="${GODOT}"

cargo build -q -p forge-cli --features consistency-v2 --manifest-path "${ROOT}/Cargo.toml"
FORGE="${ROOT}/target/debug/forge"
MANIFEST="${ROOT}/benchmarks/character-v2/frozen-20x5.json"

FORGE_CACHE_STORE="${TEST_ROOT}/full/cache" \
  "${FORGE}" benchmark run-character \
    --manifest "${MANIFEST}" \
    --output "${TEST_ROOT}/full" \
    --provider fixture \
    --workflow both \
    --skip-godot \
    --json \
    > "${TEST_ROOT}/full-command.json"

jq -e '
  .ok and
  .data.results.distinctCaseCount == 20 and
  .data.results.distinctStyleCount == 5 and
  .data.results.workflows.video.caseCount == 20 and
  .data.results.workflows.video.successfulPackCount == 20 and
  .data.results.workflows.video.medianProviderRequests == 9 and
  .data.results.workflows.keyframes.caseCount == 20 and
  .data.results.workflows.keyframes.successfulPackCount == 20 and
  .data.results.workflows.keyframes.medianProviderRequests == 32 and
  .data.results.gates.frozenScope == "pass" and
  .data.results.gates.keyframePackSuccess == "pass" and
  .data.results.gates.providerRequestBudget == "pass" and
  .data.results.gates.identityImprovement == "fail" and
  .data.results.gates.godotValidation == "not_evaluated" and
  (.data.results.upgradeRecommended | not)
' "${TEST_ROOT}/full-command.json" >/dev/null

jq -e '
  (.cases | length) == 40 and
  ([.cases[] | select(.gameReady and .packExported and .packValid)] | length) == 40 and
  ([.cases[] | select(.hardDefectDetected)] | length) == 0 and
  ([.cases[].providerRequests] | add) == 820 and
  ([.cases[] | select(.workflow == "video") | .providerRequests] | unique) == [9] and
  ([.cases[] | select(.workflow == "keyframes") | .providerRequests] | unique) == [32]
' "${TEST_ROOT}/full/benchmark-run.json" >/dev/null

FORGE_CACHE_STORE="${TEST_ROOT}/godot-calibration/cache" \
  "${FORGE}" benchmark run-character \
    --manifest "${MANIFEST}" \
    --output "${TEST_ROOT}/godot-calibration" \
    --provider fixture \
    --workflow keyframes \
    --limit 1 \
    --json \
    > "${TEST_ROOT}/godot-command.json"

jq -e '
  .ok and
  .data.results.workflows.keyframes.caseCount == 1 and
  .data.results.workflows.keyframes.successfulPackCount == 1 and
  .data.results.workflows.keyframes.godotValidationCount == 1 and
  .data.results.workflows.keyframes.godotLoadedCount == 1 and
  .data.results.gates.godotValidation == "pass"
' "${TEST_ROOT}/godot-command.json" >/dev/null

jq -e '
  (.cases | length) == 1 and
  .cases[0].gameReady and
  .cases[0].packValid and
  .cases[0].godotValidationAttempted and
  .cases[0].godotLoaded and
  .cases[0].identityPassCount == 32 and
  .cases[0].identitySampleCount == 32 and
  .cases[0].providerRequests == 32
' "${TEST_ROOT}/godot-calibration/benchmark-run.json" >/dev/null

if find "${TEST_ROOT}/godot-calibration/godot/addons/forge_assets" \
  -type f \( -name '*.tres' -o -name '*.tscn' \) -size +1048575c \
  | grep -q .; then
  echo "Character V2 Godot resource exceeds 1 MiB" >&2
  exit 1
fi
if grep -R -E \
  'sub_resource type="Image"|sub_resource type="ImageTexture"|ImageTexture.create_from_image|^data = PackedByteArray' \
  "${TEST_ROOT}/godot-calibration/godot/addons/forge_assets" \
  --include='*.tres' --include='*.tscn'; then
  echo "Character V2 Godot resource embeds image pixels" >&2
  exit 1
fi

REPORT_PATH="${FORGE_CHARACTER_MATRIX_REPORT:-${TEST_ROOT}/character-v2-matrix-report.json}"
mkdir -p "$(dirname "${REPORT_PATH}")"
jq -n \
  --slurpfile summary "${TEST_ROOT}/full/benchmark-summary.json" \
  --slurpfile calibration "${TEST_ROOT}/godot-calibration/benchmark-summary.json" \
  '{
    schemaVersion: "1",
    profile: "character-v2-full-matrix@1.0.0",
    providerId: "fixture",
    fullBenchmark: $summary[0],
    godotCalibration: $calibration[0],
    expectedFixtureIdentityGate: "fail",
    packValidation: "pass",
    godotValidation: "pass",
    resourceEmbedding: "pass"
  }' > "${REPORT_PATH}"

echo "PASS Forge Character V2 full 20x5 matrix"
