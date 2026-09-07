#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/forge-cli-product.XXXXXX")"
trap 'rm -rf "${TEST_ROOT}"' EXIT

command -v jq >/dev/null 2>&1
cargo build -q -p forge-cli --manifest-path "${ROOT}/Cargo.toml"
FORGE="${ROOT}/target/debug/forge"
export FORGE_JOB_STORE="${TEST_ROOT}/jobs"
export FORGE_PLAN_STORE="${TEST_ROOT}/plans"

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

if "${FORGE}" --help | grep -E '^  (subject|schema|component|environment|terrain|building|map)[[:space:]]'; then
  echo "post-v0.2 command leaked into the v0.2 CLI surface" >&2
  exit 1
fi

"${FORGE}" provider models --provider xai --json \
  | jq -e '
      .ok
      and (.data | any(.modelId == "grok-imagine-image-quality" and .routeStatus == "incumbent" and .defaultForKind))
      and (.data | any(.modelId == "grok-imagine-image-2.0" and .routeStatus == "candidate" and .requiresExplicitOptIn and (.defaultForKind | not)))
      and (.data | any(.modelId == "grok-imagine-video-1.5" and .routeStatus == "incumbent" and .defaultForKind))
    ' >/dev/null

"${FORGE}" project init \
  --path "${TEST_ROOT}/assets" \
  --name "CLI Contract" \
  --provider fixture --json \
  | jq -e '.ok and .data.project.provider.id == "fixture"' >/dev/null

"${FORGE}" style create \
  --project "${TEST_ROOT}/assets" \
  --spec "${ROOT}/examples/cli/style.json" \
  --wait --json \
  | jq -e '.ok and .data.lifecycle_state == "succeeded"' >/dev/null

CHARACTER_JSON="$("${FORGE}" generate character \
  --project "${TEST_ROOT}/assets" \
  --spec "${ROOT}/examples/cli/character.json" \
  --wait --json)"
printf '%s' "${CHARACTER_JSON}" | jq -e '.ok and .data.lifecycle_state == "succeeded"' >/dev/null

ICON_JSON="$("${FORGE}" generate icon-set \
  --project "${TEST_ROOT}/assets" \
  --spec "${ROOT}/examples/cli/icons.json" \
  --wait --json)"
printf '%s' "${ICON_JSON}" | jq -e '.ok and .data.lifecycle_state == "succeeded"' >/dev/null

ICON_JOB="$(printf '%s' "${ICON_JSON}" | jq -r '.data.job_id')"
ICON_RETRY_JSON="$("${FORGE}" job retry \
  --id "${ICON_JOB}" --item potion --wait --json)"
printf '%s' "${ICON_RETRY_JSON}" \
  | jq -e '.ok and .data.lifecycle_state == "succeeded" and (.data.artifacts | any(.kind == "reused_item_key"))' >/dev/null
ICON_RETRY_PACK="$(printf '%s' "${ICON_RETRY_JSON}" | jq -r '.data.artifacts[] | select(.kind == "gsfpack") | .path')"
"${FORGE}" pack validate --path "${ICON_RETRY_PACK}" --json \
  | jq -e '.ok and .data.valid' >/dev/null
"${FORGE}" asset inspect --pack "${ICON_RETRY_PACK}" --json \
  | jq -e '.ok and (.data.items | length) == 2' >/dev/null

ICON_RECHECK_JSON="$("${FORGE}" job retry \
  --id "${ICON_JOB}" --stage consistency --wait --json)"
printf '%s' "${ICON_RECHECK_JSON}" \
  | jq -e '.ok and .data.lifecycle_state == "succeeded" and ([.data.artifacts[] | select(.kind | startswith("rechecked_item_"))] | length) == 2' >/dev/null
ICON_RECHECK_JOB="$(printf '%s' "${ICON_RECHECK_JSON}" | jq -r '.data.job_id')"
"${FORGE}" job report --id "${ICON_RECHECK_JOB}" --json \
  | jq -e '.ok and (.data.providerRequestOccurred | not) and .data.reports.provider_usage.usage.requests == 0 and .data.reports.consistency_report.profile == "consistency@1.5.0"' >/dev/null

CHARACTER_JOB="$(printf '%s' "${CHARACTER_JSON}" | jq -r '.data.job_id')"
CHARACTER_ASSEMBLY_JSON="$("${FORGE}" job assemble-character \
  --base "${CHARACTER_JOB}" \
  --source "idle=${CHARACTER_JOB}" \
  --source "walk_up=${CHARACTER_JOB}" \
  --source "walk_right=${CHARACTER_JOB}" \
  --source "walk_down=${CHARACTER_JOB}" \
  --wait --json)"
printf '%s' "${CHARACTER_ASSEMBLY_JSON}" \
  | jq -e '.ok and .data.lifecycle_state == "succeeded" and (.data.artifacts | any(.kind == "character_assembly_manifest")) and (.data.artifacts | any(.kind == "character_framing_report"))' >/dev/null
CHARACTER_ASSEMBLY_JOB="$(printf '%s' "${CHARACTER_ASSEMBLY_JSON}" | jq -r '.data.job_id')"
CHARACTER_ASSEMBLY_PACK="$(printf '%s' "${CHARACTER_ASSEMBLY_JSON}" | jq -r '.data.artifacts[] | select(.kind == "gsfpack") | .path')"
"${FORGE}" job report --id "${CHARACTER_ASSEMBLY_JOB}" --json \
  | jq -e '.ok and (.data.providerRequestOccurred | not) and .data.reports.provider_usage.usage.requests == 0 and (.data.reports.character_assembly_manifest.sources | length) == 4 and .data.reports.character_semantic_quality_report.cameraProfile == "topdown-3q-orthographic@1.0.0" and .data.reports.character_semantic_quality_report.framingProfile == "body-framing@1.0.0"' >/dev/null
"${FORGE}" pack validate --path "${CHARACTER_ASSEMBLY_PACK}" --json \
  | jq -e '.ok and .data.valid' >/dev/null

CHARACTER_RETRY_JSON="$("${FORGE}" job retry \
  --id "${CHARACTER_JOB}" --item walk_right --wait --json)"
printf '%s' "${CHARACTER_RETRY_JSON}" \
  | jq -e '.ok and .data.lifecycle_state == "succeeded" and (.data.artifacts | any(.kind == "reused_video_idle"))' >/dev/null

CHARACTER_LOOP_RETRY_JSON="$("${FORGE}" job retry \
  --id "${CHARACTER_JOB}" --item walk_right --stage loop --wait --json)"
printf '%s' "${CHARACTER_LOOP_RETRY_JSON}" \
  | jq -e '.ok and .data.lifecycle_state == "succeeded" and (.data.artifacts | any(.kind == "workflow_stage_manifest"))' >/dev/null
CHARACTER_LOOP_JOB="$(printf '%s' "${CHARACTER_LOOP_RETRY_JSON}" | jq -r '.data.job_id')"
"${FORGE}" job report --id "${CHARACTER_LOOP_JOB}" --json \
  | jq -e '.ok and (.data.providerRequestOccurred | not) and .data.reports.provider_manifest.animations.walk_right.retryMethod == "loop_reprocess" and (.data.reports.loop_selection_report.animations | length) == 4' >/dev/null

CHARACTER_VIDEO_RETRY_JSON="$("${FORGE}" job retry \
  --id "${CHARACTER_JOB}" --item walk_right --stage video --wait --json)"
printf '%s' "${CHARACTER_VIDEO_RETRY_JSON}" \
  | jq -e '.ok and .data.lifecycle_state == "succeeded"' >/dev/null
CHARACTER_VIDEO_JOB="$(printf '%s' "${CHARACTER_VIDEO_RETRY_JSON}" | jq -r '.data.job_id')"
"${FORGE}" job report --id "${CHARACTER_VIDEO_JOB}" --json \
  | jq -e '.ok and .data.providerRequestOccurred and .data.reports.provider_manifest.usage.generatedImages == 0 and .data.reports.provider_manifest.usage.editedVideos == 1 and .data.reports.provider_manifest.animations.walk_right.retryMethod == "video_edit"' >/dev/null

if "${FORGE}" job retry \
  --id "${ICON_JOB}" --item potion --stage loop --wait --json >/dev/null 2>&1; then
  echo "static asset retry unexpectedly accepted a Character-only stage" >&2
  exit 1
fi

MULTI_FAILURE_JSON="$("${FORGE}" generate icon-set \
  --project "${TEST_ROOT}/assets" \
  --spec "${ROOT}/examples/cli/multi-failure-icons.json" \
  --wait --json)"
printf '%s' "${MULTI_FAILURE_JSON}" \
  | jq -e '.ok and .data.lifecycle_state == "awaiting_review" and .data.error_code == "consistency_review_required"' >/dev/null
MULTI_FAILURE_JOB="$(printf '%s' "${MULTI_FAILURE_JSON}" | jq -r '.data.job_id')"
MULTI_RETRY_JSON="$("${FORGE}" job retry \
  --id "${MULTI_FAILURE_JOB}" --item bad-a --wait --json)"
printf '%s' "${MULTI_RETRY_JSON}" \
  | jq -e '.ok and .data.lifecycle_state == "awaiting_review" and (.data.artifacts | any(.kind == "reused_item_bad-b"))' >/dev/null

PROP_JSON="$("${FORGE}" generate prop-set \
  --project "${TEST_ROOT}/assets" \
  --spec "${ROOT}/examples/cli/props.json" \
  --wait --json)"
printf '%s' "${PROP_JSON}" | jq -e '.ok and .data.lifecycle_state == "succeeded"' >/dev/null

ASYNC_JSON="$("${FORGE}" generate prop-set \
  --project "${TEST_ROOT}/assets" \
  --spec "${ROOT}/examples/cli/props.json" --json)"
ASYNC_JOB="$(printf '%s' "${ASYNC_JSON}" | jq -r '.data.job_id')"
ASYNC_STATE=""
ASYNC_COUNT=0
while [ "${ASYNC_COUNT}" -lt 100 ]; do
  ASYNC_GET="$("${FORGE}" job get --id "${ASYNC_JOB}" --json)"
  ASYNC_STATE="$(printf '%s' "${ASYNC_GET}" | jq -r '.data.lifecycle_state')"
  case "${ASYNC_STATE}" in
    succeeded|failed|awaiting_review|cancelled) break ;;
  esac
  ASYNC_COUNT=$((ASYNC_COUNT + 1))
  sleep 0.1
done
test "${ASYNC_STATE}" = "succeeded"

CANCEL_JSON="$(FORGE_FIXTURE_POLL_PENDING_ONCE=1 "${FORGE}" generate character \
  --project "${TEST_ROOT}/assets" \
  --spec "${ROOT}/examples/cli/character.json" --json)"
CANCEL_JOB="$(printf '%s' "${CANCEL_JSON}" | jq -r '.data.job_id')"
"${FORGE}" job cancel --id "${CANCEL_JOB}" --json \
  | jq -e '.ok and .data.cancellation_requested' >/dev/null
CANCEL_STATE=""
CANCEL_COUNT=0
while [ "${CANCEL_COUNT}" -lt 100 ]; do
  CANCEL_GET="$("${FORGE}" job get --id "${CANCEL_JOB}" --json)"
  CANCEL_STATE="$(printf '%s' "${CANCEL_GET}" | jq -r '.data.lifecycle_state')"
  case "${CANCEL_STATE}" in
    succeeded|failed|awaiting_review|cancelled) break ;;
  esac
  CANCEL_COUNT=$((CANCEL_COUNT + 1))
  sleep 0.1
done
test "${CANCEL_STATE}" = "cancelled"

REVIEW_JSON="$("${FORGE}" generate icon-set \
  --project "${TEST_ROOT}/assets" \
  --spec "${ROOT}/examples/cli/review-icons.json" \
  --wait --json)"
printf '%s' "${REVIEW_JSON}" | jq -e '.ok and .data.lifecycle_state == "awaiting_review"' >/dev/null
REVIEW_JOB="$(printf '%s' "${REVIEW_JSON}" | jq -r '.data.job_id')"
"${FORGE}" job review \
  --id "${REVIEW_JOB}" \
  --accept \
  --reason "fixture gray-band visual review accepted" \
  --json \
  | jq -e '.ok and .data.lifecycle_state == "succeeded" and (.data.artifacts | any(.kind == "gsfpack")) and (.data.artifacts | any(.kind == "review_decision" and (.sha256 | length) == 64)) and (.data.artifacts | any(.kind == "consistency_report" and (.sha256 | length) == 64))' >/dev/null

PACK="$(printf '%s' "${CHARACTER_JSON}" | jq -r '.data.artifacts[] | select(.kind == "gsfpack") | .path')"
"${FORGE}" pack validate --path "${PACK}" --json | jq -e '.ok and .data.valid' >/dev/null
jq -e '.spriteFrames.rendering.profile == "godot-sprite-rendering@1.0.0" and .spriteFrames.rendering.textureFilter == "nearest" and .spriteFrames.rendering.pixelSnap == true' \
  "${PACK}/assets/godot_import.json" >/dev/null

mkdir -p "${TEST_ROOT}/godot"
cp "${ROOT}/examples/godot/forge-import-smoke/project.godot" "${TEST_ROOT}/godot/project.godot"
PLAN_JSON="$("${FORGE}" godot plan-install \
  --pack "${PACK}" \
  --project "${TEST_ROOT}/godot" \
  --asset-key fixture_ranger --json)"
TOKEN="$(printf '%s' "${PLAN_JSON}" | jq -r '.data.token')"
"${FORGE}" plan execute --token "${TOKEN}" --wait --json \
  | jq -e '.ok and .data.lifecycle_state == "succeeded"' >/dev/null

RESOURCE="${TEST_ROOT}/godot/addons/forge_assets/fixture-ranger/forge_sprite_frames.tres"
SCENE="${TEST_ROOT}/godot/addons/forge_assets/fixture-ranger/forge_animated_sprite.tscn"
test -f "${RESOURCE}"
test -f "${SCENE}"
test "$(stat -f '%z' "${RESOURCE}")" -lt 1048576
! grep -q 'PackedByteArray\|sub_resource type="Image"' "${RESOURCE}"
grep -q 'texture_filter = 1' "${SCENE}"
grep -q 'centered = false' "${SCENE}"

if credential_scan_matches "${FORGE_JOB_STORE}" >/dev/null; then
  echo "credential-like material leaked into the fixture JobStore" >&2
  exit 1
fi

echo "PASS Forge CLI product contract"
