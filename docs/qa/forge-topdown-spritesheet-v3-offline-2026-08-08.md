# topdown-spritesheet V3 offline acceptance — 2026-08-08

## Result

PASS for the offline implementation gate. Real xAI acceptance was not authorized
and was not executed.

## Verified contracts

- `topdown-spritesheet@3.0.0` plans 4 expected / 8 maximum image requests.
- `--validation-animation walk_right` plans 1 expected / 2 maximum requests.
- Fixture emits one 192×192 2×2 sheet per action and Core extracts four ordered cells.
- Full fixture run uses exactly four Provider requests and exports 16 frames.
- One-shot Provider failure retries the complete selected sheet and succeeds on request two.
- Four duplicate walk cells exhaust two attempts, return
  `animation_sheet_regeneration_required`, and export no Pack.
- Child `still` retry requests only `walk_right`; three other actions are reused.
- Child `consistency` replay reuses all actions and records zero Provider requests.
- Character direction/effect, equipment, silhouette, motion and Pack gates execute.
- Character Pack validates and records `animationSheet` provenance.
- Godot installs native `SpriteFrames`/`AnimatedSprite2D`/`AtlasTexture`, emits
  `filter_clip = true`, contains no `PackedByteArray`, stays below 1 MiB and
  loads headlessly on the local Godot 4 installation.

## Commands

```bash
cargo test -p core animation_sheet
cargo test -p providers --test spritesheet_generation_contract -- --nocapture
cargo check -p core -p providers -p forge-cli --features consistency-v2
```

## Remaining external gate

Run only after separate user authorization:

1. `walk_right` validation: one expected / two maximum xAI image edits.
2. Inspect the original 2×2 response, extracted frames, contact sheet and Godot playback.
3. If it passes without manual threshold changes, authorize the full four-action
   run at four expected / eight maximum image edits.
