import { readFileSync } from "node:fs";

const packageJson = readFileSync("package.json", "utf8");
const previewSource = readFileSync("apps/mac/src/components/CanvasPreview.tsx", "utf8");
const routeSource = readFileSync("apps/mac/src/routes/ForgeRoute.tsx", "utf8");
const cssSource = readFileSync("apps/mac/src/styles/app.css", "utf8");

function assertContains(source, needle, message) {
  if (!source.includes(needle)) {
    throw new Error(message);
  }
}

assertContains(previewSource, "FootAnchor, FrameBbox, FrameSize", "CanvasPreview must type overlay coordinates from normalized frame data.");
assertContains(previewSource, "preview-frame-stage", "CanvasPreview must wrap image and overlays in a shared coordinate stage.");
assertContains(previewSource, "preview-measure-overlays", "CanvasPreview must render measurement overlays.");
assertContains(previewSource, "preview-bbox", "CanvasPreview must render bbox overlay.");
assertContains(previewSource, "preview-anchor-line", "CanvasPreview must render anchor line overlay.");
assertContains(previewSource, "preview-center-line", "CanvasPreview must render center line overlay.");
assertContains(routeSource, "selectedOverlay", "ForgeRoute must derive selected frame overlay data.");
assertContains(routeSource, "overlay={selectedOverlay}", "ForgeRoute must pass selected overlay to CanvasPreview.");
assertContains(cssSource, ".preview-frame-stage", "CSS must style the shared image coordinate stage.");
assertContains(cssSource, "--preview-aspect", "CSS must keep overlay aspect tied to frame size.");
assertContains(cssSource, ".preview-measure-overlays", "CSS must position measurement overlays.");
assertContains(cssSource, ".preview-bbox", "CSS must style bbox overlay.");
assertContains(packageJson, "scripts/test-preview-overlay-source.mjs", "test:scripts must include preview overlay source guard.");

console.log("PASS preview overlay source test");
