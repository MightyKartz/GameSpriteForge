# Forge Local Postprocess and Ecosystem MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first shippable Forge loop: local video/frames/sheet input becomes validated `.gsfpack` game animation assets, with a macOS-first workbench, creator pack primitives, and clean seams for future BYOK generation, CLI, MCP, and marketplace.

**Architecture:** Forge is split into model-independent `Source Provider` adapters and a deterministic `Forge Pipeline`. The pipeline owns job persistence, frame extraction, matting, bbox/anchor normalization, loop scoring, sheet packing, quality reports, `.gsfpack`, and engine exporters; UI, CLI, MCP, and website call the same core contracts.

**Tech Stack:** Tauri + React/TypeScript macOS app, Rust core, bundled or user-configured ffmpeg/ffprobe, JSON Schema for pack contracts, lightweight website/registry, future MCP server over the same CLI/core.

---

## Current Status

This document is now the long-term product roadmap, not the current solo-developer MVP execution plan.

The active MVP plan is:

```text
/Users/kartz/Development/Forge/docs/superpowers/plans/2026-06-04-forge-solo-local-app-mvp.md
```

For the first release, defer:

```text
image model calls
video model calls
BYOK provider settings
lightweight website
online registry
marketplace
MCP server
Codex Skill
```

Use this roadmap again after the import-only macOS app can process local videos, export `.gsfpack`, and re-import its own packs.

## Research Input

This plan incorporates the 2026-06-04 review of [`sparklecatta-lang/sprite-video-lab`](https://github.com/sparklecatta-lang/sprite-video-lab).

Sprite Video Lab proves that a local postprocess lab is valuable: import local video/image/frames, preview a segment, preview one processed frame, run chroma/AI/luma/corridor matting, normalize canvas, select frames, preview animation, and export frames/sheet/zip/manifest. It should influence Forge's workflow and QA ergonomics.

Forge must not become only a clone of that tool. Forge's product edge is:

- `Source Provider` abstraction for import, image generation, video generation, image sequence, sprite sheet, pose-guided generation, and creator recipes.
- Quality gates for game-readiness: bbox stability, foot anchor drift, center drift, loop match, alpha edge, frame diff, cell boundary safety, and engine compatibility.
- `.gsfpack` as the portable asset + recipe + license + quality report format.
- Engine exporters for generic, Godot, Unity, Phaser/PixiJS, Cocos, and GameMaker.
- Creator ecosystem from V1: create, validate, share, install, and publish draft packs.
- CLI/MCP automation after the core contracts are stable.

## Implementation Decisions

1. First implementation target is local deterministic processing, not cloud generation.
2. The UI first step is `Source`, not `Video`, because video is only one source provider.
3. `Generate` exists in the UI from V1, but V1 labels generated output as `candidate` until it passes quality gates.
4. Chroma key, bbox normalization, foot anchor alignment, sheet packing, GIF preview, manifest, and `.gsfpack` are the first hard loop.
5. BiRefNet, CorridorKey, and Luma are valuable references from Sprite Video Lab, but they enter Forge as optional matting providers after the local chroma path is stable.
6. macOS App and CLI share the same core. MCP and Codex Skill call the CLI/core instead of owning pipeline logic.
7. Website starts as download/docs/registry, not SaaS.

## Target Repository Shape

Create this structure in the Forge project or in the future standalone `game-sprite-forge` repository:

```text
game-sprite-forge/
  apps/
    mac/                         # Tauri + React desktop app
    website/                     # download, docs, registry, creator waitlist
  packages/
    core/                        # Rust pipeline: jobs, video, matting, bbox, anchors, sheet, quality
    cli/                         # gsf command wrapper over core
    exporters/                   # generic/godot/unity/phaser/cocos/gamemaker
    pack/                        # .gsfpack schema, reader, writer, validator, migration
    providers/                   # import/generation/recipe provider contracts
    mcp-server/                  # post-core automation interface
  schemas/
    gsfpack.schema.json
    manifest.schema.json
    quality-report.schema.json
    provider-run.schema.json
  examples/
    inputs/
    packs/
    outputs/
  docs/
    architecture/
    decisions/
    superpowers/plans/
```

## Milestone Gates

### Gate 1: Local Asset Loop

Input a 1-2 second side-view walk video with solid background and output:

- transparent `frames/*.png`
- `sprite_sheet.png`
- `preview.gif`
- `manifest.json`
- `quality-report.json`
- importable `.gsfpack`

### Gate 2: Workbench Loop

The macOS app can import the same video, show a single-frame processed preview, run the pipeline, inspect frame warnings, and export a pack without using cloud generation.

### Gate 3: Creator Loop

The app can create, validate, install, and export a `.gsfpack`; website can list at least three free packs through registry metadata.

### Gate 4: Generation Candidate Loop

BYOK generation can create a reference image and/or motion source, but output remains `candidate` until quality gates pass.

### Gate 5: Automation Loop

CLI and MCP can process local inputs, inspect packs, validate quality reports, and export to a game project with explicit write confirmation.

## Task 1: Define Asset Contracts

**Files:**
- Create: `schemas/gsfpack.schema.json`
- Create: `schemas/manifest.schema.json`
- Create: `schemas/quality-report.schema.json`
- Create: `schemas/provider-run.schema.json`
- Create: `docs/architecture/asset-contracts.md`

- [ ] **Step 1: Create schema directory**

Run:

```bash
mkdir -p schemas docs/architecture
```

Expected: both directories exist.

- [ ] **Step 2: Write `.gsfpack` schema v0.1**

Define required fields:

```text
schemaVersion
id
name
version
creator.name
license.type
assetTypes
animations
compatibleEngines
preview.gif
qualityReport
```

Validation rules:

- `schemaVersion` equals `0.1.0`.
- `id` uses lowercase dotted or dashed namespace, for example `creator.hero-knight`.
- `version` uses semantic versioning.
- `assetTypes` includes at least one of `frames`, `sprite_sheet`, `atlas`, `recipe`, `engine_export`.
- `compatibleEngines` includes `generic`.

- [ ] **Step 3: Write manifest schema**

Define required fields:

```text
name
source.provider
sheet.image
sheet.frameWidth
sheet.frameHeight
animations
anchor.type
anchor.x
anchor.y
```

Animation entries require:

```text
fps
loop
frames
```

- [ ] **Step 4: Write quality report schema**

Define metric fields:

```text
verdict
metrics.bboxStability
metrics.centerDriftPx
metrics.footAnchorDriftPx
metrics.loopMatchScore
metrics.alphaCoverageAvg
metrics.alphaEdgeQuality
metrics.frameDiff
metrics.cellBoundarySafety
warnings
recommendations
```

Allowed verdicts:

```text
prototype_usable
needs_cleanup
game_ready
blocked
```

- [ ] **Step 5: Document contract responsibilities**

`docs/architecture/asset-contracts.md` must state:

- `.gsfpack` is the ecosystem package.
- `manifest.json` is the runtime/import description.
- `quality-report.json` is the gatekeeper for export and marketplace.
- `provider-run.json` records source provider, prompts, cost, model, and cache path without storing private API keys.

## Task 2: Build Core Job Model

**Files:**
- Create: `packages/core/src/job.rs`
- Create: `packages/core/src/source.rs`
- Create: `packages/core/src/artifact.rs`
- Modify: `packages/core/src/lib.rs`
- Test: `packages/core/tests/job_model_tests.rs`

- [ ] **Step 1: Create Rust core package**

Run:

```bash
mkdir -p packages
cargo new packages/core --lib
```

Expected: `packages/core/Cargo.toml` exists.

- [ ] **Step 2: Add job data model**

Define:

```text
ForgeJob
ForgeJobId
SourceArtifact
FrameArtifact
ProcessedFrameArtifact
SpriteSheetArtifact
QualityReportArtifact
PackArtifact
```

Required job states:

```text
created
source_ready
frames_extracted
processed
quality_checked
packed
exported
failed
```

- [ ] **Step 3: Add source provider enum**

Provider kinds:

```text
import_video
import_frames
import_sprite_sheet
import_gsfpack
text_to_reference_image
reference_to_motion_video
image_sequence
sprite_sheet_generation
pose_guided_generation
creator_recipe
installed_pack_recipe
marketplace_recipe
```

Mark generation providers with `requires_cost_confirmation`.

- [ ] **Step 4: Add tests for state transitions**

Tests must cover:

- New job starts as `created`.
- Importing a local video moves to `source_ready`.
- A failed pipeline step records an error message and `failed` state.
- Provider metadata never serializes API keys.

Run:

```bash
cargo test -p core job_model
```

Expected: tests pass after implementation.

## Task 3: Implement Local Video and Frame Intake

**Files:**
- Create: `packages/core/src/video/ffmpeg.rs`
- Create: `packages/core/src/video/probe.rs`
- Create: `packages/core/src/video/extract.rs`
- Create: `packages/core/src/frames/import.rs`
- Modify: `packages/core/src/lib.rs`
- Test: `packages/core/tests/video_extract_tests.rs`

- [ ] **Step 1: Add ffmpeg discovery**

Discovery order:

```text
GSF_FFMPEG_PATH
bundled app resource path
system PATH
```

If no ffmpeg exists, return a typed error containing installation guidance.

- [ ] **Step 2: Implement ffprobe metadata**

Return:

```text
width
height
fps
durationSeconds
frameCountEstimate
codec
hasAlpha
```

- [ ] **Step 3: Extract raw frames**

Support:

```text
input path
start time
end time
keep every N frames
output directory
```

Frame filenames:

```text
raw/frame_00001.png
raw/frame_00002.png
```

- [ ] **Step 4: Import existing PNG sequence**

Sort by natural filename order and copy into the job's `raw/` directory.

- [ ] **Step 5: Test with generated fixture**

Generate fixture:

```bash
ffmpeg -y -f lavfi -i color=c=0x00ff00:s=256x256:d=1:r=24 -vf "drawbox=x=100:y=80:w=48:h=100:color=white:t=fill" examples/inputs/green-box-walk.mp4
```

Run extraction test:

```bash
cargo test -p core video_extract
```

Expected: extracted frame count is greater than 0 and frame dimensions are `256x256`.

## Task 4: Implement Matting and Single-Frame Preview

**Files:**
- Create: `packages/core/src/matting/mod.rs`
- Create: `packages/core/src/matting/chroma.rs`
- Create: `packages/core/src/matting/luma.rs`
- Create: `packages/core/src/preview.rs`
- Test: `packages/core/tests/matting_tests.rs`

- [ ] **Step 1: Implement matting modes**

V1 modes:

```text
none
chroma
luma
```

Reserved provider modes:

```text
birefnet
corridorkey
birefnet_luma
birefnet_corridorkey
birefnet_luma_corridorkey
```

Reserved modes appear in schema and UI but return `unsupported_in_this_build` until provider modules are installed.

- [ ] **Step 2: Implement chroma key**

Parameters:

```text
keyColor
threshold
softness
despillStrength
haloPixels
```

Default background sampling:

```text
average of four corners
```

- [ ] **Step 3: Implement single-frame preview**

Input:

```text
source frame path
matting parameters
target canvas
preview background mode
```

Output:

```text
preview/source.png
preview/processed.png
preview/preview.json
```

- [ ] **Step 4: Add tests**

Tests:

- Green background becomes transparent.
- White foreground remains opaque.
- Despill reduces green channel on semi-transparent edge pixels.
- `none` mode preserves alpha and RGB.

## Task 5: Normalize Frames, Anchors, and Quality Metrics

**Files:**
- Create: `packages/core/src/bbox.rs`
- Create: `packages/core/src/anchor.rs`
- Create: `packages/core/src/normalize.rs`
- Create: `packages/core/src/quality.rs`
- Test: `packages/core/tests/quality_tests.rs`

- [ ] **Step 1: Compute per-frame bbox**

Use alpha channel. Store:

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

- [ ] **Step 2: Compute stable bbox metrics**

Metrics:

```text
bboxWidthVariationPx
bboxHeightVariationPx
centerDriftPx
footAnchorDriftPx
alphaCoverageAvg
alphaCoverageStdDev
```

- [ ] **Step 3: Normalize frames to canvas**

Canvas modes:

```text
auto_width_center
square_center
square_bottom
fixed_anchor
```

V1 default for characters:

```text
square_bottom
```

- [ ] **Step 4: Add quality verdict rules**

Initial thresholds:

```text
game_ready:
  footAnchorDriftPx <= 2
  centerDriftPx <= 12
  bboxWidthVariationPx <= 24
  alphaCoverageAvg between 0.02 and 0.60

needs_cleanup:
  footAnchorDriftPx <= 8
  centerDriftPx <= 32

prototype_usable:
  frames exist and alpha coverage is non-zero

blocked:
  no foreground frames or frame dimensions inconsistent
```

- [ ] **Step 5: Add recommendations**

Map failures to actions:

```text
foot drift high -> "Open Anchor step and lock foot line."
center drift high -> "Regenerate with locked camera or enable fixed anchor."
alpha coverage zero -> "Check key color or disable background removal."
loop score low -> "Trim loop range or regenerate shorter motion."
```

## Task 6: Implement Loop Candidate Selection

**Files:**
- Create: `packages/core/src/looping.rs`
- Modify: `packages/core/src/quality.rs`
- Test: `packages/core/tests/looping_tests.rs`

- [ ] **Step 1: Score frame similarity**

Use a deterministic score combining:

```text
alpha bbox distance
alpha mask difference
RGB difference inside union alpha region
foot anchor difference
```

- [ ] **Step 2: Select loop candidate**

For an extracted sequence, return:

```text
startFrame
endFrame
score
reason
```

Constraints:

```text
minimum 6 frames
maximum 24 frames for V1 default export
```

- [ ] **Step 3: Add manual override contract**

Timeline UI and CLI can pass explicit:

```text
--start-frame
--end-frame
```

Manual override still runs quality report.

## Task 7: Build Sheet, GIF, Manifest, and `.gsfpack`

**Files:**
- Create: `packages/core/src/sheet.rs`
- Create: `packages/core/src/gif_preview.rs`
- Create: `packages/pack/src/lib.rs`
- Create: `packages/pack/src/writer.rs`
- Create: `packages/pack/src/validator.rs`
- Test: `packages/pack/tests/pack_roundtrip_tests.rs`

- [ ] **Step 1: Build sprite sheet**

Inputs:

```text
processed frames
columns
padding
margin
max texture size
```

Outputs:

```text
sprite_sheet.png
atlas.json
```

- [ ] **Step 2: Build GIF preview**

Default:

```text
12 fps
transparent background when supported
checkerboard fallback metadata
```

- [ ] **Step 3: Write manifest**

Manifest records:

```text
source provider
source artifact path
sheet dimensions
frame size
animations
anchor
quality verdict
```

- [ ] **Step 4: Write `.gsfpack` directory**

Pack layout:

```text
hero-knight.gsfpack/
  forgepack.json
  previews/
  assets/
    frames/
    sprite_sheet.png
    atlas.json
    manifest.json
  recipes/
  exports/
  LICENSE.md
  quality-report.json
```

- [ ] **Step 5: Validate round trip**

Test reads a pack, validates schema, re-exports generic assets, and compares frame count.

## Task 8: Add CLI as Core Harness

**Files:**
- Create: `packages/cli/src/main.rs`
- Create: `packages/cli/tests/process_command_tests.rs`
- Create: `docs/architecture/cli-contract.md`

- [ ] **Step 1: Create CLI package**

Run:

```bash
cargo new packages/cli --bin
```

- [ ] **Step 2: Add process command**

Command:

```bash
gsf process \
  --input examples/inputs/walk.mp4 \
  --name hero_knight \
  --animation walk \
  --matte chroma \
  --target-size 512 \
  --fps 12 \
  --out examples/outputs/hero-knight
```

Expected outputs:

```text
frames/*.png
sprite_sheet.png
preview.gif
manifest.json
atlas.json
quality-report.json
hero-knight.gsfpack/
```

- [ ] **Step 3: Add inspect command**

Command:

```bash
gsf inspect-pack examples/outputs/hero-knight/hero-knight.gsfpack
```

Expected: prints pack id, version, animations, engines, quality verdict, warnings.

- [ ] **Step 4: Add validate command**

Command:

```bash
gsf validate-pack examples/outputs/hero-knight/hero-knight.gsfpack
```

Expected exit codes:

```text
0 valid
1 schema or file error
2 quality blocked
```

## Task 9: Implement macOS App Workbench MVP

**Files:**
- Create: `apps/mac/src/App.tsx`
- Create: `apps/mac/src/routes/SourcePage.tsx`
- Create: `apps/mac/src/routes/FramesPage.tsx`
- Create: `apps/mac/src/routes/BackgroundPage.tsx`
- Create: `apps/mac/src/routes/AnchorPage.tsx`
- Create: `apps/mac/src/routes/SheetPage.tsx`
- Create: `apps/mac/src/routes/ExportPage.tsx`
- Create: `apps/mac/src/components/QualityInspector.tsx`
- Create: `apps/mac/src/components/FrameTimeline.tsx`
- Create: `apps/mac/src/components/SingleFramePreview.tsx`

- [ ] **Step 1: Implement workflow tabs**

Tabs:

```text
Source
Frames
Background
Anchor
Sheet
Export
```

State labels:

```text
empty
ready
processing
warning
blocked
complete
```

- [ ] **Step 2: Implement Source page**

Modes:

```text
Generate
Import
Use Recipe
```

V1 enabled:

```text
Import video
Import frames
Import sprite sheet
Import .gsfpack
```

V1 visible but guarded:

```text
Generate reference image
Generate motion video
Generate image sequence
Generate sprite sheet
```

Guard copy:

```text
Generated sources are candidates until Quality Report marks them game-ready.
```

- [ ] **Step 3: Implement Background page**

Borrow the useful Sprite Video Lab interaction:

- Original frame preview.
- Processed frame preview.
- Key color swatch.
- Threshold, softness, despill, halo controls.
- Apply to one frame.
- Apply to full selected segment.

- [ ] **Step 4: Implement Quality Inspector**

Show:

```text
Game-ready Verdict
BBox Stability
Foot Anchor Drift
Center Drift
Loop Match
Alpha Edge Quality
Frame Consistency
Cell Boundary Safety
Recommendations
```

- [ ] **Step 5: Implement Export page**

Targets:

```text
Generic PNG + JSON
Godot
Unity
Phaser/PixiJS
Cocos
GameMaker
.gsfpack
```

Godot and Phaser can start as manifest exporters. Unity/Cocos/GameMaker can start as generic export profiles with import notes.

## Task 10: Add Creator Pack Loop

**Files:**
- Create: `apps/mac/src/routes/PacksPage.tsx`
- Create: `apps/mac/src/routes/CreatorPublishPage.tsx`
- Create: `packages/pack/src/metadata.rs`
- Create: `docs/architecture/creator-ecosystem.md`
- Create: `examples/packs/registry-index.json`

- [ ] **Step 1: Implement local pack library**

Pack states:

```text
installed
draft
valid
warning
blocked
unlicensed
update_available
```

- [ ] **Step 2: Implement create pack flow**

Required user inputs:

```text
pack name
creator name
license type
compatible engines
animations
preview gif
```

- [ ] **Step 3: Implement publish draft output**

Generate:

```text
publish-draft.json
pack.zip
preview-card.png
quality-summary.md
```

The website can accept these files through a manual submission form before automated marketplace accounts exist.

- [ ] **Step 4: Add sample registry**

`examples/packs/registry-index.json` contains three free sample entries with:

```text
id
name
version
creator
license
previewGif
downloadUrl
qualityVerdict
compatibleEngines
```

## Task 11: Add Lightweight Website Registry

**Files:**
- Create: `apps/website/src/pages/index.tsx`
- Create: `apps/website/src/pages/download.tsx`
- Create: `apps/website/src/pages/packs.tsx`
- Create: `apps/website/src/pages/docs.tsx`
- Create: `apps/website/src/pages/creators.tsx`
- Create: `apps/website/public/registry/index.json`

- [ ] **Step 1: Publish positioning**

Homepage message:

```text
Game Sprite Forge turns local videos, generated motion candidates, frame sequences, and creator recipes into validated 2D game animation packs.
```

- [ ] **Step 2: Add download page**

Sections:

```text
macOS app download
Developer ID and notarization explanation
Local processing privacy statement
ffmpeg helper explanation
```

- [ ] **Step 3: Add packs page**

Read `public/registry/index.json` and render cards with preview GIF, engines, license, and quality verdict.

- [ ] **Step 4: Add creator page**

Show:

```text
how to create a pack
how to validate a pack
how to submit a publish draft
what licenses are supported
what quality gates are required
```

## Task 12: Add BYOK Provider Contracts

**Files:**
- Create: `packages/providers/src/lib.rs`
- Create: `packages/providers/src/cost.rs`
- Create: `packages/providers/src/cache.rs`
- Create: `packages/providers/src/mock.rs`
- Create: `docs/architecture/source-provider-contract.md`

- [ ] **Step 1: Define provider trait**

Provider contract:

```text
estimate_cost(request) -> CostEstimate
prepare_request(request) -> ProviderRunPlan
run_with_confirmation(plan) -> SourceArtifact
redact_sensitive_metadata(metadata) -> ProviderRunMetadata
```

- [ ] **Step 2: Enforce cost preflight**

Generation provider runs require:

```text
estimated cost
provider name
model name
input artifacts
output type
user confirmation
```

- [ ] **Step 3: Add mock provider**

Mock provider copies test fixtures into a job and writes `provider-run.json`. This lets UI, CLI, MCP, and docs test generation flow without paying model costs.

- [ ] **Step 4: Add sensitive metadata redaction**

Redact:

```text
API keys
signed URL query parameters
authorization headers
cookies
provider request IDs if configured private
```

## Task 13: Add MCP and Codex Skill Only After CLI Stability

**Files:**
- Create: `packages/mcp-server/src/main.rs`
- Create: `docs/architecture/mcp-contract.md`
- Create: `docs/architecture/codex-skill-boundary.md`

- [ ] **Step 1: Expose local free tools**

Tools:

```text
list_installed_packs
inspect_pack
validate_pack
process_local_video
build_sprite_sheet
export_pack
```

- [ ] **Step 2: Expose costed tools with confirmation**

Tools:

```text
estimate_generation_cost
generate_character_reference
generate_motion_source
```

These tools return a preflight result unless the caller passes an explicit confirmed budget token.

- [ ] **Step 3: Expose project-writing tools with confirmation**

Tools:

```text
export_to_project
create_engine_import_files
install_pack_to_project
```

They require destination path, write summary, and confirmation.

- [ ] **Step 4: Keep Skill thin**

Codex Skill instructions should call `gsf` CLI or MCP. They must not duplicate postprocess logic, provider secrets, or Flow private routing.

## Task 14: Golden Samples and Regression QA

**Files:**
- Create: `examples/inputs/README.md`
- Create: `examples/outputs/README.md`
- Create: `docs/architecture/quality-gates.md`
- Create: `packages/core/tests/golden_quality_tests.rs`

- [ ] **Step 1: Add golden sample categories**

Categories:

```text
green screen character
transparent PNG sequence
bad loop candidate
high foot drift video
wide attack motion
VFX luma sample
```

- [ ] **Step 2: Define expected metrics**

Each sample has:

```text
expected frame count range
expected alpha coverage range
expected verdict
expected warning ids
```

- [ ] **Step 3: Add regression test**

Run:

```bash
cargo test -p core golden_quality
```

Expected: metric changes fail tests unless thresholds are intentionally updated in `docs/architecture/quality-gates.md`.

## Self-Review

Spec coverage:

- Local deterministic postprocess: covered by Tasks 2-9 and 14.
- Sprite Video Lab lessons: covered by Background page, single-frame preview, local job model, matting modes, and frame selection workflow.
- Forge differentiation: covered by Tasks 1, 5-13.
- Creator ecosystem: covered by Tasks 10-11.
- BYOK generation without overcommitting quality: covered by Task 12 and UI Source guard copy.
- MCP/Codex boundary: covered by Task 13.

Placeholder scan:

- This plan avoids placeholder markers and unspecified "add error handling" language.
- Every task names concrete files and outputs.

Execution note:

- If implementation starts in the current `/Users/kartz/Development/Forge` folder, initialize version control before Task 1.
- If implementation starts in a new standalone repository, copy the three existing Forge direction docs into `docs/decisions/` first so product context travels with the codebase.
