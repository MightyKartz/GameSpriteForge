# Forge V8.1 retained-video zero-cost replay — 2026-08-10

## Scope

- Reused the retained real xAI V7 Job `cb00d5b8-a2e2-449c-b2e3-a145426d8358`.
- No Provider health check, credential lookup, Keychain access, or media request was allowed.
- Reprocessed `walk_up`, `walk_right`, and `walk_down` from their original 720×720, 24 FPS, 97-frame, 4.041667-second MP4 files.
- Reused the retained transparent static idle image.
- The V7 delivery contract remains `idle`, `walk_up`, `walk_right`, and `walk_down`; Godot derives left movement by flipping `walk_right`.

## Input evidence

| Input | SHA-256 |
| --- | --- |
| `walk_up/attempt-1/animation.mp4` | `4e2a2d6318bf3d8023e695e49213063aa948d6c486a6abf2388e41baef768a15` |
| `walk_right/attempt-1/animation.mp4` | `0e6d3b57a6fcb211ffa988ebb06439db85697207b76bf81295e7440c9f3bde50` |
| `walk_down/attempt-1/animation.mp4` | `c7bc813db40f9d93850c366e9e5d249abbe74ed452c2b2d85a8289291fc1e846` |
| `idle/static-idle.png` | `e6d38bd5ca02c102199083882850628b543390412478e9bbc93dfbe3a2635418` |

## Durable execution

- Final Job: `83e2984f-85cf-44c8-9782-c09f559b1d47`
- Job directory: `generated-assets/forge-native-cycle-v81-existing-video-20260810/jobs/83e2984f-85cf-44c8-9782-c09f559b1d47`
- Input fingerprint: `ad6f1f48ddf876eb1966cdc67cec407575b59fb10ff88c9f8d4a20d5013e2a19`
- Recipe hash: `56b8b60d7da297c1da3f28a0a1f1f69f0196bcfb653aba96fb98214332a543c3`
- Plan estimate: 0 expected / 0 maximum Provider requests.
- Execution lifecycle: `succeeded` as a diagnostic preview.
- Pack step: explicitly skipped; a preview-only Job can never export or register a Pack.

The first strict execution stopped immediately when `walk_up` could not satisfy the 12% reconstruction budget with 12 frames. The final diagnostic execution retained the best allowed 12 original source frames per walk so the failure could be inspected without weakening the release gate.

## V8.1 native-PTS results

| Animation | Source cycle | Selected source frames | Playback | Reconstruction error | Gate | Other blockers |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| `walk_up` | 25 frames, `62..87` | 12 | 1042 ms | 15.850% | blocked (`>12%`) | wrap transition discontinuity |
| `walk_right` | 17 frames, `67..84` | 12 | 708 ms | 19.135% | blocked (`>12%`) | anchor closure drift, gait/foot-lobe failure |
| `walk_down` | 24 frames, `63..87` | 12 | 1000 ms | 18.683% | blocked (`>12%`) | no additional loop blocker |

All three reports record `decision: maximum_count_required`. Closure boundary frames were used for loop proof and were not exported. Frame durations come from the selected source PTS, not a guessed FPS.

## Visual evidence

- Contact sheet: `contact-sheet-v81-preview.png`; rows are `walk_up`, `walk_right`, `walk_down`.
- Corrected GIF previews:
  - `walk_up`: 1.040 seconds for a 1.042-second PTS cycle.
  - `walk_right`: 0.710 seconds for a 0.708-second PTS cycle.
  - `walk_down`: 1.000 seconds for a 1.000-second PTS cycle.
- GIF delay conversion uses cumulative centisecond quantization so per-frame rounding does not accumulate speed drift.

## Godot verification

- Isolated project: `generated-assets/forge-native-cycle-v81-existing-video-20260810/godot-project`
- Godot: `4.6.3.stable.official.7d41c59c4`
- Headless import: passed for all 38 external PNG frames.
- Headless scene run: passed with zero resource or script errors.
- Live playback: opened in Godot and left running for review.
- Two screenshots 350 ms apart had different SHA-256 values, proving frames advanced.
- The QA scene uses `AnimatedSprite2D` plus `SpriteFrames`; relative frame durations reproduce the selected source PTS.
- The project is isolated and is not written to a production `.forge/assets.json` catalog.

## Conclusion

Native 24 FPS inspection and source-PTS playback fix the old “blindly reduce to eight frames and force a guessed speed” behavior. They do not make these retained videos release-ready: all three walks require more than 12 frames to meet the current reconstruction budget, and `walk_up`/`walk_right` also have genuine source-cycle defects. The correct next decision is to revise the adaptive frame budget and candidate scoring using this evidence, not lower unrelated identity or silhouette thresholds.
