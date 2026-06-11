import { readFileSync } from "node:fs";

const appSource = readFileSync("apps/mac/src/App.tsx", "utf8");
const appCssSource = readFileSync("apps/mac/src/styles/app.css", "utf8");
const canvasPreviewSource = readFileSync("apps/mac/src/components/CanvasPreview.tsx", "utf8");
const captureScreenshotScript = readFileSync("scripts/capture-forge-ui-screenshot.mjs", "utf8");
const exportPanelSource = readFileSync("apps/mac/src/components/ExportPanel.tsx", "utf8");
const forgeRouteSource = readFileSync("apps/mac/src/routes/ForgeRoute.tsx", "utf8");
const forgeViewModelSource = readFileSync("apps/mac/src/forgeViewModel.ts", "utf8");
const frameTimelineSource = readFileSync("apps/mac/src/components/FrameTimeline.tsx", "utf8");
const i18nSource = readFileSync("apps/mac/src/i18n.ts", "utf8");
const installLocalMacAppScript = readFileSync("scripts/install-local-mac-app.sh", "utf8");
const packageSource = readFileSync("package.json", "utf8");
const chromaPreviewPanelSource = readFileSync("apps/mac/src/components/ChromaPreviewPanel.tsx", "utf8");
const qualityInspectorSource = readFileSync("apps/mac/src/components/QualityInspector.tsx", "utf8");
const settingsRouteSource = readFileSync("apps/mac/src/routes/SettingsRoute.tsx", "utf8");
const signedDmgScript = readFileSync("scripts/build-signed-release-dmg.sh", "utf8");
const tauriInfoPlistSource = readFileSync("apps/mac/src-tauri/Info.plist", "utf8");
const tauriLibSource = readFileSync("apps/mac/src-tauri/src/lib.rs", "utf8");
const tauriConfigSource = readFileSync("apps/mac/src-tauri/tauri.conf.json", "utf8");
const videoSegmentPanelSource = readFileSync("apps/mac/src/components/VideoSegmentPanel.tsx", "utf8");
const viteConfigSource = readFileSync("apps/mac/vite.config.ts", "utf8");
const verifyReleasePackageScript = readFileSync("scripts/verify-release-package.sh", "utf8");

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

assertContains(settingsRouteSource, 'role="main"', "Settings route must expose a main landmark to macOS accessibility.");
assertContains(settingsRouteSource, 'aria-label={t("settings.title")}', "Settings route must label its main landmark.");
assertContains(settingsRouteSource, "routeRef.current?.focus()", "Settings route must move keyboard focus into the route when opened.");
assertContains(settingsRouteSource, "aria-label={label}", "Settings fields must expose explicit accessible names.");
assertContains(settingsRouteSource, "aria-label={chooseLabel}", "Settings choose buttons must expose explicit accessible names.");

assertContains(appSource, 'role="main"', "Exports route must expose a main landmark to macOS accessibility.");
assertContains(appSource, 'aria-label={t("exports.library.title")}', "Exports route must label its main landmark.");
assertContains(appSource, "exportsRouteRef.current?.focus()", "Exports route must move keyboard focus into the library when opened.");
assertContains(appSource, "exports.library.refreshAria", "Refresh Library must have a stable accessibility label.");
assertContains(appSource, "exports.library.inspectAria", "Pack Inspect buttons must have stable accessibility labels.");
assertContains(appSource, "exports.library.validateAria", "Pack Validate buttons must have stable accessibility labels.");
assertContains(appSource, "exports.library.reimportAria", "Pack Re-import buttons must have stable accessibility labels.");
assertContains(appSource, "exports.library.openAria", "Pack Open buttons must have stable accessibility labels.");

assertContains(forgeRouteSource, "const hasLiveFrames", "ForgeRoute must distinguish selected source from live frame workspace.");
assertContains(forgeRouteSource, "const hasSelectedSource", "ForgeRoute must track selected/imported sources separately.");
assertContains(forgeRouteSource, "const sourcePendingExtraction", "Imported video state must distinguish successful import from missing extracted frames.");
assertContains(forgeRouteSource, "const framesPendingQuality", "Extracted raw frames must have a dedicated pending-quality state.");
assertContains(forgeRouteSource, "summary.waitingForQuality", "Run summary must not describe extracted raw frames as export blocked.");
assertContains(forgeRouteSource, "qualityStatus: headerQualityStatus", "App header quality status must distinguish ready-to-check from generic ready.");
assertContains(forgeRouteSource, "source.awaitingFrames", "Imported videos must say they are waiting for frame extraction.");
assertContains(forgeRouteSource, "summary.waitingForFrames", "Run summary must say export is waiting for frames instead of blocked after video import.");
assertContains(forgeRouteSource, "status.framesSelectedSource", "Footer must not show sample 64 after a source has been selected.");
assertContains(forgeRouteSource, "placeholderMode={hasSelectedSource && !hasLiveFrames ? \"sourcePending\" : \"empty\"}", "Canvas must show an empty placeholder until a source or live frames exist.");
assertContains(forgeRouteSource, "source.emptyWorkspace", "No-source state must be labeled as an empty workspace, not a demo workspace.");
assertContains(forgeRouteSource, "status.framesEmpty", "Footer must not report sample frames in the default no-source state.");
assertContains(forgeRouteSource, "asset-empty-state", "Assets panel must render an empty state instead of sample assets by default.");
assertContains(forgeRouteSource, "state={hasLiveFrames ? \"live\" : hasSelectedSource ? \"sourcePending\" : \"empty\"}", "Timeline must distinguish empty, pending-source, and live-frame states.");
assertContains(forgeRouteSource, "!hasSelectedSource && activeWorkflow === \"Import\"", "No-source import screen must replace the passive canvas with an import launcher.");
assertContains(forgeRouteSource, "handleChooseVideoAndImport", "Central video CTA must choose and import a video in one action.");
assertContains(forgeRouteSource, "importLauncher.chooseVideo", "Central empty state must expose a direct Choose Video File CTA.");
assertContains(forgeRouteSource, 'useState(defaultPackName)', "Formal first-run metadata must not default to the bundled sample pack name.");
assertContains(forgeRouteSource, 'useState("idle")', "Formal first-run animation metadata must not default to the bundled sample walk animation.");
assertContains(forgeRouteSource, 'const bundledSamplePackName = "Green Box Character Pack"', "Bundled sample naming must be scoped to explicit sample actions.");
assertNotContains(forgeRouteSource, "demoAssets", "Default workbench must not render bundled sample assets as current assets.");
assertNotContains(forgeRouteSource, "demoTracks", "Default workbench must not render bundled sample tracks as the current timeline.");
assertNotContains(forgeRouteSource, "isDemo", "Timeline must not use a demo mode as the default workspace state.");

assertContains(forgeRouteSource, "summary.state.exportAwaitingValidation", "Run summary must distinguish exported from validated.");
assertContains(forgeRouteSource, "summary.validationPending", "Validate row must say validation is pending after export.");
assertContains(forgeRouteSource, "setPackSummary(null)", "Processing/exporting must clear stale validation summaries.");
assertContains(i18nSource, "已导出，等待验证", "Chinese copy must fit exported-but-not-validated state in the narrow run summary.");

assertNotContains(exportPanelSource, "advancedOpenDefault", "Advanced export metadata must not auto-open just because export becomes available.");
assertNotContains(exportPanelSource, "setAdvancedOpen(true)", "Advanced export metadata must only open from the user's disclosure action.");
assertContains(exportPanelSource, "export-panel-muted", "Export panel must show a muted pre-import state instead of full export controls.");
assertContains(exportPanelSource, "export.awaitingImportTitle", "Pre-import export panel must explain that export unlocks after import.");
assertContains(exportPanelSource, "sourcePendingExtraction", "Export panel must render a dedicated post-import, pre-extraction state.");
assertContains(exportPanelSource, "export.awaitingFramesTitle", "Export panel must point imported videos to Extract Frames before export.");
assertContains(exportPanelSource, "export.extractFirst", "Export panel must explain the next step without using blocked/error language.");
assertContains(exportPanelSource, "framesPendingQuality", "Export panel must render a friendly post-extraction, pre-quality state.");
assertContains(exportPanelSource, "export.pendingQualityTitle", "Export panel must say extracted frames are one quality step away from export.");
assertContains(exportPanelSource, "export.processQualityFirst", "Export panel must give a normal next-step instruction after extraction.");
assertContains(exportPanelSource, "export-location-card", "Export panel must make the output location visible.");
assertContains(exportPanelSource, "export.changeOutputFolder", "Export panel must offer an explicit output location change action.");
assertContains(exportPanelSource, "export-ready-priority", "Export panel must switch to an export-priority layout when export is available.");
assertContains(exportPanelSource, "export-key-fields", "Export-ready panel must keep pack and animation names near the primary export action.");
assertContains(exportPanelSource, "export-sticky-actions", "Export-ready panel must keep the primary export action visible.");
assertContains(exportPanelSource, "export-actions-result", "Export panel must switch to result actions after a pack has exported.");
assertContains(exportPanelSource, "export.reexportPack", "Export panel must keep re-export available after an export.");
assertContains(qualityInspectorSource, "quality-inspector-compact", "Quality report must support a compact mode after processing is complete.");
assertContains(qualityInspectorSource, "quality.showQualityDetails", "Compact quality report must let users expand details deliberately.");
assertContains(forgeRouteSource, "export-ready-inspector", "Right inspector must reserve more space for export after quality checks pass.");
assertContains(forgeRouteSource, "compact={Boolean(qualityReport && (canExport || exportOutput || activeWorkflow === \"Export\"))}", "ForgeRoute must compact quality details in export-ready states.");
assertContains(appCssSource, ".export-ready-priority .export-readiness-list", "Export-ready layout must hide long readiness lists from the first screen.");
assertContains(appCssSource, ".export-sticky-actions", "Export actions must have a sticky visible action area.");
assertContains(i18nSource, "显示质量详情", "Chinese compact quality report must expose a concise details action.");
assertContains(forgeRouteSource, "sourcePendingExtraction={sourcePendingExtraction}", "ForgeRoute must pass pending extraction state into ExportPanel.");
assertContains(forgeRouteSource, "framesPendingQuality={framesPendingQuality}", "ForgeRoute must pass pending quality state into ExportPanel.");
assertContains(forgeRouteSource, "onChooseOutputFolder={onChooseOutputFolder}", "ForgeRoute must pass the output-folder chooser into ExportPanel.");
assertContains(forgeRouteSource, "footer-export-status", "Footer must summarize exported state instead of duplicating right-panel actions.");
assertContains(forgeRouteSource, 't("footer.exportStatus")', "Footer export state must have a focused accessibility label.");
assertNotContains(forgeRouteSource, 't("export.openExportsFolder")', "Footer must not duplicate Open Export Folder after export.");
assertNotContains(forgeRouteSource, 't("export.validateReimport")', "Footer must not duplicate Validate Re-import after export.");
assertNotContains(forgeRouteSource, 't("export.reexportPack")', "Footer must not duplicate Re-export after export.");
assertContains(appCssSource, ".footer-export-status", "Footer export status chip must truncate cleanly instead of clipping buttons.");
assertContains(i18nSource, "底部导出状态", "Chinese footer export status accessibility copy must exist.");
assertContains(forgeRouteSource, "side-source-settings", "Side import controls must collapse into a change-source affordance after live data exists.");
assertContains(forgeRouteSource, "import.sideChangeSource", "Collapsed side import controls must use clear change-source copy.");
assertContains(appSource, "onChooseOutputFolder={handleChooseOutputFolder}", "App must wire the Forge workbench to the output-folder picker.");
assertContains(videoSegmentPanelSource, "reextract-settings", "Video segment controls must collapse into re-extract settings after frames exist.");
assertContains(videoSegmentPanelSource, "segment.reextractSettings", "Collapsed segment controls must use a clear re-extract label.");
assertContains(videoSegmentPanelSource, "segment-presets", "Video segment panel must offer a range preset control.");
assertContains(videoSegmentPanelSource, "segment.estimatedSelection", "Video segment panel must explain the expected extracted frame count.");
assertContains(videoSegmentPanelSource, "segment.extractEstimated", "Video segment primary CTA must include the expected frame count.");
assertContains(videoSegmentPanelSource, "segment-sticky-action", "Video segment primary CTA must stay visible in a sticky action area.");
assertContains(videoSegmentPanelSource, "segment.extractActionMeta", "Video segment sticky CTA must summarize the selected range and interval.");
assertContains(videoSegmentPanelSource, "segment-advanced-settings", "Loop settings must be moved behind an advanced disclosure.");
assertContains(forgeRouteSource, 'compact={activeWorkflow === "Frames" && hasSelectedSource && !hasLiveFrames}', "Left rail must use a compact run summary before frames are extracted.");
assertContains(forgeRouteSource, "summary.compactAwaitingFrames", "Compact run summary must use short pending-extraction copy.");
assertContains(i18nSource, "来源已选择 · 预计 {count} 帧 · 待抽取", "Chinese compact run summary must fit the narrow left rail.");
assertContains(i18nSource, "{start} - {end} · 每 {count} 帧", "Chinese sticky extract CTA meta must fit the narrow left rail.");
assertContains(canvasPreviewSource, "bboxLabel ?? placeholderTag", "Canvas preview must keep an explicit status tag slot.");
assertContains(chromaPreviewPanelSource, "framePreviewState", "Stage preview chip must distinguish raw frames from processed pixels.");
assertContains(chromaPreviewPanelSource, "stage.rawFramePixels", "Stage preview chip must label extracted frames as raw until processing runs.");
assertContains(frameTimelineSource, "timeline-selected-inline", "Timeline selected-frame readout must be inline instead of a bottom overlay.");
assertContains(qualityInspectorSource, "sourcePendingExtraction", "Quality inspector must distinguish pending extraction from already-imported frames.");
assertContains(qualityInspectorSource, "framesPendingQuality", "Quality inspector must distinguish extracted raw frames from generic imported frames.");
assertContains(qualityInspectorSource, "quality.processRawFrames", "Quality inspector must use raw-frame language after extraction.");
assertContains(qualityInspectorSource, "showQualityAction = !sourcePendingExtraction", "Quality inspector must not show a competing quality CTA before frames are extracted.");
assertContains(qualityInspectorSource, "quality.waitingForFramesTitle", "Quality inspector must label pre-extraction state as waiting for frames.");
assertContains(qualityInspectorSource, "quality.extractFirstNote", "Quality inspector must explain that quality checks appear after extraction.");
assertNotContains(qualityInspectorSource, "quality.extractProcessAction", "Quality inspector must not reintroduce the competing extract-and-quality CTA.");
assertNotContains(i18nSource, "抽帧并检查质量", "Chinese quality panel must not keep the competing pre-extraction CTA copy.");
assertContains(i18nSource, "先抽取帧创建实时工作区。帧可用后，这里会启用质量检查。", "Chinese quality panel must use waiting-for-frames guidance before extraction.");
assertContains(i18nSource, "下一步：抽取帧", "Chinese post-import export panel must guide the user to extract frames.");
assertContains(i18nSource, "等待抽帧", "Chinese run summary must not describe a normal post-import state as blocked.");
assertContains(i18nSource, "还差一步：处理并检查质量", "Chinese export panel must describe post-extraction export readiness as a normal next step.");
assertContains(i18nSource, "原始帧 {selected}/{count} · 背景未处理", "Chinese canvas tag must identify raw frames before background processing.");
assertContains(i18nSource, "处理原始帧以生成质量报告。", "Chinese quality panel must use raw-frame language after extraction.");
assertContains(i18nSource, "待检查", "Chinese quality status must avoid saying fully ready before the report exists.");
assertContains(i18nSource, "更改位置", "Chinese export panel must expose a concise change-location action.");
assertContains(i18nSource, "重新导出", "Chinese export panel must keep re-export legible after export.");
assertContains(i18nSource, "重新抽取设置", "Chinese segment panel must make re-extraction settings clear.");
assertContains(i18nSource, "更换来源", "Chinese left rail must make changing source clear after export.");
assertContains(i18nSource, "视频片段", "Chinese segment panel must use task-oriented video segment copy.");
assertContains(i18nSource, "前 1 秒", "Chinese segment panel must explain the default first-second preset.");
assertContains(i18nSource, "预计抽取 {count} 帧", "Chinese segment panel must show expected extraction count.");
assertContains(i18nSource, "高级设置", "Chinese loop controls must be clearly demoted behind advanced settings.");

assertNotContains(canvasPreviewSource, "hero-knight-crop.png", "Canvas placeholder must not use knight art for the Green Box sample story.");
assertNotContains(frameTimelineSource, "hero-knight-crop.png", "Timeline placeholder must not use knight thumbnails for the Green Box sample story.");
assertNotContains(forgeViewModelSource, "Hero Knight Pack", "Sample pack metadata must not keep the old knight story.");
assertContains(forgeViewModelSource, "Green Box Character Pack", "Sample pack metadata must match the Green Box sample story.");
assertContains(canvasPreviewSource, "empty-workspace-visual", "Canvas must render a neutral empty placeholder before the user chooses a source.");
assertNotContains(canvasPreviewSource, "green-box-placeholder", "Canvas must not show bundled sample art before real frames exist.");
assertContains(frameTimelineSource, "green-box-thumb-placeholder", "Timeline must render Green Box placeholder thumbnails without frame paths.");
assertContains(frameTimelineSource, "timeline-empty-state", "Timeline must render a no-frames empty state before import.");

assertContains(forgeRouteSource, "latestRecentExport", "Workbench must receive recent export context for restart recovery.");
assertContains(forgeRouteSource, "firstRun.reimportRecent", "First-run rail must offer a recent export re-import entry.");
assertContains(appSource, "latestRecentExport={recentExports[0] ?? null}", "App must pass the latest recent export to the workbench.");
assertContains(viteConfigSource, 'base: "./"', "Release builds must use relative asset URLs so Tauri embedded WebView can load JS/CSS.");
assertContains(viteConfigSource, "forge-tauri-release-html", "Vite must normalize release HTML for the Tauri WebView.");
assertContains(viteConfigSource, "crossorigin", "Vite release HTML normalization must remove crossorigin from local Tauri assets.");
assertContains(viteConfigSource, "closeBundle", "Vite release HTML normalization must patch the final HTML written to disk.");
assertContains(viteConfigSource, "writeFileSync(indexUrl, html)", "Release HTML must be written back after inlining local assets.");
assertContains(viteConfigSource, "inlineScripts", "Release JS must be inlined after #root so WKWebView does not skip external local scripts.");
assertContains(tauriConfigSource, "http://tauri.localhost", "Tauri release CSP must allow bundled frontend assets served from tauri.localhost.");
assertContains(tauriConfigSource, "tauri://localhost", "Tauri release CSP must explicitly allow the local Tauri app protocol.");
assertContains(tauriConfigSource, "script-src 'self' 'unsafe-inline'", "Inline release HTML must be permitted by the local-only CSP.");
assertContains(tauriConfigSource, "customprotocol:", "Tauri release CSP must allow the custom frontend protocol.");
assertContains(tauriConfigSource, "connect-src 'self' ipc:", "Tauri release CSP must allow Tauri IPC without broadening all default sources.");
assertNotContains(tauriConfigSource, "useHttpsScheme", "macOS release window must not keep the experimental HTTPS scheme toggle.");
assertContains(tauriConfigSource, '"hardenedRuntime": true', "Developer ID bundles must keep hardened runtime enabled for distribution validation.");
assertContains(tauriConfigSource, '"entitlements": "Entitlements.plist"', "Developer ID bundles must include WebKit-compatible runtime entitlements.");
assertContains(tauriConfigSource, '"infoPlist": "Info.plist"', "macOS bundles must merge the Cocoa startup Info.plist overrides.");
assertContains(tauriInfoPlistSource, "<key>LSRequiresCarbon</key>", "macOS Info.plist override must control the legacy Carbon launch key.");
assertContains(tauriInfoPlistSource, "<false/>", "macOS Info.plist override must disable the legacy Carbon launch key.");
assertContains(tauriInfoPlistSource, "<key>NSPrincipalClass</key>", "macOS Info.plist override must declare the Cocoa app principal class.");
assertContains(tauriInfoPlistSource, "<string>NSApplication</string>", "macOS Info.plist override must use NSApplication for Cocoa/WebKit startup.");
assertNotContains(tauriLibSource, "FORGE_BOOT_DIAGNOSTIC", "Temporary boot diagnostics must not ship in production source.");
assertNotContains(tauriLibSource, "FORGE_DOM_DIAGNOSTIC", "Temporary DOM diagnostics must not ship in production source.");
assertNotContains(tauriLibSource, "FORGE_BUNDLE_DIAGNOSTIC", "Temporary bundle diagnostics must not ship in production source.");
assertNotContains(forgeRouteSource, "sprite-sheet-crop.png", "Sample sheet preview must not use knight artwork in the Green Box sample story.");

assertContains(captureScreenshotScript, "FORGE_CAPTURE_OWNER", "Screenshot capture must support an explicit app owner.");
assertContains(captureScreenshotScript, "owner == target_owner", "Screenshot capture must match the Forge window owner exactly.");
assertNotContains(captureScreenshotScript, "Game Sprite Forge' in title", "Screenshot capture must not match stale Terminal titles.");
assertNotContains(captureScreenshotScript, '["-x", outputPath]', "Screenshot capture must not silently fall back to full-screen captures.");

assertNotContains(packageSource, "patch-macos-launcher", "Package scripts must not reference the failed release launcher workaround.");
assertNotContains(installLocalMacAppScript, "patch-macos-launcher", "Local install must copy the app bundle without injecting a launcher workaround.");
assertNotContains(installLocalMacAppScript, "Game Sprite Forge Real", "Local install must not create a renamed real executable.");
assertNotContains(signedDmgScript, "patch-macos-launcher", "Signed DMG build must not inject a launcher workaround.");
assertNotContains(signedDmgScript, "Game Sprite Forge Real", "Signed DMG build must sign the canonical app executable.");
assertNotContains(verifyReleasePackageScript, "Game Sprite Forge Real", "Release verification must look for the canonical app executable.");

console.log("PASS real UI full QA fixes source test");
