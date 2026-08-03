#!/usr/bin/env bash
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)"
REPORT_DIR="${FORGE_V03_REPORT_DIR:-${ROOT}/target/qa/v0.3-release-matrix-${RUN_ID}}"
LOG_DIR="${REPORT_DIR}/logs"
GATES_TSV="${REPORT_DIR}/gates.tsv"
GATES_JSON="${REPORT_DIR}/gates.json"
REPORT_JSON="${REPORT_DIR}/v0.3-test-report.json"

mkdir -p "${LOG_DIR}"
: > "${GATES_TSV}"

run_gate() {
  local id="$1"
  local scope="$2"
  shift 2
  local started finished duration status exit_code log_path
  log_path="${LOG_DIR}/${id}.log"
  started="$(date +%s)"
  echo "RUN  ${id} (${scope})"
  if "$@" >"${log_path}" 2>&1; then
    status="pass"
    exit_code=0
    echo "PASS ${id}"
  else
    exit_code=$?
    status="fail"
    echo "FAIL ${id}; see ${log_path}" >&2
  fi
  finished="$(date +%s)"
  duration=$((finished - started))
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
    "${id}" "${scope}" "${status}" "${duration}" "${exit_code}" "${log_path}" \
    >> "${GATES_TSV}"
}

rust_quality_gate() {
  cargo fmt --manifest-path "${ROOT}/Cargo.toml" --all -- --check &&
  cargo clippy --manifest-path "${ROOT}/Cargo.toml" --workspace --all-targets -- -D warnings &&
  cargo clippy --manifest-path "${ROOT}/Cargo.toml" -p forge-cli \
    --all-targets --features consistency-v2,world-assets -- -D warnings &&
  cargo test --manifest-path "${ROOT}/Cargo.toml" --workspace --no-fail-fast
}

character_contract_gate() {
  bash "${ROOT}/scripts/test-consistency-v2.sh" &&
  bash "${ROOT}/scripts/test-character-v2-benchmark.sh"
}

character_matrix_gate() {
  FORGE_CHARACTER_MATRIX_REPORT="${REPORT_DIR}/character-v2-matrix.json" \
    bash "${ROOT}/scripts/test-character-v2-full-matrix.sh"
}

static_contract_gate() {
  bash "${ROOT}/scripts/test-cli-product.sh"
}

static_matrix_gate() {
  FORGE_STATIC_MATRIX_REPORT="${REPORT_DIR}/static-asset-matrix.json" \
    bash "${ROOT}/scripts/test-static-asset-matrix.sh"
}

world_contract_gate() {
  cargo test --manifest-path "${ROOT}/Cargo.toml" \
    -p providers --test world_generation_contract -- --nocapture &&
  bash "${ROOT}/scripts/test-world-assets.sh"
}

STARTED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
run_gate "rust-quality" "release_blocking" rust_quality_gate
run_gate "character-v2-contract" "release_blocking" character_contract_gate
run_gate "character-v2-full-matrix" "release_blocking" character_matrix_gate
run_gate "static-cli-contract" "release_blocking" static_contract_gate
run_gate "static-five-style-matrix" "release_blocking" static_matrix_gate
run_gate "world-assets-experimental" "experimental" world_contract_gate
FINISHED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

jq -Rn '
  [
    inputs
    | select(length > 0)
    | split("\t")
    | {
        id: .[0],
        scope: .[1],
        status: .[2],
        durationSeconds: (.[3] | tonumber),
        exitCode: (.[4] | tonumber),
        logPath: .[5]
      }
  ]
' < "${GATES_TSV}" > "${GATES_JSON}"

COMMIT="$(git -C "${ROOT}" rev-parse HEAD)"
BRANCH="$(git -C "${ROOT}" branch --show-current)"
jq -n \
  --arg startedAt "${STARTED_AT}" \
  --arg finishedAt "${FINISHED_AT}" \
  --arg commit "${COMMIT}" \
  --arg branch "${BRANCH}" \
  --arg characterReport "${REPORT_DIR}/character-v2-matrix.json" \
  --arg staticReport "${REPORT_DIR}/static-asset-matrix.json" \
  --slurpfile gates "${GATES_JSON}" '
  {
    schemaVersion: "1",
    profile: "forge-v0.3-release-matrix@1.0.0",
    startedAt: $startedAt,
    finishedAt: $finishedAt,
    git: {commit: $commit, branch: $branch},
    gates: $gates[0],
    evidence: {
      characterV2: $characterReport,
      staticAssets: $staticReport
    },
    summary: {
      passed: ([$gates[0][] | select(.status == "pass")] | length),
      failed: ([$gates[0][] | select(.status == "fail")] | length),
      releaseBlockingPassed: (
        [$gates[0][] | select(.scope == "release_blocking")]
        | all(.status == "pass")
      ),
      experimentalPassed: (
        [$gates[0][] | select(.scope == "experimental")]
        | all(.status == "pass")
      )
    },
    verdict: (
      if ([$gates[0][] | select(.status == "fail")] | length) == 0
      then "pass"
      else "fail"
      end
    )
  }
' > "${REPORT_JSON}"

jq . "${REPORT_JSON}"
echo "REPORT ${REPORT_JSON}"

if jq -e '.verdict == "pass"' "${REPORT_JSON}" >/dev/null; then
  exit 0
fi
exit 1
