#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/forge-stage3-static.XXXXXX")"
trap 'rm -rf "${TEST_ROOT}"' EXIT

for schema_file in "${ROOT}"/schemas/*.json; do
  jq empty "${schema_file}"
done

cargo test --manifest-path "${ROOT}/Cargo.toml" \
  -p providers --test stage3_static_contract

cargo build -q --manifest-path "${ROOT}/Cargo.toml" \
  -p forge-cli --features collection-assets

FORGE="${ROOT}/target/debug/forge"
for command in collection portrait-set equipment-set decal-set export-editable replace-item audit; do
  case "${command}" in
    portrait-set|equipment-set|decal-set)
      "${FORGE}" generate --help | grep -q "${command}"
      ;;
    export-editable|replace-item)
      "${FORGE}" asset --help | grep -q "${command}"
      ;;
    audit)
      "${FORGE}" project --help | grep -q "${command}"
      ;;
    *)
      "${FORGE}" --help | grep -q "${command}"
      ;;
  esac
done

"${FORGE}" schema list --json \
  | jq -e '.ok
    and (.data.schemas | index("collection@1.0.0"))
    and (.data.schemas | index("portrait-set@2.0.0"))
    and (.data.schemas | index("provider-authorization@1.0.0"))
    and (.data.schemas | index("provider-request-ledger@1.0.0"))
    and (.data.schemas | index("collection-anchor-quality-report@1.1.0"))
    and (.data.schemas | index("collection-consistency-report@1.1.0"))
    and (.data.schemas | index("portrait-base-lock@1.0.0"))
    and (.data.schemas | index("portrait-base-approval@1.0.0"))
    and (.data.schemas | index("portrait-local@1.0.0"))
    and (.data.schemas | index("portrait-local@1.1.0"))
    and (.data.schemas | index("project-audit-report@1.1.0"))' >/dev/null
"${FORGE}" schema show --id collection-anchor-quality-report@1.1.0 --json \
  | jq -e '.ok and .data.properties.profile.const == "collection-anchor@1.1.0"' >/dev/null
"${FORGE}" schema show --id portrait-base-lock@1.0.0 --json \
  | jq -e '.ok and .data.properties.profile.const == "portrait-base-lock@1.0.0"' >/dev/null
"${FORGE}" schema show --id portrait-base-approval@1.0.0 --json \
  | jq -e '.ok and .data.properties.profile.const == "portrait-base-approval@1.0.0"' >/dev/null
"${FORGE}" schema show --id portrait-local@1.1.0 --json \
  | jq -e '.ok and (.data.properties.profile.enum | index("portrait-local@1.0.0")) and (.data.properties.profile.enum | index("portrait-local@1.1.0"))' >/dev/null

export FORGE_JOB_STORE="${TEST_ROOT}/jobs"
export FORGE_PLAN_STORE="${TEST_ROOT}/plans"
export FORGE_AUTHORIZATION_STORE="${TEST_ROOT}/authorizations"
ASSETS="${TEST_ROOT}/assets"
"${FORGE}" project init --path "${ASSETS}" --name "Stage 3 CLI" --provider fixture --json \
  | jq -e '.ok' >/dev/null
"${FORGE}" style create --project "${ASSETS}" --spec "${ROOT}/examples/cli/style.json" --plan-only --json \
  | jq -e '.ok and .data.estimate.providerRequestEstimate == 1 and .data.estimate.maximumProviderRequests == 1' >/dev/null
jq -e 'has("currentStyleRevision") | not' "${ASSETS}/forge-project.json" >/dev/null
test -z "$(find "${FORGE_JOB_STORE}" -mindepth 1 -maxdepth 1 -type d -print -quit 2>/dev/null)"
"${FORGE}" provider authorize --provider fixture --profile default --id stage3-style \
  --target style_board --max-requests-per-target 1 --max-requests 1 \
  --max-cost-ticks 100 --cost-reservation-ticks-per-request 100 \
  --model fixture-image --json \
  | jq -e '.ok and (.data.secretMaterialStored | not)
    and .data.ledger.requests == []' >/dev/null
"${FORGE}" style create --project "${ASSETS}" --spec "${ROOT}/examples/cli/style.json" \
  --authorization stage3-style --wait --json \
  | jq -e '.ok and .data.lifecycle_state == "succeeded"
    and .data.authorization_id == "stage3-style"' >/dev/null

LEGACY_JSON="$("${FORGE}" generate icon-set --project "${ASSETS}" --spec "${ROOT}/examples/cli/icons.json" --wait --json)"
ANCHOR_SOURCE="$(printf '%s' "${LEGACY_JSON}" | jq -r '.data.artifacts[] | select(.kind == "gsfpack") | .path + "/assets/items/potion.png"')"
cp "${ANCHOR_SOURCE}" "${ASSETS}/specs/inventory-anchor.png"
jq -n --arg anchor "${ASSETS}/specs/inventory-anchor.png" '{
  schemaVersion:"1", id:"inventory-v2", name:"Inventory V2", assetKind:"icon_set",
  prompt:"cohesive purple inventory icons", materials:["painted metal"], scale:"consistent",
  perspective:"inherit_style", grounding:"center", canvasSize:128, anchorImage:$anchor, license:"MIT"
}' > "${ASSETS}/specs/collection.json"

"${FORGE}" collection create --project "${ASSETS}" --spec "${ASSETS}/specs/collection.json" --plan-only --json \
  | jq -e '.ok and .data.estimate.providerRequestEstimate == 0 and .data.estimate.maximumProviderRequests == 0' >/dev/null
COLLECTION_JSON="$("${FORGE}" collection create --project "${ASSETS}" --spec "${ASSETS}/specs/collection.json" --wait --json)"
printf '%s' "${COLLECTION_JSON}" \
  | jq -e '.ok and (.data.artifacts | any(.kind == "collection_anchor_quality_report"))' >/dev/null
COLLECTION_REVISION="$(printf '%s' "${COLLECTION_JSON}" | jq -r '.data.asset_id | split("@")[1]')"
jq -n --arg revision "${COLLECTION_REVISION}" '{
  schemaVersion:"2", kind:"icon_set", id:"inventory-icons-v2", name:"Inventory Icons V2",
  collection:{id:"inventory-v2", revision:$revision},
  items:[{id:"potion",name:"Potion",prompt:"a purple potion"},{id:"key",name:"Key",prompt:"a purple key"}],
  license:"MIT"
}' > "${ASSETS}/specs/icons-v2.json"

V2_JSON="$("${FORGE}" generate icon-set --project "${ASSETS}" --spec "${ASSETS}/specs/icons-v2.json" --wait --json)"
printf '%s' "${V2_JSON}" | jq -e '.ok and .data.lifecycle_state == "succeeded"' >/dev/null
V2_JOB="$(printf '%s' "${V2_JSON}" | jq -r '.data.job_id')"

jq -n '{
  schemaVersion:"1", id:"two-phase-hero", name:"Two Phase Hero",
  prompt:"a compact purple ranger canonical identity image", referenceImages:[], license:"MIT"
}' > "${ASSETS}/specs/two-phase-subject.json"
SUBJECT_JSON="$("${FORGE}" subject create --project "${ASSETS}" --spec "${ASSETS}/specs/two-phase-subject.json" --wait --json)"
printf '%s' "${SUBJECT_JSON}" | jq -e '.ok and .data.lifecycle_state == "succeeded"' >/dev/null
SUBJECT_REVISION="$(printf '%s' "${SUBJECT_JSON}" | jq -r '.data.asset_id | split("@")[1]')"
SUBJECT_INSPECT="$("${FORGE}" subject inspect --project "${ASSETS}" --id two-phase-hero --revision "${SUBJECT_REVISION}" --json)"
SUBJECT_CANONICAL="$(printf '%s' "${SUBJECT_INSPECT}" | jq -r '.data.canonicalPath')"
jq -n --arg anchor "${SUBJECT_CANONICAL}" '{
  schemaVersion:"1", id:"two-phase-portraits", name:"Two Phase Portraits",
  assetKind:"portrait_set", prompt:"cohesive purple ranger portraits", materials:["cloth","leather"],
  scale:"consistent", perspective:"inherit_style", grounding:"center", canvasSize:256,
  framingProfile:"dialogue_bust@1.0.0", anchorImage:$anchor, license:"MIT"
}' > "${ASSETS}/specs/two-phase-collection.json"
PORTRAIT_COLLECTION_JSON="$("${FORGE}" collection create --project "${ASSETS}" --spec "${ASSETS}/specs/two-phase-collection.json" --wait --json)"
printf '%s' "${PORTRAIT_COLLECTION_JSON}" | jq -e '.ok and .data.lifecycle_state == "succeeded"' >/dev/null
PORTRAIT_COLLECTION_REVISION="$(printf '%s' "${PORTRAIT_COLLECTION_JSON}" | jq -r '.data.asset_id | split("@")[1]')"
jq -n --arg collectionRevision "${PORTRAIT_COLLECTION_REVISION}" --arg subjectRevision "${SUBJECT_REVISION}" '{
  schemaVersion:"2", kind:"portrait_set", id:"two-phase-hero-portraits", name:"Two Phase Hero Portraits",
  collection:{id:"two-phase-portraits",revision:$collectionRevision},
  subject:{id:"two-phase-hero",revision:$subjectRevision}, framingProfile:"dialogue_bust@1.0.0",
  expressions:[
    {id:"neutral",name:"Neutral",prompt:"neutral expression"},
    {id:"happy",name:"Happy",prompt:"happy expression"},
    {id:"angry",name:"Angry",prompt:"angry expression"},
    {id:"hurt",name:"Hurt",prompt:"hurt expression"},
    {id:"surprised",name:"Surprised",prompt:"surprised expression"}
  ], license:"MIT"
}' > "${ASSETS}/specs/two-phase-portraits.json"
"${FORGE}" generate portrait-set --project "${ASSETS}" --spec "${ASSETS}/specs/two-phase-portraits.json" \
  --phase base --neutral-reference-policy subject-style --plan-only --json \
  | jq -e '.ok and .data.estimate.providerRequestEstimate == 1
    and .data.estimate.maximumProviderRequests == 2
    and .data.estimate.workflow == "portrait-base@1.0.0"' >/dev/null
PORTRAIT_BASE_JSON="$("${FORGE}" generate portrait-set --project "${ASSETS}" --spec "${ASSETS}/specs/two-phase-portraits.json" \
  --phase base --neutral-reference-policy subject-style --wait --json)"
printf '%s' "${PORTRAIT_BASE_JSON}" \
  | jq -e '.ok and .data.lifecycle_state == "awaiting_review"
    and .data.error_code == "portrait_base_review_required"
    and (.data.artifacts | any(.kind == "portrait_base_lock"))
    and (.data.artifacts | all(.kind != "gsfpack"))' >/dev/null
PORTRAIT_BASE_JOB="$(printf '%s' "${PORTRAIT_BASE_JSON}" | jq -r '.data.job_id')"
"${FORGE}" job review --id "${PORTRAIT_BASE_JOB}" --accept \
  --reason "fixture neutral identity and equipment approved" --json \
  | jq -e '.ok and .data.lifecycle_state == "succeeded"
    and (.data.artifacts | any(.kind == "portrait_base_approval"))
    and (.data.artifacts | all(.kind != "gsfpack"))' >/dev/null
"${FORGE}" generate portrait-set --project "${ASSETS}" --spec "${ASSETS}/specs/two-phase-portraits.json" \
  --phase expressions --base-job "${PORTRAIT_BASE_JOB}" --plan-only --json \
  | jq -e '.ok and .data.estimate.providerRequestEstimate == 4
    and .data.estimate.maximumProviderRequests == 8
    and .data.estimate.workflow == "portrait-expressions@1.0.0"' >/dev/null
PORTRAIT_JSON="$("${FORGE}" generate portrait-set --project "${ASSETS}" --spec "${ASSETS}/specs/two-phase-portraits.json" \
  --phase expressions --base-job "${PORTRAIT_BASE_JOB}" --wait --json)"
PORTRAIT_JOB="$(printf '%s' "${PORTRAIT_JSON}" | jq -r '.data.job_id')"
if ! printf '%s' "${PORTRAIT_JSON}" | jq -e '.ok and .data.lifecycle_state == "succeeded"' >/dev/null; then
  printf '%s' "${PORTRAIT_JSON}" | jq -e '.ok and .data.lifecycle_state == "awaiting_review"' >/dev/null
  PORTRAIT_JSON="$("${FORGE}" job review --id "${PORTRAIT_JOB}" --accept \
    --reason "fixture expression-only gray band accepted" --json)"
  printf '%s' "${PORTRAIT_JSON}" | jq -e '.ok and .data.lifecycle_state == "succeeded"' >/dev/null
fi
PORTRAIT_PACK="$(printf '%s' "${PORTRAIT_JSON}" | jq -r '.data.artifacts[] | select(.kind == "gsfpack") | .path')"
"${FORGE}" pack validate --path "${PORTRAIT_PACK}" --json | jq -e '.ok and .data.valid' >/dev/null
jq -e '.source.metadata.portraitNeutralReferencePolicy == "subject-style@1.0.0"
  and .source.metadata.portraitBaseApprovalSourceJobId == $base
  and .assets.portraitBaseApproval == "portrait-base-approval.json"' \
  --arg base "${PORTRAIT_BASE_JOB}" "${PORTRAIT_PACK}/forgepack.json" >/dev/null
"${FORGE}" job report --id "${PORTRAIT_JOB}" --json \
  | jq -e '.ok and .data.providerRequestOccurred
    and .data.reports.provider_usage.usage.requests == 4
    and .data.reports.portrait_base_approval.accepted' >/dev/null
set +e
NEUTRAL_RETRY_JSON="$("${FORGE}" job retry --id "${PORTRAIT_JOB}" --item neutral --wait --json 2>/dev/null)"
NEUTRAL_RETRY_STATUS=$?
set -e
test "${NEUTRAL_RETRY_STATUS}" -ne 0
printf '%s' "${NEUTRAL_RETRY_JSON}" \
  | jq -e '(.ok | not) and .error.code == "portrait_base_immutable"' >/dev/null

"${FORGE}" asset export-editable --project "${ASSETS}" --id inventory-icons-v2 --output "${TEST_ROOT}/editable" --json \
  | jq -e '.ok and .data.itemCount == 2' >/dev/null
REPLACED_JSON="$("${FORGE}" asset replace-item --id "${V2_JOB}" --item potion --path "${TEST_ROOT}/editable/items/potion.png" --wait --json)"
printf '%s' "${REPLACED_JSON}" \
  | jq -e '.ok and .data.lifecycle_state == "succeeded" and (.data.artifacts | any(.kind == "replacement_item_potion"))' >/dev/null
REPLACED_JOB="$(printf '%s' "${REPLACED_JSON}" | jq -r '.data.job_id')"
"${FORGE}" job report --id "${REPLACED_JOB}" --json \
  | jq -e '.ok and (.data.providerRequestOccurred | not) and .data.reports.provider_usage.usage.requests == 0' >/dev/null
"${FORGE}" project audit --project "${ASSETS}" --scope all --json \
  | jq -e '.ok and .data.summary.errorCount == 0' >/dev/null

REVIEW_ASSETS="${TEST_ROOT}/review-assets"
"${FORGE}" project init --path "${REVIEW_ASSETS}" --name "Stage 3 Review" --provider fixture --json \
  | jq -e '.ok' >/dev/null
"${FORGE}" style create --project "${REVIEW_ASSETS}" --spec "${ROOT}/examples/cli/style.json" --wait --json \
  | jq -e '.ok and .data.lifecycle_state == "succeeded"' >/dev/null
REVIEW_JSON="$("${FORGE}" generate icon-set --project "${REVIEW_ASSETS}" --spec "${ROOT}/examples/cli/review-icons.json" --wait --json)"
printf '%s' "${REVIEW_JSON}" \
  | jq -e '.ok and .data.lifecycle_state == "awaiting_review"' >/dev/null
REVIEW_JOB="$(printf '%s' "${REVIEW_JSON}" | jq -r '.data.job_id')"
APPROVED_JSON="$("${FORGE}" job review --id "${REVIEW_JOB}" --accept --reason "fixture gray-zone accepted" --json)"
printf '%s' "${APPROVED_JSON}" \
  | jq -e '.ok and .data.lifecycle_state == "succeeded"
    and (.data.artifacts | any(.kind == "gsfpack" and (.sha256 | length == 64)))
    and (.data.artifacts | any(.kind == "project_catalog" and (.sha256 | length == 64)))' >/dev/null
jq -e '.assets["review-icons"].gameReady == true
  and .assets["review-icons"].review.status == "approved"
  and (.assets["review-icons"].packSha256 | length == 64)' \
  "${REVIEW_ASSETS}/.forge/catalog.json" >/dev/null
"${FORGE}" project audit --project "${REVIEW_ASSETS}" --scope all --json \
  | jq -e '.ok and .data.summary.errorCount == 0' >/dev/null
"${FORGE}" job review --id "${REVIEW_JOB}" --reason "human semantic rejection" --json \
  | jq -e '.ok and .data.lifecycle_state == "succeeded" and .data.error_code == "manual_rejected"' >/dev/null
jq -e '.assets["review-icons"].gameReady == false
  and .assets["review-icons"].review.status == "quarantined"
  and .assets["review-icons"].qualityVerdict == "manual_rejected"' \
  "${REVIEW_ASSETS}/.forge/catalog.json" >/dev/null
"${FORGE}" project audit --project "${REVIEW_ASSETS}" --scope all --json \
  | jq -e '.ok and .data.summary.errorCount > 0
    and (.data.findings | any(.code == "catalog_asset_quarantined"))' >/dev/null

if rg -n -i 'authorization:[[:space:]]*bearer|access[_-]?token|refresh[_-]?token|device[_-]?code|xai[_-]?api[_-]?key' "${TEST_ROOT}" >/dev/null; then
  echo "credential-like material leaked into Stage 3 CLI artifacts" >&2
  exit 1
fi

echo "PASS Forge Stage 3 static asset contract"
