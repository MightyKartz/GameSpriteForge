# Forge UI/UX Workflow Focus Evidence

Date: 2026-06-11

## Scope

This pass implemented the development plan at:

```text
docs/superpowers/plans/2026-06-11-forge-ui-ux-workflow-focus.md
```

The implemented slice covers:

```text
canonical workbench stage model
stage-aware right rail for early workflow actions
compact footer stage status instead of duplicate primary CTAs
timeline evidence strip for target/actual/range/sample/loop/selection
preview mode label and inspection overlay toggle
focus-visible coverage for summary, textarea, and generic tabindex controls
workflow-focus source guard
updated UI smoke expectations for the current Godot-capable product surface
```

## Skills And MCP Used

Skills used:

```text
forge-dev
ui-ux-pro-max
superpowers:subagent-driven-development
superpowers:dispatching-parallel-agents
superpowers:executing-plans
```

MCP/tooling used:

```text
tool_search to discover multi-agent tooling
multi_agent_v1 to dispatch three read-only explorer agents
```

Subagent roles:

```text
Copernicus: canonical WorkbenchStage, stage-aware right rail, duplicate CTA review
Kierkegaard: timeline evidence, preview mode/overlay, contrast/focus review
Confucius: source guard, QA documentation, verification strategy
```

Key agent findings applied:

```text
Stage should be derived once in forgeViewModel.ts and consumed by ForgeRoute.
Footer workflow actions should become compact status to avoid competing CTAs.
Validation stage must be tied to exportOutput && packSummary, not packSummary alone.
Cmd+Enter should require extracted frames so it does not bypass the Extract step after video import.
Timeline actual count should come from real timeline frame paths, not probe estimate.
Preview overlays must stay anchored inside preview-frame-stage.
summary and textarea focus-visible styles needed explicit coverage.
The old smoke forbidden "Godot 4" rule was obsolete after Godot export support landed.
```

## Implementation Notes

Stage model:

```text
apps/mac/src/forgeViewModel.ts exports WorkbenchStage and deriveWorkbenchStage.
apps/mac/src/routes/ForgeRoute.tsx derives workbenchStage once from source, frame, quality, export, validation, Godot, and running state.
```

Right rail and footer:

```text
ForgeRoute renders StageActionPanel for early workflow stages.
QualityInspector and ExportPanel are gated by workbenchStage.
The footer now shows footer-stage-status before export and footer-export-status after export.
The old footer primary Process & Quality CTA was removed.
```

Timeline and preview:

```text
FrameTimeline accepts TimelineEvidence and renders timeline-evidence-strip.
CanvasPreview accepts previewMode, inspectionEnabled, and onInspectionToggle.
Inspection overlays remain data-driven through preview-frame-stage coordinate mapping.
Static center/foot guide lines are hidden when a real frame is shown without the inspection overlay.
```

Quality gates:

```text
scripts/test-workflow-focus-source.mjs guards the stage model, right rail, footer status, timeline evidence, preview mode, focus styles, and i18n keys.
package.json now runs that guard in npm run test:scripts.
apps/mac/scripts/smoke-ui.mjs now allows legitimate Godot 4 export copy and expects the new stage-aware empty-state copy.
```

## Verification

Commands run:

```bash
node --check scripts/test-workflow-focus-source.mjs && node scripts/test-workflow-focus-source.mjs
npm --workspace apps/mac run build
npm run test:scripts
FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:mvp
FORGE_SMOKE_LOCALE=zh-CN npm --workspace apps/mac run smoke:ui:mvp
npm run install:mac
codesign --force --deep --sign - "target/release/bundle/macos/Game Sprite Forge.app" && scripts/install-local-mac-app.sh
codesign --verify --deep --strict --verbose=2 "/Applications/Game Sprite Forge.app"
```

Results:

```text
Workflow focus source guard: pass
Frontend build: pass
Full source/script guards: pass
English MVP UI smoke: pass
Chinese MVP UI smoke: pass
npm run install:mac: app build completed, Developer ID bundle signing failed because timestamp was missing
Ad-hoc local install fallback: pass
Installed app codesign verification: pass
```

Smoke artifacts copied to:

```text
docs/qa/artifacts/forge-ui-ux-workflow-focus-20260611/forge-workbench-mvp-en-US.png
docs/qa/artifacts/forge-ui-ux-workflow-focus-20260611/forge-workbench-mvp-zh-CN.png
docs/qa/artifacts/forge-ui-ux-workflow-focus-20260611/forge-workbench-mvp-visible-text-en-US.txt
docs/qa/artifacts/forge-ui-ux-workflow-focus-20260611/forge-workbench-mvp-visible-text-zh-CN.txt
```

## Remaining Manual QA

Manual installed-app QA is still recommended for the complete media path:

```text
choose video
extract target 24 frames
process and check quality
toggle inspection overlay
confirm timeline evidence values
export pack
export Godot project
validate re-import
```

The automated MVP smoke covers the empty/import first screen in both English
and Chinese. It does not replace a real installed-app click-through using a
local video and exported pack.

Installed app for manual testing:

```text
/Applications/Game Sprite Forge.app
Bundle identifier: dev.gamespriteforge.desktop
Version: 0.1.0
Signature: ad-hoc local test signature
```

Release packaging still needs the Developer ID timestamp path to succeed; this
local install fallback is only for manual UI testing.
