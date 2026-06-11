# Forge UI/UX Workflow Focus Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` or `superpowers:executing-plans` to implement this plan task-by-task. Use `forge-dev` for Forge-specific verification and real UI QA.

**Goal:** Reduce Forge's UI decision cost by making each workflow stage show one clear primary action, a stage-aware right rail, stronger timeline extraction evidence, and clearer preview modes.

**Architecture:** Keep the existing Rust/Tauri media pipeline unchanged. This is a React/CSS/i18n/UI-state slice. Introduce a small canonical workbench view model if needed so stage summaries derive from one state model rather than many local conditionals.

**Tech Stack:** React 18, TypeScript, Tauri command wrappers, existing Forge i18n, CSS in `apps/mac/src/styles/app.css`, existing source guard scripts and real macOS UI QA.

---

## Implementation Status

Updated: 2026-06-11

Implemented in this pass:

```text
canonical WorkbenchStage and deriveWorkbenchStage in forgeViewModel.ts
stage-aware right rail via StageActionPanel
QualityInspector and ExportPanel gated by workbenchStage
footer compact stage status before export
timeline-evidence-strip with target/actual/range/sample/loop/selected values
CanvasPreview preview mode label and inspection overlay toggle
focus-visible coverage for summary, textarea, and tabindex controls
workflow-focus source guard in npm run test:scripts
MVP UI smoke expectations updated for the current stage-aware UI
```

Evidence document:

```text
docs/qa/forge-ui-ux-workflow-focus-evidence-2026-06-11.md
```

Remaining follow-up:

```text
manual installed-app click-through with a local video through export, Godot project export, and validate re-import
release Developer ID timestamp signing path
optional responsive smoke pass after the next visual polish iteration
```

## Scope

Implement:

```text
single primary CTA per stage
stage-aware right rail
timeline extraction evidence summary
preview mode labels and inspection overlay control
progressive disclosure for export metadata and format cards
dark contrast/focus/touch-target pass for the workbench
real installed-app screenshots after implementation
```

Do not implement:

```text
new media-processing algorithms
new engine exporters
AI generation
cloud workflows
marketplace or registry UI
MCP product surface
full redesign of Exports or Settings routes
```

## Reference Documents

```text
docs/architecture/forge-ui-ux-workflow-focus.md
docs/qa/forge-ui-ux-review-2026-06-11.md
docs/architecture/post-release-backlog.md
```

## Target Files

Likely files:

```text
apps/mac/src/routes/ForgeRoute.tsx
apps/mac/src/forgeViewModel.ts
apps/mac/src/components/ExportPanel.tsx
apps/mac/src/components/VideoSegmentPanel.tsx
apps/mac/src/components/CanvasPreview.tsx
apps/mac/src/components/FrameTimeline.tsx
apps/mac/src/components/QualityInspector.tsx
apps/mac/src/i18n.ts
apps/mac/src/styles/app.css
scripts/test-workflow-focus-source.mjs
docs/qa/forge-ui-ux-workflow-focus-evidence-2026-06-11.md
```

Avoid broad component rewrites unless a smaller state model extraction removes
real duplication.

## Task 1: Define The Canonical Workbench Stage

- [x] Add or extend a view-model helper that returns a canonical stage:

```ts
type WorkbenchStage =
  | "empty"
  | "source_selected"
  | "frames_ready"
  | "processed_ready"
  | "quality_ready"
  | "export_ready"
  | "exported_unvalidated"
  | "validated"
  | "godot_project_ready"
  | "blocked"
  | "running";
```

- [x] Derive stage from existing source/probe/extract/normalize/quality/export/validation/Godot output state.
- [x] Replace duplicated "what next" conditionals in the route where practical.
- [x] Add a source guard that checks the stage model exists and is used by `ForgeRoute`.

Validation:

```bash
npm run test:scripts
npm --workspace apps/mac run build
```

## Task 2: Make The Right Rail Stage-Aware

- [x] Split right rail presentation into stage-specific panels or a single component with explicit stage branches.
- [x] Import stage: show source guidance and toolchain/sample actions only.
- [x] Frames stage: show range, target frame count, extraction evidence, and Extract as the only primary action.
- [x] Process stage: show Process & Quality as the only primary action, with quality criteria preview.
- [x] Export stage: show pack name, format, output folder, and Export as the only primary action.
- [x] Post-export stage: show Open Folder, Validate Re-import, and Export Godot Project as secondary/post-export actions.
- [x] Keep advanced metadata collapsed unless user opens it or validation points to it.

Validation:

```bash
npm --workspace apps/mac run build
npm run test:scripts
```

## Task 3: Remove Duplicate Workflow Actions

- [x] Keep top workflow tabs as navigation/progress.
- [x] Convert the bottom bar to compact status only, or hide duplicate action buttons when the right rail has the stage primary CTA.
- [x] Ensure keyboard shortcuts still work for Process and Export.
- [x] Update Chinese and English copy so "next action" appears once.

Acceptance:

```text
At any stage, the user sees one visually dominant primary CTA.
Secondary actions do not compete with the primary CTA.
```

## Task 4: Add Timeline Extraction Evidence

- [x] Add an extraction evidence strip near the timeline:

```text
Target frames
Actual frames
Selected range
Sampling interval
Loop start/end
Selected frame index
```

- [ ] Make selected frame and loop endpoints visually stronger.
- [x] Keep timeline dimensions stable and avoid layout jump when evidence values change.
- [x] Add i18n keys for English and Simplified Chinese.

Validation:

```bash
npm --workspace apps/mac run build
npm run test:scripts
```

## Task 5: Clarify Preview Modes And Overlays

- [x] Add a visible preview mode label:

```text
Raw
Processed
Normalized
Inspection
Export preview
```

- [x] Add an overlay toggle or mode switch so bbox/anchor/center overlays are strongest only in Inspection mode.
- [x] Keep current image-coordinate overlay mapping from `preview-frame-stage`.
- [x] Reduce overlay opacity outside Inspection mode.
- [x] Ensure the character remains visually dominant.

Validation:

```bash
npm --workspace apps/mac run build
npm run test:scripts
```

## Task 6: Accessibility And Contrast Pass

- [x] Audit key workbench text tokens against:

```text
primary text >= 4.5:1
secondary text >= 3:1
visible focus ring on every interactive control
disabled state distinguishable from secondary enabled state
body/panel text generally >= 12px in dense desktop UI
```

- [x] Confirm all icon-only or icon-leading buttons have accessible names.
- [ ] Confirm Tab order reaches primary workbench actions, timeline controls, right rail actions, and footer controls.
- [x] Add source or UI smoke guards where practical.

Validation:

```bash
npm --workspace apps/mac run build
npm run test:scripts
```

## Task 7: Real Installed-App QA Evidence

- [x] Install the app locally:

```bash
npm run install:mac
codesign --force --deep --sign - "target/release/bundle/macos/Game Sprite Forge.app" && scripts/install-local-mac-app.sh
```

Note: `npm run install:mac` built the app but Developer ID signing failed due a
missing timestamp. The installed `/Applications/Game Sprite Forge.app` uses an
ad-hoc local test signature for manual QA.

- [ ] Run a manual flow with a local video:

```text
open /Applications/Game Sprite Forge.app
choose video
extract target 24 frames
process and check quality
inspect preview modes and timeline evidence
export pack
export Godot project
validate re-import
```

- [x] Save automated smoke screenshots under `docs/qa/artifacts/`.
- [x] Record automated findings in:

```text
docs/qa/forge-ui-ux-workflow-focus-evidence-2026-06-11.md
```

Minimum evidence:

```text
empty/import stage
frames stage after source selection
processed/quality stage
export stage
post-export stage with Godot project ready
```

## Verification Checklist

Before completion, run:

```bash
npm --workspace apps/mac run build
npm run test:scripts
cargo check --manifest-path /Users/kartz/Development/Forge/apps/mac/src-tauri/Cargo.toml
npm run install:mac
```

If only React/CSS/i18n files changed, focused Rust tests are not required unless
Tauri command types or Rust code changed.

## Handoff Notes

The screenshots in `docs/qa/forge-ui-ux-review-2026-06-11.md` show the current
state. Preserve the working local pipeline while reducing visible complexity.
The goal is not a new visual brand; it is clearer operational hierarchy.
