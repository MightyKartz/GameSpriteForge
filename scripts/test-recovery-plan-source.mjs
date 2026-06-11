import { readFileSync } from "node:fs";

const source = readFileSync("apps/mac/src/routes/ForgeRoute.tsx", "utf8");
const i18nSource = readFileSync("apps/mac/src/i18n.ts", "utf8");

function assertContains(needle, message) {
  if (!source.includes(needle)) {
    throw new Error(message);
  }
}

function assertBefore(first, second, message) {
  const firstIndex = source.indexOf(first);
  const secondIndex = source.indexOf(second);
  if (firstIndex === -1 || secondIndex === -1 || firstIndex >= secondIndex) {
    throw new Error(message);
  }
}

assertContains(
  "invalid data found when processing input",
  "Recovery routing must detect ffmpeg invalid-video input errors.",
);
assertContains(
  "recovery.videoPath.detail",
  "Invalid-video recovery must send the user back to source selection.",
);
if (!i18nSource.includes("Choose a valid local video source, then import it again.")) {
  throw new Error("Invalid-video recovery detail must remain in the English dictionary.");
}
assertBefore(
  "invalid data found when processing input",
  "/(ffmpeg|ffprobe|toolchain)/",
  "Invalid-video recovery must run before generic ffmpeg/toolchain recovery.",
);
assertContains(
  "recovery.sheetTooLarge.title",
  "Oversized sprite-sheet recovery must use the texture-size title.",
);
assertContains(
  "recovery.sheetTooLarge.detail",
  "Oversized sprite-sheet recovery must explain splitting and sheet sizing options.",
);
if (!i18nSource.includes("Sprite sheet is too large")) {
  throw new Error("Oversized sprite-sheet recovery title must remain in the English dictionary.");
}
assertBefore(
  "exceeds max texture size",
  "/(sprite sheet|grid|slice|cell|atlas)/",
  "Oversized sprite-sheet recovery must run before generic sprite sheet recovery.",
);

console.log("PASS recovery plan source test");
