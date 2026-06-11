# Forge Post-RC Local Library And Exporter Refinement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Turn the notarized import-only MVP into a clearer, easier-to-test local workbench by making ffmpeg missing-state verification deterministic, clarifying the first-run sample flow, adding a local `.gsfpack` library, and tightening generic/Godot export evidence before any AI, registry, marketplace, cloud, or MCP product features are built.

**Architecture:** Keep the product local-first and import-only. Add small, testable seams around existing Rust pack validation and ffmpeg discovery, then reuse the current `Exports` route as a local pack library instead of introducing online registry concepts. Preserve the current pack schema and multi-page export behavior while improving metadata evidence and UI affordances.

**Tech Stack:** Tauri 2, React 18, TypeScript, Vite, Rust workspace, `forge_core`, `forge_pack`, JSON Schema, ffmpeg/ffprobe, macOS Developer ID notarized DMG, Computer Use MCP for installed-app UI checks, multi-agent MCP for implementation/review orchestration during execution.

---

## Current Code Truths

```text
Current RC: release-candidates/GameSpriteForge-0.1.0-aarch64-218a5f9adf1f-notarized
DMG SHA-256: 218a5f9adf1fb47e5f744efbda357470ac8c6aa2c065451c926842c35cd0c739
Release zip SHA-256: fa3b9331eaafe513ebd08d0ae5dc07cb8324669e65d3a2e7a827286666e3ad00
Notarization submission: 7792d837-5da7-46da-8da4-33b559dda6cc
Public release gate: notarized, stapled, Gatekeeper accepted, mounted-DMG launch verified
High-resolution export: resolved with multi-page sprite sheets and re-import validation
Clean environment gap: resolved for local QA with GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS and GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS; true clean-Mac verification remains optional external confidence
Exports route: upgraded into a local pack library with refresh, inspect, validate, open, and re-import actions
Godot helper: currently writes assets/godot_import.json with textures, atlas, manifest, frame size, animation, and anchor data, but no Godot sample-project evidence yet
MCP product scope: disabled; use MCP only as a development/testing tool in this plan
```

## Execution Status - 2026-06-05

```text
Status: implemented and verified locally.
No commit was made because /Users/kartz/Development/Forge is not a git repository.
Task 1 Computer Use evidence: deterministic missing ffmpeg message verified.
Task 6 Computer Use evidence: first-run copy and Local Pack Library refresh/validate/re-import verified.
Task 7 release evidence: notarized/stapled DMG SHA-256 218a5f9adf1fb47e5f744efbda357470ac8c6aa2c065451c926842c35cd0c739, release zip SHA-256 fa3b9331eaafe513ebd08d0ae5dc07cb8324669e65d3a2e7a827286666e3ad00.
```

## Scope Guard

Do not add these product areas in this plan:

```text
AI image generation
AI video generation
BYOK settings
website
online registry
marketplace
creator publish
cloud upload
cloud processing
hosted credits
MCP product server
Codex Skill product integration
Unity editor plugin
Godot editor plugin
```

## File Structure

- Modify: `packages/core/src/video/ffmpeg.rs` - deterministic ffmpeg discovery controls and tests.
- Modify: `apps/mac/src-tauri/src/lib.rs` - Tauri command wrappers for dependency check and local pack library.
- Modify: `apps/mac/src/tauriCommands.ts` - TypeScript wrappers for new pack-library commands.
- Modify: `apps/mac/src/App.tsx` - upgrade `Exports` route into Local Pack Library.
- Modify: `apps/mac/src/routes/ForgeRoute.tsx` - clarify first-run sample flow and consume queued local-library re-import paths.
- Modify: `apps/mac/src/components/ImportPanel.tsx` - make sample-path action text unambiguous.
- Modify: `apps/mac/src/components/ExportPanel.tsx` - expose exporter metadata evidence without changing output scope.
- Modify: `apps/mac/src/styles/app.css` - layout for local library and first-run copy changes.
- Modify: `apps/mac/scripts/smoke-ui.mjs` - smoke assertions for first-run copy and Local Pack Library.
- Modify: `scripts/test-import-panel-source.mjs` - source guard for import/sample copy.
- Create: `scripts/test-ffmpeg-resolver-source.mjs` - source guard for deterministic ffmpeg resolver env names.
- Create: `scripts/test-local-pack-library-source.mjs` - source guard for local pack library commands and UI.
- Modify: `package.json` - include new source guard scripts in `test:scripts`.
- Modify: `packages/core/src/export/mod.rs` - refine Godot helper metadata only if required by Task 5 tests.
- Modify: `packages/core/src/export/manifest.rs` - preserve manifest compatibility and add explicit evidence fields only if required by Task 5 tests.
- Modify: `packages/pack/src/lib.rs` - expose pack inspect details beyond `PackSummary`.
- Modify: `packages/pack/tests/pack_tests.rs` - pack inspect/library tests.
- Create: `docs/qa/forge-post-rc-ui-smoke-2026-06-05.md` - Computer Use evidence for Tasks 1 and 6.
- Create: `docs/qa/godot-helper-evidence-2026-06-05.md` - exporter evidence from a local sample project or schema-level fallback.
- Modify: `docs/architecture/post-release-backlog.md` - mark completed slices and next decision gates.
- Modify: `docs/architecture/mvp-scope.md` - current-state update after the plan is implemented.

## MCP And Skill Operating Model

```text
Use superpowers:subagent-driven-development for implementation.
Use multi_agent_v1 worker agents only when execution starts and file ownership is disjoint.
Use Computer Use MCP for installed-app verification in Task 1 and Task 6.
Use local shell commands for Rust, TypeScript, Vite, schema, packaging, and release checks.
Do not use Figma MCP; this is not a design import or new design system task.
Do not use GitHub MCP unless the workspace becomes a git repository or the user asks for GitHub publication.
```

## Verification Commands

Run this set before marking the full plan complete:

```bash
npm run test:scripts
npm --workspace apps/mac run build
npm --workspace apps/mac run smoke:ui:mvp
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml
scripts/build-signed-release-dmg.sh
scripts/notarization-preflight.sh --keychain-profile GameSpriteForgeNotary "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg" 218a5f9adf1fb47e5f744efbda357470ac8c6aa2c065451c926842c35cd0c739
scripts/verify-release-package.sh "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg" 218a5f9adf1fb47e5f744efbda357470ac8c6aa2c065451c926842c35cd0c739
```

Expected final result:

```text
script tests: pass
frontend build: pass
MVP UI smoke: pass
Rust tests: pass
signed DMG build: pass
notarization preflight: pass
release verifier: pass
Computer Use UI evidence: recorded for deterministic missing ffmpeg and local pack library
```

---

### Task 1: Deterministic FFmpeg Missing-State Verification

**Files:**
- Modify: `packages/core/src/video/ffmpeg.rs`
- Modify: `apps/mac/src-tauri/src/lib.rs`
- Create: `scripts/test-ffmpeg-resolver-source.mjs`
- Modify: `package.json`
- Create: `docs/qa/forge-post-rc-ui-smoke-2026-06-05.md`

- [x] **Step 1: Add a source guard for resolver override names**

Create `scripts/test-ffmpeg-resolver-source.mjs`:

```js
import { readFileSync } from "node:fs";

const ffmpegSource = readFileSync("packages/core/src/video/ffmpeg.rs", "utf8");
const tauriSource = readFileSync("apps/mac/src-tauri/src/lib.rs", "utf8");

function assertContains(source, needle, message) {
  if (!source.includes(needle)) {
    throw new Error(message);
  }
}

assertContains(
  ffmpegSource,
  "GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS",
  "ffmpeg resolver must support deterministic search directories for clean-environment QA.",
);
assertContains(
  ffmpegSource,
  "GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS",
  "ffmpeg resolver must support disabling default macOS tool directories for CE-003.",
);
assertContains(
  tauriSource,
  "find_in_path(expected_name)",
  "Tauri dependency check must continue to use the shared ffmpeg resolver path.",
);

console.log("PASS ffmpeg resolver source test");
```

- [x] **Step 2: Add the source guard to `package.json`**

In `package.json`, update `test:scripts` so it includes the new check command before `bash -n`:

```json
"node --check scripts/test-ffmpeg-resolver-source.mjs && node scripts/test-ffmpeg-resolver-source.mjs"
```

Run:

```bash
npm run test:scripts
```

Expected before implementation:

```text
FAIL ffmpeg resolver must support deterministic search directories for clean-environment QA.
```

- [x] **Step 3: Implement deterministic resolver directories**

In `packages/core/src/video/ffmpeg.rs`, replace `tool_search_directories()` with this implementation and add the helper functions below it:

```rust
fn tool_search_directories() -> Vec<PathBuf> {
    let mut directories = env::var_os("GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS")
        .map(|value| env::split_paths(&value).collect::<Vec<_>>())
        .or_else(|| env::var_os("PATH").map(|value| env::split_paths(&value).collect::<Vec<_>>()))
        .unwrap_or_default();

    if !disable_macos_default_tool_dirs() {
        for directory in default_tool_directories() {
            if !directories.iter().any(|existing| existing == &directory) {
                directories.push(directory);
            }
        }
    }

    directories
}

fn disable_macos_default_tool_dirs() -> bool {
    env_flag_enabled("GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS")
}

fn env_flag_enabled(name: &str) -> bool {
    env::var_os(name)
        .and_then(|value| value.into_string().ok())
        .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}
```

- [x] **Step 4: Add `temp-env` and pure unit coverage for search-directory behavior**

In `packages/core/Cargo.toml`, add this dev-dependency:

```toml
temp-env = "0.3"
```

In the `#[cfg(test)]` module of `packages/core/src/video/ffmpeg.rs`, add:

```rust
#[test]
fn env_flag_enabled_accepts_common_truthy_values() {
    temp_env::with_var("GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS", Some("true"), || {
        assert!(disable_macos_default_tool_dirs());
    });
}

#[test]
fn env_search_dirs_override_path_when_set() {
    let temp = tempfile::tempdir().unwrap();
    let ffmpeg = temp.path().join("ffmpeg");
    std::fs::write(&ffmpeg, b"").unwrap();
    let search_dirs = temp.path().to_string_lossy().into_owned();

    temp_env::with_var(
        "GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS",
        Some(search_dirs.as_str()),
        || {
            temp_env::with_var("GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS", Some("1"), || {
                let found = find_in_path("ffmpeg").unwrap();
                assert_eq!(found, ffmpeg.to_string_lossy());
            });
        },
    );
}
```

- [x] **Step 5: Run Task 1 verification**

Run:

```bash
npm run test:scripts
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml video::ffmpeg
```

Expected:

```text
PASS ffmpeg resolver source test
video::ffmpeg tests: pass
```

- [x] **Step 6: Use Computer Use for installed-app missing-state evidence**

Launch the app executable directly with deterministic resolver variables:

```bash
env \
  GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS=/tmp/forge-empty-tools \
  GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS=1 \
  PATH=/usr/bin:/bin \
  "/Applications/Game Sprite Forge.app/Contents/MacOS/Game Sprite Forge"
```

Use Computer Use to click `Check FFmpeg`.

Record this in `docs/qa/forge-post-rc-ui-smoke-2026-06-05.md`:

````markdown
## Task 1 - Deterministic Missing FFmpeg

Command:

```bash
env GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS=/tmp/forge-empty-tools GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS=1 PATH=/usr/bin:/bin "/Applications/Game Sprite Forge.app/Contents/MacOS/Game Sprite Forge"
```

Observed:

```text
Check FFmpeg showed: Install ffmpeg or choose an ffmpeg binary in Settings.
```

Result: Pass
````

- [x] **Step 7: Commit or handoff**

Run:

```bash
git add packages/core/src/video/ffmpeg.rs packages/core/Cargo.toml apps/mac/src-tauri/src/lib.rs scripts/test-ffmpeg-resolver-source.mjs package.json docs/qa/forge-post-rc-ui-smoke-2026-06-05.md
git commit -m "test: make ffmpeg missing state deterministic"
```

If `/Users/kartz/Development/Forge` is not a git repository, record `No commit: workspace is not a git repository` in the execution handoff.

---

### Task 2: Clarify First-Run Sample Actions

**Files:**
- Modify: `apps/mac/src/components/ImportPanel.tsx`
- Modify: `apps/mac/src/routes/ForgeRoute.tsx`
- Modify: `apps/mac/src/styles/app.css`
- Modify: `scripts/test-import-panel-source.mjs`
- Modify: `apps/mac/scripts/smoke-ui.mjs`

- [x] **Step 1: Extend source guard for sample copy**

Append these assertions to `scripts/test-import-panel-source.mjs`:

```js
assertContains(
  source,
  "Load Sample Path",
  "ImportPanel sample action must say Load Sample Path so users know it only fills the source field.",
);
assertContains(
  source,
  "Import Video",
  "ImportPanel must keep Import Video as the action that creates a live workspace.",
);
```

Create a second source read for `ForgeRoute.tsx` in the same file:

```js
const forgeRouteSource = readFileSync("apps/mac/src/routes/ForgeRoute.tsx", "utf8");

assertContains(
  forgeRouteSource,
  "Run Sample Pipeline",
  "First Run rail must label the full bundled sample action as Run Sample Pipeline.",
);
assertContains(
  forgeRouteSource,
  "Loads, processes, exports, and validates the bundled sample.",
  "First Run rail must describe what Run Sample Pipeline does.",
);
```

Run:

```bash
npm run test:scripts
```

Expected before implementation:

```text
FAIL ImportPanel sample action must say Load Sample Path so users know it only fills the source field.
```

- [x] **Step 2: Rename ImportPanel sample action**

In `apps/mac/src/components/ImportPanel.tsx`, change the `Load Sample` button label:

```tsx
<button className="load-sample-button" disabled={disabled} onClick={onLoadSample} type="button">
  <PlayCircle size={13} />
  Load Sample Path
</button>
```

- [x] **Step 3: Rename First Run rail full-pipeline action**

In `apps/mac/src/routes/ForgeRoute.tsx`, change the First Run copy:

```tsx
<small>Setup guide: check tools, then run the bundled sample.</small>
```

to:

```tsx
<small>Loads, processes, exports, and validates the bundled sample.</small>
```

Change the primary sample button label:

```tsx
<button className="status-action primary" disabled={disabled} onClick={onRunSample} type="button">
  <PlayCircle size={13} />
  Run Sample Pipeline
</button>
```

- [x] **Step 4: Keep compact buttons from overflowing**

In `apps/mac/src/styles/app.css`, ensure the sample buttons can wrap cleanly:

```css
.load-sample-button,
.activation-actions .status-action {
  min-height: 32px;
  white-space: normal;
}

.activation-actions .status-action.primary {
  min-width: 0;
}
```

- [x] **Step 5: Add smoke assertions for new copy**

In `apps/mac/scripts/smoke-ui.mjs`, add these strings to the `mvpRequired` array inside `assertSource(source, smokeMode)`:

```js
"Load Sample Path",
"Run Sample Pipeline",
"Loads, processes, exports, and validates the bundled sample.",
```

- [x] **Step 6: Run Task 2 verification**

Run:

```bash
npm run test:scripts
npm --workspace apps/mac run build
npm --workspace apps/mac run smoke:ui:mvp
```

Expected:

```text
PASS import panel source test
vite build: pass
UI smoke passed (mvp)
```

- [x] **Step 7: Commit or handoff**

Run:

```bash
git add apps/mac/src/components/ImportPanel.tsx apps/mac/src/routes/ForgeRoute.tsx apps/mac/src/styles/app.css apps/mac/scripts/smoke-ui.mjs scripts/test-import-panel-source.mjs
git commit -m "ux: clarify first run sample actions"
```

If the workspace is not a git repository, record the changed paths in the execution handoff.

---

### Task 3: Add Local Pack Library Commands

**Files:**
- Modify: `apps/mac/src-tauri/src/lib.rs`
- Modify: `apps/mac/src/tauriCommands.ts`
- Modify: `packages/pack/src/lib.rs`
- Modify: `packages/pack/tests/pack_tests.rs`
- Create: `scripts/test-local-pack-library-source.mjs`
- Modify: `package.json`

- [x] **Step 1: Add source guard for command names**

Create `scripts/test-local-pack-library-source.mjs`:

```js
import { readFileSync } from "node:fs";

const tauriSource = readFileSync("apps/mac/src-tauri/src/lib.rs", "utf8");
const commandSource = readFileSync("apps/mac/src/tauriCommands.ts", "utf8");
const appSource = readFileSync("apps/mac/src/App.tsx", "utf8");

function assertContains(source, needle, message) {
  if (!source.includes(needle)) {
    throw new Error(message);
  }
}

assertContains(tauriSource, "list_local_packs", "Tauri must expose list_local_packs.");
assertContains(tauriSource, "inspect_local_pack", "Tauri must expose inspect_local_pack.");
assertContains(commandSource, "listLocalPacks", "TypeScript must wrap listLocalPacks.");
assertContains(commandSource, "inspectLocalPack", "TypeScript must wrap inspectLocalPack.");
assertContains(appSource, "Local Pack Library", "Exports route must become Local Pack Library.");
assertContains(appSource, "Validate Pack", "Local Pack Library must expose Validate Pack.");
assertContains(appSource, "Re-import Pack", "Local Pack Library must expose Re-import Pack.");

console.log("PASS local pack library source test");
```

Add it to `package.json` `test:scripts`:

```json
"node --check scripts/test-local-pack-library-source.mjs && node scripts/test-local-pack-library-source.mjs"
```

- [x] **Step 2: Add Rust pack detail type**

In `packages/pack/src/lib.rs`, extend `PackSummary` into a detail type:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackInspectSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub frame_count: usize,
    pub preview_gif: String,
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub atlas_path: PathBuf,
    pub quality_report_path: PathBuf,
}
```

Add:

```rust
pub fn inspect_pack(pack_path: &Path) -> Result<PackInspectSummary, PackError> {
    validate_pack_layout(pack_path)?;
    let summary = read_pack_summary(pack_path)?;

    Ok(PackInspectSummary {
        id: summary.id,
        name: summary.name,
        version: summary.version,
        frame_count: summary.frame_count,
        preview_gif: summary.preview_gif,
        root: pack_path.to_path_buf(),
        manifest_path: pack_path.join("assets/manifest.json"),
        atlas_path: pack_path.join("assets/atlas.json"),
        quality_report_path: pack_path.join("quality-report.json"),
    })
}
```

- [x] **Step 3: Add pack inspect test**

In `packages/pack/tests/pack_tests.rs`, add a test using the existing valid fixture helper:

```rust
#[test]
fn inspect_pack_returns_paths_needed_by_local_library() {
    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("library.gsfpack");
    write_pack_fixture(&pack, 3);

    let summary = forge_pack::inspect_pack(&pack).unwrap();

    assert_eq!(summary.name, "Library Pack");
    assert_eq!(summary.frame_count, 3);
    assert_eq!(summary.root, pack);
    assert_eq!(summary.manifest_path, summary.root.join("assets/manifest.json"));
    assert_eq!(summary.atlas_path, summary.root.join("assets/atlas.json"));
    assert_eq!(summary.quality_report_path, summary.root.join("quality-report.json"));
}
```

- [x] **Step 4: Add Tauri commands**

In `apps/mac/src-tauri/src/lib.rs`, add:

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListLocalPacksParams {
    exports_dir: String,
}

#[tauri::command]
fn list_local_packs(params: ListLocalPacksParams) -> Result<Vec<forge_pack::PackInspectSummary>, String> {
    let exports_dir = normalize_user_export_directory(Path::new(&params.exports_dir))?;
    let mut packs = Vec::new();

    if !exports_dir.exists() {
        return Ok(packs);
    }

    for entry in fs::read_dir(exports_dir).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.extension().and_then(|value| value.to_str()) == Some("gsfpack") {
            if let Ok(summary) = forge_pack::inspect_pack(&path) {
                packs.push(summary);
            }
            continue;
        }
        if path.is_dir() {
            for nested in fs::read_dir(path).map_err(|error| error.to_string())? {
                let nested_path = nested.map_err(|error| error.to_string())?.path();
                if nested_path.extension().and_then(|value| value.to_str()) == Some("gsfpack") {
                    if let Ok(summary) = forge_pack::inspect_pack(&nested_path) {
                        packs.push(summary);
                    }
                }
            }
        }
    }

    packs.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(packs)
}

#[tauri::command]
fn inspect_local_pack(path: String) -> Result<forge_pack::PackInspectSummary, String> {
    forge_pack::inspect_pack(Path::new(&path)).map_err(|error| error.to_string())
}
```

Register both commands in `tauri::generate_handler!`:

```rust
list_local_packs,
inspect_local_pack,
```

- [x] **Step 5: Add TypeScript wrappers**

In `apps/mac/src/tauriCommands.ts`, add:

```ts
export type PackInspectSummary = PackSummary & {
  root: string;
  manifestPath: string;
  atlasPath: string;
  qualityReportPath: string;
};

export function listLocalPacks(exportsDir: string) {
  return invoke<PackInspectSummary[]>("list_local_packs", { params: { exportsDir } });
}

export function inspectLocalPack(path: string) {
  return invoke<PackInspectSummary>("inspect_local_pack", { path });
}
```

- [x] **Step 6: Run Task 3 verification**

Run:

```bash
npm run test:scripts
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml pack_tests::inspect_pack_returns_paths_needed_by_local_library
```

Expected:

```text
PASS local pack library source test
inspect_pack_returns_paths_needed_by_local_library ... ok
```

- [x] **Step 7: Commit or handoff**

Run:

```bash
git add apps/mac/src-tauri/src/lib.rs apps/mac/src/tauriCommands.ts packages/pack/src/lib.rs packages/pack/tests/pack_tests.rs scripts/test-local-pack-library-source.mjs package.json
git commit -m "feat: add local pack library commands"
```

If the workspace is not a git repository, record the changed paths in the execution handoff.

---

### Task 4: Upgrade Exports Route Into Local Pack Library UI

**Files:**
- Modify: `apps/mac/src/App.tsx`
- Modify: `apps/mac/src/routes/ForgeRoute.tsx`
- Modify: `apps/mac/src/styles/app.css`
- Modify: `apps/mac/scripts/smoke-ui.mjs`

- [x] **Step 1: Import local pack commands**

In `apps/mac/src/App.tsx`, keep the existing `openFileOrFolder` import from `./systemDialogs` and change the Tauri imports to include value wrappers:

```tsx
import type { ExportPackOutput, LocalSettings, PackInspectSummary } from "./tauriCommands";
import { importGsfpack, inspectLocalPack, listLocalPacks, validateGsfpack } from "./tauriCommands";
```

- [x] **Step 2: Replace `ExportsRoute` props and state**

Change `ExportsRoute` props to:

```tsx
function ExportsRoute({
  exports,
  libraryPacks,
  onInspectPack,
  onOpenExportFolder,
  onRefreshLibrary,
  onReimportPack,
  onValidatePack,
}: {
  exports: RecordedExport[];
  libraryPacks: PackInspectSummary[];
  onInspectPack: (path: string) => void;
  onOpenExportFolder: (path: string) => void;
  onRefreshLibrary: () => void;
  onReimportPack: (path: string) => void;
  onValidatePack: (path: string) => void;
}) {
```

- [x] **Step 3: Replace the Exports heading and empty state**

Inside `ExportsRoute`, use this heading:

```tsx
<div className="panel-title tall">
  <span>Local Pack Library</span>
  <span>Validate, inspect, open, and re-import local .gsfpack exports</span>
</div>
```

Use this refresh button near the heading:

```tsx
<button className="secondary-button export-history-open" onClick={onRefreshLibrary} type="button">
  <FolderOpen size={16} />
  Refresh Library
</button>
```

Use this empty state:

```tsx
<div className="exports-empty">
  <PackageCheck size={30} />
  <strong>No local packs found</strong>
  <span>Export a pack or refresh the default export folder.</span>
</div>
```

- [x] **Step 4: Render library pack actions**

Add this list before the recent-export fallback:

```tsx
{libraryPacks.length ? (
  <div className="export-history-list">
    {libraryPacks.map((pack) => (
      <article className="export-history-card" key={pack.root}>
        <div>
          <strong>{pack.name}</strong>
          <span>{pack.frameCount} frames · v{pack.version}</span>
          <small>{pack.root}</small>
        </div>
        <div className="export-history-actions">
          <button className="secondary-button export-history-open" onClick={() => onInspectPack(pack.root)} type="button">
            Inspect
          </button>
          <button className="secondary-button export-history-open" onClick={() => onValidatePack(pack.root)} type="button">
            Validate Pack
          </button>
          <button className="secondary-button export-history-open" onClick={() => onReimportPack(pack.root)} type="button">
            Re-import Pack
          </button>
          <button className="secondary-button export-history-open" onClick={() => onOpenExportFolder(pack.root)} type="button">
            <FolderOpen size={16} />
            Open
          </button>
        </div>
      </article>
    ))}
  </div>
) : null}
```

- [x] **Step 5: Add App state and handlers**

In `App`, add:

```tsx
const [libraryPacks, setLibraryPacks] = useState<PackInspectSummary[]>([]);
const [queuedGsfpackImportPath, setQueuedGsfpackImportPath] = useState<string | null>(null);
```

Add:

```tsx
async function handleRefreshLibrary() {
  const packs = await listLocalPacks(settings.defaultOutputFolder);
  setLibraryPacks(packs);
}

async function handleInspectPack(path: string) {
  const pack = await inspectLocalPack(path);
  setLibraryPacks((current) => [pack, ...current.filter((item) => item.root !== pack.root)]);
}

async function handleValidatePack(path: string) {
  await validateGsfpack(path);
  await handleInspectPack(path);
}

async function handleReimportPack(path: string) {
  setQueuedGsfpackImportPath(path);
  setActiveRoute("forge");
  setActiveWorkflow("Frames");
}
```

When rendering `ForgeRoute`, pass the queued import path:

```tsx
<ForgeRoute
  activeWorkflow={activeWorkflow}
  onExportRecorded={handleExportRecorded}
  onOpenSettings={() => setActiveRoute("settings")}
  onQueuedGsfpackImportConsumed={() => setQueuedGsfpackImportPath(null)}
  onWorkbenchStateChange={setWorkbenchState}
  onWorkflowChange={setActiveWorkflow}
  queuedGsfpackImportPath={queuedGsfpackImportPath}
  settings={settings}
/>
```

When rendering `ExportsRoute`, pass:

```tsx
<ExportsRoute
  exports={recentExports}
  libraryPacks={libraryPacks}
  onInspectPack={handleInspectPack}
  onOpenExportFolder={handleOpenExportFolder}
  onRefreshLibrary={handleRefreshLibrary}
  onReimportPack={handleReimportPack}
  onValidatePack={handleValidatePack}
/>
```

- [x] **Step 6: Consume queued re-import paths in ForgeRoute**

In `apps/mac/src/routes/ForgeRoute.tsx`, extend props:

```tsx
type ForgeRouteProps = {
  activeWorkflow: ForgeWorkflow;
  onExportRecorded: (record: { frameCount: number; output: ExportPackOutput; packName: string }) => void;
  onOpenSettings: () => void;
  onQueuedGsfpackImportConsumed: () => void;
  onWorkbenchStateChange: (state: WorkbenchState) => void;
  onWorkflowChange: (workflow: ForgeWorkflow) => void;
  queuedGsfpackImportPath: string | null;
  settings: LocalSettings;
};
```

Destructure the new props:

```tsx
export function ForgeRoute({
  activeWorkflow,
  onExportRecorded,
  onOpenSettings,
  onQueuedGsfpackImportConsumed,
  onWorkbenchStateChange,
  onWorkflowChange,
  queuedGsfpackImportPath,
  settings,
}: ForgeRouteProps) {
```

Add this effect after the workbench-state effect:

```tsx
useEffect(() => {
  if (!queuedGsfpackImportPath || isRunning) {
    return;
  }

  setGsfpackPath(queuedGsfpackImportPath);
  onQueuedGsfpackImportConsumed();
  void handleImportGsfpack(queuedGsfpackImportPath);
}, [queuedGsfpackImportPath, isRunning, onQueuedGsfpackImportConsumed]);
```

Change `handleImportGsfpack` to accept an optional path override:

```tsx
async function handleImportGsfpack(pathOverride = gsfpackPath) {
  await runStep("Validating and importing .gsfpack...", async () => {
    const packPath = requiredPath(pathOverride, ".gsfpack path");
    const summary = await validateGsfpack(packPath);
    const imported = await importGsfpack(packPath);
    setJob(imported.job);
    setWorkingSourcePath(null);
    setProbe(null);
    setExtractResult(imported.rawFrames);
    resetLoopRange(imported.rawFrames.frames.length);
    setSelectedFrameIndex(0);
    setPreviewResult(null);
    setNormalizeResult(null);
    setQualityReport(null);
    setExportOutput(null);
    setPackSummary(summary);
    setPackPreviewPath(imported.previewGifPath);
    setActiveSourceName(fileName(packPath));
    onWorkflowChange("Frames");
    return `Imported ${summary.name} with ${summary.frameCount} frames.`;
  });
}
```

- [x] **Step 7: Style pack actions**

In `apps/mac/src/styles/app.css`, add:

```css
.export-history-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: flex-end;
}

.export-history-actions .secondary-button {
  min-height: 32px;
}
```

- [x] **Step 8: Run Task 4 verification**

Run:

```bash
npm --workspace apps/mac run build
npm --workspace apps/mac run smoke:ui:mvp
```

Expected:

```text
vite build: pass
UI smoke passed (mvp)
```

- [x] **Step 9: Commit or handoff**

Run:

```bash
git add apps/mac/src/App.tsx apps/mac/src/routes/ForgeRoute.tsx apps/mac/src/styles/app.css apps/mac/scripts/smoke-ui.mjs
git commit -m "feat: add local pack library UI"
```

If the workspace is not a git repository, record the changed paths in the execution handoff.

---

### Task 5: Refine Exporter Evidence And Godot Helper Contract

**Files:**
- Modify: `packages/core/src/export/mod.rs`
- Modify: `packages/core/src/export/manifest.rs`
- Modify: `packages/core/src/export/sheet.rs`
- Modify: `schemas/manifest.schema.json`
- Modify: `schemas/atlas.schema.json`
- Modify: `docs/qa/godot-helper-evidence-2026-06-05.md`
- Modify: `docs/architecture/post-release-backlog.md`

- [x] **Step 1: Confirm current Godot helper contract with a focused test**

In `packages/core/src/export/mod.rs` tests, add:

```rust
#[test]
fn godot_helper_lists_all_multipage_textures() {
    let manifest = manifest::EngineManifest {
        name: "Hero Knight Pack".to_string(),
        sheet: manifest::ManifestSheet {
            image: "assets/sprite_sheet.png".to_string(),
            images: vec![
                "assets/sprite_sheet.png".to_string(),
                "assets/sprite_sheet_002.png".to_string(),
            ],
            frame_width: 64,
            frame_height: 64,
            columns: 4,
            rows: 4,
        },
        animations: vec![manifest::ManifestAnimation {
            name: "walk".to_string(),
            frames: vec![0, 1],
            fps: 12.0,
            loop_animation: true,
        }],
        anchor: manifest::ManifestAnchor {
            anchor_type: "feet".to_string(),
            x: 32.0,
            y: 64.0,
        },
    };

    let helper = godot_import_helper(&manifest);
    let textures = helper["spriteFrames"]["textures"].as_array().unwrap();

    assert_eq!(textures.len(), 2);
    assert_eq!(textures[0].as_str().unwrap(), "assets/sprite_sheet.png");
    assert_eq!(textures[1].as_str().unwrap(), "assets/sprite_sheet_002.png");
    assert_eq!(helper["spriteFrames"]["frameWidth"].as_u64().unwrap(), 64);
    assert_eq!(helper["spriteFrames"]["frameHeight"].as_u64().unwrap(), 64);
}
```

- [x] **Step 2: Run the focused test**

Run:

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml godot_helper_lists_all_multipage_textures
```

Expected:

```text
godot_helper_lists_all_multipage_textures ... ok
```

If it fails, update `godot_import_helper` so `textures` always uses `manifest.sheet.images` when present and falls back to `[manifest.sheet.image]` only for legacy single-page packs.

- [x] **Step 3: Add schema evidence for current multi-page fields**

Run:

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml validates_multipage_atlas_assets
```

Expected:

```text
validates_multipage_atlas_assets ... ok
```

- [x] **Step 4: Write Godot helper evidence doc**

Create `docs/qa/godot-helper-evidence-2026-06-05.md`:

````markdown
# Godot Helper Evidence - 2026-06-05

Artifact:

```text
/Users/kartz/Game Sprite Forge/Exports/Hero-Knight-Pack-1780644870371/Hero-Knight-Pack.gsfpack
```

Contract checked:

```text
assets/godot_import.json lists every texture from manifest.sheet.images.
assets/atlas.json keeps frame coordinates and page image references.
assets/manifest.json keeps animation frames, FPS, loop flag, frame size, and anchor.
```

Verification:

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml godot_helper_lists_all_multipage_textures
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml validates_multipage_atlas_assets
```

Result:

```text
Godot helper metadata is schema-consistent for multi-page exports.
Manual Godot editor import remains a future evidence row, not a blocker for the local import-only MVP.
```
````

- [x] **Step 5: Update post-release backlog**

In `docs/architecture/post-release-backlog.md`, under `Slice 2: Exporter Refinement`, add:

```text
Godot helper schema-level evidence exists in docs/qa/godot-helper-evidence-2026-06-05.md.
Godot editor/sample-project validation exists in docs/qa/godot-editor-helper-evidence-2026-06-06.md.
```

- [x] **Step 6: Run Task 5 verification**

Run:

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml godot_helper_lists_all_multipage_textures
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml validates_multipage_atlas_assets
npm run test:scripts
```

Expected:

```text
Godot helper focused test: pass
multi-page atlas validation test: pass
script tests: pass
```

- [x] **Step 7: Commit or handoff**

Run:

```bash
git add packages/core/src/export/mod.rs docs/qa/godot-helper-evidence-2026-06-05.md docs/architecture/post-release-backlog.md
git commit -m "test: document Godot helper multipage evidence"
```

If the workspace is not a git repository, record the changed paths in the execution handoff.

---

### Task 6: Real UI Verification With Computer Use

**Files:**
- Modify: `docs/qa/forge-post-rc-ui-smoke-2026-06-05.md`
- Modify: `docs/qa/forge-manual-mvp-checklist-2026-06-05.md`
- Modify: `docs/architecture/mvp-scope.md`

- [x] **Step 1: Build and install a fresh app**

Run:

```bash
npm --workspace apps/mac run build
npm --workspace apps/mac run tauri -- build --bundles app --no-sign
rm -rf "/Applications/Game Sprite Forge.app"
cp -R "target/release/bundle/macos/Game Sprite Forge.app" "/Applications/Game Sprite Forge.app"
```

Expected:

```text
Vite build passes.
Tauri app bundle exists at /Applications/Game Sprite Forge.app.
```

- [x] **Step 2: Computer Use verifies first-run copy**

Use Computer Use to open `/Applications/Game Sprite Forge.app`.

Record these visible labels in `docs/qa/forge-post-rc-ui-smoke-2026-06-05.md`:

````markdown
## Task 6 - First Run Copy

Observed:

```text
Import panel button: Load Sample Path
First Run primary button: Run Sample Pipeline
First Run detail: Loads, processes, exports, and validates the bundled sample.
```

Result: Pass
````

- [x] **Step 3: Computer Use verifies Local Pack Library**

Use Computer Use:

```text
Open Forge.
Run Sample Pipeline.
Open Exports.
Click Refresh Library.
Click Validate Pack on the exported sample pack.
Click Re-import Pack.
Confirm Forge opens the imported pack in Frames workflow.
```

Record:

````markdown
## Task 6 - Local Pack Library UI

Observed:

```text
Local Pack Library lists Green Box Character Pack.
Validate Pack completes without an error toast.
Re-import Pack returns to Forge and shows the pack frame count.
```

Result: Pass
````

- [x] **Step 4: Computer Use verifies deterministic missing ffmpeg**

Use the command from Task 1 Step 6 and confirm the visible missing message.

Expected:

```text
Install ffmpeg or choose an ffmpeg binary in Settings.
```

- [x] **Step 5: Update QA docs**

In `docs/qa/forge-manual-mvp-checklist-2026-06-05.md`, add a post-RC addendum:

````markdown
## Post-RC Follow-up UI Evidence

Recorded after local pack library and first-run copy changes:

```text
First-run sample copy: Pass
Local Pack Library refresh/validate/re-import: Pass
Deterministic missing ffmpeg check: Pass
```
````

In `docs/architecture/mvp-scope.md`, add:

```text
Post-RC local library evidence: Exports route now supports local pack library refresh, validation, and re-import without online registry features.
```

- [x] **Step 6: Run Task 6 verification**

Run:

```bash
npm --workspace apps/mac run build
npm --workspace apps/mac run smoke:ui:mvp
```

Expected:

```text
vite build: pass
UI smoke passed (mvp)
Computer Use evidence recorded in docs/qa/forge-post-rc-ui-smoke-2026-06-05.md
```

- [x] **Step 7: Commit or handoff**

Run:

```bash
git add docs/qa/forge-post-rc-ui-smoke-2026-06-05.md docs/qa/forge-manual-mvp-checklist-2026-06-05.md docs/architecture/mvp-scope.md
git commit -m "docs: record post-rc real ui verification"
```

If the workspace is not a git repository, record the changed paths in the execution handoff.

---

### Task 7: Release Rebuild, Notarization, And Roadmap Update

**Files:**
- Modify: `docs/architecture/release-candidate-verification.md`
- Modify: `docs/architecture/distribution-mvp.md`
- Modify: `docs/architecture/post-release-backlog.md`
- Modify: `docs/architecture/mvp-scope.md`
- Modify: `docs/qa/forge-clean-env-smoke-2026-06-05.md`
- Modify: `docs/qa/forge-post-rc-ui-smoke-2026-06-05.md`

- [x] **Step 1: Run full local verification**

Run:

```bash
npm run test:scripts
npm --workspace apps/mac run build
npm --workspace apps/mac run smoke:ui:mvp
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml
```

Expected:

```text
script tests: pass
frontend build: pass
MVP UI smoke: pass
Rust tests: pass
```

- [x] **Step 2: Build signed release DMG**

Run:

```bash
scripts/build-signed-release-dmg.sh
```

Expected:

```text
Developer ID app signature: pass
Developer ID DMG signature: pass
DMG timestamp: present
Mounted app timestamp: present
Hardened runtime: present
```

- [x] **Step 3: Preflight before submission**

Run:

```bash
scripts/notarization-preflight.sh --keychain-profile GameSpriteForgeNotary "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
```

Expected:

```text
DMG SHA-256 match or printed current hash
DMG integrity: pass
DMG signature: pass
DMG timestamp: pass
Mounted app signature: pass
Mounted app timestamp: pass
Mounted app hardened runtime: pass
Gatekeeper DMG assessment before submission: accepted or ready for submission
```

- [x] **Step 4: Submit and staple**

Run:

```bash
xcrun notarytool submit "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg" --keychain-profile GameSpriteForgeNotary --wait
xcrun stapler staple -v "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
xcrun stapler validate -v "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
```

Expected:

```text
notarytool status: Accepted
stapler staple: The staple and validate action worked
stapler validate: The validate action worked
```

- [x] **Step 5: Verify final release package**

Run:

```bash
scripts/verify-release-package.sh "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
scripts/package-release-candidate.sh
cd release-candidates/$(ls -t release-candidates | grep '^GameSpriteForge-0.1.0-aarch64-' | head -n 1)
shasum -a 256 -c SHA256SUMS
```

Expected:

```text
PASS release package verification script completed.
SHA256SUMS: all OK
```

- [x] **Step 6: Update release and roadmap docs**

Update these documents with the new SHA, submission ID, package path, and post-RC UI evidence:

```text
docs/architecture/release-candidate-verification.md
docs/architecture/distribution-mvp.md
docs/architecture/mvp-scope.md
docs/architecture/post-release-backlog.md
docs/qa/forge-clean-env-smoke-2026-06-05.md
docs/qa/forge-post-rc-ui-smoke-2026-06-05.md
```

Add this decision to `docs/architecture/post-release-backlog.md`:

```text
Completed after RC: deterministic missing ffmpeg QA, first-run sample copy, Local Pack Library, Godot helper schema evidence.
Next decision gate: CLI/MCP feasibility spike, or dedicated bundled ffmpeg implementation only after a distributable ffmpeg build is chosen.
```

- [x] **Step 7: Final verification sweep**

Run:

```bash
rg -n "<older release identifiers>" docs/architecture docs/qa scripts package.json
npm run test:scripts
scripts/verify-release-package.sh "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
```

Expected:

```text
rg finds no stale active release identifiers except historical sections explicitly labeled as history.
script tests: pass
release verifier: pass
```

- [x] **Step 8: Commit or handoff**

Run:

```bash
git add docs/architecture/release-candidate-verification.md docs/architecture/distribution-mvp.md docs/architecture/post-release-backlog.md docs/architecture/mvp-scope.md docs/qa/forge-clean-env-smoke-2026-06-05.md docs/qa/forge-post-rc-ui-smoke-2026-06-05.md release-candidates
git commit -m "ship: rebuild notarized post-rc local library candidate"
```

If the workspace is not a git repository, record the release package path, DMG SHA-256, zip SHA-256, and notarization submission ID in the execution handoff.

---

## Multi-Agent Dispatch Plan

Use this only when the user chooses Subagent-Driven execution.

```text
Task 1 worker: owns packages/core/src/video/ffmpeg.rs, scripts/test-ffmpeg-resolver-source.mjs, package.json, and Task 1 QA doc rows.
Task 2 worker: owns ImportPanel, ForgeRoute copy, CSS, smoke/source tests.
Task 3 worker: owns pack/Tauri command data layer and source tests.
Task 4 worker: owns App Exports route UI and CSS.
Task 5 worker: owns exporter evidence tests and docs.
Task 6 controller: main agent owns Computer Use verification because it requires real UI observation.
Task 7 controller: main agent owns signing/notarization because it uses local Apple credentials and release artifacts.
Spec reviewers: one fresh reviewer after each task checks file scope and requirement coverage.
Quality reviewers: one fresh reviewer after each task checks test coverage, state-chain regressions, path safety, and UI copy/layout risk.
```

## Risk Register

```text
P1: Environment-variable ffmpeg overrides could affect user launches if documented poorly. Mitigation: name them as QA/debug overrides and keep default behavior unchanged.
P1: Local Pack Library could accidentally imply online registry. Mitigation: use only local copy and no publish/upload wording.
P1: Re-import Pack can create state-chain drift if App route changes but ForgeRoute internal state does not receive imported data. Mitigation: Task 4 explicitly passes a queued .gsfpack import path into ForgeRoute and consumes it through the existing import_gsfpack pipeline.
P2: Godot helper evidence could be schema-only rather than editor-proven. Mitigation: explicitly label schema-level evidence and keep manual Godot project validation as next gate.
P2: Tauri direct unsigned install in Task 6 is only local UI verification. Mitigation: Task 7 rebuilds signed/notarized DMG before release.
P3: Workspace is currently not a git repository. Mitigation: every commit step has a no-git handoff fallback.
```

## Self-Review

```text
Spec coverage: Covers deterministic CE-003, first-run sample confusion, Local Pack Library, exporter/Godot evidence, real UI validation, and release rebuild.
Scope guard: AI, registry, marketplace, cloud, MCP product server, and editor plugins remain out of scope.
Placeholder scan: No TBD/TODO/fill-later placeholders are used.
Type consistency: PackInspectSummary is defined in Rust and TypeScript with camelCase fields; Local Pack Library commands use listLocalPacks/inspectLocalPack wrappers.
Verification gates: Every task has shell tests; Task 1 and Task 6 include Computer Use evidence; Task 7 includes signing, preflight, notarization, stapling, package verification.
```
