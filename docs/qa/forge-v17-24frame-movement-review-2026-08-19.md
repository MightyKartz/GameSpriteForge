# Forge V17 24-frame movement review

Date: 2026-08-19

Status: items 1–3 implemented. Candidate Pack and real movement scene pass
technical validation; human review remains pending and no production Pack was
promoted.

## 1. Hash-bound human review

`animation-human-review@1.0.0` binds the authoritative source/prompt hashes,
the replay-summary hash, all 24 PNG hashes, and all 24 frame durations. The
record is intentionally `pending`; every human check is `null` until the user
reviews the moving scene. Changing one frame makes closure validation fail.

Review record:
`generated-assets/experiments/motion-video-browser-benchmark-20260819/vidu-q2-rewarded-v2-phase-lock/review-v17/animation-human-review.json`.

## 2. 24-frame delivery

- legacy 8/10/12 source-cycle reports remain supported;
- `source-cycle-sampling@1.1.0` supports 16/24 frames and an exact reviewed
  24-frame policy;
- single-animation Pack export now preserves explicit per-frame durations and
  rendering policy;
- manifest: 24 frames, 24 durations, 2000ms total, 12fps;
- six external sprite-sheet PNGs keep textures below the configured 4096px
  limit;
- Pack includes the pending human-review record and validates successfully;
- Pack is explicitly a candidate with `productionEligible=false`.

Candidate Pack:
`generated-assets/experiments/motion-video-browser-benchmark-20260819/vidu-q2-rewarded-v2-phase-lock/delivery-v17-24frame-candidate-v2/v17-walk-right-review/V17-Walk-Right-Review.gsfpack`.

The first immutable candidate attempt was rejected because `explicit_left`
requires left-direction animations. The v2 candidate uses the compatible
single-direction `auto` policy; no failed output was overwritten.

## 3. Real Godot movement test

The Pack was installed through the Forge Godot installer. The scene uses a
real `CharacterBody2D` moving right at 120px/s, a fixed ground line, shared
source pivot `(720,1316)`, a 54×88 collision rectangle, and a 0.28 review
scale. Cyan and orange overlays expose pivot and collision placement.

Runtime assertions verify:

- `walk_right` exists and has exactly 24 frames;
- every installed duration reconstructs the expected 83/84ms sequence;
- external textures load through native `SpriteFrames`;
- no embedded Image, Base64, or PackedByteArray payload exists;
- `.tres/.tscn` files remain below 1 MiB.

MovieWriter produced 203 frames at 30fps / 6.766667 seconds. The first writer
attempt failed because its target directory did not yet exist; v2 pre-created
the directory and passed without overwriting the failure evidence.

Project:
`generated-assets/experiments/motion-video-browser-benchmark-20260819/vidu-q2-rewarded-v2-phase-lock/godot-v18-right-movement-review/`.

Video:
`qa-output/v18-walk-right-movement-review-v2.mp4`.

## Verification

- Core unit tests: 328 passed;
- Pack tests: 46 passed;
- Core lib/examples clippy: passed;
- Pack all-targets clippy: passed;
- full workspace all-targets clippy remains blocked by the pre-existing
  `packages/core/tests/chroma_tests.rs` `field_reassign_with_default` warning;
- no Provider requests occurred;
- production Pack written: false.

## Subsequent approval

After reviewing the v18 movement scene, the user explicitly confirmed
`v18 六项通过`. The pending record above remains unchanged. Its exact hash
closure was promoted into a separate approved review and production Pack; see
`docs/qa/forge-v18-approved-walk-right-production-2026-08-19.md`.

