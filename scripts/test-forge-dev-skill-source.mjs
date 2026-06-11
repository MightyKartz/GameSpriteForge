import { readFileSync } from "node:fs";

const skillPath = ".agents/skills/forge-dev/SKILL.md";
const skillSource = readFileSync(skillPath, "utf8");

function assertContains(needle, message) {
  if (!skillSource.includes(needle)) {
    throw new Error(message);
  }
}

function assertMatches(pattern, message) {
  if (!pattern.test(skillSource)) {
    throw new Error(message);
  }
}

assertContains("---\nname: forge-dev", "Forge project skill must use the forge-dev name.");
assertContains(
  "Use when working on Forge",
  "Forge project skill description must clearly trigger for Forge development.",
);
assertMatches(
  /description: .{1,500}\n---/,
  "Forge project skill description must stay concise enough for discovery.",
);
assertContains(
  "Do not copy external project source",
  "Forge project skill must preserve the external source boundary.",
);
assertContains(
  "choose file, configure `固定网格` or `透明间隔`, then import",
  "Forge project skill must preserve the sprite sheet intake flow.",
);
assertContains(
  "/Users/kartz/Development/Forge/target/debug/bundle/macos/Game Sprite Forge.app",
  "Forge project skill must tell agents to target the workspace debug bundle for real UI QA.",
);
assertContains(
  "stale `/Applications/Game Sprite Forge.app`",
  "Forge project skill must warn about stale installed apps with the same bundle id.",
);
assertContains(
  "docs/qa/",
  "Forge project skill must require QA findings to be recorded under docs/qa.",
);
assertContains(
  "docs/qa/artifacts/",
  "Forge project skill must keep QA artifacts in the project QA artifacts directory.",
);
assertContains(
  "npm --workspace apps/mac run build",
  "Forge project skill must include the frontend build verification command.",
);
assertContains(
  "npm run test:scripts",
  "Forge project skill must include the project source guard verification command.",
);
assertContains(
  "npm --workspace apps/mac run smoke:ui:mvp",
  "Forge project skill must include the MVP UI smoke verification command.",
);
assertContains(
  "cargo fmt --manifest-path /Users/kartz/Development/Forge/Cargo.toml --all -- --check",
  "Forge project skill must include the Rust formatting verification command.",
);
assertContains(
  "cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml sprite_sheet_transparent",
  "Forge project skill must include the focused transparent sprite sheet Rust test.",
);

console.log("PASS forge dev skill source test");
