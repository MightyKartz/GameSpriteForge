import { readFileSync } from "node:fs";

const ffmpegSource = readFileSync("packages/core/src/video/ffmpeg.rs", "utf8");
const tauriSource = readFileSync("apps/mac/src-tauri/src/lib.rs", "utf8");

function assertContains(source, needle, message) {
  if (!source.includes(needle)) {
    throw new Error(message);
  }
}

assertContains(
  ffmpegSource,
  'env::var_os("GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS")',
  "ffmpeg resolver must support deterministic search directories for clean-environment QA.",
);
assertContains(
  ffmpegSource,
  'env_flag_enabled("GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS")',
  "ffmpeg resolver must support disabling default macOS tool directories for CE-003.",
);
assertContains(
  ffmpegSource,
  "if !disable_macos_default_tool_dirs()",
  "ffmpeg resolver must gate default macOS tool directories behind the disable flag.",
);
assertContains(
  tauriSource,
  "find_in_path(expected_name)",
  "Tauri dependency check must continue to use the shared ffmpeg resolver path.",
);

console.log("PASS ffmpeg resolver source test");
