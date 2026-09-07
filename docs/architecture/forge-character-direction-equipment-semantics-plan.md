# Forge Character Direction and Equipment-Effect Semantics Plan

Date: 2026-08-07

Status: implemented; extended by `forge-character-camera-fx-framing-plan.md`

## Problem

The prior Character gate measured media integrity, cross-frame appearance, and loop closure,
but it did not prove that an animation faced the requested gameplay direction or that an
equipment effect remained semantically stable. A front-facing `walk_up` clip could therefore
score as a good loop, while video-only sparks around a staff could pass the generic consistency
metrics.

These are gameplay correctness failures, not reviewable stylistic differences. They must block
Pack export.

## Release contract

Forge adds two local, versioned Character gates:

- `direction-quality@1.1.0` (supersedes `1.0.0`, which could mistake scarf folds for a face)
- `equipment-effect-consistency@1.1.0` (`1.0.0` remains schema-readable)

The generated Character workflow now follows this contract:

```text
direction-specific prompt
→ direction still
→ still semantic gate
→ animation video
→ loop selection and normalization
→ final 32-frame semantic gate
→ quality and consistency
→ Pack
→ Godot
```

The still gate prevents a semantically wrong direction from incurring a video request. The final
gate catches direction changes and effects introduced during video generation or loop selection.

## Direction rules

- `idle` and `walk_down` are front-facing: the face must remain visible in at least 62.5% of
  frames.
- `walk_up` is rear-facing: the face may be visible in at most 12.5% of frames.
- `walk_right` receives a strict side-profile generation contract. A calibrated right-facing
  classifier is deferred to the optional vision component rather than pretending that the
  current color heuristic can prove side direction.
- A prompt that explicitly requires a hidden face opts out of the face-visibility rule, while
  all media-integrity and equipment-effect gates remain active.

Rear-facing output naturally has different edge density from front-facing output. Character
reports therefore use `consistency@1.6.0`, where cross-direction edge density is advisory while
canvas, alpha, clipping, subject count, palette, scale, anchor, identity, direction, and effects
remain enforced. Static assets continue to use `consistency@1.5.0`.

## Equipment-effect rules

The base character defines the equipment's allowed appearance. Direction still and video prompts
forbid spell casting, lightning, sparks, particles, detached glow, magic trails, and newly invented
effects unless the asset specification explicitly requests an effect animation.

The local detector analyzes alpha-connected foreground components. Small, detached, high-luminance
and high-chroma components outside the principal character component are treated as unexpected
emissive effects. Version 1.1 also builds a bounded mask around an opaque bright equipment core and
detects a translucent attached halo.

Detached or opaque effects still require regeneration. A narrow deterministic cleanup is allowed
only for translucent pixels outside the two-pixel antialiasing edge of attached bright equipment,
and only when the prompt did not request permanent baked emission. The final semantic gate runs
again on the cleaned result; it never deletes the opaque staff tip or body pixels.

## Retry and review policy

- Wrong direction in a direction still: retry `still`, invalidating video and downstream nodes.
- Direction correct in the still but wrong in the selected animation: retry `still` conservatively.
- Detached equipment effects introduced by video: retry `video`, preserving the accepted still.
- A translucent attached halo: retry `matting` or `consistency`; both are local and zero-cost.
- Local `consistency` replay performs both semantic gates without Provider requests.
- `regenerate` and `blocked` semantic failures cannot be accepted with `forge job review`.
- Failed rechecks create a child Job, preserve the source Job, export no Pack, and quarantine any
  matching generated-asset catalog entry.

## Provenance

`character-semantic-quality-report.json` is a schema-validated Job and Pack report. Successful
Pack metadata, workflow stage manifests, and Godot `forge_usage.json` record the semantic profiles
and verdict. They do not contain credentials, authorization headers, device codes, or temporary
Provider URLs.

## Acceptance gates

- Unit fixtures for correct front/rear directions, wrong `walk_up`, and detached effects.
- Provider contract: wrong rear still stops before video generation.
- Provider contract: video-introduced effects block Pack export after repair attempts.
- Full workspace tests, CLI product contract, Stage 3 contract, formatting, and Clippy.
- Zero-cost replay of the existing real xAI Character.
- No new request-ledger entry during local replay.
- A future real repair is limited to the failed `walk_up` animation and requires separate explicit
  cost authorization.
