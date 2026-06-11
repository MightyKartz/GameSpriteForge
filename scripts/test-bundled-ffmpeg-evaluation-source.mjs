import { existsSync, readFileSync } from "node:fs";

function readRequired(path) {
  if (!existsSync(path)) {
    throw new Error(`missing required bundled ffmpeg evaluation file: ${path}`);
  }
  return readFileSync(path, "utf8");
}

function assertContains(source, needle, message) {
  if (!source.includes(needle)) {
    throw new Error(message);
  }
}

const verifierSource = readRequired("scripts/verify-ffmpeg-bundle-evaluation.mjs");
const decisionSource = readRequired("docs/architecture/bundled-ffmpeg-evaluation.md");
const qaSource = readRequired("docs/qa/ffmpeg-bundle-evaluation-2026-06-06.md");
const distributionSource = readRequired("docs/architecture/distribution-mvp.md");
const packageSource = readRequired("package.json");

assertContains(
  verifierSource,
  "--enable-gpl",
  "ffmpeg bundle verifier must detect GPL-enabled builds.",
);
assertContains(
  verifierSource,
  "--enable-nonfree",
  "ffmpeg bundle verifier must detect nonfree builds.",
);
assertContains(
  verifierSource,
  "/Applications/Game Sprite Forge.app",
  "ffmpeg bundle verifier must inspect the installed app bundle.",
);
assertContains(
  verifierSource,
  "target/release/bundle/macos/Game Sprite Forge.app",
  "ffmpeg bundle verifier must inspect the release app bundle.",
);
assertContains(
  decisionSource,
  "Current decision: keep user-configured/PATH-only",
  "bundled ffmpeg decision doc must record the current decision.",
);
assertContains(
  decisionSource,
  "Homebrew ffmpeg 8.0.1 is GPL-enabled",
  "bundled ffmpeg decision doc must record the local GPL-enabled finding.",
);
assertContains(
  qaSource,
  "Computer Use",
  "ffmpeg bundle QA doc must include real UI evidence.",
);
assertContains(
  distributionSource,
  "Bundled FFmpeg Evaluation - 2026-06-06",
  "distribution MVP doc must link the bundled ffmpeg evaluation.",
);
assertContains(
  packageSource,
  "test-bundled-ffmpeg-evaluation-source.mjs",
  "package.json test:scripts must include the bundled ffmpeg evaluation source guard.",
);

console.log("PASS bundled ffmpeg evaluation source test");
