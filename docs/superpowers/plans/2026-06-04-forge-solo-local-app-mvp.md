# Forge Solo Local App MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a solo-developer-friendly macOS app that imports local video, image sequences, or sprite sheets and turns them into validated 2D game animation assets without AI generation, website, MCP, or marketplace work in the first release.

**Architecture:** The MVP is an import-only local workbench. `Source Provider` remains an internal interface, but only `import_video`, `import_frames`, `import_sprite_sheet`, and `import_gsfpack` are enabled; generation providers, website registry, marketplace, and MCP are documented as later phases and hidden from the first UI.

**Tech Stack:** Tauri + React/TypeScript macOS app, Rust core for filesystem/job/image operations, bundled or user-configured ffmpeg/ffprobe for video extraction, JSON Schema for `.gsfpack`, local sample fixtures for QA.

---

## 2026-06-05 Progress Update

The MVP is no longer only a plan. The current workspace contains the macOS app, Rust core, pack validator, schemas, examples, tests, UI smoke scripts, and a Developer ID signed DMG build.

Implemented and verified:

```text
apps/mac Tauri + React shell
packages/core Rust pipeline modules
packages/pack .gsfpack validation/import
schemas/*.schema.json
examples/inputs green-box fixture
local video import/probe/extract
PNG sequence import
sprite sheet import/slice
.gsfpack import/validate
chroma preview and batch processing
frame normalization and foot anchor
loop range quality scoring
quality report verdicts
frames/sprite_sheet/atlas/manifest/quality-report/preview.gif/.gsfpack export
re-importable sample pipeline
UI smoke coverage for MVP boundary text
Run Summary, Export readiness blockers, and pipeline recovery guidance in the UI
```

Current verification:

```text
npm --workspace apps/mac run build: pass
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml: pass
npm --workspace apps/mac run smoke:ui:mvp: pass
impeccable detector: []
npm --workspace apps/mac run tauri -- build: pass
hdiutil verify target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg: pass
codesign --verify --deep --strict /Applications/Game Sprite Forge.app: pass
spctl --assess /Applications/Game Sprite Forge.app: rejected, source=Unnotarized Developer ID
```

Current release interpretation:

```text
Gate 1 Import and Preview: implemented in app/core
Gate 2 Process and Inspect: implemented in app/core
Gate 3 Export: implemented and covered by tests
Gate 4 Re-Import: implemented and covered by tests
Gate 5 First Distribution Candidate: partially complete; signed DMG exists, public release is blocked until notarization/stapling/Gatekeeper pass
```

Next work should stay focused on manual product QA and distribution hardening, not on AI generation, website, marketplace, MCP, or Codex Skill expansion.

## Scope Lock

This is the current execution plan for a personal developer.

### MVP Includes

```text
macOS desktop app
local video import: mp4 / mov / webm
local PNG sequence import
existing sprite sheet import
ffmpeg probe and frame extraction
single-frame chroma preview
batch chroma background removal
bbox detection
square-bottom canvas normalization
manual foot anchor adjustment
loop range selection
quality report
sprite sheet export
frames export
preview GIF export
manifest.json
atlas.json
.gsfpack create/import/validate
generic export
Godot import helper metadata
```

### MVP Excludes

```text
image model calls
video model calls
BYOK provider settings
hosted credits
light website
online registry
marketplace
creator payment flow
MCP server
Codex Skill integration
cloud upload or cloud processing
full Godot editor plugin
Unity deep integration
```

### Product Positioning

Do not position V1 as an AI generator. Position it as:

```text
A local macOS workbench that turns video and frame sources into game-ready 2D sprite animation assets.
```

The difference from a simple video-to-sprite-sheet tool is:

```text
quality report
foot anchor control
loop validation
engine-friendly manifests
.gsfpack package format
re-importable local asset packs
```

## Target Repository Shape

Use a minimal tree for the first implementation:

```text
game-sprite-forge/
  apps/
    mac/
      src/
        components/
        routes/
        styles/
        tauri-commands/
  packages/
    core/
      src/
        job/
        video/
        matting/
        frames/
        quality/
        export/
    pack/
      src/
  schemas/
    gsfpack.schema.json
    manifest.schema.json
    atlas.schema.json
    quality-report.schema.json
  examples/
    inputs/
    outputs/
  docs/
    architecture/
    superpowers/plans/
```

Leave these directories out of MVP:

```text
apps/website
packages/providers
packages/mcp-server
marketplace backend
```

## Milestone Gates

### Gate 1: Import and Preview

The app can import a local video, show metadata, scrub to one frame, preview chroma key settings, and show processed preview on checkerboard.

### Gate 2: Process and Inspect

The app can extract a selected range, remove background, normalize frames, show timeline thumbnails, and compute bbox/anchor metrics.

### Gate 3: Export

The app can export frames, sprite sheet, GIF, manifest, atlas, quality report, and `.gsfpack`.

### Gate 4: Re-Import

The app can import its own `.gsfpack`, show preview, validate metadata, and export again.

### Gate 5: First Distribution Candidate

The app can run on a clean macOS machine with either bundled ffmpeg or a clear ffmpeg path configuration, and can process one included sample video end to end.

## Task 1: Scope and Contracts

**Files:**
- Create: `docs/architecture/mvp-scope.md`
- Create: `schemas/gsfpack.schema.json`
- Create: `schemas/manifest.schema.json`
- Create: `schemas/atlas.schema.json`
- Create: `schemas/quality-report.schema.json`

- [ ] **Step 1: Create scope document**

Run:

```bash
mkdir -p docs/architecture schemas
```

Write `docs/architecture/mvp-scope.md` with these exact sections:

```text
Enabled Source Providers
Disabled Source Providers
Enabled Exports
Disabled Product Areas
Quality Verdicts
First Release Acceptance Criteria
```

Enabled providers:

```text
import_video
import_frames
import_sprite_sheet
import_gsfpack
```

Disabled providers:

```text
text_to_reference_image
reference_to_motion_video
image_sequence_generation
sprite_sheet_generation
pose_guided_generation
marketplace_recipe
```

- [ ] **Step 2: Define `.gsfpack` schema**

Required fields:

```text
schemaVersion
id
name
version
createdAt
creator.name
license.type
source.kind
animations
assets.manifest
assets.qualityReport
previews.gif
```

Allowed `source.kind` values for MVP:

```text
import_video
import_frames
import_sprite_sheet
import_gsfpack
```

- [ ] **Step 3: Define manifest schema**

Required fields:

```text
name
sheet.image
sheet.frameWidth
sheet.frameHeight
sheet.columns
sheet.rows
animations
anchor.type
anchor.x
anchor.y
```

Allowed `anchor.type` values:

```text
feet
center
custom
```

- [ ] **Step 4: Define quality report schema**

Required verdicts:

```text
game_ready
needs_cleanup
prototype_usable
blocked
```

Required metric keys:

```text
bboxBottomDriftPx
bboxCenterXDriftPx
bboxCenterYDriftPx
bboxWidthVariationPx
alphaCoverageAvg
loopMatchScore
frameCount
frameSizeConsistent
cellBoundarySafe
```

## Task 2: Local Job and File Model

**Files:**
- Create: `packages/core/src/job/mod.rs`
- Create: `packages/core/src/job/store.rs`
- Create: `packages/core/src/job/types.rs`
- Create: `packages/core/tests/job_store_tests.rs`

- [ ] **Step 1: Create Rust core package**

Run:

```bash
mkdir -p packages
cargo new packages/core --lib
```

Expected: `packages/core/Cargo.toml` exists.

- [ ] **Step 2: Define job directory layout**

Each processing job writes to:

```text
~/Library/Application Support/Game Sprite Forge/jobs/<job_id>/
  job.json
  source/
  raw/
  processed/
  thumbs/
  previews/
  exports/
```

- [ ] **Step 3: Define job states**

Allowed states:

```text
created
source_ready
preview_ready
frames_extracted
processed
quality_checked
exported
failed
```

- [ ] **Step 4: Test job creation**

Test expectations:

```text
new job creates all directories
job.json contains source kind
failed job stores error summary
job ids are filesystem safe
```

Run:

```bash
cargo test -p core job_store
```

## Task 3: Video Probe and Frame Extraction

**Files:**
- Create: `packages/core/src/video/mod.rs`
- Create: `packages/core/src/video/ffmpeg.rs`
- Create: `packages/core/src/video/probe.rs`
- Create: `packages/core/src/video/extract.rs`
- Create: `packages/core/tests/video_tests.rs`
- Create: `examples/inputs/README.md`

- [ ] **Step 1: Resolve ffmpeg**

Discovery order:

```text
configured app setting path
bundled app resource path
system PATH
```

If missing, return:

```text
ffmpeg_missing
```

with user-facing message:

```text
Install ffmpeg or choose an ffmpeg binary in Settings.
```

- [ ] **Step 2: Probe video**

Return:

```text
width
height
fps
durationSeconds
frameCountEstimate
codec
pixelFormat
```

- [ ] **Step 3: Extract selected range**

Parameters:

```text
startTimeSeconds
endTimeSeconds
keepEveryNFrames
outputDirectory
```

Output filenames:

```text
raw/frame_00001.png
raw/frame_00002.png
raw/frame_00003.png
```

- [ ] **Step 4: Add fixture command**

Document fixture creation in `examples/inputs/README.md`:

```bash
ffmpeg -y -f lavfi -i color=c=0x00ff00:s=256x256:d=1:r=24 -vf "drawbox=x=96:y=88:w=64:h=96:color=white:t=fill" examples/inputs/green-box-character.mp4
```

## Task 4: Chroma Preview and Batch Background Removal

**Files:**
- Create: `packages/core/src/matting/mod.rs`
- Create: `packages/core/src/matting/chroma.rs`
- Create: `packages/core/src/preview.rs`
- Create: `packages/core/tests/chroma_tests.rs`

- [ ] **Step 1: Implement chroma parameters**

Parameters:

```text
keyMode: auto_corners | manual
manualKeyColor: #RRGGBB
threshold: integer 0-255
softness: integer 0-255
despillStrength: float 0.0-2.0
haloPixels: integer 0-4
```

- [ ] **Step 2: Implement single-frame preview**

Input:

```text
raw frame path
chroma parameters
target canvas mode
```

Output:

```text
previews/source.png
previews/processed.png
previews/preview.json
```

- [ ] **Step 3: Implement batch processing**

For each raw frame, write:

```text
processed/frame_00001.png
processed/frame_00002.png
```

Store per-frame alpha bbox in:

```text
processed/bboxes.json
```

- [ ] **Step 4: Test chroma behavior**

Test expectations:

```text
green background alpha becomes 0
white foreground alpha stays 255
manual key color overrides corner sampling
processed frame dimensions match raw frame dimensions before normalization
```

## Task 5: Normalize Canvas and Anchor

**Files:**
- Create: `packages/core/src/frames/bbox.rs`
- Create: `packages/core/src/frames/normalize.rs`
- Create: `packages/core/src/frames/anchor.rs`
- Create: `packages/core/tests/anchor_tests.rs`

- [ ] **Step 1: Compute bbox metrics**

Per frame:

```text
left
top
right
bottom
width
height
centerX
centerY
bottomY
alphaCoverage
```

- [ ] **Step 2: Implement MVP canvas modes**

MVP modes:

```text
square_bottom
square_center
auto_width_center
```

Default:

```text
square_bottom
```

- [ ] **Step 3: Implement foot anchor**

Default anchor:

```text
x = canvasWidth / 2
y = canvasHeight - marginBottom
```

Manual adjustment stores:

```text
anchor.x
anchor.y
anchor.lockedByUser = true
```

- [ ] **Step 4: Test normalization**

Test expectations:

```text
all normalized frames share identical dimensions
square_bottom keeps bbox bottom near anchor y
empty alpha frame produces blocked quality reason
```

## Task 6: Quality Report and Loop Range

**Files:**
- Create: `packages/core/src/quality/mod.rs`
- Create: `packages/core/src/quality/metrics.rs`
- Create: `packages/core/src/quality/looping.rs`
- Create: `packages/core/tests/quality_tests.rs`

- [ ] **Step 1: Compute quality metrics**

Metrics:

```text
bboxBottomDriftPx
bboxCenterXDriftPx
bboxCenterYDriftPx
bboxWidthVariationPx
alphaCoverageAvg
frameCount
frameSizeConsistent
```

- [ ] **Step 2: Compute simple loop score**

Compare first and last selected frame:

```text
bbox distance
alpha coverage difference
center distance
```

Normalize to:

```text
0.0 bad
1.0 close match
```

- [ ] **Step 3: Produce verdict**

Rules:

```text
blocked:
  frameCount < 2
  or any frame has no foreground
  or frameSizeConsistent is false

game_ready:
  bboxBottomDriftPx <= 2
  bboxCenterXDriftPx <= 12
  loopMatchScore >= 0.75

needs_cleanup:
  bboxBottomDriftPx <= 8
  bboxCenterXDriftPx <= 32

prototype_usable:
  all other non-blocked outputs
```

- [ ] **Step 4: Add recommendations**

Recommendation ids:

```text
adjust_anchor
trim_loop_range
increase_chroma_threshold
reduce_chroma_threshold
use_shorter_clip
increase_canvas_margin
```

## Task 7: Export Frames, Sheet, GIF, Manifest, and Pack

**Files:**
- Create: `packages/core/src/export/sheet.rs`
- Create: `packages/core/src/export/gif.rs`
- Create: `packages/core/src/export/manifest.rs`
- Create: `packages/pack/src/lib.rs`
- Create: `packages/pack/tests/pack_tests.rs`

- [ ] **Step 1: Export frame sequence**

Write selected frames to:

```text
exports/<export_id>/frames/frame_001.png
exports/<export_id>/frames/frame_002.png
```

- [ ] **Step 2: Build sprite sheet**

Parameters:

```text
columns
paddingPx
marginPx
maxTextureSize
```

Outputs:

```text
sprite_sheet.png
atlas.json
```

- [ ] **Step 3: Build preview GIF**

Parameters:

```text
fps
loop: true
background: transparent | checkerboard
```

Output:

```text
preview.gif
```

- [ ] **Step 4: Build `.gsfpack`**

Directory layout:

```text
<pack-name>.gsfpack/
  forgepack.json
  previews/preview.gif
  assets/frames/
  assets/sprite_sheet.png
  assets/atlas.json
  assets/manifest.json
  quality-report.json
```

- [ ] **Step 5: Test pack round trip**

Test expectations:

```text
pack validates against schema
pack can be re-imported
re-imported frame count equals original exported frame count
```

## Task 8: macOS App MVP UI

**Files:**
- Create: `apps/mac/src/App.tsx`
- Create: `apps/mac/src/routes/ForgeRoute.tsx`
- Create: `apps/mac/src/routes/SettingsRoute.tsx`
- Create: `apps/mac/src/components/ImportPanel.tsx`
- Create: `apps/mac/src/components/VideoSegmentPanel.tsx`
- Create: `apps/mac/src/components/ChromaPreviewPanel.tsx`
- Create: `apps/mac/src/components/CanvasPreview.tsx`
- Create: `apps/mac/src/components/FrameTimeline.tsx`
- Create: `apps/mac/src/components/QualityInspector.tsx`
- Create: `apps/mac/src/components/ExportPanel.tsx`

- [ ] **Step 1: Create app skeleton**

Navigation:

```text
Forge
Packs
Exports
Settings
```

Hidden until later:

```text
Marketplace
Generate
Creator Publish
```

- [ ] **Step 2: Implement import panel**

Buttons:

```text
Import Video
Import PNG Sequence
Import Sprite Sheet
Import .gsfpack
```

- [ ] **Step 3: Implement processing steps**

Workflow tabs:

```text
Import
Frames
Background
Anchor
Sheet
Export
```

- [ ] **Step 4: Implement Quality Inspector**

Show:

```text
Verdict
BBox Bottom Drift
Center Drift
Loop Match
Frame Count
Alpha Coverage
Recommendations
```

- [ ] **Step 5: Implement Settings**

Fields:

```text
ffmpeg path
ffprobe path
default output folder
default FPS
default sheet size
```

No provider API key fields in MVP.

## Task 9: Distribution Candidate

**Files:**
- Create: `docs/architecture/distribution-mvp.md`
- Create: `examples/outputs/README.md`
- Modify: `apps/mac/tauri.conf.json`

- [ ] **Step 1: Document ffmpeg strategy**

Choose one MVP strategy:

```text
user-configured ffmpeg path
```

Bundled ffmpeg can be a later packaging improvement. The app must show a dependency check on launch.

- [ ] **Step 2: Add sample processing checklist**

Checklist:

```text
open app
set ffmpeg path
import examples/inputs/green-box-character.mp4
select 1 second range
preview chroma
process frames
adjust anchor if warning appears
export .gsfpack
re-import .gsfpack
open preview GIF
```

- [ ] **Step 3: Define first release acceptance**

First release candidate passes when:

```text
notarized Developer ID DMG passes Gatekeeper assessment and launches from the mounted DMG
ffmpeg missing state is understandable
sample video processes end to end
exported .gsfpack validates
app can re-import exported .gsfpack
no AI provider or website setup is required
```

Current 2026-06-05 status:

```text
Developer ID signed DMG exists
hdiutil verify passes
/Applications/Game Sprite Forge.app install exists
codesign verification passes
notarization is not complete for the latest local candidate
Gatekeeper assessment rejects the installed app with source=Unnotarized Developer ID
```

Do not mark the first release candidate complete until notarization, stapling, Gatekeeper assessment, and mounted-DMG launch verification pass for the latest DMG.

## Deferred Roadmap

After the MVP is usable, continue in this order:

```text
1. Local pack library polish
2. More exporters: Godot .tres/.tscn, Phaser atlas, Unity import notes
3. Lightweight docs/download page
4. Creator publish draft
5. BYOK image/video generation
6. MCP and Codex Skill
7. Online registry and paid marketplace
```

## Self-Review

Spec coverage:

- Import-only local app: covered by Tasks 2, 3, 4, and 8.
- No image/video model costs: generation providers are explicitly disabled in Task 1 and hidden in Task 8.
- No light website: website is excluded from target repository shape and deferred roadmap.
- Game asset differentiation: quality report, anchor control, loop score, engine-friendly manifest, and `.gsfpack` are covered by Tasks 5, 6, and 7.
- Solo-developer feasibility: distribution starts with user-configured ffmpeg path and avoids MCP, website, provider keys, and marketplace.

Placeholder scan:

- This plan avoids placeholder markers and keeps each task tied to concrete files and outputs.

Execution handoff:

- Start with Task 1 and Gate 1.
- Treat `docs/superpowers/plans/2026-06-04-forge-local-postprocess-and-ecosystem.md` as the long-term roadmap, not the current MVP execution plan.
