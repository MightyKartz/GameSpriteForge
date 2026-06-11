import { spawnSync } from "node:child_process";
import { mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";

const outputPath = resolve(process.argv[2] ?? `docs/qa/screenshots/forge-ui-${Date.now()}.png`);
mkdirSync(dirname(outputPath), { recursive: true });

const targetOwner = process.env.FORGE_CAPTURE_OWNER ?? "Game Sprite Forge";

if (process.env.FORGE_CAPTURE_ACTIVATE === "1") {
  spawnSync("osascript", ["-e", `tell application "${targetOwner}" to activate`], { stdio: "ignore" });
}

const windowId = spawnSync(
  "python3",
  [
    "-c",
    [
      "import Quartz",
      "wins = Quartz.CGWindowListCopyWindowInfo(Quartz.kCGWindowListOptionOnScreenOnly, Quartz.kCGNullWindowID)",
      `target_owner = ${JSON.stringify(targetOwner)}`,
      "for w in wins:",
      "    owner = str(w.get('kCGWindowOwnerName', ''))",
      "    if owner == target_owner:",
      "        print(w.get('kCGWindowNumber'))",
      "        break",
    ].join("\n"),
  ],
  { encoding: "utf8" },
).stdout.trim();

if (!windowId) {
  process.stderr.write(`No visible ${targetOwner} window found for screenshot capture.\n`);
  process.exit(2);
}

const args = ["-x", "-l", windowId, outputPath];
const result = spawnSync("screencapture", args, { encoding: "utf8" });

if (result.status !== 0) {
  process.stderr.write(result.stderr || "screencapture failed\n");
  process.exit(result.status ?? 1);
}

console.log(`${windowId ? "window" : "screen"} screenshot: ${outputPath}`);
