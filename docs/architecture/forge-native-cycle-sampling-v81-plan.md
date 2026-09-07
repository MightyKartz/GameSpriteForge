# Forge Native Cycle Sampling V8.1

Status: implementation in progress inside the unreleased
`topdown-direction-motion@8.0.0` workflow. V7 and earlier workflows retain
their existing extraction, loop-selection, and timing behavior.

## Problem

A four-second xAI video is normally encoded at 24 FPS and contains about 97
decoded frames. The old V8 draft reduced the full clip to at most 12 FPS before
cycle detection and then exported every frame in the selected interval. That
mixes two separate decisions:

1. which source interval is one complete, closed motion cycle;
2. which source poses Godot needs to reproduce that cycle economically.

Early temporal decimation can discard a foot contact, passing pose, motion
extreme, or clean closure witness. Exporting the entire selected interval can
then produce an unnecessarily large or inconsistently timed animation.

## V8.1 contract

```text
source video (native PTS, at most 24 FPS / 120 frames)
-> low-resolution foreground analysis
-> complete cycle selection over the full clip
-> adaptive source-pose selection inside [start, end-boundary)
-> source-PTS frame durations
-> shared animation normalization
-> SpriteFrames / Godot
```

- The first second is a candidate, not a privileged interval.
- The end-boundary frame proves closure and is never exported twice.
- Cycle detection sees all retained native-time frames before output reduction.
- Output reduction is deterministic and occurs only after a cycle is selected.
- V8.1 exports 8, 10, or 12 frames. It starts with eight, measures the maximum
  feature reconstruction error of skipped source poses, and increases the
  output count only when required.
- The offline V8 biped fixture calibrates the V1 reconstruction ceiling to
  12%. A twelve-frame result above that ceiling fails closed; the threshold is
  not a claim of real-provider calibration and must be revisited with frozen
  real clips before release.
- The sampler always preserves the cycle start. The closure boundary remains a
  virtual reconstruction anchor, while optional semantic phase indices are
  immutable inputs to the sampler.
- Every output frame is an original decoded frame. Forge does not synthesize or
  interpolate pixels.
- Per-frame durations are derived from original PTS gaps, so preview, Pack, and
  Godot share one timing source and `playbackSpeedRatio` remains `1.0`.
- Analysis uses reduced foreground-aware features; matting and export use only
  the selected full-resolution frames.
- V8 cycle discovery does not apply per-frame foot/bounding-box alignment.
  Natural body rise, weight shift, and foot travel remain visible to the cycle
  and sampling gates. Final delivery still uses one shared scale and one stable
  anchor per animation.

## Versioned evidence

Each V8 walk writes `source-cycle-sampling-report.json` with profile
`source-cycle-sampling@1.0.0`, the source interval, mandatory indices, output
indices, source timestamps, normalized reconstruction error, selected frame
count, and reason for choosing 8, 10, or 12 frames. The loop report continues
to carry the final output indices and exact timing consumed by the Pack.

## Compatibility and safety

- No Provider request contract changes: image locks and four I2V requests remain
  separate, explicitly authorized stages.
- Local `loop`, `matting`, and `consistency` replay remains zero-cost and never
  checks credentials.
- V7 and earlier workflows retain their old 12 FPS candidate ceiling.
- V8 source extraction is capped at 24 FPS and 120 frames to bound memory and
  quadratic loop-analysis cost.
- A sampling report that references the closure boundary, an out-of-window
  frame, a duplicate frame, or fewer than eight frames is a hard local error.

## Offline gates

- Native 24 FPS extraction retains the source PTS sequence before selection.
- Complete cycles in the middle of a clip are selectable; the first second has
  no special score.
- Eight frames are retained for simple uniform motion; complex in-between
  motion promotes deterministically to ten or twelve frames.
- Required semantic phase frames cannot be removed.
- The closure boundary is excluded and all output indices are strictly
  increasing.
- Output durations sum to the selected source interval and are consumed without
  retiming by GIF, Pack, and Godot.
- V8 fixture image review, complete stage, local replay, Pack validation, and
  Godot 4.6 headless load pass with zero real Provider requests.
