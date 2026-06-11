import { existsSync, readFileSync } from "node:fs";

const appSource = readFileSync("apps/mac/src/App.tsx", "utf8");
const exportPanelSource = readFileSync("apps/mac/src/components/ExportPanel.tsx", "utf8");
const forgeRouteSource = readFileSync("apps/mac/src/routes/ForgeRoute.tsx", "utf8");
const canvasPreviewSource = readFileSync("apps/mac/src/components/CanvasPreview.tsx", "utf8");
const frameTimelineSource = readFileSync("apps/mac/src/components/FrameTimeline.tsx", "utf8");
const previewHookSource = readFileSync("apps/mac/src/hooks/usePreviewImage.ts", "utf8");
const settingsRouteSource = readFileSync("apps/mac/src/routes/SettingsRoute.tsx", "utf8");
const cssSource = readFileSync("apps/mac/src/styles/app.css", "utf8");
const i18nSource = readFileSync("apps/mac/src/i18n.ts", "utf8");
const packageSource = readFileSync("package.json", "utf8");
const tauriCommandSource = readFileSync("apps/mac/src-tauri/src/lib.rs", "utf8");
const tauriCommandClientSource = readFileSync("apps/mac/src/tauriCommands.ts", "utf8");
const tauriConfig = JSON.parse(readFileSync("apps/mac/src-tauri/tauri.conf.json", "utf8"));

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function assertContains(source, needle, message) {
  assert(source.includes(needle), message);
}

assert(!tauriConfig.app?.security?.assetProtocol, "Local preview rendering must not rely on the global Tauri asset protocol.");
assertContains(tauriCommandSource, "read_preview_image", "Tauri must expose a scoped preview-image reader for generated local PNGs.");
assertContains(tauriCommandSource, "ensure_preview_image_scope", "Preview-image reads must be constrained to Forge job/export paths.");
assertContains(tauriCommandClientSource, "readPreviewImage", "TypeScript must wrap the preview-image reader.");
assertContains(previewHookSource, "usePreviewImage", "Frontend previews must load generated images through the preview hook.");
assertContains(canvasPreviewSource, "usePreviewImage", "Canvas preview must render generated frames through data URLs.");
assertContains(frameTimelineSource, "usePreviewImage", "Timeline thumbnails must render generated frames through data URLs.");
assertContains(forgeRouteSource, "exportedSheetPreviewSrc", "Exported sprite-sheet preview must render through the preview hook.");

assertContains(appSource, "RecentExportActions", "Recent export cards must expose inspect, validate, re-import, and open actions.");
assertContains(appSource, "onInspectPack(record.output.packDir)", "Recent export Inspect must target the exported .gsfpack directory.");
assertContains(appSource, "onValidatePack(record.output.packDir)", "Recent export Validate must target the exported .gsfpack directory.");
assertContains(appSource, "onReimportPack(record.output.packDir)", "Recent export Re-import must target the exported .gsfpack directory.");

assertContains(forgeRouteSource, "hasQualityWarnings", "Quality status must distinguish warnings from all-passed checks.");
assertContains(forgeRouteSource, "quality.status.hasWarnings", "Quality warning status must render a translated label.");
assertContains(exportPanelSource, "export.readyWithWarnings", "Export readiness must acknowledge warnings instead of saying everything passed.");
assertContains(i18nSource, "可导出，但有建议", "Chinese quality/export copy must explain warning-but-exportable state.");

assertContains(forgeRouteSource, "WorkflowFocusPanel", "Sheet and Export workflow tabs must render mode-specific focus content.");
assertContains(i18nSource, "workflowFocus.sheet.title", "Sheet workflow focus copy must be localized.");
assertContains(i18nSource, "workflowFocus.export.title", "Export workflow focus copy must be localized.");

assertContains(settingsRouteSource, "settings-layout", "Settings route must use a wider two-column layout instead of a narrow single panel.");
assertContains(settingsRouteSource, "settings.localRuntime.title", "Settings route must show local runtime context.");

assertContains(exportPanelSource, "data-closed-label", "Export metadata disclosure must use localized show/hide labels.");
assert(!cssSource.includes('content: "Show"'), "Export metadata disclosure must not hardcode English Show in CSS.");
assert(!cssSource.includes('content: "Hide"'), "Export metadata disclosure must not hardcode English Hide in CSS.");
assertContains(cssSource, "content: attr(data-closed-label)", "Export metadata disclosure CSS must read the localized closed label.");
assertContains(cssSource, "content: attr(data-open-label)", "Export metadata disclosure CSS must read the localized open label.");

assertContains(cssSource, "overflow-wrap: anywhere", "Long run summary and export paths must wrap instead of silently truncating.");

assertContains(packageSource, "\"install:mac\"", "Root package must include a local install script to prevent stale /Applications builds.");
assert(existsSync("scripts/install-local-mac-app.sh"), "Local install helper must exist for syncing the latest app bundle to /Applications.");

console.log("PASS real UI issue fixes source test");
