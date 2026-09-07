# Forge full-body keyframe timeline V13

Date: 2026-08-18

## Decision

Forge must not create walking animation by copying an immutable raster region
above a horizontal body cut. A side walk is authored as one direction and one
action at a time, with four complete-body semantic keyframes:

`contact A -> passing A -> contact B -> passing B`

Every frame owns one coherent pelvis-to-boot pose. Static reuse is permitted
only for independently authored layers; a flattened torso/cape/hip rectangle is
not a reusable layer.

## Authoring contract

- Approve one direction still before generating its actions.
- Generate all four keyframes in one 2x2 request so identity, camera and scale
  share one context.
- Use a style-free joint-chain guide for pose order. The guide distinguishes
  foreground and background limbs but contains no character silhouette to copy.
- Allow modest pelvis bob, hip rotation, torso counter-rotation, arm swing and
  lower-cape lag. Do not require byte-identical upper-body pixels.
- Retry an individual failed keyframe with the surrounding timeline as context;
  never replace accepted source files in place.
- Split, matte, use one shared scale, align the upper-body center and foot
  baseline, then decontaminate edge RGB without modifying Alpha.
- Review four frames in Godot before generating in-betweens or another
  direction.

## Motion gate

`motion-semantics@1.2.0` extends the existing gait gate for side-facing walks:

- `proximalLegDynamicDegree` measures the 74-86% body-height band;
- `kneeShinDynamicDegree` measures the 84-94% band;
- `footDynamicDegree` measures the bottom 6%;
- `footToKneeShinMotionRatio` detects motion concentrated in boots beneath
  nearly frozen knees and shins.

Articulation bands align by the stable upper-body center instead of the whole
body bounding box, because a real forward boot legitimately changes the whole
silhouette center. A side walk is blocked when knee/shin motion is below `0.14`
or foot motion exceeds knee/shin motion by more than `3.5x`.

This gate is additive. Direction, phase order, extra-foot, identity, silhouette,
equipment, Alpha and Pack gates remain authoritative.

## Godot delivery

Accepted candidates use four external `512x512` RGBA PNGs in a native
`SpriteFrames` resource played by `AnimatedSprite2D` at 4 FPS. The four-frame
pilot must pass visual review before it is expanded to eight frames. An
eight-frame version inserts one generated in-between between each accepted
semantic keyframe; it does not regenerate the four anchors.
