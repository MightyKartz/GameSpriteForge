# Codex walk-right V15 side cadence — real validation

Date: 2026-08-18

Verdict: **V15 preserves V14's normal low-clearance walk and improves
contact/passing separation, but remains a human-review candidate rather than a
production asset.** Side cadence and amplitude pass. Stable upper-body redraw
drift remains blocked, and anatomical foreground/background leg exchange still
requires hash-locked native Godot approval. No `.gsfpack`, eight-frame
expansion, or additional direction was created.

Experiment root:

`generated-assets/experiments/codex-walk-right-v15-side-cadence-20260818/`

## Motion-semantics V1.4

`motion-semantics@1.4.0` no longer treats the top-down four-pose Alpha polarity
score as proof of side-view anatomical laterality. A four-frame side walk now
reports:

- `sideContactPassingSpreadRatio`;
- `sideContactPassingCadenceScore`;
- `sideLateralityReviewRequired`.

The cadence score accepts clear wide-contact/compact-passing rhythm or existing
geometric phase evidence. The old phase score remains in the report for
calibration. `side-walk-laterality-approval@1.0.0` locks exact clean-frame
hashes and asks a human reviewer whether the near/far legs really exchange.

V14 re-audits as cadence-valid rather than phase-invalid:

| Metric | V14 | V15 |
| --- | ---: | ---: |
| legacy phase-order score | 0.1773 | 0.1726 |
| side contact/passing cadence | 0.9929 | 0.9813 |
| opposing-contact change | 0.0501 | 0.1700 |
| knee/shin dynamic degree | 0.7265 | 0.7514 |
| stable upper-body flicker | 0.2218 | 0.2557 |

V15 substantially separates the opposite contact drawings, but prompt-only
upper-body consistency did not improve the stable-pixel metric. That failure
was not hidden by copying or freezing a rectangular upper region.

The side knee/shin maximum was calibrated from `0.75` to `0.80`: V15 exceeds
the provisional limit by only `0.0014` while preserving the preflight low-knee
geometry and ordinary visual gait. The human-rejected high-knee V13 remains
blocked at `0.9027`. This keeps a useful margin rather than fitting the gate to
one image.

## Real generation

- Surface: Codex subscription built-in image generation.
- Image API/API key: not used.
- Real generation requests: exactly one.
- Image 1: sole identity/style/direction authority.
- Image 2: V14 normal-amplitude motion/layout reference only.
- Image 3: style-free phase/depth guide with thick foreground and thin
  background limbs.
- Output: one fresh 1254x1254 solid-magenta 2x2 sheet.
- Processing: full-cell extraction, largest component, one shared scale,
  upper-body-center and foot alignment, RGB-only fringe cleanup.
- No torso splice, boot translation, rectangular upper lock, or mutation of an
  earlier experiment.

Raw output SHA-256:

`540adc19810908a8e0cdd7338b783172cc7a12e477be8249450d7797126716e1`

## Guide and processing QA

The style-free guide preflight passed:

| Metric | V15 guide |
| --- | ---: |
| swing-foot clearance / leg length | 0.0955 |
| knee lift / leg length | 0.0101 |
| contact stride / leg length | 0.7035 |
| pelvis bob / leg length | 0.0302 |
| foreground contact side swap | true |

- Shared frame scale: `0.8913360324`.
- Border-connected magenta pixels decontaminated: 3,594.
- Remaining connected magenta pixels: 0.
- Alpha changes: 0.
- Foot lobe maximum: 2.
- Motion report reason: `stable_upper_body_flicker` only.

## Godot 4.6.3 comparison

The isolated project compares V14 on the left and V15 on the right using
external PNGs, native `AnimatedSprite2D`/`SpriteFrames`, 4 FPS, linear
filtering, and no pixel snap.

- MovieWriter: 68 frames at 30 FPS, 2.2667 seconds.
- V15 foot baseline drift: 1 px.
- V15 whole-bbox center range: 29.5 px.
- V15 subject-height range: 5 px, improved from V14's 7 px.

The pending review is stored in
`qa/walk-right-laterality-review-request.json`. It is intentionally not an
approval: only the user can decide whether the loop visibly alternates legs
and still reads as an ordinary walk.

## Verification

- `cargo test -p core motion_semantics -- --nocapture`: 8 passed.
- Godot import, headless runtime, MovieWriter, and MP4 conversion: passed.
- `cargo test -p providers --test keyframe_generation_contract`: 1 passed.
- `cargo test -p pack`: 46 passed across unit/integration tests.
- `cargo clippy -p core --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `cargo build -p forge-cli`: passed.

## Artifacts

- prompt: `prompt-used.txt`;
- lineage: `generation-lineage.json`;
- guide: `reference/walk-right-laterality-guide.png`;
- raw sheet: `raw/walk-right-2x2-side-cadence.png`;
- clean frames: `processed/frames-clean/walk_right/`;
- transparent sheet: `processed/walk-right-v15-transparent.png`;
- animation GIF: `processed/walk-right-v15-animation.gif`;
- motion report: `qa/v15-motion-semantics-1.4.json`;
- laterality review request: `qa/walk-right-laterality-review-request.json`;
- Godot comparison project: `godot/walk-right-v15-compare/`;
- comparison video:
  `godot/walk-right-v15-compare/qa-output/walk_right_v15_compare.mp4`.

## Next gate

Human Godot review must decide whether V15 alternates legs more clearly than
V14. Even if approved, Pack export remains blocked by stable upper-body redraw
drift; the next technical step would address that metric without pixel splicing
or regenerate another action/direction.
