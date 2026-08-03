#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "${ROOT}/scripts/macos-cli-signing.env"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/forge-cli-signing.XXXXXX")"
trap 'rm -rf "${TEST_ROOT}"' EXIT

grep -F 'appleSigned":false' "${ROOT}/.github/workflows/release-cli.yml" >/dev/null
grep -F 'shasum -a 256 -c MANIFEST.sha256' "${ROOT}/.github/workflows/release-cli.yml" >/dev/null
grep -F 'release payload manifest verification failed' "${ROOT}/install.sh" >/dev/null
if rg -n 'APPLE_DEVELOPER|APPLE_NOTARY|notarytool submit' \
  "${ROOT}/.github/workflows/release-cli.yml" >/dev/null; then
  echo "unsigned CLI release workflow unexpectedly depends on Apple secrets" >&2
  exit 1
fi
grep -F "FORGE_CLI_CODESIGN_IDENTIFIER=\"${FORGE_CLI_CODESIGN_IDENTIFIER}\"" "${ROOT}/install.sh" >/dev/null
grep -F "FORGE_APPLE_TEAM_ID=\"${FORGE_APPLE_TEAM_ID}\"" "${ROOT}/install.sh" >/dev/null
grep -F -- '--timestamp=none' "${ROOT}/scripts/sign-dev-cli.sh" >/dev/null
grep -F 'Ad-hoc signing is intentionally rejected' "${ROOT}/scripts/sign-dev-cli.sh" >/dev/null

verify_identifier() {
  local name="$1"
  local expected="$2"
  local target="${TEST_ROOT}/${name}"
  cp /usr/bin/true "${target}"
  codesign --force --identifier "${expected}" --sign - "${target}" >/dev/null
  codesign --verify --strict "${target}"
  codesign -dvvv "${target}" 2>&1 | grep -F "Identifier=${expected}" >/dev/null
}

verify_identifier forge "${FORGE_CLI_CODESIGN_IDENTIFIER}"
verify_identifier ffmpeg "${FORGE_FFMPEG_CODESIGN_IDENTIFIER}"
verify_identifier ffprobe "${FORGE_FFPROBE_CODESIGN_IDENTIFIER}"

echo "PASS Forge CLI unsigned release and optional signing contract"
