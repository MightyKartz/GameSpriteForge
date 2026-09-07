# Forge Character DirectionLock and background cleanup V2.3

Date: 2026-08-08

## Problem

The real `topdown-keyframes@2.2.0` four-direction run proved that reference
isolation and explicit equipment removed the Style-board/staff leak, but it also
showed two different propagation failures:

1. `walk_up` and `walk_right` could remain front-facing because direction was a
   prompt-only property and was checked only after the expensive collection had
   been generated.
2. A Provider-created opaque blue/gray background could survive the combined
   matte/normalize operation and then become an adjacent-frame reference.

Changing thresholds cannot repair either causal boundary. V2.2 remains readable
and executable; the corrected behavior is versioned as
`topdown-keyframes@2.3.0`.

## Fixed workflow

```text
SubjectLock + explicit equipment + direction guide
  -> provider raw image (hashed, immutable)
  -> keyframe-background-cleanup@1.1.0
  -> keyframe-normalize@1.0.0
  -> direction-anchor@1.2.0
  -> DirectionLockV1 (frame 0 for front/rear/right)
  -> same-direction anchors 2/4/6
  -> cleaned adjacent-frame edits 1/3/5/7
  -> existing hard defects, consistency, loop and shared normalization
  -> Pack V2 + portable DirectionLock provenance
  -> Godot external textures + forge_usage.json provenance
```

## DirectionLock contract

- `idle` and `walk_down` use the `front` contract.
- `walk_up` uses `rear`: the rear hood/torso must be visible and a readable
  frontal face must not be present.
- `walk_right` uses `right`: the warm upper-face centroid must be displaced to
  screen-right and the body must remain present.
- Frame 0 establishes the direction anchor using an already authorized frame
  target. The normal request estimate remains 32 and the maximum remains 64.
- `walk_down` reuses the front DirectionLock created by `idle`; a one-direction
  validation Job creates only the selected direction entry.
- Every frame, not only frame 0, crosses the direction assessment. A direction
  anchor that fails two attempts stops the Job before frames 1-7 are purchased.
- The first accepted frame of each direction establishes a direction-local edge
  and appearance baseline. Correct rear/profile geometry is not compared against
  the front-facing Subject edge density.

The deterministic right-facing metric is deliberately a conservative proxy, not
a claim of general pose recognition. Real-provider promotion still requires
native-size review and a separately authorized xAI acceptance.

## Background boundary

Each Provider response produces three independently hashed artifacts:

1. `source.png`: immutable Provider output and cache object.
2. `background-cleanup.png` plus a versioned JSON report.
3. the normalized `keyframes/<action>/frame-XX.png` used downstream.

The cleanup preserves useful native Alpha. Opaque inputs use border-connected
chroma removal, reject foreground touching the canvas border, retain the central
subject component and nearby equipment satellites, and hard-block missing
foreground, residual border opacity, or implausible coverage. It never crops or
rescales; normalization is a separate local node. Only the normalized output can
become a DirectionLock or an adjacent-frame reference.

## WorkflowGraph and retry

V2.3 records separate nodes:

- `provider_image:<action>:<frame>` — the only paid node;
- `background_cleanup:<action>:<frame>` — deterministic and local;
- `frame_image:<action>:<frame>` — deterministic normalization and the stable
  compatibility ID used by retry/replay.

A single-frame retry reuses the source Job's DirectionLock and unaffected cleaned
frames, issues exactly one Provider request, and invalidates only downstream local
nodes. `matting`, `loop`, `consistency`, and graph replay remain zero-cost. A
failed direction anchor writes a partial Provider manifest, DirectionLock,
consistency report, and WorkflowGraph before returning
`direction_lock_required`.

## Pack and Godot

Successful Character Pack V2 output contains
`character-direction-lock.json`. Job-local absolute paths are rewritten to
portable `assets/frames/frame_NNN.png` paths before Pack validation. The Godot
installer exposes the DirectionLock and cleanup profile in `forge_usage.json`;
textures remain external and existing text-resource size/embedded-image gates are
unchanged.

## Compatibility and security

- V2.2 behavior is not retroactively changed and is covered by an executable
  compatibility test.
- Existing `frame_image:*` node IDs, Job recipes, plan tokens, Provider/model
  locking and request authorization targets are retained.
- No Token, API key, Device Code, Authorization header or temporary media URL is
  added to the new schemas, reports, Pack or Godot output.
- This implementation and its acceptance are fixture-only; it performs no real
  xAI call.
