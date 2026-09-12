# Sprite Sheet Intake Contracts

Updated: 2026-09-12. The former desktop followup plan has been replaced by the
CLI intake contract below. Animation preparation remains experimental.

## Ownership and inputs

`packages/core/src/video/sprite_sheet.rs` owns fixed-grid and transparent-gutter
slicing. The CLI runner calls those same functions from a `sprite_sheet` input
in a `prepare-asset` or `prepare-character` request. The request types live in
`packages/core/src/automation/types.rs`.

Use [the CLI protocol](../automation/forge-cli.md#local-animation-contracts) and
[the animation guide](../../.agents/skills/forge-use/references/animation.md) for
complete requests, preservation options, quality review, and Godot delivery.

## Fixed grid

A `fixed_grid` split supplies `frameWidth`, `frameHeight`, `columns`, and `rows`.
Dimensions must be positive and fit the source sheet. Frames are extracted in
row-major order. Optional whole-sheet padding and offsets are explicit request
fields; transformations that would discard nontransparent pixels are rejected.

## Transparent gutters

A `transparent_gutters` split takes `alpha_threshold` (default `0`) and
`min_gap_px` (default `1`). A pixel at or below the alpha threshold counts as
transparent for boundary detection. Gap lengths are clamped to at least one
pixel.

The slicer:

1. Finds qualifying transparent row gaps.
2. Finds qualifying transparent column gaps within each resulting row band.
3. Ignores regions without foreground and orders the remaining regions by row,
   then column.
4. Copies each region without interpolation into a shared canvas sized to the
   largest extracted width and height, centered horizontally and aligned at the
   bottom.
5. Writes `raw/frame_00001.png` onward into the Job directory.

An entirely transparent sheet is rejected. A sheet without useful transparent
gutters needs an explicit fixed grid to obtain the intended frame boundaries.
The two slicing modes feed the normal normalization, quality, and Pack pipeline.

## Verification

Run the focused Rust contracts from the repository root:

```bash
cargo test -p core sprite_sheet_transparent
cargo test -p core --test automation_tests --test animation_delivery_tests
```

[The original split evidence](../qa/transparent-gutter-sprite-split-evidence-2026-06-11.md)
records the 2026-06-11 implementation. Its desktop checks are historical.
