# Sprite Tooling Followups For Forge

Date: 2026-06-11

## Purpose

This document records Forge's local sprite-tooling followups after the first
transparent-gutter sprite sheet import slice. It is intended to guide future
work without drifting Forge away from its local-first macOS workbench scope.

## Forge Scope Guard

Forge remains a local-first Tauri macOS workbench. The first public release and
near-term followups should keep these product areas out of scope:

```text
AI image generation
AI video generation
BYOK settings
website
online registry
marketplace
creator publish
cloud upload
cloud processing
hosted credits
product MCP server
Codex Skill product integration
remote task queue
user account or wallet login
NFT claim flow
```

Sprite tooling should stay inside Forge's local Rust/Tauri pipeline and should
not add hosted services, account dependencies, or cloud workers.

## Decision Matrix

| Area | Forge Decision | Reason |
| --- | --- | --- |
| Transparent-gutter sprite sheet splitting | Adapt first | High value for `import_sprite_sheet`; fits local Rust core and removes manual grid friction. |
| Per-frame crop/offset/recombine UI | Adapt after auto split | Directly supports quality cleanup after Forge reports anchor, bbox, and cell-boundary issues. |
| GIF to frames and frames to GIF | Consider later | Useful intake/export utility, but Forge already exports preview GIF and should first harden PNG/sheet workflows. |
| Engine and format presets | Consider later | Presets should become explicit export/import settings only after manifest versioning is stable. |
| Remote worker pipeline | Avoid | Contradicts Forge's local-first desktop boundary and packaging goals. |
| AI matte endpoints | Avoid for near term | Current scope excludes AI/model calls and cloud/provider settings. |
| Wallet/NFT/claim and hosted features | Avoid | Not part of Forge's release or post-release scope. |
| Large all-in-one React components | Avoid | Forge UI components should stay focused and testable. |

## Product Patterns

### 1. Automatic Transparent Split

When a sheet already has transparent gutters, users should not need to type cell
width, cell height, columns, and rows.

Forge implementation:

```text
Input: one PNG/WebP-like sprite sheet already copied into the job source folder
Output: raw/frame_00001.png ... raw/frame_N.png
Core owner: packages/core/src/video/sprite_sheet.rs
Tauri command owner: apps/mac/src-tauri/src/lib.rs
UI owner: apps/mac/src/components/ImportPanel.tsx and apps/mac/src/routes/ForgeRoute.tsx
```

Desired behavior:

```text
scan fully transparent rows as horizontal gutters
within each row band, scan fully transparent columns as vertical gutters
ignore all-transparent regions
write frames in row-major order
normalize extracted cells to a shared max width and max height
bottom-align and horizontally center foregrounds in the shared cell
preserve exact pixels; do not use interpolating resize
```

This belongs in Rust so tests cover the real desktop import path.

### 2. Frame Repair Workbench

After automatic split is stable, Forge should add a frame repair workflow for
selecting frames, nudging offsets, cropping edges, adding blank frames, deleting
frames, and recombining.

Forge direction:

```text
Input: extracted or normalized frames in the current job
Output: a patched processed frame set before quality report and export
Core owner: packages/core/src/frames or a new packages/core/src/edit module
UI owner: a new apps/mac/src/components/FrameRepairPanel.tsx
```

Recommended first controls:

```text
selected frame
dx/dy pixel nudge
top/right/bottom/left crop or transparent expand
delete frame
duplicate previous frame
reset frame edits
apply same dx/dy to all frames
```

Reason to wait until after automatic split:

```text
auto split increases the chance users import imperfect third-party sheets;
frame repair turns quality recommendations into actionable fixes.
```

### 3. GIF And WebP Utility Layer

Treat GIF/WebP support as a later intake/export convenience, not as the next
core feature.

Possible Forge direction:

```text
import GIF as frame sequence
export animated GIF already exists as preview; keep it preview-only unless users ask for production GIF
consider animated WebP only after PNG frame and atlas workflows stay stable
```

### 4. Format Presets

Users value engine-specific presets, but Forge should keep the output contract
explicit:

```text
Godot helper remains the first engine helper
additional presets should be added as named import/export settings, not hidden transforms
Unity should wait until repeated real user demand appears
```

## Near-Term Development Sequence

1. Implement automatic transparent-gutter sprite sheet splitting.
2. Record deterministic fixtures and source guards for the new import mode.
3. Dogfood with at least three sprite sheets: clean transparent gutters, uneven
   cell sizes, and a sheet with non-transparent background that should fall back
   to manual grid.
4. Add frame repair workbench only after the new import mode has stable QA.
5. Revisit GIF/WebP and engine presets after real user friction is observed.

## Documentation Links

Implementation plan for the first slice:

```text
docs/superpowers/plans/2026-06-11-forge-transparent-gutter-sprite-sheet-intake.md
```

Current Forge scope guard:

```text
docs/architecture/mvp-scope.md
docs/architecture/post-release-backlog.md
```

## Acceptance Standard

Any sprite-tooling feature must pass this checklist before it is considered
ready:

```text
implemented in Forge's local Rust/Tauri path
covered by Rust tests for media behavior
covered by TypeScript source guards or smoke tests for UI wiring
documented as local-only, with no cloud queue or account dependency
does not copy external project source code
does not add AI/model/provider settings
does not weaken .gsfpack export, import, validation, or release packaging
```
