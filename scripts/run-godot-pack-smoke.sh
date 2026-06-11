#!/usr/bin/env bash
set -euo pipefail

ROOT="/Users/kartz/Development/Forge"
ARTIFACT_ROOT="$ROOT/docs/qa/artifacts/godot-pack-smoke-$(date +%Y%m%d-%H%M%S)"
GODOT="${GODOT_BIN:-/Applications/Godot.app/Contents/MacOS/Godot}"

mkdir -p "$ARTIFACT_ROOT"
cargo run --manifest-path "$ROOT/Cargo.toml" -p core --example generate_godot_smoke_pack -- "$ARTIFACT_ROOT" | tee "$ARTIFACT_ROOT/generate.log"

PACK_DIR="$(awk -F= '/^PACK_DIR=/{print $2}' "$ARTIFACT_ROOT/generate.log")"
PROJECT_DIR="$ARTIFACT_ROOT/godot-project"
mkdir -p "$PROJECT_DIR/addons/game_sprite_forge"
cp "$ROOT/scripts/godot/import_forge_pack.gd" "$PROJECT_DIR/addons/game_sprite_forge/import_forge_pack.gd"
cat > "$PROJECT_DIR/project.godot" <<'PROJECT'
config_version=5

[application]

config/name="Forge Godot Smoke"
config/features=PackedStringArray("4.6")
PROJECT

"$GODOT" --headless --path "$PROJECT_DIR" --script "$PROJECT_DIR/addons/game_sprite_forge/import_forge_pack.gd" -- "$PACK_DIR" | tee "$ARTIFACT_ROOT/godot.log"
grep -q "PASS Forge Godot import smoke" "$ARTIFACT_ROOT/godot.log"
echo "PASS Godot pack smoke: $ARTIFACT_ROOT"
