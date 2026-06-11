import { mkdir, mkdtemp, readdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { spawn } from "node:child_process";

const root = path.resolve(import.meta.dirname, "..");
const outputDir = path.join(root, "smoke-output");
const url = "http://127.0.0.1:1420";
const smokeLocale = process.env.FORGE_SMOKE_LOCALE === "zh-CN" ? "zh-CN" : "en-US";
const targetUrl = `${url}/?forgeSmokeLocale=${encodeURIComponent(smokeLocale)}`;
const chromePath =
  process.env.CHROME_PATH || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const mode = process.argv.includes("--mode=mvp")
  ? "mvp"
  : process.argv.includes("--mode=responsive")
    ? "responsive"
    : "reference";
const staleBuildMessage = "Build output is stale; run npm --workspace apps/mac run build before smoke:ui:mvp";

await assertBuildOutputFresh();
await mkdir(outputDir, { recursive: true });

const preview = startPreview();

try {
  await waitForHttp(targetUrl, 15_000);

  const userDataDir = await mkdtemp(path.join(tmpdir(), "forge-chrome-"));
  const screenshotPath = path.join(
    outputDir,
    mode === "mvp"
      ? `forge-workbench-mvp-${smokeLocale}.png`
      : mode === "responsive"
        ? `forge-workbench-responsive-1280-${smokeLocale}.png`
        : `forge-workbench-1568-${smokeLocale}.png`,
  );
  const sourcePath = path.join(
    outputDir,
    mode === "mvp" ? `forge-workbench-mvp-source-${smokeLocale}.txt` : `forge-workbench-source-${smokeLocale}.txt`,
  );
  const visibleTextPath = path.join(
    outputDir,
    mode === "mvp" ? `forge-workbench-mvp-visible-text-${smokeLocale}.txt` : `forge-workbench-visible-text-${smokeLocale}.txt`,
  );
  const source = await readBuiltSource();
  await writeFile(sourcePath, source);

  if (mode === "responsive") {
    await captureResponsiveScreenshots(userDataDir);
  } else {
    await runChrome(
      [
        "--headless=new",
        "--disable-gpu",
        "--no-first-run",
        `--user-data-dir=${userDataDir}`,
        "--window-size=1568,1003",
        `--screenshot=${screenshotPath}`,
        targetUrl,
      ],
      { timeoutOkFile: screenshotPath },
    );
  }

  const visibleText = await dumpVisibleText(userDataDir);
  await writeFile(visibleTextPath, visibleText);

  await rm(userDataDir, { recursive: true, force: true });
  await assertScreenshot(screenshotPath);
  assertSource(source, mode, smokeLocale);
  assertVisibleText(visibleText, mode, smokeLocale);

  console.log(`UI smoke passed (${mode}, ${smokeLocale}). Screenshot: ${screenshotPath}`);
  console.log(`Source dump: ${sourcePath}`);
  console.log(`Visible text dump: ${visibleTextPath}`);
} finally {
  stopPreview(preview);
}

function startPreview() {
  const previewProcess = spawn(
    "npx",
    ["vite", "preview", "--host", "127.0.0.1", "--port", "1420", "--strictPort"],
    {
      cwd: root,
      detached: true,
      stdio: "ignore",
    },
  );

  previewProcess.unref();
  return previewProcess;
}

async function assertBuildOutputFresh() {
  const distIndexPath = path.join(root, "dist", "index.html");
  const distAssetsPath = path.join(root, "dist", "assets");
  const sourceFiles = [
    ...(await collectFiles(path.join(root, "src"), (file) => {
      const extension = path.extname(file);
      return extension === ".ts" || extension === ".tsx" || extension === ".css";
    })),
    ...[
      "index.html",
      "package.json",
      "tsconfig.json",
      "tsconfig.node.json",
      "vite.config.ts",
    ]
      .map((file) => path.join(root, file))
      .filter((file) => existsSync(file)),
  ];
  if (!(await isFile(distIndexPath)) || !(await isDirectory(distAssetsPath))) {
    throw new Error(staleBuildMessage);
  }

  const assetFiles = await collectFiles(distAssetsPath, () => true);
  if (!assetFiles.length) {
    throw new Error(staleBuildMessage);
  }

  const builtFiles = [distIndexPath, ...assetFiles];

  if (!sourceFiles.length || !builtFiles.length) {
    throw new Error(staleBuildMessage);
  }

  const newestSourceMtime = await newestMtime(sourceFiles);
  const newestBuildMtime = await newestMtime(builtFiles);
  if (newestBuildMtime < newestSourceMtime) {
    throw new Error(staleBuildMessage);
  }
}

async function isFile(file) {
  try {
    const info = await stat(file);
    return info.isFile();
  } catch {
    return false;
  }
}

async function isDirectory(directory) {
  try {
    const info = await stat(directory);
    return info.isDirectory();
  } catch {
    return false;
  }
}

async function collectFiles(directory, includeFile) {
  if (!existsSync(directory)) {
    return [];
  }

  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await collectFiles(entryPath, includeFile)));
    } else if (entry.isFile() && includeFile(entryPath)) {
      files.push(entryPath);
    }
  }
  return files;
}

async function newestMtime(files) {
  let newest = 0;
  for (const file of files) {
    const info = await stat(file);
    newest = Math.max(newest, info.mtimeMs);
  }
  return newest;
}

async function waitForHttp(targetUrl, timeoutMs) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    try {
      const response = await fetch(targetUrl);
      if (response.ok) {
        return;
      }
    } catch {
      // Preview is still starting.
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }

  throw new Error(`Vite preview did not become ready at ${targetUrl}`);
}

function runChrome(args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(chromePath, args, {
      cwd: root,
      detached: true,
      stdio: "ignore",
    });
    child.unref();

    let settled = false;
    const settle = (callback) => {
      if (settled) {
        return;
      }
      settled = true;
      clearTimeout(timeout);
      child.removeAllListeners();
      callback();
    };
    const timeout = setTimeout(() => {
      killProcessGroup(child.pid, "SIGKILL");
      if (options.timeoutOkFile && existsSync(options.timeoutOkFile)) {
        settle(resolve);
      } else {
        settle(() => reject(new Error("Chrome timed out before writing smoke screenshot")));
      }
    }, 20_000);
    child.on("exit", (code) => {
      if (code === 0) {
        settle(resolve);
      } else {
        settle(() => reject(new Error(`Chrome exited ${code}`)));
      }
    });
  });
}

function stopPreview(previewProcess) {
  killProcessGroup(previewProcess.pid, "SIGTERM");
}

function killProcessGroup(pid, signal) {
  try {
    process.kill(-pid, signal);
  } catch {
    try {
      process.kill(pid, signal);
    } catch {
      // Already stopped.
    }
  }
}

async function readBuiltSource() {
  const dist = path.join(root, "dist");
  const assets = path.join(dist, "assets");
  const files = await readdir(assets);
  const jsFiles = files.filter((file) => file.endsWith(".js"));
  const cssFiles = files.filter((file) => file.endsWith(".css"));
  const html = await readFile(path.join(dist, "index.html"), "utf8");
  const assetsSource = await Promise.all(
    [...jsFiles, ...cssFiles].map((file) => readFile(path.join(assets, file), "utf8")),
  );
  return [html, ...assetsSource].join("\n");
}

async function captureResponsiveScreenshots(userDataDir) {
  const viewports = [
    { width: 1120, height: 760 },
    { width: 1280, height: 820 },
    { width: 1568, height: 1003 },
  ];
  for (const viewport of viewports) {
    const screenshotPath = path.join(
      outputDir,
      `forge-workbench-responsive-${viewport.width}-${smokeLocale}.png`,
    );
    await runChrome(
      [
        "--headless=new",
        "--disable-gpu",
        "--no-first-run",
        `--user-data-dir=${userDataDir}`,
        `--window-size=${viewport.width},${viewport.height}`,
        `--screenshot=${screenshotPath}`,
        targetUrl,
      ],
      { timeoutOkFile: screenshotPath },
    );
    await assertScreenshot(screenshotPath);
  }
}

async function assertScreenshot(screenshotPath) {
  const info = await stat(screenshotPath);
  if (info.size < 50_000) {
    throw new Error(`UI smoke screenshot is unexpectedly small: ${info.size} bytes`);
  }
}

async function dumpVisibleText(userDataDir) {
  const port = 42000 + Math.floor(Math.random() * 1000);
  const child = spawn(
    chromePath,
    [
      "--headless=new",
      "--disable-gpu",
      "--no-first-run",
      `--remote-debugging-port=${port}`,
      `--user-data-dir=${userDataDir}`,
      "--window-size=1568,1003",
      targetUrl,
    ],
    {
      cwd: root,
      stdio: "ignore",
    },
  );

  try {
    const wsUrl = await waitForDebuggerUrl(port, 20_000);
    const client = await createCdpClient(wsUrl);
    try {
      const text = await waitForRuntimeText(client, 20_000);
      return text.replace(/\s+/g, " ").trim();
    } finally {
      client.close();
    }
  } finally {
    child.kill("SIGKILL");
  }
}

async function waitForDebuggerUrl(port, timeoutMs) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/json/list`);
      if (response.ok) {
        const pages = await response.json();
        const page = pages.find((item) => item.type === "page" && item.webSocketDebuggerUrl);
        if (page) {
          return page.webSocketDebuggerUrl;
        }
      }
    } catch {
      // Chrome is still starting.
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error("Chrome DevTools endpoint did not become ready for visible text dump");
}

function createCdpClient(wsUrl) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(wsUrl);
    let nextId = 1;
    const pending = new Map();
    const timeout = setTimeout(() => reject(new Error("Timed out connecting to Chrome DevTools")), 10_000);

    ws.addEventListener("open", () => {
      clearTimeout(timeout);
      resolve({
        close: () => ws.close(),
        send(method, params = {}) {
          const id = nextId;
          nextId += 1;
          ws.send(JSON.stringify({ id, method, params }));
          return new Promise((innerResolve, innerReject) => {
            pending.set(id, { reject: innerReject, resolve: innerResolve });
          });
        },
      });
    });
    ws.addEventListener("message", (event) => {
      const message = JSON.parse(event.data);
      if (!message.id || !pending.has(message.id)) {
        return;
      }
      const callbacks = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) {
        callbacks.reject(new Error(message.error.message));
      } else {
        callbacks.resolve(message.result);
      }
    });
    ws.addEventListener("error", () => {
      clearTimeout(timeout);
      reject(new Error("Chrome DevTools WebSocket failed"));
    });
  });
}

async function waitForRuntimeText(client, timeoutMs) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    const result = await client.send("Runtime.evaluate", {
      expression: "document.body ? document.body.innerText : ''",
      returnByValue: true,
    });
    const text = result.result?.value ?? "";
    if (text.includes("Game Sprite Forge") && text.length > 100) {
      return text;
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error("Runtime visible text did not become ready");
}

function assertVisibleText(text, smokeMode, locale) {
  if (locale !== "zh-CN") {
    return;
  }

  const required = [
    "本地工作台",
    "当前来源",
    "未选择来源",
    "帧时间线",
    "精灵表预览",
    "当前还没有载入项目",
    "导入视频生成精灵素材",
    "选择视频文件",
    "导入 PNG 序列",
    "运行内置示例",
    "暂无资源",
    "暂无帧",
    "当前步骤",
    "选择来源",
    "阶段",
    "处理帧",
    "上一帧",
    "下一帧",
    "检查待处理",
  ];
  const forbidden = [
    "Current Source",
    "No source selected",
    "Frame Timeline",
    "Sprite Sheet Preview",
    "Sample Assets",
    "Processed frames",
    "Output folder",
    "Previous Frame",
    "Next Frame",
    "Checks pending",
    "Import workspace",
    "Frame inspection",
    "Processed pixels",
  ];

  for (const value of required) {
    if (!text.includes(value)) {
      throw new Error(`zh-CN visible text is missing required copy: ${value}`);
    }
  }
  for (const value of forbidden) {
    if (text.includes(value)) {
      throw new Error(`zh-CN visible text still contains English UI copy: ${value}`);
    }
  }

  if (smokeMode === "mvp" && /\.GSFPACK/.test(text)) {
    throw new Error("zh-CN visible text uppercased .gsfpack; token casing must be preserved.");
  }
}

function assertSource(source, smokeMode, locale) {
  const required = [
    "Game Sprite Forge",
    "Green Box Character Pack",
    "Forge",
    "Exports",
    "Settings",
    "Frames",
    "Import Video",
    "Import PNG Sequence",
    "Import Sprite Sheet",
    "Import .gsfpack",
    "Quality Report",
    "Sprite Sheet Preview",
    "Export Pack",
    ".gsfpack",
  ];
  const mvpRequired = [
    "IMPORT-ONLY MVP",
    "Quality status",
    "Workflow step locked",
    "Check FFmpeg",
    "Extract Frames",
    "Process & Quality",
    "No quality report yet",
    "Resolve the missing items above to enable export",
    "Export readiness",
    "Missing before export",
    "Processed frames",
    "Output folder",
    "Pack metadata and sheet settings",
    "Checks after processing",
    "Process frames from Quality Report",
    "Run Sample Pipeline",
    "Use the center launcher to import your source. The sample is optional.",
    "Import Video To Create Sprite Assets",
    "Choose Video File",
    "Export unlocks after import",
    "Empty workspace",
    "No source loaded",
    "No assets yet",
    "No frames yet",
    "Keyboard Shortcuts",
    "Cmd+Enter Process",
    "Cmd+E Export",
    "Checks pending",
    "Frames: none",
    "Live workspace",
    "Pipeline Steps",
    "Local Pack Library",
    "Validate, inspect, open, and re-import local .gsfpack exports",
    "library-status",
    "Refresh Library",
    "Validate Pack",
    "Re-import Pack",
    "No local packs found",
    "Choose File",
    "Choose ffmpeg",
    "Choose ffprobe",
    "Choose Folder",
    "Settings saved locally",
    "Open Exports Folder",
    "Export to build sprite sheet preview",
    "Previous Frame",
    "Next Frame",
    "Anchor X",
    "Anchor Y",
    "Apply Anchor",
    "Validate Re-import",
    "No source selected",
    "Load Sample Path",
    "sample-action-hint",
    "First Run",
    "Run Summary",
    "Toolchain needs attention",
    "Open Settings",
    "Fix Background",
    "Adjust Anchor",
    "Adjust Loop",
    "Review Sheet",
    "Select Source",
    "Import Selected",
    "Segment Range",
    "Use Start and End fields",
    "Animation Name",
    "Creator",
    "License",
    "Loop export",
    "Loop Start",
    "Loop End",
    "Loop Range",
    "Godot Helper",
    "Generated Outputs",
    "validation-result",
    "Sheet Columns",
    "Sheet Padding",
    "Sheet Margin",
    "Split sheets when needed",
    "Show Info",
  ];
  const localeRequired = locale === "zh-CN"
    ? [
        "本地工作台",
        "导入来源",
        "运行示例流程",
        "质量报告",
        "导出资源包",
        "跟随系统",
      ]
    : [
        "Local Workbench",
        "Import Sources",
        "Run Sample Pipeline",
        "Quality Report",
        "Export Pack",
        "Automatic",
      ];
  const forbidden = [
    "Generate Candidates",
    "Generate Final Animation",
    "Creator Publish",
    "provider API",
    "BYOK",
    "cost preflight",
    "hosted credits",
    "online registry",
    "Marketplace",
    "Unity",
    "Undo",
    "Redo",
    "Save",
    "Run Quality Check",
    "Quality check waits for a source",
    "More engine targets",
    "Preview Export",
    "Export Targets",
    "Sheet options",
    "Frame 12 / 64",
    "/Users/kartz/Development/Forge/examples/inputs/green-box-character.mp4",
    "Duration: 12.3s",
  ];

  for (const text of required) {
    if (!source.includes(text)) {
      throw new Error(`UI smoke missing required text: ${text}`);
    }
  }
  for (const text of localeRequired) {
    if (!source.includes(text)) {
      throw new Error(`UI smoke missing ${locale} locale text: ${text}`);
    }
  }
  if (smokeMode === "mvp") {
    for (const text of mvpRequired) {
      if (!source.includes(text)) {
        throw new Error(`MVP UI smoke missing required boundary text: ${text}`);
      }
    }
  }
  if (smokeMode === "responsive") {
    const responsiveForbidden = [
      "className:\"traffic\"",
      "className:\"dot red\"",
      ".traffic",
      ".dot.red",
      "min-width:1568px",
      "min-height:1003px",
      "border-left:2px solid var(--cyan)",
      "border-bottom:3px solid",
    ];
    for (const text of responsiveForbidden) {
      if (source.includes(text)) {
        throw new Error(`Responsive UI smoke found forbidden implementation detail: ${text}`);
      }
    }
  }
  for (const text of forbidden) {
    if (source.includes(text)) {
      throw new Error(`UI smoke found forbidden text: ${text}`);
    }
  }
}
