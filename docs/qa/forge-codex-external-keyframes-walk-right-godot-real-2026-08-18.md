# Codex external keyframes V11 — real `walk_right` Godot QA

Date: 2026-08-18

Verdict: **PASS as a source animation asset for V11 cleanup and packing.**
This gate validates the four-frame right-facing runtime sequence before the
shared chroma-fringe cleanup and final Pack installation gate.

## Scope

- Source: four real Codex built-in image-generation frames, independently
  processed to 512×512 RGBA PNG files.
- Engine: Godot 4.6.3 stable, official build `7d41c59c`.
- Runtime: native `AnimatedSprite2D` + `SpriteFrames`, external imported PNG
  textures, 4 FPS and looping playback.
- Rendering: linear texture filtering, texture repeat disabled and
  `pixelSnap=false`.
- Isolation: `godot-walk-right-qa` is contained under the experiment directory;
  it does not mutate a production Godot project, source Job or Pack.
- Provider usage: zero Forge media-Provider requests.

## Result

Godot rendered 68 movie frames at 30 FPS (2.2667 seconds, more than two
animation loops) using the macOS OpenGL Compatibility renderer.

Measured from the textures after Godot import:

- foot baseline drift: **0.0 px**;
- horizontal center drift: **0.5 px**;
- subject-height drift: **0.0 px**;
- per-frame canvas: **512×512 RGBA**;
- geometry gate: **passed**.

The native checkpoints show `contact A → passing A → contact B → passing B`.
Frames 0 and 2 retain the same stride width while reversing arm/leg phase;
frames 1 and 3 retain a compact high passing pose with opposite leg depth.
The strict right-facing direction, ranger identity, orange scarf, plain olive
cape and stable feet line remain readable. The sequence returns from frame 3 to
frame 0 without a position jump.

The current source frames still have a thin residual magenta antialias fringe.
That is a deterministic matting issue, not a motion blocker, and remains in
scope for the shared cleanup pass before V11 Pack creation.

## Artifacts

- QA project: `generated-assets/experiments/codex-external-keyframes-v11-real-20260817/godot-walk-right-qa/`
- Runtime metrics: `qa-output/walk_right_runtime_metrics.json`
- Runtime capture: `qa-output/runtime_frame_0.png`
- Four-phase checkpoint: `qa-output/walk_right_runtime_phases.png`
- Final movie: `qa-output/walk_right_runtime-final.mp4`

## Remaining boundary

This result approves deterministic edge cleanup and V11 packing. It does not
yet prove the final `.gsfpack`, complete six-animation semantic gates or the
final Godot installer output.
