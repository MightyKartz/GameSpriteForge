import { readFileSync } from "node:fs";

const packageJson = readFileSync("package.json", "utf8");
const viewModelSource = readFileSync("apps/mac/src/forgeViewModel.ts", "utf8");
const routeSource = readFileSync("apps/mac/src/routes/ForgeRoute.tsx", "utf8");
const canvasPreviewSource = readFileSync("apps/mac/src/components/CanvasPreview.tsx", "utf8");
const exportPanelSource = readFileSync("apps/mac/src/components/ExportPanel.tsx", "utf8");
const frameTimelineSource = readFileSync("apps/mac/src/components/FrameTimeline.tsx", "utf8");
const cssSource = readFileSync("apps/mac/src/styles/app.css", "utf8");
const i18nSource = readFileSync("apps/mac/src/i18n.ts", "utf8");

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function assertContains(source, needle, message) {
  assert(source.includes(needle), message);
}

function assertNotContains(source, needle, message) {
  assert(!source.includes(needle), message);
}

for (const stage of [
  "empty",
  "source_selected",
  "frames_ready",
  "processed_ready",
  "quality_ready",
  "export_ready",
  "exported_unvalidated",
  "validated",
  "godot_project_ready",
  "blocked",
  "running",
]) {
  assertContains(viewModelSource, `"${stage}"`, `WorkbenchStage must include ${stage}.`);
}

assertContains(viewModelSource, "export type WorkbenchStage", "forgeViewModel must export the canonical workbench stage type.");
assertContains(viewModelSource, "deriveWorkbenchStage", "forgeViewModel must derive the canonical stage in one helper.");
assertContains(routeSource, "const workbenchStage = deriveWorkbenchStage", "ForgeRoute must derive one workbenchStage.");
assertContains(routeSource, "hasValidation: Boolean(exportOutput && packSummary)", "Validation stage must be tied to the current export.");
assertContains(routeSource, "showStageActionPanel", "ForgeRoute must branch early right-rail actions by stage.");
assertContains(routeSource, "showQualityInspector", "ForgeRoute must gate quality inspector visibility by stage.");
assertContains(routeSource, "showExportPanel", "ForgeRoute must gate export panel visibility by stage.");
assertContains(routeSource, "StageActionPanel", "ForgeRoute must render a stage-aware right rail action panel.");
assertContains(routeSource, "stage={workbenchStage}", "StageActionPanel must receive the canonical stage.");
assertContains(routeSource, "stage-primary-action", "StageActionPanel must expose one primary action.");
assertContains(routeSource, "footer-stage-status", "Footer must show compact workflow stage status.");
assertNotContains(routeSource, 'className="status-action primary"', "Footer must not reintroduce the duplicate primary pipeline CTA.");
assertContains(routeSource, "canProcessCurrentFrames", "Keyboard Process shortcut must require extracted frames.");
assert(
  (exportPanelSource.match(/export\.exportGodotProject/g) ?? []).length === 1,
  "ExportPanel must not render duplicate Godot export actions.",
);

assertContains(frameTimelineSource, "type TimelineEvidence", "FrameTimeline must type timeline evidence.");
assertContains(frameTimelineSource, "timeline-evidence-strip", "FrameTimeline must render a compact evidence strip.");
assertContains(frameTimelineSource, "targetFrameCount", "Timeline evidence must include target frame count.");
assertContains(frameTimelineSource, "actualFrameCount", "Timeline evidence must include actual frame count.");
assertContains(frameTimelineSource, "samplingInterval", "Timeline evidence must include sampling interval.");
assertContains(frameTimelineSource, "loopStartFrame", "Timeline evidence must include loop start.");
assertContains(frameTimelineSource, "loopEndFrame", "Timeline evidence must include loop end.");
assertContains(frameTimelineSource, 'className="timeline-loop-markers"', "FrameTimeline must expose loop start/end markers.");
assertContains(frameTimelineSource, 'data-loop-role="start"', "Loop start marker must expose a semantic role for tests and accessibility.");
assertContains(frameTimelineSource, 'data-loop-role="end"', "Loop end marker must expose a semantic role for tests and accessibility.");
assertContains(frameTimelineSource, 'role="list"', "Timeline evidence strip must expose a list semantics container.");
assertContains(frameTimelineSource, 'role="listitem"', "Timeline evidence items must expose list item semantics.");
assertContains(frameTimelineSource, "data-evidence={item.key}", "Timeline evidence items must expose stable evidence keys.");
assertContains(frameTimelineSource, 'aria-current={isSelected ? "true" : undefined}', "Selected timeline frame must expose aria-current.");
assertContains(frameTimelineSource, "title={selectedFrameLabel}", "Selected timeline frame state must be discoverable without relying on color.");
assertContains(routeSource, "actualFrameCount: timelineFramePaths.length", "Route must pass actual rendered frame count into timeline evidence.");

assertContains(canvasPreviewSource, "type PreviewMode", "CanvasPreview must type preview mode.");
assertContains(canvasPreviewSource, "preview-mode-label", "CanvasPreview must label the active preview mode.");
assertContains(canvasPreviewSource, "inspectionEnabled", "CanvasPreview must support inspection overlay toggling.");
assertContains(canvasPreviewSource, "aria-pressed={inspectionEnabled}", "Inspection toggle must expose pressed state.");
assertContains(canvasPreviewSource, "data-preview-mode={previewMode}", "CanvasPreview must expose the active mode as stable state.");
assertContains(canvasPreviewSource, "data-preview-mode-label={previewMode}", "Preview mode labels must expose stable mode keys.");
assertContains(canvasPreviewSource, "data-preview-processing-state={previewProcessingState}", "CanvasPreview must expose before/after/inspection processing state.");
assertContains(canvasPreviewSource, 'raw: "before-processing"', "Raw preview must be identifiable as before processing.");
assertContains(canvasPreviewSource, 'normalized: "after-processing"', "Normalized preview must be identifiable as after processing.");
assertContains(canvasPreviewSource, 'inspection: "inspection"', "Inspection preview must expose a stable inspection state.");
assertContains(canvasPreviewSource, "aria-controls={inspectionOverlayId}", "Inspection toggle must point at the overlay it controls.");
assertContains(canvasPreviewSource, "data-inspection-overlay={hasOverlay ? \"visible\" : \"hidden\"}", "Inspection overlay state must be discoverable without color.");
assertContains(canvasPreviewSource, "role=\"img\"", "Inspection overlays must expose a readable semantic description.");
assertContains(canvasPreviewSource, "data-overlay-part=\"bbox\"", "Inspection bbox overlay must expose a stable part key.");
assertContains(routeSource, "previewInspectionEnabled", "ForgeRoute must own the inspection overlay toggle state.");
assertContains(routeSource, "overlay={selectedOverlay}", "ForgeRoute must pass selected overlay only through preview state.");
assertContains(routeSource, "previewMode={previewMode}", "ForgeRoute must pass preview mode to CanvasPreview.");
assertContains(routeSource, "type StageActionKey", "Stage actions must expose stable next-step keys.");
assertContains(routeSource, "data-workbench-stage={stage}", "StageActionPanel must expose the current stage.");
assertContains(routeSource, "data-stage-next-action={action?.key ?? \"import\"}", "StageActionPanel must expose the next action even when the center CTA owns it.");
assertContains(routeSource, "{action ? (", "StageActionPanel must avoid rendering a duplicate primary action for empty state.");
assertNotContains(routeSource, "workflowStage.action.chooseSource", "Empty state must not duplicate the center import CTA in the right rail.");

assertContains(cssSource, ".stage-action-panel", "CSS must style the stage action panel.");
assertContains(cssSource, ".stage-primary-action", "CSS must style the single stage primary action.");
assertContains(cssSource, ".timeline-evidence-strip", "CSS must style timeline evidence.");
assertContains(cssSource, ".timeline-evidence-item[data-evidence=\"loop\"]", "CSS must make loop evidence visually distinct.");
assertContains(cssSource, ".timeline-loop-marker", "CSS must style loop start/end markers.");
assertContains(cssSource, ".frame-cell[aria-current=\"true\"]", "CSS must expose selected frame state beyond color.");
assertContains(cssSource, ".preview-mode-control", "CSS must style preview mode control.");
assertContains(cssSource, ".canvas-preview[data-preview-processing-state=\"inspection\"] .preview-mode-control", "CSS must emphasize inspection mode state.");
assertContains(cssSource, "summary:focus-visible", "Global focus rings must include disclosure summaries.");
assertContains(cssSource, "textarea:focus-visible", "Global focus rings must include textareas.");

assertContains(i18nSource, "workflowStage.panelTitle", "i18n must include stage panel copy.");
assertContains(i18nSource, "timeline.evidence.target", "i18n must include timeline evidence copy.");
assertContains(i18nSource, "stage.previewMode.inspection", "i18n must include preview mode copy.");
assertContains(i18nSource, "当前步骤", "Chinese i18n must include stage panel copy.");
assertContains(i18nSource, "时间线证据", "Chinese i18n must include timeline evidence copy.");
assertContains(i18nSource, "检查视图", "Chinese i18n must include inspection mode copy.");
assertContains(packageJson, "scripts/test-workflow-focus-source.mjs", "test:scripts must include workflow focus source guard.");

console.log("PASS workflow focus source test");
