#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/forge-real-provider-guard.XXXXXX")"
trap 'rm -rf "${TEST_ROOT}"' EXIT

cargo build -q -p forge-cli --features consistency-v2 --manifest-path "${ROOT}/Cargo.toml"
FORGE="${ROOT}/target/debug/forge"
MANIFEST="${ROOT}/benchmarks/character-v2/frozen-20x5.json"

if env -u FORGE_REAL_PROVIDER_ACCEPT \
  -u FORGE_REAL_PROVIDER_MAX_REQUESTS \
  -u FORGE_REAL_PROVIDER_MAX_COST_TICKS \
  "${FORGE}" benchmark run-character \
    --manifest "${MANIFEST}" \
    --output "${TEST_ROOT}/missing-acceptance" \
    --provider xai \
    --workflow keyframes \
    --limit 1 \
    --skip-godot \
    --accept-provider-cost \
    --json >"${TEST_ROOT}/missing-acceptance.json"; then
  echo "real Provider execution unexpectedly ran without the environment guard" >&2
  exit 1
fi
jq -e '.ok == false and .error.code == "real_provider_not_accepted"' \
  "${TEST_ROOT}/missing-acceptance.json" >/dev/null

if FORGE_REAL_PROVIDER_ACCEPT=1 \
  FORGE_REAL_PROVIDER_MAX_REQUESTS=0 \
  FORGE_REAL_PROVIDER_MAX_COST_TICKS=100 \
  "${FORGE}" benchmark run-character \
    --manifest "${MANIFEST}" \
    --output "${TEST_ROOT}/invalid-limit" \
    --provider xai \
    --workflow keyframes \
    --limit 1 \
    --skip-godot \
    --accept-provider-cost \
    --json >"${TEST_ROOT}/invalid-limit.json"; then
  echo "real Provider execution unexpectedly accepted a zero request limit" >&2
  exit 1
fi
jq -e '.ok == false and .error.code == "real_provider_not_accepted"' \
  "${TEST_ROOT}/invalid-limit.json" >/dev/null

echo "PASS real Provider budget guard contract"
