# Forge Character Direction Motion V8

Status: implemented as the experimental workflow `topdown-direction-motion@8.0.0`.
V7 and every earlier workflow remain unchanged. Real xAI acceptance has not been
run for V8; the current gate is fixture, zero-cost replay, Pack, and Godot 4.6.

## Product contract

Godot does not require a video, skeleton, ControlNet, or extension. It requires
a `SpriteFrames` resource containing named animations, ordered textures, loop
flags, and per-frame duration multipliers. V8 therefore delivers exactly eight
animations:

```text
idle_down  idle_up  idle_right  idle_left
walk_down  walk_up  walk_right  walk_left
```

Left is a real visual direction in V8. Godot does not mirror `walk_right` or
`idle_right` for left-facing playback.

## Immutable visual graph

```text
prompt (+ optional StyleLock / descriptive SubjectLock)
└── front_idle                         text-to-image canonical truth
    ├── back_idle                      image edit from front_idle
    │   └── back_walk                  image edit from back_idle + front_idle
    │       └── walk_up video
    ├── right_idle                     image edit from front_idle
    │   └── right_walk                 image edit from right_idle + front_idle
    │       └── walk_right video
    ├── left_idle                      image edit from front_idle
    │   └── left_walk                  image edit from left_idle + front_idle
    │       └── walk_left video
    └── front_walk                     image edit from front_idle + front_idle
        └── walk_down video
```

The first paid image request is `front_idle`; an older SubjectLock image is not
silently used as its pixels. A SubjectLock is optional for V8 and, when present,
only supplies a reviewed description and provenance. Other workflows continue
to require their existing SubjectLock.

Every node records role, direction, animation, parent node/hash, canonical
node/hash, delivery image/hash, generation master/hash, attempt, and generic
visual feedback. `video_input()` rejects a direction/parent/hash mismatch before
a Provider call. This prevents `walk_up` from accidentally receiving a front or
cape-less input.

`front_idle` is additionally gated as the canonical resting reference. For
`biped_walk@1.0.0`, a candidate with no readable support, fragmented support,
or a lower-body span/centroid pattern that indicates stepping is rejected as
`canonical_idle_*`. Direction and walk-pose edits compare components,
occupancy, and upper-body silhouette against their immediate parent; losing a
permanent covering or splitting permanent structure is reported as
`parent_permanent_structure_loss` or `parent_component_structure_drift`. These
checks are structural and never name a cape, species, costume, limb count, or
prop.

Head and shoulder covering feedback is intentionally generative rather than a
hard prohibition. If a parent shows a broad permanent head or shoulder/body
covering and the direction edit weakens that silhouette, Forge records
`parent_head_covering_loss` or `parent_shoulder_covering_loss` and turns it
into an explicit repair instruction for the next edit. The instruction tells
the Provider to restore the permanent covering from the canonical reference
while preserving the requested view; it does not claim that every subject must
have such coverings.

The same guidance now covers the complete lower extent of a permanent covering.
`parent_lower_covering_loss` means the edit shortened a full covering into a
collar, shoulder-only layer, or otherwise lost its hemline/lower-body extent.
The retry instruction asks for the full shape, length, hemline, and occlusion
relationship instead of merely asking for "more coverage".

V8 image materialization uses the deterministic keyframe background cleanup
before normalization. It removes border-connected scene/background remnants
and small line-like chroma residue, while protecting opaque green components
that are far enough from the sampled background color to be legitimate
clothing or equipment. A failed cleanup retries the Provider candidate with a
clean-canvas correction; it does not silently normalize a green-fringed lock
image.

## Mandatory two-stage authorization

Image and video work are different Jobs and different cost grants.

1. `image_locks` normally costs 8 image requests, at most 16. It always stops in
   `awaiting_review`; it cannot export a Pack or request video.
2. `forge job review --accept` writes `direction-motion-approval@1.0.0`, bound to
   the source Job, complete Lock SHA-256, and all eight node SHA-256 values.
3. `complete` requires `--image-lock-job <approved-job-id>`. It copies and
   re-hashes the approved Lock, makes exactly four I2V requests (at most eight),
   and never regenerates an image. Each video request receives the byte-exact
   typed walk-pose generation master for its animation; Forge does not
   composite the approved image onto an adaptive or chroma background before
   image-to-video.
   `--validation-animation walk_*` narrows this stage to exactly one video
   request (at most two), performs the same local extraction/quality checks,
   and never exports a Pack.

The request-level default is `image_locks`, not merely the CLI default. Missing,
rejected, moved, or modified approval evidence fails closed.

## Targeted image-lock retry

A rejected source Job remains immutable. `forge job review --id <job> --reason
"..."` without `--accept` records the rejection. A child Job can then retry a
node by animation name and `--stage still`; `idle_up` regenerates only
`back_idle` and its dependent `back_walk`, while `walk_right` regenerates only
`right_walk`. Retrying `idle_down` invalidates all seven derived nodes because
the canonical front image changed.

The retry uses the current canonical parent as the edit target and the rejected
prior image only as negative evidence, together with the human review note.
The prompt explicitly tells the Provider not to copy missing or weakened head,
shoulder, or body coverings from the rejected attempt.
Unchanged nodes are copied and re-hashed, no video request is issued, and the
new lock again stops at `awaiting_review`.

## Provider-neutral feedback

Hard integrity failures remain universal: missing subject, multiple subjects,
cropping, corrupt media, invalid Alpha, and a broken lineage/hash. Feedback for
visual drift is relative to the immediate parent and reports palette, scale, and
foreground-structure changes. It does not require a cape, two legs, human skin,
a staff, a species, or a fixed costume. Motion profiles are optional prompt and
diagnostic guidance:

- `freeform_motion@1.0.0`
- `biped_walk@1.0.0`
- `quadruped_walk@1.0.0`
- `flying_cycle@1.0.0`
- `slither_cycle@1.0.0`

Provider retries receive the prior generic reason codes. A last soft mismatch
becomes reviewable; a corrupt/cropped/multi-subject result does not.

## Video extraction and motion preservation

V8.1 decodes candidate frames by source presentation timestamp (PTS), retaining
the native source cadence up to 24 FPS and 120 frames. It does not re-time
frames through `fps=`. Cycle detection runs before output reduction over the
complete retained clip; the first second has no privileged status. The selected
interval is `[start, end-boundary)`: the boundary proves closure but is not
duplicated.

After the complete interval is known, `source-cycle-sampling@1.0.0` selects 8,
10, or 12 original source poses. It preserves mandatory semantic phases and
promotes the frame count when eight poses cannot reconstruct the intervening
motion within the deterministic error budget. Frame durations come from the
original PTS gaps and `playbackSpeedRatio` is exactly `1.0`.

Normalization uses one scale and one anchor per animation. It no longer aligns
each frame independently to its current body bottom/center, so natural weight
shift and vertical body motion survive. Unexpected drift is reported rather
than hidden by geometry rewriting.

## Pack and Godot delivery

The Pack contains external PNGs, all eight animation entries, source-time frame
durations, a portable DirectionMotionLock, approval evidence, and a sanitized
Provider manifest containing hashes but no absolute JobStore paths, token,
header, or temporary URL. Godot creates ordinary `SpriteFrames`,
`AnimatedSprite2D`, `.tres`, and `.tscn` resources. `forge_usage.json` maps each
cardinal direction to its explicit walk and idle animation with `flipH: false`.

Godot text resources remain below 1 MiB and may not contain an embedded Image,
`PackedByteArray`, or `ImageTexture.create_from_image`.

## Retry and replay

`loop`, `matting`, and `consistency` retries reuse the completed source Job with
a fail-closed `LocalReplayProvider`. They skip credentials and Provider health,
issue zero media requests, preserve source video hashes, and create a new child
Job. The original Job is immutable.

## Release gates

- Schema and typed-lineage unit tests.
- Fixture image stage: 1 generate + 7 edits + 0 videos, no Pack.
- Fixture targeted image retry: `idle_up` costs 2 expected / 4 maximum image
  edits, regenerates only `back_idle` and `back_walk`, and leaves the source
  Job byte-identical.
- Canonical stance feedback rejects a stepping biped `front_idle`, while a
  planted stance remains game-ready.
- Direction feedback flags a same-scale candidate that loses broad permanent
  upper-body structure.
- Head-covering loss produces targeted retry guidance instead of a terminal
  rejection.
- Lower-covering loss produces full-length/hemline retry guidance.
- V8 materialization cleans chroma fringe while preserving enclosed green
  clothing in fixture evidence.
- Explicit approval followed by complete stage: 0 images + 4 videos.
- PTS contract: encoded 80 ms fixture frames remain 80 ms in the Pack.
- Local replay: plan 0/0, observed usage 0, loop reports byte-identical.
- Pack validation and credential/path scan.
- Godot 4.6 headless import/load with all eight animations and explicit left.
- V7 and earlier workspace regression tests remain green.
- Real xAI starts with one approved `walk_right` probe before a separately
  authorized four-direction run.

## Chroma boundary

V7's adaptive chroma compositing is intentionally not part of V8. Chroma or
matting is only a local extraction concern after a Provider video exists; it
must never rewrite the approved image that defines the first video frame. If a
Provider cannot preserve a usable empty/alpha background, the Job fails closed
through the ordinary generic quality gates instead of repairing the source
image with a generated background.
