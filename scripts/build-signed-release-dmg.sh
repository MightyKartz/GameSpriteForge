#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="0.1.0"
ARCH="aarch64"
APP_NAME="Game Sprite Forge.app"
APP_EXECUTABLE="Game Sprite Forge"
IDENTITY="${FORGE_CODESIGN_IDENTITY:-Developer ID Application: Ka Yan (J6P96F432P)}"
TIMESTAMP_URL="${FORGE_CODESIGN_TIMESTAMP_URL:-http://timestamp.apple.com/ts01}"
ENTITLEMENTS="${ROOT}/apps/mac/src-tauri/Entitlements.plist"

APP_PARENT="${ROOT}/target/release/bundle/macos"
APP="${APP_PARENT}/${APP_NAME}"
APP_BINARY="${APP}/Contents/MacOS/${APP_EXECUTABLE}"
DMG_DIR="${ROOT}/target/release/bundle/dmg"
DMG="${DMG_DIR}/Game Sprite Forge_${VERSION}_${ARCH}.dmg"
DMG_ICON="${DMG_DIR}/Game Sprite Forge.icns"
BUNDLE_DMG_SCRIPT="${DMG_DIR}/bundle_dmg.sh"
STAGE_DIR=""
MOUNT_PATH=""

cleanup() {
  if [[ -n "${MOUNT_PATH}" ]]; then
    hdiutil detach "${MOUNT_PATH}" >/dev/null 2>&1 || true
    rmdir "${MOUNT_PATH}" >/dev/null 2>&1 || true
  fi
  if [[ -n "${STAGE_DIR}" ]]; then
    rm -rf "${STAGE_DIR}"
  fi
}

trap cleanup EXIT

require_command() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "FAIL missing required command: $1" >&2
    exit 69
  }
}

print_signature_summary() {
  codesign -dvvv "$1" 2>&1 | awk '/Authority=Developer ID Application|TeamIdentifier=|Timestamp=|Runtime Version=|Identifier=/ { print }'
}

require_command npm
require_command codesign
require_command hdiutil
require_command shasum
require_command xattr

cd "${ROOT}"
npm --workspace apps/mac run tauri -- build --bundles app --no-sign

if [[ ! -d "${APP}" ]]; then
  echo "FAIL app bundle missing after Tauri build: ${APP}" >&2
  exit 66
fi
if [[ ! -x "${APP_BINARY}" ]]; then
  echo "FAIL app executable missing after Tauri build: ${APP_BINARY}" >&2
  exit 66
fi

xattr -crs "${APP}"
codesign --force "--timestamp=${TIMESTAMP_URL}" --options runtime --entitlements "${ENTITLEMENTS}" --sign "${IDENTITY}" "${APP_BINARY}"
codesign --force "--timestamp=${TIMESTAMP_URL}" --options runtime --entitlements "${ENTITLEMENTS}" --sign "${IDENTITY}" "${APP}"
codesign --verify --deep --strict --verbose=4 "${APP}"
print_signature_summary "${APP}"

rm -f "${DMG}" "${DMG}.pre-manual-sign"
mkdir -p "${DMG_DIR}"

if [[ -f "${BUNDLE_DMG_SCRIPT}" && -f "${DMG_ICON}" ]]; then
  bash "${BUNDLE_DMG_SCRIPT}" \
    --volname "Game Sprite Forge" \
    --volicon "${DMG_ICON}" \
    --window-size 660 400 \
    --icon-size 128 \
    --icon "${APP_NAME}" 180 170 \
    --hide-extension "${APP_NAME}" \
    --app-drop-link 480 170 \
    --no-internet-enable \
    --format UDZO \
    "${DMG}" \
    "${APP_PARENT}"
else
  STAGE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/forge-dmg-stage.XXXXXX")"
  ditto "${APP}" "${STAGE_DIR}/${APP_NAME}"
  ln -s /Applications "${STAGE_DIR}/Applications"
  hdiutil create -volname "Game Sprite Forge" -srcfolder "${STAGE_DIR}" -ov -format UDZO "${DMG}"
fi

codesign --force "--timestamp=${TIMESTAMP_URL}" --sign "${IDENTITY}" "${DMG}"
codesign --verify --deep --strict --verbose=4 "${DMG}"
print_signature_summary "${DMG}"
hdiutil verify "${DMG}"

MOUNT_PATH="$(mktemp -d "${TMPDIR:-/tmp}/forge-signed-dmg-mount.XXXXXX")"
hdiutil attach -nobrowse -readonly -mountpoint "${MOUNT_PATH}" "${DMG}" >/dev/null
codesign --verify --deep --strict --verbose=4 "${MOUNT_PATH}/${APP_NAME}"
print_signature_summary "${MOUNT_PATH}/${APP_NAME}"
cleanup
MOUNT_PATH=""

DMG_SHA="$(shasum -a 256 "${DMG}" | awk '{print $1}')"
cat <<SUMMARY
Signed release DMG created:
  DMG: ${DMG}
  SHA-256: ${DMG_SHA}

Next:
  xcrun notarytool submit "${DMG}" --keychain-profile "GameSpriteForgeNotary" --wait
  xcrun stapler staple -v "${DMG}"
  xcrun stapler validate -v "${DMG}"
SUMMARY
