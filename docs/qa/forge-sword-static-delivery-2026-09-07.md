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

## Real-source alpha bounds correction

Sword's five original Codex image_gen PNGs contain distant pixels with alpha 1–15.
Using every nonzero alpha pixel as the crop extent made the visible spirit, lantern,
and rock smaller and lifted them above the expected prop origin. Added optional
`foregroundAlphaThreshold` (default 1) and `edgePaddingPx` (default 0) to local
requests. The threshold selects bounds only; original alpha is retained inside the
padded crop, and original input files remain byte-identical in the Job store.

Executed `plan prepare-static` → `plan execute --wait` on all five actual originals
twice at a 256 px canvas: baseline 1/0 and explicit threshold/padding 16/16. Both
Packs validated, every Job reported zero Provider requests, and all original
SHA-256 values matched the Sword source manifest after both imports. Measured
visible output extents at alpha ≥16:

| Asset | Baseline | 16/16 | Visible bottom before → after |
| --- | --- | --- | --- |
| Spirit | 163 px high | 204 px high | 209 → 237 |
| Stone lantern | 161 px high | 204 px high | 218 → 237 |
| Moss rock | 165 px wide | 204 px wide | 203 → 237 |
| Cultivator | 207 px high | 206 px high | 238 → 238 |
| Jade sword | 210 px high | 206 px high | 240 → 238 |

The nominal normalized prop origin is y=240; retained source padding accounts for
the remaining 2–3 px gap. Source images were read and imported, not edited.
Evidence: `artifacts/forge-sword-alpha-bounds-20260907/summary.json`. Private source
PNGs and generated Job stores remain ignored local evidence.

`cargo test -p core --test prepare_static_tests --test static_delivery_tests` passed
7 tests after this correction (the independent real-engine test remained ignored
in this targeted invocation). The new regression covers distant alpha noise,
retained alpha-8 edge pixels within padding, and unchanged raw-source hashes.
`cargo build -p forge-cli` and `cargo fmt --all -- --check` passed. This follow-up
changes normalization only; the previously verified Godot metadata/installation
contract remains unchanged.
