# Forge Native Cycle Sampling V8.1 — Offline Acceptance

Date: 2026-08-10  
Scope: deterministic processing, fixture Provider, Pack, and Godot only  
Real Provider requests: **0**

## Result

The offline V8.1 gate passes. Forge now analyzes the retained native source
timeline before reducing a selected complete cycle to a Godot `SpriteFrames`
sequence.

Retained Ayla V7 source-video probe evidence:

| Animation | FPS | Frames | Duration |
| --- | ---: | ---: | ---: |
| `walk_up` | 24 | 97 | 4.041667 s |
| `walk_right` | 24 | 97 | 4.041667 s |
| `walk_down` | 24 | 97 | 4.041667 s |

The old V8 draft used a 12 FPS ceiling before cycle selection. V8.1 retains
source PTS up to 24 FPS / 120 frames, searches the full clip, and only then
selects 8, 10, or 12 original frames.

## Implemented gates

- `source-cycle-sampling@1.0.0` deterministic adaptive sampler.
- Strict output cardinality: only 8, 10, or 12 frames.
- The selected cycle start is always the first output frame.
- The closure boundary proves the loop but is never exported.
- Optional semantic phase indices are mandatory outputs.
- Twelve frames that still exceed the reconstruction budget fail closed and
  cannot produce a Pack.
- The V1 reconstruction ceiling is 12%, calibrated only against the frozen
  deterministic biped fixture. Real-provider calibration remains a release
  gate; Forge does not silently accept an over-budget twelve-frame cycle.
- Source PTS is strictly increasing; missing, non-numeric, negative, or
  duplicate decoded PTS fails closed.
- A 30 FPS timeline is evenly reduced to approximately 24 FPS instead of the
  previous greedy 15 FPS failure mode.
- Output frame duration is the exact next selected PTS delta; the last output
  duration reaches the selected closure boundary.
- Preview, Pack manifest, Godot helper, and `SpriteFrames` share that timing.
- V8 cycle analysis no longer uses per-frame foot/bbox normalization before
  selection. Final delivery uses shared scale and a stable per-animation
  anchor.
- Biped semantic phase selection is enabled only for
  `biped_walk@1.0.0`; freeform, quadruped, flying, and slither profiles do not
  inherit human limb requirements.
- Every V8 Pack carries four hash-bound sampling reports under
  `provenance/source-cycle-sampling/`.
- Pack validation requires that provenance whenever a DirectionMotionLock is
  present and validates report schema, path, SHA-256, field relationships,
  mandatory phases, exact PTS-derived per-frame durations, and source-cycle
  duration.

## Verification

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `cargo test -p core`: pass, including native 24 FPS extraction, 30 FPS
  reduction, PTS failure handling, adaptive 8/10/12 selection, reconstruction
  budget, mandatory phases, closure boundary, and source timing.
- `cargo test -p pack`: pass, including missing provenance, same-total/wrong
  per-frame timing, missing cycle start, report hash, and timing tampering.
- `cargo test -p providers --test direction_motion_generation_contract`: pass.
  The two-stage V8 fixture creates and approves the immutable eight-image Lock,
  generates four fixture videos, exports eight Godot animations, validates the
  Pack, rejects a tampered sampling report, and completes zero-cost local
  replay. Biped phase preservation is covered independently by the gait and
  adaptive-sampler contracts; real-provider biped calibration remains pending.
- `cargo test -p providers --test video_cycle_generation_contract`: pass for
  V6 and V7 compatibility; the retained real-job test remains explicitly
  ignored and performs no Provider work.
- `cargo test --workspace`: pass before the final stricter Pack-validator
  assertions; the affected Core/Pack/V8 focused gates were rerun after those
  assertions.
- Godot 4.6 headless import/load: pass inside the V8 direction-motion contract.
- `scripts/test-cli-product.sh`: pass.
- `.tres` remains below 1 MiB and contains no `PackedByteArray`.

## Safety notes

- No xAI credential, Keychain item, OAuth token, authorization header, or
  Provider endpoint was accessed.
- No media-generation authorization was consumed.
- The original Job and source videos were not modified.
- The local Cargo `target` build cache was cleared after the data volume reached
  100% capacity. Only reproducible build artifacts were removed; source,
  JobStore, generated media, Packs, and user files were preserved.

## Remaining real gate

Offline acceptance does not prove that a real xAI video contains a valid cycle
or stays within the 12-frame reconstruction budget. The next paid gate should
first reuse already generated videos through a zero-cost local analysis where
compatible. Only after reviewing the selected source range, contact sheet, GIF,
and Godot playback should a new single-direction video request be authorized.
