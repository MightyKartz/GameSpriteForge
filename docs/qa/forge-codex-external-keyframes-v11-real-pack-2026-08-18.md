# Codex external keyframes V11 — cleanup and real Pack attempt

Date: 2026-08-18

Verdict: **edge cleanup passed; Godot `walk_right` runtime passed; V11 Pack
export blocked by character semantic, motion and silhouette gates.** No
`.gsfpack` was emitted.

## Deterministic magenta-fringe cleanup

The 15 accepted 512×512 RGBA frames were treated as immutable sources. A
deterministic decontamination pass wrote sibling frames under `frames-clean/`.
It changed only RGB values for magenta-like pixels connected to the transparent
exterior and copied RGB from the nearest non-magenta foreground donor.

- decontaminated pixels: **9,420**;
- changed Alpha pixels: **0**;
- remaining border-connected magenta fringe pixels: **0**;
- changed Alpha bounding boxes: **0**;
- cleanup report verdict: **passed**.

The source frames under `frames/` were not overwritten.

## Godot-first `walk_right` gate

The four clean-source-equivalent right-facing frames were loaded through Godot
4.6.3 native `AnimatedSprite2D` + `SpriteFrames` at 4 FPS. Godot rendered 68
movie frames at 30 FPS over 2.2667 seconds.

- foot baseline drift: **0.0 px**;
- horizontal center drift: **0.5 px**;
- subject-height drift: **0.0 px**;
- runtime geometry gate: **passed**.

This approves the sequence as a prototype/runtime source animation. It does not
override Forge production consistency gates.

## V11 Pack execution

- Workflow: `topdown-external-keyframes@11.0.0`.
- Job: `70c0b7fb-5e41-4235-a4f4-9ab09d5ecd55`.
- Provider request estimate: **0**.
- Maximum Provider requests: **0**.
- Actual Forge media-Provider requests: **0**.
- Ingest, preserve-Alpha matting, shared normalization and every per-animation
  base quality step: **succeeded**.
- Pack export: **blocked** (`character_semantic_quality_failed`).

### Blocking evidence

Direction/effect semantics:

- `idle_up`: selected interval direction drift;
- `walk_up`: selected interval direction drift;
- `idle_right`, `walk_right`, `walk_up`: orange/high-luminance costume pixels
  were classified as an unexpected attached emissive halo.

Motion semantics:

- `walk_down`: upper-body flicker `0.1352` (budget `0.08`) and phase-order
  score `0.2583` (minimum `0.45`);
- `walk_up`: upper-body flicker `0.3533` and phase-order score `0.2259`;
- `walk_right`: upper-body flicker `0.4111`, phase-order score `0.4066`, and
  maximum foot-lobe count `3` (budget `2`).

Silhouette temporal consistency:

- all three walk animations failed upper-body contour drift, unsupported core
  edge drift and temporal edge-color flicker;
- `walk_right` also reported a body-center step of `10.5 px`.

The no-equipment hand/contact report passed for all animations.

## Decision boundary

Godot playback alone is sufficient to call `walk_right` usable for a prototype,
but it is not sufficient to call the independently redrawn set a production V11
Pack. `quality.requireGameReady=false` does not bypass these hard semantic,
motion and silhouette failures. Shipping now would require a new explicit
human-review/candidate-Pack product contract; silently weakening thresholds or
forging a Pack is not permitted.

The production-safe next route is targeted keyframe remediation with a locked
upper-body identity/contour and corrected gait ordering, plus a separately
tested detector fix so bright non-emissive costume colors are not treated as
glow.

## Artifacts

- Clean frames and cleanup report:
  `generated-assets/experiments/codex-external-keyframes-v11-real-20260817/`
- Godot right-facing QA: `godot-walk-right-qa/`
- V11 request: `request-v11.json`
- Copied Job reports: `reports/v11-job-70c0b7fb/`
