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
