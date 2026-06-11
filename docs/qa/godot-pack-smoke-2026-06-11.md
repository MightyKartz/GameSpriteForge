# Godot Pack Smoke

Date: 2026-06-11

## Scope

Generate a real Forge `.gsfpack`, create a clean Godot project, import the pack
into that project, create native Godot animation resources, and verify the
saved scene can load and play the imported animation.

## Environment

```text
Forge workspace: /Users/kartz/Development/Forge
Godot: /Applications/Godot.app/Contents/MacOS/Godot
Godot version: 4.6.3.stable.official.7d41c59c4
Input sprite sheet: examples/inputs/manual-qa/sprite-sheet/forge-walk-sheet.png
```

## Generated Pack

```text
Artifact root: docs/qa/artifacts/godot-pack-smoke-2026-06-11
Pack: docs/qa/artifacts/godot-pack-smoke-2026-06-11/exports/godot_smoke_walk/Godot-Smoke-Walk.gsfpack
Frame count: 8
Animation: walk
```

The pack was generated through Forge's Rust core and validated with
`forge_pack::validate_pack_layout`.

## Godot Import

The smoke project is:

```text
docs/qa/artifacts/godot-pack-smoke-2026-06-11/godot-project
```

The import script copies the pack texture into `res://imported/GodotSmokeWalk`,
reads `assets/godot_import.json` and `assets/atlas.json`, creates Godot
`AtlasTexture` frames, builds a `SpriteFrames` resource, creates an
`AnimatedSprite2D`, saves a scene, loads that scene back, and calls `play()` on
the imported animation.

Generated Godot resources:

```text
res://imported/GodotSmokeWalk/GodotSmokeWalk.spriteframes.tres
res://GodotSmokeWalkSmoke.tscn
```

## Commands

```bash
cargo run --manifest-path /tmp/forge-godot-packgen/Cargo.toml -- \
  /Users/kartz/Development/Forge/docs/qa/artifacts/godot-pack-smoke-2026-06-11

/Applications/Godot.app/Contents/MacOS/Godot --headless \
  --path /Users/kartz/Development/Forge/docs/qa/artifacts/godot-pack-smoke-2026-06-11/godot-project \
  --script /Users/kartz/Development/Forge/docs/qa/artifacts/godot-pack-smoke-2026-06-11/godot-project/import_forge_pack.gd -- \
  /Users/kartz/Development/Forge/docs/qa/artifacts/godot-pack-smoke-2026-06-11/exports/godot_smoke_walk/Godot-Smoke-Walk.gsfpack
```

## Result

```text
PASS Forge Godot import smoke: imported 8 frames into res://imported/GodotSmokeWalk/GodotSmokeWalk.spriteframes.tres and saved res://GodotSmokeWalkSmoke.tscn
```

## Finding

The generated pack is usable in Godot 4.6.3 after a small import script converts
Forge's `godot_import.json` and atlas metadata into native Godot resources.

## Implementation Update

The smoke has been formalized as:

```bash
npm run smoke:godot
```

Latest passing artifact:

```text
docs/qa/artifacts/godot-pack-smoke-20260611-140713
```

The current importer is `scripts/godot/import_forge_pack.gd`. It derives safe
resource names from `forgepack.json`, supports multi-sheet texture pages, creates
native `SpriteFrames`, saves a Godot scene, loads it back, and verifies
`AnimatedSprite2D.play()`.
