#!/usr/bin/env bash
set -euo pipefail

DEFAULT_DMG_PATH="target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
DEFAULT_KEYCHAIN_PROFILE="GameSpriteForgeNotary"
APP_NAME="Game Sprite Forge.app"
MOUNT_PATH=""

usage() {
  cat >&2 <<'USAGE'
Usage: scripts/notarization-preflight.sh [--keychain-profile NAME] [/path/to/Game Sprite Forge_0.1.0_aarch64.dmg] [expected_sha256]

Checks local notarization readiness without submitting to Apple and without
printing credential secrets.
USAGE
}

KEYCHAIN_PROFILE_ARG=""
POSITIONAL_ARGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 64
      ;;
    --keychain-profile)
      if [[ -z "${2:-}" ]]; then
        echo "FAIL --keychain-profile requires a value" >&2
        exit 64
      fi
      KEYCHAIN_PROFILE_ARG="$2"
      shift 2
      ;;
    --keychain-profile=*)
      KEYCHAIN_PROFILE_ARG="${1#--keychain-profile=}"
      shift
      ;;
    --)
      shift
      while [[ $# -gt 0 ]]; do
        POSITIONAL_ARGS+=("$1")
        shift
      done
      ;;
    -*)
      echo "FAIL unknown option: $1" >&2
      usage
      exit 64
      ;;
    *)
      POSITIONAL_ARGS+=("$1")
      shift
      ;;
  esac
done

if [[ "${#POSITIONAL_ARGS[@]}" -gt 2 ]]; then
  usage
  exit 64
fi

DMG_PATH="${POSITIONAL_ARGS[0]:-${DEFAULT_DMG_PATH}}"
EXPECTED_SHA="${POSITIONAL_ARGS[1]:-}"

cleanup() {
  if [[ -n "${MOUNT_PATH}" ]]; then
    hdiutil detach "${MOUNT_PATH}" >/dev/null 2>&1 || true
    rmdir "${MOUNT_PATH}" >/dev/null 2>&1 || true
  fi
}

trap cleanup EXIT

require_command() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "FAIL missing required command: $1" >&2
    exit 69
  }
}

raise_open_file_limit() {
  local desired=65536
  ulimit -n "${desired}" >/dev/null 2>&1 || true
}

run_spctl() {
  local output=""
  local status=0
  local attempt
  local limit

  for attempt in 1 2 3 4 5; do
    for limit in 4096 8192 16384 32768 65536; do
      if output="$( (ulimit -n "${limit}" >/dev/null 2>&1 || true; spctl "$@") 2>&1 )"; then
        status=0
      else
        status=$?
      fi
      printf '%s\n' "${output}"
      if [[ "${output}" == *"Too many open files"* || "${output}" == *"cannot access a database"* ]]; then
        status=1
      fi
      if [[ "${status}" -eq 0 ]]; then
        return 0
      fi
      if [[ "${output}" != *"Too many open files"* && "${output}" != *"cannot access a database"* ]]; then
        return "${status}"
      fi
    done
    if [[ "${attempt}" -lt 5 ]]; then
      sleep "${attempt}"
    fi
  done
  return "${status}"
}

shell_quote() {
  local value="$1"
  printf "'%s'" "$(printf '%s' "${value}" | sed "s/'/'\\\\''/g")"
}

env_ref() {
  printf '"$%s"' "$1"
}

require_no_newline_arg() {
  local label="$1"
  local value="$2"
  if [[ "${value}" == *$'\n'* ]]; then
    echo "FAIL ${label} cannot contain a newline" >&2
    exit 65
  fi
}

print_relevant_codesign_lines() {
  awk '/Authority=Developer ID Application|TeamIdentifier=|Timestamp=|Runtime Version=|Format=|Identifier=/ { print }'
}

require_signature_line() {
  local details="$1"
  local pattern="$2"
  local pass_message="$3"
  local fail_message="$4"

  if printf '%s\n' "${details}" | grep -q "${pattern}"; then
    echo "${pass_message}"
  else
    echo "FAIL ${fail_message}" >&2
    exit 65
  fi
}

detect_credential_method() {
  KEYCHAIN_PROFILE="${KEYCHAIN_PROFILE_ARG:-${FORGE_NOTARY_KEYCHAIN_PROFILE:-${NOTARYTOOL_KEYCHAIN_PROFILE:-}}}"
  API_KEY_PATH_VAR=""
  API_KEY_ID_VAR=""
  API_ISSUER_ID_VAR=""

  if [[ -n "${KEYCHAIN_PROFILE}" ]]; then
    CREDENTIAL_METHOD="keychain profile"
  elif [[ -n "${APP_STORE_CONNECT_API_KEY_PATH:-}" && -n "${APP_STORE_CONNECT_KEY_ID:-}" && -n "${APP_STORE_CONNECT_ISSUER_ID:-}" ]]; then
    API_KEY_PATH_VAR="APP_STORE_CONNECT_API_KEY_PATH"
    API_KEY_ID_VAR="APP_STORE_CONNECT_KEY_ID"
    API_ISSUER_ID_VAR="APP_STORE_CONNECT_ISSUER_ID"
    CREDENTIAL_METHOD="App Store Connect API key"
  elif [[ -n "${NOTARYTOOL_API_KEY_PATH:-}" && -n "${NOTARYTOOL_API_KEY_ID:-}" && -n "${NOTARYTOOL_API_ISSUER_ID:-}" ]]; then
    API_KEY_PATH_VAR="NOTARYTOOL_API_KEY_PATH"
    API_KEY_ID_VAR="NOTARYTOOL_API_KEY_ID"
    API_ISSUER_ID_VAR="NOTARYTOOL_API_ISSUER_ID"
    CREDENTIAL_METHOD="App Store Connect API key"
  else
    KEYCHAIN_PROFILE="${DEFAULT_KEYCHAIN_PROFILE}"
    CREDENTIAL_METHOD="not configured"
  fi
}

require_command shasum
require_command hdiutil
require_command codesign
require_command spctl
require_command xcrun
require_command mktemp
require_command awk
require_command sed
require_command grep
require_command tail
raise_open_file_limit

require_no_newline_arg "DMG path" "${DMG_PATH}"

if [[ ! -f "${DMG_PATH}" ]]; then
  echo "FAIL DMG not found: ${DMG_PATH}" >&2
  exit 66
fi

NOTARYTOOL_PATH="$(xcrun -f notarytool 2>/dev/null || true)"
STAPLER_PATH="$(xcrun -f stapler 2>/dev/null || true)"
if [[ -z "${NOTARYTOOL_PATH}" ]]; then
  echo "FAIL xcrun could not locate notarytool" >&2
  exit 69
fi
if [[ -z "${STAPLER_PATH}" ]]; then
  echo "FAIL xcrun could not locate stapler" >&2
  exit 69
fi

detect_credential_method
require_no_newline_arg "keychain profile" "${KEYCHAIN_PROFILE}"

echo "== Game Sprite Forge Notarization Preflight =="
echo "DMG: ${DMG_PATH}"

echo "== Tool Availability =="
echo "xcrun: $(command -v xcrun)"
echo "notarytool: ${NOTARYTOOL_PATH}"
echo "stapler: ${STAPLER_PATH}"
echo "hdiutil: $(command -v hdiutil)"
echo "codesign: $(command -v codesign)"
echo "spctl: $(command -v spctl)"

echo "== Credential Method =="
echo "Credential method: ${CREDENTIAL_METHOD}"
if [[ "${CREDENTIAL_METHOD}" == "keychain profile" ]]; then
  echo "Keychain profile: ${KEYCHAIN_PROFILE}"
  echo "Keychain profile availability: not checked by this preflight; run notarytool history after store-credentials to verify it exists."
elif [[ "${CREDENTIAL_METHOD}" == "App Store Connect API key" ]]; then
  echo "API key environment: present"
  echo "API key path, key id, issuer id, and private key contents are not printed."
else
  echo "No credential method selected in environment."
  echo "Set NOTARYTOOL_KEYCHAIN_PROFILE for Method A, or set APP_STORE_CONNECT_API_KEY_PATH, APP_STORE_CONNECT_KEY_ID, and APP_STORE_CONNECT_ISSUER_ID for Method B."
fi
echo "Secrets printed: no"

echo "== DMG Hash =="
ACTUAL_SHA="$(shasum -a 256 "${DMG_PATH}" | awk '{print $1}')"
echo "DMG SHA-256: ${ACTUAL_SHA}"
if [[ -n "${EXPECTED_SHA}" ]]; then
  echo "Expected SHA-256: ${EXPECTED_SHA}"
  if [[ "${ACTUAL_SHA}" != "${EXPECTED_SHA}" ]]; then
    echo "FAIL SHA-256 mismatch" >&2
    exit 65
  fi
  echo "DMG SHA-256 match: pass"
fi

echo "== DMG Integrity =="
hdiutil verify "${DMG_PATH}"
echo "DMG integrity: pass"

echo "== DMG Signature =="
codesign --verify --verbose=2 "${DMG_PATH}"
DMG_SIGNATURE_DETAILS="$(codesign -dvvv "${DMG_PATH}" 2>&1 || true)"
printf '%s\n' "${DMG_SIGNATURE_DETAILS}" | print_relevant_codesign_lines
require_signature_line "${DMG_SIGNATURE_DETAILS}" 'Authority=Developer ID Application' "DMG signature: pass" "DMG is not signed with Developer ID Application"
require_signature_line "${DMG_SIGNATURE_DETAILS}" 'Timestamp=' "DMG timestamp: pass" "DMG signature is missing a secure timestamp"

echo "== Mounted App Signature =="
MOUNT_PATH="$(mktemp -d "${TMPDIR:-/tmp}/forge-notary-preflight-mount.XXXXXX")"
hdiutil attach -nobrowse -readonly -mountpoint "${MOUNT_PATH}" "${DMG_PATH}" >/dev/null
APP_PATH="${MOUNT_PATH}/${APP_NAME}"
if [[ ! -d "${APP_PATH}" ]]; then
  echo "FAIL app bundle missing at ${APP_PATH}" >&2
  exit 67
fi
codesign --verify --deep --strict --verbose=2 "${APP_PATH}"
APP_SIGNATURE_DETAILS="$(codesign -dvvv "${APP_PATH}" 2>&1 || true)"
printf '%s\n' "${APP_SIGNATURE_DETAILS}" | print_relevant_codesign_lines
require_signature_line "${APP_SIGNATURE_DETAILS}" 'Authority=Developer ID Application' "Mounted app signature: pass" "mounted app is not signed with Developer ID Application"
require_signature_line "${APP_SIGNATURE_DETAILS}" 'Timestamp=' "Mounted app timestamp: pass" "mounted app signature is missing a secure timestamp"
require_signature_line "${APP_SIGNATURE_DETAILS}" 'Runtime Version=' "Mounted app hardened runtime: pass" "mounted app signature is missing hardened runtime"
cleanup
MOUNT_PATH=""

echo "== Pre-Notarization State =="
STAPLER_OUTPUT="$(xcrun stapler validate -v "${DMG_PATH}" 2>&1)" && STAPLER_STATUS=0 || STAPLER_STATUS=$?
printf '%s\n' "${STAPLER_OUTPUT}" | tail -n 8
if [[ "${STAPLER_STATUS}" -eq 0 ]]; then
  echo "Stapled ticket: present"
else
  echo "Stapled ticket: missing"
  echo "Notarization hold: submit the DMG to Apple, wait for Accepted, then staple."
fi

SPCTL_OUTPUT="$(run_spctl -a -vvv -t install "${DMG_PATH}")" && SPCTL_STATUS=0 || SPCTL_STATUS=$?
printf '%s\n' "${SPCTL_OUTPUT}" | tail -n 5
if [[ "${SPCTL_STATUS}" -eq 0 ]]; then
  echo "Gatekeeper DMG assessment before submission: accepted"
else
  echo "Gatekeeper DMG assessment before submission: not accepted"
fi

echo "== Next Commands =="
if [[ "${CREDENTIAL_METHOD}" == "App Store Connect API key" ]]; then
  cat <<NEXT
xcrun notarytool submit $(shell_quote "${DMG_PATH}") \\
  --key $(env_ref "${API_KEY_PATH_VAR}") \\
  --key-id $(env_ref "${API_KEY_ID_VAR}") \\
  --issuer $(env_ref "${API_ISSUER_ID_VAR}") \\
  --wait
NEXT
else
  if [[ "${CREDENTIAL_METHOD}" == "not configured" ]]; then
    cat <<NEXT
xcrun notarytool store-credentials $(shell_quote "${KEYCHAIN_PROFILE}") \\
  --apple-id "\$APPLE_ID" \\
  --team-id "\$APPLE_TEAM_ID" \\
  --password "\$APPLE_APP_SPECIFIC_PASSWORD"
NEXT
  fi
  echo "xcrun notarytool submit $(shell_quote "${DMG_PATH}") --keychain-profile $(shell_quote "${KEYCHAIN_PROFILE}") --wait"
fi

cat <<NEXT
xcrun stapler staple -v $(shell_quote "${DMG_PATH}")
xcrun stapler validate -v $(shell_quote "${DMG_PATH}")
scripts/verify-release-package.sh $(shell_quote "${DMG_PATH}") $(shell_quote "${ACTUAL_SHA}")
NEXT

echo "Preflight complete. This did not submit, staple, or claim notarization passed."
