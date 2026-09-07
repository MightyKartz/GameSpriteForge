# Codex walk-right V13 full-body keyframes — real validation

Date: 2026-08-18

Verdict: **human Godot review rejected this candidate.** The full-body,
joint-guided method fixes the earlier boots-only failure and produces the
correct four-phase order, but its passing frames raise the knee and boot like a
march or stair-climb rather than a normal walk. Pack export remained blocked by
upper-body redraw/contour gates and the known orange-scarf attached-halo false
positive. No production asset or source experiment was overwritten.

## Human review result

The V13 pose guide itself encoded excessive passing-frame amplitude: the swing
foot sat roughly 30% of a nominal leg length above the ground and the knee was
pulled sharply forward. The prompt reinforced that shape by asking for a
"lifted knee/boot." Automated articulation gates only imposed a lower bound,
so they correctly rejected frozen legs but could not reject marching.

V13 is therefore a negative calibration sample. It must not be promoted,
interpolated to eight frames, or copied to other directions. V14 replaces the
pose authority with a measured low-clearance normal-walk guide and adds an
upper motion envelope.

## Real authoring scope

- Generation surface: Codex subscription built-in image generation; no Image
  API, xAI Provider request, credential, or temporary media URL entered Forge.
- Generation unit: one `walk_right` action in one 2x2 sheet.
- Identity authority: the accepted V11 right-facing direction still.
- Pose authority: a deterministic style-free four-phase joint guide with blue
  foreground limbs and orange background limbs.
- Postprocess: complete-cell extraction, magenta matting, largest-component
  selection, one shared scale, upper-body-center/foot alignment, and RGB-only
  fringe decontamination.
- Forbidden operations: torso cut, upper-body lock, cape lock, lower-body
  compositing, or mutation of V11/V12 inputs.

Experiment root:

`generated-assets/experiments/codex-walk-right-v13-keyframe-timeline-20260818/`

## Implemented Forge gate

Core now emits `motion-semantics@1.2.0` with proximal-leg, knee/shin and foot
dynamic degrees plus a foot-to-knee/shin motion ratio. Side-walk articulation is
aligned by upper-body center, not the moving whole-body box.

The actual locked V12 frames are now blocked with:

| Metric | Locked V12 |
| --- | ---: |
| phase-order score | 0.607436 |
| proximal-leg dynamic degree | 0.043077 |
| knee/shin dynamic degree | 0.105869 |
| foot dynamic degree | 0.386668 |
| foot / knee-shin ratio | 3.652318 |
| verdict | blocked |

Reasons: `walk_side_knee_shin_motion_missing` and
`walk_side_foot_only_motion`. This closes the false-positive that previously
called the visually frozen animation game-ready.

## V13 attempts

The first full-body sheet established strong articulation but did not create two
distinct compact passing poses. A selective fourth-frame repair improved phase
order from `0.160813` to `0.372780`, still below the `0.45` minimum.

The accepted review candidate was freshly generated from the direction still
and the style-free joint guide:

| Metric | Joint-guided V13 |
| --- | ---: |
| distinct poses | 4 |
| phase-order score | 0.581948 |
| maximum foot lobes | 2 |
| proximal-leg dynamic degree | 0.548619 |
| knee/shin dynamic degree | 0.902715 |
| foot dynamic degree | 0.910793 |
| foot / knee-shin ratio | 1.008949 |
| direction match | 4/4 right-facing |
| foot baseline drift | 0 px |

The new anatomical and phase gates pass. Motion semantics remains blocked only
by `stable_upper_body_flicker`: `0.296500` against the existing `0.08` limit.
Silhouette temporal QA also blocks upper contour/core-edge drift, body center
step, and edge-color flicker. Character semantic QA still reports the known
orange-scarf `unexpected_attached_emissive_halo` false positive. These gates
were not weakened to manufacture a passing Pack.

Final zero-request Forge Job:

`a157e47f-120c-4ecf-bbb0-b01696c19956`

Plan estimate and maximum Provider requests were both zero. The Job reached
`quality_checked`; `.gsfpack` export correctly remained blocked.

## Godot 4.6.3 comparison

An isolated native Godot project compares the rejected locked V12 on the left
with joint-guided V13 on the right. Both use external PNGs,
`AnimatedSprite2D`, native `SpriteFrames`, 4 FPS, linear filtering and no pixel
snap.

- rendered movie frames: 68 at 30 FPS;
- duration: 2.2667 seconds;
- V13 foot drift: 0 px;
- V13 whole-silhouette bbox-center range: 23.5 px;
- V13 subject-height range: 9 px.

The whole-silhouette center changes because contact poses extend one leg while
passing poses compact it; the upper-body alignment stays on the shared canvas.
This is retained for visual review rather than hidden by recentering or raster
locking.

## Artifacts

- joint guide: `reference/walk-right-joint-phase-guide.png`;
- joint-guided raw sheet: `raw/walk-right-2x2-joint-guided.png`;
- accepted review frames: `joint-guided/frames-clean/walk_right/`;
- direct V13 motion report: `qa/v13-joint-guided-motion-semantics.json`;
- actual locked-V12 rejection: `qa/v12-locked-motion-semantics-1.2.json`;
- final Forge evidence: `jobs/a157e47f-120c-4ecf-bbb0-b01696c19956/`;
- Godot project: `godot/walk-right-v13-compare/`;
- comparison video: `godot/walk-right-v13-compare/qa-output/walk_right_v13_compare.mp4`;
- runtime metrics: `godot/walk-right-v13-compare/qa-output/walk_right_v13_runtime_metrics.json`.

## Verification

- `cargo test -p core motion_semantics -- --nocapture`: 7 passed, 0 failed.
- `cargo test -p providers --test keyframe_generation_contract`: 1 passed,
  0 failed after calibrating the foot/knee ratio ceiling to `3.5` against the
  existing valid fixture (`3.271`) and the real locked negative (`3.652`).
- `cargo test -p pack`: 46 passed, 0 failed.
- `cargo clippy -p core --all-targets -- -D warnings`: passed.
- `cargo build -p forge-cli`: passed.
- deterministic matting/fringe cleanup: Alpha changed pixels `0`, remaining
  border-connected magenta pixels `0`.
- Godot 4.6.3 import and headless runtime: passed.
- Godot OpenGL Compatibility MovieWriter: 68 frames written successfully.

## Next gate

Human Godot review decides whether the stronger leg motion is the desired walk
style. If accepted, the next bounded task is identity-preserving refinement of
frames 2 and 3 or a calibrated full-body redraw profile; it is not another
hard upper-body lock. Eight-frame interpolation and other directions remain
blocked until this four-frame review closes.
