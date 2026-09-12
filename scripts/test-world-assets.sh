#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
FORGE_BIN="${ROOT_DIR}/target/debug/forge"
mkdir -p "${ROOT_DIR}/target/qa"
TEST_ROOT="$(mktemp -d "${ROOT_DIR}/target/qa/forge-world-product.XXXXXX")"
trap 'world_exit=$?; if [ "$world_exit" -eq 0 ]; then rm -rf "${TEST_ROOT}"; else printf "FAIL world contract; fixture retained at %s\n" "${TEST_ROOT}" >&2; fi' EXIT
trap 'world_exit=$?; printf "FAIL world contract at line %s (exit %s)\n" "$LINENO" "$world_exit" >&2; exit "$world_exit"' ERR

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
	local result="${TEST_ROOT}/install-$(basename "${target}").json"
	if ! "${FORGE_BIN}" plan execute --token "${token}" --wait --json >"${result}"; then
		cat "${result}" >&2
		for log in "${FORGE_JOB_STORE}"/*/logs/godot*.stderr.log; do
			if [ -f "${log}" ] && [ -s "${log}" ]; then
				printf 'Godot diagnostic: %s\n' "${log}" >&2
				cat "${log}" >&2
			fi
		done
		return 1
	fi
}

install_pack "${TERRAIN_PACK}" "addons/forge_assets/terrain"
install_pack "${BUILDING_PACK}" "addons/forge_assets/buildings"
install_pack "${MAP_PACK}" "addons/forge_assets/world"

"${GODOT}" \
	--headless \
	--path "${TEST_ROOT}/godot" \
	--script "${ROOT_DIR}/scripts/godot/verify_forge_world.gd" \
	-- \
	"res://addons/forge_assets" >"${TEST_ROOT}/world-verification.log" 2>&1
cat "${TEST_ROOT}/world-verification.log"
grep -F 'PASS Forge world resources load headlessly:' "${TEST_ROOT}/world-verification.log" >/dev/null
if grep -Eq '^(ERROR:|SCRIPT ERROR:|FAIL )' "${TEST_ROOT}/world-verification.log"; then
	echo "Godot reported an error during world verification" >&2
	exit 1
fi

# The verifier must reject a saved TileSet that lost its terrain metadata,
# even when all native resource files still exist and load successfully.
TERRAIN_RESOURCE="${TEST_ROOT}/godot/addons/forge_assets/terrain/forge_terrain_set.tres"
cp "${TERRAIN_RESOURCE}" "${TEST_ROOT}/terrain-before-negative.tres"
sed 's/"forge_mask"/"broken_mask"/g' "${TEST_ROOT}/terrain-before-negative.tres" >"${TERRAIN_RESOURCE}"
negative_exit=0
"${GODOT}" --headless --path "${TEST_ROOT}/godot" \
	--script "${ROOT_DIR}/scripts/godot/verify_forge_world.gd" -- \
	"res://addons/forge_assets" >"${TEST_ROOT}/world-verification-negative.log" 2>&1 || negative_exit=$?
cp "${TEST_ROOT}/terrain-before-negative.tres" "${TERRAIN_RESOURCE}"
test "${negative_exit}" -ne 0
grep -F 'FAIL Forge world verification: Terrain custom-data layers are missing or invalid.' "${TEST_ROOT}/world-verification-negative.log" >/dev/null
! grep -F 'PASS Forge world' "${TEST_ROOT}/world-verification-negative.log" >/dev/null
printf 'PASS world verifier rejects missing terrain custom data without reporting success\n'

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
