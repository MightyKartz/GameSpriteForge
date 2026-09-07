# Ayla topdown-spritesheet V3 real xAI acceptance — 2026-08-08

## Verdict

`failed_safe`. Forge generated eight real 2×2 images—two attempts for each of
`idle`, `walk_up`, `walk_right`, and `walk_down`—but no action passed all hard
gates. No Character Pack was exported and no Godot installation was attempted.

The experiment still establishes a useful result: generating all four phases in
one image materially improves identity, palette, framing and direction
consistency, but `grok-imagine-image-quality` commonly collapses the four cells
to the same or nearly the same locomotion pose. This workflow is not yet a
production replacement for the existing path.

## Authorization and usage

- Provider/model: `xai` / `grok-imagine-image-quality`
- Authorization: `ayla-spritesheet-v3-four-directions-20260808`
- Allowed targets: `idle`, `walk_up`, `walk_right`, `walk_down`
- Limit: 8 requests / 6,400,000,000 cost ticks
- Actual: 8 requests / 5,600,000,000 cost ticks (approximately USD 0.56)
- Generated videos, Subject Locks, Style Locks and unrelated assets: zero

## Results

- `idle`: both sheets have good identity and exactly two visible boots, but the
  motion audit reports `foot_lobe_count_exceeded`. Manual inspection indicates a
  likely detector false positive caused by lower garment/cape contours rather
  than an actual third foot.
- `walk_up`: rear direction is correct. Some leg changes exist, but the cycle
  lacks four sufficiently distinct ordered phases. Attempt two also contains
  grid dividers and stronger cloak occlusion.
- `walk_right`: direction and identity are strong, but the four cells repeat an
  almost identical contact pose. The motion hard gate is correct.
- `walk_down`: attempt one violates cell-boundary safety; attempt two preserves
  the front view but repeats nearly identical phases. A blue trailing scarf
  detail also drifts from the locked amber scarf.

## Required remediation before another paid run

1. Make the extra-foot detector anatomy-aware or restrict its component search
   so cape/coat tips cannot become foot lobes. Do not simply lower the threshold.
2. Add explicit sheet-level phase evidence: left/right foot contact location,
   limb separation, ordered phase signatures, and pairwise pose distance.
3. Reject grid divider lines deterministically, including thin lines exactly on
   the 50% cell boundaries.
4. Strengthen the sheet prompt around phase-specific silhouette changes and
   forbid expression-only variation for locomotion.
5. Add an accessory/color lock for the amber scarf so the model cannot introduce
   a blue trailing scarf or wind effect.
6. Re-run all existing outputs locally after the detector fix. Only then request
   a separately authorized one-direction real probe; do not immediately buy
   another four-direction run.

## Safety audit

All four Jobs are `failed` with
`animation_sheet_regeneration_required`. No `gsfpack` or candidate Pack exists.
JobStore scanning found no OAuth/API credentials, bearer headers, Device Codes,
temporary media URLs or authorization headers.

Machine-readable evidence:
[`acceptance-summary.json`](artifacts/forge-character-spritesheet-v3-real-20260808/acceptance-summary.json).
