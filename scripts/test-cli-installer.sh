#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "${ROOT}/scripts/macos-cli-signing.env"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/forge-installer-test.XXXXXX")"
trap 'rm -rf "${TEST_ROOT}"' EXIT

PAYLOAD="${TEST_ROOT}/forge-dist"
RELEASE="${TEST_ROOT}/release"
mkdir -p "${PAYLOAD}/bin" "${PAYLOAD}/licenses" "${RELEASE}"
for binary in forge ffmpeg ffprobe; do
  cp /usr/bin/true "${PAYLOAD}/bin/${binary}"
  codesign --remove-signature "${PAYLOAD}/bin/${binary}" >/dev/null 2>&1 || true
done
cp "${ROOT}/LICENSE" "${PAYLOAD}/licenses/"
printf '%s\n' '{"version":"test","target":"aarch64-apple-darwin","appleSigned":false,"notarized":false}' \
  > "${PAYLOAD}/BUILD_INFO.json"
(
  cd "${PAYLOAD}"
  shasum -a 256 bin/forge bin/ffmpeg bin/ffprobe licenses/* BUILD_INFO.json > MANIFEST.sha256
)
ditto -c -k --keepParent "${PAYLOAD}" "${RELEASE}/forge-aarch64-apple-darwin.zip"
shasum -a 256 "${RELEASE}/forge-aarch64-apple-darwin.zip" \
  > "${RELEASE}/forge-aarch64-apple-darwin.zip.sha256"

run_installer() {
  test_version="${1:-v-test}"
  release_root="${2:-${RELEASE}}"
  FORGE_VERSION="${test_version}" \
  FORGE_INSTALL_TEST_MODE=1 \
  FORGE_RELEASE_BASE_URL="file://${release_root}" \
  FORGE_INSTALL_ROOT="${TEST_ROOT}/share" \
  FORGE_BIN_DIR="${TEST_ROOT}/public-bin" \
  FORGE_PROFILE_FILE="${TEST_ROOT}/.zprofile" \
    sh "${ROOT}/install.sh"
}

run_installer
run_installer

test -L "${TEST_ROOT}/public-bin/forge"
test -x "${TEST_ROOT}/share/versions/v-test/bin/forge"
test -x "${TEST_ROOT}/share/versions/v-test/bin/ffmpeg"
test -x "${TEST_ROOT}/share/versions/v-test/bin/ffprobe"
test "$(grep -c '^# >>> Game Sprite Forge >>>$' "${TEST_ROOT}/.zprofile")" -eq 1

run_installer v-next
test -x "${TEST_ROOT}/share/versions/v-test/bin/forge"
test -x "${TEST_ROOT}/share/versions/v-next/bin/forge"
test "$(readlink "${TEST_ROOT}/public-bin/forge")" = "${TEST_ROOT}/share/versions/v-next/bin/forge"

printf 'bad checksum  forge-aarch64-apple-darwin.zip\n' \
  > "${RELEASE}/forge-aarch64-apple-darwin.zip.sha256"
if run_installer >/dev/null 2>&1; then
  echo "installer unexpectedly accepted a bad checksum" >&2
  exit 1
fi

test -x "${TEST_ROOT}/share/versions/v-test/bin/forge"
test "$(readlink "${TEST_ROOT}/public-bin/forge")" = "${TEST_ROOT}/share/versions/v-next/bin/forge"

BAD_MANIFEST_RELEASE="${TEST_ROOT}/bad-manifest-release"
BAD_MANIFEST_PAYLOAD="${TEST_ROOT}/bad-manifest-dist"
cp -R "${PAYLOAD}" "${BAD_MANIFEST_PAYLOAD}"
printf 'tampered' >> "${BAD_MANIFEST_PAYLOAD}/bin/forge"
mkdir -p "${BAD_MANIFEST_RELEASE}"
ditto -c -k --keepParent "${BAD_MANIFEST_PAYLOAD}" \
  "${BAD_MANIFEST_RELEASE}/forge-aarch64-apple-darwin.zip"
shasum -a 256 "${BAD_MANIFEST_RELEASE}/forge-aarch64-apple-darwin.zip" \
  > "${BAD_MANIFEST_RELEASE}/forge-aarch64-apple-darwin.zip.sha256"
if run_installer v-bad-manifest "${BAD_MANIFEST_RELEASE}" >/dev/null 2>&1; then
  echo "installer unexpectedly accepted a bad payload manifest" >&2
  exit 1
fi
test "$(readlink "${TEST_ROOT}/public-bin/forge")" = "${TEST_ROOT}/share/versions/v-next/bin/forge"

WRONG_ID_RELEASE="${TEST_ROOT}/wrong-id-release"
WRONG_ID_PAYLOAD="${TEST_ROOT}/wrong-id-dist"
cp -R "${PAYLOAD}" "${WRONG_ID_PAYLOAD}"
for binary in forge ffmpeg ffprobe; do
  case "${binary}" in
    forge) identifier="${FORGE_CLI_CODESIGN_IDENTIFIER}" ;;
    ffmpeg) identifier="${FORGE_FFMPEG_CODESIGN_IDENTIFIER}" ;;
    ffprobe) identifier="${FORGE_FFPROBE_CODESIGN_IDENTIFIER}" ;;
  esac
  codesign --force --identifier "${identifier}" --sign - \
    "${WRONG_ID_PAYLOAD}/bin/${binary}" >/dev/null
done
codesign --force --identifier "dev.gamespriteforge.wrong" --sign - \
  "${WRONG_ID_PAYLOAD}/bin/forge" >/dev/null
printf '%s\n' '{"version":"test","target":"aarch64-apple-darwin","appleSigned":true,"notarized":false}' \
  > "${WRONG_ID_PAYLOAD}/BUILD_INFO.json"
(
  cd "${WRONG_ID_PAYLOAD}"
  shasum -a 256 bin/forge bin/ffmpeg bin/ffprobe licenses/* BUILD_INFO.json > MANIFEST.sha256
)
mkdir -p "${WRONG_ID_RELEASE}"
ditto -c -k --keepParent "${WRONG_ID_PAYLOAD}" \
  "${WRONG_ID_RELEASE}/forge-aarch64-apple-darwin.zip"
shasum -a 256 "${WRONG_ID_RELEASE}/forge-aarch64-apple-darwin.zip" \
  > "${WRONG_ID_RELEASE}/forge-aarch64-apple-darwin.zip.sha256"
if run_installer v-wrong-id "${WRONG_ID_RELEASE}" >/dev/null 2>&1; then
  echo "installer unexpectedly accepted a wrong signing identifier" >&2
  exit 1
fi
test "$(readlink "${TEST_ROOT}/public-bin/forge")" = "${TEST_ROOT}/share/versions/v-next/bin/forge"

if run_installer v-network-failure "${TEST_ROOT}/missing-release" >/dev/null 2>&1; then
  echo "installer unexpectedly accepted a missing release" >&2
  exit 1
fi
test "$(readlink "${TEST_ROOT}/public-bin/forge")" = "${TEST_ROOT}/share/versions/v-next/bin/forge"
echo "PASS Forge CLI installer contract"
