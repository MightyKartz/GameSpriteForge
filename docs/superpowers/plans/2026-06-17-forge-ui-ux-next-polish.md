# Forge UI/UX Next Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Improve the Forge workbench's right rail, timeline density, keyboard QA evidence, and dense dark UI readability without changing the product direction.

**Architecture:** Keep the current Tauri/React workbench structure. Add source-level UI invariants to `scripts/test-workflow-focus-source.mjs`, then implement narrowly in existing components and CSS. Record real evidence under `docs/qa/` and keep development tooling out of product UI.

**Tech Stack:** React + TypeScript in `apps/mac/src`, CSS in `apps/mac/src/styles/app.css`, Node source guards in `scripts/`, Tauri debug app QA for macOS evidence.

---

## File Structure

- Modify `scripts/test-workflow-focus-source.mjs`: add next-polish guards for right rail checklist semantics, timeline density controls, focus/contrast CSS hooks, and QA doc references.
- Modify `apps/mac/src/routes/ForgeRoute.tsx`: refine `StageActionPanel` and right-rail stage evidence structure without duplicating empty-state primary CTAs.
- Modify `apps/mac/src/components/FrameTimeline.tsx`: improve selected-frame summary, loop range visibility, and density metadata without changing public props.
- Modify `apps/mac/src/styles/app.css`: tune right rail, timeline density, focus, and readable text styles while preserving Forge's dark desktop utility design.
- Create `docs/qa/forge-ui-ux-next-polish-2026-06-17.md`: current-run QA ledger with commands, screenshots, keyboard notes, and residual risks.
- Create `docs/qa/artifacts/forge-ui-ux-next-polish-20260617/README.md`: artifact acceptance rules and accepted file list.
- Create `docs/architecture/forge-ui-ux-next-polish-2026-06-17.md`: concise architecture note if behavior or structure changes beyond CSS.

## Task 1: Source Guard Red Tests

**Files:**
- Modify: `scripts/test-workflow-focus-source.mjs`

- [x] **Step 1: Add failing guards**

Add checks requiring:

```js
assertContains(routeSource, "stage-checklist", "StageActionPanel must expose a compact checklist structure.");
assertContains(routeSource, "data-stage-check", "Stage checklist items must expose stable check keys.");
assertContains(frameTimelineSource, "timeline-density", "FrameTimeline must expose density metadata.");
assertContains(frameTimelineSource, "timeline-selected-summary", "Selected timeline frame must have a readable summary.");
assertContains(cssSource, ".stage-action-panel.stage-empty", "Empty stage right rail must use a quieter explanatory treatment.");
assertContains(cssSource, ".timeline-density", "CSS must style the timeline density row.");
assertContains(cssSource, ".focus-evidence-ring", "CSS must provide a reusable strong focus evidence ring.");
assertContains(cssSource, "--readable-small", "CSS must define a readable small text token.");
```

- [x] **Step 2: Verify red**

Run:

```bash
node scripts/test-workflow-focus-source.mjs
```

Expected: FAIL on the first missing next-polish guard.

## Task 2: Right Rail Information Architecture

**Files:**
- Modify: `apps/mac/src/routes/ForgeRoute.tsx`
- Modify: `apps/mac/src/styles/app.css`

- [x] **Step 1: Implement stage checklist**

In `StageActionPanel`, add a compact `stage-checklist` list for source, frames, quality, and output. Each item should expose `data-stage-check`, use `role="listitem"`, and render a concise status label.

- [x] **Step 2: Quiet empty-stage panel**

Keep the central import launcher as the only empty primary CTA. Style `.stage-action-panel.stage-empty` as an explanatory state with lower visual weight and no primary button.

- [x] **Step 3: Verify**

Run:

```bash
node scripts/test-workflow-focus-source.mjs
```

Expected: remaining failures only from tasks not implemented yet.

## Task 3: Timeline Density And Selected Frame Clarity

**Files:**
- Modify: `apps/mac/src/components/FrameTimeline.tsx`
- Modify: `apps/mac/src/styles/app.css`

- [x] **Step 1: Add density metadata**

Expose a `timeline-density` row or chip that summarizes visible thumbnails, total frames, selected frame, and loop range. Keep `FrameTimeline` props unchanged.

- [x] **Step 2: Strengthen selected summary**

Add `timeline-selected-summary` text that remains readable at narrow widths and duplicates selected-frame state in text, not color alone.

- [x] **Step 3: Verify**

Run:

```bash
node scripts/test-workflow-focus-source.mjs
```

Expected: source guard passes after shared CSS hooks are complete.

## Task 4: Readability, Focus, And Contrast Hooks

**Files:**
- Modify: `apps/mac/src/styles/app.css`

- [x] **Step 1: Add readable text token**

Define `--readable-small: 12px` in `:root` and apply it to dense helper text where appropriate.

- [x] **Step 2: Add strong focus evidence class**

Create `.focus-evidence-ring` and apply equivalent visual treatment to focus-visible states used in import, timeline, and stage controls.

- [x] **Step 3: Verify**

Run:

```bash
node scripts/test-workflow-focus-source.mjs
git diff --check
```

Expected: both commands exit 0.

## Task 5: QA Evidence And Documentation

**Files:**
- Create: `docs/qa/forge-ui-ux-next-polish-2026-06-17.md`
- Create: `docs/qa/artifacts/forge-ui-ux-next-polish-20260617/README.md`
- Create: `docs/architecture/forge-ui-ux-next-polish-2026-06-17.md`

- [x] **Step 1: Create QA ledger**

Record planned and final evidence slots for build, scripts, MVP smoke, responsive smoke, workspace debug app, native keyboard focus, browser/CDP focus, and known risk items.

- [x] **Step 2: Create artifact folder**

Document accepted filenames for:

```text
01-smoke-mvp-en-US.png
02-smoke-mvp-zh-CN.png
03-smoke-responsive-1120-en-US.png
04-browser-focus-next-polish.png
10-real-app-next-polish.png
```

- [x] **Step 3: Create architecture note**

Explain that this pass refines existing right rail/timeline/focus semantics without introducing a new design system.

## Task 6: Final Verification And PR

**Files:**
- No code edits unless verification finds a concrete issue.

- [ ] **Step 1: Run required checks**

Run:

```bash
npm --workspace apps/mac run build
npm run test:scripts
FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:mvp
FORGE_SMOKE_LOCALE=zh-CN npm --workspace apps/mac run smoke:ui:mvp
FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:responsive
```

- [ ] **Step 2: Run real app QA if feasible**

Run:

```bash
npm --workspace apps/mac run tauri -- build --debug --bundles app
```

Launch `/Users/kartz/Development/Forge/target/debug/bundle/macos/Game Sprite Forge.app` and capture current-run screenshots into `docs/qa/artifacts/forge-ui-ux-next-polish-20260617/`.

- [ ] **Step 3: Publish**

Stage only intended files, commit, push `codex/forge-ui-ux-next-polish`, and create an English draft PR with Summary, Tests, Screenshots/QA Evidence, and Risks. If PR #1 is still unmerged, use `codex/forge-ui-ux-polish` as the PR base.
