# Forge `topdown-frames@4.0.0` offline acceptance

Date: 2026-08-09

Result: **PASS for offline/fixture implementation; remains experimental pending separately authorized real xAI acceptance.**

Machine summary: [summary.json](artifacts/forge-topdown-frames-v4-offline-20260809/summary.json)

## What was accepted

- A new full Character plans 5 image requests and caps at 10: one four-view DirectionLock sheet plus four action sheets.
- DirectionLock cell order is deterministic: front, rear, right, left.
- The DirectionLock request uses Subject identity and optional equipment; action requests use the selected Direction anchor and optional equipment.
- The Style board image is absent from action and frame-repair requests. Style metadata remains text-only.
- Each action produces four complete raster sprites. Forge uses one shared scale per sheet, translation-only foot alignment, and `onion-skin@1.0.0` overlays/reports.
- A malformed DirectionLock sheet consumes at most its two authorized attempts, makes no action request, and exports no Pack.
- A single-frame child retry makes exactly one fixture Provider request, authorizes only `walk_right:frame:2`, and preserves frames 0, 1, and 3 byte-for-byte.
- The Character Pack validates as V2, carries a portable four-view DirectionLock, and contains no local Job paths.
- Godot 4.6.3 installs the Pack as external PNGs plus native `SpriteFrames` and `AnimatedSprite2D`; the project imports and loads headlessly.

## Regression evidence

- `cargo test --workspace`: 333 passed, 0 failed.
- The workspace run includes legacy video, keyframe/keypose, `topdown-spritesheet@3.0.0`, static collections, world assets, Provider OAuth/API contracts, Pack, CLI and the new V4 contract.
- `scripts/test-cli-product.sh`: PASS.
- `scripts/test-consistency-v2.sh`: PASS.
- `cargo fmt --all -- --check`: PASS.
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS.
- `git diff --check`: PASS.

## Security and provenance

- No real Provider call was made.
- The fixture JobStore security contract found no bearer header, access/refresh token, Device Code, or xAI API key material.
- Direction and action manifests record Provider/model, request usage, reference roles and SHA-256 values without credentials or temporary URLs.
- Portable DirectionLock files rewrite all anchor paths into `assets/direction-lock/*.png`.

## Remaining release gate

This result validates orchestration and deterministic processing, not xAI's visual behavior. Promotion from `experimental` requires a separately authorized, budgeted real-model acceptance run. The first real gate should use one Character and visually inspect the four DirectionLock anchors before allowing the four action requests; failed anchors must stop the run.

