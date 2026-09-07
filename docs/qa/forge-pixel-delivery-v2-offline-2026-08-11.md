# Forge Pixel Delivery V2 Offline Gate

Date: 2026-08-11

Status: P0-A implemented and fixture/offline gates passed. No real Provider
requests were made. `pixel-delivery-v2` is a core feature and is disabled by
default.

## Baseline preservation

The required worktree snapshot was saved before implementation:

`docs/qa/artifacts/forge-character-method-remediation-20260811/git-status-before.txt`

No unrelated uncommitted files were reverted or deleted. Portrait and World
Lanczos3 paths were left outside this phase.

## Implementation

- Added `packages/core/src/pixel_grid.rs`.
- Added deterministic `PaletteLockV1` derivation and sidecar writing under
  `.forge/palette/<revision>/palette-lock.json` when the feature is enabled.
- Added grid-aware delivery resize with explicit `fallback` reporting when no
  stable integer pixel grid is detected.
- Delivery alpha is binary (`0` or `255`), and RGB output is nearest-mapped to
  the locked palette.
- Added feature `pixel-delivery-v2`, default off.
- Explicit character-delivery branches use the new path only when the feature
  is compiled in; shared Portrait/World normalization keeps the existing
  Lanczos3 path, and the default build keeps every legacy branch.

## Files

- `packages/core/src/pixel_grid.rs`
- `packages/core/src/asset_project.rs`
- `packages/core/src/animation_sheet.rs`
- `packages/core/src/automation/runner.rs`
- `schemas/palette-lock.schema.json`
- `schemas/pixel-delivery-report.schema.json`
- `scripts/test-pixel-delivery.sh`

## Offline evidence

`scripts/test-pixel-delivery.sh` completed successfully on 2026-08-11:

- `cargo test -p core pixel_grid`: 5/5 passed.
- `cargo test -p core --features pixel-delivery-v2 pixel_delivery`: 1/1 passed.
- `cargo check -p core`: passed.
- `cargo check -p core --features pixel-delivery-v2`: passed.
- V6 focused Pack generation test: passed.
- V7 focused Pack generation test: passed.
- V8 focused Pack generation test: passed.

The feature-off compatibility proof is compile-time and behavioral: the new
delivery branch is behind `#[cfg(feature = "pixel-delivery-v2")]`, while the
same call sites retain the legacy `#[cfg(not(feature = "pixel-delivery-v2"))]`
Lanczos3 branch. The three focused V6/V7/V8 Pack generation tests all passed
with the feature disabled. No threshold was lowered.

## Scope boundary

- Analysis resampling paths in loop selection, source-cycle sampling, and
  pHash preparation were not changed.
- `packages/core/src/collection.rs` Portrait reframing was not changed.
- `packages/core/src/world.rs` was not changed.
- During final verification, a full disk interrupted the build. Only the
  recoverable Rust incremental/query cache was truncated/moved to Trash and
  rebuilt; no source, QA evidence, generated asset, or user file was removed.
- Real xAI validation was not run and is not claimed.
