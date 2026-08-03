#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
FORGE_BIN="${ROOT_DIR}/target/debug/forge"
TEST_ROOT="$(mktemp -d /tmp/forge-world-product.XXXXXX)"
trap 'rm -rf "${TEST_ROOT}"' EXIT

GODOT="${FORGE_GODOT_PATH:-/Applications/Godot.app/Contents/MacOS/Godot}"
if [ ! -x "${GODOT}" ]; then
	GODOT="$(command -v godot || command -v godot4 || true)"
fi
test -x "${GODOT}"
export FORGE_GODOT_PATH="${GODOT}"

credential_scan_matches() {
	if command -v rg >/dev/null 2>&1; then
		rg -n -i \
			'authorization:[[:space:]]*bearer|access[_-]?token|refresh[_-]?token|device[_-]?code|xai[_-]?api[_-]?key' \
			"$@"
	else
		grep -R -I -n -E \
			'authorization:[[:space:]]*bearer|access[_-]?token|refresh[_-]?token|device[_-]?code|xai[_-]?api[_-]?key' \
			"$@"
	fi
}

export FORGE_JOB_STORE="${TEST_ROOT}/jobs"
export FORGE_PLAN_STORE="${TEST_ROOT}/plans"

cargo build --manifest-path "${ROOT_DIR}/Cargo.toml" -p forge-cli --features world-assets
for command in environment terrain building map; do
	"${FORGE_BIN}" --help | grep -E "^  ${command}[[:space:]]" >/dev/null
done
"${FORGE_BIN}" project init --path "${TEST_ROOT}/project" --name "World Product" --provider fixture --json >/dev/null
"${FORGE_BIN}" style create --project "${TEST_ROOT}/project" --spec "${ROOT_DIR}/examples/cli/style.json" --wait --json >/dev/null
"${FORGE_BIN}" environment create --project "${TEST_ROOT}/project" --spec "${ROOT_DIR}/examples/cli/world/environment.json" --wait --json >/dev/null
"${FORGE_BIN}" generate terrain-set --project "${TEST_ROOT}/project" --spec "${ROOT_DIR}/examples/cli/world/terrain.json" --wait --json >/dev/null
"${FORGE_BIN}" generate building-kit --project "${TEST_ROOT}/project" --spec "${ROOT_DIR}/examples/cli/world/buildings.json" --wait --json >/dev/null

TERRAIN_PACK="$(find "${FORGE_JOB_STORE}" -type d -path '*/exports/forest-ground.gsfpack' -print -quit)"
BUILDING_PACK="$(find "${FORGE_JOB_STORE}" -type d -path '*/exports/forest-houses.gsfpack' -print -quit)"
test -n "${TERRAIN_PACK}"
test -n "${BUILDING_PACK}"
"${FORGE_BIN}" pack validate --path "${TERRAIN_PACK}" --json >/dev/null
"${FORGE_BIN}" pack validate --path "${BUILDING_PACK}" --json >/dev/null
"${FORGE_BIN}" terrain test --pack "${TERRAIN_PACK}" --samples 32 --json >/dev/null
"${FORGE_BIN}" building test --pack "${BUILDING_PACK}" --json >/dev/null

mkdir -p "${TEST_ROOT}/map-spec/packs"
cp "${ROOT_DIR}/examples/cli/world/map.json" "${TEST_ROOT}/map-spec/map.json"
cp -R "${TERRAIN_PACK}" "${TEST_ROOT}/map-spec/packs/forest-ground.gsfpack"
cp -R "${BUILDING_PACK}" "${TEST_ROOT}/map-spec/packs/forest-houses.gsfpack"
"${FORGE_BIN}" map schema --json >/dev/null
"${FORGE_BIN}" map compile --project "${TEST_ROOT}/project" --spec "${TEST_ROOT}/map-spec/map.json" --wait --json >/dev/null
MAP_PACK="$(find "${FORGE_JOB_STORE}" -type d -path '*/exports/forest-village.gsfpack' -print -quit)"
test -n "${MAP_PACK}"
"${FORGE_BIN}" map validate --pack "${MAP_PACK}" --json >/dev/null

mkdir -p "${TEST_ROOT}/godot"
printf '%s\n' '[application]' 'config/name="Forge World Product"' '[rendering]' 'renderer/rendering_method="gl_compatibility"' >"${TEST_ROOT}/godot/project.godot"

install_pack() {
	local pack="$1"
	local target="$2"
	local plan token
	plan="$("${FORGE_BIN}" godot plan-install --pack "${pack}" --project "${TEST_ROOT}/godot" --target "${target}" --json)"
	token="$(printf '%s' "${plan}" | /usr/bin/python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["token"])')"
	"${FORGE_BIN}" plan execute --token "${token}" --wait --json >/dev/null
}

install_pack "${TERRAIN_PACK}" "addons/forge_assets/terrain"
install_pack "${BUILDING_PACK}" "addons/forge_assets/buildings"
install_pack "${MAP_PACK}" "addons/forge_assets/world"

"${GODOT}" \
	--headless \
	--path "${TEST_ROOT}/godot" \
	--script "${ROOT_DIR}/scripts/godot/verify_forge_world.gd" \
	-- \
	"res://addons/forge_assets"

if find "${TEST_ROOT}/godot/addons/forge_assets" -type f \( -name '*.tres' -o -name '*.tscn' \) -size +1048575c | grep -q .; then
	echo "World resource exceeds 1 MiB" >&2
	exit 1
fi
if grep -R -E 'sub_resource type="Image"|sub_resource type="ImageTexture"|ImageTexture.create_from_image|^data = PackedByteArray' "${TEST_ROOT}/godot/addons/forge_assets" --include='*.tres' --include='*.tscn'; then
	echo "World resource embeds image pixels" >&2
	exit 1
fi
if credential_scan_matches "${FORGE_JOB_STORE}" "${TEST_ROOT}/godot" >/dev/null; then
	echo "credential-like material leaked into world outputs" >&2
	exit 1
fi

echo "PASS Forge fixture world assets, V3 Packs, and Godot delivery"
