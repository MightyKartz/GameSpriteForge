# Codex external keyframes V11 — real `walk_up` Godot QA

Date: 2026-08-17

Verdict: **PASS for continuing directional frame generation.** This is a
provisional four-frame runtime gate, not full V11 Pack acceptance.

## Scope

- Source: four real Codex built-in image-generation candidates, processed to
  independent 512×512 RGBA PNG files.
- Engine: Godot 4.6.3 stable, official build `7d41c59c`.
- Runtime: native `AnimatedSprite2D` + `SpriteFrames`, external imported PNG
  textures, 4 FPS and looping playback.
- Rendering: linear texture filtering, texture repeat disabled and
  `pixelSnap=false`.
- Isolation: the QA project is contained under the experiment directory and
  does not mutate a production Godot project, Pack, catalog or source Job.
- Provider usage: zero Forge media-Provider requests.

## Result

Godot rendered 68 movie frames at 30 FPS (2.2667 seconds, more than two
animation loops) using the macOS OpenGL Compatibility renderer.

Measured from the textures as loaded by Godot:

- foot baseline drift: **0.0 px**;
- horizontal center drift: **0.5 px**;
- subject-height drift: **0.0 px**;
- per-frame canvas: **512×512 RGBA**;
- geometry gate: **passed**.

Native movie checkpoints show the sequence `0 → 1 → 2 → 3 → 0` without a
position jump on loop return. The rear-facing direction, plain cape topology,
orange scarf band and leg alternation remain readable. No detached object,
checkerboard background or Alpha-box artifact appears in the Godot capture.

## Artifacts

- QA project: `generated-assets/experiments/codex-external-keyframes-v11-real-20260817/godot-walk-up-qa/`
- Runtime metrics: `qa-output/walk_up_runtime_metrics.json`
- Runtime capture: `qa-output/runtime_frame_0.png`
- Final movie: `qa-output/walk_up_runtime-final.mp4`
- Movie checkpoints: `qa-output/walk_up_runtime_contact.png`

## Invocation finding

Godot must finish `--headless --import` before the runtime attempt. Movie Maker
must then run with the normal macOS display driver and an output path relative
to the project. Combining incomplete import, a repository-relative movie path
and the headless dummy renderer entered a Godot Movie Writer crash path; that
failed attempt is not acceptance evidence. The final normal-driver recording
completed with exit code 0 and no warnings.

## Remaining boundary

The result approves continuing with `walk_right`. It does not yet prove the
complete six-animation V11 request, Forge semantic gates, `.gsfpack` output or
the final Godot installer path.
