# Codex external keyframes and Godot rendering contract — offline QA

Date: 2026-08-17

Status: implementation complete; offline/fixture and Godot 4.6 gates passed.
No Image API, video API, xAI, or other media Provider request was made.

## Delivered

- Added experimental zero-request workflow
  `topdown-external-keyframes@11.0.0`.
- Added the low-level request template
  `examples/cli/character-external-keyframes-v11.json`.
- Added strict independent-PNG validation: real Alpha, one shared square
  canvas, unique hashes, exact idle/walk frame counts and cadence, explicit
  mirror policy, top-down camera and character semantic context.
- Reused shared canvas/feet-anchor normalization and enabled character
  direction, motion, equipment and silhouette gates for external frames.
- Added `godot-sprite-rendering@1.0.0` to Engine Manifest and Godot helper.
- Added Pack fail-closed validation for rendering agreement, pixel-safe
  anchors and mirror animation closure.
- Updated the Godot 4.6 installer to apply nearest/linear filtering, disable
  texture repeat, avoid half-pixel centering for snapped assets, and retain
  external atlas textures with `filter_clip=true`.
- Added explicit/mirrored direction playback to `forge_usage.json`.

## Verification completed

- `cargo check -p core`: passed.
- `cargo check --workspace --all-targets`: passed.
- `cargo test -p core --test automation_tests`: 19/19 passed.
- `cargo test -p core export::`: 19/19 passed.
- `cargo test -p pack`: 46/46 unit/integration tests passed at the recorded
  intermediate gate (6 library + 40 integration).
- Focused explicit/mirrored direction playback tests: passed.
- `scripts/test-cli-product.sh`: passed, including real Godot 4.6 headless
  import/install, sub-1 MiB external `SpriteFrames`, no embedded Image or
  `PackedByteArray`, `texture_filter = 1`, and `centered = false` for the
  nearest/pixel-snapped fixture.
- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo test --workspace`: passed with exit code 0; every runnable unit,
  integration and documentation test passed. One retained-real-job replay was
  ignored by design and no Provider request was made.
- JSON syntax validation for the workflow profile, request example and manifest
  schema: passed.
- `git diff --check`: passed.

Fixture success does not claim any real media-generation acceptance.

## Scope boundary

- This work does not make Codex subscription image generation callable from
  the Forge CLI.
- It does not approve the rejected 2026-08-16/17 direction-grid or cape-edit
  candidates.
- It does not add gameplay movement, collision, AnimationPlayer events or
  CharacterBody2D control logic.
- It does not mutate source Jobs, existing Packs, catalog entries or Godot
  projects outside the temporary CLI product test.
