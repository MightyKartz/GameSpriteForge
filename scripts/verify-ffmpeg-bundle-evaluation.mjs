import { accessSync, constants, existsSync, readdirSync, statSync } from "node:fs";
import { delimiter, join } from "node:path";
import { spawnSync } from "node:child_process";

const explicitFfmpeg = process.env.FFMPEG_BIN;
const explicitFfprobe = process.env.FFPROBE_BIN;
const installedApp = "/Applications/Game Sprite Forge.app";
const releaseApp = "target/release/bundle/macos/Game Sprite Forge.app";
const appBundleCandidates = [installedApp, releaseApp];
const licenseFlags = ["--enable-gpl", "--enable-nonfree"];

function findExecutable(name, explicitPath) {
  const candidates = [
    explicitPath,
    ...((process.env.PATH || "")
      .split(delimiter)
      .filter(Boolean)
      .map((directory) => join(directory, name))),
    `/opt/homebrew/bin/${name}`,
    `/usr/local/bin/${name}`,
  ].filter(Boolean);

  for (const candidate of candidates) {
    try {
      accessSync(candidate, constants.X_OK);
      return candidate;
    } catch {
      // Keep looking.
    }
  }
  throw new Error(`Could not find executable ${name}. Set ${name.toUpperCase()}_BIN to evaluate a specific binary.`);
}

function commandOutput(binary, args) {
  const result = spawnSync(binary, args, { encoding: "utf8" });
  const output = `${result.stdout}${result.stderr}`;
  if (result.status !== 0) {
    throw new Error(`${binary} ${args.join(" ")} failed:\n${output}`);
  }
  return output;
}

function parseVersion(output) {
  return output.split("\n").find((line) => line.includes(" version "))?.trim() || "unknown";
}

function parseConfiguration(output) {
  return output.split("\n").find((line) => line.startsWith("configuration: ")) || "";
}

function findBundledTools(root) {
  if (!existsSync(root)) {
    return [];
  }
  const found = [];
  const queue = [{ path: root, depth: 0 }];
  while (queue.length > 0) {
    const current = queue.shift();
    if (!current || current.depth > 5) {
      continue;
    }
    for (const entry of readdirSync(current.path, { withFileTypes: true })) {
      const path = join(current.path, entry.name);
      if (entry.isDirectory()) {
        queue.push({ path, depth: current.depth + 1 });
      } else if (entry.isFile() && (entry.name === "ffmpeg" || entry.name === "ffprobe")) {
        found.push(path);
      }
    }
  }
  return found;
}

function appBundleSummary() {
  return appBundleCandidates.map((bundle) => {
    const tools = findBundledTools(bundle);
    const exists = existsSync(bundle) && statSync(bundle).isDirectory();
    return { bundle, exists, tools };
  });
}

const ffmpeg = findExecutable("ffmpeg", explicitFfmpeg);
const ffprobe = findExecutable("ffprobe", explicitFfprobe);
const ffmpegVersionOutput = commandOutput(ffmpeg, ["-version"]);
const ffprobeVersionOutput = commandOutput(ffprobe, ["-version"]);
const configuration = parseConfiguration(ffmpegVersionOutput);
const enabledFlags = licenseFlags.filter((flag) => configuration.includes(flag));
const bundleSummary = appBundleSummary();
const bundledTools = bundleSummary.flatMap((summary) => summary.tools);

console.log("== Game Sprite Forge Bundled FFmpeg Evaluation ==");
console.log(`ffmpeg: ${ffmpeg}`);
console.log(`ffprobe: ${ffprobe}`);
console.log(`ffmpeg version: ${parseVersion(ffmpegVersionOutput)}`);
console.log(`ffprobe version: ${parseVersion(ffprobeVersionOutput)}`);
console.log(`configuration: ${configuration}`);
console.log(`license flags: ${enabledFlags.length ? enabledFlags.join(", ") : "none detected"}`);
for (const summary of bundleSummary) {
  console.log(
    `app bundle: ${summary.bundle} ${summary.exists ? "exists" : "missing"}; bundled tools: ${
      summary.tools.length ? summary.tools.join(", ") : "none"
    }`,
  );
}

if (bundledTools.length > 0) {
  throw new Error(`Current app bundle unexpectedly contains ffmpeg tools: ${bundledTools.join(", ")}`);
}

if (enabledFlags.includes("--enable-nonfree")) {
  console.log("Current decision: do not bundle this ffmpeg build because --enable-nonfree is present.");
} else if (enabledFlags.includes("--enable-gpl")) {
  console.log("Current decision: keep user-configured/PATH-only; the local Homebrew ffmpeg build is GPL-enabled.");
} else {
  console.log("Current decision: permissive LGPL-style candidate not found in this run; keep user-configured/PATH-only until a dedicated bundle build is produced and signed.");
}

console.log("PASS bundled ffmpeg evaluation completed");
