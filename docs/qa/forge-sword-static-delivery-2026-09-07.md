# Sword static delivery contract verification — 2026-09-07

Worktree: `/Users/kartz/Development/Forge-sword-static-delivery`.
Branch: `codex/sword-static-delivery`, based on `b44e1d3`.
No Provider calls, credential access, or changes to the Sword project were used.

## Behavior

- Static Style sampling now reaches the Pack manifest, source metadata, Godot
  helper, installed prop Sprite2D, and `forge_usage.json`.
- Prop local origin uses the normalized ground line, leaving `canvas / 16` bottom
  padding. For a 64 px canvas this is `(32, 60)`; icons use `(32, 32)`.
- Usage retains stable item IDs and supplies installed `res://` texture paths for
  icon consumers. Legacy Packs keep centered sprites and inherited filtering.
- Pack validation rejects missing or conflicting new delivery metadata.

## Executed checks

- `cargo test -p core --test static_delivery_tests`: 3 passed; real-engine test
  intentionally selected separately below.
- `cargo test -p core --test static_delivery_tests -- --ignored --nocapture`:
  1 passed. Real Godot 4.6.3 at `/Applications/Godot.app/Contents/MacOS/Godot`
  installed linear props, nearest props, icons, and a legacy static Pack through
  the existing plan/job installer, then loaded the saved scenes and checked node
  filtering, geometry, and usage mappings.
- `cargo test -p pack`: 37 integration tests passed, including existing character
  timing/rendering contracts.
- `cargo fmt --all -- --check`: passed.

The engine check uses deterministic local rectangles and verifies resource delivery;
it does not establish production art quality or iPhone performance. Existing install
backup/rollback behavior is reused and was not redesigned in this change.

## Local PNG intake follow-up

Added `forge plan prepare-static --request ... --json`, followed by the existing
`plan execute`. The request accepts `icon_set`/`prop_set`, explicit sampling and
license, a 64–512 power-of-two canvas, and stable item IDs with local PNG paths.
Relative paths resolve from the request file. No Provider or generated Style Lock
is needed, and no Provider resolver is called for the operation.

The importer retains original PNGs and source/normalized SHA-256 evidence, checks
plan fingerprints before and after intake, preserves low-alpha and green pixels
without chroma-key matting, then reuses static Pack export and validation. Local
quality evidence explicitly says style consistency was not evaluated. The normal
Godot install registry and ownership rules apply; local intake does not add a
Forge asset-project catalog or targeted retry workflow.

Executed after the follow-up:

- `cargo test -p core --test prepare_static_tests --test static_delivery_tests`:
  6 passed, 1 separately selected engine test. New coverage includes green and
  semitransparent pixels with both sampling modes and both static kinds, license
  and source hashes, modified-plan inputs, duplicate IDs, invalid canvas, missing
  license, corrupt media, and opaque input rejection.
- `cargo test --workspace`: 248 passed, 1 ignored (the real-engine test above).
  Existing generated static, character, keyframe, and world fixture contracts
  passed; no real Provider calls were made.
- `cargo build -p forge-cli` and `cargo fmt --all -- --check`: passed.
- `python3 scripts/test-local-static-cli.py --forge
  /Users/kartz/Development/Forge-sword-static-delivery/target/debug/forge --godot
  /Applications/Godot.app/Contents/MacOS/Godot`: passed. Three cases covered linear
  props, nearest props, and linear icons, each with two distinct static items.
  The script checked the single-JSON CLI envelope, relative input paths, durable
  plan/job execution, zero Provider usage, correct static asset types and item IDs,
  source hashes, Pack validation, installed usage mappings, and saved Godot nodes.
- An earlier retained run is summarized in
  `artifacts/forge-local-static-20260907/summary.json`. Generated Job stores,
  fixture PNGs, and imported Godot caches remain local and are excluded from Git.

Development binary: `/Users/kartz/Development/Forge-sword-static-delivery/target/debug/forge`.
It reports `forge 0.2.1`; pin the development branch commit as well, because the
published v0.2.1 release does not contain `prepare-static`.
