# Bundled FFmpeg Evaluation

Date: 2026-06-06

## Current decision

```text
Current decision: keep user-configured/PATH-only.
```

Reason:

```text
Homebrew ffmpeg 8.0.1 is GPL-enabled on this Mac because its configuration includes --enable-gpl.
The current app and release app bundle do not contain bundled ffmpeg or ffprobe.
Bundling should wait for a dedicated licensing decision and a purpose-built distributable ffmpeg/ffprobe pair.
```

## Local Finding

Command:

```bash
node scripts/verify-ffmpeg-bundle-evaluation.mjs
```

Observed:

```text
ffmpeg: /opt/homebrew/bin/ffmpeg
ffprobe: /opt/homebrew/bin/ffprobe
ffmpeg version: ffmpeg version 8.0.1
license flags: --enable-gpl
app bundle /Applications/Game Sprite Forge.app: no bundled ffmpeg or ffprobe
app bundle target/release/bundle/macos/Game Sprite Forge.app: no bundled ffmpeg or ffprobe
PASS bundled ffmpeg evaluation completed
```

## Proposed Bundle Layout

If a future release chooses to bundle ffmpeg, use this app resource layout:

```text
Game Sprite Forge.app/
  Contents/
    Resources/
      ffmpeg/
        bin/
          ffmpeg
          ffprobe
        LICENSES/
        NOTICE.txt
```

The resolver already supports `resource_path/bin/ffmpeg` and `resource_path/bin/ffprobe` through the core `bundled_resource_path` hook. The Tauri commands intentionally pass `None` today, so enabling this must be a deliberate packaging change.

## Packaging And Notarization Requirements

```text
Sign nested ffmpeg and ffprobe binaries before signing the app bundle.
Use hardened runtime with the app signature.
Run hdiutil verify, codesign --verify --deep --strict, stapler validate, spctl DMG assessment, and mounted-DMG launch verification after any bundle change.
Keep configured user paths highest priority so users can override bundled tools.
```

## Decision Gate For Enabling Bundling

Bundling can move from evaluation to implementation only when all rows are satisfied:

```text
license mode chosen and documented
redistribution obligations reviewed for every enabled ffmpeg library
dedicated ffmpeg/ffprobe build path documented
app resource layout wired in Tauri without breaking user-configured paths
signed app and notarized DMG pass release verification
real UI dependency check passes with configured path, bundled path, PATH fallback, and deterministic missing-tool override
```
