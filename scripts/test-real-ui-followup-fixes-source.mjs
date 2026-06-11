import { readFileSync } from "node:fs";

const appSource = readFileSync("apps/mac/src/App.tsx", "utf8");
const canvasPreviewSource = readFileSync("apps/mac/src/components/CanvasPreview.tsx", "utf8");
const exportPanelSource = readFileSync("apps/mac/src/components/ExportPanel.tsx", "utf8");
const forgeRouteSource = readFileSync("apps/mac/src/routes/ForgeRoute.tsx", "utf8");
const i18nSource = readFileSync("apps/mac/src/i18n.ts", "utf8");
const settingsRouteSource = readFileSync("apps/mac/src/routes/SettingsRoute.tsx", "utf8");
const cssSource = readFileSync("apps/mac/src/styles/app.css", "utf8");

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

assertContains(appSource, "isActive={activeRoute === \"forge\"}", "ForgeRoute must stay mounted while hidden so Settings/language changes do not reset workspace state.");
assertContains(appSource, "hidden={activeRoute !== \"forge\"}", "Hidden workbench route must preserve state without staying visible.");
assertContains(forgeRouteSource, "isActive", "ForgeRoute must receive route activity so global shortcuts do not fire while hidden.");
assertContains(forgeRouteSource, "if (!isActive || isEditableTarget(event.target))", "ForgeRoute keyboard shortcuts must be disabled while the workbench is hidden.");

assertContains(forgeRouteSource, "setPackName(summary.name)", "Imported .gsfpack manifest name must become the active pack name.");
assertContains(forgeRouteSource, "setActiveSourceName(fileName(packPath))", "Re-imported .gsfpack file name must stay as secondary source identity.");
assertContains(forgeRouteSource, "sourceName: metadataOverride.sourceName ?? activeSourceName", "Exports must keep source identity separate from pack display name.");

assertContains(appSource, "sortLocalPacks", "Local Pack Library scans and updates must use one stable sort function.");
assertContains(appSource, "selectedLibraryPack", "Inspecting a local pack must select details instead of collapsing the list.");
assertContains(appSource, "library-detail-panel", "Exports route must show a detail panel for the selected local pack.");
assertContains(i18nSource, "exports.library.detailTitle", "Local Pack Library detail panel must be localized.");

assertContains(forgeRouteSource, "APP_VERSION", "Footer version must come from a shared version constant.");
assertNotContains(forgeRouteSource, "<span>v1.2.0</span>", "Footer version must not be hardcoded to v1.2.0.");

assertContains(canvasPreviewSource, "stage.selectedFrameAlt", "Preview alt text must be generic or source-aware, not knight-specific.");
assertNotContains(canvasPreviewSource, "stage.selectedKnightAlt", "Canvas preview must not use knight-specific alt text for every asset.");
assertContains(i18nSource, "stage.selectedFrameAlt", "Generic selected-frame alt text must be localized.");

assertContains(exportPanelSource, "const advancedOpen", "Export metadata disclosure must control collapsed content visibility.");
assertContains(exportPanelSource, "hidden={!advancedOpen}", "Collapsed export metadata controls must be hidden from accessibility and keyboard navigation.");
assertContains(exportPanelSource, "aria-hidden={!advancedOpen}", "Collapsed export metadata body must be removed from the accessibility tree.");

assertNotContains(settingsRouteSource, "npm run install:mac", "Product Settings must not show developer install commands.");
assertContains(i18nSource, "settings.localRuntime.version", "Settings runtime card must show app version instead of developer sync instructions.");
assertContains(appSource, "const routeShowsWorkflow", "Header workflow tabs must be route-aware.");
assertContains(appSource, "routeShowsWorkflow ? t(\"app.qualityStatus\")", "Header status copy must be route-aware.");

assertNotContains(i18nSource, "IMPORT-ONLY MVP", "Visible English app subtitle must not expose internal MVP wording.");
assertNotContains(i18nSource, "仅导入 MVP", "Visible Chinese app subtitle must not expose internal MVP wording.");
assertContains(i18nSource, "Local Import Workbench", "English subtitle must describe the user-facing local workflow.");
assertContains(i18nSource, "本地导入工作台", "Chinese subtitle must describe the user-facing local workflow.");

assertContains(cssSource, ".exports-library-layout", "Exports route must use a full-width master/detail layout.");
assertContains(cssSource, ".library-detail-panel", "Exports route detail panel must be styled.");

console.log("PASS real UI follow-up fixes source test");
