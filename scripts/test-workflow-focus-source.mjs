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
assertContains(routeSource, "actualFrameCount: timelineFramePaths.length", "Route must pass actual rendered frame count into timeline evidence.");

assertContains(canvasPreviewSource, "type PreviewMode", "CanvasPreview must type preview mode.");
assertContains(canvasPreviewSource, "preview-mode-label", "CanvasPreview must label the active preview mode.");
assertContains(canvasPreviewSource, "inspectionEnabled", "CanvasPreview must support inspection overlay toggling.");
assertContains(canvasPreviewSource, "aria-pressed={inspectionEnabled}", "Inspection toggle must expose pressed state.");
assertContains(routeSource, "previewInspectionEnabled", "ForgeRoute must own the inspection overlay toggle state.");
assertContains(routeSource, "overlay={selectedOverlay}", "ForgeRoute must pass selected overlay only through preview state.");
assertContains(routeSource, "previewMode={previewMode}", "ForgeRoute must pass preview mode to CanvasPreview.");

assertContains(cssSource, ".stage-action-panel", "CSS must style the stage action panel.");
assertContains(cssSource, ".stage-primary-action", "CSS must style the single stage primary action.");
assertContains(cssSource, ".timeline-evidence-strip", "CSS must style timeline evidence.");
assertContains(cssSource, ".preview-mode-control", "CSS must style preview mode control.");
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
