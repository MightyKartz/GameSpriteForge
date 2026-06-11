#!/usr/bin/env node
import { mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { deflateSync } from "node:zlib";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const args = process.argv.slice(2);

if (args.includes("--help")) {
  console.log(`Usage: node scripts/prepare-manual-qa-fixtures.mjs [--output <dir>]

Creates deterministic local manual QA fixtures for:
- valid PNG sequence import
- valid sprite sheet grid slicing
- safe failure-state checks

This script does not create the required real short video for Task 6.`);
  process.exit(0);
}

const outputFlagIndex = args.indexOf("--output");
const outputValue = outputFlagIndex >= 0 ? args[outputFlagIndex + 1] : null;
if (outputFlagIndex >= 0 && (!outputValue || outputValue.startsWith("--"))) {
  throw new Error("--output requires a directory");
}
const outputRoot = outputValue
  ? path.resolve(outputValue)
  : path.join(repoRoot, "examples/inputs/manual-qa");

const crcTable = makeCrcTable();
const green = [0, 255, 0, 255];
const transparent = [0, 0, 0, 0];

await assertSafeOutputRoot(outputRoot);
await prepareCleanOutput(outputRoot);

const pngSequenceDir = path.join(outputRoot, "png-sequence");
const spriteSheetDir = path.join(outputRoot, "sprite-sheet");
const failureDir = path.join(outputRoot, "failure-states");

const sequenceFrames = [];
for (let index = 0; index < 6; index += 1) {
  const filePath = path.join(pngSequenceDir, `frame_${String(index + 1).padStart(3, "0")}.png`);
  await writePng(filePath, walkFrameImage(96, 96, index));
  sequenceFrames.push(filePath);
}

const spriteSheetPath = path.join(spriteSheetDir, "forge-walk-sheet.png");
await writePng(spriteSheetPath, spriteSheetImage(4, 2, 64, 64));

const mixedSizeDir = path.join(failureDir, "mixed-size-sequence");
await writePng(path.join(mixedSizeDir, "frame_001.png"), walkFrameImage(96, 96, 0));
await writePng(path.join(mixedSizeDir, "frame_002.png"), walkFrameImage(80, 96, 1));
await writePng(path.join(mixedSizeDir, "frame_003.png"), walkFrameImage(96, 80, 2));

const corruptSequenceDir = path.join(failureDir, "corrupt-frame-sequence");
await writePng(path.join(corruptSequenceDir, "frame_001.png"), walkFrameImage(96, 96, 0));
await mkdir(corruptSequenceDir, { recursive: true });
await writeFile(path.join(corruptSequenceDir, "frame_002.png"), "not a png fixture\n", "utf8");

const invalidPackDir = path.join(failureDir, "invalid-pack.gsfpack");
await mkdir(invalidPackDir, { recursive: true });
await writeFile(
  path.join(invalidPackDir, "forgepack.json"),
  `${JSON.stringify({
    schemaVersion: "1.0.0",
    id: "invalid-manual-qa-pack",
    name: "Invalid Manual QA Pack",
    version: "0.1.0",
    createdAt: "2026-06-05T00:00:00Z",
    creator: { name: "Game Sprite Forge QA" },
    license: { type: "private" },
    source: { kind: "import_frames", name: "invalid fixture" },
    animations: [{ name: "walk", frames: [0], fps: 12, loop: true }],
    assets: {
      frames: "assets/frames",
      spriteSheet: "assets/sprite_sheet.png",
      atlas: "assets/atlas.json",
      manifest: "assets/manifest.json",
      qualityReport: "quality-report.json",
    },
    previews: { gif: "previews/preview.gif" },
  }, null, 2)}\n`,
  "utf8",
);

await writeFile(path.join(failureDir, "not-video.txt"), "This is not a video file.\n", "utf8");

const manifest = {
  schemaVersion: "1.0.0",
  root: toRepoRelative(outputRoot),
  realShortVideo: {
    generated: false,
    requiredForTask6: true,
    note: "Supply a tester-selected real local .mp4, .mov, or .webm. Do not treat any generated fixture as the real short video.",
  },
  pngSequence: {
    directory: toRepoRelative(pngSequenceDir),
    frameCount: sequenceFrames.length,
    width: 96,
    height: 96,
    firstFrame: toRepoRelative(sequenceFrames[0]),
    lastFrame: toRepoRelative(sequenceFrames[sequenceFrames.length - 1]),
  },
  blockedQualitySingleFrame: {
    file: toRepoRelative(sequenceFrames[0]),
    expectedVerdict: "blocked",
    expectedNote: "frame_count_below_minimum",
    expectedExportState: "disabled",
  },
  spriteSheet: {
    file: toRepoRelative(spriteSheetPath),
    columns: 4,
    rows: 2,
    cellWidth: 64,
    cellHeight: 64,
    expectedFrames: 8,
    invalidGridForFailureCheck: {
      columns: 5,
      rows: 2,
      cellWidth: 64,
      cellHeight: 64,
      expectedError: "sprite sheet grid exceeds source image bounds",
    },
  },
  failureStates: {
    mixedSizeSequence: toRepoRelative(mixedSizeDir),
    corruptFrameSequence: toRepoRelative(corruptSequenceDir),
    invalidGsfpack: toRepoRelative(invalidPackDir),
    notVideoFile: toRepoRelative(path.join(failureDir, "not-video.txt")),
  },
};

await writeFile(path.join(outputRoot, "qa-fixtures.json"), `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
await writeFile(path.join(outputRoot, "README.md"), readmeFor(manifest), "utf8");

await verifyPng(spriteSheetPath, 256, 128);
for (const frame of sequenceFrames) {
  await verifyPng(frame, 96, 96);
}
await verifyPng(path.join(mixedSizeDir, "frame_001.png"), 96, 96);
await verifyPng(path.join(mixedSizeDir, "frame_002.png"), 80, 96);
await verifyPng(path.join(mixedSizeDir, "frame_003.png"), 96, 80);

console.log(`Manual QA fixtures prepared under ${outputRoot}`);
console.log(`PNG sequence: ${pngSequenceDir} (${sequenceFrames.length} frames, 96x96)`);
console.log("Sprite sheet: " +
  `${spriteSheetPath} (columns=4 rows=2 cellWidth=64 cellHeight=64)`);
console.log(`Failure-state fixtures: ${failureDir}`);
console.log("Real short video: not generated; Task 6 still requires a tester-selected real local video.");

async function prepareCleanOutput(root) {
  await mkdir(root, { recursive: true });
  for (const relativePath of [
    "png-sequence",
    "sprite-sheet",
    "failure-states",
    "README.md",
    "qa-fixtures.json",
  ]) {
    await rm(path.join(root, relativePath), { recursive: true, force: true });
  }
}

async function assertSafeOutputRoot(root) {
  if (path.basename(root) !== "manual-qa") {
    throw new Error("--output must point to a directory named manual-qa");
  }
  if (root === repoRoot || root === path.parse(root).root) {
    throw new Error("--output cannot be the repository root or filesystem root");
  }

  let entries = [];
  try {
    entries = await readdir(root);
  } catch (error) {
    if (error.code === "ENOENT") {
      return;
    }
    throw error;
  }

  if (entries.length === 0) {
    return;
  }
  if (!entries.includes("qa-fixtures.json")) {
    throw new Error("--output exists but does not look like generated manual QA fixtures");
  }
}

function walkFrameImage(width, height, frameIndex) {
  const image = createImage(width, height, green);
  const centerX = Math.round(width / 2 + [-8, -4, 0, 4, 8, 4][frameIndex % 6]);
  const groundY = height - 10;
  drawRect(image, 6, groundY + 2, width - 12, 2, [0, 120, 0, 255]);
  drawCharacter(image, centerX, groundY, frameIndex, 1);
  return image;
}

function spriteSheetImage(columns, rows, cellWidth, cellHeight) {
  const image = createImage(columns * cellWidth, rows * cellHeight, transparent);
  for (let row = 0; row < rows; row += 1) {
    for (let column = 0; column < columns; column += 1) {
      const index = row * columns + column;
      const cellX = column * cellWidth;
      const cellY = row * cellHeight;
      drawRect(image, cellX, cellY, cellWidth, cellHeight, green);
      drawRect(image, cellX + 4, cellY + cellHeight - 8, cellWidth - 8, 2, [0, 120, 0, 255]);
      drawCharacter(image, cellX + Math.round(cellWidth / 2), cellY + cellHeight - 12, index, 0.75);
    }
  }
  return image;
}

function drawCharacter(image, centerX, groundY, frameIndex, scale) {
  const step = frameIndex % 4;
  const leftFoot = [-7, -3, 5, 2][step] * scale;
  const rightFoot = [7, 3, -5, -2][step] * scale;
  const torsoW = Math.round(18 * scale);
  const torsoH = Math.round(28 * scale);
  const headSize = Math.round(13 * scale);
  const legW = Math.max(3, Math.round(5 * scale));
  const legH = Math.round(18 * scale);
  const armW = Math.max(3, Math.round(4 * scale));
  const armH = Math.round(20 * scale);
  const bodyTop = groundY - legH - torsoH;
  const torsoX = Math.round(centerX - torsoW / 2);

  drawRect(image, Math.round(centerX - 4 * scale + leftFoot), groundY - legH, legW, legH, [35, 39, 65, 255]);
  drawRect(image, Math.round(centerX - legW + rightFoot), groundY - legH, legW, legH, [35, 39, 65, 255]);
  drawRect(image, torsoX, bodyTop, torsoW, torsoH, [54, 95, 186, 255]);
  drawRect(image, torsoX + 2, bodyTop + 4, torsoW - 4, Math.max(2, Math.round(5 * scale)), [248, 248, 255, 255]);
  drawRect(image, torsoX - armW, bodyTop + Math.round(5 * scale), armW, armH, [242, 217, 180, 255]);
  drawRect(image, torsoX + torsoW, bodyTop + Math.round(5 * scale), armW, armH, [242, 217, 180, 255]);
  drawRect(image, Math.round(centerX - headSize / 2), bodyTop - headSize + 1, headSize, headSize, [245, 210, 160, 255]);
  drawRect(image, Math.round(centerX - headSize / 2), bodyTop - headSize + 1, headSize, Math.max(3, Math.round(4 * scale)), [88, 54, 34, 255]);
  drawRect(image, Math.round(centerX - 2 * scale), bodyTop - Math.round(headSize / 2), Math.max(1, Math.round(2 * scale)), Math.max(1, Math.round(2 * scale)), [20, 20, 20, 255]);
}

function createImage(width, height, color) {
  const pixels = Buffer.alloc(width * height * 4);
  const image = { width, height, pixels };
  drawRect(image, 0, 0, width, height, color);
  return image;
}

function drawRect(image, x, y, width, height, color) {
  const startX = Math.max(0, Math.round(x));
  const startY = Math.max(0, Math.round(y));
  const endX = Math.min(image.width, Math.round(x + width));
  const endY = Math.min(image.height, Math.round(y + height));
  for (let py = startY; py < endY; py += 1) {
    for (let px = startX; px < endX; px += 1) {
      const offset = (py * image.width + px) * 4;
      image.pixels[offset] = color[0];
      image.pixels[offset + 1] = color[1];
      image.pixels[offset + 2] = color[2];
      image.pixels[offset + 3] = color[3];
    }
  }
}

async function writePng(filePath, image) {
  await mkdir(path.dirname(filePath), { recursive: true });
  await writeFile(filePath, encodePng(image));
}

function encodePng(image) {
  const stride = image.width * 4;
  const raw = Buffer.alloc((stride + 1) * image.height);
  for (let y = 0; y < image.height; y += 1) {
    const rawOffset = y * (stride + 1);
    raw[rawOffset] = 0;
    image.pixels.copy(raw, rawOffset + 1, y * stride, y * stride + stride);
  }

  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(image.width, 0);
  ihdr.writeUInt32BE(image.height, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  ihdr[10] = 0;
  ihdr[11] = 0;
  ihdr[12] = 0;

  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

function chunk(type, data) {
  const typeBuffer = Buffer.from(type, "ascii");
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length, 0);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuffer, data])), 0);
  return Buffer.concat([length, typeBuffer, data, crc]);
}

function makeCrcTable() {
  const table = new Uint32Array(256);
  for (let index = 0; index < 256; index += 1) {
    let value = index;
    for (let bit = 0; bit < 8; bit += 1) {
      value = value & 1 ? 0xedb88320 ^ (value >>> 1) : value >>> 1;
    }
    table[index] = value >>> 0;
  }
  return table;
}

function crc32(buffer) {
  let value = 0xffffffff;
  for (const byte of buffer) {
    value = crcTable[(value ^ byte) & 0xff] ^ (value >>> 8);
  }
  return (value ^ 0xffffffff) >>> 0;
}

async function verifyPng(filePath, expectedWidth, expectedHeight) {
  const buffer = await readFile(filePath);
  if (
    buffer[0] !== 137 ||
    buffer[1] !== 80 ||
    buffer[2] !== 78 ||
    buffer[3] !== 71
  ) {
    throw new Error(`not a PNG: ${filePath}`);
  }
  const width = buffer.readUInt32BE(16);
  const height = buffer.readUInt32BE(20);
  if (width !== expectedWidth || height !== expectedHeight) {
    throw new Error(`unexpected dimensions for ${filePath}: ${width}x${height}`);
  }
}

function toRepoRelative(filePath) {
  return path.relative(repoRoot, filePath).split(path.sep).join("/");
}

function readmeFor(manifest) {
  return `# Manual QA Fixtures

Generated by \`node scripts/prepare-manual-qa-fixtures.mjs\`.

These fixtures prepare Task 6 app QA for PNG sequence, sprite sheet, and safe failure-state checks. They do not replace the required tester-selected real short video.

## Valid Inputs

PNG sequence:
\`${manifest.pngSequence.directory}\`

Sprite sheet:
\`${manifest.spriteSheet.file}\`

Sprite sheet grid:

\`\`\`text
columns: ${manifest.spriteSheet.columns}
rows: ${manifest.spriteSheet.rows}
cell width: ${manifest.spriteSheet.cellWidth}
cell height: ${manifest.spriteSheet.cellHeight}
expected frames: ${manifest.spriteSheet.expectedFrames}
\`\`\`

## Failure-State Inputs

\`\`\`text
mixed-size sequence: ${manifest.failureStates.mixedSizeSequence}
corrupt frame sequence: ${manifest.failureStates.corruptFrameSequence}
invalid .gsfpack: ${manifest.failureStates.invalidGsfpack}
not-video file: ${manifest.failureStates.notVideoFile}
blocked-quality single frame: ${manifest.blockedQualitySingleFrame.file}
invalid sprite sheet grid: columns=5 rows=2 cellWidth=64 cellHeight=64
\`\`\`

These fixtures alone do not close Task 6. Use the manual QA checklist for current app-session evidence, including the required real local video.
`;
}
