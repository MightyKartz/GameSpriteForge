# Forge external DirectionGrid import plan

Status: implemented and validated with the 2026-08-15 Codex built-in imagegen
candidate.

## Objective

Allow a caller to evaluate a square external 2×2 front/rear/right/left sheet
without attributing its pixels to the configured media Provider and without
mutating an approved V9 DirectionGrid source.

## Contract

- New operation: `import_direction_grid`, schema version `1`.
- CLI: `forge job import-direction-grid`.
- Source: one succeeded and approved `topdown-grid@9.0.0` DirectionGrid Job.
- Input: one even square PNG, 128–4096 px, stored outside the source Job.
- Provider request estimate / maximum: `0 / 0`.
- Imported Job never inherits the source authorization or approval.
- Parent and lineage bind to the approved source; source Job, Lock, approval,
  Subject/Style inputs and the external sheet are input-fingerprinted.
- A changed input fails before local processing.

## Deterministic processing

`checkerboard-sheet-matting@1.0.0` accepts native alpha or a light-neutral
checkerboard. For opaque checkerboards it:

1. requires a strong two-tone bright-neutral border signature;
2. clears only qualifying pixels reachable from the outer border;
3. applies two narrow neutral-fringe passes adjacent to cleared pixels;
4. requires a transparent border, 5–60% foreground coverage, exactly four
   significant components and no residual exposed neutral edge pixels.

The matted sheet then enters the existing fixed DirectionGrid extraction.
Every cell passes ordinary background cleanup. All four raw cells use one
shared scale and translation-only foot alignment; scale drift above 20%, center
drift above 2 px or baseline drift above 2 px is rejected. Identity, equipment,
hand state and appearance gates run against the immutable Subject canonical.

## Evidence and delivery boundary

Successful materialization writes:

- `direction-grid-import@1.0.0` provenance;
- `checkerboard-sheet-matting@1.0.0` report;
- shared alignment report;
- four normalized PNGs, contact sheet, appearance report and review Lock;
- zero-request Provider usage and manifest evidence.

All imported nodes record `providerRequestOccurred=false`. The Job stops at
`awaiting_review`; it creates no approval, Pack, catalog entry or Godot asset.
The Lock retains the approved downstream Provider/model binding solely for
future compatibility, while import evidence and per-node request flags remain
the producer authority.

An explicit later acceptance re-hashes every imported node, original/matted
sheet, matting/alignment report, import evidence, Provider manifest and
zero-request usage record. It also rechecks the approved source Lock and all
hard matting/alignment thresholds before writing DirectionGridApproval.
