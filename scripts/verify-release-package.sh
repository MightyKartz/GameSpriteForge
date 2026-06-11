#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "Usage: $0 /path/to/Game\\ Sprite\\ Forge_0.1.0_aarch64.dmg [expected_sha256]" >&2
  exit 64
fi

DMG_PATH="$1"
EXPECTED_SHA="${2:-218a5f9adf1fb47e5f744efbda357470ac8c6aa2c065451c926842c35cd0c739}"
APP_NAME="Game Sprite Forge.app"
APP_EXECUTABLE="Game Sprite Forge"
MOUNT_PATH=""
APP_PID=""

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

cleanup() {
  osascript -e 'tell application "Game Sprite Forge" to quit' >/dev/null 2>&1 || true
  pkill -x "${APP_EXECUTABLE}" >/dev/null 2>&1 || true
  if [[ -n "${APP_PID:-}" ]]; then
    kill "${APP_PID}" >/dev/null 2>&1 || true
  fi
  if [[ -n "${MOUNT_PATH}" ]]; then
    hdiutil detach "${MOUNT_PATH}" >/dev/null 2>&1 || true
    rmdir "${MOUNT_PATH}" >/dev/null 2>&1 || true
  fi
}

trap cleanup EXIT

find_launched_app_pid() {
  local executable_path="$1"
  local candidate=""
  while IFS= read -r candidate; do
    if lsof -p "${candidate}" -Fn 2>/dev/null | grep -Fxq "n${executable_path}"; then
      printf '%s\n' "${candidate}"
      return 0
    fi
  done < <(pgrep -x "${APP_EXECUTABLE}" || true)
  return 1
}

require_command shasum
require_command hdiutil
require_command spctl
require_command codesign
require_command xcrun
require_command open
require_command pgrep
require_command lsof
require_command osascript
raise_open_file_limit

if [[ ! -f "${DMG_PATH}" ]]; then
  echo "FAIL DMG not found: ${DMG_PATH}" >&2
  exit 66
fi

echo "== Game Sprite Forge Release Package Verification =="
echo "DMG: ${DMG_PATH}"
echo "Expected SHA-256: ${EXPECTED_SHA}"

ACTUAL_SHA="$(shasum -a 256 "${DMG_PATH}" | awk '{print $1}')"
echo "Actual SHA-256:   ${ACTUAL_SHA}"
if [[ "${ACTUAL_SHA}" != "${EXPECTED_SHA}" ]]; then
  echo "FAIL SHA-256 mismatch" >&2
  exit 65
fi

echo "== Disk Image Integrity =="
hdiutil verify "${DMG_PATH}"

echo "== Notarization Ticket =="
xcrun stapler validate -v "${DMG_PATH}" | tail -n 5

echo "== Gatekeeper DMG Assessment =="
run_spctl -a -vvv -t install "${DMG_PATH}"
run_spctl -a -vvv --context context:primary-signature -t open "${DMG_PATH}"

echo "== Attach DMG =="
MOUNT_PATH="$(mktemp -d "${TMPDIR:-/tmp}/forge-release-mount.XXXXXX")"
hdiutil attach -nobrowse -readonly -mountpoint "${MOUNT_PATH}" "${DMG_PATH}"
APP_PATH="${MOUNT_PATH}/${APP_NAME}"

if [[ ! -d "${APP_PATH}" ]]; then
  echo "FAIL app bundle missing at ${APP_PATH}" >&2
  exit 67
fi

echo "== App Signature =="
codesign --verify --deep --strict --verbose=2 "${APP_PATH}"
codesign -dvvv "${APP_PATH}" 2>&1 | grep -E 'Authority=Developer ID Application|TeamIdentifier=|Runtime Version=|Sealed Resources'

echo "== Gatekeeper App Assessment =="
run_spctl -a -vvv -t exec "${APP_PATH}"

echo "== Launch App From DMG =="
open -n "${APP_PATH}"
sleep 6
APP_EXECUTABLE_PATH="$(cd "${APP_PATH}/Contents/MacOS" && pwd -P)/${APP_EXECUTABLE}"
APP_PID="$(find_launched_app_pid "${APP_EXECUTABLE_PATH}" || true)"
if [[ -z "${APP_PID}" ]]; then
  echo "FAIL app did not launch from mounted DMG" >&2
  exit 68
fi
echo "Observed Game Sprite Forge pid: ${APP_PID}"

cleanup

echo "== Manual Acceptance Items =="
cat <<'MANUAL'
Confirm these in the app UI on the verification Mac:
- If ffmpeg/ffprobe are missing, Settings or dependency check shows exactly:
  Install ffmpeg or choose an ffmpeg binary in Settings.
- If ffmpeg/ffprobe are configured, examples/inputs/green-box-character.mp4 processes end to end.
- Exported .gsfpack validates and can be re-imported with the same frame count.
- No AI provider, website, MCP, marketplace, cloud, BYOK, or cost setup is required.
MANUAL

echo "PASS release package verification script completed."
