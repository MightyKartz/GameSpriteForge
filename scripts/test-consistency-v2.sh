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
REPORT_DIR="${FORGE_CONSISTENCY_V2_REPORT_DIR:-}"

credential_scan_matches() {
  if command -v rg >/dev/null 2>&1; then
    rg -n -i \
      'authorization:[[:space:]]*bearer|access[_-]?token|refresh[_-]?token|device[_-]?code|xai[_-]?api[_-]?key' \
      "$@"
  else
    grep -R -I -n -E \
      'authorization:[[:space:]]*bearer|access[_-]?token|refresh[_-]?token|device[_-]?code|xai[_-]?api[_-]?key' \
      "$@"
  fi
}

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
jq --arg revision "${SUBJECT_REVISION}" '.subject.revision = $revision' \
  "${ROOT}/examples/cli/character-v2-keyframes.json" > "${TEST_ROOT}/character-v2-keyframes.json"
jq --arg revision "${SUBJECT_REVISION}" '.subject.revision = $revision' \
  "${ROOT}/examples/cli/character-v2-keyposes.json" > "${TEST_ROOT}/character-v2-keyposes.json"

"${FORGE}" schema show --id character@2.0.0 --json \
  | jq -e '.ok and .data.title == "Forge Character Asset Spec V2"' >/dev/null
"${FORGE}" schema show --id character-direction-lock@1.0.0 --json \
  | jq -e '.ok and .data.title == "Forge DirectionLockV1"' >/dev/null
"${FORGE}" schema show --id keyframe-background-cleanup-report@1.0.0 --json \
  | jq -e '.ok and .data.title == "Forge KeyframeBackgroundCleanupReportV1"' >/dev/null
"${FORGE}" component doctor fixture-vision --json \
  | jq -e '.ok and .data.ok and .data.result.protocol == "vision-component@1.0.0"' >/dev/null

PLAN_JSON="$("${FORGE}" generate character \
  --project "${TEST_ROOT}/project" --spec "${TEST_ROOT}/character-v2.json" --plan-only --json)"
printf '%s' "${PLAN_JSON}" \
  | jq -e '.ok
    and .data.estimate.providerRequestEstimate == 8
    and .data.estimate.maximumProviderRequests == 16
    and .data.estimate.workflow == "topdown-video@2.0.0"
    and .data.estimate.model == "fixture-image"' >/dev/null
PLAN_TOKEN="$(printf '%s' "${PLAN_JSON}" | jq -r '.data.token')"
jq -e '.operation.request.generation.imageModel == "fixture-image"
  and .operation.request.generation.videoModel == "fixture-video"
  and .operation.request.cameraProfile == "topdown-orthographic@2.0.0"' \
  "${FORGE_PLAN_STORE}/${PLAN_TOKEN}.pending.json" >/dev/null

VIDEO_VALIDATION_JSON="$("${FORGE}" generate character \
  --project "${TEST_ROOT}/project" --spec "${TEST_ROOT}/character-v2.json" \
  --validation-animation walk_right --wait --json)"
printf '%s' "${VIDEO_VALIDATION_JSON}" \
  | jq -e '.ok and .data.lifecycle_state == "succeeded" and ([.data.artifacts[] | select(.kind == "gsfpack")] | length) == 0' >/dev/null
VIDEO_VALIDATION_JOB="$(printf '%s' "${VIDEO_VALIDATION_JSON}" | jq -r '.data.job_id')"
"${FORGE}" job report --id "${VIDEO_VALIDATION_JOB}" --json \
  | jq -e '.ok
    and .data.reports.provider_manifest.workflow == "topdown-video@2.0.0"
    and .data.reports.provider_manifest.cameraProfile == "topdown-orthographic@2.0.0"
    and .data.providerRequestCount == 2' >/dev/null
"${FORGE}" job get --id "${VIDEO_VALIDATION_JOB}" --json \
  | jq -e '.ok
    and ([.data.artifacts[] | select(.kind | startswith("direction_still_preflight_walk_right"))] | length) == 1
    and ([.data.artifacts[] | select(.kind | startswith("provider_video_walk_right_attempt_"))] | length) == 1' >/dev/null

KEYFRAME_PLAN_JSON="$("${FORGE}" generate character \
  --project "${TEST_ROOT}/project" --spec "${TEST_ROOT}/character-v2-keyframes.json" --plan-only --json)"
printf '%s' "${KEYFRAME_PLAN_JSON}" \
  | jq -e '.ok
    and .data.estimate.providerRequestEstimate == 32
    and .data.estimate.maximumProviderRequests == 64
    and .data.estimate.workflow == "topdown-keyframes@2.3.0"
    and .data.estimate.model == "fixture-image"' >/dev/null

KEYPOSE_PLAN_JSON="$("${FORGE}" generate character \
  --project "${TEST_ROOT}/project" --spec "${TEST_ROOT}/character-v2-keyposes.json" --plan-only --json)"
printf '%s' "${KEYPOSE_PLAN_JSON}" \
  | jq -e '.ok
    and .data.estimate.providerRequestEstimate == 16
    and .data.estimate.maximumProviderRequests == 32
    and .data.estimate.workflow == "topdown-keyposes@2.5.0"
    and .data.estimate.model == "fixture-image"' >/dev/null

CHARACTER_JSON="$("${FORGE}" generate character \
  --project "${TEST_ROOT}/project" --spec "${TEST_ROOT}/character-v2-keyframes.json" --wait --json)"
printf '%s' "${CHARACTER_JSON}" | jq -e '.ok and .data.lifecycle_state == "succeeded"' >/dev/null
CHARACTER_JOB="$(printf '%s' "${CHARACTER_JSON}" | jq -r '.data.job_id')"
CHARACTER_JOB_DIR="$(printf '%s' "${CHARACTER_JSON}" | jq -r '.data.job_dir')"
CHARACTER_PACK="$(printf '%s' "${CHARACTER_JSON}" | jq -r '.data.artifacts[] | select(.kind == "gsfpack") | .path')"
"${FORGE}" job graph --id "${CHARACTER_JOB}" --json \
  | jq -e '.ok
    and ([.data.nodes[] | select(.stage == "provider_image")] | length) == 32
    and ([.data.nodes[] | select(.stage == "background_cleanup")] | length) == 32
    and ([.data.nodes[] | select(.stage == "frame_image")] | length) == 32
    and ([.data.nodes[] | select((.stage == "background_cleanup" or .stage == "frame_image") and .providerRequest)] | length) == 0' >/dev/null
"${FORGE}" job report --id "${CHARACTER_JOB}" --json \
  | jq -e '.ok
    and .data.reports.direction_lock.profile == "direction-lock@1.0.0"
    and (.data.reports.direction_lock.entries | length) == 3
    and (.data.reports.provider_manifest.frames | all(.backgroundCleanupProfile == "keyframe-background-cleanup@1.3.0"))' >/dev/null
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

if credential_scan_matches "${FORGE_JOB_STORE}" >/dev/null; then
  echo "credential-like material leaked into the V2 fixture JobStore" >&2
  exit 1
fi

if [[ -n "${REPORT_DIR}" ]]; then
  mkdir -p "${REPORT_DIR}/reports" "${REPORT_DIR}/pack"
  "${FORGE}" job report --id "${CHARACTER_JOB}" --json > "${REPORT_DIR}/reports/job-report.json"
  "${FORGE}" job graph --id "${CHARACTER_JOB}" --json > "${REPORT_DIR}/reports/workflow-graph.json"
  cp "${CHARACTER_JOB_DIR}/source/direction-lock.json" "${REPORT_DIR}/reports/direction-lock.json"
  cp "${CHARACTER_JOB_DIR}/source/keyframe-provider-manifest.json" "${REPORT_DIR}/reports/keyframe-provider-manifest.json"
  cp "${CHARACTER_JOB_DIR}/consistency-report.json" "${REPORT_DIR}/reports/consistency-report.json"
  cp "${CHARACTER_PACK}/character-direction-lock.json" "${REPORT_DIR}/pack/character-direction-lock.json"
  cp "${CHARACTER_PACK}/forgepack.json" "${REPORT_DIR}/pack/forgepack.json"
  if [[ -f "${CHARACTER_PACK}/previews/preview.gif" ]]; then
    cp "${CHARACTER_PACK}/previews/preview.gif" "${REPORT_DIR}/pack/preview.gif"
  fi
  jq -n \
    --arg generatedAt "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --arg commit "$(git -C "${ROOT}" rev-parse HEAD)" \
    --arg jobId "${CHARACTER_JOB}" \
    --arg workflow "topdown-keyframes@2.3.0" \
    --arg directionLock "direction-lock@1.0.0" \
    --arg directionAnchor "direction-anchor@1.2.0" \
    --arg cleanup "keyframe-background-cleanup@1.3.0" \
    '{
      schemaVersion: "1",
      generatedAt: $generatedAt,
      commit: $commit,
      provider: "fixture",
      networkRequests: 0,
      jobId: $jobId,
      workflow: $workflow,
      profiles: {
        directionLock: $directionLock,
        directionAnchor: $directionAnchor,
        backgroundCleanup: $cleanup
      },
      plannedProviderRequests: 32,
      maximumProviderRequests: 64,
      assertions: {
        directionEntries: 3,
        providerNodes: 32,
        backgroundCleanupNodes: 32,
        normalizedFrameNodes: 32,
        singleFrameRetryRequests: 1,
        localReplayRequests: 0,
        packValidated: true,
        credentialScanClean: true
      },
      realProviderAcceptance: "not_run_requires_separate_authorization"
    }' > "${REPORT_DIR}/summary.json"
fi

echo "PASS Forge consistency V2 contract"
