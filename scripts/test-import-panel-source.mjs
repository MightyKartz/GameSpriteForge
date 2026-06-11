import { readFileSync } from "node:fs";

const source = readFileSync("apps/mac/src/components/ImportPanel.tsx", "utf8");
const forgeRouteSource = readFileSync("apps/mac/src/routes/ForgeRoute.tsx", "utf8");
const tauriCommandsSource = readFileSync("apps/mac/src/tauriCommands.ts", "utf8");
const tauriLibSource = readFileSync("apps/mac/src-tauri/src/lib.rs", "utf8");
const i18nSource = readFileSync("apps/mac/src/i18n.ts", "utf8");
const pngSequenceLabelIndex = source.indexOf('summary={t("import.pngSequence")}');
const pngSequenceAction = pngSequenceLabelIndex >= 0
  ? source.slice(Math.max(0, pngSequenceLabelIndex - 300), pngSequenceLabelIndex + 500)
  : "";

function assertSourceContains(needle, message) {
  if (!source.includes(needle)) {
    throw new Error(message);
  }
}

function assertI18nContains(needle, message) {
  if (!i18nSource.includes(needle)) {
    throw new Error(message);
  }
}

assertI18nContains(
  "newline or comma separated PNGs",
  "ImportPanel must keep the PNG sequence affordance text.",
);
assertSourceContains(
  'summary={t("import.pngSequence")}',
  "ImportPanel must keep the PNG sequence import action.",
);
if (!pngSequenceAction.includes("multiline")) {
  throw new Error("PNG sequence PathAction must opt into multiline input.");
}
assertSourceContains(
  "<textarea",
  "PathAction must render a textarea for multiline path input.",
);
assertSourceContains(
  'className="path-input path-input-multiline"',
  "Multiline path input must use the multiline path-input class.",
);
assertI18nContains(
  "Load Sample Path",
  "ImportPanel sample action must say Load Sample Path so users know it only fills the source field.",
);
assertI18nContains(
  "Import Video",
  "ImportPanel must keep Import Video as the action that creates a live workspace.",
);

function assertForgeRouteContains(needle, message) {
  if (!forgeRouteSource.includes(needle)) {
    throw new Error(message);
  }
}

function assertTauriCommandsContains(needle, message) {
  if (!tauriCommandsSource.includes(needle)) {
    throw new Error(message);
  }
}

function assertTauriLibContains(needle, message) {
  if (!tauriLibSource.includes(needle)) {
    throw new Error(message);
  }
}

assertSourceContains(
  "spriteSheetSliceMode",
  "ImportPanel must expose sprite sheet split mode.",
);
assertSourceContains(
  "import.spriteSheetSplitTransparent",
  "ImportPanel must expose transparent-gutter split mode.",
);
assertI18nContains(
  "Transparent gutters",
  "ImportPanel i18n must label transparent-gutter split mode.",
);
assertI18nContains(
  "透明间隔",
  "ImportPanel i18n must include Simplified Chinese transparent-gutter split mode.",
);
assertTauriCommandsContains(
  "sliceSpriteSheetTransparent",
  "TypeScript must wrap transparent sprite sheet splitting.",
);
assertForgeRouteContains(
  "sliceSpriteSheetTransparent",
  "ForgeRoute must call transparent sprite sheet splitting.",
);
assertForgeRouteContains(
  "transparentSplitAlphaThreshold",
  "ForgeRoute must keep transparent split alpha threshold state.",
);
assertForgeRouteContains(
  "transparentSplitMinGapPx",
  "ForgeRoute must keep transparent split gap state.",
);
assertForgeRouteContains(
  "handleChooseSpriteSheetForConfiguration",
  "Central sprite sheet launcher must choose a file before import so users can select the transparent-gutter mode.",
);
assertForgeRouteContains(
  "onChooseSpriteSheet={handleChooseSpriteSheetForConfiguration}",
  "Central sprite sheet launcher must not immediately import before split mode selection.",
);
assertForgeRouteContains(
  'selectedSpriteSheetName={spriteSheetPath.trim() ? fileName(spriteSheetPath) : null}',
  "Central sprite sheet launcher must receive the selected sprite sheet name.",
);
assertForgeRouteContains(
  "onImportSpriteSheet={handleImportSpriteSheet}",
  "Central sprite sheet launcher must expose an import-selected action after a sprite sheet is chosen.",
);
assertForgeRouteContains(
  "importLauncher.spriteSheetReadyTitle",
  "Central sprite sheet launcher must switch to a sprite-sheet configuration state after file selection.",
);
assertForgeRouteContains(
  "importLauncher.importSpriteSheet",
  "Central sprite sheet launcher must render a visible Import Selected Sprite Sheet CTA.",
);
assertI18nContains(
  "Configure Sprite Sheet Import",
  "Central sprite sheet launcher must have English copy for the sprite-sheet configuration state.",
);
assertI18nContains(
  "配置精灵表导入",
  "Central sprite sheet launcher must have Chinese copy for the sprite-sheet configuration state.",
);
assertTauriLibContains(
  "fn slice_sprite_sheet_transparent",
  "Tauri must expose a transparent sprite sheet split command.",
);
assertTauriLibContains(
  "slice_sprite_sheet_transparent,",
  "Tauri generate_handler must register transparent sprite sheet splitting.",
);

assertI18nContains(
  "Run Sample Pipeline",
  "First Run rail must label the full bundled sample action as Run Sample Pipeline.",
);
assertI18nContains(
  "Use the center launcher to import your source. The sample is optional.",
  "First Run rail must make the sample secondary to the center import launcher.",
);
assertForgeRouteContains(
  't("firstRun.runSample")',
  "First Run rail must render the translated full bundled sample action.",
);
assertI18nContains(
  "Choose Video File",
  "The empty workbench must expose a direct Choose Video File CTA.",
);
assertForgeRouteContains(
  "ImportLauncher",
  "ForgeRoute must render the central import launcher in the no-source state.",
);

console.log("PASS import panel source test");
