# Forge Manual MVP QA Checklist

Date: 2026-06-05

Note: this checklist captures the 2026-06-05 manual QA session. The current 2026-06-06 release candidate is `release-candidates/GameSpriteForge-0.1.0-aarch64-0a5d18c3d1c8-notarized`; see `PRODUCT.md`, `docs/architecture/distribution-mvp.md`, and `docs/qa/local-workbench-ux-evidence-2026-06-06.md`.

## Scope

This checklist verifies the local import-only Game Sprite Forge MVP against real local assets. It covers the enabled source providers only:

```text
import_video
import_frames
import_sprite_sheet
import_gsfpack
```

The checklist verifies local import, preview, extraction or slicing, processing, quality reporting, export, validation, and re-import. It does not cover AI generation, BYOK provider settings, website setup, online registry, marketplace, MCP, Codex Skill integration, hosted credits, cloud upload, cloud processing, creator publishing, or game-engine-specific exporters.

Use this document during a manual app session. Every result must be recorded as `Pass`, `Fail`, `Blocked`, or `Not applicable`. Every `Fail` or `Blocked` result must include the exact visible error text, missing file path, or blocked release-gate reason.

## Environment

Recorded values for the 2026-06-05 installed-app session:

| Field | Recorded Value |
| --- | --- |
| App | `/Applications/Game Sprite Forge.app` |
| DMG | `target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg` |
| Bundle identifier | `dev.gamespriteforge.desktop` |
| Tester | Codex via computer-use |
| Machine | MacBook Air Mac16,12, Apple M4, arm64, 16 GB |
| macOS version | macOS 26.5.1, build 25F80 |
| ffmpeg path | `/opt/homebrew/bin/ffmpeg` |
| ffprobe path | `/opt/homebrew/bin/ffprobe` |
| Default output folder | `/Users/kartz/Game Sprite Forge/Exports` |
| Jobs folder checked | `~/Library/Application Support/Game Sprite Forge/jobs/` |
| Export folder checked | `/Users/kartz/Game Sprite Forge/Exports` |

Launch command:

```bash
open -n "/Applications/Game Sprite Forge.app"
```

Optional local artifact checks before the manual run:

```bash
shasum -a 256 "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
hdiutil verify "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
codesign --verify --deep --strict --verbose=2 "/Applications/Game Sprite Forge.app"
```

Current local artifact status for this notarized candidate:

```text
DMG SHA-256: 218a5f9adf1fb47e5f744efbda357470ac8c6aa2c065451c926842c35cd0c739
Notarization submission: 7792d837-5da7-46da-8da4-33b559dda6cc
hdiutil verify: pass
codesign installed app: pass
stapler validate: pass
Gatekeeper public-release assessment: accepted, source=Notarized Developer ID
mounted-DMG launch: pass
release candidate package: release-candidates/GameSpriteForge-0.1.0-aarch64-218a5f9adf1f-notarized
```

## Asset Matrix

| Source Type | Input Requirement | Run Record | Expected Result | Result |
| --- | --- | --- | --- | --- |
| Bundled sample video | `/Users/kartz/Development/Forge/examples/inputs/green-box-character.mp4` | Exported `/Users/kartz/Game Sprite Forge/Exports/Green-Box-Character-Pack-1780625648745/Green-Box-Character-Pack.gsfpack`; 24 frames; `game_ready`; validated in app | Import, probe, preview, extract, process, export, validate, and re-import | Pass |
| Real short video | Tester-selected local `.mp4`, `.mov`, or `.webm`, 1-15 seconds, with visible foreground motion or a distinct object | `/Applications/Wacom Tablet.localized/Wacom Center.app/Contents/Resources/Gesture3FingerTap.mp4`; probed 300 x 200, 29.97 FPS, 1.17 sec; `Every=4`; exported `/Users/kartz/Game Sprite Forge/Exports/Real-Short-Video-UITest-1780628419646/Real-Short-Video-UITest.gsfpack`; 8 frames; `game_ready`; validated in app | Import, probe metadata, extract chosen range, process, export, and validate | Pass |
| PNG sequence | `/Users/kartz/Development/Forge/examples/inputs/manual-qa/png-sequence/frame_001.png` through `frame_006.png` | Exported `/Users/kartz/Game Sprite Forge/Exports/Multiline-PNG-UI-Test-1780626245133/Multiline-PNG-UI-Test.gsfpack`; 6 frames; `game_ready`; validated in app | Import copied frames, preview timeline, process, export, validate, and re-import | Pass |
| Sprite sheet | `/Users/kartz/Development/Forge/examples/inputs/manual-qa/sprite-sheet/forge-walk-sheet.png` | Grid `columns=4 rows=2 cellWidth=64 cellHeight=64`; exported `/Users/kartz/Game Sprite Forge/Exports/Manual-Sprite-Sheet-UI-Test-1780625791782/Manual-Sprite-Sheet-UI-Test.gsfpack`; 8 frames; `game_ready`; validated in app | Slice grid, preview cells, process, export, validate, and re-import | Pass |
| Exported `.gsfpack` | `/Users/kartz/Game Sprite Forge/Exports/Multiline-PNG-UI-Test-1780626245133/Multiline-PNG-UI-Test.gsfpack` | Imported as a source, processed, exported `/Users/kartz/Game Sprite Forge/Exports/Reimported-GSFPack-UI-Test-1780626657130/Reimported-GSFPack-UI-Test.gsfpack`; re-export preserved 6 frames; validated in app | Validate, import, preview GIF, preserve frame count, and re-export | Pass |

## Deterministic Local Fixture Prep

Before the manual session, run:

```bash
npm run qa:fixtures
npm run qa:pipeline
```

This prepares deterministic local fixtures under:

```text
/Users/kartz/Development/Forge/examples/inputs/manual-qa/
```

Use these fixture paths for the PNG sequence and sprite sheet rows if no better local real assets are available:

```text
PNG sequence folder:
/Users/kartz/Development/Forge/examples/inputs/manual-qa/png-sequence/

Sprite sheet file:
/Users/kartz/Development/Forge/examples/inputs/manual-qa/sprite-sheet/forge-walk-sheet.png

Sprite sheet grid:
columns=4 rows=2 cellWidth=64 cellHeight=64 expectedFrames=8
```

The fixture prep also creates safe failure-state inputs:

```text
mixed-size sequence:
/Users/kartz/Development/Forge/examples/inputs/manual-qa/failure-states/mixed-size-sequence/

corrupt frame sequence:
/Users/kartz/Development/Forge/examples/inputs/manual-qa/failure-states/corrupt-frame-sequence/

invalid .gsfpack:
/Users/kartz/Development/Forge/examples/inputs/manual-qa/failure-states/invalid-pack.gsfpack

not-video file:
/Users/kartz/Development/Forge/examples/inputs/manual-qa/failure-states/not-video.txt

blocked-quality single-frame input:
/Users/kartz/Development/Forge/examples/inputs/manual-qa/png-sequence/frame_001.png

invalid sprite sheet grid using the valid sheet:
columns=5 rows=2 cellWidth=64 cellHeight=64
```

These generated fixtures do not satisfy the required real short video row. The real short video must still be a tester-selected local `.mp4`, `.mov`, or `.webm` that was not generated by Forge for this test.

The pipeline evidence command writes `docs/qa/forge-pre-manual-pipeline-evidence-2026-06-05.md`. That file proves deterministic fixture processing/export/import through the Rust pipeline, but it does not replace the interactive app QA rows in this checklist.

The bundled sample video can be regenerated if the file is missing:

```bash
ffmpeg -y -f lavfi -i color=c=0x00ff00:s=256x256:d=1:r=24 -vf "drawbox=x=96:y=88:w=64:h=96:color=white:t=fill" examples/inputs/green-box-character.mp4
```

## Shared Per-Asset Checks

Run this set for every asset in the matrix.

### Source And Setup

```text
[ ] Source import succeeds without crashing the app
[ ] Live workspace badge appears after import
[ ] The current source type is visible or inferable from the workflow state
[ ] Workflow tabs remain locked until their required prior stage is complete
[ ] No AI, BYOK, website, registry, marketplace, MCP, cloud, or cost setup is requested
```

### Preview And Navigation

```text
[ ] Preview frame displays
[ ] Frame scrubber changes the selected frame
[ ] Timeline thumbnails render for the imported frames or sliced cells
[ ] Bounding boxes appear when processing data is available
[ ] Manual foot anchor controls can be inspected without shifting unrelated UI
[ ] Loop range controls accept a valid start and end range
```

### Processing And Quality

```text
[ ] Extract or slice completes for the selected source
[ ] Chroma preview or processing controls update the preview without clearing the job
[ ] Process & Quality completes
[ ] Processed frames are written under the job `processed/` folder
[ ] `processed/bboxes.json` exists when processing completes
[ ] Quality report shows one of: `game_ready`, `needs_cleanup`, `prototype_usable`, `blocked`
[ ] Quality report includes recommendations or reasons appropriate to the verdict
[ ] Export readiness lists blockers before export when required settings or quality gates are missing
```

### Export And Re-Import

```text
[ ] Export folder blocker is visible before a default output folder is configured
[ ] Export folder blocker clears after Settings output folder is set
[ ] Export Pack completes
[ ] Export creates `frames/`
[ ] Export creates `sprite_sheet.png`
[ ] Export creates `atlas.json`
[ ] Export creates `manifest.json`
[ ] Export creates `quality-report.json`
[ ] Export creates `preview.gif`
[ ] Export creates a `.gsfpack` folder package
[ ] Validate Re-import succeeds
[ ] Re-imported pack preview GIF renders
[ ] Re-export preserves the original frame count
[ ] Recent Exports records the output
[ ] Open Exports Folder opens the local export directory
```

## Asset-Specific Checks

### Bundled Sample Video

```text
[ ] Import `/Users/kartz/Development/Forge/examples/inputs/green-box-character.mp4`
[ ] Probe metadata shows width `256`, height `256`, fps near `24`, and duration near `1` second
[ ] Codec and pixel format values are populated
[ ] Extracted frames appear under the job `raw/` folder
[ ] Chroma processing removes or handles the green background as configured
[ ] Exported pack validates locally
```

### Real Short Video

```text
[ ] Import a real local `.mp4`, `.mov`, or `.webm` that was not generated by Forge for this test
[ ] Probe metadata populates `width`, `height`, `fps`, `durationSeconds`, `frameCountEstimate`, `codec`, and `pixelFormat`
[ ] Select a loop range shorter than or equal to the source duration
[ ] Extracted frame count matches the selected range within one frame of expected rounding
[ ] Processing completes without overwriting a different job
[ ] Exported pack validates locally
```

### PNG Sequence

```text
[ ] Import a folder of sequential PNG files sorted in animation order
[ ] Imported frame count matches the number of PNG files selected
[ ] Frame dimensions are consistent across the sequence
[ ] Preview and scrubber follow the expected frame order
[ ] Processing keeps normalized output dimensions consistent
[ ] Exported pack validates locally
```

### Sprite Sheet

```text
[ ] Import a PNG sprite sheet with known columns and rows
[ ] Enter grid settings that multiply to the expected total frame count
[ ] Sliced frame count equals columns multiplied by rows
[ ] Cell bounds do not exceed the source image dimensions
[ ] Preview and scrubber follow row-major cell order
[ ] Exported pack validates locally
```

### Exported `.gsfpack`

```text
[ ] Import a `.gsfpack` exported by this app during the same manual session
[ ] Pack validation succeeds before or during import
[ ] Manifest metadata is accepted by the app
[ ] Preview GIF renders after import
[ ] Re-export completes into a new export folder
[ ] Re-exported frame count equals the original exported frame count
```

## Failure-State Checks

Run these checks at least once during the manual session, using a disposable job or restoring valid settings immediately after the failure check.

### Dependency Failures

```text
[x] Missing ffmpeg state says exactly: Install ffmpeg or choose an ffmpeg binary in Settings. Verified with `GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS=/tmp/forge-empty-tools`, `GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS=1`, `PATH=/usr/bin:/bin`, and cleared saved tool paths.
[x] Missing or invalid ffprobe path blocks video probing with an actionable message: `ffprobe: configured tool path must point to ffprobe`
[x] Restoring valid ffmpeg and ffprobe paths clears the dependency failure without relaunching the app: `ffmpeg and ffprobe are available.`
```

### Input Failures

```text
[x] Canceling an import dialog leaves the current workspace unchanged: after canceling the video `Choose File` dialog, the sample video path stayed populated and the footer showed `Selection cancelled.`
[x] Selecting a non-video file for video import fails without replacing the active workspace: `ffmpeg_command_failed: ... not-video.txt: Invalid data found when processing input`
[x] Confirming PNG sequence import with no selected frames fails with an actionable message: `choose at least one PNG frame`
[x] Selecting mixed-size PNG sequence frames has an accepted non-blocking rationale: current normalization supports mixed source dimensions; the fixture imported 3 raw frames and processed 3 normalized frames with `Needs Cleanup` and `Frame Consistency Consistent`
[x] Invalid sprite sheet grid reports source image bounds or grid dimension error: `sprite sheet grid exceeds source image bounds`
[x] Invalid `.gsfpack` folder reports validation failure without deleting existing exports: `required pack file is missing: previews/preview.gif`
```

### Workflow Failures

```text
[x] Blocked quality verdict disables export: importing only `frame_001.png` produced `Quality Report Blocked`, `Missing before export (1) Blocked verdict`, and disabled `Export Pack` / `Validate Re-import`; core export now also rejects blocked quality reports if the UI gate is bypassed
[x] Export remains blocked when no default output folder is configured: `Output folder Choose an export folder in Settings.` and `Export Pack` disabled
[x] Recovery card offers an action matching the failed stage for recorded path-style failures: rebuilt installed app routes non-video probe failures to `A local path needs attention`, `Choose a valid local video source, then import it again.`, with `Select source` and `Open Frames`
[x] Retrying the failed stage after fixing the cause succeeds or reports a new exact error: restoring `/opt/homebrew/bin/ffmpeg`, `/opt/homebrew/bin/ffprobe`, and `~/Game Sprite Forge/Exports` cleared toolchain failure with `ffmpeg and ffprobe are available.`
[x] A failed job does not corrupt Recent Exports entries from successful jobs: after non-video and blocked-quality attempts, Recent Exports still listed the prior successful packs, headed by `Real Short Video UITest`, without adding failed jobs
```

## Issue Log

For every `Fail` or `Blocked` result, record a row with the exact user-visible evidence.

| ID | Asset | Stage | Severity | Exact Visible Error Text Or Missing Path | Decision |
| --- | --- | --- | --- | --- | --- |
| QA-002 | Failure-state checks | Manual app QA | resolved-local | Recorded UI failures or accepted rationale for empty PNG sequence, deterministic missing ffmpeg, invalid configured ffmpeg, invalid configured ffprobe, non-video import, mixed-size PNG normalization, invalid sprite sheet grid, invalid `.gsfpack`, no-output-folder blocker, dependency recovery, cancel-dialog no-op, blocked-quality export disablement, failed-job Recent Exports integrity, and non-video recovery routing | Local installed-app failure-state gate accepted; true clean-Mac ffmpeg confidence remains optional external verification |
| QA-003 | Public distribution | Notarization/Gatekeeper | resolved | Current DMG is notarized and stapled; Gatekeeper assessment is `accepted, source=Notarized Developer ID`; mounted-DMG launch verifier passed | Release candidate package created under `release-candidates/GameSpriteForge-0.1.0-aarch64-218a5f9adf1f-notarized` |
| QA-004 | High-resolution video export sizing | Manual app QA | resolved | `/Users/kartz/Development/social-auto-upload/videos/demo.mp4` exported with multi-page sprite sheets; `/Users/kartz/Game Sprite Forge/Exports/Hero-Knight-Pack-1780644870371` contains 30 `sprite_sheet*.png` pages at 1308 x 1308 each, and in-app `.gsfpack` validation/re-import preserved 30 frames | Resolved by multi-page sheet export and explicit oversized-sheet recovery guidance |
| QA-005 | Non-video recovery classification | Source fix | resolved | Installed app before the source fix routed `ffmpeg_command_failed ... Invalid data found when processing input` to `Toolchain check failed`; rebuilt installed app now routes invalid-video probe errors to `A local path needs attention` with valid-video source guidance | Resolved in the rebuilt installed app |

If no issues are observed, record one row with `global`, `manual session`, `none`, `No failures or blockers observed`, and `release gate can proceed`.

## Post-RC Follow-up UI Evidence

Recorded after local pack library and first-run copy changes:

```text
First-run sample copy: Pass
Local Pack Library refresh/validate/re-import: Pass
Deterministic missing ffmpeg check: Pass
```

## Release Gate Summary

Use this summary only after the manual run is complete.

```text
Manual QA result: Local installed-app QA pass; notarized release candidate packaged
Bundled sample video: Pass; 24-frame Green Box Character pack exported and validated
Real short video: Pass; Wacom Gesture3FingerTap.mp4 imported, extracted with Every=4, processed, exported, and validated with 8 frames
PNG sequence: Pass; 6-frame multiline PNG sequence exported and validated
Sprite sheet: Pass; 4 x 2 fixture sheet exported and validated with 8 frames
Exported .gsfpack: Pass; imported, processed, re-exported, and validated with 6 frames
Failure-state checks: Pass/accepted locally; empty PNG sequence, invalid configured ffmpeg, invalid configured ffprobe, non-video import, mixed-size PNG normalization rationale, invalid sprite sheet grid, invalid .gsfpack, no-output-folder blocker, dependency recovery, cancel-dialog no-op, blocked-quality export disablement, failed-job Recent Exports integrity, and recovery routing recorded
Blocking issues: none for the current notarized release candidate
Non-blocking issues: none currently open
Release candidate decision: package-ready; deterministic missing ffmpeg/ffprobe QA and PATH/default-directory recovery are verified locally; true clean-Mac ffmpeg confidence remains optional before broad distribution
```

Release gate rules:

```text
[x] Bundled sample video passes end to end
[x] At least one real short video passes end to end
[x] At least one PNG sequence passes end to end
[x] At least one sprite sheet passes end to end
[x] At least one exported .gsfpack imports, validates, previews, and re-exports with the same frame count
[x] Every exported asset contains frames/, sprite_sheet.png, atlas.json, manifest.json, quality-report.json, preview.gif, and a .gsfpack folder
[x] Every failure-state check either passes or has an accepted non-release-blocking rationale
[x] No enabled MVP path requires AI providers, BYOK, website, registry, marketplace, MCP, cloud processing, or creator publishing
[x] Public release gate passes notarization, stapling, Gatekeeper assessment, and mounted-DMG launch for the current candidate
```
