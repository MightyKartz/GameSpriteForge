#!/usr/bin/env bash
# Exercise the actual archive in isolated install roots, including an old-to-new upgrade.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RELEASE="$(cd "${1:?release artifact directory required}" && pwd)"
VERSION="${2:?release tag required}"
PREVIOUS="$(cd "${3:?previous release artifact directory required}" && pwd)"
PREVIOUS_VERSION="${4:?previous release tag required}"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/forge-release-artifact.XXXXXX")"
trap 'rm -rf "${TEST_ROOT}"' EXIT
command -v godot >/dev/null
godot --version | grep -E '^4\.6\.'

install_archive() {
  FORGE_VERSION="$2" FORGE_INSTALL_TEST_MODE=1 \
    FORGE_RELEASE_BASE_URL="file://$1" \
    FORGE_INSTALL_ROOT="${TEST_ROOT}/$3/share" \
    FORGE_BIN_DIR="${TEST_ROOT}/$3/bin" \
    FORGE_PROFILE_FILE="${TEST_ROOT}/$3.profile" \
    sh "${ROOT}/install.sh"
}

install_archive "${RELEASE}" "${VERSION}" fresh
install_archive "${RELEASE}" "${VERSION}" fresh
install_archive "${PREVIOUS}" "${PREVIOUS_VERSION}" upgrade
OLD="${TEST_ROOT}/upgrade/share/versions/${PREVIOUS_VERSION}/bin/forge"
OLD_HASH="$(shasum -a 256 "${OLD}" | awk '{print $1}')"
"${OLD}" --version | grep -F -- "${PREVIOUS_VERSION#v}"
install_archive "${RELEASE}" "${VERSION}" upgrade
test "$(shasum -a 256 "${OLD}" | awk '{print $1}')" = "${OLD_HASH}"

for kind in fresh upgrade; do
  PAYLOAD="${TEST_ROOT}/${kind}/share/versions/${VERSION}"
  test "$(readlink "${TEST_ROOT}/${kind}/bin/forge")" = "${PAYLOAD}/bin/forge"
  jq -e --arg version "${VERSION#v}" \
    '.version == $version and .target == "aarch64-apple-darwin"' "${PAYLOAD}/BUILD_INFO.json" >/dev/null
  for binary in forge ffmpeg ffprobe; do
    file "${PAYLOAD}/bin/${binary}" | grep -F arm64 >/dev/null
  done
  "${TEST_ROOT}/${kind}/bin/forge" --version | grep -F -- "${VERSION#v}"
  "${PAYLOAD}/bin/ffmpeg" -version >/dev/null
  "${PAYLOAD}/bin/ffprobe" -version >/dev/null
  python3 "${ROOT}/scripts/verify-cli-build.py" \
    --forge "${TEST_ROOT}/${kind}/bin/forge" --version "${VERSION#v}" \
    --release --build-info "${PAYLOAD}/BUILD_INFO.json"
done

python3 "${ROOT}/scripts/test-cli-skill.py" \
  --forge "${TEST_ROOT}/upgrade/bin/forge" --output "${TEST_ROOT}/packaged-skill"
python3 "${ROOT}/scripts/test-asset-library-cli.py" \
  --forge "${TEST_ROOT}/upgrade/bin/forge" --legacy-forge "${OLD}"
python3 "${ROOT}/scripts/test-asset-library-review-cli.py" \
  --forge "${TEST_ROOT}/upgrade/bin/forge"
python3 "${ROOT}/scripts/test-asset-library-portability-cli.py" \
  --forge "${TEST_ROOT}/upgrade/bin/forge" --godot "$(command -v godot)" \
  --output "${TEST_ROOT}/packaged-library-portability"

PAYLOAD="${TEST_ROOT}/upgrade/share/versions/${VERSION}"
FORGE_BINARY="${TEST_ROOT}/upgrade/bin/forge" FORGE_VERIFY_GODOT=1 \
  bash "${ROOT}/scripts/test-cli-product.sh"
python3 "${ROOT}/scripts/test-local-static-cli.py" \
  --forge "${TEST_ROOT}/upgrade/bin/forge" --godot "$(command -v godot)" \
  --output "${TEST_ROOT}/packaged-local-static"
python3 "${ROOT}/scripts/test-local-animation-delivery.py" \
  --forge "${TEST_ROOT}/upgrade/bin/forge" --godot "$(command -v godot)" \
  --output "${TEST_ROOT}/packaged-local-animation"
echo "PASS packaged CLI fresh install, reinstall, upgrade, product and local asset contracts (${VERSION})"
