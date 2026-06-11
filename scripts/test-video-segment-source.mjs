import { readFileSync } from "node:fs";

const packageJson = readFileSync("package.json", "utf8");
const helperSource = readFileSync("apps/mac/src/videoSegment.ts", "utf8");
const panelSource = readFileSync("apps/mac/src/components/VideoSegmentPanel.tsx", "utf8");
const routeSource = readFileSync("apps/mac/src/routes/ForgeRoute.tsx", "utf8");
const i18nSource = readFileSync("apps/mac/src/i18n.ts", "utf8");

function assertContains(source, needle, message) {
  if (!source.includes(needle)) {
    throw new Error(message);
  }
}

assertContains(helperSource, "export type FrameSelectionMode", "videoSegment helper must expose frame selection mode.");
assertContains(helperSource, "keepEveryForTargetFrames", "videoSegment helper must derive keep-every interval from target frames.");
assertContains(helperSource, "estimateSelectedFrames", "videoSegment helper must keep frame count estimation testable.");
assertContains(panelSource, "segment-target-controls", "VideoSegmentPanel must render target frame controls.");
assertContains(panelSource, "manual_interval", "VideoSegmentPanel must preserve manual interval mode.");
assertContains(panelSource, "segment.extractionRationaleTarget", "VideoSegmentPanel must explain target frame extraction.");
assertContains(routeSource, "targetFrameCount", "ForgeRoute must own target frame count state.");
assertContains(routeSource, "frameSelectionMode", "ForgeRoute must own frame selection mode state.");
assertContains(routeSource, "activeKeepEveryNFrames", "ForgeRoute must pass the effective interval to extraction.");
assertContains(routeSource, "keepEveryForTargetFrames", "ForgeRoute must derive target-frame extraction interval.");
assertContains(i18nSource, "segment.targetFramesValue", "i18n must include target frame copy.");
assertContains(i18nSource, "目标 {target} 帧", "Chinese i18n must describe target frame extraction.");
assertContains(packageJson, "scripts/test-video-segment-source.mjs", "test:scripts must include video segment source guard.");

console.log("PASS video segment source test");
