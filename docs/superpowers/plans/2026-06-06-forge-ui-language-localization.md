# Forge UI Language Localization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a lightweight bilingual UI language system so Forge follows the macOS/browser language by default while allowing users to override the app language in Settings.

**Architecture:** Use a small local TypeScript i18n dictionary and hook instead of a heavyweight localization framework. Store the user's language mode in the existing `LocalSettings` object, resolve `auto` from `navigator.languages`, and pass a typed translation function through the app shell and high-impact workflow components. Keep product scope local-first: no cloud translation, website localization, marketplace, MCP product surface, or online account work.

**Tech Stack:** Tauri 2, React 18, TypeScript, Vite, existing `localStorage` settings, source-guard scripts, Vite smoke UI, Computer Use MCP for installed-app real UI verification.

---

## Product Decision

Ship this language model:

```text
Language mode:
- Automatic
- English
- 简体中文

Automatic behavior:
- If navigator.languages contains zh-CN, zh-Hans, zh, zh-SG, zh-MY, or zh-HK, show Simplified Chinese.
- Otherwise show English.

Manual override:
- Settings -> Language overrides Automatic.
- The selected mode persists in game-sprite-forge.local-settings.v1.
```

Why this approach:

```text
Follow system language by default: matches macOS user expectations.
Settings override: supports bilingual users, screenshots, QA, and support.
Small local dictionary: fits the current desktop MVP and avoids heavy framework churn.
```

## UX Writing Constraints

Audience:

```text
Solo game developers, 2D artists, technical artists, and small teams.
They are technical enough to understand frames, sprite sheets, ffmpeg, and .gsfpack, but they are often in a focused production state and need unambiguous next actions.
```

Tone:

```text
Dense, calm, operational.
No marketing language.
No cute error copy.
No AI/cloud/marketplace wording in the MVP.
```

Terminology:

| English | 简体中文 |
| --- | --- |
| Local Workbench | 本地工作台 |
| Forge | 工作台 |
| Exports | 导出 |
| Settings | 设置 |
| Source | 来源 |
| Frames | 帧 |
| Sprite sheet | 精灵表 |
| Pack | 资源包 |
| `.gsfpack` pack | `.gsfpack` 资源包 |
| Run Sample Pipeline | 运行示例流程 |
| Load Sample Path | 填入示例路径 |
| Process & Quality | 处理并检查质量 |
| Export Pack | 导出资源包 |
| Validate Re-import | 验证重新导入 |
| Quality Report | 质量报告 |
| Game Ready | 可用于游戏 |
| Missing before export | 导出前还缺少 |

Non-translated terms:

```text
ffmpeg
ffprobe
Godot
PNG
JSON
MVP
.gsfpack
atlas.json
manifest.json
quality-report.json
preview.gif
```

## File Structure

- Create: `apps/mac/src/i18n.ts` - typed locale modes, locale detection, dictionary, and translation helpers.
- Modify: `apps/mac/src/tauriCommands.ts` - add `languageMode` to `LocalSettings`.
- Modify: `apps/mac/src/App.tsx` - add default setting, resolve locale, pass `t`, localize app shell, nav, workflow tabs, Exports route, and status strings.
- Modify: `apps/mac/src/routes/SettingsRoute.tsx` - add Settings language selector and localize settings copy.
- Modify: `apps/mac/src/components/ImportPanel.tsx` - localize source labels and sample hint.
- Modify: `apps/mac/src/components/ExportPanel.tsx` - localize export blockers, settings labels, export details, and validation result.
- Modify: `apps/mac/src/components/QualityInspector.tsx` - localize quality report labels, empty state, actions, and verdict display strings.
- Modify: `apps/mac/src/routes/ForgeRoute.tsx` - localize high-impact workflow copy, run summary labels, recovery cards, pipeline actions, and statuses.
- Modify: `apps/mac/scripts/smoke-ui.mjs` - allow locale-specific smoke assertions for English and Simplified Chinese.
- Create: `scripts/test-ui-language-source.mjs` - source guard for language mode, dictionary coverage, Settings selector, smoke locale assertions, and no product MCP/CLI surface.
- Modify: `package.json` - add the source guard to `test:scripts`.
- Create: `docs/qa/ui-language-evidence-2026-06-06.md` - shell and Computer Use evidence for English, Chinese, Automatic, and Settings override.
- Modify: `docs/architecture/mvp-scope.md` - record language support and verification after implementation.
- Modify: `docs/architecture/post-release-backlog.md` - mark UI language localization as active/completed once verified.

## Verification Commands

Run before marking the full plan complete:

```bash
npm run test:scripts
npm --workspace apps/mac run build
FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:mvp
FORGE_SMOKE_LOCALE=zh-CN npm --workspace apps/mac run smoke:ui:mvp
FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:responsive
FORGE_SMOKE_LOCALE=zh-CN npm --workspace apps/mac run smoke:ui:responsive
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml
```

Expected final result:

```text
script source guards: pass
frontend build: pass
English MVP UI smoke: pass
Chinese MVP UI smoke: pass
English responsive UI smoke: pass
Chinese responsive UI smoke: pass
Rust tests: pass
Computer Use evidence: recorded for Automatic, English override, Simplified Chinese override, Run Sample Pipeline, Exports, Settings, and no CLI/MCP product surface
```

---

### Task 1: Add UI Language Source Guard

**Files:**
- Create: `scripts/test-ui-language-source.mjs`
- Modify: `package.json`

- [ ] **Step 1: Create the source guard**

Create `scripts/test-ui-language-source.mjs`:

```js
import { existsSync, readFileSync } from "node:fs";

const appSource = readFileSync("apps/mac/src/App.tsx", "utf8");
const i18nSource = existsSync("apps/mac/src/i18n.ts")
  ? readFileSync("apps/mac/src/i18n.ts", "utf8")
  : "";
const settingsSource = readFileSync("apps/mac/src/routes/SettingsRoute.tsx", "utf8");
const commandsSource = readFileSync("apps/mac/src/tauriCommands.ts", "utf8");
const smokeSource = readFileSync("apps/mac/scripts/smoke-ui.mjs", "utf8");
const packageSource = readFileSync("package.json", "utf8");

function assertContains(source, needle, message) {
  if (!source.includes(needle)) {
    throw new Error(message);
  }
}

assertContains(i18nSource, "export type LanguageMode", "i18n must expose a persisted language mode type.");
assertContains(i18nSource, "\"auto\"", "Language mode must support Automatic.");
assertContains(i18nSource, "\"en-US\"", "Language mode must support English.");
assertContains(i18nSource, "\"zh-CN\"", "Language mode must support Simplified Chinese.");
assertContains(i18nSource, "resolveAppLocale", "i18n must resolve Automatic from system/browser language.");
assertContains(i18nSource, "createTranslator", "i18n must expose a typed translator.");
assertContains(i18nSource, "本地工作台", "Simplified Chinese dictionary must include app shell copy.");
assertContains(i18nSource, "运行示例流程", "Simplified Chinese dictionary must include the sample pipeline action.");
assertContains(i18nSource, "导出资源包", "Simplified Chinese dictionary must include export action copy.");

assertContains(commandsSource, "languageMode", "LocalSettings must persist languageMode.");
assertContains(appSource, "languageMode: \"auto\"", "Default settings must use Automatic language mode.");
assertContains(appSource, "resolveAppLocale(settings.languageMode", "App must resolve locale from settings.");
assertContains(appSource, "createTranslator(locale)", "App must create a translator from resolved locale.");
assertContains(appSource, "\"app.nav.forge\"", "App shell nav must use translation keys.");
assertContains(appSource, "\"workflow.import\"", "Workflow tabs must use translation keys.");

assertContains(settingsSource, "languageMode", "Settings route must edit languageMode.");
assertContains(settingsSource, "settings-language-select", "Settings route must render a language selector.");
assertContains(settingsSource, "settings.language.auto", "Settings language selector must render the Automatic translation key.");
assertContains(i18nSource, "Automatic", "English dictionary must expose Automatic.");
assertContains(i18nSource, "跟随系统", "Chinese dictionary must expose Automatic as 跟随系统.");

assertContains(smokeSource, "FORGE_SMOKE_LOCALE", "UI smoke must support locale-specific assertions.");
assertContains(smokeSource, "运行示例流程", "UI smoke must assert Chinese core copy.");
assertContains(smokeSource, "Run Sample Pipeline", "UI smoke must keep English core copy assertions.");

assertContains(packageSource, "scripts/test-ui-language-source.mjs", "test:scripts must include the UI language source guard.");

if (!existsSync("docs/qa/ui-language-evidence-2026-06-06.md")) {
  throw new Error("Computer Use evidence must be recorded in docs/qa/ui-language-evidence-2026-06-06.md");
}

console.log("PASS UI language source test");
```

- [ ] **Step 2: Add the guard to `package.json`**

In `package.json`, insert these checks after `scripts/test-local-workbench-ux-source.mjs`:

```json
"node --check scripts/test-ui-language-source.mjs && node scripts/test-ui-language-source.mjs"
```

The `test:scripts` command should contain both the check and execution forms:

```text
node --check scripts/test-ui-language-source.mjs
node scripts/test-ui-language-source.mjs
```

- [ ] **Step 3: Run the guard and confirm it fails before implementation**

Run:

```bash
npm run test:scripts
```

Expected:

```text
FAIL with i18n must expose a persisted language mode type.
```

Do not weaken the source guard. Implement the missing language system in later tasks.

---

### Task 2: Add Lightweight i18n Engine

**Files:**
- Create: `apps/mac/src/i18n.ts`

- [ ] **Step 1: Create `apps/mac/src/i18n.ts`**

Create this file:

```ts
export type LanguageMode = "auto" | "en-US" | "zh-CN";
export type AppLocale = Exclude<LanguageMode, "auto">;

export type TranslationKey =
  | "app.name"
  | "app.nav.forge"
  | "app.nav.exports"
  | "app.nav.settings"
  | "app.workbench.title"
  | "app.workbench.subtitle"
  | "app.qualityStatus"
  | "app.status.pending"
  | "app.status.ready"
  | "app.status.openForge"
  | "workflow.import"
  | "workflow.frames"
  | "workflow.background"
  | "workflow.anchor"
  | "workflow.sheet"
  | "workflow.export"
  | "workflow.locked"
  | "exports.library.title"
  | "exports.library.subtitle"
  | "exports.library.refresh"
  | "exports.library.refreshing"
  | "exports.library.initialStatus"
  | "exports.library.scanning"
  | "exports.library.found"
  | "exports.library.inspect"
  | "exports.library.inspecting"
  | "exports.library.inspected"
  | "exports.library.validate"
  | "exports.library.validating"
  | "exports.library.validationPassed"
  | "exports.library.reimport"
  | "exports.library.reimporting"
  | "exports.library.open"
  | "exports.library.opening"
  | "exports.library.opened"
  | "exports.empty.title"
  | "exports.empty.detail"
  | "exports.openFolder"
  | "settings.title"
  | "settings.subtitle"
  | "settings.language.label"
  | "settings.language.auto"
  | "settings.language.english"
  | "settings.language.chinese"
  | "settings.ffmpegPath"
  | "settings.ffprobePath"
  | "settings.defaultOutputFolder"
  | "settings.defaultFps"
  | "settings.defaultSheetSize"
  | "settings.chooseFfmpeg"
  | "settings.chooseFfprobe"
  | "settings.chooseFolder"
  | "settings.note"
  | "import.sources"
  | "import.loadSamplePath"
  | "import.sampleHint"
  | "import.video"
  | "import.pngSequence"
  | "import.spriteSheet"
  | "import.gsfpack"
  | "import.chooseFile"
  | "import.chooseFolder"
  | "import.selectSource"
  | "import.importSelected"
  | "firstRun.title"
  | "firstRun.detail"
  | "firstRun.checkToolchain"
  | "firstRun.selectSource"
  | "firstRun.processFrames"
  | "firstRun.exportPack"
  | "firstRun.runSample"
  | "quality.title"
  | "quality.waiting"
  | "quality.noReport"
  | "quality.noReportDetail"
  | "quality.checksAfterProcessing"
  | "quality.processImported"
  | "quality.runSample"
  | "quality.processFromReport"
  | "quality.verdict.gameReady"
  | "quality.verdict.needsCleanup"
  | "quality.verdict.prototypeUsable"
  | "quality.verdict.blocked"
  | "export.generatedOutputs"
  | "export.ready"
  | "export.readyDetail"
  | "export.missingBeforeExport"
  | "export.readiness"
  | "export.packName"
  | "export.animationName"
  | "export.metadata"
  | "export.metadata.show"
  | "export.metadata.hide"
  | "export.creator"
  | "export.license"
  | "export.sheetColumns"
  | "export.sheetPadding"
  | "export.sheetMargin"
  | "export.splitSheets"
  | "export.loop"
  | "export.output"
  | "export.exportPack"
  | "export.validateReimport"
  | "export.noExport"
  | "export.openExportsFolder"
  | "export.outputDetails"
  | "export.outputPack"
  | "export.outputFrames"
  | "export.outputSpriteSheets"
  | "export.outputGodotHelper"
  | "export.lastValidated"
  | "pipeline.checkFfmpeg"
  | "pipeline.extractFrames"
  | "pipeline.processQuality";

type TranslationParams = Record<string, string | number>;
type TranslationDictionary = Record<TranslationKey, string>;

const enUS: TranslationDictionary = {
  "app.name": "Game Sprite Forge",
  "app.nav.forge": "Forge",
  "app.nav.exports": "Exports",
  "app.nav.settings": "Settings",
  "app.workbench.title": "Local Workbench",
  "app.workbench.subtitle": "IMPORT-ONLY MVP",
  "app.qualityStatus": "Quality status",
  "app.status.pending": "Pending",
  "app.status.ready": "Ready",
  "app.status.openForge": "Open Forge",
  "workflow.import": "Import",
  "workflow.frames": "Frames",
  "workflow.background": "Background",
  "workflow.anchor": "Anchor",
  "workflow.sheet": "Sheet",
  "workflow.export": "Export",
  "workflow.locked": "Workflow step locked - complete previous step first.",
  "exports.library.title": "Local Pack Library",
  "exports.library.subtitle": "Validate, inspect, open, and re-import local .gsfpack exports",
  "exports.library.refresh": "Refresh Library",
  "exports.library.refreshing": "Refreshing...",
  "exports.library.initialStatus": "Refresh the library to scan the default export folder.",
  "exports.library.scanning": "Scanning local export folder...",
  "exports.library.found": "Found {count} local {packWord}.",
  "exports.library.inspect": "Inspect",
  "exports.library.inspecting": "Inspecting...",
  "exports.library.inspected": "Inspected {name}.",
  "exports.library.validate": "Validate Pack",
  "exports.library.validating": "Validating...",
  "exports.library.validationPassed": "Pack validation passed.",
  "exports.library.reimport": "Re-import Pack",
  "exports.library.reimporting": "Opening...",
  "exports.library.open": "Open",
  "exports.library.opening": "Opening {name}...",
  "exports.library.opened": "Opened {name}.",
  "exports.empty.title": "No local packs found",
  "exports.empty.detail": "Export a pack or refresh the default export folder.",
  "exports.openFolder": "Open Folder",
  "settings.title": "Local Toolchain",
  "settings.subtitle": "Required for import and export jobs",
  "settings.language.label": "Language",
  "settings.language.auto": "Automatic",
  "settings.language.english": "English",
  "settings.language.chinese": "简体中文",
  "settings.ffmpegPath": "ffmpeg path",
  "settings.ffprobePath": "ffprobe path",
  "settings.defaultOutputFolder": "default output folder",
  "settings.defaultFps": "default FPS",
  "settings.defaultSheetSize": "default sheet size",
  "settings.chooseFfmpeg": "Choose ffmpeg",
  "settings.chooseFfprobe": "Choose ffprobe",
  "settings.chooseFolder": "Choose Folder",
  "settings.note": "Settings saved locally. Empty ffmpeg fields fall back to the system PATH.",
  "import.sources": "Import Sources",
  "import.loadSamplePath": "Load Sample Path",
  "import.sampleHint": "Fills the video path only. Run Sample Pipeline processes and validates the bundled sample.",
  "import.video": "Import Video - mov, mp4, webm",
  "import.pngSequence": "Import PNG Sequence - newline or comma separated PNGs",
  "import.spriteSheet": "Import Sprite Sheet - grid or atlas",
  "import.gsfpack": "Import .gsfpack - local pack",
  "import.chooseFile": "Choose File",
  "import.chooseFolder": "Choose Folder",
  "import.selectSource": "Select Source",
  "import.importSelected": "Import Selected",
  "firstRun.title": "First Run",
  "firstRun.detail": "Run the bundled sample end to end, or import your own local source below.",
  "firstRun.checkToolchain": "Check toolchain",
  "firstRun.selectSource": "Select source",
  "firstRun.processFrames": "Process frames",
  "firstRun.exportPack": "Export pack",
  "firstRun.runSample": "Run Sample Pipeline",
  "quality.title": "Quality Report",
  "quality.waiting": "Waiting for processing",
  "quality.noReport": "No quality report yet",
  "quality.noReportDetail": "Use a live source or run the bundled sample pipeline.",
  "quality.checksAfterProcessing": "Checks after processing",
  "quality.processImported": "Process imported frames to unlock export checks.",
  "quality.runSample": "Run sample pipeline",
  "quality.processFromReport": "Process frames from Quality Report",
  "quality.verdict.gameReady": "Game Ready",
  "quality.verdict.needsCleanup": "Needs Cleanup",
  "quality.verdict.prototypeUsable": "Prototype Usable",
  "quality.verdict.blocked": "Blocked",
  "export.generatedOutputs": "Generated Outputs",
  "export.ready": "Export is ready",
  "export.readyDetail": "Processed frames and a quality report are available.",
  "export.missingBeforeExport": "Missing before export ({count})",
  "export.readiness": "Export readiness",
  "export.packName": "Pack Name",
  "export.animationName": "Animation Name",
  "export.metadata": "Pack metadata and sheet settings",
  "export.metadata.show": "Show",
  "export.metadata.hide": "Hide",
  "export.creator": "Creator",
  "export.license": "License",
  "export.sheetColumns": "Sheet Columns",
  "export.sheetPadding": "Sheet Padding",
  "export.sheetMargin": "Sheet Margin",
  "export.splitSheets": "Split sheets when needed",
  "export.loop": "Loop export",
  "export.output": "Output .gsfpack + generic files",
  "export.exportPack": "Export Pack",
  "export.validateReimport": "Validate Re-import",
  "export.noExport": "No export yet",
  "export.openExportsFolder": "Open Exports Folder",
  "export.outputDetails": "Export output details",
  "export.outputPack": "Pack: {path}",
  "export.outputFrames": "Frames: {count}",
  "export.outputSpriteSheets": "Sprite sheet pages: {count}",
  "export.outputGodotHelper": "Godot helper: {path}",
  "export.lastValidated": "Last validated: {name} with {count} frames.",
  "pipeline.checkFfmpeg": "Check FFmpeg",
  "pipeline.extractFrames": "Extract Frames",
  "pipeline.processQuality": "Process & Quality",
};

const zhCN: TranslationDictionary = {
  ...enUS,
  "app.nav.forge": "工作台",
  "app.nav.exports": "导出",
  "app.nav.settings": "设置",
  "app.workbench.title": "本地工作台",
  "app.workbench.subtitle": "仅导入 MVP",
  "app.qualityStatus": "质量状态",
  "app.status.pending": "待处理",
  "app.status.ready": "就绪",
  "app.status.openForge": "打开工作台",
  "workflow.import": "导入",
  "workflow.frames": "帧",
  "workflow.background": "背景",
  "workflow.anchor": "锚点",
  "workflow.sheet": "精灵表",
  "workflow.export": "导出",
  "workflow.locked": "工作流步骤已锁定，请先完成前一步。",
  "exports.library.title": "本地资源包库",
  "exports.library.subtitle": "验证、检查、打开并重新导入本地 .gsfpack 资源包",
  "exports.library.refresh": "刷新资源包库",
  "exports.library.refreshing": "正在刷新...",
  "exports.library.initialStatus": "刷新资源包库以扫描默认导出文件夹。",
  "exports.library.scanning": "正在扫描本地导出文件夹...",
  "exports.library.found": "找到 {count} 个本地资源包。",
  "exports.library.inspect": "检查",
  "exports.library.inspecting": "正在检查...",
  "exports.library.inspected": "已检查 {name}。",
  "exports.library.validate": "验证资源包",
  "exports.library.validating": "正在验证...",
  "exports.library.validationPassed": "资源包验证通过。",
  "exports.library.reimport": "重新导入资源包",
  "exports.library.reimporting": "正在打开...",
  "exports.library.open": "打开",
  "exports.library.opening": "正在打开 {name}...",
  "exports.library.opened": "已打开 {name}。",
  "exports.empty.title": "没有找到本地资源包",
  "exports.empty.detail": "导出一个资源包，或刷新默认导出文件夹。",
  "exports.openFolder": "打开文件夹",
  "settings.title": "本地工具链",
  "settings.subtitle": "导入和导出任务需要这些工具",
  "settings.language.label": "语言",
  "settings.language.auto": "跟随系统",
  "settings.language.english": "English",
  "settings.language.chinese": "简体中文",
  "settings.ffmpegPath": "ffmpeg 路径",
  "settings.ffprobePath": "ffprobe 路径",
  "settings.defaultOutputFolder": "默认导出文件夹",
  "settings.defaultFps": "默认 FPS",
  "settings.defaultSheetSize": "默认精灵表尺寸",
  "settings.chooseFfmpeg": "选择 ffmpeg",
  "settings.chooseFfprobe": "选择 ffprobe",
  "settings.chooseFolder": "选择文件夹",
  "settings.note": "设置已保存到本机。ffmpeg 字段留空时会回退到系统 PATH。",
  "import.sources": "导入来源",
  "import.loadSamplePath": "填入示例路径",
  "import.sampleHint": "只填入视频路径。运行示例流程会处理并验证内置示例。",
  "import.video": "导入视频 - mov、mp4、webm",
  "import.pngSequence": "导入 PNG 序列 - 换行或逗号分隔的 PNG",
  "import.spriteSheet": "导入精灵表 - 网格或图集",
  "import.gsfpack": "导入 .gsfpack - 本地资源包",
  "import.chooseFile": "选择文件",
  "import.chooseFolder": "选择文件夹",
  "import.selectSource": "选择来源",
  "import.importSelected": "导入所选",
  "firstRun.title": "首次使用",
  "firstRun.detail": "运行内置示例的完整流程，或在下方导入你自己的本地来源。",
  "firstRun.checkToolchain": "检查工具链",
  "firstRun.selectSource": "选择来源",
  "firstRun.processFrames": "处理帧",
  "firstRun.exportPack": "导出资源包",
  "firstRun.runSample": "运行示例流程",
  "quality.title": "质量报告",
  "quality.waiting": "等待处理",
  "quality.noReport": "还没有质量报告",
  "quality.noReportDetail": "使用真实来源，或运行内置示例流程。",
  "quality.checksAfterProcessing": "处理后检查",
  "quality.processImported": "处理导入帧以解锁导出检查。",
  "quality.runSample": "运行示例流程",
  "quality.processFromReport": "从质量报告处理帧",
  "quality.verdict.gameReady": "可用于游戏",
  "quality.verdict.needsCleanup": "需要清理",
  "quality.verdict.prototypeUsable": "可用于原型",
  "quality.verdict.blocked": "已阻止",
  "export.generatedOutputs": "生成输出",
  "export.ready": "可以导出",
  "export.readyDetail": "已有处理后的帧和质量报告。",
  "export.missingBeforeExport": "导出前还缺少（{count}）",
  "export.readiness": "导出准备状态",
  "export.packName": "资源包名称",
  "export.animationName": "动画名称",
  "export.metadata": "资源包元数据和精灵表设置",
  "export.metadata.show": "显示",
  "export.metadata.hide": "隐藏",
  "export.creator": "作者",
  "export.license": "许可",
  "export.sheetColumns": "精灵表列数",
  "export.sheetPadding": "精灵表内边距",
  "export.sheetMargin": "精灵表外边距",
  "export.splitSheets": "需要时拆分精灵表",
  "export.loop": "导出循环动画",
  "export.output": "输出 .gsfpack 和通用文件",
  "export.exportPack": "导出资源包",
  "export.validateReimport": "验证重新导入",
  "export.noExport": "尚未导出",
  "export.openExportsFolder": "打开导出文件夹",
  "export.outputDetails": "导出输出详情",
  "export.outputPack": "资源包：{path}",
  "export.outputFrames": "帧数：{count}",
  "export.outputSpriteSheets": "精灵表页数：{count}",
  "export.outputGodotHelper": "Godot 辅助文件：{path}",
  "export.lastValidated": "上次验证：{name}，{count} 帧。",
  "pipeline.checkFfmpeg": "检查 FFmpeg",
  "pipeline.extractFrames": "抽取帧",
  "pipeline.processQuality": "处理并检查质量",
};

const dictionaries: Record<AppLocale, TranslationDictionary> = {
  "en-US": enUS,
  "zh-CN": zhCN,
};

export function resolveAppLocale(languageMode: LanguageMode, languages = globalThis.navigator?.languages ?? []): AppLocale {
  if (languageMode !== "auto") {
    return languageMode;
  }

  const preferred = languages.length ? languages : [globalThis.navigator?.language ?? "en-US"];
  return preferred.some((language) => language.toLowerCase().startsWith("zh")) ? "zh-CN" : "en-US";
}

export function createTranslator(locale: AppLocale) {
  const dictionary = dictionaries[locale];
  return function t(key: TranslationKey, params: TranslationParams = {}) {
    const template = dictionary[key] ?? enUS[key] ?? key;
    return template.replace(/\{(\w+)\}/g, (_, name: string) => String(params[name] ?? `{${name}}`));
  };
}

export const languageModes: LanguageMode[] = ["auto", "en-US", "zh-CN"];
```

- [ ] **Step 2: Run TypeScript check through build**

Run:

```bash
npm --workspace apps/mac run build
```

Expected before wiring:

```text
vite build completes successfully because i18n.ts is self-contained.
```

---

### Task 3: Persist Language Mode In Settings

**Files:**
- Modify: `apps/mac/src/tauriCommands.ts`
- Modify: `apps/mac/src/App.tsx`
- Modify: `apps/mac/src/routes/SettingsRoute.tsx`

- [ ] **Step 1: Extend `LocalSettings`**

In `apps/mac/src/tauriCommands.ts`, import the type and add the field:

```ts
import type { LanguageMode } from "./i18n";

export type LocalSettings = {
  ffmpegPath: string;
  ffprobePath: string;
  defaultOutputFolder: string;
  defaultFps: number;
  defaultSheetSize: number;
  languageMode: LanguageMode;
};
```

- [ ] **Step 2: Set the default language mode**

In `apps/mac/src/App.tsx`, update `defaultSettings`:

```ts
const defaultSettings: LocalSettings = {
  ffmpegPath: "",
  ffprobePath: "",
  defaultOutputFolder: "~/Game Sprite Forge/Exports",
  defaultFps: 12,
  defaultSheetSize: 2048,
  languageMode: "auto",
};
```

Add a language sanitizer near `positiveNumber`:

```ts
function languageModeValue(value: unknown): LocalSettings["languageMode"] {
  return value === "en-US" || value === "zh-CN" || value === "auto" ? value : "auto";
}
```

Update `loadSettings()` merge:

```ts
return {
  ...defaultSettings,
  ...parsed,
  defaultFps: positiveNumber(parsed.defaultFps, defaultSettings.defaultFps),
  defaultSheetSize: positiveNumber(parsed.defaultSheetSize, defaultSettings.defaultSheetSize),
  languageMode: languageModeValue(parsed.languageMode),
};
```

- [ ] **Step 3: Add Settings language selector props**

In `apps/mac/src/routes/SettingsRoute.tsx`, import translator types:

```ts
import type { TranslationKey } from "../i18n";
import { languageModes } from "../i18n";
```

Update props:

```ts
type SettingsRouteProps = {
  onChooseFfmpegPath: () => void;
  onChooseFfprobePath: () => void;
  onChooseOutputFolder: () => void;
  settings: LocalSettings;
  onSettingsChange: (settings: LocalSettings) => void;
  t: (key: TranslationKey) => string;
};
```

Render this field as the first settings-grid item:

```tsx
<label className="settings-field settings-language-field">
  <span className="settings-label">
    <Languages size={16} />
    {t("settings.language.label")}
  </span>
  <select
    className="settings-language-select"
    value={settings.languageMode}
    onChange={(event) => updateField("languageMode", event.target.value as LocalSettings["languageMode"])}
  >
    {languageModes.map((mode) => (
      <option key={mode} value={mode}>
        {mode === "auto"
          ? t("settings.language.auto")
          : mode === "en-US"
            ? t("settings.language.english")
            : t("settings.language.chinese")}
      </option>
    ))}
  </select>
</label>
```

Also import the icon:

```ts
import { FolderCog, Gauge, Languages, MonitorDown } from "lucide-react";
```

- [ ] **Step 4: Localize Settings labels**

Replace hard-coded settings strings with `t(...)`:

```tsx
<span>{t("settings.title")}</span>
<span>{t("settings.subtitle")}</span>
chooseLabel={t("settings.chooseFfmpeg")}
label={t("settings.ffmpegPath")}
chooseLabel={t("settings.chooseFfprobe")}
label={t("settings.ffprobePath")}
label={t("settings.defaultOutputFolder")}
label={t("settings.defaultFps")}
label={t("settings.defaultSheetSize")}
{t("settings.note")}
```

- [ ] **Step 5: Run the build**

Run:

```bash
npm --workspace apps/mac run build
```

Expected:

```text
TypeScript reports missing `t` prop at the SettingsRoute call in App.tsx.
```

This expected failure is resolved in Task 4.

---

### Task 4: Wire Translator Through App Shell And Exports

**Files:**
- Modify: `apps/mac/src/App.tsx`

- [ ] **Step 1: Import i18n helpers**

At the top of `apps/mac/src/App.tsx`:

```ts
import { createTranslator, resolveAppLocale, type TranslationKey } from "./i18n";
```

- [ ] **Step 2: Replace static nav items with translated nav rendering**

Replace the `navItems` constant with icon-only metadata:

```ts
const navItems: Array<{
  key: RouteKey;
  labelKey: TranslationKey;
  icon: LucideIcon;
  available?: boolean;
}> = [
  { key: "forge", labelKey: "app.nav.forge", icon: Hammer },
  { key: "exports", labelKey: "app.nav.exports", icon: FolderOutput },
  { key: "settings", labelKey: "app.nav.settings", icon: Settings },
];
```

In `App()`, after `settings`:

```ts
const locale = resolveAppLocale(settings.languageMode);
const t = createTranslator(locale);
```

Update nav rendering:

```tsx
{navItems.map((item) => (
  <button
    className={activeRoute === item.key ? "active" : ""}
    disabled={item.available === false}
    key={item.key}
    onClick={() => setActiveRoute(item.key)}
    type="button"
  >
    <item.icon size={20} />
    {t(item.labelKey)}
  </button>
))}
```

- [ ] **Step 3: Translate workflow labels without changing workflow keys**

Keep `workflowTabs` as English internal keys:

```ts
const workflowTabs = ["Import", "Frames", "Background", "Anchor", "Sheet", "Export"] as const;
```

Add helper:

```ts
const workflowLabelKeys: Record<WorkflowKey, TranslationKey> = {
  Import: "workflow.import",
  Frames: "workflow.frames",
  Background: "workflow.background",
  Anchor: "workflow.anchor",
  Sheet: "workflow.sheet",
  Export: "workflow.export",
};
```

Render tab labels with:

```tsx
{t(workflowLabelKeys[tab])}
```

Keep `activeWorkflow` and `workflowAccess` as English internal keys so no Rust/Tauri state contracts change.

- [ ] **Step 4: Translate shell header and quality status**

Replace:

```tsx
<h1>Game Sprite Forge</h1>
<strong>Local Workbench</strong>
<span>IMPORT-ONLY MVP</span>
<span>Quality status</span>
```

With:

```tsx
<h1>{t("app.name")}</h1>
<strong>{t("app.workbench.title")}</strong>
<span>{t("app.workbench.subtitle")}</span>
<span>{t("app.qualityStatus")}</span>
```

Replace `qualityStateLabel` with:

```ts
const qualityStateLabel =
  activeRoute !== "forge"
    ? t("app.status.openForge")
    : workbenchState.canRunQualityCheck
      ? t("app.status.ready")
      : t("app.status.pending");
```

- [ ] **Step 5: Localize ExportsRoute props and status handlers**

Add `t` to `ExportsRoute` props:

```ts
t: (key: TranslationKey, params?: Record<string, string | number>) => string;
```

Pass it:

```tsx
<ExportsRoute
  exports={recentExports}
  libraryAction={libraryAction}
  libraryPacks={libraryPacks}
  libraryStatus={libraryStatus}
  onInspectPack={handleInspectPack}
  onOpenExportFolder={handleOpenExportFolder}
  onRefreshLibrary={handleRefreshLibrary}
  onReimportPack={handleReimportPack}
  onValidatePack={handleValidatePack}
  t={t}
/>
```

Replace ExportsRoute hard-coded copy:

```tsx
<span>{t("exports.library.title")}</span>
<span>{t("exports.library.subtitle")}</span>
{libraryAction === "refresh" ? t("exports.library.refreshing") : t("exports.library.refresh")}
{libraryAction === "inspect" ? t("exports.library.inspecting") : t("exports.library.inspect")}
{libraryAction === "validate" ? t("exports.library.validating") : t("exports.library.validate")}
{libraryAction === "reimport" ? t("exports.library.reimporting") : t("exports.library.reimport")}
{libraryAction === "open" ? t("exports.library.opening", { name: fileName(pack.root) }) : t("exports.library.open")}
{t("exports.openFolder")}
<strong>{t("exports.empty.title")}</strong>
<span>{t("exports.empty.detail")}</span>
```

Update the state initial value:

```ts
const [libraryStatus, setLibraryStatus] = useState(() => createTranslator(resolveAppLocale(defaultSettings.languageMode))("exports.library.initialStatus"));
```

Add this effect after `t` exists:

```ts
useEffect(() => {
  if (!libraryPacks.length && libraryAction === null) {
    setLibraryStatus(t("exports.library.initialStatus"));
  }
}, [libraryAction, libraryPacks.length, settings.languageMode]);
```

Update status handlers:

```ts
setLibraryStatus(t("exports.library.opening", { name: fileName(path) }));
setLibraryStatus(t("exports.library.opened", { name: fileName(path) }));
setLibraryStatus(t("exports.library.scanning"));
setLibraryStatus(t("exports.library.found", { count: packs.length, packWord: packs.length === 1 ? "pack" : "packs" }));
setLibraryStatus(t("exports.library.inspected", { name: pack.name }));
setLibraryStatus(t("exports.library.validationPassed"));
```

For Chinese, `packWord` is ignored by the template. For English, it preserves singular/plural.

- [ ] **Step 6: Pass `t` into SettingsRoute**

Update the SettingsRoute call:

```tsx
<SettingsRoute
  onChooseFfmpegPath={handleChooseFfmpegPath}
  onChooseFfprobePath={handleChooseFfprobePath}
  onChooseOutputFolder={handleChooseOutputFolder}
  onSettingsChange={setSettings}
  settings={settings}
  t={t}
/>
```

- [ ] **Step 7: Run source guard and build**

Run:

```bash
npm run test:scripts
npm --workspace apps/mac run build
```

Expected:

```text
PASS UI language source test
vite build completes without TypeScript errors
```

---

### Task 5: Localize High-Impact Workflow Components

**Files:**
- Modify: `apps/mac/src/components/ImportPanel.tsx`
- Modify: `apps/mac/src/components/ExportPanel.tsx`
- Modify: `apps/mac/src/components/QualityInspector.tsx`
- Modify: `apps/mac/src/routes/ForgeRoute.tsx`

- [ ] **Step 1: Add `t` props to component boundaries**

For each component, import:

```ts
import type { TranslationKey } from "../i18n";
```

Add prop:

```ts
t: (key: TranslationKey, params?: Record<string, string | number>) => string;
```

Wire the prop from `ForgeRoute`:

```tsx
<ImportPanel ... t={t} />
<QualityInspector ... t={t} />
<ExportPanel ... t={t} />
```

Add `t` to `ForgeRouteProps`:

```ts
t: (key: TranslationKey, params?: Record<string, string | number>) => string;
```

Pass `t` from `App.tsx`:

```tsx
<ForgeRoute
  ...
  t={t}
/>
```

- [ ] **Step 2: Localize ImportPanel high-impact copy**

Replace hard-coded ImportPanel strings:

```tsx
<span>{t("import.sources")}</span>
{t("import.loadSamplePath")}
<small className="sample-action-hint">{t("import.sampleHint")}</small>
{t("import.video")}
{t("import.pngSequence")}
{t("import.spriteSheet")}
{t("import.gsfpack")}
{t("import.chooseFile")}
{t("import.chooseFolder")}
```

For source mode buttons, use:

```tsx
{selected ? t("import.importSelected") : t("import.selectSource")}
```

- [ ] **Step 3: Localize ExportPanel high-impact copy**

Replace hard-coded ExportPanel strings:

```tsx
{t("export.generatedOutputs")}
{canExport ? t("export.ready") : t("export.missingBeforeExport", { count: blockers.length })}
{canExport ? t("export.readyDetail") : blockers[0]?.detail ?? "Confirm settings before exporting."}
aria-label={t("export.readiness")}
{t("export.packName")}
{t("export.animationName")}
<summary>{t("export.metadata")}</summary>
{detailsOpen ? t("export.metadata.hide") : t("export.metadata.show")}
{t("export.creator")}
{t("export.license")}
{t("export.sheetColumns")}
{t("export.sheetPadding")}
{t("export.sheetMargin")}
{t("export.splitSheets")}
{t("export.loop")}
{t("export.output")}
{t("export.exportPack")}
{t("export.validateReimport")}
{exportOutput ? null : t("export.noExport")}
{t("export.openExportsFolder")}
aria-label={t("export.outputDetails")}
{t("export.outputPack", { path: exportOutput.packDir })}
{t("export.outputFrames", { count: exportOutput.framePaths.length })}
{t("export.outputSpriteSheets", { count: exportOutput.spriteSheetPaths.length })}
{t("export.outputGodotHelper", { path: exportOutput.godotHelperPath })}
{t("export.lastValidated", { name: packSummary.name, count: packSummary.frameCount })}
```

Keep raw blocker details from ForgeRoute until Task 6 maps them to translation keys.

- [ ] **Step 4: Localize QualityInspector high-impact copy**

Replace hard-coded QualityInspector strings:

```tsx
{t("quality.title")}
{report ? t(verdictLabelKey(report.verdict)) : t("quality.waiting")}
{t("quality.noReport")}
{canProcessAndQuality ? t("quality.processImported") : t("quality.noReportDetail")}
{t("quality.checksAfterProcessing")}
{canProcessAndQuality ? t("quality.processFromReport") : t("quality.runSample")}
```

Add helper in `QualityInspector.tsx`:

```ts
function verdictLabelKey(verdict: QualityReport["verdict"]): TranslationKey {
  switch (verdict) {
    case "game_ready":
      return "quality.verdict.gameReady";
    case "needs_cleanup":
      return "quality.verdict.needsCleanup";
    case "prototype_usable":
      return "quality.verdict.prototypeUsable";
    case "blocked":
      return "quality.verdict.blocked";
  }
}
```

- [ ] **Step 5: Localize First Run and pipeline action labels in ForgeRoute**

Replace `ActivationRail` hard-coded labels:

```tsx
<span>{t("firstRun.title")}</span>
<small>{t("firstRun.detail")}</small>
{t("firstRun.checkToolchain")}
{t("firstRun.selectSource")}
{t("firstRun.processFrames")}
{t("firstRun.exportPack")}
{t("firstRun.runSample")}
```

Replace pipeline buttons:

```tsx
{t("pipeline.checkFfmpeg")}
{t("pipeline.extractFrames")}
{t("pipeline.processQuality")}
```

- [ ] **Step 6: Run build**

Run:

```bash
npm --workspace apps/mac run build
```

Expected:

```text
vite build completes without TypeScript errors.
```

---

### Task 6: Add Locale-Aware UI Smoke Tests

**Files:**
- Modify: `apps/mac/scripts/smoke-ui.mjs`

- [ ] **Step 1: Add locale storage injection**

Near the top of `apps/mac/scripts/smoke-ui.mjs`, add:

```js
const smokeLocale = process.env.FORGE_SMOKE_LOCALE || "en-US";
```

Before screenshot capture, point Chrome at a locale-specific URL:

```js
const targetUrl = `${url}?forgeSmokeLocale=${encodeURIComponent(smokeLocale)}`;
```

Use `targetUrl` instead of `url` in Chrome arguments.

In `apps/mac/src/App.tsx`, add a test-only initializer inside `loadSettings()` before reading saved settings:

```ts
const smokeLocale = new URLSearchParams(globalThis.location.search).get("forgeSmokeLocale");
if (smokeLocale === "en-US" || smokeLocale === "zh-CN") {
  return { ...defaultSettings, languageMode: smokeLocale };
}
```

This keeps production behavior unchanged and makes Vite smoke deterministic.

- [ ] **Step 2: Split source assertions by locale**

In `assertSource(source, smokeMode)`, keep shared structural assertions, then add:

```js
const englishRequired = [
  "Local Workbench",
  "Run Sample Pipeline",
  "Load Sample Path",
  "Export Pack",
  "Validate Re-import",
  "Settings saved locally",
  "Local Pack Library",
];

const chineseRequired = [
  "本地工作台",
  "运行示例流程",
  "填入示例路径",
  "导出资源包",
  "验证重新导入",
  "设置已保存到本机",
  "本地资源包库",
];

const localeRequired = smokeLocale === "zh-CN" ? chineseRequired : englishRequired;
for (const needle of localeRequired) {
  if (!source.includes(needle)) {
    throw new Error(`Missing ${smokeLocale} UI smoke string: ${needle}`);
  }
}
```

Keep the no-product-surface assertions:

```js
const forbidden = ["BYOK", "Marketplace", "Creator Publish", "MCP server", "Cloud upload"];
for (const needle of forbidden) {
  if (source.includes(needle)) {
    throw new Error(`MVP UI smoke found forbidden product surface: ${needle}`);
  }
}
```

- [ ] **Step 3: Run English and Chinese smoke**

Run:

```bash
npm --workspace apps/mac run build
FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:mvp
FORGE_SMOKE_LOCALE=zh-CN npm --workspace apps/mac run smoke:ui:mvp
FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:responsive
FORGE_SMOKE_LOCALE=zh-CN npm --workspace apps/mac run smoke:ui:responsive
```

Expected:

```text
UI smoke passed (mvp)
UI smoke passed (responsive)
```

---

### Task 7: Record Real UI Evidence With Computer Use MCP

**Files:**
- Create: `docs/qa/ui-language-evidence-2026-06-06.md`
- Modify: `docs/architecture/mvp-scope.md`
- Modify: `docs/architecture/post-release-backlog.md`

- [ ] **Step 1: Build and launch the app**

Run:

```bash
npm --workspace apps/mac run build
npm --workspace apps/mac run tauri -- build
open -n "/Applications/Game Sprite Forge.app"
```

If the implementation is not yet packaged into `/Applications`, launch the Tauri dev or built artifact used by the current project workflow and record the exact app path in the QA evidence.

- [ ] **Step 2: Use Computer Use MCP for Automatic mode**

In the app:

```text
Open Settings.
Set Language to Automatic / 跟随系统.
Quit and reopen the app.
Verify the app resolves to the current macOS/WebView preferred language.
Record whether the current machine resolves Automatic to English or Simplified Chinese.
```

Record:

```text
Automatic mode: pass/fail
Resolved language:
Visible Settings strings:
Visible Forge strings:
```

- [ ] **Step 3: Use Computer Use MCP for English override**

In the app:

```text
Open Settings.
Set Language to English.
Verify Settings shows Local Toolchain, Language, Automatic, English, 简体中文.
Open Forge.
Verify Local Workbench, Run Sample Pipeline, Load Sample Path.
Open Exports.
Verify Local Pack Library and Refresh Library.
```

Record:

```text
English override: pass/fail
Visible strings observed
App path
Bundle identifier
```

- [ ] **Step 4: Use Computer Use MCP for Simplified Chinese override**

In the app:

```text
Open Settings.
Set Language to 简体中文.
Verify Settings shows 本地工具链, 语言, 跟随系统, English, 简体中文.
Open Forge.
Verify 本地工作台, 运行示例流程, 填入示例路径.
Open Exports.
Verify 本地资源包库 and 刷新资源包库.
```

Record:

```text
Simplified Chinese override: pass/fail
Visible strings observed
Any clipping or overlap
```

- [ ] **Step 5: Verify the primary workflow in one locale**

Use Simplified Chinese unless the build is blocked:

```text
Click 运行示例流程.
Confirm the app processes 24 frames.
Confirm export and validate re-import still work.
Confirm no UI text overlap in the right panel, First Run rail, Settings selector, and Exports action buttons.
```

Record:

```text
Run Sample Pipeline localized flow: pass/fail
Frame count
Export result
Validate Re-import result
```

- [ ] **Step 6: Create QA evidence doc**

Create `docs/qa/ui-language-evidence-2026-06-06.md`:

````md
# UI Language Evidence - 2026-06-06

## Artifact

```text
App path:
Bundle identifier:
Build source:
```

## Language Strategy

```text
Default mode: Automatic
Manual modes: English, 简体中文
Persistence: game-sprite-forge.local-settings.v1
```

## English Override

```text
Result:
Observed Settings strings:
Observed Forge strings:
Observed Exports strings:
```

## Automatic Mode

```text
Result:
Resolved language:
Observed Settings strings:
Observed Forge strings:
```

## Simplified Chinese Override

```text
Result:
Observed Settings strings:
Observed Forge strings:
Observed Exports strings:
Layout issues:
```

## Primary Workflow

```text
Result:
Sample pipeline:
Export:
Validate Re-import:
Frame count:
```

## Scope Guard

```text
No AI, BYOK, cloud, marketplace, product CLI, or product MCP surface appeared in the localized MVP UI.
```
````

- [ ] **Step 7: Update architecture docs**

In `docs/architecture/mvp-scope.md`, add to enabled MVP status:

```text
- UI language mode supports Automatic, English, and Simplified Chinese for the app shell and primary local workbench flow.
```

In latest verification evidence, add:

```text
English and Simplified Chinese UI smoke: pass
Computer Use UI language evidence: pass; see docs/qa/ui-language-evidence-2026-06-06.md
```

In `docs/architecture/post-release-backlog.md`, add:

```text
UI Language Localization: completed for Automatic, English, and Simplified Chinese app shell plus primary workflow.
Future localization follow-up: migrate long-tail diagnostic strings and Rust-originated errors into typed recovery messages.
```

- [ ] **Step 8: Final verification**

Run:

```bash
npm run test:scripts
npm --workspace apps/mac run build
FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:mvp
FORGE_SMOKE_LOCALE=zh-CN npm --workspace apps/mac run smoke:ui:mvp
FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:responsive
FORGE_SMOKE_LOCALE=zh-CN npm --workspace apps/mac run smoke:ui:responsive
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml
```

Expected:

```text
All commands pass.
Computer Use evidence exists.
No forbidden product surfaces appear.
```

---

## Non-Goals

```text
Machine translation
Remote translation service
Website localization
Marketplace localization
Cloud account or BYOK language settings
Product CLI language flags
Product MCP server
Rust core error-code redesign
Full locale formatting for dates, numbers, and file sizes
Localizing generated filenames or .gsfpack schema values
```

## Follow-Up After This Plan

```text
1. Add typed recovery-message keys for Rust/Tauri errors so raw technical messages can be wrapped cleanly in both languages.
2. Add zh-CN real UI screenshot evidence to release candidate verification after the next notarized package.
3. Consider Japanese only after English and Simplified Chinese are stable and the app has broader external testers.
```

## Execution Notes

```text
/Users/kartz/Development/Forge is not a git repository in this environment, so this plan intentionally omits commit steps.
Use Computer Use MCP only for real installed-app verification; do not expose MCP as a product feature.
Use Figma only if a later visual pass is needed for Chinese layout clipping; this plan can be implemented from the local codebase.
```
