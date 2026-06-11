# Local Workbench UX Evidence - 2026-06-06

## App

```text
/Applications/Game Sprite Forge.app
/Volumes/Game Sprite Forge/Game Sprite Forge.app
release-candidates/GameSpriteForge-0.1.0-aarch64-0a5d18c3d1c8-notarized/Game Sprite Forge_0.1.0_aarch64.dmg
```

Computer Use note:

```text
The refreshed notarized/stapled DMG was verified from /Volumes/Game Sprite Forge/Game Sprite Forge.app with bundleID dev.gamespriteforge.desktop.
The app was then synchronized to /Applications/Game Sprite Forge.app, stapled, Gatekeeper-verified, launched from /Applications, and showed the new UI text from this implementation.
DMG SHA-256: 0a5d18c3d1c8df79ba73e617ed972241cc2208b553227fda56ec124e90b5cc42
Notarization submission: 834c0445-1c45-4201-88d5-a2c99c008714
```

## Local Pack Library

```text
Refresh Library status: Found 11 local packs.
Inspect status: Inspected Green Box Character Pack.
Validate Pack status: Pack validation passed.
Re-import Pack result: Forge switched back to a live 24-frame workspace and reported Imported Green Box Character Pack with 24 frames.
Observed result: Exports route showed Local Pack Library, Refresh Library, Inspect, Validate Pack, Re-import Pack, and Open actions after refresh. Re-import Pack returned to Forge with a live 24-frame workspace.
```

## First Run

```text
Load Sample Path hint: Fills the video path only. Run Sample Pipeline processes and validates the bundled sample.
Run Sample Pipeline button: visible in the First Run rail.
Run Sample Pipeline result: Full pipeline passed: 24 frames exported and re-imported.
Observed result: First Run now distinguishes path-only sample loading from the full bundled sample pipeline.
```

## Export And Validation

```text
Export details: Pack, Frames, Sprite sheet pages, and Godot helper rows appeared after Export Pack.
Validation result: Last validated: Hero Knight Pack with 24 frames.
Observed result: Re-imported Green Box Character Pack was processed to Game Ready, exported to /Users/kartz/Game Sprite Forge/Exports/Hero-Knight-Pack-1780707555839/Hero-Knight-Pack.gsfpack, and Validate Re-import updated the validation result.
Pipeline status: Validated and re-imported Hero Knight Pack with 24 frames. All checks passed.
```

## Scope Guard

```text
Forge UI routes remained Forge, Exports, Settings.
No AI, BYOK, cloud, marketplace, product CLI, or product MCP surface appeared in the installed app.
```
