# Codex locked-upper-body V12 pilot — real validation

Date: 2026-08-18

Verdict: **the revised one-direction action-sheet method is valid for
`walk_right`: one shared 2x2 generation plus deterministic upper-body locking
passed Forge motion and silhouette gates and passed native Godot 4.6 playback.**
The complete Character Pack remains blocked by the existing orange-scarf
`unexpected_attached_emissive_halo` false positive and by unchanged legacy
directions, so no production `.gsfpack` or production Godot asset was modified.

## Implemented method

The source is attempt 1 from the preceding direction-action-sheet pilot. No new
image generation or Provider request occurred in this step.

`compose_locked_upper_body.py` deterministically assembles each frame from only:

- one immutable generated upper-body authority, attempt-1 frame 1;
- the corresponding generated attempt-1 lower-body pose;
- an upper lock through Y=366 plus the authority cape through Y=430/X=235;
- lower-source cape de-duplication restricted to X<=235 and 366<Y<=430.

It never synthesizes pixels and never overwrites its inputs. The final V8 report
proves `sourcesUnchanged=true`, `lockedRegionUnchanged=true`, zero changed locked
pixels in every frame, and `passed=true`. Magenta cleanup changed zero Alpha
pixels and left zero connected fringe pixels.

The first hard-occlusion draft incorrectly classified olive trousers as cape and
created visible transparent gaps. Restricting the operation horizontally removed
that defect. A later draft still reached below the cape and removed dark pixels
from a rear boot; restricting the operation vertically produced V8 with the same
bottom coordinate (`472`) in all four Alpha bounds. Rejected drafts remain in the
experiment directory for audit and were never promoted.

## Forge support change

Core now exposes opt-in `normalize.mode="preserve_canvas"`. It retains the maximum
source canvas and uses zero X/Y offset for every frame, so a sheet-authored common
coordinate system is not destroyed by per-frame bounding-box centering. Existing
normalization defaults and modes are unchanged.

The new unit test uses two differently placed source rectangles and verifies:

- output images equal both sources byte-for-pixel;
- all X/Y offsets are zero;
- output and source bounding boxes are equal.

Validation command: `cargo test -p core --test anchor_tests` — 6 passed, 0 failed.

## Real V11 result

Final Forge Job: `6e4f3049-74e7-496d-b709-db0dfff30580`.

Plan estimate and maximum were both zero Provider requests. `walk_right` results:

| Gate / metric | Result |
| --- | ---: |
| direction match | 4/4 right-facing |
| distinct poses | 4 |
| phase-order score | 0.607436 |
| stable upper-body flicker | 0.000254 |
| maximum foot lobes | 2 |
| core-mask IoU min / median | 0.993483 / 0.994261 |
| maximum contour distance | 1.035242 px |
| maximum unsupported core edge | 0.0 |
| maximum body-center step | 1.0 px |
| maximum foot-anchor step | 0.0 px |
| motion semantics | `game_ready` |
| silhouette temporal | `game_ready` |

The animation-quality aggregate is still `blocked` only because all four frames
trigger `unexpected_attached_emissive_halo` (maximum 58 pixels). This is the known
orange-costume semantic false positive; it is not evidence of animation, Alpha,
direction, gait, or temporal failure. The Pack exporter correctly remained blocked.

## Godot 4.6.3 result

The four final RGBA frames were imported into an isolated native
`AnimatedSprite2D`/`SpriteFrames` project at 4 FPS with linear filtering and
`pixelSnap=false`. Godot rendered 68 movie frames at 30 FPS.

| Runtime metric | Result |
| --- | ---: |
| foot drift | 0.0 px |
| horizontal bbox-center drift | 2.5 px |
| subject-height drift | 0.0 px |
| geometry gate | passed |

The runtime capture shows no transparent waist gap, detached cape fragment, boot
deletion, or direction flip. This validates the frame sequence as a usable isolated
Godot animation candidate, while the full Pack remains correctly unpromoted.

## Bound artifacts

Experiment root:
`generated-assets/experiments/codex-direction-action-sheet-v12-pilot-20260818/`.

- compositor: `tools/compose_locked_upper_body.py`;
- final candidate: `locked-upper-body-v8-phase-retained/`;
- frame SHA-256 values: `d721c239...`, `ea63eccb...`, `e3ea9f93...`,
  `a4f97f1c...` in row-major playback order;
- upper authority SHA-256: `ea63eccb54b6f922181ec0221f4096b34b7dfe7a38eecb3a65f3b22a6ef4e37a`;
- lock report SHA-256: `45c163a9a7be881b1631f708e3437cd44622ac6952e6515e67b50a78ac48a8b5`;
- V11 reports: `locked-upper-body-v8-phase-retained/forge-job-6e4f3049/`;
- Godot metrics: `locked-upper-body-v8-phase-retained/godot-walk-right-qa/qa-output/walk_right_runtime_metrics.json`;
- Godot movie SHA-256: `e81feb95ffb05b905b9bdf0f9f1a415e57fc856b646f4ab86a68096274a2da64`.

## Decision

Use this topology for the next direction: **one direction-authority image, one
same-direction 2x2 action sheet, deterministic upper/cape lock, boot-safe lower
mask, `preserve_canvas`, then Forge and Godot gates.** Do not return to independent
per-frame generation, and do not generate all remaining directions in one batch.
The next bounded content step is `walk_up` using the same method after separately
fixing or calibrating the orange-scarf halo detector; `walk_right` itself needs no
new image generation.
