#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/forge-character-benchmark.XXXXXX")"
trap 'rm -rf "${TEST_ROOT}"' EXIT

command -v jq >/dev/null 2>&1
cargo build -q -p forge-cli --features consistency-v2 --manifest-path "${ROOT}/Cargo.toml"
FORGE="${ROOT}/target/debug/forge"
MANIFEST="${ROOT}/benchmarks/character-v2/frozen-20x5.json"

"${FORGE}" --help | grep -E '^  benchmark[[:space:]]' >/dev/null
"${FORGE}" schema show --id character-benchmark@1.0.0 --json \
  | jq -e '.ok and .data.title == "Forge Character Benchmark V1"' >/dev/null

"${FORGE}" benchmark validate \
  --manifest "${MANIFEST}" --provider fixture --json \
  | jq -e '
      .ok and .data.valid and
      .data.plan.styleCount == 5 and
      .data.plan.caseCount == 20 and
      .data.plan.fullFrozenScope and
      .data.plan.hardDefectLabels == 0 and
      .data.plan.bothWorkflowsRequests.normal == 845 and
      .data.plan.bothWorkflowsRequests.maximum == 1565
    ' >/dev/null

"${FORGE}" benchmark plan \
  --manifest "${MANIFEST}" --provider xai --profile default --json \
  | jq -e '
      .ok and .data.providerId == "xai" and
      .data.sharedSetupRequests.normal == 25 and
      .data.videoRequests.normal == 180 and
      .data.keyframeRequests.normal == 640
    ' >/dev/null

if "${FORGE}" benchmark run-character \
  --manifest "${MANIFEST}" \
  --output "${TEST_ROOT}/unapproved-xai" \
  --provider xai \
  --workflow keyframes \
  --limit 1 \
  --skip-godot \
  --json \
  | jq -e '.ok' >/dev/null; then
  echo "real Provider benchmark ran without explicit cost acceptance" >&2
  exit 1
fi

jq -n '
  def result($index; $workflow):
    {
      caseId: ("case-" + ($index | tostring)),
      styleId: ("style-" + (($index % 5) | tostring)),
      workflow: $workflow,
      jobId: ("job-" + ($index | tostring) + "-" + $workflow),
      lifecycleState: (if $index == 0 then "failed" else "succeeded" end),
      gameReady: ($index != 0),
      packExported: ($index != 0),
      packValid: ($index != 0),
      godotValidationAttempted: ($index != 0),
      godotLoaded: ($index != 0),
      hardDefectExpected: ($index == 0),
      hardDefectDetected: ($index == 0),
      identityPassCount: (if $index == 0 then 0 elif $workflow == "keyframes" then 9 else 7 end),
      identitySampleCount: 10,
      providerRequests: (if $workflow == "keyframes" then 32 else 9 end),
      errorCode: (if $index == 0 then "fixture_hard_defect" else null end)
    };
  {
    schemaVersion: "1",
    profile: "character-v2-benchmark@1.0.0",
    benchmarkId: "fixture-gate-calibration",
    providerId: "fixture",
    profileId: "default",
    startedAt: "2026-08-03T00:00:00Z",
    finishedAt: "2026-08-03T00:01:00Z",
    cases: ([range(0; 20) as $index | result($index; "video")] +
            [range(0; 20) as $index | result($index; "keyframes")])
  }
' > "${TEST_ROOT}/passing-run.json"

"${FORGE}" benchmark summarize --input "${TEST_ROOT}/passing-run.json" --json \
  | jq -e '
      .ok and .data.upgradeRecommended and
      .data.distinctCaseCount == 20 and
      .data.distinctStyleCount == 5 and
      .data.workflows.keyframes.automatedPackSuccessRate == 0.95 and
      .data.gates.hardDefectInterception == "pass" and
      .data.gates.zeroErroneousPackExports == "pass" and
      .data.gates.identityImprovement == "pass" and
      .data.gates.providerRequestBudget == "pass"
      and .data.gates.godotValidation == "pass"
    ' >/dev/null

jq '.styles[0].spec.referenceImages = ["../escape.png"]' \
  "${MANIFEST}" > "${TEST_ROOT}/unsafe-reference.json"
if "${FORGE}" benchmark validate \
  --manifest "${TEST_ROOT}/unsafe-reference.json" --json \
  | jq -e '.ok' >/dev/null; then
  echo "benchmark accepted a parent-directory reference" >&2
  exit 1
fi

"${FORGE}" benchmark run-character \
  --manifest "${MANIFEST}" \
  --output "${TEST_ROOT}/fixture-run" \
  --provider fixture \
  --workflow keyframes \
  --limit 1 \
  --skip-godot \
  --json \
  | jq -e '
      .ok and
      (.data.run | endswith("benchmark-run.json")) and
      (.data.summary | endswith("benchmark-summary.json")) and
      .data.results.distinctCaseCount == 1 and
      .data.results.gates.frozenScope == "fail"
    ' >/dev/null
jq -e '
  (.cases | length) == 1 and
  .cases[0].workflow == "keyframes" and
  .cases[0].packExported and
  .cases[0].packValid and
  .cases[0].providerRequests <= 64
' "${TEST_ROOT}/fixture-run/benchmark-run.json" >/dev/null

"${FORGE}" benchmark run-character \
  --manifest "${ROOT}/benchmarks/character-v2/fixture-hard-defect.json" \
  --output "${TEST_ROOT}/hard-defect-run" \
  --provider fixture \
  --workflow keyframes \
  --skip-godot \
  --json \
  | jq -e '
      .ok and
      .data.results.expectedHardDefectCount == 1 and
      .data.results.interceptedHardDefectCount == 1 and
      .data.results.erroneousPackExportCount == 0 and
      .data.results.gates.hardDefectInterception == "pass" and
      .data.results.gates.zeroErroneousPackExports == "pass"
    ' >/dev/null
jq -e '
  (.cases | length) == 1 and
  .cases[0].hardDefectExpected and
  .cases[0].hardDefectDetected and
  (.cases[0].packExported | not) and
  (.cases[0].packValid | not)
' "${TEST_ROOT}/hard-defect-run/benchmark-run.json" >/dev/null

cargo build -q -p forge-cli --manifest-path "${ROOT}/Cargo.toml"
if "${ROOT}/target/debug/forge" --help | grep -E '^  benchmark[[:space:]]' >/dev/null; then
  echo "v0.2 default build unexpectedly exposes the v0.3 benchmark interface" >&2
  exit 1
fi

echo "PASS Forge Character V2 benchmark contract"
