import { existsSync, readFileSync } from "node:fs";

const appSource = readFileSync("apps/mac/src/App.tsx", "utf8");
const importPanelSource = readFileSync("apps/mac/src/components/ImportPanel.tsx", "utf8");
const forgeRouteSource = readFileSync("apps/mac/src/routes/ForgeRoute.tsx", "utf8");
const exportPanelSource = readFileSync("apps/mac/src/components/ExportPanel.tsx", "utf8");
const i18nSource = readFileSync("apps/mac/src/i18n.ts", "utf8");
const smokeSource = readFileSync("apps/mac/scripts/smoke-ui.mjs", "utf8");
const backlogSource = readFileSync("docs/architecture/post-release-backlog.md", "utf8");

function assertContains(source, needle, message) {
  if (!source.includes(needle)) {
    throw new Error(message);
  }
}

assertContains(appSource, "type LibraryAction", "Local Pack Library must track the active action.");
assertContains(appSource, "libraryStatus", "Local Pack Library must expose status text.");
assertContains(appSource, 'role="status"', "Local Pack Library status must be announced as status text.");
assertContains(i18nSource, "Scanning local export folder", "Refresh Library must show scanning feedback.");
assertContains(appSource, "exports.library.scanning", "Refresh Library must render translated scanning feedback.");
assertContains(i18nSource, "Found {count} local", "Refresh Library must show how many packs were found.");
assertContains(appSource, "exports.library.found", "Refresh Library must render translated found-count feedback.");
assertContains(i18nSource, "Pack validation passed", "Validate Pack must show success feedback.");
assertContains(appSource, "exports.library.validationPassed", "Validate Pack must render translated success feedback.");
assertContains(i18nSource, "Opening local pack in Forge", "Re-import Pack must tell the user it is switching routes.");
assertContains(appSource, "exports.library.reimportStatus", "Re-import Pack must render translated route-switch feedback.");
assertContains(appSource, 'libraryAction === "refresh"', "Refresh Library button must expose busy state.");
assertContains(appSource, "formatLibraryError", "Local Pack Library errors must be normalized before display.");

assertContains(importPanelSource, "sample-action-hint", "Load Sample Path must have a visible hint.");
assertContains(i18nSource, "Fills the video path only", "Sample hint must explain that Load Sample Path is not the full pipeline.");
assertContains(importPanelSource, "import.sampleHint", "ImportPanel must render the translated sample hint.");
assertContains(i18nSource, "Run Sample Pipeline", "First Run must keep the full sample pipeline action.");
assertContains(forgeRouteSource, "firstRun.runSample", "First Run must render the translated full sample pipeline action.");
assertContains(i18nSource, "Full pipeline passed", "Sample pipeline completion must remain explicit.");
assertContains(forgeRouteSource, "status.fullPipelinePassed", "Sample pipeline completion must render the translated success message.");

assertContains(exportPanelSource, "validation-result", "Export panel must expose validation result copy.");
assertContains(i18nSource, "Last validated", "Validation result must be easy to scan after re-import.");
assertContains(exportPanelSource, "export.lastValidated", "Export panel must render the translated validation result.");

assertContains(smokeSource, "library-status", "UI smoke must assert Local Pack Library status text.");
assertContains(smokeSource, "sample-action-hint", "UI smoke must assert the sample path hint.");
assertContains(smokeSource, "validation-result", "UI smoke must assert validation result output.");

assertContains(backlogSource, "Local Workbench UX Hardening", "Backlog must point at the UI-first next slice.");
assertContains(backlogSource, "CLI/MCP deferred", "Backlog must keep CLI/MCP out of the active product slice.");

if (!existsSync("docs/qa/local-workbench-ux-evidence-2026-06-06.md")) {
  throw new Error("Computer Use evidence must be recorded in docs/qa/local-workbench-ux-evidence-2026-06-06.md");
}

console.log("PASS local workbench UX source test");
