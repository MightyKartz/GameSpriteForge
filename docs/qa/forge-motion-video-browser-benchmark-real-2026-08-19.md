# Forge motion-video browser benchmark — real run

Date: 2026-08-19

Status: browser acquisition, locked native PTS intake, complete-cycle
selection, FrameRonin-style cleanup/normalization, Motion/Identity QA, and
Godot previews implemented and verified. No production `.gsfpack` was written.

## Provider and quota audit

All calls used existing web subscriptions or explicitly rewarded/free quota;
no cash purchase and no official paid Seedance API call occurred.

- Vidu Q3: one displayed free generation; credits stayed `25 → 25`.
- Vidu Q2 candidate 1: cost 10, immediate newcomer reward 10; `25 → 15 → 25`.
- Vidu Q2 candidate 2: cost 10, immediate newcomer reward 10; `25 → 15 → 25`.
- Kimi: one Allegro `视频生成` plugin call. Kimi exposed neither the underlying
  video model name nor numeric before/after quota, so both remain explicitly
  unknown in provenance rather than being inferred.

## Candidate results

### Vidu Q3 free

The 1440×1440, 24fps, 97-frame source alternated among green, pale-green and
white backgrounds. Deterministic subject cleanup eventually produced no
foreground at source frame 95, so gait selection stopped before sampling. This
candidate is blocked.

### Vidu Q2 rewarded, ordinary-walk prompt

The 1440×1440, 24fps, 122-frame source retained the solid magenta background.
The final pipeline decoded all 122 original PTS frames, removed detached Vidu
text components, selected source frames 47–63 (PTS 1958–2625ms), and exported
12 native poses for a 709ms loop.

- gait: `game_ready` (phase order 0.9053, closure 0.8676, foot lobes 2);
- reconstruction: 0.08594 ≤ 0.12;
- translation-only stabilization: `game_ready` (28px on a 1440px source,
  0 clipped foreground, residual center 1px, foot baseline 0px);
- same-direction Identity: `game_ready` (minimum 0.9523, mean 0.9691,
  threshold 0.95);
- Motion: `blocked` because upper-body color flicker is 0.1089 (>0.08) and
  Motion phase order is 0.3762 (<0.45).

This is the recommended human-review candidate, but it is not production
eligible.

### Vidu Q2 rewarded, explicit A/B phase prompt

The second Q2 prompt raised gait phase order to 0.9874 and Motion phase order
to 0.4232, but did not reach 0.45. Upper-body color flicker rose to 0.1188 and
the final two identity frames scored 0.9480/0.9404. It is inferior to the first
Q2 candidate overall and remains blocked.

### Kimi Allegro plugin

The downloaded 960×960, 24fps, 97-frame silent source looks coherent before
matting, but its green background overlaps the olive cloak palette. Gait passes
after component cleanup, while adaptive native sampling requires 36.21%
reconstruction error versus the 12% ceiling. The diagnostic replay also shows
cloak alpha/color damage; Identity has one frame below 0.95. This candidate is
blocked and demonstrates why the provider input must contain an actual
palette-safe solid background, not merely request one in text.

## Godot evidence

Each preview uses native `SpriteFrames`, original-PTS-derived per-frame
durations, OpenGL Compatibility MovieWriter, and `productionPackWritten=false`.

- recommended Q2 preview:
  `generated-assets/experiments/motion-video-browser-benchmark-20260819/vidu-q2-rewarded/godot-v2-magenta-despill/qa-output/vidu-q2-rewarded-v2.mp4`;
- phase-lock Q2 preview:
  `generated-assets/experiments/motion-video-browser-benchmark-20260819/vidu-q2-rewarded-v2-phase-lock/godot-v19-approved-production/qa-output/v19-approved-walk-right.mp4`;
- Kimi diagnostic preview:
  `generated-assets/experiments/motion-video-browser-benchmark-20260819/kimi/godot-diagnostic-v2-subject-components/qa-output/kimi-diagnostic-v2.mp4`.

Machine-readable comparison:
`generated-assets/experiments/motion-video-browser-benchmark-20260819/comparison.json`.

## Decision

The architecture and implementation direction are correct only with the new
locks and fail-closed gates. Video generation is useful for motion proposals,
but a visually plausible MP4 is not a Godot-ready asset. The current best
candidate still requires human review and a subsequent repair/regeneration
decision. No production Pack is authorized from this run.
