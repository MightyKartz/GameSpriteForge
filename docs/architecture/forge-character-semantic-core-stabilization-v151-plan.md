# Forge character semantic-core stabilization V15.1 plan

Date: 2026-08-19

## Decision

Before mutating V15, correct the stable-upper-body metric. The previous metric
aligned frames by the whole-character bounding box, which moves horizontally
as legs extend. `motion-semantics@1.5.0` instead aligns the upper body by its own
Alpha anchor and reports geometry and texture separately:

- `stableUpperBodyAlphaDriftRatioMax`;
- `stableUpperBodyColorFlickerRatioMax`.

Only if the corrected color gate still blocks may a zero-Provider semantic
core stabilization run.

## Stabilization contract

1. Translate each frame into a shared upper-body anchor space.
2. Intersect the four upper Alpha masks and locally uniform interior-color
   masks.
3. Erode that non-rectangular semantic core and feather its boundary.
4. Compute per-channel median RGB from all four aligned frames.
5. Blend the median texture back into each frame's own upper-body position.
6. Preserve every Alpha byte and every lower-body RGB byte.

This is not a torso splice or rectangular upper-body lock. Existing silhouette,
arms, cape edges, legs, boots, anchors, and gait remain authoritative.

## Acceptance

- Provider requests: zero;
- Alpha changed pixels: zero;
- lower-body RGB changed pixels: zero;
- stable upper Alpha drift <= 0.20;
- stable upper color flicker <= 0.08;
- cadence, knee/shin motion, contact change, foot baseline, and canvas geometry
  unchanged;
- native Godot V15/V15.1 comparison has no pasted-texture or seam read.

Pack export remains blocked until the existing side-laterality human approval
is completed against the selected frame hashes.
