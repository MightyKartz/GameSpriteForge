# Forge `topdown-grid@9.2.0` structured-gait implementation plan

Date: 2026-08-12  
Status: implemented and offline-accepted  
Real Provider scope: **forbidden in this goal**

## Decision

Keep the user-approved V9.0 `DirectionGridLock` immutable. V9.2 is a new,
validation-only workflow version because V9.1 real acceptance proved that four
raster-distinct frames can still encode the same contacting leg in every
frame. V9.1 reports remain valid historical evidence but are not reusable as
V9.2 action evidence.

The first V9.2 release surface accepts only one `walk_down` validation probe.
It cannot generate `walk_up`, `walk_right`, or `walk_left`, export a Pack, run
Godot installation, or call a video model. Multi-direction support remains
blocked until one later explicitly authorized real `walk_down` run passes
native review.

## V9.2 contract

```text
approved V9.0 DirectionGridLock/front_idle
├── frame 0: screen-left contact  + grayscale PoseStructure
├── frame 1: screen-left support  + grayscale PoseStructure
├── frame 2: screen-right contact + grayscale PoseStructure
└── frame 3: screen-right support + grayscale PoseStructure
    ↓
static identity/equipment/direction/guide-leak gates
    ↓
motion-semantics@1.1.0 + gait-laterality@1.0.0
    ↓
native review only; no partial Pack
```

### Spatial phase semantics

Prompts and reports use viewer-space `screen-left` and `screen-right`, not the
ambiguous phrase "the character's left/right". For the initial front/down
view, frames 0 and 1 must have a screen-left contact/support extreme; frames 2
and 3 must have the screen-right extreme. Opposite pairs must have opposite
signed foot-baseline evidence.

`gait-laterality@1.0.0` records per-frame lower-body bounds, screen-half bottom
extrema, contact signal, expected and observed spatial side, confidence, match
state, action verdict, reasons, and exact retry frames. A collapsed same-side
cycle fails with stable reason `walk_laterality_not_alternating`; ambiguous
foot evidence fails closed. The V9.1 real rejected frames are frozen as a
negative regression fixture.

### PoseStructure

Every V9.2 request carries a deterministic transparent grayscale structure
image. It contains no cyan/magenta pixels, labels, floor line, cell boundary,
or appearance content. Fresh generation references are:

1. `DirectionAnchor`;
2. `PoseStructure`.

A general motion-only retry references:

1. immutable failed `EditTarget` snapshot;
2. `DirectionAnchor`;
3. the same phase's `PoseStructure`.

The structure path and SHA-256 are stored per frame, in the WorkflowGraph and
Provider manifest. A structure validator rejects colored/opaque-panel guides;
an output leak gate rejects copied guide ink before action quality can pass.
Providers must advertise at least three image references before any V9.2
request, preserving the retry ceiling.

After real Job `0048ce3b-a0ea-4a63-8ed9-9063d01e9043` proved that feeding a
wrong-side raster back as `EditTarget` preserved its leg geometry and reduced
sharpness, a **laterality-only** retry now uses the fresh two-reference form:

1. `DirectionAnchor`;
2. the selected phase's `PoseStructure`.

It is recorded as `laterality_fresh_retry`, never writes `inputFrameSha256`,
and never materializes `edit-target.png`. Generic motion failures retain the
three-reference diagnostic edit; fresh retry is selected only when motion is
already `game_ready` and the explicit laterality gate owns the failure.

### Retry policy

Laterality owns retry selection whenever generic motion is already
`game_ready`. A same-side collapse selects only the failed opposite-half
frames. Retry never changes frame 0 or 1 unless a separate static gate
identifies that exact frame.

The initial validation Job retains four expected / eight maximum image edits.
After it ends in stable `walk_laterality_not_alternating`, a child validation
is permitted only when the immutable Action and laterality reports recommend
exactly one frame. The child must select that same frame, uses one expected /
one maximum request, byte-reuses the other three frames, and exposes only
`walk_down:frame:<selected>` to authorization. Wrong-frame, multi-frame,
non-laterality, unbound, or hash-changed sources fail during Plan preparation.
If the selected source frame is already `laterality_fresh_retry`, another
child is rejected: the fresh allowance is single-use even when the model still
chooses the wrong side.

New V9.2 reports use `grid-keyframe-action-report@1.2.0`. The former
`@1.1.0` remains readable for the frozen real failure evidence, but only
`@1.2.0` may emit `laterality_fresh_retry`.

## Deterministic pair-transform experiment

V9.2 also evaluates a zero-Provider lower-body mirror transform on the frozen
real frames. The transform must preserve every upper-body pixel byte-for-byte,
flip the measured contact signal, retain a single subject and two-foot
silhouette, and emit a seam/geometry report. It remains an experiment and may
not silently replace Provider output. Native review decides whether it is a
future seed-pair optimization; a poor garment/leg seam keeps it disabled.

## Budget and delivery boundary

- validation estimate: 4 image edits;
- validation maximum: 8 image edits;
- authorization targets: four `walk_down:frame:*` targets only;
- laterality child: one recommended `walk_down:frame:*` target, 1/1 request;
- videos: 0;
- Pack/Godot: prohibited for V9.2 validation;
- real calls: 0 during this implementation goal.

## Offline acceptance

1. Current V9.1 real rejected frames fail `gait-laterality@1.0.0` and recommend
   only frames 2 and 3.
2. Synthetic alternating contact/support frames pass; ambiguous and extra-foot
   fixtures fail closed.
3. All four grayscale PoseStructures are distinct, neutral-only, transparent,
   structurally valid, and hash-bound to requests/reports/WorkflowGraph.
4. Provider reference order is exact for fresh and diagnostic requests;
   insufficient/unknown reference capacity fails before use.
5. Guide leakage fails the static frame gate.
6. V9.2 Plan accepts only validation-only `walk_down`, estimates 4/8, exposes
   no video/Pack/Godot effect and rejects V9.1 action-report reuse.
7. Failure Jobs preserve laterality, motion, equipment, raw attempts, usage,
   manifest and WorkflowGraph evidence.
8. The deterministic mirror experiment proves or rejects signal inversion and
   upper-body byte preservation without entering the production route.
9. Focused Grid tests, applicable workspace tests, formatting, lint, CLI
   product isolation, credential scan and `git diff --check` pass offline.

Only after these gates pass may a separate Goal propose one paid V9.2
`walk_down` validation probe. That later Goal still requires explicit user
authorization.
