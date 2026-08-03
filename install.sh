#!/bin/sh
set -eu

REPOSITORY="MightyKartz/GameSpriteForge"
INSTALL_ROOT="${FORGE_INSTALL_ROOT:-${HOME}/.local/share/forge}"
PUBLIC_BIN_DIR="${FORGE_BIN_DIR:-${HOME}/.local/bin}"
PROFILE_FILE="${FORGE_PROFILE_FILE:-${HOME}/.zprofile}"
MARKER_BEGIN="# >>> Game Sprite Forge >>>"
MARKER_END="# <<< Game Sprite Forge <<<"
FORGE_CLI_CODESIGN_IDENTIFIER="dev.gamespriteforge.cli"
FORGE_FFMPEG_CODESIGN_IDENTIFIER="dev.gamespriteforge.ffmpeg"
FORGE_FFPROBE_CODESIGN_IDENTIFIER="dev.gamespriteforge.ffprobe"
FORGE_APPLE_TEAM_ID="J6P96F432P"

fail() {
  printf 'Forge install failed: %s\n' "$1" >&2
  exit 1
}

[ "$(uname -s)" = "Darwin" ] || fail "v0.2 supports macOS only"
[ "$(uname -m)" = "arm64" ] || fail "v0.2 supports Apple Silicon (arm64) only"
command -v curl >/dev/null 2>&1 || fail "curl is required"
command -v shasum >/dev/null 2>&1 || fail "shasum is required"
command -v ditto >/dev/null 2>&1 || fail "ditto is required"

if [ -n "${FORGE_VERSION:-}" ]; then
  VERSION="${FORGE_VERSION}"
else
  RELEASE_JSON="$(curl --proto '=https' --tlsv1.2 -fsSL "https://api.github.com/repos/${REPOSITORY}/releases/latest")"
  VERSION="$(printf '%s' "${RELEASE_JSON}" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)"
fi
[ -n "${VERSION}" ] || fail "could not resolve the latest Forge release"
case "${VERSION}" in
  v*) ;;
  *) fail "release version must begin with v" ;;
esac
case "${VERSION}" in
  *[!A-Za-z0-9._-]*) fail "release version contains unsafe characters" ;;
esac

ASSET="forge-aarch64-apple-darwin.zip"
BASE_URL="https://github.com/${REPOSITORY}/releases/download/${VERSION}"
if [ -n "${FORGE_RELEASE_BASE_URL:-}" ]; then
  [ "${FORGE_INSTALL_TEST_MODE:-}" = "1" ] \
    || fail "FORGE_RELEASE_BASE_URL is reserved for installer contract tests"
  BASE_URL="${FORGE_RELEASE_BASE_URL}"
fi
TEMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/forge-install.XXXXXX")"
VERSION_ROOT="${INSTALL_ROOT}/versions/${VERSION}"
BACKUP_ROOT="${INSTALL_ROOT}/versions/.${VERSION}.backup.$$"
cleanup() {
  if [ -d "${BACKUP_ROOT}" ] && [ ! -e "${VERSION_ROOT}" ]; then
    mv "${BACKUP_ROOT}" "${VERSION_ROOT}" || true
  fi
  rm -rf "${TEMP_ROOT}"
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM

download() {
  if [ "${FORGE_INSTALL_TEST_MODE:-}" = "1" ]; then
    curl -fL "$1" -o "$2"
  else
    curl --proto '=https' --tlsv1.2 -fL "$1" -o "$2"
  fi
}

download "${BASE_URL}/${ASSET}" "${TEMP_ROOT}/${ASSET}"
download "${BASE_URL}/${ASSET}.sha256" "${TEMP_ROOT}/${ASSET}.sha256"

EXPECTED="$(awk '{print $1}' "${TEMP_ROOT}/${ASSET}.sha256")"
ACTUAL="$(shasum -a 256 "${TEMP_ROOT}/${ASSET}" | awk '{print $1}')"
[ "${EXPECTED}" = "${ACTUAL}" ] || fail "release checksum mismatch"

EXTRACT_ROOT="${TEMP_ROOT}/payload"
mkdir -p "${EXTRACT_ROOT}"
ditto -x -k "${TEMP_ROOT}/${ASSET}" "${EXTRACT_ROOT}"
PAYLOAD="${EXTRACT_ROOT}"
if [ ! -d "${PAYLOAD}/bin" ]; then
  PAYLOAD_CANDIDATE="$(find "${EXTRACT_ROOT}" -mindepth 1 -maxdepth 1 -type d -print | head -n 1)"
  [ -n "${PAYLOAD_CANDIDATE}" ] || fail "release payload has no top-level directory"
  PAYLOAD="${PAYLOAD_CANDIDATE}"
fi
[ -f "${PAYLOAD}/BUILD_INFO.json" ] || fail "release is missing BUILD_INFO.json"
[ -f "${PAYLOAD}/MANIFEST.sha256" ] || fail "release is missing MANIFEST.sha256"
(
  cd "${PAYLOAD}"
  shasum -a 256 -c MANIFEST.sha256 >/dev/null
) || fail "release payload manifest verification failed"

if grep -E '"appleSigned"[[:space:]]*:[[:space:]]*true' "${PAYLOAD}/BUILD_INFO.json" >/dev/null; then
  APPLE_SIGNED="true"
elif grep -E '"appleSigned"[[:space:]]*:[[:space:]]*false' "${PAYLOAD}/BUILD_INFO.json" >/dev/null; then
  APPLE_SIGNED="false"
else
  fail "BUILD_INFO.json has no appleSigned declaration"
fi
for BINARY in forge ffmpeg ffprobe; do
  [ -x "${PAYLOAD}/bin/${BINARY}" ] || fail "release is missing bin/${BINARY}"
  if [ "${APPLE_SIGNED}" = "true" ]; then
    command -v codesign >/dev/null 2>&1 || fail "codesign is required for signed releases"
    codesign --verify --strict --verbose=2 "${PAYLOAD}/bin/${BINARY}" >/dev/null 2>&1 \
      || fail "code signature verification failed for ${BINARY}"
    case "${BINARY}" in
      forge) EXPECTED_IDENTIFIER="${FORGE_CLI_CODESIGN_IDENTIFIER}" ;;
      ffmpeg) EXPECTED_IDENTIFIER="${FORGE_FFMPEG_CODESIGN_IDENTIFIER}" ;;
      ffprobe) EXPECTED_IDENTIFIER="${FORGE_FFPROBE_CODESIGN_IDENTIFIER}" ;;
    esac
    SIGNATURE="$(codesign -dvvv "${PAYLOAD}/bin/${BINARY}" 2>&1)"
    printf '%s\n' "${SIGNATURE}" | grep -F "Identifier=${EXPECTED_IDENTIFIER}" >/dev/null \
      || fail "unexpected code-signing identifier for ${BINARY}"
  fi
  if [ "${APPLE_SIGNED}" = "true" ] && [ "${FORGE_INSTALL_TEST_MODE:-}" != "1" ]; then
    printf '%s\n' "${SIGNATURE}" | grep -F "TeamIdentifier=${FORGE_APPLE_TEAM_ID}" >/dev/null \
      || fail "unexpected Developer ID team for ${BINARY}"
    printf '%s\n' "${SIGNATURE}" | grep -F "Authority=Developer ID Application" >/dev/null \
      || fail "${BINARY} is not signed with Developer ID Application"
  fi
  if [ "${FORGE_INSTALL_TEST_MODE:-}" != "1" ]; then
    /usr/bin/file "${PAYLOAD}/bin/${BINARY}" | grep -F "arm64" >/dev/null \
      || fail "${BINARY} is not an Apple Silicon executable"
  fi
done

STAGED_ROOT="${INSTALL_ROOT}/versions/.${VERSION}.staged.$$"
mkdir -p "${INSTALL_ROOT}/versions" "${PUBLIC_BIN_DIR}"
rm -rf "${STAGED_ROOT}"
mv "${PAYLOAD}" "${STAGED_ROOT}"
if [ -e "${VERSION_ROOT}" ]; then
  rm -rf "${BACKUP_ROOT}"
  mv "${VERSION_ROOT}" "${BACKUP_ROOT}"
fi
if ! mv "${STAGED_ROOT}" "${VERSION_ROOT}"; then
  if [ -d "${BACKUP_ROOT}" ]; then
    mv "${BACKUP_ROOT}" "${VERSION_ROOT}"
  fi
  fail "could not commit the new version directory"
fi

LINK_TEMP="${PUBLIC_BIN_DIR}/.forge.$$"
ln -s "${VERSION_ROOT}/bin/forge" "${LINK_TEMP}"
if ! mv -f "${LINK_TEMP}" "${PUBLIC_BIN_DIR}/forge"; then
  rm -rf "${VERSION_ROOT}"
  if [ -d "${BACKUP_ROOT}" ]; then
    mv "${BACKUP_ROOT}" "${VERSION_ROOT}"
  fi
  fail "could not atomically update the forge command"
fi
rm -rf "${BACKUP_ROOT}"

if ! printf '%s' ":${PATH}:" | grep -F ":${PUBLIC_BIN_DIR}:" >/dev/null 2>&1; then
  if [ ! -f "${PROFILE_FILE}" ] || ! grep -F "${MARKER_BEGIN}" "${PROFILE_FILE}" >/dev/null 2>&1; then
    {
      printf '\n%s\n' "${MARKER_BEGIN}"
      printf 'export PATH="%s:$PATH"\n' "${PUBLIC_BIN_DIR}"
      printf '%s\n' "${MARKER_END}"
    } >> "${PROFILE_FILE}"
  fi
fi

printf 'Forge %s installed successfully.\n' "${VERSION}"
printf 'Open a new terminal, then run: forge doctor --json\n'
