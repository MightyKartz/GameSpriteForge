# Forge Solo Local App MVP Multi-Agent Execution Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Execute `/Users/kartz/Development/Forge/docs/superpowers/plans/2026-06-04-forge-solo-local-app-mvp.md` with a coordinated multi-agent workflow that lands the macOS local app MVP end to end.

**Architecture:** The MVP is a local import-only workbench. The main thread owns shared contracts, integration, final verification, and release gates; subagents own disjoint modules with explicit write scopes so core processing, pack export, and UI can progress without file conflicts.

**Tech Stack:** Tauri + React/TypeScript macOS app, Rust workspace with `packages/core` and `packages/pack`, JSON Schema contracts, ffmpeg/ffprobe, local examples and generated asset fixtures.

---

## Authoritative Inputs

- Current execution spec: `/Users/kartz/Development/Forge/docs/superpowers/plans/2026-06-04-forge-solo-local-app-mvp.md`
- UI contract: `/Users/kartz/Development/Forge/GAME-SPRITE-FORGE-UI-DESIGN-SPEC.md`
- HTML visual baseline: `/Users/kartz/Development/Forge/figma-import/game-sprite-forge-app.html`
- Existing generated asset references: `/Users/kartz/Development/Forge/generated-assets/`

Treat the older 2026-05-31 direction and development-plan files as background only. They contain long-term AI, website, marketplace, MCP, and creator ecosystem concepts that are explicitly out of scope for this MVP.

## Current Workspace Truth

Updated 2026-06-05: the workspace now contains a formal app codebase and verified MVP pipeline.

Present now:

```text
/Users/kartz/Development/Forge/package.json
/Users/kartz/Development/Forge/Cargo.toml
/Users/kartz/Development/Forge/apps/mac
/Users/kartz/Development/Forge/apps/mac/src-tauri
/Users/kartz/Development/Forge/packages/core
/Users/kartz/Development/Forge/packages/pack
/Users/kartz/Development/Forge/schemas
/Users/kartz/Development/Forge/docs/architecture
/Users/kartz/Development/Forge/examples
/Users/kartz/Development/Forge/target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg
/Applications/Game Sprite Forge.app
```

Still absent:

```text
.git repository metadata
apps/website
packages/providers
packages/mcp-server
marketplace backend
```

Do not create a nested `game-sprite-forge/` folder unless the human explicitly asks for it. The intended project root for this execution is `/Users/kartz/Development/Forge`.

Current verification evidence:

```text
npm --workspace apps/mac run build: pass
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml: pass
npm --workspace apps/mac run smoke:ui:mvp: pass
impeccable detector: []
npm --workspace apps/mac run tauri -- build: pass
hdiutil verify DMG: pass
codesign installed app: pass
spctl installed app: rejected, source=Unnotarized Developer ID
```

Current gate status:

```text
Gate 1 Import and Preview: implemented
Gate 2 Process and Inspect: implemented
Gate 3 Export: implemented
Gate 4 Re-Import: implemented
Gate 5 First Distribution Candidate: signed local candidate exists; public release blocked until notarization/stapling/Gatekeeper pass
```

## Scope Lock

MVP enabled providers:

```text
import_video
import_frames
import_sprite_sheet
import_gsfpack
```

MVP disabled providers:

```text
text_to_reference_image
reference_to_motion_video
image_sequence_generation
sprite_sheet_generation
pose_guided_generation
marketplace_recipe
```

MVP UI must hide:

```text
Generate
Marketplace
Creator Publish
provider API key settings
cost preflight
hosted credits
online registry
```

Position V1 as:

```text
A local macOS workbench that turns video and frame sources into game-ready 2D sprite animation assets.
```

Do not position V1 as an AI generator.

## Multi-Agent Operating Model

Use fresh subagents per implementation task. The main thread coordinates, integrates, verifies, and updates this plan.

Required per-task loop:

```text
1. Main thread extracts the exact task card and context.
2. Main thread dispatches one implementer worker with an explicit write scope.
3. Worker implements, tests, self-reviews, and reports changed paths.
4. Main thread dispatches a spec-compliance reviewer.
5. Worker fixes every spec gap found by review.
6. Main thread dispatches a code-quality reviewer.
7. Worker fixes every approved quality issue.
8. Main thread runs integration verification and marks the task complete only after evidence passes.
```

Parallel implementation rule:

```text
Do not dispatch two implementation workers that can edit the same file.
Parallel workers are allowed only when their write scopes are disjoint and shared Rust/TypeScript API contracts are already stable.
```

Main thread always owns:

- `/Users/kartz/Development/Forge/Cargo.toml`
- `/Users/kartz/Development/Forge/package.json`
- `/Users/kartz/Development/Forge/packages/core/Cargo.toml`
- `/Users/kartz/Development/Forge/packages/core/src/lib.rs`
- `/Users/kartz/Development/Forge/apps/mac/src-tauri/**`
- `/Users/kartz/Development/Forge/apps/mac/tauri.conf.json`
- final Tauri command names and serialization types
- final Gate 1-5 evidence

## Agent Roster

| Agent | Type | Owns | Write Scope |
| --- | --- | --- | --- |
| Main Integrator | main thread | workspace bootstrap, shared manifests, command wiring, final gates | shared files listed above |
| Contracts Worker | worker | original Task 1 | `docs/architecture/mvp-scope.md`, `schemas/*.schema.json` |
| Job Core Worker | worker | original Task 2 | `packages/core/src/job/**`, `packages/core/tests/job_store_tests.rs` |
| Video Worker | worker | original Task 3 | `packages/core/src/video/**`, `packages/core/tests/video_tests.rs`, `examples/inputs/README.md` |
| Matting Worker | worker | original Task 4 | `packages/core/src/matting/**`, `packages/core/src/preview.rs`, `packages/core/tests/chroma_tests.rs` |
| Frames Quality Worker | worker | original Tasks 5 and 6 | `packages/core/src/frames/**`, `packages/core/src/quality/**`, `packages/core/tests/anchor_tests.rs`, `packages/core/tests/quality_tests.rs` |
| Export Pack Worker | worker | original Task 7 | `packages/core/src/export/**`, `packages/pack/**` |
| UI Worker | worker | original Task 8 UI files | `apps/mac/src/**` |
| Distribution Worker | worker or main thread | original Task 9 docs/checklists | `docs/architecture/distribution-mvp.md`, `examples/outputs/README.md` |
| Spec Reviewer | reviewer | requirement coverage | no writes |
| Code Quality Reviewer | reviewer | maintainability, test quality, integration risk | no writes |

## Dependency Graph

```text
Phase 0: Bootstrap execution surface
  -> Phase 1: Contracts
      -> Phase 2: Job core
          -> Phase 3A: Video
          -> Phase 3B: Matting
          -> Phase 3C: Frames and quality
              -> Phase 4: Export and pack
                  -> Phase 5: App integration and distribution

UI skeleton may start after Phase 1.
UI command integration waits for Phases 2-4.
Distribution waits for Gate 4 re-import evidence.
```

## Verification Gates

Gate 1: Import and Preview passes when:

- app imports a local video
- app shows metadata: `width`, `height`, `fps`, `durationSeconds`, `frameCountEstimate`, `codec`, `pixelFormat`
- app can scrub to one frame
- app previews chroma settings on checkerboard
- job preview files exist under `~/Library/Application Support/Game Sprite Forge/jobs/<job_id>/previews/`

Gate 2: Process and Inspect passes when:

- selected range extracts to `raw/frame_00001.png`
- processed frames exist under `processed/`
- `processed/bboxes.json` exists
- normalized frame dimensions are identical
- UI timeline thumbnails and bbox/anchor metrics render

Gate 3: Export passes when:

- exports contain `frames/`, `sprite_sheet.png`, `atlas.json`, `manifest.json`, `quality-report.json`, `preview.gif`
- `.gsfpack` directory layout matches original MVP plan
- schemas validate exported metadata

Gate 4: Re-Import passes when:

- app imports its own exported `.gsfpack`
- pack preview GIF renders
- pack validates against local schemas
- re-export produces the same frame count as the original export

Gate 5: First Distribution Candidate passes when:

- notarized Developer ID DMG passes Gatekeeper assessment
- app launches from the mounted notarized DMG
- missing ffmpeg state says `Install ffmpeg or choose an ffmpeg binary in Settings.`
- user-configured ffmpeg path works
- sample video processes end to end
- exported `.gsfpack` validates and re-imports
- no AI provider, website, MCP, marketplace, or cloud setup is required
- optional external macOS smoke check can be run from the release candidate package

## Phase 0: Bootstrap Execution Surface

**Owner:** Main Integrator

**Files:**
- Create: `/Users/kartz/Development/Forge/Cargo.toml`
- Create: `/Users/kartz/Development/Forge/package.json`
- Create: `/Users/kartz/Development/Forge/apps/mac/package.json`
- Create: `/Users/kartz/Development/Forge/apps/mac/src-tauri/Cargo.toml`
- Create: `/Users/kartz/Development/Forge/apps/mac/src-tauri/tauri.conf.json`
- Create: `/Users/kartz/Development/Forge/packages/core/Cargo.toml`
- Create: `/Users/kartz/Development/Forge/packages/core/src/lib.rs`
- Create: `/Users/kartz/Development/Forge/packages/pack/Cargo.toml`
- Create: `/Users/kartz/Development/Forge/packages/pack/src/lib.rs`

- [x] **Step 1: Decide git execution mode**

  Current workspace is not a git repository. Before worker execution, choose one of these modes and record it in the next status update:

  ```text
  Mode A: initialize git at /Users/kartz/Development/Forge and allow per-task commits
  Mode B: no git initialization; workers report changed paths and verification only
  ```

  Recommended mode:

  ```text
  Mode A
  ```

- [x] **Step 2: Create Rust workspace shell**

  Create a top-level Cargo workspace with these members:

  ```toml
  [workspace]
  members = [
    "apps/mac/src-tauri",
    "packages/core",
    "packages/pack"
  ]
  resolver = "2"
  ```

  Verification command:

  ```bash
  cargo metadata --format-version 1
  ```

  Expected result:

  ```text
  metadata resolves all three workspace members
  ```

- [x] **Step 3: Create JavaScript workspace shell**

  Create top-level scripts that delegate to the mac app:

  ```json
  {
    "name": "game-sprite-forge",
    "private": true,
    "version": "0.1.0",
    "scripts": {
      "dev": "npm --workspace apps/mac run dev",
      "build": "npm --workspace apps/mac run build",
      "tauri": "npm --workspace apps/mac run tauri"
    },
    "workspaces": [
      "apps/mac"
    ]
  }
  ```

  Verification command:

  ```bash
  npm pkg get workspaces
  ```

  Expected result:

  ```text
  [
    "apps/mac"
  ]
  ```

- [x] **Step 4: Create minimal app shell without product UI**

  Create the Tauri/React shell only. Do not implement Generate, Marketplace, Creator Publish, or provider settings.

  Verification command:

  ```bash
  npm --workspace apps/mac run build
  ```

  Expected result:

  ```text
  frontend build completes
  ```

## Phase 1: Contracts and Schemas

**Owner:** Contracts Worker

**Original spec:** Task 1 from `/Users/kartz/Development/Forge/docs/superpowers/plans/2026-06-04-forge-solo-local-app-mvp.md`

**Files:**
- Create: `/Users/kartz/Development/Forge/docs/architecture/mvp-scope.md`
- Create: `/Users/kartz/Development/Forge/schemas/gsfpack.schema.json`
- Create: `/Users/kartz/Development/Forge/schemas/manifest.schema.json`
- Create: `/Users/kartz/Development/Forge/schemas/atlas.schema.json`
- Create: `/Users/kartz/Development/Forge/schemas/quality-report.schema.json`

- [x] **Step 1: Dispatch Contracts Worker**

  Worker prompt must include:

  ```text
  You own only Task 1: Scope and Contracts from the MVP plan.
  Write only docs/architecture/mvp-scope.md and schemas/*.schema.json.
  Include exactly the enabled providers, disabled providers, quality verdicts, required schema fields, and first release acceptance criteria from the MVP plan.
  Do not create Rust, Tauri, website, provider, marketplace, or MCP files.
  ```

- [x] **Step 2: Verify schema presence**

  Run:

  ```bash
  test -f docs/architecture/mvp-scope.md
  test -f schemas/gsfpack.schema.json
  test -f schemas/manifest.schema.json
  test -f schemas/atlas.schema.json
  test -f schemas/quality-report.schema.json
  ```

  Expected result:

  ```text
  all files exist
  ```

- [x] **Step 3: Verify MVP excludes are documented**

  Run:

  ```bash
  rg -n "image model|video model|BYOK|website|registry|marketplace|MCP|Codex Skill|cloud" docs/architecture/mvp-scope.md
  ```

  Expected result:

  ```text
  each excluded product area appears as disabled or out of scope
  ```

## Phase 2: Job Core and Local File Model

**Owner:** Job Core Worker

**Original spec:** Task 2 from the MVP plan.

**Files:**
- Create: `/Users/kartz/Development/Forge/packages/core/src/job/mod.rs`
- Create: `/Users/kartz/Development/Forge/packages/core/src/job/store.rs`
- Create: `/Users/kartz/Development/Forge/packages/core/src/job/types.rs`
- Create: `/Users/kartz/Development/Forge/packages/core/tests/job_store_tests.rs`

- [x] **Step 1: Dispatch Job Core Worker**

  Worker prompt must include:

  ```text
  You own only Task 2: Local Job and File Model.
  Write only packages/core/src/job/** and packages/core/tests/job_store_tests.rs.
  The job root is ~/Library/Application Support/Game Sprite Forge/jobs/<job_id>/.
  Required directories are job.json, source, raw, processed, thumbs, previews, exports.
  Required states are created, source_ready, preview_ready, frames_extracted, processed, quality_checked, exported, failed.
  Job ids must be filesystem safe.
  Do not edit video, matting, frames, quality, export, pack, or UI modules.
  ```

- [x] **Step 2: Main thread wires module export**

  Main thread adds the job module to `/Users/kartz/Development/Forge/packages/core/src/lib.rs`.

- [x] **Step 3: Verify job store tests**

  Run:

  ```bash
  cargo test -p core job_store
  ```

  Expected result:

  ```text
  job creation tests pass
  ```

## Phase 3A: Video Probe and Extraction

**Owner:** Video Worker

**Original spec:** Task 3 from the MVP plan.

**Files:**
- Create: `/Users/kartz/Development/Forge/packages/core/src/video/mod.rs`
- Create: `/Users/kartz/Development/Forge/packages/core/src/video/ffmpeg.rs`
- Create: `/Users/kartz/Development/Forge/packages/core/src/video/probe.rs`
- Create: `/Users/kartz/Development/Forge/packages/core/src/video/extract.rs`
- Create: `/Users/kartz/Development/Forge/packages/core/tests/video_tests.rs`
- Create: `/Users/kartz/Development/Forge/examples/inputs/README.md`

- [x] **Step 1: Dispatch Video Worker**

  Worker prompt must include:

  ```text
  You own only Task 3: Video Probe and Frame Extraction.
  Write only packages/core/src/video/**, packages/core/tests/video_tests.rs, and examples/inputs/README.md.
  Implement ffmpeg discovery order: configured app setting path, bundled app resource path, system PATH.
  If missing, return code ffmpeg_missing and message "Install ffmpeg or choose an ffmpeg binary in Settings."
  Probe returns width, height, fps, durationSeconds, frameCountEstimate, codec, pixelFormat.
  Extraction writes raw/frame_00001.png style filenames.
  Document the exact green-box-character.mp4 fixture command from the MVP plan.
  Do not edit UI settings or Tauri commands.
  ```

- [x] **Step 2: Main thread wires module export**

  Main thread adds the video module to `/Users/kartz/Development/Forge/packages/core/src/lib.rs`.

- [x] **Step 3: Verify video tests**

  Run:

  ```bash
  cargo test -p core video
  ```

  Expected result:

  ```text
  video probe/extract unit tests pass or skip only when ffmpeg is explicitly unavailable
  ```

## Phase 3B: Chroma Preview and Batch Matting

**Owner:** Matting Worker

**Original spec:** Task 4 from the MVP plan.

**Files:**
- Create: `/Users/kartz/Development/Forge/packages/core/src/matting/mod.rs`
- Create: `/Users/kartz/Development/Forge/packages/core/src/matting/chroma.rs`
- Create: `/Users/kartz/Development/Forge/packages/core/src/preview.rs`
- Create: `/Users/kartz/Development/Forge/packages/core/tests/chroma_tests.rs`

- [x] **Step 1: Dispatch Matting Worker**

  Worker prompt must include:

  ```text
  You own only Task 4: Chroma Preview and Batch Background Removal.
  Write only packages/core/src/matting/**, packages/core/src/preview.rs, and packages/core/tests/chroma_tests.rs.
  Chroma parameters are keyMode, manualKeyColor, threshold, softness, despillStrength, haloPixels.
  Single-frame preview writes previews/source.png, previews/processed.png, previews/preview.json.
  Batch processing writes processed/frame_00001.png and processed/bboxes.json.
  Tests must prove green background alpha becomes 0 and white foreground alpha stays 255.
  Do not implement normalization, quality verdicts, export, pack, or UI.
  ```

- [x] **Step 2: Main thread wires module export**

  Main thread adds matting and preview modules to `/Users/kartz/Development/Forge/packages/core/src/lib.rs`.

- [x] **Step 3: Verify chroma tests**

  Run:

  ```bash
  cargo test -p core chroma
  ```

  Expected result:

  ```text
  chroma behavior tests pass
  ```

## Phase 3C: Frames, Anchor, Quality, and Loop

**Owner:** Frames Quality Worker

**Original spec:** Tasks 5 and 6 from the MVP plan.

**Files:**
- Create: `/Users/kartz/Development/Forge/packages/core/src/frames/bbox.rs`
- Create: `/Users/kartz/Development/Forge/packages/core/src/frames/normalize.rs`
- Create: `/Users/kartz/Development/Forge/packages/core/src/frames/anchor.rs`
- Create: `/Users/kartz/Development/Forge/packages/core/src/quality/mod.rs`
- Create: `/Users/kartz/Development/Forge/packages/core/src/quality/metrics.rs`
- Create: `/Users/kartz/Development/Forge/packages/core/src/quality/looping.rs`
- Create: `/Users/kartz/Development/Forge/packages/core/tests/anchor_tests.rs`
- Create: `/Users/kartz/Development/Forge/packages/core/tests/quality_tests.rs`

- [x] **Step 1: Dispatch Frames Quality Worker**

  Worker prompt must include:

  ```text
  You own only Task 5 and Task 6: Normalize Canvas, Anchor, Quality Report, and Loop Range.
  Write only packages/core/src/frames/**, packages/core/src/quality/**, packages/core/tests/anchor_tests.rs, and packages/core/tests/quality_tests.rs.
  Frame bbox metrics are left, top, right, bottom, width, height, centerX, centerY, bottomY, alphaCoverage.
  MVP canvas modes are square_bottom, square_center, auto_width_center; default square_bottom.
  Default foot anchor is x = canvasWidth / 2 and y = canvasHeight - marginBottom.
  Quality verdicts are blocked, game_ready, needs_cleanup, prototype_usable.
  Recommendations are adjust_anchor, trim_loop_range, increase_chroma_threshold, reduce_chroma_threshold, use_shorter_clip, increase_canvas_margin.
  Include cellBoundarySafe in the quality report shape because Task 1 requires it.
  Do not implement export pack layout or UI.
  ```

- [x] **Step 2: Main thread wires module export**

  Main thread adds frames and quality modules to `/Users/kartz/Development/Forge/packages/core/src/lib.rs`.

- [x] **Step 3: Verify anchor and quality tests**

  Run:

  ```bash
  cargo test -p core anchor
  cargo test -p core quality
  ```

  Expected result:

  ```text
  normalization, anchor, metric, loop score, and verdict tests pass
  ```

## Phase 4: Export, Manifest, GIF, and Pack Round Trip

**Owner:** Export Pack Worker

**Original spec:** Task 7 from the MVP plan.

**Files:**
- Create: `/Users/kartz/Development/Forge/packages/core/src/export/sheet.rs`
- Create: `/Users/kartz/Development/Forge/packages/core/src/export/gif.rs`
- Create: `/Users/kartz/Development/Forge/packages/core/src/export/manifest.rs`
- Create: `/Users/kartz/Development/Forge/packages/pack/src/lib.rs`
- Create: `/Users/kartz/Development/Forge/packages/pack/tests/pack_tests.rs`

- [x] **Step 1: Dispatch Export Pack Worker**

  Worker prompt must include:

  ```text
  You own only Task 7: Export Frames, Sheet, GIF, Manifest, and Pack.
  Write only packages/core/src/export/** and packages/pack/**.
  Export frames to exports/<export_id>/frames/frame_001.png style paths.
  Sprite sheet parameters are columns, paddingPx, marginPx, maxTextureSize.
  GIF parameters are fps, loop true, background transparent or checkerboard.
  .gsfpack layout must contain forgepack.json, previews/preview.gif, assets/frames/, assets/sprite_sheet.png, assets/atlas.json, assets/manifest.json, quality-report.json.
  Pack tests must validate schema, re-import, and frame count round trip.
  Do not edit UI, Tauri config, distribution docs, or video/matting/frames internals.
  ```

- [x] **Step 2: Main thread wires module export and package dependency**

  Main thread adds export modules to `/Users/kartz/Development/Forge/packages/core/src/lib.rs` and links `packages/pack` to the workspace.

- [x] **Step 3: Verify export and pack tests**

  Run:

  ```bash
  cargo test -p core export
  cargo test -p pack pack
  ```

  Expected result:

  ```text
  export tests and pack round-trip tests pass
  ```

## Phase 5: macOS App MVP UI

**Owner:** UI Worker, integrated by Main Integrator

**Original spec:** Task 8 from the MVP plan plus MVP sections of `/Users/kartz/Development/Forge/GAME-SPRITE-FORGE-UI-DESIGN-SPEC.md`.

**Files:**
- Create: `/Users/kartz/Development/Forge/apps/mac/src/App.tsx`
- Create: `/Users/kartz/Development/Forge/apps/mac/src/routes/ForgeRoute.tsx`
- Create: `/Users/kartz/Development/Forge/apps/mac/src/routes/SettingsRoute.tsx`
- Create: `/Users/kartz/Development/Forge/apps/mac/src/components/ImportPanel.tsx`
- Create: `/Users/kartz/Development/Forge/apps/mac/src/components/VideoSegmentPanel.tsx`
- Create: `/Users/kartz/Development/Forge/apps/mac/src/components/ChromaPreviewPanel.tsx`
- Create: `/Users/kartz/Development/Forge/apps/mac/src/components/CanvasPreview.tsx`
- Create: `/Users/kartz/Development/Forge/apps/mac/src/components/FrameTimeline.tsx`
- Create: `/Users/kartz/Development/Forge/apps/mac/src/components/QualityInspector.tsx`
- Create: `/Users/kartz/Development/Forge/apps/mac/src/components/ExportPanel.tsx`

- [x] **Step 1: Dispatch UI Worker for static MVP shell**

  Worker prompt must include:

  ```text
  You own only Task 8 UI files under apps/mac/src/**.
  Build the MVP workbench UI using the design contract in GAME-SPRITE-FORGE-UI-DESIGN-SPEC.md and the visual baseline in figma-import/game-sprite-forge-app.html.
  Navigation shows Forge, Packs, Exports, Settings.
  Workflow tabs show Import, Frames, Background, Anchor, Sheet, Export.
  Import buttons are Import Video, Import PNG Sequence, Import Sprite Sheet, Import .gsfpack.
  Quality Inspector shows Verdict, BBox Bottom Drift, Center Drift, Loop Match, Frame Count, Alpha Coverage, Recommendations.
  Settings shows ffmpeg path, ffprobe path, default output folder, default FPS, default sheet size.
  Do not show Generate, Marketplace, Creator Publish, provider API key fields, BYOK, cost preflight, online registry, or AI controls.
  Do not edit src-tauri, Rust core, package manifests, or tauri.conf.json.
  ```

- [x] **Step 2: Main thread wires Tauri commands**

  Main thread creates commands in `/Users/kartz/Development/Forge/apps/mac/src-tauri/**` that expose the stabilized core operations:

  ```text
  check_ffmpeg
  create_job
  import_video
  probe_video
  preview_chroma_frame
  extract_frames
  process_chroma_batch
  normalize_frames
  compute_quality_report
  export_pack
  import_gsfpack
  validate_gsfpack
  ```

- [x] **Step 3: Verify frontend build**

  Run:

  ```bash
  npm --workspace apps/mac run build
  ```

  Expected result:

  ```text
  React/TypeScript build passes
  ```

- [x] **Step 4: Verify local app target**

  Run:

  ```bash
  npm --workspace apps/mac run tauri -- build
  ```

  Expected result:

  ```text
  Tauri build reaches packaging stage or fails only on documented signing/notarization prerequisites
  ```

## Phase 6: Distribution Candidate

**Owner:** Main Integrator, optional Distribution Worker for docs

**Original spec:** Task 9 from the MVP plan.

**Files:**
- Create: `/Users/kartz/Development/Forge/docs/architecture/distribution-mvp.md`
- Create: `/Users/kartz/Development/Forge/examples/outputs/README.md`
- Modify: `/Users/kartz/Development/Forge/apps/mac/src-tauri/tauri.conf.json`

- [x] **Step 1: Document ffmpeg MVP strategy**

  Write `/Users/kartz/Development/Forge/docs/architecture/distribution-mvp.md` with the MVP strategy:

  ```text
  user-configured ffmpeg path
  ```

  Include the exact missing-state message:

  ```text
  Install ffmpeg or choose an ffmpeg binary in Settings.
  ```

- [x] **Step 2: Add sample processing checklist**

  Write `/Users/kartz/Development/Forge/examples/outputs/README.md` with this checklist:

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

- [x] **Step 3: Verify all workspace tests**

  Run:

  ```bash
  cargo test --workspace
  npm --workspace apps/mac run build
  ```

  Expected result:

  ```text
  all Rust tests pass and frontend build passes
  ```

- [x] **Step 4: Verify first release acceptance**

  Public release evidence must prove:

  ```text
  notarized Developer ID DMG passes Gatekeeper assessment
  app launches from the mounted notarized DMG
  ffmpeg missing state is understandable
  sample video processes end to end
  exported .gsfpack validates
  app can re-import exported .gsfpack
  no AI provider or website setup is required
  Developer ID signing and notarization pass Gatekeeper
  ```

  Current 2026-06-05 status:

  ```text
  signed DMG exists
  hdiutil verify passes
  installed app codesign passes
  local /Applications install works
  notarization is not complete for the latest rebuilt local candidate
  Gatekeeper rejects the latest installed app with source=Unnotarized Developer ID
  ```

  Therefore Step 4 is functionally implemented for local QA, but public release acceptance must be re-run after Apple notarization credentials are configured.

### Historical Execution Evidence - 2026-06-04

The following evidence belongs to the 2026-06-04 notarized release-candidate package and is retained as history. It does not describe the latest 2026-06-05 rebuilt candidate, whose SHA-256 is `f2060117105df8477ab8e3eb91f1ba7f1a3a38db5a05f99bbe61bdf630a8116d` and whose current Gatekeeper result is `accepted, source=Notarized Developer ID`.

Passed verification:

```bash
cargo test --workspace
npm --workspace apps/mac run build
npm --workspace apps/mac run smoke:ui
npm --workspace apps/mac run tauri -- build
hdiutil verify target/release/bundle/dmg/Game\ Sprite\ Forge_0.1.0_aarch64.dmg
xcrun notarytool submit target/release/bundle/dmg/Game\ Sprite\ Forge_0.1.0_aarch64.dmg --keychain-profile forge-notary --wait
xcrun stapler staple -v target/release/bundle/dmg/Game\ Sprite\ Forge_0.1.0_aarch64.dmg
xcrun stapler validate -v target/release/bundle/dmg/Game\ Sprite\ Forge_0.1.0_aarch64.dmg
spctl -a -vvv -t install target/release/bundle/dmg/Game\ Sprite\ Forge_0.1.0_aarch64.dmg
spctl -a -vvv --context context:primary-signature -t open target/release/bundle/dmg/Game\ Sprite\ Forge_0.1.0_aarch64.dmg
mount notarized DMG, run spctl -a -vvv -t exec /Volumes/Game\ Sprite\ Forge/Game\ Sprite\ Forge.app, launch app, verify game-sprite-forge-mac process, quit app, detach DMG
scripts/verify-release-package.sh target/release/bundle/dmg/Game\ Sprite\ Forge_0.1.0_aarch64.dmg 35505be18b04ae19a5dd1ff4bc3595a7ba298f13ca7faa908b888e1e1b07d164
scripts/package-release-candidate.sh
cd release-candidates/GameSpriteForge-0.1.0-aarch64-notarized && shasum -a 256 -c SHA256SUMS
cd release-candidates/GameSpriteForge-0.1.0-aarch64-notarized && scripts/verify-release-package.sh ./Game\ Sprite\ Forge_0.1.0_aarch64.dmg 35505be18b04ae19a5dd1ff4bc3595a7ba298f13ca7faa908b888e1e1b07d164
jq empty schemas/gsfpack.schema.json schemas/manifest.schema.json schemas/atlas.schema.json schemas/quality-report.schema.json
```

Additional end-to-end evidence:

```text
apps/mac/src-tauri/tests/sample_pipeline_tests.rs::sample_video_exports_schema_valid_reimportable_pack passed.
This test probes examples/inputs/green-box-character.mp4, extracts frames, runs chroma processing, normalizes frames, computes quality, exports .gsfpack, validates all local JSON Schemas, re-imports the pack with matching frame count, re-exports from imported pack frames, and verifies the re-exported frame count.
apps/mac/src-tauri/src/lib.rs::tests::sprite_sheet_grid_slices_into_raw_frames passed.
This test generates a 2x2 sprite sheet fixture and verifies the import_sprite_sheet grid slicer emits four raw PNG frames.
packages/core/tests/job_store_tests.rs::set_state_persists_job_progress passed.
This test verifies job state transitions are persisted to job.json.
UI smoke passed via apps/mac/scripts/smoke-ui.mjs. It captured apps/mac/smoke-output/forge-workbench.png at 1440x960 and checked latest dist text for required local-workbench UI copy and forbidden AI/marketplace/provider copy.
Latest DMG generated at target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg on 2026-06-04 17:18:13 local time after the .gsfpack re-export test, UI smoke, icon resource, and Developer ID signing update.
hdiutil verify reports checksum VALID for the latest DMG.
Latest stapled DMG SHA-256: 35505be18b04ae19a5dd1ff4bc3595a7ba298f13ca7faa908b888e1e1b07d164.
Mounted app bundle contains Contents/Resources/Game Sprite Forge.icns and Contents/_CodeSignature/CodeResources.
codesign --verify --verbose=2 passes for the DMG.
codesign -dvvv reports Authority=Developer ID Application: Ka Yan (J6P96F432P), TeamIdentifier=J6P96F432P, and secure timestamp for the DMG.
codesign --verify --deep --strict --verbose=2 passes for /Volumes/Game Sprite Forge/Game Sprite Forge.app.
codesign -dvvv reports Authority=Developer ID Application: Ka Yan (J6P96F432P), TeamIdentifier=J6P96F432P, hardened runtime, sealed resources, and secure timestamp for the app bundle.
notarytool submission c21d457e-ffbe-4ebe-848a-48b21468c2ea reached status Accepted.
xcrun stapler staple reports "The staple and validate action worked!" for the DMG.
xcrun stapler validate reports "The validate action worked!" for the DMG.
spctl -a -vvv -t install accepts the DMG with source=Notarized Developer ID.
spctl -a -vvv --context context:primary-signature -t open accepts the DMG with source=Notarized Developer ID.
spctl -a -vvv -t exec accepts /Volumes/Game Sprite Forge/Game Sprite Forge.app with source=Notarized Developer ID.
Local mounted-DMG launch check started /Volumes/Game Sprite Forge/Game Sprite Forge.app and observed game-sprite-forge-mac process id 94529, then quit the app and detached disk9.
Post-launch cleanup confirmed no Game Sprite Forge process, no /Volumes/Game Sprite Forge mount, and no Vite listener on TCP port 1420.
scripts/verify-release-package.sh was added and passed locally against the stapled DMG. It verifies SHA-256, hdiutil, stapler, Gatekeeper DMG assessment, app codesign, Gatekeeper app assessment, mounted-DMG launch, and cleanup. The local script run observed game-sprite-forge-mac process id 20060.
docs/architecture/release-candidate-verification.md was added with the exact release package verification command and manual Gate 5 checklist.
scripts/package-release-candidate.sh was added and passed locally. It creates release-candidates/GameSpriteForge-0.1.0-aarch64-notarized and release-candidates/GameSpriteForge-0.1.0-aarch64-notarized.zip.
Release candidate zip SHA-256: 53a0cf9cd5699d8f2f9e68988ffbcc64d0394ee7a1b1e99383c4c2b68f7aed5c.
release-candidates/GameSpriteForge-0.1.0-aarch64-notarized/SHA256SUMS verifies OK for the DMG, verification script, release candidate verification doc, and package README.
Running scripts/verify-release-package.sh from inside the release candidate package passed locally and observed game-sprite-forge-mac process id 77278.
```

Non-blocking notes:

```text
In-app Browser visual QA was not available from the current tool surface, so UI layout was verified by TypeScript/Vite build plus a Chrome headless smoke screenshot and text scan.
Apple Developer ID signing is now performed with Developer ID Application: Ka Yan (J6P96F432P).
Apple notarization and DMG stapling were complete for the historical 2026-06-04 release-candidate package.
For the latest 2026-06-05 local candidate, notarization is not complete and Gatekeeper still rejects the installed app.
Optional external macOS smoke verification remains available through the release candidate package, but the latest rebuilt artifact must pass notarization/stapling/Gatekeeper before it can be treated as a public release candidate.
```

## Multi-Agent Dispatch Schedule

Use this schedule during execution.

```text
Round 0:
  Main Integrator: Phase 0 bootstrap

Round 1:
  Contracts Worker: Phase 1
  UI Worker may start only static shell after contracts exist

Round 2:
  Job Core Worker: Phase 2

Round 3:
  Video Worker: Phase 3A
  Matting Worker: Phase 3B
  Frames Quality Worker: Phase 3C

Round 4:
  Export Pack Worker: Phase 4
  UI Worker continues command-ready panels if static shell is already approved

Round 5:
  Main Integrator: Tauri command wiring and app-core integration
  Spec Reviewer: Gate 1-4 compliance
  Code Quality Reviewer: integrated code review

Round 6:
  Distribution Worker or Main Integrator: Phase 6
  Main Integrator: Gate 5 evidence and final acceptance
```

Do not start Round 3 until `cargo test -p core job_store` passes. Do not start Round 4 until the main thread confirms stable normalized frame and quality report types.

## Completion Audit Checklist

Before marking this MVP implementation complete, verify every item below with current-state evidence.

- [x] `docs/architecture/mvp-scope.md` exists and excludes AI, BYOK, website, MCP, marketplace, cloud.
- [x] `schemas/gsfpack.schema.json`, `manifest.schema.json`, `atlas.schema.json`, and `quality-report.schema.json` exist and validate example outputs.
- [x] `packages/core` exists and has passing tests for job, video, chroma, anchor, quality, and export modules.
- [x] `packages/pack` exists and passes pack validation/re-import tests.
- [x] `apps/mac` exists and builds.
- [x] UI exposes Forge, Packs, Exports, Settings.
- [x] UI hides Generate, Marketplace, Creator Publish, provider API key settings, and cost preflight.
- [x] ffmpeg missing state uses the exact required user-facing message.
- [x] sample fixture instructions exist in `examples/inputs/README.md`.
- [x] exported `.gsfpack` layout matches the MVP plan.
- [x] app can re-import its own `.gsfpack`.
- [x] `docs/architecture/distribution-mvp.md` documents user-configured ffmpeg path.
- [x] `examples/outputs/README.md` contains the end-to-end checklist.
- [x] Gate 1 through Gate 5 evidence is recorded in the final status update.

## Self-Review

Spec coverage:

- Original Tasks 1-9 are mapped to Phases 1-6.
- Workspace bootstrap is added because the current directory has no formal `Cargo.toml`, `package.json`, `apps/mac`, `packages/core`, or `packages/pack`.
- UI scope incorporates the MVP sections of `GAME-SPRITE-FORGE-UI-DESIGN-SPEC.md`.
- Multi-agent write scopes avoid overlapping module ownership.
- Deferred AI, website, MCP, and marketplace work is explicitly excluded.

Known execution risks:

- Git is not initialized. Worker execution must choose git mode before code changes.
- Tauri setup may require platform dependencies outside this plan. If packaging fails only on signing/notarization, record that separately from app build failure.
- ffmpeg may be absent locally. Tests must distinguish pure unit coverage from integration tests requiring a configured binary.
- Existing `generated-assets` are useful references but are not a substitute for the required `examples/inputs/green-box-character.mp4` fixture.
