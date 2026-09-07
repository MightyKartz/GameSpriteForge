# Codex walk-right V14 normal-amplitude — real validation

Date: 2026-08-18

Verdict: **the low-amplitude guide removes V13's marching/high-knee failure,
but this four-frame candidate is still blocked and has not been promoted.** The
two opposite contact drawings do not produce a sufficiently distinct opposing
silhouette, and complete-body redraw still changes stable upper-body pixels.
No `.gsfpack`, production replacement, eight-frame expansion, or additional
direction was created.

Experiment root:

`generated-assets/experiments/codex-walk-right-v14-normal-amplitude-20260818/`

## Generation

- Surface: Codex subscription built-in image generation.
- Image API/API key: not used.
- Real generation requests: exactly one.
- Inputs: accepted V11 right-direction still as sole identity/style authority;
  V14 style-free low-amplitude joint guide as geometry only.
- Output: one fresh 1254x1254 solid-magenta 2x2 sheet.
- Processing: complete-cell extraction, largest component, one shared scale,
  upper-body-center and planted-foot alignment, then RGB-only fringe cleanup.
- Forbidden operations remained forbidden: torso lock/cut, lower-body splice,
  boot translation, or mutation of an earlier experiment.

Raw output SHA-256:

`dd2c03b408eb45f017e11209271836e2914eb7e7702b2e0d4c823a3ab6e055de`

## Guide amplitude preflight

The pose guide was audited before the generation request:

| Metric | V14 guide | Allowed |
| --- | ---: | ---: |
| maximum swing-foot clearance / leg length | 0.0955 | 0.02–0.12 |
| maximum knee lift / leg length | 0.0101 | 0.00–0.18 |
| contact stride / leg length | 0.7035 | 0.35–0.85 |
| pelvis bob / leg length | 0.0302 | 0.00–0.08 |

All guide checks passed. The prompt explicitly prohibited marching, running,
jogging, stair climbing, obstacle stepping, horizontal thighs, and knees near
the hip/waist.

## Motion-semantics V1.3 calibration

`motion-semantics@1.3.0` adds a side-walk maximum
`kneeShinDynamicDegree` of `0.75`. It remains a raster proxy rather than a pose
estimator or replacement for human review.

| Sample | Knee/shin dynamic degree | Amplitude envelope |
| --- | ---: | --- |
| V13 human-rejected high knee | 0.9027 | blocked |
| V14 normal-amplitude candidate | 0.7265 | passed |

A proposed maximum `footDynamicDegree` gate was removed during calibration:
V14 measured `0.9743`, higher than V13's `0.9108`, because normal forward/back
boot displacement also changes the foot silhouette. It does not measure foot
clearance and would have been a false gate.

The V14 motion report remains blocked:

| Metric | V14 |
| --- | ---: |
| distinct poses | 4 |
| opposing contact change | 0.0501 |
| phase-order score | 0.1773 (minimum 0.45) |
| proximal-leg dynamic degree | 0.3298 |
| knee/shin dynamic degree | 0.7265 |
| stable upper-body flicker | 0.2218 (maximum 0.08) |

Reasons: `walk_phase_order_invalid` and `stable_upper_body_flicker`. The model
changed which leg appears near/far between contact frames, but the two contact
silhouettes remain too similar in extension direction for the deterministic
four-phase gate. This is visible review evidence, not a shippable loop.

## Matting and Godot

- Shared frame scale: `0.8788822355`.
- Border-connected magenta pixels decontaminated: 3,632.
- Remaining connected magenta pixels: 0.
- Alpha changes: 0.
- Godot: 4.6.3 stable, OpenGL Compatibility.
- Runtime: native `AnimatedSprite2D` and `SpriteFrames`, external PNGs, 4 FPS,
  linear filtering, no pixel snap.
- MovieWriter: 68 frames at 30 FPS, 2.2667 seconds.
- V14 foot baseline drift: 1 px.
- V14 whole-bbox center range: 25 px.
- V14 subject-height range: 7 px.

The first MovieWriter attempt did not receive the script's auto-quit flag and
was manually stopped. The preview script now consumes an explicit user
`--auto-quit` argument; the final AVI/MP4 were overwritten by the verified
68-frame run.

## Artifacts

- prompt: `prompt-used.txt`;
- lineage: `generation-lineage.json`;
- guide: `reference/walk-right-normal-joint-guide.png`;
- guide QA: `qa/normal-walk-guide-amplitude.json`;
- raw sheet: `raw/walk-right-2x2-normal-amplitude.png`;
- clean frames: `processed/frames-clean/walk_right/`;
- transparent 2x2 sheet: `processed/walk-right-v14-transparent.png`;
- animation GIF: `processed/walk-right-v14-animation.gif`;
- V13/V14 amplitude calibration: `qa/normal-walk-amplitude-calibration.json`;
- V14 motion report: `qa/v14-motion-semantics-1.3.json`;
- V13 negative re-audit: `qa/v13-rejection-motion-semantics-1.3.json`;
- Godot project: `godot/walk-right-v14-compare/`;
- Godot comparison MP4:
  `godot/walk-right-v14-compare/qa-output/walk_right_v14_compare.mp4`.

## Verification

- `cargo test -p core motion_semantics -- --nocapture`: 8 passed.
- `cargo test -p providers --test keyframe_generation_contract`: 1 passed.
- `cargo test -p pack`: 46 passed across unit/integration tests.
- `cargo clippy -p core --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `cargo build -p forge-cli`: passed.
- Godot import/headless runtime/MovieWriter: passed.

## Next gate

Native Godot visual review decides whether V14's reduced amplitude feels like a
normal walk. Even if the amplitude is accepted, the next generation must make
the second contact frame visibly oppose the first without increasing knee
height. Eight frames, other directions, and Pack export remain blocked.
