# Godot Export And Video Intake Hardening

Date: 2026-06-11

## Purpose

This document turns the current Forge QA findings into the next development
direction. The goal is to make Forge's first public workflow more trustworthy:
users should understand whether a video frame selection is correct, whether the
processed character position is stable, and whether the exported asset can be
used directly in Godot.

## Current Evidence

Forge already has a working local pipeline:

```text
source import -> raw frames -> chroma processing -> normalization -> quality report -> .gsfpack export -> pack validation
```

Recent Godot smoke evidence shows the exported pack is usable in Godot 4.6.3:

```text
docs/qa/godot-pack-smoke-2026-06-11.md
docs/qa/godot-export-video-intake-hardening-2026-06-11.md
docs/qa/artifacts/godot-pack-smoke-2026-06-11/exports/godot_smoke_walk/Godot-Smoke-Walk.gsfpack
docs/qa/artifacts/godot-pack-smoke-2026-06-11/godot-project/GodotSmokeWalkSmoke.tscn
docs/qa/artifacts/godot-pack-smoke-2026-06-11/godot-project/imported/GodotSmokeWalk/GodotSmokeWalk.spriteframes.tres
docs/qa/artifacts/godot-pack-smoke-20260611-140713
```

The important finding: the pack data is valid, and Forge now includes a
generated Godot project plus an import script that converts Forge metadata into
native `SpriteFrames` and scene resources. The importer supports multi-sheet
packs by mapping each atlas frame to the texture page listed in the atlas.

## Product Decisions

### 1. Do Not Treat 24 Frames As Correctness

Selecting 24 frames can be a reasonable default for a short action loop, but it
is not itself proof that the extraction is correct.

Correctness should be based on:

```text
selected segment covers one complete motion
estimated frame count is visible before extraction
actual extracted frame count is visible after extraction
first and last selected frames are close enough for the intended loop
quality report uses the selected loop range, not a hidden default
```

Forge should keep 24 as a preset, not as a hard rule.

### 2. Make Character Position Measurable

The character position should be judged by visible and numeric signals:

```text
foot anchor line
bbox rectangle
center line
bottom drift in pixels
center-x drift in pixels
loop start/end comparison
cell boundary safety
```

The existing quality metrics already compute most of this. The UI should expose
the signals on the frame preview instead of hiding them inside the quality card.

### 3. Make Godot Export Native

Forge should continue exporting `.gsfpack` as the portable source of truth, but
Godot users should also get a directly usable Godot export folder.

Target Godot export:

```text
<project-name>/
  project.godot
  README.md
  addons/game_sprite_forge/import_forge_pack.gd
```

Running the importer creates:

```text
res://imported/<safe-pack-name>/sprite_sheet*.png
res://imported/<safe-pack-name>/<safe-pack-name>.spriteframes.tres
res://<safe-pack-name>.tscn
```

The generated Godot project loads in Godot without requiring manual JSON
interpretation.

### 4. Keep Smoke Testing Real

The Godot smoke should remain a real engine test:

```text
generate a real Forge pack
open a clean Godot project in headless mode
run the importer script
create SpriteFrames
create AnimatedSprite2D
save a scene
load the scene back
call play() on the imported animation
```

Schema validation alone is not enough for engine compatibility.

## Recommended Development Order

Completed on 2026-06-11:

1. Productized the Godot import script and generated Godot export folder.
2. Added a reusable Godot smoke command to `scripts/` and `package.json`.
3. Added target frame-count controls for video extraction.
4. Added position overlays to the preview: bbox, anchor line, and center line.
5. Updated export UX so successful export clearly shows Godot project readiness.

Recommended next order:

1. Record a real installed-app UI click-through for the new Godot export button.
2. Add optional loop start/end visual comparison in the preview.
3. Consider a fuller Godot editor plugin only after the generated-project path
   remains stable.

## Acceptance Standards

A change is ready when these checks pass:

```text
npm --workspace apps/mac run build
npm run test:scripts
cargo fmt --manifest-path /Users/kartz/Development/Forge/Cargo.toml --all -- --check
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml
/Applications/Godot.app/Contents/MacOS/Godot --headless --path <smoke-project> --script <import-script> -- <pack.gsfpack>
```

For UI-visible changes, add real app evidence under `docs/qa/`.

## Non-Goals

```text
cloud export
online registry
marketplace publishing
AI generation
Godot editor marketplace plugin distribution
Unity/Phaser/Cocos/GameMaker exporters
```

Those can be revisited after Godot export and video intake correctness are
stable in the local app.
