#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/forge-notary-preflight-test.XXXXXX")"
FAKE_BIN="${TMP_DIR}/bin"
DMG_DIR="${TMP_DIR}/path with dollar \$sign"
DMG="${DMG_DIR}/Game Sprite Forge_0.1.0_aarch64.dmg"
MOUNTED_APP=""

cleanup() {
  rm -rf "${TMP_DIR}"
}

trap cleanup EXIT

mkdir -p "${FAKE_BIN}"
mkdir -p "${DMG_DIR}"
printf 'fake dmg\n' > "${DMG}"
EXPECTED_SHA="$(shasum -a 256 "${DMG}" | awk '{print $1}')"

cat > "${FAKE_BIN}/hdiutil" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  verify)
    echo "hdiutil: verify: checksum of \"$2\" is VALID"
    ;;
  attach)
    mount_path=""
    while [[ $# -gt 0 ]]; do
      case "$1" in
        -mountpoint)
          mount_path="$2"
          shift 2
          ;;
        *)
          shift
          ;;
      esac
    done
    mkdir -p "${mount_path}/Game Sprite Forge.app"
    echo "/dev/disk-test ${mount_path}"
    ;;
  detach)
    if [[ -n "${2:-}" && -d "${2}/Game Sprite Forge.app" ]]; then
      rmdir "${2}/Game Sprite Forge.app"
    fi
    exit 0
    ;;
  *)
    echo "unexpected hdiutil args: $*" >&2
    exit 2
    ;;
esac
STUB

cat > "${FAKE_BIN}/codesign" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "--verify" ]]; then
  last=""
  for arg in "$@"; do
    last="${arg}"
  done
  echo "${last}: valid on disk"
  exit 0
fi
if [[ "${1:-}" == "-dvvv" ]]; then
  cat >&2 <<'DETAILS'
Authority=Developer ID Application: Ka Yan (J6P96F432P)
Authority=Developer ID Certification Authority
TeamIdentifier=J6P96F432P
Timestamp=Jun 5, 2026 at 08:54:30
Runtime Version=26.5.0
DETAILS
  exit 0
fi
echo "unexpected codesign args: $*" >&2
exit 2
STUB

cat > "${FAKE_BIN}/xcrun" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "-f" && "${2:-}" == "notarytool" ]]; then
  echo "/fake/notarytool"
  exit 0
fi
if [[ "${1:-}" == "-f" && "${2:-}" == "stapler" ]]; then
  echo "/fake/stapler"
  exit 0
fi
if [[ "${1:-}" == "stapler" && "${2:-}" == "validate" ]]; then
  last=""
  for arg in "$@"; do
    last="${arg}"
  done
  echo "Processing: ${last}"
  echo "Game Sprite Forge_0.1.0_aarch64.dmg does not have a ticket stapled to it."
  exit 65
fi
echo "unexpected xcrun args: $*" >&2
exit 2
STUB

cat > "${FAKE_BIN}/spctl" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
last=""
for arg in "$@"; do
  last="${arg}"
done
echo "${last}: rejected"
echo "source=Unnotarized Developer ID"
exit 3
STUB

chmod +x "${FAKE_BIN}/hdiutil" "${FAKE_BIN}/codesign" "${FAKE_BIN}/xcrun" "${FAKE_BIN}/spctl"

shell_quote_for_test() {
  printf "'%s'" "$(printf '%s' "$1" | sed "s/'/'\\\\''/g")"
}

run_preflight() {
  PATH="${FAKE_BIN}:$PATH" bash "${ROOT}/scripts/notarization-preflight.sh" "${DMG}" "${1:-${EXPECTED_SHA}}"
}

assert_contains() {
  local needle="$1"
  if [[ "${OUTPUT}" != *"${needle}"* ]]; then
    echo "Expected output to contain: ${needle}" >&2
    echo "--- output ---" >&2
    echo "${OUTPUT}" >&2
    exit 1
  fi
}

assert_not_contains() {
  local needle="$1"
  if [[ "${OUTPUT}" == *"${needle}"* ]]; then
    echo "Expected output not to contain: ${needle}" >&2
    echo "--- output ---" >&2
    echo "${OUTPUT}" >&2
    exit 1
  fi
}

KEYCHAIN_PROFILE="GameSpriteForgeNotary \$profile"
OUTPUT="$(
  PATH="${FAKE_BIN}:$PATH" \
  NOTARYTOOL_KEYCHAIN_PROFILE="${KEYCHAIN_PROFILE}" \
  APPLE_APP_SPECIFIC_PASSWORD="do-not-print-this-secret" \
  bash "${ROOT}/scripts/notarization-preflight.sh" "${DMG}" "${EXPECTED_SHA}"
)"

assert_contains "notarytool: /fake/notarytool"
assert_contains "stapler: /fake/stapler"
assert_contains "Credential method: keychain profile"
assert_contains "Keychain profile availability: not checked by this preflight"
assert_contains "DMG SHA-256: ${EXPECTED_SHA}"
assert_contains "DMG signature: pass"
assert_contains "DMG timestamp: pass"
assert_contains "Mounted app signature: pass"
assert_contains "Mounted app timestamp: pass"
assert_contains "Mounted app hardened runtime: pass"
assert_contains "Stapled ticket: missing"
assert_contains "xcrun notarytool submit $(shell_quote_for_test "${DMG}") --keychain-profile $(shell_quote_for_test "${KEYCHAIN_PROFILE}") --wait"
assert_contains "xcrun stapler staple -v $(shell_quote_for_test "${DMG}")"
assert_contains "scripts/verify-release-package.sh $(shell_quote_for_test "${DMG}") $(shell_quote_for_test "${EXPECTED_SHA}")"
assert_not_contains "do-not-print-this-secret"

CLI_PROFILE="GameSpriteForge CLI \$profile"
OUTPUT="$(
  PATH="${FAKE_BIN}:$PATH" \
  bash "${ROOT}/scripts/notarization-preflight.sh" --keychain-profile "${CLI_PROFILE}" "${DMG}" "${EXPECTED_SHA}"
)"
assert_contains "Credential method: keychain profile"
assert_contains "Keychain profile: ${CLI_PROFILE}"
assert_contains "Keychain profile availability: not checked by this preflight"
assert_contains "xcrun notarytool submit $(shell_quote_for_test "${DMG}") --keychain-profile $(shell_quote_for_test "${CLI_PROFILE}") --wait"

OUTPUT="$(
  PATH="${FAKE_BIN}:$PATH" \
  NOTARYTOOL_API_KEY_PATH="/private/AuthKey_SECRET.p8" \
  NOTARYTOOL_API_KEY_ID="SECRETKEYID" \
  NOTARYTOOL_API_ISSUER_ID="SECRETISSUER" \
  bash "${ROOT}/scripts/notarization-preflight.sh" "${DMG}" "${EXPECTED_SHA}"
)"
assert_contains "Credential method: App Store Connect API key"
assert_contains "--key \"\$NOTARYTOOL_API_KEY_PATH\""
assert_contains "--key-id \"\$NOTARYTOOL_API_KEY_ID\""
assert_contains "--issuer \"\$NOTARYTOOL_API_ISSUER_ID\""
assert_not_contains "/private/AuthKey_SECRET.p8"
assert_not_contains "SECRETKEYID"
assert_not_contains "SECRETISSUER"

OUTPUT="$(PATH="${FAKE_BIN}:$PATH" bash "${ROOT}/scripts/notarization-preflight.sh" "${DMG}" "${EXPECTED_SHA}")"
assert_contains "Credential method: not configured"
assert_contains "xcrun notarytool store-credentials 'GameSpriteForgeNotary'"

set +e
MISMATCH_OUTPUT="$(run_preflight "bad-sha" 2>&1)"
MISMATCH_STATUS=$?
set -e
if [[ "${MISMATCH_STATUS}" -ne 65 ]]; then
  echo "Expected SHA mismatch status 65, got ${MISMATCH_STATUS}" >&2
  echo "${MISMATCH_OUTPUT}" >&2
  exit 1
fi
if [[ "${MISMATCH_OUTPUT}" != *"FAIL SHA-256 mismatch"* ]]; then
  echo "Expected SHA mismatch failure text" >&2
  echo "${MISMATCH_OUTPUT}" >&2
  exit 1
fi

NEWLINE_DMG="${DMG_DIR}/bad"$'\n'"path.dmg"
printf 'fake dmg\n' > "${NEWLINE_DMG}"
set +e
NEWLINE_OUTPUT="$(PATH="${FAKE_BIN}:$PATH" bash "${ROOT}/scripts/notarization-preflight.sh" "${NEWLINE_DMG}" 2>&1)"
NEWLINE_STATUS=$?
set -e
if [[ "${NEWLINE_STATUS}" -ne 65 || "${NEWLINE_OUTPUT}" != *"FAIL DMG path cannot contain a newline"* ]]; then
  echo "Expected newline DMG path rejection" >&2
  echo "status=${NEWLINE_STATUS}" >&2
  echo "${NEWLINE_OUTPUT}" >&2
  exit 1
fi

set +e
PROFILE_NEWLINE_OUTPUT="$(
  PATH="${FAKE_BIN}:$PATH" \
  NOTARYTOOL_KEYCHAIN_PROFILE=$'bad\nprofile' \
  bash "${ROOT}/scripts/notarization-preflight.sh" "${DMG}" "${EXPECTED_SHA}" 2>&1
)"
PROFILE_NEWLINE_STATUS=$?
set -e
if [[ "${PROFILE_NEWLINE_STATUS}" -ne 65 || "${PROFILE_NEWLINE_OUTPUT}" != *"FAIL keychain profile cannot contain a newline"* ]]; then
  echo "Expected newline keychain profile rejection" >&2
  echo "status=${PROFILE_NEWLINE_STATUS}" >&2
  echo "${PROFILE_NEWLINE_OUTPUT}" >&2
  exit 1
fi

set +e
CLI_PROFILE_NEWLINE_OUTPUT="$(
  PATH="${FAKE_BIN}:$PATH" \
  bash "${ROOT}/scripts/notarization-preflight.sh" --keychain-profile $'bad\nprofile' "${DMG}" "${EXPECTED_SHA}" 2>&1
)"
CLI_PROFILE_NEWLINE_STATUS=$?
set -e
if [[ "${CLI_PROFILE_NEWLINE_STATUS}" -ne 65 || "${CLI_PROFILE_NEWLINE_OUTPUT}" != *"FAIL keychain profile cannot contain a newline"* ]]; then
  echo "Expected newline CLI keychain profile rejection" >&2
  echo "status=${CLI_PROFILE_NEWLINE_STATUS}" >&2
  echo "${CLI_PROFILE_NEWLINE_OUTPUT}" >&2
  exit 1
fi

set +e
UNKNOWN_OPTION_OUTPUT="$(PATH="${FAKE_BIN}:$PATH" bash "${ROOT}/scripts/notarization-preflight.sh" --unknown "${DMG}" 2>&1)"
UNKNOWN_OPTION_STATUS=$?
set -e
if [[ "${UNKNOWN_OPTION_STATUS}" -ne 64 || "${UNKNOWN_OPTION_OUTPUT}" != *"FAIL unknown option: --unknown"* ]]; then
  echo "Expected unknown option rejection" >&2
  echo "status=${UNKNOWN_OPTION_STATUS}" >&2
  echo "${UNKNOWN_OPTION_OUTPUT}" >&2
  exit 1
fi

echo "PASS notarization preflight test"
