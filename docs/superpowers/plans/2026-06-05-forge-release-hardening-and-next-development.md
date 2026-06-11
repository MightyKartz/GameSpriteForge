# Forge Release Hardening And Next Development Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the current local Game Sprite Forge MVP into a notarized release candidate, prove the import-to-export loop on real local assets, and prepare the next development slice without expanding into AI, website, marketplace, MCP, or cloud work.

**Architecture:** Keep the current Tauri + React macOS app and Rust `forge_core` / `forge_pack` split. Treat local code and tests as source of truth, keep MVP source providers limited to `import_video`, `import_frames`, `import_sprite_sheet`, and `import_gsfpack`, and harden only the local processing, packaging, verification, and user-configured toolchain paths.

**Tech Stack:** Tauri 2, React 18, TypeScript, Vite, Rust workspace, ffmpeg/ffprobe, JSON Schema, macOS Developer ID signing, Apple notarytool/stapler, local smoke testing.

---

## Implementation Status

Updated during multi-agent execution on 2026-06-05:

```text
Task 1 release verifier drift: completed
Task 2 ffmpeg runtime/docs alignment: completed
Task 3 manual QA checklist artifact: completed
Task 4 Settings ffmpeg/ffprobe pickers: completed
Task 5 Rust tool-path guard tests: completed
Task 6 manual real-asset QA run: locally completed/accepted for installed-app MVP QA; installed-app interactive QA is recorded for bundled sample video, a real non-Forge short video, PNG sequence, sprite sheet, exported .gsfpack re-import/export, invalid configured ffmpeg/ffprobe, non-video import, mixed-size PNG normalization rationale, empty PNG sequence, invalid sprite sheet grid, invalid .gsfpack, no-output-folder blocker, dependency recovery, cancel-dialog no-op, blocked-quality export disablement, failed-job Recent Exports integrity, and non-video recovery routing; true-missing/PATH-fallback behavior still needs a clean or scrubbed launch environment
Task 7 notarization/stapling/Gatekeeper public release gate: completed; Apple notarization submission was Accepted, the DMG was stapled, Gatekeeper accepted the DMG and app, mounted-DMG launch passed, and the SHA-prefixed release candidate package was created
Task 8 post-release backlog: completed
```

Code-quality follow-up fixes completed in this pass:

```text
Invalid configured ffmpeg/ffprobe paths now surface dependency-check errors instead of silently falling back to PATH
Configured ffmpeg/ffprobe files must be executable on Unix/macOS
Settings inputs now use explicit label htmlFor/input id wiring, with chooser buttons as sibling controls
Release verification now uses a per-run temporary DMG mountpoint instead of assuming /Volumes/Game Sprite Forge
Manual QA wording now matches the current multi-select PNG frame picker
Invalid-video recovery now routes `Invalid data found when processing input` to valid-video source selection before generic ffmpeg/toolchain recovery
Core export rejects blocked quality reports if a script or stale UI path bypasses the disabled Export Pack button
```

Current verification evidence:

```text
npm --workspace apps/mac run build: pass
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml: pass, including `export_pack_rejects_blocked_quality_report`
npm --workspace apps/mac run smoke:ui:mvp: pass
npm run qa:fixtures: pass
npm run qa:pipeline: pass
npm run test:scripts: pass
npm --workspace apps/mac run tauri -- build: pass
current DMG SHA-256: f2060117105df8477ab8e3eb91f1ba7f1a3a38db5a05f99bbe61bdf630a8116d
hdiutil verify DMG: pass
codesign installed app: pass
bash -n scripts/verify-release-package.sh: pass
bash -n scripts/notarization-preflight.sh: pass
bash -n scripts/package-release-candidate.sh: pass
bash scripts/test-notarization-preflight.sh: pass
scripts/notarization-preflight.sh --keychain-profile GameSpriteForgeNotary target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg: pass; credential method keychain profile, DMG/app signatures pass, stapled ticket present, Gatekeeper DMG accepted
scripts/notarization-preflight.sh supports explicit --keychain-profile for Task 7 handoff without printing secrets or submitting to Apple
xcrun notarytool history --keychain-profile GameSpriteForgeNotary: Successfully received submission history
notarization submission 554dc7d7-6ac9-4f43-a689-d2240a37eb15: Accepted
xcrun stapler staple/validate: pass
spctl DMG assessment: accepted, source=Notarized Developer ID
spctl installed app assessment: accepted, source=Notarized Developer ID
mounted DMG Info.plist identity check: pass
mounted DMG app codesign verification: pass
scripts/verify-release-package.sh target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg: pass; mounted-DMG launch observed after fixing /var versus /private/var process-path detection
scripts/package-release-candidate.sh: created release-candidates/GameSpriteForge-0.1.0-aarch64-f2060117105d-notarized and matching zip; final zip SHA-256 4e58969cea1fdf5b2998d02b6cfa22b2c46461ad5bcaece6a9e9e5806e89b100
local release-candidates directory: current package is release-candidates/GameSpriteForge-0.1.0-aarch64-f2060117105d-notarized; older 2026-06-04 packages remain historical only
installed app manual QA: bundled sample video, real non-Forge short video, PNG sequence, sprite sheet, and exported .gsfpack re-import/export pass; empty PNG sequence, invalid configured ffmpeg, invalid configured ffprobe, non-video import, mixed-size PNG normalization rationale, invalid sprite sheet grid, invalid .gsfpack, no-output-folder blocker, dependency recovery, cancel-dialog no-op, blocked-quality export disablement, failed-job Recent Exports integrity, and non-video recovery routing are recorded; true-missing/PATH-fallback environment behavior remains for a clean or scrubbed launch environment
```

The workspace at `/Users/kartz/Development/Forge` is not currently a git repository, so plan commit steps were not executed. The implementation records the exact commit commands in each task for use after the project is placed under git.

---

## Current Implementation Truth

As of 2026-06-05 local verification:

- Frontend build passes with `npm --workspace apps/mac run build`.
- Rust workspace tests pass with `cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml`.
- MVP UI smoke passes with `npm --workspace apps/mac run smoke:ui:mvp`.
- DMG integrity passes with `hdiutil verify "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"`.
- Installed app passes `codesign --verify --deep --strict --verbose=2 "/Applications/Game Sprite Forge.app"`.
- Gatekeeper accepts the installed app with `source=Notarized Developer ID`.
- Current DMG SHA-256 is `f2060117105df8477ab8e3eb91f1ba7f1a3a38db5a05f99bbe61bdf630a8116d`.

Current product scope:

- Enabled: local video import, PNG sequence import, sprite sheet grid slicing, `.gsfpack` import, ffmpeg/ffprobe checks, probe, extract, chroma preview/process, square-bottom normalization, manual foot anchor, loop range, quality report, export, validate/re-import.
- Deferred: AI generation, BYOK, website, registry, marketplace, MCP, Codex Skill integration, hosted credits, cloud upload, cloud processing, creator publishing.

Resolved drift in this implementation:

- `scripts/verify-release-package.sh` default SHA matches the current DMG.
- `scripts/verify-release-package.sh` launch detection uses the current `CFBundleExecutable`, `Game Sprite Forge`.
- `scripts/verify-release-package.sh` uses a temporary DMG mountpoint for each run.
- `docs/architecture/distribution-mvp.md` describes current configured-path/PATH ffmpeg behavior and treats bundled ffmpeg as deferred.

Remaining release-environment checks:

- Hard public-release gate: satisfied for the current notarized/stapled DMG and SHA-prefixed release candidate package.
- External release-environment checks: true missing ffmpeg/ffprobe on a clean or scrubbed launch environment and PATH fallback in app launch environment still need verification. These are not local installed-app MVP QA blockers; bundled sample video, real non-Forge short video, PNG sequence, sprite sheet, exported `.gsfpack` re-import/export, empty PNG sequence failure, invalid configured ffmpeg/ffprobe, non-video import, mixed-size PNG normalization rationale, invalid sprite sheet grid failure, invalid `.gsfpack` failure, no-output-folder blocker, dependency recovery, cancel-dialog no-op, blocked-quality export disablement, failed-job Recent Exports integrity, and non-video recovery routing have installed-app evidence.

## File Structure

The implementation should keep changes focused in these files:

- `scripts/verify-release-package.sh`: release artifact SHA, executable name, mounted-DMG launch detection.
- `scripts/notarization-preflight.sh`: credential-safe notarization readiness check before Task 7 submission.
- `scripts/package-release-candidate.sh`: public release package creation; must refuse unstapled DMGs.
- `scripts/prepare-manual-qa-fixtures.mjs`: deterministic manual-QA fixture preparation for PNG sequence, sprite sheet, and safe failure states.
- `scripts/run-pre-manual-pipeline-evidence.sh`: automated pre-manual processing/export/import evidence runner for deterministic fixtures.
- `scripts/test-manual-qa-fixtures.mjs`: fixture determinism and unsafe-output regression tests.
- `scripts/test-notarization-preflight.sh`: notarization preflight regression tests.
- `examples/inputs/manual-qa/qa-fixtures.json`: deterministic fixture manifest, including the single-frame blocked-quality input.
- `packages/core/src/export/mod.rs`: core export guardrails, including blocked-quality rejection.
- `docs/architecture/distribution-mvp.md`: current distribution truth, ffmpeg strategy, release gate.
- `docs/architecture/release-candidate-verification.md`: commands and expected release verification results.
- `docs/architecture/mvp-scope.md`: current local MVP truth and verification evidence.
- `docs/qa/forge-manual-mvp-checklist-2026-06-05.md`: manual QA matrix for real local assets.
- `docs/qa/forge-pre-manual-pipeline-evidence-2026-06-05.md`: generated evidence for deterministic fixture pipeline checks; does not replace manual app QA.
- `apps/mac/src/systemDialogs.ts`: optional ffmpeg/ffprobe binary chooser helpers.
- `apps/mac/src/routes/SettingsRoute.tsx`: optional Settings controls for choosing ffmpeg/ffprobe paths.
- `apps/mac/src/App.tsx`: optional handlers connecting Settings binary chooser actions.
- `apps/mac/scripts/smoke-ui.mjs`: UI smoke assertions for any new Settings controls.
- `apps/mac/src-tauri/src/lib.rs`: optional tests or command behavior changes around tool path sanitization and job scoping.
- `packages/core/src/video/ffmpeg.rs`: only change if bundled ffmpeg support is intentionally enabled.
- `packages/core/src/video/sprite_sheet.rs`: shared sprite-sheet grid slicing used by Tauri commands and pre-manual pipeline tests.

## Confirmation Gates

These choices require explicit user confirmation during execution:

- Apple notarization credentials: use an app-specific password/keychain profile or App Store Connect API key.
- ffmpeg strategy: keep user-configured/PATH-only for the first public release, or bundle ffmpeg and handle licensing/package verification now.
- Public release artifact promotion: after notarization and mounted-DMG verification pass, confirm whether to archive the DMG as the first public candidate.

Default assumption for this plan:

- Keep ffmpeg user-configured/PATH-only for the first release.
- Do not add bundled ffmpeg in this development slice.
- Do not add AI, website, marketplace, MCP, Codex Skill, or creator publish work.

---

### Task 1: Fix Release Verification Script Drift

**Files:**
- Modify: `scripts/verify-release-package.sh`
- Verify: `apps/mac/src-tauri/tauri.conf.json`
- Verify: `/Applications/Game Sprite Forge.app/Contents/Info.plist`

- [x] **Step 1: Verify current artifact identity**

Run:

```bash
shasum -a 256 "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
plutil -p "/Applications/Game Sprite Forge.app/Contents/Info.plist" | rg "CFBundleExecutable|CFBundleIdentifier|CFBundleName"
```

Expected:

```text
f2060117105df8477ab8e3eb91f1ba7f1a3a38db5a05f99bbe61bdf630a8116d  target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg
"CFBundleExecutable" => "Game Sprite Forge"
"CFBundleIdentifier" => "dev.gamespriteforge.desktop"
"CFBundleName" => "Game Sprite Forge"
```

- [x] **Step 2: Update verifier constants and process matching**

Change `scripts/verify-release-package.sh` so the top constants are:

```bash
DMG_PATH="$1"
EXPECTED_SHA="${2:-f2060117105df8477ab8e3eb91f1ba7f1a3a38db5a05f99bbe61bdf630a8116d}"
APP_NAME="Game Sprite Forge.app"
APP_EXECUTABLE="Game Sprite Forge"
MOUNT_PATH=""
```

Change cleanup process matching to:

```bash
pkill -f "${MOUNT_PATH}/${APP_NAME}/Contents/MacOS/${APP_EXECUTABLE}" >/dev/null 2>&1 || true
```

Change mounted-DMG launch detection to:

```bash
APP_PID="$(pgrep -f "${MOUNT_PATH}/${APP_NAME}/Contents/MacOS/${APP_EXECUTABLE}" | head -n 1 || true)"
```

- [x] **Step 3: Run verifier and confirm the expected current failure**

Run:

```bash
scripts/verify-release-package.sh "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
```

Expected before notarization:

```text
Actual SHA-256:   f2060117105df8477ab8e3eb91f1ba7f1a3a38db5a05f99bbe61bdf630a8116d
== Disk Image Integrity ==
hdiutil: verify: checksum of "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg" is VALID
== Notarization Ticket ==
```

The command should fail at `xcrun stapler validate` until notarization is completed. It should not fail on SHA mismatch.

- [ ] **Step 4: Commit**

Run:

```bash
git add scripts/verify-release-package.sh
git commit -m "chore: align release verifier with current mac artifact"
```

If this directory is still not a git repository, record this command in the handoff notes instead of committing.

---

### Task 2: Align FFmpeg Strategy Documentation With Runtime

**Files:**
- Modify: `docs/architecture/distribution-mvp.md`
- Modify: `docs/architecture/release-candidate-verification.md`
- Modify: `docs/architecture/mvp-scope.md`
- Verify: `apps/mac/src-tauri/src/lib.rs`
- Verify: `packages/core/src/video/ffmpeg.rs`

- [x] **Step 1: Confirm current runtime behavior**

Run:

```bash
rg -n "bundled_resource_path|configured_ffmpeg_path|configured_ffprobe_path" apps/mac/src-tauri/src/lib.rs packages/core/src/video
```

Expected signal:

```text
apps/mac/src-tauri/src/lib.rs: params.bundled_resource_path = None;
packages/core/src/video/ffmpeg.rs: configured_path
packages/core/src/video/ffmpeg.rs: bundled_resource_path
packages/core/src/video/ffmpeg.rs: find_in_path
```

Interpretation: the core supports bundled resources, but the current Tauri runtime disables bundled ffmpeg/ffprobe for the MVP.

- [x] **Step 2: Update `distribution-mvp.md` dependency wording**

Replace the dependency order section with:

````markdown
## Dependency Check

The current MVP runtime checks dependencies in this order:

```text
configured app setting path
system PATH
```

The Rust core already has a `bundled_resource_path` hook, but the Tauri commands intentionally pass `None` for the first release. Bundled ffmpeg can be enabled in a later packaging slice after licensing, notarization behavior, and helper verification are handled.

The first release acceptance path is either a user-configured ffmpeg/ffprobe path or a working system PATH. Settings must keep the missing dependency state clear and fixable.
````

- [x] **Step 3: Update release checklist wording**

In `docs/architecture/release-candidate-verification.md`, ensure the manual app check includes:

```text
[ ] missing ffmpeg state says: Install ffmpeg or choose an ffmpeg binary in Settings.
[ ] configured ffmpeg and ffprobe paths work
[ ] PATH fallback works when ffmpeg and ffprobe are available in the app launch environment
```

In `docs/architecture/mvp-scope.md`, ensure the acceptance criteria include:

```text
- Allow user-configured ffmpeg/ffprobe paths and PATH fallback.
- Bundled ffmpeg is not required for this release.
```

- [x] **Step 4: Verify docs no longer overpromise bundled ffmpeg**

Run:

```bash
rg -n "bundled ffmpeg|bundled app resource path|bundled_resource_path|bundle.*ffmpeg" docs/architecture
```

Expected: only references that explicitly say bundled ffmpeg is deferred, or that the Rust core has a disabled hook.

- [ ] **Step 5: Commit**

Run:

```bash
git add docs/architecture/distribution-mvp.md docs/architecture/release-candidate-verification.md docs/architecture/mvp-scope.md
git commit -m "docs: align ffmpeg strategy with current runtime"
```

If this directory is still not a git repository, record this command in the handoff notes instead of committing.

---

### Task 3: Add Manual MVP QA Checklist For Real Assets

**Files:**
- Create: `docs/qa/forge-manual-mvp-checklist-2026-06-05.md`
- Reference: `examples/inputs/README.md`
- Reference: `docs/architecture/mvp-scope.md`

- [x] **Step 1: Create QA directory**

Run:

```bash
mkdir -p docs/qa
```

Expected: `docs/qa` exists.

- [x] **Step 2: Create the checklist document**

Create `docs/qa/forge-manual-mvp-checklist-2026-06-05.md` with:

````markdown
# Forge Manual MVP QA Checklist

Date: 2026-06-05

## Scope

This checklist verifies the local import-only MVP on real local assets. It does not cover AI generation, BYOK, website, registry, marketplace, MCP, cloud processing, or creator publishing.

## Environment

```text
App: /Applications/Game Sprite Forge.app
DMG: target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg
Bundle identifier: dev.gamespriteforge.desktop
ffmpeg path:
ffprobe path:
Default output folder:
Tester:
Machine:
macOS version:
```

## Asset Matrix

| Source Type | Input Path | Expected Result | Result | Notes |
| --- | --- | --- | --- | --- |
| Bundled sample video | `examples/inputs/green-box-character.mp4` | Import, probe, extract, process, export, validate re-import | Not run |  |
| Real short MP4/MOV/WebM |  | Import, probe metadata, extract chosen range, process, export | Not run |  |
| PNG sequence |  | Import copied frames, process, export | Not run |  |
| Sprite sheet |  | Slice grid, process, export | Not run |  |
| Exported `.gsfpack` |  | Validate, import, preview GIF, re-export same frame count | Not run |  |

## Per-Asset Checks

For every source type:

```text
[ ] source import succeeds
[ ] live workspace badge appears
[ ] workflow tabs unlock only after required prior steps
[ ] preview frame displays
[ ] frame scrubber changes selected frame
[ ] Process & Quality completes
[ ] quality report shows verdict and recommendations
[ ] export readiness lists blockers before export
[ ] export folder blocker clears after Settings output folder is set
[ ] Export Pack creates frames, sprite_sheet.png, atlas.json, manifest.json, quality-report.json, preview.gif, and .gsfpack
[ ] Validate Re-import succeeds
[ ] Recent Exports records the output
[ ] Open Exports Folder opens the local export directory
```

## Failure State Checks

```text
[ ] missing ffmpeg state says: Install ffmpeg or choose an ffmpeg binary in Settings.
[ ] invalid sprite sheet grid reports source image bounds or grid dimension error
[ ] blocked quality verdict disables export
[ ] recovery card offers an action matching the failed stage
```

## Release Gate Summary

```text
Manual QA result:
Blocking issues:
Non-blocking issues:
Release candidate decision:
```
````

- [x] **Step 3: Use the checklist during a manual app session**

Run the app:

```bash
open -n "/Applications/Game Sprite Forge.app"
```

Fill every `Result` cell with one of:

```text
Pass
Fail
Blocked
Not applicable
```

Every `Fail` or `Blocked` row must include a note with the exact visible error text or the missing file path.

- [ ] **Step 4: Commit**

Run:

```bash
git add docs/qa/forge-manual-mvp-checklist-2026-06-05.md
git commit -m "docs: add manual MVP QA checklist"
```

If this directory is still not a git repository, record this command in the handoff notes instead of committing.

---

### Task 4: Add Settings File Pickers For FFmpeg And FFprobe

**Files:**
- Modify: `apps/mac/src/systemDialogs.ts`
- Modify: `apps/mac/src/routes/SettingsRoute.tsx`
- Modify: `apps/mac/src/App.tsx`
- Modify: `apps/mac/scripts/smoke-ui.mjs`

- [x] **Step 1: Extend system dialogs**

In `apps/mac/src/systemDialogs.ts`, add:

```ts
export async function chooseFfmpegBinary(defaultPath?: string) {
  const path = await openDialog({
    title: "Choose ffmpeg Binary",
    defaultPath: defaultPath || undefined,
    filters: [{ name: "ffmpeg", extensions: ["*"] }],
  });
  return singlePath(path);
}

export async function chooseFfprobeBinary(defaultPath?: string) {
  const path = await openDialog({
    title: "Choose ffprobe Binary",
    defaultPath: defaultPath || undefined,
    filters: [{ name: "ffprobe", extensions: ["*"] }],
  });
  return singlePath(path);
}
```

- [x] **Step 2: Add Settings route callbacks**

In `apps/mac/src/routes/SettingsRoute.tsx`, change the props type to:

```ts
type SettingsRouteProps = {
  onChooseFfmpegPath: () => void;
  onChooseFfprobePath: () => void;
  onChooseOutputFolder: () => void;
  settings: LocalSettings;
  onSettingsChange: (settings: LocalSettings) => void;
};
```

Change the function signature to:

```ts
export function SettingsRoute({
  onChooseFfmpegPath,
  onChooseFfprobePath,
  onChooseOutputFolder,
  settings,
  onSettingsChange,
}: SettingsRouteProps) {
```

Add `onChoose={onChooseFfmpegPath}` to the ffmpeg field and `onChoose={onChooseFfprobePath}` to the ffprobe field. Change `SettingsField` so its button label can vary:

```ts
function SettingsField({
  chooseLabel = "Choose Folder",
  icon,
  inputMode,
  label,
  onChange,
  onChoose,
  placeholder,
  value,
}: {
  chooseLabel?: string;
  icon: ReactNode;
  inputMode?: HTMLAttributes<HTMLInputElement>["inputMode"];
  label: string;
  onChange: (value: string) => void;
  onChoose?: () => void;
  placeholder?: string;
  value: string;
}) {
```

Render the button with:

```tsx
{onChoose ? (
  <button className="choose-path-button" onClick={onChoose} type="button">
    {chooseLabel}
  </button>
) : null}
```

Use these labels:

```tsx
chooseLabel="Choose ffmpeg"
chooseLabel="Choose ffprobe"
```

- [x] **Step 3: Wire App handlers**

In `apps/mac/src/App.tsx`, change imports to:

```ts
import {
  chooseFfmpegBinary,
  chooseFfprobeBinary,
  chooseOutputFolder,
  openFileOrFolder,
} from "./systemDialogs";
```

Add:

```ts
async function handleChooseFfmpegPath() {
  const path = await chooseFfmpegBinary(settings.ffmpegPath);
  if (path) {
    setSettings((current) => ({ ...current, ffmpegPath: path }));
  }
}

async function handleChooseFfprobePath() {
  const path = await chooseFfprobeBinary(settings.ffprobePath);
  if (path) {
    setSettings((current) => ({ ...current, ffprobePath: path }));
  }
}
```

Pass props:

```tsx
<SettingsRoute
  onChooseFfmpegPath={handleChooseFfmpegPath}
  onChooseFfprobePath={handleChooseFfprobePath}
  onChooseOutputFolder={handleChooseOutputFolder}
  onSettingsChange={setSettings}
  settings={settings}
/>
```

- [x] **Step 4: Update UI smoke requirements**

In `apps/mac/scripts/smoke-ui.mjs`, add to `mvpRequired`:

```js
"Choose ffmpeg",
"Choose ffprobe",
```

- [x] **Step 5: Verify**

Run:

```bash
npm --workspace apps/mac run build
npm --workspace apps/mac run smoke:ui:mvp
```

Expected:

```text
✓ built
UI smoke passed (mvp).
```

- [ ] **Step 6: Commit**

Run:

```bash
git add apps/mac/src/systemDialogs.ts apps/mac/src/routes/SettingsRoute.tsx apps/mac/src/App.tsx apps/mac/scripts/smoke-ui.mjs
git commit -m "feat: add ffmpeg binary pickers in settings"
```

If this directory is still not a git repository, record this command in the handoff notes instead of committing.

---

### Task 5: Add Rust Coverage For Tauri Path And Tool Guards

**Files:**
- Modify: `apps/mac/src-tauri/src/lib.rs`

- [x] **Step 1: Add tests for tool path sanitization**

Inside the existing `#[cfg(test)] mod tests` in `apps/mac/src-tauri/src/lib.rs`, add:

```rust
#[test]
fn configured_tool_path_must_match_expected_binary_name() {
    let temp = tempfile::tempdir().unwrap();
    let wrong = temp.path().join("not-ffmpeg");
    std::fs::write(&wrong, b"").unwrap();

    let error = sanitize_configured_tool_path("ffmpeg", Some(wrong)).unwrap_err();

    assert_eq!(error, "configured tool path must point to ffmpeg");
}

#[test]
fn configured_tool_path_canonicalizes_matching_file() {
    let temp = tempfile::tempdir().unwrap();
    let binary = temp.path().join("ffmpeg");
    std::fs::write(&binary, b"").unwrap();

    let resolved = sanitize_configured_tool_path("ffmpeg", Some(binary.clone()))
        .unwrap()
        .unwrap();

    assert_eq!(resolved, binary.canonicalize().unwrap());
}
```

- [x] **Step 2: Run targeted tests**

Run:

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/apps/mac/src-tauri/Cargo.toml configured_tool_path -- --nocapture
```

Expected:

```text
test tests::configured_tool_path_must_match_expected_binary_name ... ok
test tests::configured_tool_path_canonicalizes_matching_file ... ok
```

- [x] **Step 3: Run full Rust tests**

Run:

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml
```

Expected:

```text
test result: ok
```

- [ ] **Step 4: Commit**

Run:

```bash
git add apps/mac/src-tauri/src/lib.rs
git commit -m "test: cover tauri tool path guards"
```

If this directory is still not a git repository, record this command in the handoff notes instead of committing.

---

### Task 6: Run Real-Asset Manual QA And Capture Evidence

**Files:**
- Modify: `docs/qa/forge-manual-mvp-checklist-2026-06-05.md`
- Modify: `docs/architecture/mvp-scope.md`
- Modify: `docs/architecture/release-candidate-verification.md`

- [x] **Step 1: Prepare output folder**

Run:

```bash
mkdir -p "$HOME/Game Sprite Forge/Exports"
open -n "/Applications/Game Sprite Forge.app"
```

Expected: app launches and Settings can point default output folder to `$HOME/Game Sprite Forge/Exports`.

- [x] **Step 1A: Record deterministic pre-manual pipeline evidence**

Run:

```bash
npm run qa:pipeline
```

Expected:

```text
manual_qa_png_sequence_exports_schema_valid_reimportable_pack ... ok
manual_qa_sprite_sheet_exports_schema_valid_reimportable_pack ... ok
Pre-manual pipeline evidence written to docs/qa/forge-pre-manual-pipeline-evidence-2026-06-05.md
```

This proves the Rust processing/export/import pipeline for deterministic PNG sequence and sprite sheet fixtures. It does not satisfy the interactive app QA session, bundled sample app QA, real short video app QA, or final release decision rows.

- [x] **Step 2: Execute all checklist rows**

Use `docs/qa/forge-manual-mvp-checklist-2026-06-05.md` and run each source type through:

```text
Import
Extract or slice
Process & Quality
Export Pack
Validate Re-import
Open Exports Folder
```

Expected: every row is marked `Pass`, `Fail`, `Blocked`, or `Not applicable`, and every non-pass row has an exact note.

Current app-session status: source-provider rows pass for bundled sample video, a real non-Forge short video, PNG sequence, sprite sheet, and exported `.gsfpack`; failure-state rows are recorded or accepted for empty PNG sequence input, invalid configured ffmpeg, invalid configured ffprobe, non-video import, mixed-size PNG normalization rationale, invalid sprite sheet grid, invalid `.gsfpack`, no-output-folder blocker, recovery-card display, dependency recovery, cancel-dialog no-op, blocked-quality export disablement, failed-job Recent Exports integrity, and non-video recovery routing. Step 2 is locally accepted for installed-app MVP QA; true missing/PATH-fallback behavior still needs a clean or scrubbed launch environment.

- [x] **Step 3: Record final evidence in architecture docs**

Append this evidence block to `docs/architecture/mvp-scope.md` after the latest verification evidence:

````markdown
## Manual Real-Asset QA Evidence

Recorded on 2026-06-05:

```text
Bundled sample video:
Real short video:
PNG sequence:
Sprite sheet:
Exported .gsfpack:
Blocking issues:
Release decision:
```
````

In `docs/architecture/release-candidate-verification.md`, update the manual app check entries from unchecked to checked only for checks that actually passed.

- [ ] **Step 4: Commit**

Run:

```bash
git add docs/qa/forge-manual-mvp-checklist-2026-06-05.md docs/architecture/mvp-scope.md docs/architecture/release-candidate-verification.md
git commit -m "docs: record manual MVP QA evidence"
```

If this directory is still not a git repository, record this command in the handoff notes instead of committing.

---

### Task 7: Build, Notarize, Staple, And Verify Public Candidate

**Files:**
- Modify: `docs/architecture/distribution-mvp.md`
- Modify: `docs/architecture/release-candidate-verification.md`
- Verify: `apps/mac/src-tauri/tauri.conf.json`
- Verify: `scripts/verify-release-package.sh`

- [x] **Step 1: Confirm notarization credential method**

Choose one method and record it in the handoff notes:

```text
Method A: notarytool keychain profile
Method B: App Store Connect API key
```

For Method A, create the profile:

```bash
xcrun notarytool store-credentials "GameSpriteForgeNotary" \
  --apple-id "$APPLE_ID" \
  --team-id "$APPLE_TEAM_ID" \
  --password "$APPLE_APP_SPECIFIC_PASSWORD"
```

Expected:

```text
Credentials saved to keychain
```

Then verify the profile without printing secrets:

```bash
xcrun notarytool history --keychain-profile "GameSpriteForgeNotary"
```

Expected after credentials are stored:

```text
Successfully received submission history
```

Current local check:

```text
Successfully received submission history
```

- [x] **Step 1A: Run credential-safe preflight**

Run:

```bash
bash scripts/notarization-preflight.sh \
  --keychain-profile "GameSpriteForgeNotary" \
  "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg" \
  "$(shasum -a 256 "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg" | awk '{print $1}')"
```

Expected before notarization:

```text
Credential method: keychain profile
Stapled ticket: missing
Preflight complete. This did not submit, staple, or claim notarization passed.
```

If no credential method is configured, keep Task 7 open and record the missing credential method in the handoff notes.

The `--keychain-profile` flag selects the profile name explicitly for preflight output and next-command generation. It does not verify that the keychain item exists, submit to Apple, staple, or claim notarization passed. Use `xcrun notarytool history --keychain-profile "GameSpriteForgeNotary"` after `store-credentials` to prove the keychain profile exists.

- [x] **Step 2: Build release DMG**

Run:

```bash
npm --workspace apps/mac run tauri -- build
```

Expected:

```text
Finished 1 bundle at:
target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg
```

- [x] **Step 3: Submit DMG for notarization**

Run:

```bash
xcrun notarytool submit "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg" \
  --keychain-profile "GameSpriteForgeNotary" \
  --wait
```

Expected:

```text
status: Accepted
```

Observed:

```text
Submission ID: 554dc7d7-6ac9-4f43-a689-d2240a37eb15
status: Accepted
statusSummary: Ready for distribution
issues: null
```

If the status is not `Accepted`, run:

```bash
xcrun notarytool log "$SUBMISSION_ID" --keychain-profile "GameSpriteForgeNotary"
```

Record the exact issue paths and messages in the handoff notes.

- [x] **Step 4: Staple and verify ticket**

Run:

```bash
xcrun stapler staple "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
xcrun stapler validate -v "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
```

Expected:

```text
The staple and validate action worked!
```

Observed:

```text
xcrun stapler staple -v: pass
xcrun stapler validate -v: pass
Post-staple DMG SHA-256: f2060117105df8477ab8e3eb91f1ba7f1a3a38db5a05f99bbe61bdf630a8116d
```

- [x] **Step 5: Run release verifier**

Run:

```bash
scripts/verify-release-package.sh "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg" \
  "$(shasum -a 256 "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg" | awk '{print $1}')"
```

Expected:

```text
PASS release package verification script completed.
```

Observed:

```text
scripts/verify-release-package.sh: pass
Gatekeeper DMG assessment: accepted, source=Notarized Developer ID
Gatekeeper app assessment: accepted, source=Notarized Developer ID
mounted-DMG launch: pass
```

- [x] **Step 5A: Verify package guard before stapling**

Run:

```bash
scripts/package-release-candidate.sh
```

Expected after notarization and stapling:

```text
Release candidate package created:
```

Expected before notarization and stapling:

```text
DMG is not notarized/stapled; run Task 7 notarization and stapling before packaging.
```

Current status: the package guard was verified before stapling, and after stapling the script created `release-candidates/GameSpriteForge-0.1.0-aarch64-f2060117105d-notarized` plus a matching zip. Older files under `release-candidates/` belong to the 2026-06-04 package lineage.

- [x] **Step 6: Update distribution docs**

In `docs/architecture/distribution-mvp.md`, update:

```text
SHA-256:
notarization:
Gatekeeper app assessment:
mounted-DMG launch:
release candidate decision:
```

In `docs/architecture/release-candidate-verification.md`, update the artifact SHA and verification result block with the notarized/stapled evidence.

- [ ] **Step 7: Commit**

Run:

```bash
git add docs/architecture/distribution-mvp.md docs/architecture/release-candidate-verification.md
git commit -m "docs: record notarized release candidate evidence"
```

Current status: not executed because `/Users/kartz/Development/Forge` is not currently a git repository.

If this directory is still not a git repository, record this command in the handoff notes instead of committing.

---

### Task 8: Create Post-Release Development Backlog

**Files:**
- Create: `docs/architecture/post-release-backlog.md`
- Reference: `GAME-SPRITE-FORGE-DEVELOPMENT-PLAN-2026-05-31.md`
- Reference: `GAME-SPRITE-FORGE-UI-DESIGN-SPEC.md`
- Reference: `docs/architecture/mvp-scope.md`

- [x] **Step 1: Create backlog document**

Create `docs/architecture/post-release-backlog.md` with:

````markdown
# Post-Release Backlog

Date: 2026-06-05

## Scope Guard

The first public release remains import-only and local-first. AI generation, BYOK, website, registry, marketplace, MCP, Codex Skill, cloud upload, and creator publishing stay deferred until the local pack format and release pipeline are stable.

## Next Development Slices

### Slice 1: Local Pack Library

Goal: let users inspect previously exported `.gsfpack` folders without using online registry features.

Entry criteria:

```text
notarized release candidate verified
manual real-asset QA has no P0/P1 blockers
```

Deliverables:

```text
local pack list
pack inspect view
validate selected pack
open pack export folder
re-import selected pack into Forge
```

Verification:

```bash
npm --workspace apps/mac run build
npm --workspace apps/mac run smoke:ui:mvp
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml
```

### Slice 2: Exporter Refinement

Goal: improve generic and Godot helper outputs before adding more engines.

Entry criteria:

```text
local pack library can inspect and validate exported packs
at least two real packs have been re-imported successfully
```

Deliverables:

```text
clear manifest versioning
Godot helper checked against a small Godot sample project
export preview evidence stored in docs/qa
```

### Slice 3: Bundled FFmpeg Evaluation

Goal: decide whether to bundle ffmpeg or keep user-configured/PATH-only.

Entry criteria:

```text
first public release shipped or release candidate ready
license review completed for ffmpeg distribution mode
```

Deliverables:

```text
licensing decision
bundle layout proposal
notarization behavior tested
fallback order tested
```

### Slice 4: CLI/MCP Feasibility Spike

Goal: validate automation only after local app and pack workflows are stable.

Entry criteria:

```text
local pack library and exporter refinement are complete
core operations are callable without UI-only state
```

Deliverables:

```text
CLI command map
MCP command map
cost-free local-only automation contract
write-before-confirmation rules
```

## Explicit Non-Goals For The Next Slice

```text
AI image generation
AI video generation
BYOK settings
online pack registry
marketplace
creator publish
cloud processing
hosted credits
```
````

- [x] **Step 2: Verify backlog matches MVP guardrails**

Run:

```bash
rg -n "AI|BYOK|marketplace|MCP|cloud|registry|creator publish" docs/architecture/post-release-backlog.md docs/architecture/mvp-scope.md
```

Expected: every hit either appears under deferred/non-goal sections or describes a future feasibility spike after release stabilization.

- [ ] **Step 3: Commit**

Run:

```bash
git add docs/architecture/post-release-backlog.md
git commit -m "docs: add post-release development backlog"
```

If this directory is still not a git repository, record this command in the handoff notes instead of committing.

---

## Multi-Agent Execution Option

Use disjoint scopes so agents do not edit the same files at the same time:

```text
Release Engineer:
  Task 1, Task 7
  Files: scripts/verify-release-package.sh, docs/architecture/distribution-mvp.md, docs/architecture/release-candidate-verification.md

Docs/Scope Engineer:
  Task 2, Task 8
  Files: docs/architecture/*

QA Engineer:
  Task 3, Task 6
  Files: docs/qa/*, docs/architecture/mvp-scope.md

UI Toolchain Engineer:
  Task 4
  Files: apps/mac/src/systemDialogs.ts, apps/mac/src/routes/SettingsRoute.tsx, apps/mac/src/App.tsx, apps/mac/scripts/smoke-ui.mjs

Rust Guard Engineer:
  Task 5
  Files: apps/mac/src-tauri/src/lib.rs
```

Merge order:

```text
1. Task 1
2. Task 2
3. Task 3
4. Task 4 and Task 5 in parallel
5. Task 6
6. Task 7
7. Task 8
```

## Verification Commands

Run before claiming the release-hardening slice is complete:

```bash
npm --workspace apps/mac run build
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml
npm --workspace apps/mac run smoke:ui:mvp
hdiutil verify "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
codesign --verify --deep --strict --verbose=2 "/Applications/Game Sprite Forge.app"
scripts/verify-release-package.sh "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg" \
  "$(shasum -a 256 "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg" | awk '{print $1}')"
```

Expected final state after notarization:

```text
frontend build: pass
Rust tests: pass
MVP UI smoke: pass
DMG hdiutil verify: pass
installed or mounted app codesign: pass
stapler validate: pass
Gatekeeper DMG assessment: pass
Gatekeeper app assessment: pass
mounted-DMG launch: pass
manual real-asset QA: no P0/P1 blockers
```

## First Active Task

Start with Task 1. It is low-risk, fixes a real release verifier drift, and makes later notarization verification trustworthy.

## Self-Review

Spec coverage:

- Current code reality is captured in `Current Implementation Truth`.
- Release blocker is covered by Task 7.
- Script drift is covered by Task 1.
- ffmpeg runtime/doc mismatch is covered by Task 2.
- Manual real-asset QA is covered by Task 3 and Task 6.
- Settings toolchain improvement is covered by Task 4.
- Rust guard coverage is covered by Task 5.
- Post-release development direction is covered by Task 8.

Placeholder scan:

- This plan does not use open-ended task wording.
- Every task has exact files, commands, and expected outcomes.

Type consistency:

- TypeScript functions use existing `LocalSettings` fields: `ffmpegPath`, `ffprobePath`, `defaultOutputFolder`, `defaultFps`, `defaultSheetSize`.
- Tauri/Rust guard tests call existing private helper `sanitize_configured_tool_path` from the same test module.
- Release script constants match the current app bundle name and executable name.
