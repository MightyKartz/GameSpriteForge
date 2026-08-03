#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "${ROOT}/scripts/macos-cli-signing.env"

TARGET="${1:-${ROOT}/target/debug/forge}"
if [[ ! -x "${TARGET}" ]]; then
  cargo build -p forge-cli --manifest-path "${ROOT}/Cargo.toml"
fi

IDENTITY="${FORGE_DEV_CODESIGN_IDENTITY:-}"
if [[ -z "${IDENTITY}" ]]; then
  IDENTITY="$(security find-identity -v -p codesigning \
    | sed -n "s/.*\"\(.*(${FORGE_APPLE_TEAM_ID})\)\".*/\1/p" \
    | head -n 1)"
fi

if [[ -z "${IDENTITY}" || "${IDENTITY}" == "-" ]]; then
  echo "No persistent Apple code-signing identity for team ${FORGE_APPLE_TEAM_ID}." >&2
  echo "Set FORGE_DEV_CODESIGN_IDENTITY to an Apple Development or Developer ID identity." >&2
  echo "Ad-hoc signing is intentionally rejected because rebuilds trigger repeated Keychain approval." >&2
  exit 1
fi

codesign --force \
  --identifier "${FORGE_CLI_CODESIGN_IDENTIFIER}" \
  --options runtime \
  --timestamp=none \
  --sign "${IDENTITY}" \
  "${TARGET}"
codesign --verify --strict --verbose=2 "${TARGET}"

SIGNATURE="$(codesign -dvvv "${TARGET}" 2>&1)"
printf '%s\n' "${SIGNATURE}" | grep -F "Identifier=${FORGE_CLI_CODESIGN_IDENTIFIER}"
printf '%s\n' "${SIGNATURE}" | grep -F "TeamIdentifier=${FORGE_APPLE_TEAM_ID}"
printf 'Signed development CLI: %s\n' "${TARGET}"
printf 'Re-run this script after rebuilding forge.\n'
