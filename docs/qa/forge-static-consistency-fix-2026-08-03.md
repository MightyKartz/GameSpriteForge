# Forge Static Consistency Fix and Real-Asset Replay — 2026-08-03

## Outcome

The real xAI Icon Set and Prop Set assets from the initial acceptance were replayed with
the release CLI without generating new media. Both repaired Packs validate and load in
Godot `4.6.3.stable`.

- Style baseline: `style-baseline@2.3.0`
- Consistency report: `consistency@1.3.0`
- Final Style revision: `597c90fd50d6c333`
- Provider requests for migration + both replays: **0**
- Existing thresholds were not lowered.

## Root cause and implementation

The legacy Style baseline treated the opaque style-board background as 86.5% of the
palette. A single asset was then compared symmetrically against the full board color
distribution, so valid foreground colors were reported as palette drift.

The repair:

1. Estimates the dominant border color and removes only its border-connected background
   before extracting the Style palette.
2. Stores up to 24 foreground colors and scores candidate-to-Style color support
   directionally, so an item need not reproduce every color used by other asset types.
3. Preserves the established edge-density scale and all pass/review/regenerate thresholds.
4. Includes `style-baseline@2.3.0` in the Style revision hash. The old lock remains immutable.
5. Reuses the SHA-256-verified legacy style board during migration instead of calling the
   Provider, and records `migratedFromRevision`.
6. Adds `forge job retry --stage consistency` for zero-cost static replay. It copies
   normalized pixels, recalculates reports, and exports only after the normal gate/review.
7. Rechecks non-targeted reused items, including prior hard failures, so one targeted retry
   no longer aborts merely because a different item also failed.

## Real release-build replay

### Style migration

- Release Job: `23cb45a4-41fe-4f94-9815-2ab9dee1141e`
- Old revision: `f17362d90094f516`
- New revision: `597c90fd50d6c333`
- Style-board SHA-256 remained
  `5a0b3110565594606afa099a7bb1874e6309d7a88295f92a26a6d82ec0515d43`.
- Provider usage: 0 requests, 0 generated images.

### Icon Set

- Replay Job: `a3df022a-ee99-4093-ae78-031b9728838e`
- Before: two `regenerate`, three `awaiting_review`, no Pack.
- After automatic replay: four `game_ready`, one `awaiting_review`, no hard failure.
- Coin Pouch was explicitly accepted in the allowed edge-density gray zone; all palette,
  alpha, canvas, bounds, scale, subject-count, and anchor checks passed.
- Final Pack verdict: `game_ready`.

| Item | Palette overlap | Edge ratio | Automatic verdict |
|---|---:|---:|---|
| Healing Potion | 0.927 | 1.066 | `game_ready` |
| Mana Crystal | 0.910 | 1.090 | `game_ready` |
| Brass Key | 0.990 | 1.139 | `game_ready` |
| Coin Pouch | 0.993 | 0.632 | `awaiting_review` |
| Ancient Scroll | 0.962 | 1.014 | `game_ready` |

- [Icon contact sheet](artifacts/forge-static-real-20260803/job-store/a3df022a-ee99-4093-ae78-031b9728838e/exports/forest-inventory-icons/contact-sheet.png)
- [Icon preview](artifacts/forge-static-real-20260803/job-store/a3df022a-ee99-4093-ae78-031b9728838e/exports/forest-inventory-icons/preview.gif)
- [Icon Pack manifest](artifacts/forge-static-real-20260803/job-store/a3df022a-ee99-4093-ae78-031b9728838e/exports/forest-inventory-icons/forest-inventory-icons.gsfpack/forgepack.json)

### Prop Set

- Replay Job: `93ed1e91-18ca-4376-bc11-bb24e1ac4ce4`
- After automatic replay: three `game_ready`, two `awaiting_review`, no hard failure.
- Supply Crate and Travel Barrel were explicitly accepted in the permitted edge-density
  gray zone. The remaining hard gates and palette checks passed.
- Final Pack verdict: `game_ready`.

| Item | Palette overlap | Edge ratio | Automatic verdict |
|---|---:|---:|---|
| Supply Crate | 0.988 | 1.264 | `awaiting_review` |
| Travel Barrel | 0.992 | 1.289 | `awaiting_review` |
| Campfire Ring | 0.980 | 1.058 | `game_ready` |
| Trail Signpost | 0.991 | 0.905 | `game_ready` |
| Bedroll Pack | 0.992 | 0.977 | `game_ready` |

- [Prop contact sheet](artifacts/forge-static-real-20260803/job-store/93ed1e91-18ca-4376-bc11-bb24e1ac4ce4/exports/forest-camp-props/contact-sheet.png)
- [Prop preview](artifacts/forge-static-real-20260803/job-store/93ed1e91-18ca-4376-bc11-bb24e1ac4ce4/exports/forest-camp-props/preview.gif)
- [Prop Pack manifest](artifacts/forge-static-real-20260803/job-store/93ed1e91-18ca-4376-bc11-bb24e1ac4ce4/exports/forest-camp-props/forest-camp-props.gsfpack/forgepack.json)

## Godot and security verification

- Icon install Job: `1740344c-e864-4c44-8ad8-e5211ff3e7a6`
- Prop install Job: `9e87a482-3a81-4f84-be4e-0ae01c774cd1`
- Godot loaded all five icon PNGs as `Texture2D` resources.
- Godot loaded and instantiated all five prop scenes with textured `Sprite2D` children.
- No `.tres`/`.tscn` is at or above 1 MiB.
- No generated text resource contains `PackedByteArray`, embedded Image data, or
  `ImageTexture.create_from_image`.
- JobStore and Godot output contain no bearer credential, access/refresh token, device
  code, or video data URL.

## Regression gates

- `cargo fmt --all -- --check`: passed.
- Full workspace Rust tests: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `scripts/test-cli-product.sh`: passed, including zero-request replay and multi-failure
  targeted retry.
- `scripts/test-cli-installer.sh`: passed.
- `scripts/test-cli-signing-contract.sh`: passed.
- Release CLI Pack validation and Godot resource loading: passed.
- Review-decision and post-review consistency-report SHA-256 values match their Job artifact
  records.

## Remaining product gate

This repairs the deterministic false-negative defect and proves the replay path with real
xAI assets. It does not replace the planned 20-set, five-style commercial benchmark. Static
assets may be described as supporting hard gates plus explicit gray-zone review; a no-review
success-rate claim still requires that larger benchmark.
