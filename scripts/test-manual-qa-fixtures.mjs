#!/usr/bin/env node
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const fixtureScript = path.join(repoRoot, "scripts/prepare-manual-qa-fixtures.mjs");
const tempRoot = mkdtempSync(path.join(os.tmpdir(), "forge-manual-qa-fixtures-test."));

try {
  const outputRoot = path.join(tempRoot, "manual-qa");
  runFixture(outputRoot);
  const firstHash = hashDirectory(outputRoot);
  runFixture(outputRoot);
  const secondHash = hashDirectory(outputRoot);
  if (firstHash !== secondHash) {
    throw new Error("fixture output is not deterministic across repeated runs");
  }

  const manifest = JSON.parse(readFileSync(path.join(outputRoot, "qa-fixtures.json"), "utf8"));
  if (manifest.realShortVideo.generated !== false || manifest.realShortVideo.requiredForTask6 !== true) {
    throw new Error("fixture manifest must preserve the required real short video gate");
  }
  if (
    manifest.blockedQualitySingleFrame?.file !== manifest.pngSequence.firstFrame ||
    manifest.blockedQualitySingleFrame?.expectedVerdict !== "blocked" ||
    manifest.blockedQualitySingleFrame?.expectedExportState !== "disabled"
  ) {
    throw new Error("fixture manifest must include the single-frame blocked quality input");
  }

  expectFixtureFailure(path.join(tempRoot, "not-manual"), "--output must point to a directory named manual-qa");

  const occupiedRoot = path.join(tempRoot, "occupied", "manual-qa");
  writeFileSync(path.join(mkdirp(occupiedRoot), "README.md"), "do not delete\n", "utf8");
  expectFixtureFailure(occupiedRoot, "--output exists but does not look like generated manual QA fixtures");

  console.log("PASS manual QA fixture test");
} finally {
  rmSync(tempRoot, { recursive: true, force: true });
}

function runFixture(outputRoot) {
  execFileSync(process.execPath, [fixtureScript, "--output", outputRoot], {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: "pipe",
  });
}

function expectFixtureFailure(outputRoot, expectedText) {
  try {
    runFixture(outputRoot);
  } catch (error) {
    const output = `${error.stdout || ""}${error.stderr || ""}`;
    if (!output.includes(expectedText)) {
      throw new Error(`expected failure text ${JSON.stringify(expectedText)}, got ${JSON.stringify(output)}`);
    }
    return;
  }
  throw new Error(`expected fixture generation to fail for ${outputRoot}`);
}

function hashDirectory(root) {
  const hash = createHash("sha256");
  for (const filePath of listFiles(root)) {
    const relativePath = path.relative(root, filePath).split(path.sep).join("/");
    hash.update(relativePath);
    hash.update("\0");
    hash.update(readFileSync(filePath));
    hash.update("\0");
  }
  return hash.digest("hex");
}

function listFiles(root) {
  const entries = readdirSync(root, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const entryPath = path.join(root, entry.name);
    if (entry.isDirectory()) {
      files.push(...listFiles(entryPath));
    } else {
      files.push(entryPath);
    }
  }
  return files.sort();
}

function mkdirp(root) {
  mkdirSync(root, { recursive: true });
  return root;
}
