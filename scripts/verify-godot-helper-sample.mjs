import { accessSync, constants, existsSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { delimiter, join, resolve } from "node:path";

const defaultPackPath =
  "/Users/kartz/Game Sprite Forge/Exports/Hero-Knight-Pack-1780644870371/Hero-Knight-Pack.gsfpack";
const projectPath = resolve("examples/godot/forge-import-smoke");
const packPath = process.env.GODOT_HELPER_SAMPLE_PACK || defaultPackPath;

const candidateBins = [
  process.env.GODOT_BIN,
  "godot",
  "godot4",
  "/Applications/Godot.app/Contents/MacOS/Godot",
].filter(Boolean);

function findGodotBin() {
  for (const candidate of candidateBins) {
    if (candidate.includes("/")) {
      try {
        accessSync(candidate, constants.X_OK);
        return candidate;
      } catch {
        continue;
      }
    }

    for (const directory of (process.env.PATH || "").split(delimiter)) {
      if (!directory) {
        continue;
      }
      const resolved = join(directory, candidate);
      try {
        accessSync(resolved, constants.X_OK);
        return resolved;
      } catch {
        // Keep checking later PATH entries.
      }
    }
  }
  return null;
}

if (!existsSync(packPath)) {
  throw new Error(`Godot helper sample pack does not exist: ${packPath}`);
}

if (!existsSync(projectPath)) {
  throw new Error(`Godot helper sample project does not exist: ${projectPath}`);
}

const godotBin = findGodotBin();
if (!godotBin) {
  throw new Error(
    "Godot CLI unavailable. Install Godot or set GODOT_BIN to run editor/sample-project validation.",
  );
}

const version = spawnSync(godotBin, ["--version"], { encoding: "utf8" });
const versionText = `${version.stdout}${version.stderr}`.trim();

const result = spawnSync(
  godotBin,
  [
    "--headless",
    "--path",
    projectPath,
    "--script",
    "res://verify_forge_helper.gd",
    "--",
    packPath,
  ],
  { encoding: "utf8" },
);

process.stdout.write(result.stdout);
process.stderr.write(result.stderr);

if (result.status !== 0) {
  throw new Error(`Godot helper sample validation failed with status ${result.status}`);
}

console.log(`PASS Godot helper sample validation with ${versionText}`);
