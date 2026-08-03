#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/forge-static-matrix.XXXXXX")"
trap 'rm -rf "${TEST_ROOT}"' EXIT

command -v jq >/dev/null 2>&1
GODOT="${FORGE_GODOT_PATH:-/Applications/Godot.app/Contents/MacOS/Godot}"
if [ ! -x "${GODOT}" ]; then
  GODOT="$(command -v godot || command -v godot4 || true)"
fi
test -x "${GODOT}"

cargo build -q -p forge-cli --manifest-path "${ROOT}/Cargo.toml"
FORGE="${ROOT}/target/debug/forge"
export FORGE_JOB_STORE="${TEST_ROOT}/jobs"
export FORGE_PLAN_STORE="${TEST_ROOT}/plans"
export FORGE_CACHE_STORE="${TEST_ROOT}/cache"

mkdir -p "${TEST_ROOT}/godot"
printf '%s\n' \
  '[application]' \
  'config/name="Forge Static Asset Matrix"' \
  '[rendering]' \
  'renderer/rendering_method="gl_compatibility"' \
  > "${TEST_ROOT}/godot/project.godot"

install_pack() {
  local pack="$1"
  local target="$2"
  local asset_key="$3"
  local plan token
  plan="$("${FORGE}" godot plan-install \
    --pack "${pack}" \
    --project "${TEST_ROOT}/godot" \
    --target "${target}" \
    --asset-key "${asset_key}" \
    --json)"
  token="$(printf '%s' "${plan}" | jq -r '.data.token')"
  "${FORGE}" plan execute --token "${token}" --wait --json \
    | jq -e '.ok and .data.lifecycle_state == "succeeded"' >/dev/null
}

STYLE_COUNT=0
ICON_PACK_COUNT=0
PROP_PACK_COUNT=0
ICON_ITEM_COUNT=0
PROP_ITEM_COUNT=0
ICON_REQUEST_COUNT=0
PROP_REQUEST_COUNT=0

while IFS= read -r style_id; do
  STYLE_COUNT=$((STYLE_COUNT + 1))
  PROJECT="${TEST_ROOT}/projects/${style_id}"
  SPEC_ROOT="${TEST_ROOT}/specs/${style_id}"
  mkdir -p "${SPEC_ROOT}"

  jq --arg id "${style_id}" '.styles[] | select(.id == $id) | .spec' \
    "${ROOT}/benchmarks/character-v2/frozen-20x5.json" \
    > "${SPEC_ROOT}/style.json"

  jq -n --arg style "${style_id}" '{
    schemaVersion: "1",
    kind: "icon_set",
    id: ($style + "-inventory-icons"),
    name: ($style + " Inventory Icons"),
    items: [
      {id: "healing-potion", name: "Healing Potion", prompt: "a red healing potion bottle"},
      {id: "mana-crystal", name: "Mana Crystal", prompt: "a luminous blue mana crystal"},
      {id: "brass-key", name: "Brass Key", prompt: "a small ornate brass key"},
      {id: "coin-pouch", name: "Coin Pouch", prompt: "a tied leather coin pouch"},
      {id: "ancient-scroll", name: "Ancient Scroll", prompt: "a rolled ancient parchment scroll"}
    ],
    license: "MIT"
  }' > "${SPEC_ROOT}/icons.json"

  jq -n --arg style "${style_id}" '{
    schemaVersion: "1",
    kind: "prop_set",
    id: ($style + "-camp-props"),
    name: ($style + " Camp Props"),
    items: [
      {id: "supply-crate", name: "Supply Crate", prompt: "a compact wooden supply crate"},
      {id: "travel-barrel", name: "Travel Barrel", prompt: "a reinforced travel barrel"},
      {id: "campfire-ring", name: "Campfire Ring", prompt: "a stone campfire ring with stacked wood"},
      {id: "trail-signpost", name: "Trail Signpost", prompt: "a weathered wooden trail signpost"},
      {id: "bedroll-pack", name: "Bedroll Pack", prompt: "a rolled bedroll tied to a small pack"}
    ],
    license: "MIT"
  }' > "${SPEC_ROOT}/props.json"

  "${FORGE}" project init \
    --path "${PROJECT}" \
    --name "Static Matrix ${style_id}" \
    --provider fixture --json \
    | jq -e '.ok and .data.project.provider.id == "fixture"' >/dev/null

  "${FORGE}" style create \
    --project "${PROJECT}" \
    --spec "${SPEC_ROOT}/style.json" \
    --wait --json \
    | jq -e '.ok and .data.lifecycle_state == "succeeded"' >/dev/null

  ICON_JSON="$("${FORGE}" generate icon-set \
    --project "${PROJECT}" \
    --spec "${SPEC_ROOT}/icons.json" \
    --wait --json)"
  printf '%s' "${ICON_JSON}" \
    | jq -e '.ok and .data.lifecycle_state == "succeeded"' >/dev/null
  ICON_JOB="$(printf '%s' "${ICON_JSON}" | jq -r '.data.job_id')"
  ICON_PACK="$(printf '%s' "${ICON_JSON}" \
    | jq -r '.data.artifacts[] | select(.kind == "gsfpack") | .path')"
  "${FORGE}" pack validate --path "${ICON_PACK}" --json \
    | jq -e '.ok and .data.valid' >/dev/null
  jq -e '
    .assetType == "icon_set" and
    .verdict == "game_ready" and
    (.items | length) == 5 and
    ([.items[].verdict] | all(. == "game_ready"))
  ' "${ICON_PACK}/consistency-report.json" >/dev/null
  ICON_REQUESTS="$("${FORGE}" job report --id "${ICON_JOB}" --json \
    | jq -r '.data.providerRequestCount')"
  test "${ICON_REQUESTS}" -le 10
  ICON_REQUEST_COUNT=$((ICON_REQUEST_COUNT + ICON_REQUESTS))
  ICON_PACK_COUNT=$((ICON_PACK_COUNT + 1))
  ICON_ITEM_COUNT=$((ICON_ITEM_COUNT + 5))

  PROP_JSON="$("${FORGE}" generate prop-set \
    --project "${PROJECT}" \
    --spec "${SPEC_ROOT}/props.json" \
    --wait --json)"
  printf '%s' "${PROP_JSON}" \
    | jq -e '.ok and .data.lifecycle_state == "succeeded"' >/dev/null
  PROP_JOB="$(printf '%s' "${PROP_JSON}" | jq -r '.data.job_id')"
  PROP_PACK="$(printf '%s' "${PROP_JSON}" \
    | jq -r '.data.artifacts[] | select(.kind == "gsfpack") | .path')"
  "${FORGE}" pack validate --path "${PROP_PACK}" --json \
    | jq -e '.ok and .data.valid' >/dev/null
  jq -e '
    .assetType == "prop_set" and
    .verdict == "game_ready" and
    (.items | length) == 5 and
    ([.items[].verdict] | all(. == "game_ready"))
  ' "${PROP_PACK}/consistency-report.json" >/dev/null
  PROP_REQUESTS="$("${FORGE}" job report --id "${PROP_JOB}" --json \
    | jq -r '.data.providerRequestCount')"
  test "${PROP_REQUESTS}" -le 10
  PROP_REQUEST_COUNT=$((PROP_REQUEST_COUNT + PROP_REQUESTS))
  PROP_PACK_COUNT=$((PROP_PACK_COUNT + 1))
  PROP_ITEM_COUNT=$((PROP_ITEM_COUNT + 5))

  install_pack \
    "${ICON_PACK}" \
    "addons/forge_assets/static_matrix/${style_id}/icons" \
    "${style_id}_icons"
  install_pack \
    "${PROP_PACK}" \
    "addons/forge_assets/static_matrix/${style_id}/props" \
    "${style_id}_props"

  ICON_USAGE="${TEST_ROOT}/godot/addons/forge_assets/static_matrix/${style_id}/icons/forge_usage.json"
  PROP_USAGE="${TEST_ROOT}/godot/addons/forge_assets/static_matrix/${style_id}/props/forge_usage.json"
  jq -e '.kind == "icon_set" and (.items | length) == 5' "${ICON_USAGE}" >/dev/null
  jq -e '.kind == "prop_set" and (.items | length) == 5' "${PROP_USAGE}" >/dev/null
  test "$(find "${TEST_ROOT}/godot/addons/forge_assets/static_matrix/${style_id}/props/scenes" -name '*.tscn' | wc -l | tr -d ' ')" -eq 5
done < <(jq -r '.styles[].id' "${ROOT}/benchmarks/character-v2/frozen-20x5.json")

test "${STYLE_COUNT}" -eq 5
test "${ICON_PACK_COUNT}" -eq 5
test "${PROP_PACK_COUNT}" -eq 5
test "${ICON_ITEM_COUNT}" -eq 25
test "${PROP_ITEM_COUNT}" -eq 25

"${GODOT}" --headless --editor --quit --path "${TEST_ROOT}/godot" >/dev/null

if find "${TEST_ROOT}/godot/addons/forge_assets/static_matrix" \
  -type f \( -name '*.tres' -o -name '*.tscn' \) -size +1048575c \
  | grep -q .; then
  echo "Static matrix resource exceeds 1 MiB" >&2
  exit 1
fi
if grep -R -E \
  'sub_resource type="Image"|sub_resource type="ImageTexture"|ImageTexture.create_from_image|^data = PackedByteArray' \
  "${TEST_ROOT}/godot/addons/forge_assets/static_matrix" \
  --include='*.tres' --include='*.tscn'; then
  echo "Static matrix resource embeds image pixels" >&2
  exit 1
fi
if rg -n -i \
  'authorization:[[:space:]]*bearer|access[_-]?token|refresh[_-]?token|device[_-]?code|xai[_-]?api[_-]?key' \
  "${FORGE_JOB_STORE}" "${TEST_ROOT}/godot" >/dev/null; then
  echo "credential-like material leaked into static matrix outputs" >&2
  exit 1
fi

REPORT_PATH="${FORGE_STATIC_MATRIX_REPORT:-${TEST_ROOT}/static-matrix-report.json}"
mkdir -p "$(dirname "${REPORT_PATH}")"
jq -n \
  --arg profile "static-matrix@1.0.0" \
  --argjson styles "${STYLE_COUNT}" \
  --argjson iconPacks "${ICON_PACK_COUNT}" \
  --argjson propPacks "${PROP_PACK_COUNT}" \
  --argjson iconItems "${ICON_ITEM_COUNT}" \
  --argjson propItems "${PROP_ITEM_COUNT}" \
  --argjson iconRequests "${ICON_REQUEST_COUNT}" \
  --argjson propRequests "${PROP_REQUEST_COUNT}" \
  '{
    schemaVersion: "1",
    profile: $profile,
    providerId: "fixture",
    styles: $styles,
    iconPacks: $iconPacks,
    propPacks: $propPacks,
    iconItems: $iconItems,
    propItems: $propItems,
    iconProviderRequests: $iconRequests,
    propProviderRequests: $propRequests,
    packValidation: "pass",
    consistency: "pass",
    godotValidation: "pass",
    resourceEmbedding: "pass",
    credentialScan: "pass"
  }' > "${REPORT_PATH}"

echo "PASS Forge five-style Icon/Prop matrix"
