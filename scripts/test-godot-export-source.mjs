import { readFileSync } from "node:fs";

const script = readFileSync("scripts/godot/import_forge_pack.gd", "utf8");
const packageJson = readFileSync("package.json", "utf8");
const coreModSource = readFileSync("packages/core/src/export/mod.rs", "utf8");
const coreGodotSource = readFileSync("packages/core/src/export/godot.rs", "utf8");
const tauriSource = readFileSync("apps/mac/src-tauri/src/lib.rs", "utf8");
const commandsSource = readFileSync("apps/mac/src/tauriCommands.ts", "utf8");
const exportPanelSource = readFileSync("apps/mac/src/components/ExportPanel.tsx", "utf8");
const forgeRouteSource = readFileSync("apps/mac/src/routes/ForgeRoute.tsx", "utf8");

function assertContains(source, needle, message) {
  if (!source.includes(needle)) {
    throw new Error(message);
  }
}

assertContains(script, "SpriteFrames.new()", "Godot importer must create SpriteFrames.");
assertContains(script, "AtlasTexture.new()", "Godot importer must create AtlasTexture frames.");
assertContains(script, "AnimatedSprite2D.new()", "Godot importer must create AnimatedSprite2D.");
assertContains(script, "texture_map", "Godot importer must map every exported sprite sheet texture.");
assertContains(script, "atlas_frame.get(\"image\"", "Godot importer must select per-frame textures for multi-sheet packs.");
assertContains(script, "loaded_sprite.play", "Godot importer must verify the loaded animation can play.");
assertContains(packageJson, "smoke:godot", "package.json must expose the Godot smoke command.");
assertContains(coreModSource, "pub mod godot", "forge_core export module must expose Godot export.");
assertContains(coreGodotSource, "export_godot_project", "forge_core must implement Godot project export.");
assertContains(tauriSource, "export_godot_project", "Tauri must expose export_godot_project.");
assertContains(tauriSource, "normalize_user_export_directory(&params.output_dir)", "Tauri Godot export must normalize the output folder.");
assertContains(tauriSource, "validate_pack_layout(&params.pack_dir)", "Tauri Godot export must validate the .gsfpack layout.");
assertContains(commandsSource, "exportGodotProject", "TypeScript must wrap Godot project export.");
assertContains(exportPanelSource, "export.exportGodotProject", "ExportPanel must show the Godot export action.");
assertContains(forgeRouteSource, "handleExportGodotProject", "ForgeRoute must wire the Godot export handler.");

console.log("PASS Godot export source test");
