# FFmpeg Bundle Evaluation - 2026-06-06

## Shell Evidence

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
app bundle: /Applications/Game Sprite Forge.app exists; bundled tools: none
app bundle: target/release/bundle/macos/Game Sprite Forge.app exists; bundled tools: none
Current decision: keep user-configured/PATH-only; the local Homebrew ffmpeg build is GPL-enabled.
PASS bundled ffmpeg evaluation completed
```

## Real UI Evidence

```text
Computer Use opened /Applications/Game Sprite Forge.app.
Settings showed ffmpeg path /opt/homebrew/bin/ffmpeg and ffprobe path /opt/homebrew/bin/ffprobe.
Settings copy showed: Settings saved locally. Empty ffmpeg fields fall back to the system PATH.
Forge Check FFmpeg showed: ffmpeg and ffprobe are available.
No bundled ffmpeg UI surface or account/cloud setup appeared.
```

## Decision

```text
Do not bundle the current Homebrew ffmpeg build.
Keep the current release on user-configured/PATH-only ffmpeg and ffprobe.
If bundling is revisited, use a dedicated distributable build and re-run signing, notarization, Gatekeeper, and real UI dependency checks.
```
