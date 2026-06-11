# Distribution MVP

Last updated: 2026-06-06.

## Current Distribution Status

The current build produces:

```text
/Users/kartz/Development/Forge/target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg
SHA-256: 0a5d18c3d1c8df79ba73e617ed972241cc2208b553227fda56ec124e90b5cc42
Installed app: /Applications/Game Sprite Forge.app
Bundle executable: Game Sprite Forge
Bundle identifier: dev.gamespriteforge.desktop
Signing identity: Developer ID Application: Ka Yan (J6P96F432P)
```

Verified:

```text
hdiutil verify: pass
codesign --verify --deep --strict: pass
local /Applications install: pass
stapler validate: pass
Gatekeeper DMG assessment: accepted, source=Notarized Developer ID
Gatekeeper mounted app assessment: accepted, source=Notarized Developer ID
local /Applications app ticket validation: pass
mounted-DMG launch: pass
```

Release candidate:

```text
release-candidates/GameSpriteForge-0.1.0-aarch64-0a5d18c3d1c8-notarized
release-candidates/GameSpriteForge-0.1.0-aarch64-0a5d18c3d1c8-notarized.zip
```

This means the current artifact is a notarized and stapled public release candidate. Deterministic missing ffmpeg/ffprobe and PATH/default-directory recovery are verified locally; a separate clean macOS smoke check is still useful before broad distribution as extra environment confidence.

For future rebuilt DMGs, run the preflight before submitting:

```bash
bash scripts/notarization-preflight.sh "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
```

The preflight checks local notarization readiness without contacting Apple: `xcrun`, `notarytool`, `stapler`, credential method selection without printing secrets, DMG hash/integrity/signature, DMG secure timestamp, mounted app signature, mounted app secure timestamp, hardened runtime, current stapled-ticket state, current Gatekeeper DMG assessment, and the exact next commands for submission, stapling, and final release verification.

## FFmpeg Strategy

The first distribution candidate uses a user-configured ffmpeg and ffprobe path.

## Bundled FFmpeg Evaluation - 2026-06-06

Current decision:

```text
Keep user-configured/PATH-only ffmpeg and ffprobe for the current public release candidate.
```

Evidence:

```text
docs/architecture/bundled-ffmpeg-evaluation.md
docs/qa/ffmpeg-bundle-evaluation-2026-06-06.md
```

The local Homebrew ffmpeg 8.0.1 build is GPL-enabled (`--enable-gpl`), so it is not the bundling candidate for this app. The current app bundles do not contain `ffmpeg` or `ffprobe`; any future bundled-helper implementation must use a dedicated distributable build, nested-code signing, notarization, Gatekeeper, and real UI dependency checks.

Bundling ffmpeg can be added later, but the MVP must make the dependency visible and fixable in Settings. On launch and before video processing, the app checks:

```text
ffmpeg path
ffprobe path
```

If either binary is missing, the app must show this exact message:

```text
Install ffmpeg or choose an ffmpeg binary in Settings.
```

If a configured path is present, the app validates that it points to the expected binary name and, on Unix/macOS, that the file is executable. Invalid configured paths do not silently fall back to PATH; only empty settings use PATH fallback.

## Why User-Configured First

- It avoids packaging and licensing complexity for the first local app candidate.
- It keeps the app usable for developers who already have Homebrew ffmpeg.
- It lets the processing pipeline ship before notarized bundled helpers are finalized.

## Dependency Check

The current MVP runtime checks dependencies in this order:

```text
configured app setting path
system PATH
```

The configured app setting path is used only when it passes validation. If the setting is blank, the app checks system PATH. If the setting is non-blank but invalid, the dependency check reports the configured-path error so the later processing commands do not fail with a contradictory path.

The Rust core already has a `bundled_resource_path` hook, but the Tauri commands intentionally pass `None` for the first release. Bundled ffmpeg can be enabled in a later packaging slice after licensing, notarization behavior, and helper verification are handled.

The first release acceptance path is either a user-configured ffmpeg/ffprobe path or a working system PATH. Settings must keep the missing dependency state clear and fixable.

## First Release Acceptance

The first release candidate passes when:

```text
Developer ID signed DMG verifies with hdiutil
app bundle passes codesign --verify --deep --strict
notarized and stapled DMG passes Gatekeeper assessment
app launches from the mounted notarized DMG without bypassing Gatekeeper
ffmpeg missing state is understandable
sample video processes end to end
exported .gsfpack validates
app can re-import exported .gsfpack
no AI provider or website setup is required
```

A separate clean macOS smoke check is useful before broad distribution as extra environment confidence. Deterministic missing ffmpeg/ffprobe and PATH/default-directory recovery are verified locally, so the hard public-release gate is satisfied for the current notarized/stapled DMG and mounted-DMG launch verification.

Current 2026-06-06 status:

```text
Developer ID signed DMG: done via scripts/build-signed-release-dmg.sh with explicit secure timestamp
current DMG SHA-256: 0a5d18c3d1c8df79ba73e617ed972241cc2208b553227fda56ec124e90b5cc42
hdiutil verify: pass
local app install: done
codesign verification: pass
notarization submission: Accepted, submission 834c0445-1c45-4201-88d5-a2c99c008714
stapled ticket: pass
Gatekeeper DMG assessment: accepted, source=Notarized Developer ID
Gatekeeper mounted app assessment: accepted, source=Notarized Developer ID
local /Applications app ticket validation: pass
mounted-DMG launch: pass
release verification script: pass
release package script: created release-candidates/GameSpriteForge-0.1.0-aarch64-0a5d18c3d1c8-notarized
release candidate zip SHA-256: 41831d5fa1155ad5620cf0f3e1351b8c6953d8d45d52276c4db97750c5dcb7ce
deterministic missing ffmpeg QA: pass; Check FFmpeg showed Install ffmpeg or choose an ffmpeg binary in Settings.
installed-app interactive QA: sample video, real non-Forge short video, high-resolution demo video with multi-page sheets, PNG sequence, sprite sheet, .gsfpack re-import/export paths, Local Pack Library actions, first-run sample copy, export output details, and Validate Re-import feedback passed; local failure-state rows are recorded or accepted, including cancel-dialog no-op, blocked-quality export disablement, failed-job Recent Exports integrity, and non-video recovery routing
```

## Out of Scope

```text
image model calls
video model calls
BYOK provider settings
hosted credits
website registry
marketplace
MCP server
Codex Skill integration
cloud upload or cloud processing
full Godot editor plugin
Unity deep integration
```
