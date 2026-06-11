import { existsSync, readFileSync } from "node:fs";

const appSource = readFileSync("apps/mac/src/App.tsx", "utf8");
const i18nSource = existsSync("apps/mac/src/i18n.ts")
  ? readFileSync("apps/mac/src/i18n.ts", "utf8")
  : "";
const settingsSource = readFileSync("apps/mac/src/routes/SettingsRoute.tsx", "utf8");
const commandsSource = readFileSync("apps/mac/src/tauriCommands.ts", "utf8");
const smokeSource = readFileSync("apps/mac/scripts/smoke-ui.mjs", "utf8");
const packageSource = readFileSync("package.json", "utf8");
const qualitySchemaSource = readFileSync("schemas/quality-report.schema.json", "utf8");
const qualityInspectorSource = readFileSync("apps/mac/src/components/QualityInspector.tsx", "utf8");

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
assertContains(smokeSource, "visible-text", "UI smoke must write runtime visible text evidence.");
assertContains(smokeSource, "assertVisibleText", "UI smoke must assert runtime visible text, not only built source.");
assertContains(smokeSource, "Current Source", "UI smoke must forbid obvious English source-panel copy in zh-CN mode.");
assertContains(smokeSource, "Frame Timeline", "UI smoke must forbid obvious English timeline copy in zh-CN mode.");
assertContains(i18nSource, "当前来源", "Chinese dictionary must include source panel copy.");
assertContains(i18nSource, "帧时间线", "Chinese dictionary must include timeline copy.");
assertContains(i18nSource, "精灵表预览", "Chinese dictionary must include sprite sheet preview copy.");
assertContains(i18nSource, "完整流程已通过", "Chinese dictionary must include full sample pipeline completion copy.");
assertContains(appSource, "data-locale", "App shell must expose locale for CSS language-specific overrides.");

for (const recommendation of [
  ["adjust_anchor", "quality.recommendation.adjustAnchor", "调整脚底锚点"],
  ["trim_loop_range", "quality.recommendation.trimLoopRange", "修剪循环"],
  ["increase_chroma_threshold", "quality.recommendation.increaseChromaThreshold", "提高色键阈值"],
  ["reduce_chroma_threshold", "quality.recommendation.reduceChromaThreshold", "降低色键阈值"],
  ["use_shorter_clip", "quality.recommendation.useShorterClip", "使用更短"],
  ["increase_canvas_margin", "quality.recommendation.increaseCanvasMargin", "增加画布边距"],
]) {
  const [id, key, zhCopy] = recommendation;
  assertContains(qualitySchemaSource, id, `Quality schema must continue exposing recommendation id ${id}.`);
  assertContains(qualityInspectorSource, `${id}: "${key}"`, `QualityInspector must map ${id} to a translation key.`);
  assertContains(i18nSource, `"${key}"`, `i18n must define ${key}.`);
  assertContains(i18nSource, zhCopy, `Chinese dictionary must translate recommendation ${id}.`);
}

assertContains(packageSource, "scripts/test-ui-language-source.mjs", "test:scripts must include the UI language source guard.");

if (!existsSync("docs/qa/ui-language-evidence-2026-06-06.md")) {
  throw new Error("Computer Use evidence must be recorded in docs/qa/ui-language-evidence-2026-06-06.md");
}

console.log("PASS UI language source test");
