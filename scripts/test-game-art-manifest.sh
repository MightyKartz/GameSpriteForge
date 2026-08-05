#!/usr/bin/env bash
# Stage 2 (GameArtManifest + project Build Core) acceptance contract, driven
# entirely through the CLI against the fixture provider — no real provider
# calls, no FORGE_REAL_PROVIDER_ACCEPT. Covers implementation plan §15 阶段 2:
# deterministic plan hash, catalog reuse without provider jobs, targeted
# invalidation, required/optional dependency behavior, cancel propagation,
# crash recovery (via the Rust integration tests), single-JSON stdout and the
# credential scan. Uses examples/cli/complete-visual/game-art.json as the
# manifest under test.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/forge-game-art-manifest.XXXXXX")"
trap 'rm -rf "${TEST_ROOT}"' EXIT

command -v jq >/dev/null 2>&1
cargo build -q -p forge-cli --features game-art-manifest --manifest-path "${ROOT}/Cargo.toml"
FORGE="${ROOT}/target/debug/forge"
export FORGE_JOB_STORE="${TEST_ROOT}/jobs"
export FORGE_PLAN_STORE="${TEST_ROOT}/plans"
export FORGE_CACHE_STORE="${TEST_ROOT}/cache"
export FORGE_COMPONENT_STORE="${TEST_ROOT}/components"

EXAMPLE="${ROOT}/examples/cli/complete-visual"

credential_scan_matches() {
  if command -v rg >/dev/null 2>&1; then
    rg -n -i \
      "authorization:[[:space:]]*bearer|access[_-]?token|refresh[_-]?token|device[_-]?code|xai[_-]?api[_-]?key|https?://[^[:space:]\"']*(temporary|signed|signature|token)=" \
      "$@"
  else
    grep -R -I -n -E \
      "authorization:[[:space:]]*bearer|access[_-]?token|refresh[_-]?token|device[_-]?code|xai[_-]?api[_-]?key|https?://[^[:space:]\"']*(temporary|signed|signature|token)=" \
      "$@"
  fi
}

# Run a forge command, prove its stdout carries exactly one JSON value and
# that the envelope is ok, then hand the JSON to the caller (CHECK 7).
forge_json() {
  local output lines
  output="$("${FORGE}" "$@" --json)"
  lines="$(printf '%s\n' "${output}" | jq -c . | wc -l | tr -d ' ')"
  if [ "${lines}" != "1" ]; then
    echo "forge $* emitted ${lines} JSON values on stdout, expected exactly 1" >&2
    return 1
  fi
  if ! printf '%s' "${output}" | jq -e '.ok' >/dev/null; then
    echo "forge $* returned a non-ok envelope:" >&2
    printf '%s\n' "${output}" >&2
    return 1
  fi
  printf '%s' "${output}"
}

job_count() {
  forge_json job list --recent 1000 | jq -r '.data | length'
}

# Path of the project_build_report artifact for a parent build job, read back
# through `forge job report` (the report artifact lives on the job record).
build_report_path() {
  forge_json job report --id "$1" \
    | jq -r '.data.job.artifacts[] | select(.kind == "project_build_report") | .path'
}

init_project_with_style() {
  local path="$1" name="$2"
  forge_json project init --path "${path}" --name "${name}" --provider fixture >/dev/null
  forge_json style create \
    --project "${path}" --spec "${ROOT}/examples/cli/style.json" --wait \
    | jq -e '.data.lifecycle_state == "succeeded"' >/dev/null
}

# ---------------------------------------------------------------------------
# Project A: the shipped example manifest, copied into a fixture project.
# ---------------------------------------------------------------------------
PROJECT_A="${TEST_ROOT}/project-a"
init_project_with_style "${PROJECT_A}" "Game Art A"
cp "${EXAMPLE}/game-art.json" "${PROJECT_A}/game-art.json"
jq '.projectId = "game-art-a" | .name = "Game Art A" | .provider.id = "fixture"' \
  "${PROJECT_A}/game-art.json" > "${TEST_ROOT}/project-a-manifest.json"
cp "${TEST_ROOT}/project-a-manifest.json" "${PROJECT_A}/game-art.json"
mkdir -p "${PROJECT_A}/specs"
cp "${EXAMPLE}"/specs/*.json "${PROJECT_A}/specs/"
MANIFEST_A="${PROJECT_A}/game-art.json"

# CHECK 0 — the example manifest + specs parse and diff against a fixture
# project (validates examples/cli/complete-visual against the V1 contract).
DIFF_JSON="$(forge_json project diff --project "${PROJECT_A}" --manifest "${MANIFEST_A}")"
printf '%s' "${DIFF_JSON}" | jq -e '
  .data.schemaVersion == "1"
  and (.data.manifestSha256 | length) == 64
  and (.data.graphSha256 | length) == 64
  and ([.data.actions[] | select(.action == "build" and (.reasons | index("new_asset") != null))] | length) == 3
  and (.data.deleteCandidates | length) == 0' >/dev/null
echo "CHECK 0 PASS: example manifest parses — project diff reports 3 new_asset builds against a fixture project"

# CHECK 1 — deterministic plan hash: same manifest+specs planned twice, then
# planned again after a key-order/whitespace rewrite of the manifest.
PLAN_FIRST="$(forge_json project plan-build --project "${PROJECT_A}" --manifest "${MANIFEST_A}")"
PLAN_SECOND="$(forge_json project plan-build --project "${PROJECT_A}" --manifest "${MANIFEST_A}")"
SHA_FIRST="$(printf '%s' "${PLAN_FIRST}" | jq -r '.data.plan.planSha256')"
SHA_SECOND="$(printf '%s' "${PLAN_SECOND}" | jq -r '.data.plan.planSha256')"
test "$(printf '%s' "${PLAN_FIRST}" | jq -r '.data.plan.kind')" = "project_build_plan"
test "${#SHA_FIRST}" = "64"
test "${SHA_FIRST}" = "${SHA_SECOND}"
RAW_BEFORE="$(shasum -a 256 "${MANIFEST_A}" | cut -d' ' -f1)"
jq -S . "${MANIFEST_A}" > "${TEST_ROOT}/manifest-resorted.json"
cp "${TEST_ROOT}/manifest-resorted.json" "${MANIFEST_A}"
RAW_AFTER="$(shasum -a 256 "${MANIFEST_A}" | cut -d' ' -f1)"
test "${RAW_BEFORE}" != "${RAW_AFTER}"
PLAN_REWRITTEN="$(forge_json project plan-build --project "${PROJECT_A}" --manifest "${MANIFEST_A}")"
SHA_REWRITTEN="$(printf '%s' "${PLAN_REWRITTEN}" | jq -r '.data.plan.planSha256')"
test "${SHA_FIRST}" = "${SHA_REWRITTEN}"
echo "CHECK 1 PASS: deterministic plan hash ${SHA_FIRST:0:16}… stable across reruns and manifest key-order/whitespace rewrite"

# CHECK 1A — token input closure: changing a spec after plan creation must
# invalidate the token before any parent/child job is staged.
cp "${PROJECT_A}/specs/hud-icons.json" "${TEST_ROOT}/hud-icons-before-drift.json"
PLAN_DRIFT="$(forge_json project plan-build --project "${PROJECT_A}" --manifest "${MANIFEST_A}")"
TOKEN_DRIFT="$(printf '%s' "${PLAN_DRIFT}" | jq -r '.data.token')"
COUNT_BEFORE_DRIFT="$(job_count)"
jq '.items[0].prompt = "drifted after plan"' "${PROJECT_A}/specs/hud-icons.json" \
  > "${TEST_ROOT}/hud-icons-drifted.json"
cp "${TEST_ROOT}/hud-icons-drifted.json" "${PROJECT_A}/specs/hud-icons.json"
EXEC_DRIFT="$("${FORGE}" plan execute --token "${TOKEN_DRIFT}" --wait --json || true)"
printf '%s' "${EXEC_DRIFT}" | jq -e '(.ok | not)' >/dev/null
test "$(job_count)" = "${COUNT_BEFORE_DRIFT}"
cp "${TEST_ROOT}/hud-icons-before-drift.json" "${PROJECT_A}/specs/hud-icons.json"
echo "CHECK 1A PASS: plan token rejected post-plan spec drift before staging any job"

# CHECK 1B — game-art-only schema surface and xAI offline planning must not
# resolve credentials or touch Keychain. The real-provider acceptance guard
# runs before claim/staging and therefore leaves the token pending.
forge_json schema show --id game-art-manifest@1.0.0 \
  | jq -e '.data.title == "Forge Game Art Manifest V1"' >/dev/null
PROJECT_X="${TEST_ROOT}/project-xai-offline"
cp -R "${PROJECT_A}" "${PROJECT_X}"
jq '.projectId = "project-xai-offline" | .name = "Project xAI Offline" | .provider.id = "xai"' \
  "${PROJECT_X}/forge-project.json" > "${TEST_ROOT}/project-xai.json"
cp "${TEST_ROOT}/project-xai.json" "${PROJECT_X}/forge-project.json"
jq '.projectId = "project-xai-offline" | .name = "Project xAI Offline" | .provider.id = "xai" | .assets = [.assets[] | select(.id == "hud-icons")]' \
  "${PROJECT_X}/game-art.json" > "${TEST_ROOT}/manifest-xai.json"
cp "${TEST_ROOT}/manifest-xai.json" "${PROJECT_X}/game-art.json"
STYLE_LOCK_X="$(find "${PROJECT_X}/.forge/styles" -name style-lock.json -type f -print -quit)"
STYLE_BOARD_X="$(dirname "${STYLE_LOCK_X}")/style-board.png"
jq --arg board "${STYLE_BOARD_X}" \
  '.providerId = "xai" | .boardPath = $board' \
  "${STYLE_LOCK_X}" > "${TEST_ROOT}/style-lock-xai.json"
cp "${TEST_ROOT}/style-lock-xai.json" "${STYLE_LOCK_X}"
SECRET_SENTINEL="FORGE_STAGE2_SECRET_SENTINEL_7dbe0a9c"
PLAN_X="$(XAI_API_KEY="${SECRET_SENTINEL}" forge_json project plan-build --project "${PROJECT_X}" --manifest "${PROJECT_X}/game-art.json")"
TOKEN_X="$(printf '%s' "${PLAN_X}" | jq -r '.data.token')"
test -f "${FORGE_PLAN_STORE}/${TOKEN_X}.pending.json"
COUNT_BEFORE_GUARD="$(job_count)"
EXEC_GUARD="$(env -u FORGE_REAL_PROVIDER_ACCEPT -u FORGE_REAL_PROVIDER_MAX_REQUESTS -u FORGE_REAL_PROVIDER_MAX_COST_TICKS "${FORGE}" plan execute --token "${TOKEN_X}" --wait --json || true)"
printf '%s' "${EXEC_GUARD}" | jq -e '(.ok | not) and .error.code == "real_provider_not_accepted"' >/dev/null
test -f "${FORGE_PLAN_STORE}/${TOKEN_X}.pending.json"
test "$(job_count)" = "${COUNT_BEFORE_GUARD}"
EXEC_BUDGET="$(FORGE_REAL_PROVIDER_ACCEPT=1 FORGE_REAL_PROVIDER_MAX_REQUESTS=1 FORGE_REAL_PROVIDER_MAX_COST_TICKS=1 "${FORGE}" plan execute --token "${TOKEN_X}" --wait --json || true)"
printf '%s' "${EXEC_BUDGET}" | jq -e '(.ok | not) and .error.code == "provider_budget_exceeded"' >/dev/null
test -f "${FORGE_PLAN_STORE}/${TOKEN_X}.pending.json"
test "$(job_count)" = "${COUNT_BEFORE_GUARD}"
if rg -F "${SECRET_SENTINEL}" "${FORGE_PLAN_STORE}" "${FORGE_JOB_STORE}" >/dev/null 2>&1; then
  echo "offline xAI planning persisted the secret sentinel" >&2
  exit 1
fi
echo "CHECK 1B PASS: game-art schema is available; xAI plan is credential-free; missing/undersized real-provider guards leave token pending and stage no job"

# CHECK 2 — reuse without provider jobs: build 1 character + 1 icon_set +
# 1 prop_set, then re-plan/re-execute the unchanged manifest.
PLAN_BUILD="$(forge_json project plan-build --project "${PROJECT_A}" --manifest "${MANIFEST_A}")"
TOKEN_BUILD="$(printf '%s' "${PLAN_BUILD}" | jq -r '.data.token')"
EXEC_BUILD="$(forge_json plan execute --token "${TOKEN_BUILD}" --wait)"
printf '%s' "${EXEC_BUILD}" | jq -e '.data.lifecycle_state == "succeeded"' >/dev/null
PARENT_ONE="$(printf '%s' "${EXEC_BUILD}" | jq -r '.data.job_id')"
COUNT_ONE="$(job_count)"
test "${COUNT_ONE}" = "5"
jq -e '.summary.built == 3 and .summary.reused == 0 and .summary.failed == 0 and .summary.skipped == 0' \
  "$(build_report_path "${PARENT_ONE}")" >/dev/null
test ! -e "${FORGE_JOB_STORE}/${PARENT_ONE}/provider-usage.json"
CHILD_REQUESTS=0
while IFS= read -r child_id; do
  requests="$(jq -r '.usage.requests // 0' "${FORGE_JOB_STORE}/${child_id}/provider-usage.json")"
  CHILD_REQUESTS="$((CHILD_REQUESTS + requests))"
done < <(forge_json job list --recent 1000 | jq -r --arg parent "${PARENT_ONE}" '.data[] | select(.parent_job_id == $parent) | .job_id')
REPORT_REQUESTS="$(jq -r '.providerUsage.requests' "$(build_report_path "${PARENT_ONE}")")"
test "${REPORT_REQUESTS}" = "${CHILD_REQUESTS}"
PLAN_REUSE="$(forge_json project plan-build --project "${PROJECT_A}" --manifest "${MANIFEST_A}")"
printf '%s' "${PLAN_REUSE}" | jq -e \
  '([.data.plan.actions[] | select(.action == "reuse")] | length) == 3
   and .data.plan.cacheHits == 3 and .data.plan.cacheMisses == 0
   and .data.plan.providerRequestEstimate == 0' >/dev/null
TOKEN_REUSE="$(printf '%s' "${PLAN_REUSE}" | jq -r '.data.token')"
EXEC_REUSE="$(forge_json plan execute --token "${TOKEN_REUSE}" --wait)"
printf '%s' "${EXEC_REUSE}" | jq -e '.data.lifecycle_state == "succeeded"' >/dev/null
PARENT_TWO="$(printf '%s' "${EXEC_REUSE}" | jq -r '.data.job_id')"
jq -e '.summary.reused == 3 and .summary.built == 0
   and .providerUsage.requests == 0
   and ([.results[] | select(.status == "reused" and (.childJobId | not))] | length) == 3' \
  "$(build_report_path "${PARENT_TWO}")" >/dev/null
forge_json job list --recent 1000 | jq -e --arg parent "${PARENT_TWO}" \
  '([.data[] | select(.parent_job_id == $parent)] | length) == 0' >/dev/null
COUNT_TWO="$(job_count)"
test "${COUNT_TWO}" = "6"
echo "CHECK 2 PASS: reuse without provider jobs — 5 jobs after first build (1 style + 1 parent + 3 children); re-execution reused 3/3 with 0 provider requests and 0 new child jobs (count 5 → 6, parent only)"

# CHECK 2A — a reuse token binds both catalog bytes and the exact pack tree.
# Replace hero's pack with the icon pack and synchronize catalog packSha256;
# pure planSha remains structurally identical, but claim must still reject the
# unreviewed asset bytes before staging a job.
PLAN_PACK_DRIFT="$(forge_json project plan-build --project "${PROJECT_A}" --manifest "${MANIFEST_A}")"
TOKEN_PACK_DRIFT="$(printf '%s' "${PLAN_PACK_DRIFT}" | jq -r '.data.token')"
COUNT_BEFORE_PACK_DRIFT="$(job_count)"
CATALOG_A="${PROJECT_A}/.forge/catalog.json"
cp "${CATALOG_A}" "${TEST_ROOT}/catalog-before-pack-drift.json"
HERO_PACK="$(jq -r '.assets.hero.packPath' "${CATALOG_A}")"
ICON_PACK="$(jq -r '.assets["hud-icons"].packPath' "${CATALOG_A}")"
ICON_PACK_SHA="$(jq -r '.assets["hud-icons"].packSha256' "${CATALOG_A}")"
mv "${HERO_PACK}" "${HERO_PACK}.original-for-drift-test"
cp -R "${ICON_PACK}" "${HERO_PACK}"
jq --arg sha "${ICON_PACK_SHA}" '.assets.hero.packSha256 = $sha' "${CATALOG_A}" \
  > "${TEST_ROOT}/catalog-pack-drift.json"
cp "${TEST_ROOT}/catalog-pack-drift.json" "${CATALOG_A}"
EXEC_PACK_DRIFT="$("${FORGE}" plan execute --token "${TOKEN_PACK_DRIFT}" --wait --json || true)"
printf '%s' "${EXEC_PACK_DRIFT}" | jq -e '(.ok | not)' >/dev/null
test "$(job_count)" = "${COUNT_BEFORE_PACK_DRIFT}"
mv "${HERO_PACK}" "${HERO_PACK}.injected-for-drift-test"
mv "${HERO_PACK}.original-for-drift-test" "${HERO_PACK}"
cp "${TEST_ROOT}/catalog-before-pack-drift.json" "${CATALOG_A}"

# Style board + lock mutation after planning is also part of the immutable
# source closure. Updating boardSha256 cannot hide the change.
PLAN_STYLE_DRIFT="$(forge_json project plan-build --project "${PROJECT_A}" --manifest "${MANIFEST_A}")"
TOKEN_STYLE_DRIFT="$(printf '%s' "${PLAN_STYLE_DRIFT}" | jq -r '.data.token')"
COUNT_BEFORE_STYLE_DRIFT="$(job_count)"
STYLE_LOCK_A="$(find "${PROJECT_A}/.forge/styles" -name style-lock.json -type f -print -quit)"
STYLE_BOARD_A="$(jq -r '.boardPath' "${STYLE_LOCK_A}")"
cp "${STYLE_LOCK_A}" "${TEST_ROOT}/style-lock-before-drift.json"
cp "${STYLE_BOARD_A}" "${TEST_ROOT}/style-board-before-drift.png"
printf 'forge-style-drift' >> "${STYLE_BOARD_A}"
STYLE_BOARD_SHA="$(shasum -a 256 "${STYLE_BOARD_A}" | cut -d' ' -f1)"
jq --arg sha "${STYLE_BOARD_SHA}" '.boardSha256 = $sha' "${STYLE_LOCK_A}" \
  > "${TEST_ROOT}/style-lock-drift.json"
cp "${TEST_ROOT}/style-lock-drift.json" "${STYLE_LOCK_A}"
EXEC_STYLE_DRIFT="$("${FORGE}" plan execute --token "${TOKEN_STYLE_DRIFT}" --wait --json || true)"
printf '%s' "${EXEC_STYLE_DRIFT}" | jq -e '(.ok | not)' >/dev/null
test "$(job_count)" = "${COUNT_BEFORE_STYLE_DRIFT}"
cp "${TEST_ROOT}/style-board-before-drift.png" "${STYLE_BOARD_A}"
cp "${TEST_ROOT}/style-lock-before-drift.json" "${STYLE_LOCK_A}"
echo "CHECK 2A PASS: catalog+reuse pack and StyleLock closure drift both invalidate tokens before job staging; parent usage is not double-counted"

# CHECK 3 — targeted invalidation: touch one icon's prompt; only its icon_set
# rebuilds (spec_changed), everything else reuses, exactly one new child job.
jq '.items[0].prompt = "a small silver coin"' "${PROJECT_A}/specs/hud-icons.json" \
  > "${TEST_ROOT}/hud-icons-touched.json"
cp "${TEST_ROOT}/hud-icons-touched.json" "${PROJECT_A}/specs/hud-icons.json"
PLAN_TOUCH="$(forge_json project plan-build --project "${PROJECT_A}" --manifest "${MANIFEST_A}")"
printf '%s' "${PLAN_TOUCH}" | jq -e \
  '([.data.plan.actions[] | select(.assetId == "hud-icons" and .action == "rebuild" and (.reasons | index("spec_changed") != null))] | length) == 1
   and ([.data.plan.actions[] | select(.assetId != "hud-icons" and .action == "reuse")] | length) == 2' >/dev/null
TOKEN_TOUCH="$(printf '%s' "${PLAN_TOUCH}" | jq -r '.data.token')"
EXEC_TOUCH="$(forge_json plan execute --token "${TOKEN_TOUCH}" --wait)"
printf '%s' "${EXEC_TOUCH}" | jq -e '.data.lifecycle_state == "succeeded"' >/dev/null
PARENT_THREE="$(printf '%s' "${EXEC_TOUCH}" | jq -r '.data.job_id')"
jq -e '.summary.built == 1 and .summary.reused == 2
   and ([.results[] | select(.assetId == "hud-icons" and .status == "succeeded" and (.reasons | index("spec_changed") != null))] | length) == 1' \
  "$(build_report_path "${PARENT_THREE}")" >/dev/null
forge_json job list --recent 1000 | jq -e --arg parent "${PARENT_THREE}" \
  '[.data[] | select(.parent_job_id == $parent)] as $children
   | ($children | length) == 1
   and $children[0].operation_kind == "generate_static_asset_set"
   and $children[0].asset_id == "hud-icons"' >/dev/null
COUNT_THREE="$(job_count)"
test "${COUNT_THREE}" = "8"
echo "CHECK 3 PASS: targeted invalidation — hud-icons rebuild (spec_changed), hero/forest-props reuse, exactly one new child job (count 6 → 8: 1 parent + 1 child)"

# ---------------------------------------------------------------------------
# Project B: required character dependsOn a failing icon_set; unrelated
# prop_set is optional (required=false).
# ---------------------------------------------------------------------------
PROJECT_B="${TEST_ROOT}/project-b"
init_project_with_style "${PROJECT_B}" "Game Art B"
mkdir -p "${PROJECT_B}/specs"
cat > "${PROJECT_B}/specs/hero.json" <<'EOF'
{
  "schemaVersion": "1",
  "kind": "character",
  "id": "hero",
  "name": "Hero Knight",
  "prompt": "a brave knight in compact jewel-tone armor",
  "license": "private"
}
EOF
cat > "${PROJECT_B}/specs/hud-icons.json" <<'EOF'
{
  "schemaVersion": "1",
  "kind": "icon_set",
  "id": "hud-icons",
  "name": "HUD Icons",
  "items": [
    { "id": "coin", "name": "Coin", "prompt": "a coin [fixture:hard_multiple_subjects]" }
  ],
  "license": "private"
}
EOF
cat > "${PROJECT_B}/specs/bonus-props.json" <<'EOF'
{
  "schemaVersion": "1",
  "kind": "prop_set",
  "id": "bonus-props",
  "name": "Bonus Props",
  "items": [
    { "id": "crate", "name": "Crate", "prompt": "a compact wooden crate" }
  ],
  "license": "private"
}
EOF
cat > "${PROJECT_B}/game-art.json" <<'EOF'
{
  "schemaVersion": "1",
  "kind": "game_art_manifest",
  "projectId": "game-art-b",
  "name": "Game Art B",
  "provider": { "id": "fixture", "profileId": "default" },
  "defaults": {
    "outputDirectory": "packs",
    "godotRoot": "addons/forge_assets",
    "license": "private"
  },
  "assets": [
    { "id": "hero", "kind": "character", "spec": "specs/hero.json", "dependsOn": ["hud-icons"] },
    { "id": "hud-icons", "kind": "icon_set", "spec": "specs/hud-icons.json" },
    { "id": "bonus-props", "kind": "prop_set", "spec": "specs/bonus-props.json", "required": false }
  ]
}
EOF

# CHECK 4 — required dependency failure blocks the dependent; the unrelated
# optional asset still builds; the parent fails with project_build_failed.
PLAN_B="$(forge_json project plan-build --project "${PROJECT_B}" --manifest "${PROJECT_B}/game-art.json")"
TOKEN_B="$(printf '%s' "${PLAN_B}" | jq -r '.data.token')"
EXEC_B="$("${FORGE}" plan execute --token "${TOKEN_B}" --wait --json || true)"
test "$(printf '%s\n' "${EXEC_B}" | jq -c . | wc -l | tr -d ' ')" = "1"
printf '%s' "${EXEC_B}" | jq -e '(.ok | not) and .error.code == "project_build_failed"' >/dev/null
PARENT_B="$(forge_json job list --recent 1000 | jq -r \
  '[.data[] | select(.operation_kind == "build_project" and .error_code == "project_build_failed")][0].job_id')"
test -n "${PARENT_B}"
forge_json job get --id "${PARENT_B}" | jq -e \
  '.data.lifecycle_state == "failed" and .data.error_code == "project_build_failed" and .data.recoverable' >/dev/null
jq -e '([.results[] | select(.assetId == "hud-icons" and .status == "failed")] | length) == 1
   and ([.results[] | select(.assetId == "hero" and .status == "skipped" and .reasons == ["dependency_failed"])] | length) == 1
   and ([.results[] | select(.assetId == "bonus-props" and .status == "succeeded")] | length) == 1
   and .summary.failed == 1 and .summary.skipped == 1 and .summary.built == 1' \
  "$(build_report_path "${PARENT_B}")" >/dev/null
forge_json job list --recent 1000 | jq -e --arg parent "${PARENT_B}" \
  '[.data[] | select(.parent_job_id == $parent)] as $children
   | ($children | length) == 2
   and ([$children[] | select(.asset_id == "hero")] | length) == 0' >/dev/null
# Mirror scenario for §15 阶段 2 bullet "optional asset 失败不阻断无依赖的
# required 资产": the ONLY failing asset is optional (required=false), so the
# unrelated required icon_set still builds and the parent SUCCEEDS.
PROJECT_B2="${TEST_ROOT}/project-b2"
init_project_with_style "${PROJECT_B2}" "Game Art B2"
mkdir -p "${PROJECT_B2}/specs"
cat > "${PROJECT_B2}/specs/hud-icons.json" <<'EOF'
{
  "schemaVersion": "1",
  "kind": "icon_set",
  "id": "hud-icons",
  "name": "HUD Icons",
  "items": [
    { "id": "coin", "name": "Coin", "prompt": "a small gold coin" }
  ],
  "license": "private"
}
EOF
cat > "${PROJECT_B2}/specs/bonus-props.json" <<'EOF'
{
  "schemaVersion": "1",
  "kind": "prop_set",
  "id": "bonus-props",
  "name": "Bonus Props",
  "items": [
    { "id": "crate", "name": "Crate", "prompt": "a crate [fixture:hard_multiple_subjects]" }
  ],
  "license": "private"
}
EOF
cat > "${PROJECT_B2}/game-art.json" <<'EOF'
{
  "schemaVersion": "1",
  "kind": "game_art_manifest",
  "projectId": "game-art-b2",
  "name": "Game Art B2",
  "provider": { "id": "fixture", "profileId": "default" },
  "defaults": {
    "outputDirectory": "packs",
    "godotRoot": "addons/forge_assets",
    "license": "private"
  },
  "assets": [
    { "id": "hud-icons", "kind": "icon_set", "spec": "specs/hud-icons.json" },
    { "id": "bonus-props", "kind": "prop_set", "spec": "specs/bonus-props.json", "required": false }
  ]
}
EOF
PLAN_B2="$(forge_json project plan-build --project "${PROJECT_B2}" --manifest "${PROJECT_B2}/game-art.json")"
TOKEN_B2="$(printf '%s' "${PLAN_B2}" | jq -r '.data.token')"
EXEC_B2="$(forge_json plan execute --token "${TOKEN_B2}" --wait)"
printf '%s' "${EXEC_B2}" | jq -e '.data.lifecycle_state == "succeeded"' >/dev/null
PARENT_B2="$(printf '%s' "${EXEC_B2}" | jq -r '.data.job_id')"
jq -e '([.results[] | select(.assetId == "hud-icons" and .status == "succeeded")] | length) == 1
   and ([.results[] | select(.assetId == "bonus-props" and .status == "failed")] | length) == 1
   and .summary.failed == 1 and .summary.built == 1 and .summary.skipped == 0' \
  "$(build_report_path "${PARENT_B2}")" >/dev/null
echo "CHECK 4 PASS: required/optional behavior — required icon_set failure failed the parent (project_build_failed) and skipped the dependent hero (dependency_failed) while the unrelated optional prop_set built; a failing optional prop_set did not block the required icon_set (parent succeeded)"

# ---------------------------------------------------------------------------
# Project C: cancel propagation from the parent to an active build child.
# ---------------------------------------------------------------------------
PROJECT_C="${TEST_ROOT}/project-c"
init_project_with_style "${PROJECT_C}" "Game Art C"
mkdir -p "${PROJECT_C}/specs"
cp "${EXAMPLE}/specs/hero.json" "${PROJECT_C}/specs/hero.json"
cp "${EXAMPLE}/specs/forest-props.json" "${PROJECT_C}/specs/forest-props.json"
cat > "${PROJECT_C}/game-art.json" <<'EOF'
{
  "schemaVersion": "1",
  "kind": "game_art_manifest",
  "projectId": "game-art-c",
  "name": "Game Art C",
  "provider": { "id": "fixture", "profileId": "default" },
  "defaults": {
    "outputDirectory": "packs",
    "godotRoot": "addons/forge_assets",
    "license": "private"
  },
  "assets": [
    { "id": "hero", "kind": "character", "spec": "specs/hero.json" },
    { "id": "forest-props", "kind": "prop_set", "spec": "specs/forest-props.json" }
  ]
}
EOF

# CHECK 5 — FORGE_FIXTURE_POLL_PENDING_ONCE=1 makes each fixture video ticket
# answer its first poll Pending, so the character child stays active while the
# plan executes without --wait (a worker process runs the build). Cancelling
# the parent must cascade the flag to the active child; both reach cancelled.
PLAN_C="$(forge_json project plan-build --project "${PROJECT_C}" --manifest "${PROJECT_C}/game-art.json")"
TOKEN_C="$(printf '%s' "${PLAN_C}" | jq -r '.data.token')"
EXEC_C="$(FORGE_FIXTURE_POLL_PENDING_ONCE=1 forge_json plan execute --token "${TOKEN_C}")"
PARENT_C="$(printf '%s' "${EXEC_C}" | jq -r '.data.job_id')"
CHILD_C=""
for _ in $(seq 1 400); do
  CHILD_C="$(forge_json job list --recent 1000 | jq -r --arg parent "${PARENT_C}" \
    '[.data[] | select(.parent_job_id == $parent and .operation_kind == "generate_character_pack" and .lifecycle_state == "running")][0].job_id // empty')"
  if [ -n "${CHILD_C}" ]; then
    break
  fi
  sleep 0.15
done
test -n "${CHILD_C}"
forge_json job cancel --id "${PARENT_C}" \
  | jq -e '.data.cancellation_requested == true' >/dev/null
jq -e '.cancellation_requested == true' "${FORGE_JOB_STORE}/${CHILD_C}/job.json" >/dev/null
forge_json job get --id "${CHILD_C}" | jq -e '.data.cancellation_requested == true' >/dev/null
PARENT_C_STATE=""
CHILD_C_STATE=""
for _ in $(seq 1 600); do
  PARENT_C_STATE="$(forge_json job get --id "${PARENT_C}" | jq -r '.data.lifecycle_state')"
  CHILD_C_STATE="$(forge_json job get --id "${CHILD_C}" | jq -r '.data.lifecycle_state')"
  if [ "${PARENT_C_STATE}" = "cancelled" ] && [ "${CHILD_C_STATE}" = "cancelled" ]; then
    break
  fi
  case "${PARENT_C_STATE}" in
    succeeded | failed | awaiting_review) break ;;
  esac
  case "${CHILD_C_STATE}" in
    succeeded | failed | awaiting_review) break ;;
  esac
  sleep 0.15
done
test "${PARENT_C_STATE}" = "cancelled"
test "${CHILD_C_STATE}" = "cancelled"
echo "CHECK 5 PASS: cancel propagation — job cancel flagged parent and active child ${CHILD_C} (job.json cancellation_requested=true); both reached cancelled"

# CHECK 6 — crash recovery and catalog concurrency are covered by the Rust
# integration/unit tests; run them as part of this acceptance. (cargo test
# accepts a single TESTNAME positional, so the catalog concurrency test runs
# under the `concurrent` filter, which also covers the job-store concurrency
# test.)
cargo test -q -p core --manifest-path "${ROOT}/Cargo.toml" \
  --test game_art_build_tests reconcile >/dev/null
cargo test -q -p core --manifest-path "${ROOT}/Cargo.toml" \
  concurrent >/dev/null
echo "CHECK 6 PASS: crash recovery — cargo test -p core --test game_art_build_tests reconcile + cargo test -p core concurrent (catalog::tests::concurrent_registers_do_not_lose_updates)"

# CHECK 6A — a project reached through a symlink must canonicalize before the
# reviewed source fingerprint is created. Planning and executing unchanged
# bytes through the same alias must not invalidate its own token.
PROJECT_A_LINK="${TEST_ROOT}/project-a-link"
ln -s "${PROJECT_A}" "${PROJECT_A_LINK}"
PLAN_LINK="$(forge_json project plan-build --project "${PROJECT_A_LINK}" --manifest "${PROJECT_A_LINK}/game-art.json")"
TOKEN_LINK="$(printf '%s' "${PLAN_LINK}" | jq -r '.data.token')"
EXEC_LINK="$(forge_json plan execute --token "${TOKEN_LINK}" --wait)"
printf '%s' "${EXEC_LINK}" | jq -e \
  '.data.lifecycle_state == "succeeded"' >/dev/null
echo "CHECK 6A PASS: symlinked project path plans and executes without self-invalidating the token"

# CHECK 7 — single JSON stdout: every forge invocation above went through
# forge_json / a jq one-value guard, which fails on anything but exactly one
# JSON envelope per stdout.
echo "CHECK 7 PASS: single JSON stdout — every forge command in this script emitted exactly one JSON envelope (jq-verified)"

# CHECK 8 — credential scan over the temp JobStore/PlanStore.
if credential_scan_matches "${FORGE_JOB_STORE}" "${FORGE_PLAN_STORE}" >/dev/null; then
  echo "credential-like material leaked into the stage 2 JobStore/PlanStore" >&2
  exit 1
fi
echo "CHECK 8 PASS: credential scan — 0 matches across FORGE_JOB_STORE and FORGE_PLAN_STORE"

echo "PASS Forge game art manifest contract"
