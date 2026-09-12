#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GODOT="${GODOT_BIN:-}"
if [[ -z "$GODOT" ]]; then
  GODOT="$(command -v godot || command -v godot4 || true)"
fi
if [[ -z "$GODOT" && -x /Applications/Godot.app/Contents/MacOS/Godot ]]; then
  GODOT="/Applications/Godot.app/Contents/MacOS/Godot"
fi
if [[ -z "$GODOT" ]]; then
  echo "Godot is required; install Godot or set GODOT_BIN." >&2
  exit 69
fi

# Full Packs and imported Godot projects are temporary QA products, not Git evidence.
mkdir -p "$ROOT/target/qa"
ARTIFACT_ROOT="$(mktemp -d "$ROOT/target/qa/godot-pack-smoke-XXXXXX")"
cargo run --locked --manifest-path "$ROOT/Cargo.toml" -p core --example generate_godot_smoke_pack -- "$ARTIFACT_ROOT" | tee "$ARTIFACT_ROOT/generate.log"

PACK_DIR="$(awk -F= '/^PACK_DIR=/{print $2}' "$ARTIFACT_ROOT/generate.log")"
PROJECT_DIR="$ARTIFACT_ROOT/godot-project"
mkdir -p "$PROJECT_DIR/addons/game_sprite_forge"
cp "$ROOT/scripts/godot/import_forge_pack.gd" "$PROJECT_DIR/addons/game_sprite_forge/import_forge_pack.gd"
cp "$ROOT/scripts/godot/install_forge_pack.gd" "$PROJECT_DIR/addons/game_sprite_forge/install_forge_pack.gd"
cp "$ROOT/examples/godot/forge-import-smoke/project.godot" "$PROJECT_DIR/project.godot"
cp "$ROOT/examples/godot/forge-import-smoke/verify_installed_frames.gd" "$PROJECT_DIR/verify_installed_frames.gd"

"$GODOT" --headless --path "$PROJECT_DIR" --script "$PROJECT_DIR/addons/game_sprite_forge/import_forge_pack.gd" -- "$PACK_DIR" 2>&1 | tee "$ARTIFACT_ROOT/godot.log"
grep -q "PASS Forge Godot import smoke" "$ARTIFACT_ROOT/godot.log"
"$GODOT" --headless --path "$PROJECT_DIR" --script res://verify_installed_frames.gd -- \
  res://imported/Godot_Smoke_Walk/Godot_Smoke_Walk.spriteframes.tres \
  "$PACK_DIR/assets/manifest.json" 2>&1 | tee "$ARTIFACT_ROOT/frames.log"
grep -q "PASS installed Godot SpriteFrames" "$ARTIFACT_ROOT/frames.log"
for log in "$ARTIFACT_ROOT/godot.log" "$ARTIFACT_ROOT/frames.log"; do
  if grep -Eq '^(ERROR:|SCRIPT ERROR:|FAIL )' "$log"; then
    echo "FAIL Godot reported an error; see $log" >&2
    exit 1
  fi
done
echo "PASS Godot pack smoke: $ARTIFACT_ROOT"
