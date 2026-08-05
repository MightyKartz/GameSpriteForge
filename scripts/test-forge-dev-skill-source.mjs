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
  "public command name `forge`",
  "Forge project skill must preserve the public CLI command contract.",
);
assertContains(
  "/Users/kartz/Development/Forge/target/debug/forge",
  "Forge project skill must target the workspace CLI for real QA.",
);
assertContains(
  "Keep Providers locked per job",
  "Forge project skill must preserve Provider and credential boundaries.",
);
assertContains(
  "docs/qa/",
  "Forge project skill must require QA findings to be recorded under docs/qa.",
);
assertContains(
  "Godot 4.6.x",
  "Forge project skill must require the supported Godot release line.",
);
assertContains(
  "bash scripts/test-cli-product.sh",
  "Forge project skill must include the CLI product contract.",
);
assertContains(
  "bash scripts/test-cli-installer.sh",
  "Forge project skill must include the installer contract.",
);
assertContains(
  "do not add Homebrew or manual installation paths",
  "Forge project skill must preserve the single installation path.",
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
