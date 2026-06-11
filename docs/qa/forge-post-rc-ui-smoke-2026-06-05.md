# Forge Post-RC UI Smoke - 2026-06-05

## Task 1 - Deterministic Missing FFmpeg

Code verification commands:

```bash
npm run test:scripts
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml video::ffmpeg
```

Current-source signed-app verification command:

```bash
env HOME=/tmp/forge-clean-home GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS=/tmp/forge-empty-tools GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS=1 PATH=/usr/bin:/bin "/Users/kartz/Development/Forge/target/release/bundle/macos/Game Sprite Forge.app/Contents/MacOS/Game Sprite Forge"
```

Computer Use observation:

```text
Check FFmpeg shows: Install ffmpeg or choose an ffmpeg binary in Settings.
```

Result: Pass

Notes:

```text
The target app was the rebuilt signed app under target/release/bundle/macos, not the older /Applications copy.
Saved ffmpeg/ffprobe settings can override PATH fallback, so the UI test cleared those fields before checking the missing-toolchain state.
With PATH=/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin and empty configured paths, Check FFmpeg showed: ffmpeg and ffprobe are available.
```

## Task 6 - First Run Copy

Current-source signed-app observation:

```text
Import panel button: Load Sample Path
First Run primary button: Run Sample Pipeline
First Run detail: Loads, processes, exports, and validates the bundled sample.
```

Result: Pass

## Task 6 - Local Pack Library UI

Current-source signed-app setup:

```bash
env HOME=/tmp/forge-clean-home PATH=/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin "/Users/kartz/Development/Forge/target/release/bundle/macos/Game Sprite Forge.app/Contents/MacOS/Game Sprite Forge"
```

Computer Use observations:

```text
Settings default output folder set to /Users/kartz/Game Sprite Forge/Exports for this temp-HOME session.
Exports route showed Local Pack Library.
Refresh Library listed 9 local .gsfpack exports.
Pack actions became visible after Refresh Library: Inspect, Validate Pack, Re-import Pack, Open.
Inspect on Green Box Character Pack completed without an error dialog.
Validate Pack on Green Box Character Pack completed without an error dialog.
Re-import Pack returned to Forge, imported Green Box Character Pack, and showed 24 frames in the active workspace.
Forge status showed: Imported Green Box Character Pack with 24 frames.
Export panel showed: Validated Green Box Character Pack with 24 frames.
```

Result: Pass
