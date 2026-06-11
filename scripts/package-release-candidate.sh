#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="0.1.0"
ARCH="aarch64"
DMG="${ROOT}/target/release/bundle/dmg/Game Sprite Forge_${VERSION}_${ARCH}.dmg"
VERIFY_SCRIPT="${ROOT}/scripts/verify-release-package.sh"
VERIFY_DOC="${ROOT}/docs/architecture/release-candidate-verification.md"
OUT_ROOT="${ROOT}/release-candidates"

require_command() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "Missing required command: $1" >&2
    exit 69
  }
}

require_command shasum
require_command xcrun
require_command ditto

if [[ ! -f "${DMG}" ]]; then
  echo "Missing DMG: ${DMG}" >&2
  exit 66
fi
if [[ ! -x "${VERIFY_SCRIPT}" ]]; then
  echo "Missing executable verification script: ${VERIFY_SCRIPT}" >&2
  exit 66
fi
if [[ ! -f "${VERIFY_DOC}" ]]; then
  echo "Missing verification doc: ${VERIFY_DOC}" >&2
  exit 66
fi

ACTUAL_SHA="$(shasum -a 256 "${DMG}" | awk '{print $1}')"
SHA_PREFIX="${ACTUAL_SHA:0:12}"
NAME="GameSpriteForge-${VERSION}-${ARCH}-${SHA_PREFIX}-notarized"
OUT_DIR="${OUT_ROOT}/${NAME}"
ZIP_PATH="${OUT_ROOT}/${NAME}.zip"

if ! xcrun stapler validate -v "${DMG}" >/dev/null 2>&1; then
  echo "DMG is not notarized/stapled; run Task 7 notarization and stapling before packaging." >&2
  echo "Current DMG SHA: ${ACTUAL_SHA}" >&2
  echo "No release-candidates package was created for this SHA; older release-candidates may belong to a previous build." >&2
  exit 65
fi

rm -rf "${OUT_DIR}" "${ZIP_PATH}"
mkdir -p "${OUT_DIR}/scripts" "${OUT_DIR}/docs"

cp "${DMG}" "${OUT_DIR}/"
cp "${VERIFY_SCRIPT}" "${OUT_DIR}/scripts/"
cp "${VERIFY_DOC}" "${OUT_DIR}/docs/"
cat >"${OUT_DIR}/README.md" <<README
# Game Sprite Forge ${VERSION} ${ARCH} Release Candidate

This package contains the notarized macOS DMG and release verification material.
Package name includes the first 12 characters of the DMG SHA-256 to avoid
confusing this candidate with older packages.

## Files

\`\`\`text
Game Sprite Forge_${VERSION}_${ARCH}.dmg
scripts/verify-release-package.sh
docs/release-candidate-verification.md
SHA256SUMS
\`\`\`

## Verify The Release Package

DMG SHA-256:

\`\`\`text
${ACTUAL_SHA}
\`\`\`

\`\`\`bash
chmod +x scripts/verify-release-package.sh
scripts/verify-release-package.sh "./Game Sprite Forge_${VERSION}_${ARCH}.dmg" \\
  ${ACTUAL_SHA}
\`\`\`

Then complete the manual checklist in \`docs/release-candidate-verification.md\`.
README

(
  cd "${OUT_DIR}"
  shasum -a 256 "Game Sprite Forge_${VERSION}_${ARCH}.dmg" \
    scripts/verify-release-package.sh \
    docs/release-candidate-verification.md \
    README.md > SHA256SUMS
)

(
  cd "${OUT_ROOT}"
  ditto -c -k --sequesterRsrc --keepParent "${NAME}" "${ZIP_PATH}"
)

ZIP_SHA="$(shasum -a 256 "${ZIP_PATH}" | awk '{print $1}')"

cat <<SUMMARY
Release candidate package created:
  Directory: ${OUT_DIR}
  Zip:       ${ZIP_PATH}
  DMG SHA:   ${ACTUAL_SHA}
  Zip SHA:   ${ZIP_SHA}
SUMMARY
