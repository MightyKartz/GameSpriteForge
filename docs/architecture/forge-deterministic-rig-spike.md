# Forge Deterministic Rig / Deformation Spike

Status: report only
Date: 2026-08-11

## Conclusion

Deterministic rigging/deformation is conditionally feasible for Forge, but
only as a later optional workflow for well-posed, single-subject, biped-like
top-down walk cycles. It is not yet a safe replacement for image-grid
generation or the current Pack pipeline.

The right next step is not implementation. It is a separate prototype that
uses one accepted `front_idle` and produces only `walk_down` before any
multi-direction expansion.

## Why this path is attractive

AnimatedDrawings demonstrates the core idea: one character image can be
segmented, assigned a skeleton, and driven by an existing motion clip. For a
top-down game sprite walk cycle, the same architecture would offer:

- zero marginal generation cost after the base image;
- exact frame timing and loop closure;
- no identity drift between frames;
- deterministic left/right phase order;
- natural compatibility with Forge's JobStore, Pack, and Godot contracts.

This fits Forge's deterministic philosophy better than asking an image model
to independently invent every animation frame.

## Minimum viable prototype

```text
approved front_idle
→ foreground segmentation / matting
→ body-part keypoints or manual rig template
→ 2D skeletal bind
→ versioned walk_down motion curve
→ deterministic render of 4 or 8 frames
→ existing motion-semantics and Pack gates
```

The first prototype should produce only `walk_down` from an approved
`front_idle`. Multi-direction rigs should wait until the forward walk is
visually acceptable.

## Main blockers

1. Occlusion and layered clothing. Capes, scarves, weapons, hair, and loose
   garments cannot be treated as one rigid silhouette. They need layer
   decomposition or separate deformation rules.
2. Top-down anatomy. Humanoid keypoint models are usually trained for
   front/side photos, not top-down game sprites with short proportions.
3. Direction changes. A single front image does not contain enough visual
   evidence to synthesize a correct rear or side view. Deterministic rigging
   cannot replace multi-view generation.
4. Commercial licensing. Any rig, weight, motion clip, or segmentation model
   must pass license and redistribution review before becoming a Forge
   component.

## Recommended position

- Keep `topdown-grid@9.0.0` as the primary next acceptance path.
- Keep deterministic rigging as an optional later `rig2d` component.
- Prototype only after Pixel/grid acceptance and Provider selection are
  resolved.
- Use rigging first for animation reuse within one approved direction, not
  for direction synthesis.

## Decision

Do not implement a rig path in the current remediation work. The grid path is
closer to production, needs fewer unproven components, and preserves the
existing Provider-neutral architecture. A future rig prototype should be
scoped to one approved front image and one forward walk cycle.
