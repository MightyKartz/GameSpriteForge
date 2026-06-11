#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EVIDENCE_DOC="${ROOT}/docs/qa/forge-pre-manual-pipeline-evidence-2026-06-05.md"
FIXTURE_ROOT="${ROOT}/examples/inputs/manual-qa"
TEST_FILTER="manual_qa_"

require_command() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "FAIL missing required command: $1" >&2
    exit 69
  }
}

relative_hash_paths() {
  awk -v root="${ROOT}/" '{
    hash=$1
    $1=""
    sub(/^[[:space:]]+/, "")
    path=$0
    sub(root, "", path)
    print hash "  " path
  }'
}

require_command cargo
require_command npm
require_command shasum
require_command awk
require_command sed

cd "${ROOT}"

echo "== Preparing manual QA fixtures =="
npm run qa:fixtures

echo "== Running pre-manual pipeline tests =="
TEST_OUTPUT="$(cargo test --manifest-path Cargo.toml --test sample_pipeline_tests "${TEST_FILTER}" -- --nocapture 2>&1)"
printf '%s\n' "${TEST_OUTPUT}"

expected_tests=(
  "manual_qa_png_sequence_exports_schema_valid_reimportable_pack"
  "manual_qa_sprite_sheet_exports_schema_valid_reimportable_pack"
)

RESULT_LINES=""
for test_name in "${expected_tests[@]}"; do
  if ! printf '%s\n' "${TEST_OUTPUT}" | grep -Fq "test ${test_name} ... ok"; then
    echo "FAIL expected test did not pass or was not run: ${test_name}" >&2
    exit 65
  fi
  RESULT_LINES+="${test_name}: pass"$'\n'
done
RESULT_LINES="$(printf '%s' "${RESULT_LINES}" | sed '/^$/d')"

PNG_HASH="$(find "${FIXTURE_ROOT}/png-sequence" -type f -name '*.png' -print0 | sort -z | xargs -0 shasum -a 256 | relative_hash_paths)"
SHEET_HASH="$(shasum -a 256 "${FIXTURE_ROOT}/sprite-sheet/forge-walk-sheet.png" | relative_hash_paths)"
MANIFEST_HASH="$(shasum -a 256 "${FIXTURE_ROOT}/qa-fixtures.json" | relative_hash_paths)"

cat >"${EVIDENCE_DOC}" <<EVIDENCE
# Forge Pre-Manual Pipeline Evidence

Recorded on 2026-06-05.

This evidence proves the local Rust processing/export/import pipeline for deterministic manual-QA fixtures before the interactive app QA session. It does not prove the Task 6 manual app QA gate, and it does not replace the required tester-selected real short video.

The sprite sheet fixture path uses the shared \`forge_core::video::slice_sprite_sheet_grid\` implementation that the Tauri command calls for app sprite-sheet slicing.

## Commands

\`\`\`bash
npm run qa:fixtures
cargo test --manifest-path Cargo.toml --test sample_pipeline_tests "${TEST_FILTER}" -- --nocapture
\`\`\`

## Results

\`\`\`text
${RESULT_LINES%$'\n'}
\`\`\`

## Fixture Baseline

Fixture root:

\`\`\`text
examples/inputs/manual-qa
\`\`\`

PNG sequence hashes:

\`\`\`text
${PNG_HASH}
\`\`\`

Sprite sheet hash:

\`\`\`text
${SHEET_HASH}
\`\`\`

Fixture manifest hash:

\`\`\`text
${MANIFEST_HASH}
\`\`\`

## Remaining Manual Gate

\`\`\`text
Bundled sample video app QA: not recorded here
Real short video app QA: not recorded here
PNG sequence app QA: not recorded here
Sprite sheet app QA: not recorded here
Exported .gsfpack app re-import QA: not recorded here
Release decision: hold until interactive app QA and notarization gates pass
\`\`\`
EVIDENCE

echo "Pre-manual pipeline evidence written to ${EVIDENCE_DOC}"
