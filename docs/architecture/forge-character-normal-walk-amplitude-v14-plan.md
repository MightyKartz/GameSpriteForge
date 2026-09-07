# Forge character normal-walk amplitude V14 plan

Date: 2026-08-18

## Decision

Retain one action per generated multi-row sheet and complete-body redraws, but
replace V13's marching-biased pose authority with a measured low-amplitude
normal-walk guide. V14 is a bounded `walk_right` 2x2 experiment. It cannot
promote a Pack, add in-betweens, or expand directions before native Godot
visual approval.

## Changes

1. Record V13 as human-rejected because its passing frames raise the knee and
   boot like marching.
2. Preflight a style-free guide for foot clearance, knee lift, contact stride,
   and pelvis bob before any generation request.
3. Add a maximum side-walk knee/shin dynamic envelope to Forge motion
   semantics. This is a raster proxy for excessive articulation/redraw, not a
   replacement for human gait review.
4. Generate one fresh 2x2 sheet through Codex built-in image generation using
   the accepted right-direction still plus the new guide.
5. Split complete cells, remove magenta, preserve one shared scale, align upper
   body and planted-foot baseline, and run motion/fringe QA.
6. Compare rejected V13 and V14 in Godot 4.6 with external PNGs and native
   `AnimatedSprite2D`/`SpriteFrames`.

## Stop conditions

- Any marching, running, stair-climbing, high-knee, or floating-foot read.
- Missing/duplicate legs or boots, cell crossing, direction drift, or identity
  redesign.
- Failure of motion/edge/silhouette gates. A blocked candidate remains review
  evidence and is not exported as `.gsfpack`.
