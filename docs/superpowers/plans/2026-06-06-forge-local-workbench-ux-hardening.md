# Forge Local Workbench UX Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the installed local Forge workbench easier to understand and verify by adding explicit status feedback around Local Pack Library, first-run sample flow, export validation, and real UI QA evidence.

**Architecture:** Keep Forge as a local-first Tauri desktop app. React owns visible workflow state and user feedback, Tauri commands keep filesystem and pack boundaries, `forge_core` and `forge_pack` remain the media/package engines. CLI and MCP stay deferred; this slice improves the product surface users already touch.

**Tech Stack:** Tauri 2, React 18, TypeScript, Vite, Rust workspace, existing source guard scripts, Computer Use MCP for installed-app verification.

## Execution Result - 2026-06-06

Status: implemented and verified.

```text
Local Pack Library status feedback: implemented
First Run sample copy and Load Sample Path hint: implemented
Export output details and validation result feedback: implemented
Current notarized release package: release-candidates/GameSpriteForge-0.1.0-aarch64-0a5d18c3d1c8-notarized
Current DMG SHA-256: 0a5d18c3d1c8df79ba73e617ed972241cc2208b553227fda56ec124e90b5cc42
Notarization submission: 834c0445-1c45-4201-88d5-a2c99c008714
Installed app: /Applications/Game Sprite Forge.app synchronized from the notarized DMG, stapled, Gatekeeper-accepted, and launched through Computer Use
```

Verification:

```text
npm run test:scripts: pass
npm --workspace apps/mac run build: pass
npm --workspace apps/mac run smoke:ui:mvp: pass
npm --workspace apps/mac run smoke:ui:responsive: pass
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml: pass
scripts/verify-release-package.sh target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg 0a5d18c3d1c8df79ba73e617ed972241cc2208b553227fda56ec124e90b5cc42: pass
Computer Use: mounted-DMG and /Applications app UI checks passed for first-run copy, sample pipeline, Local Pack Library actions, export output details, and Validate Re-import
```

---

## Current Code Truth

```text
Project root: /Users/kartz/Development/Forge
Installed app: /Applications/Game Sprite Forge.app
Current release package: release-candidates/GameSpriteForge-0.1.0-aarch64-0a5d18c3d1c8-notarized
Current UI routes: Forge, Exports, Settings
Current primary workflow: Import -> Extract -> Process/Quality -> Export -> Validate/Re-import
Local Pack Library: present in apps/mac/src/App.tsx with Refresh Library, Inspect, Validate Pack, Re-import Pack, Open
Source providers: import_video, import_frames, import_sprite_sheet, import_gsfpack
Product non-goals for this slice: AI, BYOK, cloud, marketplace, product MCP server, CLI as user surface
Git status: /Users/kartz/Development/Forge is not a git repository, so this plan does not include commit steps
```

## Product Decision

The next development slice is UI-first product hardening, not CLI/MCP. CLI/MCP remains useful as a future internal automation layer, but the current product risk is that local users can complete the flow yet still miss what happened after clicking Refresh, Inspect, Validate, Re-import, Load Sample Path, or Run Sample Pipeline.

## File Structure

- Create: `scripts/test-local-workbench-ux-source.mjs` - source guard for the UI-first workbench contract.
- Modify: `package.json` - add the new source guard to `test:scripts`.
- Modify: `apps/mac/src/App.tsx` - add Local Pack Library status, busy state, success/error messages, and safer action feedback.
- Modify: `apps/mac/src/components/ImportPanel.tsx` - clarify that Load Sample Path only fills the source path.
- Modify: `apps/mac/src/routes/ForgeRoute.tsx` - keep Run Sample Pipeline prominent and improve first-run state copy.
- Modify: `apps/mac/src/components/ExportPanel.tsx` - make export and validation results easier to scan.
- Modify: `apps/mac/src/styles/app.css` - style library status, sample hint, and validation result text.
- Modify: `apps/mac/scripts/smoke-ui.mjs` - assert the new source strings are present in built UI output.
- Create: `docs/qa/local-workbench-ux-evidence-2026-06-06.md` - Computer Use evidence for installed-app workbench UX.
- Modify: `docs/architecture/post-release-backlog.md` - mark this plan as the active next slice and keep CLI/MCP deferred.
- Modify: `docs/architecture/mvp-scope.md` - record the UX hardening evidence after verification.

## Verification Commands

Run this set before marking the full plan complete:

```bash
npm run test:scripts
npm --workspace apps/mac run build
npm --workspace apps/mac run smoke:ui:mvp
npm --workspace apps/mac run smoke:ui:responsive
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml
```

Expected final result:

```text
script source guards: pass
frontend build: pass
MVP UI smoke: pass
responsive UI smoke: pass
Rust tests: pass
Computer Use evidence: recorded for Local Pack Library status, first-run sample pipeline, export validation, and no CLI/MCP product UI
```

---

### Task 1: Add UI-First Source Guard

**Files:**
- Create: `scripts/test-local-workbench-ux-source.mjs`
- Modify: `package.json`

- [ ] **Step 1: Create the source guard**

Create `scripts/test-local-workbench-ux-source.mjs`:

```js
import { existsSync, readFileSync } from "node:fs";

const appSource = readFileSync("apps/mac/src/App.tsx", "utf8");
const importPanelSource = readFileSync("apps/mac/src/components/ImportPanel.tsx", "utf8");
const forgeRouteSource = readFileSync("apps/mac/src/routes/ForgeRoute.tsx", "utf8");
const exportPanelSource = readFileSync("apps/mac/src/components/ExportPanel.tsx", "utf8");
const smokeSource = readFileSync("apps/mac/scripts/smoke-ui.mjs", "utf8");
const backlogSource = readFileSync("docs/architecture/post-release-backlog.md", "utf8");

function assertContains(source, needle, message) {
  if (!source.includes(needle)) {
    throw new Error(message);
  }
}

assertContains(appSource, "type LibraryAction", "Local Pack Library must track the active action.");
assertContains(appSource, "libraryStatus", "Local Pack Library must expose status text.");
assertContains(appSource, "role=\"status\"", "Local Pack Library status must be announced as status text.");
assertContains(appSource, "Scanning local export folder", "Refresh Library must show scanning feedback.");
assertContains(appSource, "Found ${packs.length} local pack", "Refresh Library must show how many packs were found.");
assertContains(appSource, "Pack validation passed", "Validate Pack must show success feedback.");
assertContains(appSource, "Opening local pack in Forge", "Re-import Pack must tell the user it is switching routes.");
assertContains(appSource, "libraryAction === \"refresh\"", "Refresh Library button must expose busy state.");
assertContains(appSource, "formatLibraryError", "Local Pack Library errors must be normalized before display.");

assertContains(importPanelSource, "sample-action-hint", "Load Sample Path must have a visible hint.");
assertContains(importPanelSource, "Fills the video path only", "Sample hint must explain that Load Sample Path is not the full pipeline.");
assertContains(forgeRouteSource, "Run Sample Pipeline", "First Run must keep the full sample pipeline action.");
assertContains(forgeRouteSource, "Full pipeline passed", "Sample pipeline completion must remain explicit.");

assertContains(exportPanelSource, "validation-result", "Export panel must expose validation result copy.");
assertContains(exportPanelSource, "Last validated", "Validation result must be easy to scan after re-import.");

assertContains(smokeSource, "library-status", "UI smoke must assert Local Pack Library status text.");
assertContains(smokeSource, "sample-action-hint", "UI smoke must assert the sample path hint.");
assertContains(smokeSource, "validation-result", "UI smoke must assert validation result output.");

assertContains(backlogSource, "Local Workbench UX Hardening", "Backlog must point at the UI-first next slice.");
assertContains(backlogSource, "CLI/MCP deferred", "Backlog must keep CLI/MCP out of the active product slice.");

if (!existsSync("docs/qa/local-workbench-ux-evidence-2026-06-06.md")) {
  throw new Error("Computer Use evidence must be recorded in docs/qa/local-workbench-ux-evidence-2026-06-06.md");
}

console.log("PASS local workbench UX source test");
```

- [ ] **Step 2: Add the guard to `package.json`**

In `package.json`, update `scripts.test:scripts` by inserting these two commands after `scripts/test-local-pack-library-source.mjs` checks:

```json
"node --check scripts/test-local-workbench-ux-source.mjs && node scripts/test-local-workbench-ux-source.mjs"
```

- [ ] **Step 3: Run the source guard after Tasks 2-6**

Run:

```bash
npm run test:scripts
```

Expected after all implementation tasks:

```text
PASS local workbench UX source test
```

---

### Task 2: Add Local Pack Library Status Feedback

**Files:**
- Modify: `apps/mac/src/App.tsx`
- Modify: `apps/mac/src/styles/app.css`

- [ ] **Step 1: Add action state types and state variables**

In `apps/mac/src/App.tsx`, add this type near `RecordedExport`:

```ts
type LibraryAction = "refresh" | "inspect" | "validate" | "reimport" | "open" | null;
```

Inside `App()`, add these state variables after `libraryPacks`:

```ts
const [libraryStatus, setLibraryStatus] = useState("Refresh the library to scan the default export folder.");
const [libraryAction, setLibraryAction] = useState<LibraryAction>(null);
```

- [ ] **Step 2: Pass status props into `ExportsRoute`**

Update the `ExportsRoute` call:

```tsx
<ExportsRoute
  exports={recentExports}
  libraryAction={libraryAction}
  libraryPacks={libraryPacks}
  libraryStatus={libraryStatus}
  onInspectPack={handleInspectPack}
  onOpenExportFolder={handleOpenExportFolder}
  onRefreshLibrary={handleRefreshLibrary}
  onReimportPack={handleReimportPack}
  onValidatePack={handleValidatePack}
/>
```

Update `ExportsRoute` props:

```ts
libraryAction: LibraryAction;
libraryStatus: string;
```

- [ ] **Step 3: Render the status row and busy labels**

Inside `ExportsRoute`, define:

```ts
const isLibraryBusy = libraryAction !== null;
```

Under the Local Pack Library title block, render:

```tsx
<p className="library-status" role="status">
  {libraryStatus}
</p>
```

Change the Refresh button body to:

```tsx
<FolderOpen size={16} />
{libraryAction === "refresh" ? "Refreshing..." : "Refresh Library"}
```

Set Refresh disabled state:

```tsx
disabled={isLibraryBusy}
```

For each card action, set disabled state and labels:

```tsx
<button
  className="secondary-button export-history-open"
  disabled={isLibraryBusy}
  onClick={() => onInspectPack(pack.root)}
  type="button"
>
  {libraryAction === "inspect" ? "Inspecting..." : "Inspect"}
</button>
<button
  className="secondary-button export-history-open"
  disabled={isLibraryBusy}
  onClick={() => onValidatePack(pack.root)}
  type="button"
>
  {libraryAction === "validate" ? "Validating..." : "Validate Pack"}
</button>
<button
  className="secondary-button export-history-open"
  disabled={isLibraryBusy}
  onClick={() => onReimportPack(pack.root)}
  type="button"
>
  {libraryAction === "reimport" ? "Opening..." : "Re-import Pack"}
</button>
<button
  className="secondary-button export-history-open"
  disabled={isLibraryBusy}
  onClick={() => onOpenExportFolder(pack.root)}
  type="button"
>
  <FolderOpen size={16} />
  {libraryAction === "open" ? "Opening..." : "Open"}
</button>
```

- [ ] **Step 4: Update handlers with status and errors**

Replace the Local Pack Library handlers in `apps/mac/src/App.tsx` with:

```ts
async function handleOpenExportFolder(path: string) {
  setLibraryAction("open");
  setLibraryStatus(`Opening ${fileName(path)}...`);
  try {
    await openFileOrFolder(path);
    setLibraryStatus(`Opened ${fileName(path)}.`);
  } catch (error) {
    setLibraryStatus(formatLibraryError(error));
  } finally {
    setLibraryAction(null);
  }
}

async function handleRefreshLibrary() {
  setLibraryAction("refresh");
  setLibraryStatus("Scanning local export folder...");
  try {
    const packs = await listLocalPacks(settings.defaultOutputFolder);
    setLibraryPacks(packs);
    setLibraryStatus(`Found ${packs.length} local pack${packs.length === 1 ? "" : "s"}.`);
  } catch (error) {
    setLibraryStatus(formatLibraryError(error));
  } finally {
    setLibraryAction(null);
  }
}

async function handleInspectPack(path: string) {
  setLibraryAction("inspect");
  setLibraryStatus(`Inspecting ${fileName(path)}...`);
  try {
    const pack = await inspectLocalPack(path);
    setLibraryPacks((current) => [pack, ...current.filter((item) => item.root !== pack.root)]);
    setLibraryStatus(`Inspected ${pack.name}.`);
  } catch (error) {
    setLibraryStatus(formatLibraryError(error));
  } finally {
    setLibraryAction(null);
  }
}

async function handleValidatePack(path: string) {
  setLibraryAction("validate");
  setLibraryStatus(`Validating ${fileName(path)}...`);
  try {
    await validateGsfpack(path);
    await handleInspectPack(path);
    setLibraryStatus("Pack validation passed.");
  } catch (error) {
    setLibraryStatus(formatLibraryError(error));
  } finally {
    setLibraryAction(null);
  }
}

async function handleReimportPack(path: string) {
  setLibraryAction("reimport");
  setLibraryStatus(`Opening local pack in Forge: ${fileName(path)}.`);
  setQueuedGsfpackImportPath(path);
  setActiveRoute("forge");
  setActiveWorkflow("Frames");
  setLibraryAction(null);
}
```

Add helpers near `loadSettings()`:

```ts
function fileName(path: string) {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

function formatLibraryError(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
```

- [ ] **Step 5: Add CSS**

Add to `apps/mac/src/styles/app.css` near the export route styles:

```css
.library-status {
  margin: 8px 0 0;
  color: var(--text-muted);
  font-size: 12px;
  line-height: 1.45;
}

.export-history-actions .secondary-button:disabled,
.library-title .export-history-open:disabled {
  cursor: default;
  opacity: 0.55;
}
```

- [ ] **Step 6: Validate Task 2**

Run:

```bash
npm --workspace apps/mac run build
```

Expected:

```text
vite build completes without TypeScript errors
```

---

### Task 3: Clarify First-Run Sample Actions

**Files:**
- Modify: `apps/mac/src/components/ImportPanel.tsx`
- Modify: `apps/mac/src/routes/ForgeRoute.tsx`
- Modify: `apps/mac/src/styles/app.css`

- [ ] **Step 1: Add visible hint under Load Sample Path**

In `apps/mac/src/components/ImportPanel.tsx`, replace the panel title block with:

```tsx
<div className="panel-title import-panel-title">
  <span>Import Sources</span>
  <div className="sample-action-group">
    <button className="load-sample-button" disabled={disabled} onClick={onLoadSample} type="button">
      <PlayCircle size={13} />
      Load Sample Path
    </button>
    <small className="sample-action-hint">
      Fills the video path only. Run Sample Pipeline processes and validates the bundled sample.
    </small>
  </div>
</div>
```

- [ ] **Step 2: Make first-run copy route users to the right action**

In `ActivationRail` inside `apps/mac/src/routes/ForgeRoute.tsx`, replace the head copy with:

```tsx
<div className="activation-head">
  <span>First Run</span>
  <small>Run the bundled sample end to end, or import your own local source below.</small>
</div>
```

Keep the primary button label exactly:

```tsx
Run Sample Pipeline
```

- [ ] **Step 3: Add CSS for the sample hint**

Add near `.load-sample-button` in `apps/mac/src/styles/app.css`:

```css
.import-panel-title {
  align-items: flex-start;
  gap: 10px;
}

.sample-action-group {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 5px;
  max-width: 260px;
}

.sample-action-hint {
  color: var(--text-muted);
  font-size: 11px;
  line-height: 1.35;
  text-align: right;
}
```

- [ ] **Step 4: Validate Task 3**

Run:

```bash
npm --workspace apps/mac run build
npm --workspace apps/mac run smoke:ui:mvp
```

Expected:

```text
frontend build passes
UI smoke passed (mvp)
```

---

### Task 4: Improve Export And Validation Result Feedback

**Files:**
- Modify: `apps/mac/src/components/ExportPanel.tsx`
- Modify: `apps/mac/src/styles/app.css`

- [ ] **Step 1: Make validation result a status row**

In `apps/mac/src/components/ExportPanel.tsx`, replace the current `packSummary` paragraph with:

```tsx
{packSummary ? (
  <p className="validation-result" role="status">
    Last validated: {packSummary.name} with {packSummary.frameCount} frames.
  </p>
) : null}
```

- [ ] **Step 2: Make export details easier to scan**

Replace the export output block with:

```tsx
{exportOutput ? (
  <div className="export-output-details" aria-label="Export output details">
    <p className="export-path">Pack: {exportOutput.packDir}</p>
    <p className="export-path">Frames: {exportOutput.framePaths.length}</p>
    <p className="export-path">Sprite sheet pages: {exportOutput.spriteSheetPaths.length}</p>
    <p className="export-path">Godot helper: {exportOutput.godotHelperPath}</p>
  </div>
) : null}
```

- [ ] **Step 3: Add CSS for result rows**

Add near export panel styles:

```css
.export-output-details {
  display: grid;
  gap: 6px;
  margin-top: 10px;
}

.validation-result {
  margin: 10px 0 0;
  color: var(--text-strong);
  font-size: 12px;
  font-weight: 700;
  line-height: 1.4;
}
```

- [ ] **Step 4: Validate Task 4**

Run:

```bash
npm --workspace apps/mac run build
npm --workspace apps/mac run smoke:ui:mvp
```

Expected:

```text
frontend build passes
UI smoke passed (mvp)
```

---

### Task 5: Extend UI Smoke And Real UI Evidence

**Files:**
- Modify: `apps/mac/scripts/smoke-ui.mjs`
- Create: `docs/qa/local-workbench-ux-evidence-2026-06-06.md`

- [ ] **Step 1: Extend built-source smoke assertions**

In `apps/mac/scripts/smoke-ui.mjs`, add these strings to the `mvpRequired` array inside `assertSource(source, smokeMode)`:

```js
"library-status",
"sample-action-hint",
"Run Sample Pipeline",
"validation-result",
```

- [ ] **Step 2: Run smoke checks**

Run:

```bash
npm --workspace apps/mac run build
npm --workspace apps/mac run smoke:ui:mvp
npm --workspace apps/mac run smoke:ui:responsive
```

Expected:

```text
UI smoke passed (mvp)
UI smoke passed (responsive)
```

- [ ] **Step 3: Record Computer Use evidence**

Create `docs/qa/local-workbench-ux-evidence-2026-06-06.md`:

````markdown
# Local Workbench UX Evidence - 2026-06-06

## App

```text
/Applications/Game Sprite Forge.app
```

## Local Pack Library

```text
Refresh Library status:
Inspect status:
Validate Pack status:
Re-import Pack status:
Open status:
Observed result:
```

## First Run

```text
Load Sample Path hint:
Run Sample Pipeline button:
Run Sample Pipeline result:
Observed result:
```

## Export And Validation

```text
Export details:
Validation result:
Observed result:
```

## Scope Guard

```text
Forge UI routes remained Forge, Exports, Settings.
No AI, BYOK, cloud, marketplace, product CLI, or product MCP surface appeared in the installed app.
```
````

- [ ] **Step 4: Use Computer Use MCP to fill the evidence**

Use Computer Use against the installed app and fill the evidence with observed text:

```text
Open /Applications/Game Sprite Forge.app.
Open Exports.
Click Refresh Library.
Click Inspect on one listed local pack.
Click Validate Pack on the same pack.
Click Re-import Pack.
Confirm Forge opens the pack as a live workspace.
Run or verify Run Sample Pipeline from the first-run rail when no live source is active.
Export and Validate Re-import from the Forge panel if a live workspace is available.
Confirm only Forge, Exports, and Settings are visible as product routes.
```

- [ ] **Step 5: Validate evidence file**

Run:

```bash
test -f /Users/kartz/Development/Forge/docs/qa/local-workbench-ux-evidence-2026-06-06.md
rg -n "Refresh Library status|Run Sample Pipeline|Validation result|No AI, BYOK, cloud, marketplace, product CLI, or product MCP" /Users/kartz/Development/Forge/docs/qa/local-workbench-ux-evidence-2026-06-06.md
```

Expected:

```text
All four evidence markers are present.
```

---

### Task 6: Sync Roadmap And Current-State Docs

**Files:**
- Modify: `docs/architecture/post-release-backlog.md`
- Modify: `docs/architecture/mvp-scope.md`
- Modify: `GAME-SPRITE-FORGE-DEVELOPMENT-PLAN-2026-05-31.md`

- [ ] **Step 1: Keep backlog UI-first**

In `docs/architecture/post-release-backlog.md`, ensure the active next slice says:

```markdown
### Slice 4: Local Workbench UX Hardening

Goal: make the existing local app clearer after each user action before expanding to CLI/MCP, AI generation, website, registry, marketplace, or cloud features.
```

Ensure this text appears under the slice:

```text
CLI/MCP deferred until the local workbench UX evidence is complete and stable.
```

- [ ] **Step 2: Add MVP scope evidence line**

Append to `docs/architecture/mvp-scope.md`:

````markdown
## Local Workbench UX Hardening Evidence

```text
Local Pack Library status feedback, first-run sample hints, export validation result feedback, and installed-app Computer Use evidence are tracked in docs/qa/local-workbench-ux-evidence-2026-06-06.md.
CLI/MCP remains deferred and is not part of the public MVP product surface.
```
````

- [ ] **Step 3: Add current-state correction to the historical plan**

Append to `GAME-SPRITE-FORGE-DEVELOPMENT-PLAN-2026-05-31.md`:

````markdown
## 2026-06-06 Current-State Correction

```text
The notarized/stapled local MVP release candidate exists and is package-ready.
The next active development slice is Local Workbench UX Hardening: improve visible feedback for Local Pack Library, first-run sample actions, export validation, and installed-app real UI evidence.
CLI/MCP remains a deferred automation option, not the next product development priority.
```
````

- [ ] **Step 4: Validate docs sync**

Run:

```bash
rg -n "Local Workbench UX Hardening|CLI/MCP deferred|local-workbench-ux-evidence-2026-06-06" /Users/kartz/Development/Forge/docs/architecture/post-release-backlog.md /Users/kartz/Development/Forge/docs/architecture/mvp-scope.md /Users/kartz/Development/Forge/GAME-SPRITE-FORGE-DEVELOPMENT-PLAN-2026-05-31.md
npm run test:scripts
```

Expected:

```text
The three docs reference Local Workbench UX Hardening.
PASS local workbench UX source test.
```

---

## Execution Order

1. Task 1: Add the source guard and wire it into script tests.
2. Task 2: Add Local Pack Library status and busy feedback.
3. Task 3: Clarify first-run sample actions.
4. Task 4: Improve export and validation result feedback.
5. Task 5: Extend smoke checks and record real UI evidence.
6. Task 6: Sync current-state docs.

## Recommended Subagent Split

- **Worker A:** Task 1 and Task 6 source guard/docs sync.
- **Worker B:** Task 2 Local Pack Library UI state and CSS.
- **Worker C:** Task 3 first-run sample copy and Task 4 export validation feedback.
- **Worker D:** Task 5 smoke checks and Computer Use evidence.

Each worker should run the validation commands listed in its task before handing off. The final verifier should run the full verification command set from the top of this document.

## Completion Criteria

```text
Local Pack Library always reports scanning, success, validation, re-import, open, and error states.
Load Sample Path is visibly explained as path-only.
Run Sample Pipeline remains the end-to-end bundled sample action.
Export details and validation result are easy to scan in the Export panel.
Computer Use evidence exists for the installed app.
Backlog says Local Workbench UX Hardening is active and CLI/MCP deferred.
All verification commands pass.
```
