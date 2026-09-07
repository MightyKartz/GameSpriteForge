# Character timing, equipment, and keyframe V2.1 offline acceptance

Date: 2026-08-08

Result: implementation accepted offline; real xAI acceptance not run and not implied.

## Covered

- `animation-timing@1.0.0`: shortest-strong fundamental/harmonic selection, timestamp
  and safely retimed per-frame duration generation, non-uniform intervals, GIF
  duration conversion.
- Godot installers: `frameDurationsMs` length/positive validation and per-frame
  relative duration application with external atlas textures.
- `hand-equipment-contact@1.0.0`: stable grip, finger-like protrusion, and unarmed
  fixtures; Pack schema and validator integration.
- Provider authorization: atomic transport + model reservation, separate model and
  operation counts, zero transport cost reservation, safe release of an unsubmitted
  model attempt, and xAI upload/edit contract.
- `topdown-keyframes@2.1.0`: four anchors followed by four adjacent-keyframe
  in-betweens, typed workflow dependencies, Pack/catalog completion, one-frame retry,
  and zero-request local replay.

## Evidence commands

```text
cargo test -p core quality::loop_selection --lib
cargo test -p core equipment_contact --lib
cargo test -p core export::gif::tests --lib
cargo test -p providers authorization::tests --lib
cargo test -p providers durable_video_edit_upload_does_not_consume_the_model_retry_slot --lib
cargo test -p providers --test keyframe_generation_contract
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/test-cli-product.sh
scripts/test-stage3-static.sh
```

All commands passed locally. The complete workspace included 179 Core unit tests,
project-build regressions, 32 Provider unit tests, Character/video and keyframe
contracts, Pack validation, static assets, and deterministic world assets. The CLI
product contract installed its fixture Character Pack through the Godot plan/execute
path and verified a small external-texture `SpriteFrames` resource.

The shared fixture and the Stage 2 project-build mirror were corrected to move both
legs symmetrically around the fixed body center. This preserves the meaning of a
`game_ready` fixture under `silhouette-temporal@2.0.0`; no production threshold was
lowered. No xAI credential or paid request was used.
