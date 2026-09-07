#!/bin/zsh
set -eu
review_dir="${0:A:h}"
review_engine="${FORGE_GODOT_PATH:-/Applications/Godot.app/Contents/MacOS/Godot}"
# A source-project delivery has no .godot cache. Import external PNGs first.
"$review_engine" --headless --editor --import --path "$review_dir/godot-project"
exec "$review_engine" --path "$review_dir/godot-project" "$@"
