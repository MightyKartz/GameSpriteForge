import { existsSync, readFileSync } from "node:fs";

function readRequired(path) {
  if (!existsSync(path)) {
    throw new Error(`missing required Godot helper sample file: ${path}`);
  }
  return readFileSync(path, "utf8");
}

function assertContains(source, needle, message) {
  if (!source.includes(needle)) {
    throw new Error(message);
  }
}

const verifierSource = readRequired("scripts/verify-godot-helper-sample.mjs");
const projectSource = readRequired("examples/godot/forge-import-smoke/project.godot");
const godotScriptSource = readRequired("examples/godot/forge-import-smoke/verify_forge_helper.gd");
const evidenceSource = readRequired("docs/qa/godot-editor-helper-evidence-2026-06-06.md");
const packageSource = readRequired("package.json");

assertContains(
  verifierSource,
  "GODOT_HELPER_SAMPLE_PACK",
  "Godot helper verifier must support overriding the sample .gsfpack path.",
);
assertContains(
  verifierSource,
  "/Applications/Godot.app/Contents/MacOS/Godot",
  "Godot helper verifier must use the installed Godot app on this Mac when no CLI is on PATH.",
);
assertContains(
  projectSource,
  'config/name="Forge Godot Helper Smoke"',
  "Godot sample project must have a stable project name.",
);
assertContains(
  godotScriptSource,
  "OS.get_cmdline_user_args()",
  "Godot sample script must accept a pack path from the verifier.",
);
assertContains(
  evidenceSource,
  "Godot editor/sample-project validation",
  "Godot evidence doc must record editor/sample-project validation.",
);
assertContains(
  packageSource,
  "test-godot-helper-sample-source.mjs",
  "package.json test:scripts must include the Godot helper sample source guard.",
);

console.log("PASS Godot helper sample source test");
