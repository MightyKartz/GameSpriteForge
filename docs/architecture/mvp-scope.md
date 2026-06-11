## Status Snapshot

Last updated: 2026-06-06.

Current local MVP status:

- Tauri + React macOS app exists under `apps/mac`.
- Rust workspace exists under `packages/core`, `packages/pack`, and `apps/mac/src-tauri`.
- Schemas exist under `schemas/`.
- The app supports the enabled source providers listed below.
- The app exports local frames, sprite sheet, atlas, manifest, quality report, preview GIF, and `.gsfpack`.
- The app can import and validate its own `.gsfpack` output in tests.
- The app supports UI language mode: Automatic, English, and Simplified Chinese. Automatic follows the local system/browser language and Settings can override it.
- The installed app at `/Applications/Game Sprite Forge.app` passes `codesign --verify --deep --strict`.
- The current DMG and mounted-DMG app are notarized/stapled and accepted by Gatekeeper as `source=Notarized Developer ID`; the synchronized `/Applications` app passes codesign, validates its stapled app ticket, and launches locally.
- Public distribution release packaging exists at `release-candidates/GameSpriteForge-0.1.0-aarch64-0a5d18c3d1c8-notarized`.

## Enabled Source Providers

- `import_video`
- `import_frames`
- `import_sprite_sheet`
- `import_gsfpack`

Forge V1 is a local macOS workbench for importing existing media and packaging it into game-ready 2D sprite animation assets. Source Provider is an internal contract, but only the providers above are enabled in the first release.

## Disabled Source Providers

- `text_to_reference_image`
- `reference_to_motion_video`
- `image_sequence_generation`
- `sprite_sheet_generation`
- `pose_guided_generation`
- `marketplace_recipe`

These providers are not available in the MVP UI, schemas, or runtime flows.

## Enabled Exports

- Individual processed PNG frames
- Sprite sheet PNG
- `atlas.json`
- `manifest.json`
- `quality-report.json`
- Animated preview GIF
- `.gsfpack` directory package

Exports must be local files. The MVP does not upload, publish, or register exported assets.

## Disabled Product Areas

- AI model calls
- image model calls
- video model calls
- BYOK
- website
- online registry
- marketplace
- MCP
- Codex Skill
- cloud processing
- hosted credits
- provider API key settings
- cost preflight
- Creator Publish

The first release must not require accounts, remote services, generation providers, cloud queues, online publishing, or external agent integrations.

## Quality Verdicts

- `game_ready`
- `needs_cleanup`
- `prototype_usable`
- `blocked`

Quality reports must use one of these verdicts and include the MVP metric keys needed to evaluate frame stability, loop continuity, alpha coverage, frame count, frame size consistency, and sprite-sheet cell safety.

## First Release Acceptance Criteria

- Import a local video and show metadata: `width`, `height`, `fps`, `durationSeconds`, `frameCountEstimate`, `codec`, and `pixelFormat`.
- Scrub to a single frame and preview chroma settings on a checkerboard.
- Store job preview files under `~/Library/Application Support/Game Sprite Forge/jobs/<job_id>/previews/`.
- Extract the selected range to `raw/frame_00001.png` and write processed frames under `processed/`.
- Write `processed/bboxes.json` and normalize processed frame dimensions.
- Render timeline thumbnails, bounding boxes, anchor controls, and quality metrics.
- Export `frames/`, `sprite_sheet.png`, `atlas.json`, `manifest.json`, `quality-report.json`, `preview.gif`, and a `.gsfpack` package.
- Validate exported metadata against the local JSON Schemas.
- Import the app's own exported `.gsfpack`, render its preview GIF, validate it locally, and re-export with the same frame count.
- Build a Developer ID signed macOS DMG and installable app bundle.
- Public release gate: notarize and staple the DMG/app, pass Gatekeeper assessment, launch from the mounted DMG, and confirm no AI providers, BYOK, website setup, online registry, marketplace, MCP, Codex Skill, cloud processing, or video model calls are required.
- When ffmpeg is missing, show: `Install ffmpeg or choose an ffmpeg binary in Settings.`
- Allow user-configured ffmpeg/ffprobe paths and PATH fallback.
- Reject invalid configured ffmpeg/ffprobe paths instead of silently falling back to PATH.
- Bundled ffmpeg is not required for this release.
- Process a sample video end to end with configured paths or PATH fallback.

## Latest Verification Evidence

Commands run on 2026-06-06:

```bash
npm --workspace apps/mac run build
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml
npm --workspace apps/mac run smoke:ui:mvp
npm --workspace apps/mac run smoke:ui:responsive
FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:mvp
FORGE_SMOKE_LOCALE=zh-CN npm --workspace apps/mac run smoke:ui:mvp
FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:responsive
FORGE_SMOKE_LOCALE=zh-CN npm --workspace apps/mac run smoke:ui:responsive
npm --workspace apps/mac run tauri -- build --bundles app
npm run test:scripts
bash -n scripts/verify-release-package.sh scripts/notarization-preflight.sh scripts/package-release-candidate.sh scripts/build-signed-release-dmg.sh scripts/test-notarization-preflight.sh
bash scripts/test-notarization-preflight.sh
bash scripts/notarization-preflight.sh "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg" "$(shasum -a 256 "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg" | awk '{print $1}')"
scripts/verify-release-package.sh "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
scripts/package-release-candidate.sh
scripts/build-signed-release-dmg.sh
hdiutil verify "/Users/kartz/Development/Forge/target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
xcrun stapler validate -v "/Users/kartz/Development/Forge/target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
spctl --assess --type install --verbose=4 "/Users/kartz/Development/Forge/target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
codesign --verify --deep --strict --verbose=2 "/Applications/Game Sprite Forge.app"
spctl --assess --type execute --verbose=4 "/Applications/Game Sprite Forge.app"
scripts/verify-release-package.sh "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg" 0a5d18c3d1c8df79ba73e617ed972241cc2208b553227fda56ec124e90b5cc42
scripts/package-release-candidate.sh
```

Current result:

```text
frontend build: pass
Rust tests: pass
MVP UI smoke: pass
responsive UI smoke: pass
English MVP UI smoke: pass
Simplified Chinese MVP UI smoke: pass
English responsive UI smoke: pass
Simplified Chinese responsive UI smoke: pass
script test suite: pass
Tauri .app bundle build: pass
Bash syntax: pass
notarization preflight test: pass
impeccable detector: []
current DMG SHA-256: 0a5d18c3d1c8df79ba73e617ed972241cc2208b553227fda56ec124e90b5cc42
DMG hdiutil verify: pass
DMG stapler validate: pass
DMG Gatekeeper assessment: accepted, source=Notarized Developer ID
mounted DMG Info.plist identity check: pass
mounted DMG app codesign verification: pass
installed app codesign: pass
installed app stapler validate: pass
installed app Gatekeeper assessment: accepted, source=Notarized Developer ID
mounted-DMG app Gatekeeper assessment: accepted, source=Notarized Developer ID
notarization submission: Accepted, submission 834c0445-1c45-4201-88d5-a2c99c008714
release verification script: pass; mounted-DMG launch observed
release package script: created release-candidates/GameSpriteForge-0.1.0-aarch64-0a5d18c3d1c8-notarized and zip
release candidate zip SHA-256: 41831d5fa1155ad5620cf0f3e1351b8c6953d8d45d52276c4db97750c5dcb7ce
deterministic missing ffmpeg QA: pass; Check FFmpeg showed Install ffmpeg or choose an ffmpeg binary in Settings. with GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS=/tmp/forge-empty-tools and GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS=1
Local Workbench UX hardening QA: pass; see docs/qa/local-workbench-ux-evidence-2026-06-06.md
UI Language Localization QA: pass; includes bilingual smoke screenshots, zh-CN runtime visible-text guards, and quality recommendation i18n mapping guards. See docs/qa/ui-language-evidence-2026-06-06.md
```

## Manual QA Fixture Prep

`npm run qa:fixtures` prepares deterministic local fixtures under:

```text
/Users/kartz/Development/Forge/examples/inputs/manual-qa/
```

These fixtures cover PNG sequence import, sprite sheet slicing, and safe failure-state checks. They do not replace the required tester-selected real short video, and they do not prove the manual app QA gate passed.

`npm run qa:pipeline` records automated pre-manual evidence in:

```text
/Users/kartz/Development/Forge/docs/qa/forge-pre-manual-pipeline-evidence-2026-06-05.md
```

That evidence proves the deterministic PNG sequence and sprite sheet fixtures can be processed, exported, validated, imported, and re-exported through the Rust pipeline. It does not replace the installed-app interactive QA evidence recorded below.

The sprite sheet fixture path uses the shared `forge_core::video::slice_sprite_sheet_grid` implementation that the Tauri command calls for app sprite-sheet slicing.

## Manual Real-Asset QA Evidence

Recorded on 2026-06-05 with computer-use against `/Applications/Game Sprite Forge.app` on an Apple Silicon Mac:

```text
Bundled sample video: Pass; /Users/kartz/Development/Forge/examples/inputs/green-box-character.mp4 exported Green-Box-Character-Pack.gsfpack with 24 frames, then validated in-app.
Real short video: Pass; /Applications/Wacom Tablet.localized/Wacom Center.app/Contents/Resources/Gesture3FingerTap.mp4 probed as 300 x 200, 29.97 FPS, 1.17 sec, extracted with Every=4, processed to Game Ready, exported /Users/kartz/Game Sprite Forge/Exports/Real-Short-Video-UITest-1780628419646/Real-Short-Video-UITest.gsfpack with 8 frames, then validated in-app.
PNG sequence: Pass; /Users/kartz/Development/Forge/examples/inputs/manual-qa/png-sequence/frame_001.png through frame_006.png exported Multiline-PNG-UI-Test.gsfpack with 6 frames, then validated in-app.
Sprite sheet: Pass; /Users/kartz/Development/Forge/examples/inputs/manual-qa/sprite-sheet/forge-walk-sheet.png with 4 columns x 2 rows exported Manual-Sprite-Sheet-UI-Test.gsfpack with 8 frames, then validated in-app.
Exported .gsfpack: Pass; /Users/kartz/Game Sprite Forge/Exports/Multiline-PNG-UI-Test-1780626245133/Multiline-PNG-UI-Test.gsfpack imported as a source, processed, exported to Reimported-GSFPack-UI-Test.gsfpack, and validated with 6 frames.
Failure-state checks: Pass/accepted for the local installed-app session; installed app reported choose at least one PNG frame for empty PNG sequence input, ffmpeg: configured tool path must point to ffmpeg for invalid configured ffmpeg, ffprobe: configured tool path must point to ffprobe for invalid configured ffprobe, ffmpeg_command_failed ... not-video.txt: Invalid data found when processing input for non-video import, sprite sheet grid exceeds source image bounds for an invalid sheet grid, required pack file is missing: previews/preview.gif for an invalid .gsfpack fixture, and Output folder Choose an export folder in Settings. when the default output folder is empty. The mixed-size PNG fixture is accepted by current normalization and processed 3 frames with Needs Cleanup and Frame Consistency Consistent. Dependency recovery was restored in-app with ffmpeg and ffprobe are available. Canceling the video Choose File dialog left the sample path intact and showed Selection cancelled. Importing only frame_001.png produced Quality Report Blocked, Missing before export (1) Blocked verdict, and disabled Export Pack / Validate Re-import; core export now also rejects blocked quality reports if the UI gate is bypassed. After non-video and blocked-quality attempts, Recent Exports still listed the prior successful packs and did not add failed jobs. Rebuilt installed app routes non-video probe failures to A local path needs attention with valid-video source guidance.
High-resolution video sizing follow-up: resolved; /Users/kartz/Development/social-auto-upload/videos/demo.mp4 exports with multi-page sprite sheets under the selected max texture size, validates as a .gsfpack, and re-imports with 30 frames. The 2026-06-05 Computer Use verification exported /Users/kartz/Game Sprite Forge/Exports/Hero-Knight-Pack-1780644870371 with 30 pages at 1308 x 1308 each.
Non-video recovery follow-up: resolved in the rebuilt installed app; invalid-video probe errors now route to valid-video source selection instead of generic toolchain recovery.
Blocking issues: none for the current notarized release candidate.
Non-blocking follow-up: none currently open.
Release decision: current notarized release candidate is package-ready; deterministic missing ffmpeg/ffprobe QA and PATH/default-directory recovery are verified locally. A true clean-Mac pass remains useful confidence before broad distribution.
```

## Post-RC Local Library Evidence

```text
Post-RC local library evidence: Exports route now supports local pack library refresh, validation, and re-import without online registry features.
Computer Use confirmed Local Pack Library refresh listed local .gsfpack exports, Validate Pack completed without an error dialog, and Re-import Pack returned to Forge with the original 24-frame count.
Computer Use confirmed deterministic missing-ffmpeg copy: Install ffmpeg or choose an ffmpeg binary in Settings.
```

## Local Workbench UX Hardening Evidence

```text
Local Pack Library status feedback, first-run sample hints, export validation result feedback, and installed-app Computer Use evidence are tracked in docs/qa/local-workbench-ux-evidence-2026-06-06.md.
CLI/MCP remains deferred and is not part of the public MVP product surface.
```
