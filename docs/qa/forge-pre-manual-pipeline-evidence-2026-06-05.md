# Forge Pre-Manual Pipeline Evidence

Recorded on 2026-06-05.

This evidence proves the local Rust processing/export/import pipeline for deterministic manual-QA fixtures before the interactive app QA session. It does not prove the Task 6 manual app QA gate, and it does not replace the required tester-selected real short video.

The sprite sheet fixture path uses the shared `forge_core::video::slice_sprite_sheet_grid` implementation that the Tauri command calls for app sprite-sheet slicing.

## Commands

```bash
npm run qa:fixtures
cargo test --manifest-path Cargo.toml --test sample_pipeline_tests "manual_qa_" -- --nocapture
```

## Results

```text
manual_qa_png_sequence_exports_schema_valid_reimportable_pack: pass
manual_qa_sprite_sheet_exports_schema_valid_reimportable_pack: pass
```

## Fixture Baseline

Fixture root:

```text
examples/inputs/manual-qa
```

PNG sequence hashes:

```text
a38ec220236333db5c7d3fce0e39050019e3f24d2cb9dbd7d918169590876802  examples/inputs/manual-qa/png-sequence/frame_001.png
2d941dddc98edebdc145b30f76f42bd80f749889f45ea940faa613ad8cca300d  examples/inputs/manual-qa/png-sequence/frame_002.png
3a253aeeb686328338b0cd9dce3429f6fcc340232d4f9b6e80d3025e08a11873  examples/inputs/manual-qa/png-sequence/frame_003.png
44819e7a22e96a3f96e5fa5dd83483e69d18914a1ef039082cca3172be27da65  examples/inputs/manual-qa/png-sequence/frame_004.png
0a3e3fbe8e3f594b714c789af19b00b6175e88d25819ebf39b045fb3115c5768  examples/inputs/manual-qa/png-sequence/frame_005.png
44819e7a22e96a3f96e5fa5dd83483e69d18914a1ef039082cca3172be27da65  examples/inputs/manual-qa/png-sequence/frame_006.png
```

Sprite sheet hash:

```text
e95f738ce415425d0a706043bec23ba9c9c0221a6e07e3b5cae7ba778d19b590  examples/inputs/manual-qa/sprite-sheet/forge-walk-sheet.png
```

Fixture manifest hash:

```text
fcd35b7220e486456878f12b7ce5faef61430fb9de92c4ea00db9c643b498843  examples/inputs/manual-qa/qa-fixtures.json
```

## Remaining Manual Gate

```text
Bundled sample video app QA: not recorded here
Real short video app QA: not recorded here
PNG sequence app QA: not recorded here
Sprite sheet app QA: not recorded here
Exported .gsfpack app re-import QA: not recorded here
Release decision: hold until interactive app QA and notarization gates pass
```
