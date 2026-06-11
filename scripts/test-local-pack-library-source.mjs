import { readFileSync } from "node:fs";

const packSource = readFileSync("packages/pack/src/lib.rs", "utf8");
const packTests = readFileSync("packages/pack/tests/pack_tests.rs", "utf8");
const tauriSource = readFileSync("apps/mac/src-tauri/src/lib.rs", "utf8");
const commandSource = readFileSync("apps/mac/src/tauriCommands.ts", "utf8");
const appSource = readFileSync("apps/mac/src/App.tsx", "utf8");
const forgeRouteSource = readFileSync("apps/mac/src/routes/ForgeRoute.tsx", "utf8");
const i18nSource = readFileSync("apps/mac/src/i18n.ts", "utf8");

function assertContains(source, needle, message) {
  if (!source.includes(needle)) {
    throw new Error(message);
  }
}

assertContains(tauriSource, "list_local_packs", "Tauri must expose list_local_packs.");
assertContains(tauriSource, "inspect_local_pack", "Tauri must expose inspect_local_pack.");
assertContains(packSource, "symlink_metadata", "Pack validation must inspect symlink metadata without following links.");
assertContains(packSource, "entry.file_type()", "Pack frame enumeration must check regular file entries.");
assertContains(tauriSource, "canonical_exports_dir", "Local pack scan must canonicalize the exports root.");
assertContains(tauriSource, "canonical_pack_candidate", "Local pack scan must canonicalize pack candidates.");
assertContains(tauriSource, "starts_with(canonical_exports_dir)", "Local pack scan must keep candidates contained under the exports root.");
assertContains(tauriSource, "entry.file_type()", "Local pack scan must use entry file types to skip symlinks.");
assertContains(packTests, "symlinked_required_file_fails_validation", "Pack tests must cover symlinked forgepack.json rejection.");
assertContains(packTests, "symlinked_frames_directory_fails_validation", "Pack tests must cover symlinked assets/frames rejection.");
assertContains(packTests, "symlinked_parent_asset_directory_fails_validation", "Pack tests must cover symlinked parent asset directory rejection.");
assertContains(packTests, "symlinked_pack_root_fails_validation", "Pack tests must cover symlinked pack root rejection.");
assertContains(packTests, "symlinked_frame_pngs_are_not_counted", "Pack tests must cover symlinked frame PNG exclusion.");
assertContains(packTests, "inspect_pack_returns_paths_needed_by_local_library", "Pack tests must cover inspect_pack paths for Local Pack Library.");
assertContains(tauriSource, "list_local_packs_lists_direct_and_one_level_nested_valid_packs", "Local library tests must cover direct and nested packs.");
assertContains(tauriSource, "list_local_packs_ignores_broken_pack_folders", "Local library tests must cover broken pack folders.");
assertContains(tauriSource, "list_local_packs_skips_symlinked_pack_and_nested_directory", "Local library tests must cover symlinked pack and nested directory skips.");
assertContains(commandSource, "listLocalPacks", "TypeScript must wrap listLocalPacks.");
assertContains(commandSource, "inspectLocalPack", "TypeScript must wrap inspectLocalPack.");
assertContains(i18nSource, "Local Pack Library", "Exports route must become Local Pack Library.");
assertContains(i18nSource, "Refresh Library", "Local Pack Library must expose Refresh Library.");
assertContains(i18nSource, "Inspect", "Local Pack Library must expose Inspect.");
assertContains(i18nSource, "Validate Pack", "Local Pack Library must expose Validate Pack.");
assertContains(i18nSource, "Re-import Pack", "Local Pack Library must expose Re-import Pack.");
assertContains(i18nSource, "Open", "Local Pack Library must expose a pack-card Open action label.");
assertContains(appSource, "exports.library.title", "Exports route must render the translated Local Pack Library title.");
assertContains(appSource, "exports.library.refresh", "Local Pack Library must render the translated refresh action.");
assertContains(appSource, "exports.library.inspect", "Local Pack Library must render the translated inspect action.");
assertContains(appSource, "exports.library.validate", "Local Pack Library must render the translated validate action.");
assertContains(appSource, "exports.library.reimport", "Local Pack Library must render the translated re-import action.");
assertContains(appSource, "onOpenExportFolder(pack.root)", "Local Pack Library Open must target the selected pack root.");
assertContains(appSource, "setQueuedGsfpackImportPath(path)", "Re-import Pack must queue the selected pack path.");
assertContains(appSource, "queuedGsfpackImportPath", "App must pass queued .gsfpack imports into ForgeRoute.");
assertContains(forgeRouteSource, "onQueuedGsfpackImportConsumed", "ForgeRoute must consume queued .gsfpack imports.");

console.log("PASS local pack library source test");
