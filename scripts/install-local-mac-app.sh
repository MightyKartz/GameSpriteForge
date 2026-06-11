#!/usr/bin/env bash
set -euo pipefail

source_app="${FORGE_SOURCE_APP:-target/release/bundle/macos/Game Sprite Forge.app}"
dest_app="${FORGE_DEST_APP:-/Applications/Game Sprite Forge.app}"

if [[ ! -d "$source_app" ]]; then
  echo "Source app bundle not found: $source_app" >&2
  echo "Run npm --workspace apps/mac run tauri -- build --bundles app first." >&2
  exit 1
fi

osascript -e 'tell application "Game Sprite Forge" to quit' >/dev/null 2>&1 || true
sleep 1

mkdir -p "$(dirname "$dest_app")"
rsync -a --delete "$source_app/" "$dest_app/"
xattr -dr com.apple.quarantine "$dest_app" >/dev/null 2>&1 || true

echo "Installed latest Game Sprite Forge bundle to $dest_app"
