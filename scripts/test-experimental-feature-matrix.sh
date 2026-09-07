#!/usr/bin/env bash
# Portable, offline coverage for opt-in features. The release matrix already
# runs default workspace tests; do not repeat those or flip feature builds back
# to default here. Each suite has its own CI job and machine-readable evidence.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SUITE="${FORGE_EXPERIMENTAL_SUITE:-}"
case "${SUITE}" in
  project-assets|subject-grid|processing) ;;
  *)
    echo 'Set FORGE_EXPERIMENTAL_SUITE to project-assets, subject-grid, or processing.' >&2
    exit 2
    ;;
esac

unset FORGE_REAL_PROVIDER_ACCEPT FORGE_REAL_PROVIDER_MAX_REQUESTS \
  FORGE_REAL_PROVIDER_MAX_COST_TICKS XAI_API_KEY
cd "${ROOT}" || exit 1
command -v jq >/dev/null 2>&1 || exit 1

REPORT_DIR="${FORGE_EXPERIMENTAL_REPORT_DIR:-${ROOT}/target/qa/experimental-${SUITE}}"
mkdir -p "${REPORT_DIR}/logs" || exit 1
REPORT_DIR="$(cd "${REPORT_DIR}" && pwd)"
GATES_TSV="${REPORT_DIR}/gates.tsv"
REPORT_JSON="${REPORT_DIR}/experimental-feature-report.json"
: > "${GATES_TSV}"
STARTED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

run_gate() {
  local id="$1"
  shift
  local started finished status exit_code log_path
  started="$(date +%s)"
  log_path="${REPORT_DIR}/logs/${id}.log"
  echo "RUN  ${id}"
  # Run each gate with errexit outside an if/&& condition: Bash otherwise
  # disables errexit inside shell functions and can conceal an early failure.
  (set -e; "$@") > "${log_path}" 2>&1
  exit_code=$?
  finished="$(date +%s)"
  if [ "${exit_code}" -eq 0 ]; then
    status=pass
    echo "PASS ${id}"
  else
    status=fail
    echo "FAIL ${id}; see ${log_path}" >&2
  fi
  printf '%s\t%s\t%s\t%s\t%s\n' \
    "${id}" "${status}" "$((finished - started))" "${exit_code}" "${log_path}" \
    >> "${GATES_TSV}"
}

media_tools_gate() {
  command -v ffmpeg
  command -v ffprobe
  ffmpeg -version
  ffprobe -version
  local godot="${FORGE_GODOT_PATH:-/Applications/Godot.app/Contents/MacOS/Godot}"
  test -x "${godot}"
  "${godot}" --version
  if [ "${SUITE}" = subject-grid ]; then
    test -x /Applications/Godot.app/Contents/MacOS/Godot
  fi
}

subject_import_gate() {
  cargo test --locked -p core --features subject-import --test subject_import_tests
  cargo test --locked -p forge-cli --features subject-import \
    tests::subject_import_requires_neither_provider_resolution_nor_authorization_targets -- --exact
  cargo build --locked -q -p forge-cli --features subject-import
  target/debug/forge subject import --help
  target/debug/forge schema show --id subject-import-report@1.0.0 --json \
    | jq -e '.ok and .data.properties.profile.const == "subject-import@1.0.0"'
}

grid_generation_gate() {
  cargo test --locked -p core --features grid-generation --test grid_feature_gate_tests
  cargo test --locked -p forge-cli --features grid-generation
  cargo test --locked -p providers --features grid-generation --test grid_generation_contract
  cargo build --locked -q -p forge-cli --features grid-generation
  local forge="${ROOT}/target/debug/forge"
  "${forge}" subject create --help
  "${forge}" subject import --help
  "${forge}" job import-direction-grid --help > "${REPORT_DIR}/grid-import-help.txt"
  grep -q -- '--generator' "${REPORT_DIR}/grid-import-help.txt"
  grep -q -- '--cape-hem' "${REPORT_DIR}/grid-import-help.txt"
  "${forge}" provider authorize --help > "${REPORT_DIR}/provider-authorize-help.txt"
  local option
  for option in max-provider-operations recipe-hash input-fingerprint; do
    grep -q -- "--${option}" "${REPORT_DIR}/provider-authorize-help.txt"
  done
  local schema
  for schema in \
    subject-import-report@1.0.0=subject-import@1.0.0 \
    character-direction-grid-lock@1.1.0=direction-grid-lock@1.1.0 \
    character-direction-grid-appearance-report@1.0.0=direction-grid-appearance@1.0.0 \
    grid-action-report@1.1.0=grid-action-report@1.1.0 \
    grid-keyframe-action-report@1.0.0=grid-keyframe-action-report@1.0.0 \
    grid-keyframe-action-report@1.3.0=grid-keyframe-action-report@1.3.0 \
    grid-keyframe-action-report@1.4.0=grid-keyframe-action-report@1.4.0 \
    gait-laterality@1.0.0=gait-laterality@1.0.0 \
    checkerboard-sheet-matting-report@1.0.0=checkerboard-sheet-matting@1.0.0 \
    checkerboard-sheet-matting-report@1.1.0=checkerboard-sheet-matting@1.1.0 \
    direction-grid-import-evidence@1.0.0=direction-grid-import@1.0.0 \
    direction-grid-import-evidence@1.1.0=direction-grid-import@1.1.0 \
    direction-grid-import-consistency@1.0.0=direction-grid-import-consistency@1.0.0 \
    direction-grid-authority-alignment@1.0.0=direction-grid-authority-alignment@1.0.0 \
    alpha-edge-halo@1.0.0=alpha-edge-halo@1.0.0 \
    cape-hem-consistency@1.0.0=cape-hem-consistency@1.0.0 \
    character-anchor-stabilization@1.0.0=character-anchor-stabilization@1.0.0; do
    "${forge}" schema show --id "${schema%=*}" --json \
      | jq -e --arg profile "${schema#*=}" \
        '.ok and ([.data | .. | strings] | index($profile) != null)'
  done
}

case "${SUITE}" in
  project-assets)
    export FORGE_REQUIRE_GODOT=1
    run_gate media-tools media_tools_gate
    run_gate game-art-manifest bash "${ROOT}/scripts/test-game-art-manifest.sh"
    run_gate collection-assets bash "${ROOT}/scripts/test-stage3-static.sh"
    ;;
  subject-grid)
    export FORGE_REQUIRE_GODOT=1
    run_gate media-tools media_tools_gate
    run_gate subject-import subject_import_gate
    run_gate grid-generation grid_generation_gate
    ;;
  processing)
    # One feature combination reuses the same core unit-test binary for both
    # filters. Default V6/V7/V8 Pack compatibility remains in the release job.
    run_gate pixel-delivery-v2 cargo test --locked -p core --lib \
      --features pixel-delivery-v2,identity-metric-v2 pixel_grid::tests
    run_gate pixel-delivery-normalization cargo test --locked -p core --lib \
      --features pixel-delivery-v2,identity-metric-v2 pixel_delivery
    run_gate identity-metric-v2 cargo test --locked -p core --lib \
      --features pixel-delivery-v2,identity-metric-v2 quality::identity::tests
    run_gate identity-calibration-example-build cargo check --locked -p core \
      --features pixel-delivery-v2,identity-metric-v2 --example evaluate_identity_metric
    ;;
esac

jq -Rn '[inputs | select(length > 0) | split("\t") | {
  id: .[0], scope: "offline_experimental", status: .[1],
  durationSeconds: (.[2] | tonumber), exitCode: (.[3] | tonumber), logPath: .[4]
}]' < "${GATES_TSV}" > "${REPORT_DIR}/gates.json" || exit 1

jq -n \
  --arg suite "${SUITE}" \
  --arg startedAt "${STARTED_AT}" \
  --arg finishedAt "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg commit "$(git rev-parse HEAD)" \
  --slurpfile gates "${REPORT_DIR}/gates.json" '
  {
    schemaVersion: "1", profile: "forge-experimental-feature-matrix@1.0.0",
    suite: $suite, startedAt: $startedAt, finishedAt: $finishedAt,
    git: {commit: $commit}, acceptanceScope: "offline_contracts_only",
    gates: $gates[0],
    unverifiedEvidence: ([{
      id: "real-provider-visual-acceptance", status: "not_run",
      reason: "Fixture contracts do not establish real-provider or human visual acceptance."
    }] + if $suite == "processing" then [{
      id: "identity-metric-human-calibration", status: "not_run",
      script: "scripts/test-identity-metric.sh",
      reason: "The retained human-labeled calibration corpus under docs/qa/artifacts/forge-identity-metric-calibration-2026-08-11 is not versioned. CI runs identity unit tests and compiles its evaluator, but does not claim calibration accuracy."
    }] else [] end),
    summary: {
      passed: ([$gates[0][] | select(.status == "pass")] | length),
      failed: ([$gates[0][] | select(.status == "fail")] | length)
    },
    verdict: (if ($gates[0] | length > 0) and ($gates[0] | all(.status == "pass"))
      then "pass" else "fail" end)
  }' > "${REPORT_JSON}" || exit 1

jq . "${REPORT_JSON}"
echo "REPORT ${REPORT_JSON}"
jq -e '.verdict == "pass"' "${REPORT_JSON}" >/dev/null
