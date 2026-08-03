#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/forge-consistency-v2.XXXXXX")"
trap 'rm -rf "${TEST_ROOT}"' EXIT

command -v jq >/dev/null 2>&1
cargo build -q -p forge-cli --features consistency-v2 --manifest-path "${ROOT}/Cargo.toml"
FORGE="${ROOT}/target/debug/forge"
export FORGE_JOB_STORE="${TEST_ROOT}/jobs"
export FORGE_PLAN_STORE="${TEST_ROOT}/plans"
export FORGE_CACHE_STORE="${TEST_ROOT}/cache"
export FORGE_COMPONENT_STORE="${TEST_ROOT}/components"

for command in subject schema component; do
  "${FORGE}" --help | grep -E "^  ${command}[[:space:]]" >/dev/null
done

"${FORGE}" project init \
  --path "${TEST_ROOT}/project" --name "Consistency V2" --provider fixture --json \
  | jq -e '.ok' >/dev/null
"${FORGE}" style create \
  --project "${TEST_ROOT}/project" --spec "${ROOT}/examples/cli/style.json" --wait --json \
  | jq -e '.ok and .data.lifecycle_state == "succeeded"' >/dev/null
SUBJECT_JSON="$("${FORGE}" subject create \
  --project "${TEST_ROOT}/project" --spec "${ROOT}/examples/cli/subject.json" --wait --json)"
printf '%s' "${SUBJECT_JSON}" | jq -e '.ok and .data.lifecycle_state == "succeeded"' >/dev/null
SUBJECT_LOCK="$(printf '%s' "${SUBJECT_JSON}" | jq -r '.data.artifacts[] | select(.kind == "subject_lock") | .path')"
SUBJECT_REVISION="$(jq -r '.revision' "${SUBJECT_LOCK}")"
jq --arg revision "${SUBJECT_REVISION}" '.subject.revision = $revision' \
  "${ROOT}/examples/cli/character-v2.json" > "${TEST_ROOT}/character-v2.json"

"${FORGE}" schema show --id character@2.0.0 --json \
  | jq -e '.ok and .data.title == "Forge Character Asset Spec V2"' >/dev/null
"${FORGE}" component doctor fixture-vision --json \
  | jq -e '.ok and .data.ok and .data.result.protocol == "vision-component@1.0.0"' >/dev/null

PLAN_JSON="$("${FORGE}" generate character \
  --project "${TEST_ROOT}/project" --spec "${TEST_ROOT}/character-v2.json" --plan-only --json)"
printf '%s' "${PLAN_JSON}" \
  | jq -e '.ok and .data.estimate.providerRequestEstimate == 32 and .data.estimate.maximumProviderRequests == 64' >/dev/null

CHARACTER_JSON="$("${FORGE}" generate character \
  --project "${TEST_ROOT}/project" --spec "${TEST_ROOT}/character-v2.json" --wait --json)"
printf '%s' "${CHARACTER_JSON}" | jq -e '.ok and .data.lifecycle_state == "succeeded"' >/dev/null
CHARACTER_JOB="$(printf '%s' "${CHARACTER_JSON}" | jq -r '.data.job_id')"
"${FORGE}" job graph --id "${CHARACTER_JOB}" --json \
  | jq -e '.ok and ([.data.nodes[] | select(.stage == "frame_image")] | length) == 32' >/dev/null
"${FORGE}" asset list --project "${TEST_ROOT}/project" --json \
  | jq -e '.ok and (.data | length) == 1 and .data[0].assetId == "fixture-ranger-keyframes"' >/dev/null

RETRY_JSON="$("${FORGE}" job retry \
  --id "${CHARACTER_JOB}" --item walk_right --frame 3 --stage frame --wait --json)"
printf '%s' "${RETRY_JSON}" \
  | jq -e --arg parent "${CHARACTER_JOB}" '.ok and .data.lifecycle_state == "succeeded" and .data.parent_job_id == $parent' >/dev/null
RETRY_JOB="$(printf '%s' "${RETRY_JSON}" | jq -r '.data.job_id')"
"${FORGE}" job report --id "${RETRY_JOB}" --json \
  | jq -e '.ok and .data.providerRequestCount == 1 and .data.providerRequestOccurred' >/dev/null

REPLAY_JSON="$("${FORGE}" job replay \
  --id "${CHARACTER_JOB}" --from collection_consistency --wait --json)"
printf '%s' "${REPLAY_JSON}" \
  | jq -e --arg parent "${CHARACTER_JOB}" '.ok and .data.lifecycle_state == "succeeded" and .data.parent_job_id == $parent' >/dev/null
REPLAY_JOB="$(printf '%s' "${REPLAY_JSON}" | jq -r '.data.job_id')"
"${FORGE}" job report --id "${REPLAY_JOB}" --json \
  | jq -e '.ok and .data.providerRequestCount == 0 and (.data.providerRequestOccurred | not)' >/dev/null

if rg -n -i \
  'authorization:[[:space:]]*bearer|access[_-]?token|refresh[_-]?token|device[_-]?code|xai[_-]?api[_-]?key' \
  "${FORGE_JOB_STORE}" >/dev/null; then
  echo "credential-like material leaked into the V2 fixture JobStore" >&2
  exit 1
fi

echo "PASS Forge consistency V2 contract"
