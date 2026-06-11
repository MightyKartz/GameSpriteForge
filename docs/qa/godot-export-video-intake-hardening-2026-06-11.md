# Godot Export And Video Intake Hardening QA

Date: 2026-06-11

## Scope

This pass implemented the development plan at:

```text
docs/superpowers/plans/2026-06-11-forge-godot-export-video-intake-hardening.md
```

The implemented slice covers:

```text
one-click Godot project export from an exported .gsfpack
repeatable npm smoke:godot command
target frame-count controls for video extraction
preview overlays for bbox, foot anchor, and character center
source guards for Godot export, video extraction math, and preview overlays
```

## Skills And MCP Used

Skills used:

```text
forge-dev
superpowers:subagent-driven-development
superpowers:dispatching-parallel-agents
```

MCP/tooling used:

```text
tool_search to discover multi-agent tooling
multi_agent_v1 to dispatch three read-only explorer/reviewer agents
```

Subagent roles:

```text
Turing: Godot export and smoke-test risk review
Cicero: video segment and preview overlay implementation review
Dewey: plan consistency and current-worktree risk review
```

Key agent findings applied:

```text
ExportPanel and ForgeRoute wiring was the immediate red source-guard blocker.
The Godot importer needed to preserve the generalized pack-name implementation, not revert to a hardcoded smoke script.
Video extraction should derive keepEveryNFrames from a target frame count without changing the Rust/Tauri extraction API.
Preview overlay coordinates must be based on the displayed image box, not the full canvas container.
The smoke pack generator should not delete an arbitrary output root.
Godot project export should normalize output paths and validate the .gsfpack layout before writing the project.
The importer should support multi-sheet exports because Forge allows multi-sheet packs by default.
```

## Implementation Notes

Godot export:

```text
packages/core/src/export/godot.rs writes a minimal Godot 4 project.
apps/mac/src-tauri/src/lib.rs exposes export_godot_project and validates pack layout.
apps/mac/src/tauriCommands.ts wraps exportGodotProject.
ExportPanel shows a Godot export action and readiness card after pack export.
scripts/godot/import_forge_pack.gd creates SpriteFrames, AnimatedSprite2D, and a smoke scene.
The importer maps atlas frames to the correct texture page for multi-sheet packs.
```

Video intake:

```text
apps/mac/src/videoSegment.ts owns pure frame-count and interval math.
VideoSegmentPanel exposes 12-frame, 24-frame, and manual interval modes.
ForgeRoute uses the derived activeKeepEveryNFrames for real extraction.
```

Preview position:

```text
CanvasPreview receives selected normalized frame bbox/anchor/size data.
Overlays render inside preview-frame-stage so percentages use image coordinates.
Static center/foot lines are hidden when data-driven overlays exist.
```

## Verification

Commands run:

```bash
npm --workspace apps/mac run build
npm run test:scripts
cargo fmt --manifest-path /Users/kartz/Development/Forge/Cargo.toml --all -- --check
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml -p core
cargo check --manifest-path /Users/kartz/Development/Forge/apps/mac/src-tauri/Cargo.toml
npm run smoke:godot
```

Results:

```text
Frontend build: pass
Source guards: pass
Rust format check: pass
forge_core tests: pass, 48 tests across unit/integration suites
Tauri cargo check: pass
Godot smoke: pass
```

Godot smoke artifact:

```text
docs/qa/artifacts/godot-pack-smoke-20260611-140713
```

Smoke result:

```text
PASS Forge Godot import smoke: imported 8 frames into res://imported/Godot_Smoke_Walk/Godot_Smoke_Walk.spriteframes.tres and saved res://Godot_Smoke_Walk.tscn
PASS Godot pack smoke: /Users/kartz/Development/Forge/docs/qa/artifacts/godot-pack-smoke-20260611-140713
```

## Remaining Follow-Ups

Real UI click-through of the new Godot export button should be recorded after
the next installed app build. The current pass verified TypeScript build,
Tauri command compilation, source guards, Rust tests, and real Godot headless
import.

Future exporter slices can add editor-facing Godot plugin UX, but this slice
keeps Godot support as a generated local project plus importer script.
