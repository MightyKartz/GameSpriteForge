# Forge Vidu Q2 Godot cadence and edge fix

Date: 2026-08-19

## Reported problem

The user confirmed that the latest Vidu source video was visually correct, but
the previously shown Godot result did not walk correctly and retained colored
edge contamination.

## Root causes

1. The earlier handoff displayed a Godot preview from the first Q2 candidate
   (`e5c046…`) while the subsequently shared Vidu source was the second
   phase-lock Q2 candidate (`123d82…`). The artifacts were not siblings.
2. The second candidate's sparse 10-frame sampling contained a 208ms hold while
   its average hold was 100ms. That 2.08× cadence spike caused visible
   stop/jump playback in `AnimatedSprite2D`.
3. H.264 compression blended a thin magenta band into the subject outline.
   Component cleanup could remove detached text, but not spill connected to the
   character silhouette.

## Fix

- The replacement Godot preview is locked to candidate SHA-256
  `123d82fe5088a86083ab2dd689c9c5b9388181700341bb9ee09f666341007d38`.
- `source-cycle-sampling` now rejects sparse playback whose maximum hold is
  greater than 1.80× the average. This promoted the cycle from 10 to 12 native
  source frames.
- Reconstruction error improved from 10.47% to 6.04%.
- The longest hold dropped from 208ms to 125ms; the 12 output durations are
  `[125, 83, 42, 42, 83, 83, 42, 125, 125, 125, 83, 42]` ms.
- Magenta-key browser video cleanup applies a three-source-pixel alpha contraction
  before subject-component retention. At 1440→520 review scale this remains
  below one displayed pixel and removes the compressed colored rim.
- Translation-only normalization remains `game_ready`: 16px maximum source
  translation, zero clipped foreground, 1px residual center drift, and 0px
  residual foot drift.

## Output

- Godot project:
  `generated-assets/experiments/motion-video-browser-benchmark-20260819/vidu-q2-rewarded-v2-phase-lock/godot-v10-full-cycle-ground-lock/`;
- fixed MP4:
  `qa-output/vidu-q2-full-cycle-ground-lock.mp4`;
- candidate/source-cycle/replay provenance remains outside any production Pack;
  `productionPackWritten=false`.

The strict Motion gate still reports upper-body color flicker and phase-order
risk, so this remains a human-review preview rather than an authorized
production asset.

## Complete-cycle correction after human review

The user correctly rejected the 12-frame preview because gait selection had
accepted a 1000ms half-cycle. The source video returns to the same complete
left/right gait phase approximately every 2000ms. The corrected diagnostic
path now:

- rejects gait windows shorter than 75% of the MotionDriverLock-derived
  2000ms expected period;
- selects the game-ready source interval from frame 57 through closure frame
  105 (PTS 2375–4375ms);
- samples 24 original frames uniformly from indices 57, 59, …, 103;
- uses native alternating durations of 83/84ms for a 2000ms cycle;
- plays that complete cycle twice in the 4.2667-second MovieWriter proof;
- removes exposed magenta spill in concave silhouette gaps after alpha
  contraction.

The 24-frame result is diagnostic-only because the current production sampling
profile still caps delivery at 12 frames. It is intentionally not packed.

## Ground-lock correction after rejecting vertical scaling

Human review correctly rejected a vertical-scaling experiment because changing
each raster's height deforms the character. The final v10 path does not call
that optional diagnostic operation: every frame keeps `scaleX=scaleY=1.0`.

The translation-only anchor now measures the support-foot contact line from a
robust high percentile of lower-body column bottoms. It ignores a lifted foot,
thin detached pixels, and ordinary body-box extremes.

- target support-foot ground line: 1316px;
- output support-foot ground drift: 0px;
- maximum integer translation: 19px on the 1440px source canvas;
- vertical or horizontal raster scaling: none;
- stabilization verdict: `game_ready`.

Natural head and torso motion from the source is preserved. No attempt is made
to align the head by moving the feet away from the ground plane.

## Cyclic support-contact correction after v10 review

Human review found one remaining perceived lift while the right foot extended.
The v10 detector evaluated each frame independently, so a support-leg handoff
could change which sole supplied the ground observation. A diagnostic
full-width mode made the defect explicit: the selected contact jumped 38px
between frames 2 and 3.

The v13 anchor replay replaced that local decision with a deterministic cycle-wide
track. It retains multiple lower-body sole candidates per frame, then selects
one closed 24-frame path using candidate support, adjacent-frame velocity,
acceleration, and the last-to-first transition. The output remains translation
only; no scale, warp, redraw, provider request, or Pack write occurs.

The final v15 replay retains that anchor track and also closes thin exposed
magenta components iteratively. This removes a compressed background-colored
strip that continued into the enclosed gap between the boots after its exposed
tip was removed. Across the 24 frames 4,079 spill pixels were cleared; frame 23
accounted for 555. At the 520px Godot review scale the former bright strip is no
longer visible.

- candidate SHA-256: `123d82fe5088a86083ab2dd689c9c5b9388181700341bb9ee09f666341007d38`;
- selected source frames: 57, 59, ..., 103 (unchanged);
- target tracked ground line: 1311px;
- maximum integer translation: 14px, down from 19px in v10;
- total vertical translation range: 24px, down from 34px in v10;
- maximum circular adjacent-frame step: 6px on the 1440px source;
- last-to-first closure step: 2px;
- output tracked-ground drift: 0px;
- clipped foreground: 0px;
- vertical/horizontal raster scaling: none;
- anchor-focused tests: 9 passed;
- cleanup-focused tests: 15 passed;
- full core unit suite: 319 passed;
- `cargo fmt --all -- --check`: passed;
- `cargo clippy -p core --lib -- -D warnings`: passed;
- Godot 4.6.3 import and 128-frame MovieWriter run: passed.

The immutable final v15 Godot review project is
`generated-assets/experiments/motion-video-browser-benchmark-20260819/vidu-q2-rewarded-v2-phase-lock/godot-v15-cyclic-support-spill-closure/`,
and its MP4 is
`qa-output/vidu-q2-cyclic-support-spill-closure.mp4`. The motion and identity production
gates remain blocked, so `productionPackWritten=false`.

## V16 visual-root correction and source-height gate

Further human review correctly found that v15 still rose during the right-foot
extension. The cyclic support track was no longer switching feet, but the
source raster itself grew around its torso: the measured body-height span was
47 source pixels and the maximum relative deviation from the median was
2.957%, above the new translation-only limit of 1.5%.

The implementation now fails closed on this condition with
`source regeneration is required`. It also provides a diagnostic visual-root
mode which locks the middle of the measured silhouette using integer Y
translation only. No frame is scaled, warped, redrawn, or cropped.

- v15 right-foot-extension apparent rise (frames 18→22): 28 source pixels;
- v16 right-foot-extension apparent rise: 14 source pixels;
- visual-root residual drift: 1px;
- v16 top range: 23px, down from 48px;
- resulting foot-baseline range: 24px, explicitly exposed as the unavoidable
  tradeoff of a translation-only correction;
- body-height drift: unchanged at 45–47px, therefore blocked;
- provider requests: 0;
- production Pack written: false.
- root/height focused tests: 12 passed;
- full core unit suite: 322 passed;
- fmt and clippy (`core --lib --examples`): passed;
- visual-root JSON Schema validation: passed;
- Godot 4.6.3 import and 128-frame MovieWriter run: passed.

The immutable diagnostic project is
`generated-assets/experiments/motion-video-browser-benchmark-20260819/vidu-q2-rewarded-v2-phase-lock/godot-v16-visual-root-lock-height-gate/`.
It is an improved review aid, not a production promotion. A source with stable
character scale is still required to lock both the perceived root and the
ground plane.

## V17 native-placement correction after source-video review

Human review accepted the original Vidu Q2 task shown at 12:23 as visually
correct. That task is now the authoritative motion source, with prompt hash
`cf110e6e8bd503200a8053421ca9f37132e2a5a2b0887600a4140a7b168a8861`
and source hash
`123d82fe5088a86083ab2dd689c9c5b9388181700341bb9ee09f666341007d38`.

The review showed that v15/v16 changed a source whose native framing was
already acceptable. V17 therefore performs only deterministic magenta cleanup
and retains the 24 selected native coordinates and PTS-derived timing.

- source frames: 57, 59, ..., 103;
- cycle boundary: frames 57–105, 2375–4375ms;
- frame count: 24;
- translation X/Y: 0px on every frame;
- scale X/Y: 1.0 on every frame;
- crop, warp, root lock, and ground lock: none;
- native coordinates preserved: true;
- torso area drift: 3.191%, accepted under the 12% limit;
- torso width drift: 8.658%, slightly above the 8% limit;
- native placement verdict: blocked pending human review;
- provider requests: 0;
- production Pack written: false.

The durable prompt record is
`docs/qa/forge-vidu-q2-1223-authoritative-prompt-2026-08-19.md`. The v17 Godot
project is
`generated-assets/experiments/motion-video-browser-benchmark-20260819/vidu-q2-rewarded-v2-phase-lock/godot-v17-native-placement-passthrough/`.
