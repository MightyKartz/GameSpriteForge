# Forge Next Development Roadmap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move Game Sprite Forge from a notarized import-only MVP candidate into a reliable local asset workbench by hardening the local pack library, reconciling documentation drift, proving installed-app workflows with real UI evidence, and preparing exporter/automation decisions without adding AI, cloud, marketplace, or product MCP features.

**Architecture:** Keep the app local-first: React/Tauri owns UI state, Rust commands own filesystem boundaries, `forge_core` owns media/export behavior, and `forge_pack` owns `.gsfpack` validation/import. The next phase should strengthen boundaries and evidence before broadening product scope; MCP is used as a development and verification tool, not as a user-facing product surface in this slice.

**Tech Stack:** Tauri 2, React 18, TypeScript, Vite, Rust workspace, `forge_core`, `forge_pack`, JSON Schema, ffmpeg/ffprobe, macOS Developer ID notarization, Computer Use MCP for installed-app QA, multi-agent MCP for disjoint implementation/review tasks.

---

## Current Code Truth

```text
Project root: /Users/kartz/Development/Forge
Installed app: /Applications/Game Sprite Forge.app
Current candidate DMG: target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg
Current release package: release-candidates/GameSpriteForge-0.1.0-aarch64-218a5f9adf1f-notarized
Current DMG SHA-256: 218a5f9adf1fb47e5f744efbda357470ac8c6aa2c065451c926842c35cd0c739
Current release zip SHA-256: fa3b9331eaafe513ebd08d0ae5dc07cb8324669e65d3a2e7a827286666e3ad00
Current public release gate: notarized, stapled, Gatekeeper accepted, mounted-DMG launch verified
Enabled source providers: import_video, import_frames, import_sprite_sheet, import_gsfpack
Enabled export outputs: frames, sprite sheet pages, atlas.json, manifest.json, quality-report.json, preview.gif, .gsfpack
Current UI routes: Forge, Exports, Settings
Current Exports route direction: Local Pack Library has begun in code
```

## Documentation Drift To Resolve

```text
PRODUCT.md now matches the current notarized, stapled, package-ready release status.
docs/architecture/mvp-scope.md and docs/architecture/release-candidate-verification.md say notarization is accepted and package-ready.
docs/qa/forge-post-rc-ui-smoke-2026-06-05.md now records deterministic missing-ffmpeg Computer Use evidence as Pass.
docs/architecture/post-release-backlog.md now records first-pass Local Pack Library, Godot helper schema evidence, and Godot editor/sample-project validation as completed post-RC work.
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
product MCP server
Codex Skill product integration
Unity editor plugin
Godot editor plugin
```

## Skill And MCP Operating Model

```text
Use project-codebase-onboarding-and-roadmap before changing roadmap or current-state docs.
Use superpowers:test-driven-development for Tasks 2, 3, and 5.
Use superpowers:subagent-driven-development for execution when tasks have disjoint file ownership.
Use computer-use:computer-use for installed-app verification in Task 4.
Use build-macos-apps:packaging-notarization only when a rebuilt DMG must be re-signed, notarized, stapled, or verified.
Use multi_agent_v1 worker agents for bounded implementation slices, and reviewer agents for spec/quality reviews after each slice.
Do not use Figma MCP for this plan because there is no new design import.
Do not use GitHub MCP unless the user asks to publish, open PRs, or inspect remote CI.
```

## File Structure

- Modify: `PRODUCT.md` - correct release-gate status and next blockers.
- Modify: `GAME-SPRITE-FORGE-DEVELOPMENT-PLAN-2026-05-31.md` - keep historical plan but append current-state correction.
- Modify: `docs/architecture/post-release-backlog.md` - move first-pass Local Pack Library from future-only to active hardening.
- Modify: `docs/architecture/mvp-scope.md` - record post-library status after verification.
- Modify: `docs/architecture/release-candidate-verification.md` - add missing-ffmpeg and PATH fallback evidence once verified.
- Modify: `docs/qa/forge-post-rc-ui-smoke-2026-06-05.md` - replace pending rows with Computer Use observations.
- Modify: `apps/mac/src-tauri/src/lib.rs` - harden local pack listing against symlinks and add behavioral tests.
- Modify: `packages/pack/src/lib.rs` - make pack validation reject symlink escapes and non-regular pack files.
- Modify: `packages/pack/tests/pack_tests.rs` - add symlink validation tests.
- Modify: `scripts/test-local-pack-library-source.mjs` - upgrade source guard for safe local-library behavior and re-import wiring.
- Modify: `apps/mac/src/App.tsx` - improve Local Pack Library action feedback and refresh behavior.
- Modify: `apps/mac/src/styles/app.css` - keep Local Pack Library actions stable on narrow desktop widths.
- Create: `docs/qa/local-pack-library-evidence-2026-06-05.md` - record local library installed-app and command evidence.
- Create: `docs/qa/godot-editor-helper-evidence-2026-06-06.md` - record Godot editor/sample-project validation when run.
- Create: `docs/architecture/cli-mcp-feasibility.md` - document CLI/MCP feasibility and product deferral gates.

## Verification Commands

Run this set before marking the full plan complete:

```bash
npm run test:scripts
npm --workspace apps/mac run build
npm --workspace apps/mac run smoke:ui:mvp
cargo fmt --manifest-path /Users/kartz/Development/Forge/Cargo.toml --all -- --check
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml
cargo check --manifest-path /Users/kartz/Development/Forge/apps/mac/src-tauri/Cargo.toml
bash scripts/notarization-preflight.sh --keychain-profile GameSpriteForgeNotary "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
scripts/verify-release-package.sh "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
```

Expected final result:

```text
script source guards: pass
frontend build: pass
MVP UI smoke: pass
Rust format check: pass
Rust tests: pass
Tauri Rust check: pass
notarization preflight: pass for the current or rebuilt candidate
release verifier: pass for the notarized/stapled candidate
Computer Use evidence: recorded for missing ffmpeg, PATH fallback, Local Pack Library inspect/validate/re-import
```

---

### Task 1: Reconcile Current-State Documentation

**Files:**
- Modify: `PRODUCT.md`
- Modify: `GAME-SPRITE-FORGE-DEVELOPMENT-PLAN-2026-05-31.md`
- Modify: `docs/architecture/post-release-backlog.md`
- Modify: `docs/architecture/mvp-scope.md`
- Modify: `docs/architecture/release-candidate-verification.md`

- [ ] **Step 1: Write the current-state replacement text**

Use this wording in `PRODUCT.md` under `## Current MVP Status`:

```markdown
## Current MVP Status

As of 2026-06-05, the project has a working Tauri + React macOS MVP installed locally at `/Applications/Game Sprite Forge.app`.

Implemented:

- Local source intake for video, PNG sequences, sprite sheets, and `.gsfpack` folders.
- ffmpeg/ffprobe dependency check with user-configurable paths and deterministic clean-environment QA controls.
- Video probe, frame extraction, chroma preview, batch chroma processing, square-bottom normalization, manual foot anchor support, loop range selection, quality report generation, and export.
- `.gsfpack` create/import/validate/re-import flow backed by local schemas and Rust tests.
- UI truth pass: demo data is labeled, unavailable workflow steps are locked, export readiness lists blockers, live workspaces show a compact Run Summary, and failed pipeline states provide recovery actions.
- Developer ID signed, notarized, stapled DMG build and local `/Applications` install.

Current release status:

- The current release package is `release-candidates/GameSpriteForge-0.1.0-aarch64-218a5f9adf1f-notarized`.
- The current DMG SHA-256 is `218a5f9adf1fb47e5f744efbda357470ac8c6aa2c065451c926842c35cd0c739`; the current release zip SHA-256 is `fa3b9331eaafe513ebd08d0ae5dc07cb8324669e65d3a2e7a827286666e3ad00`.
- Gatekeeper accepts the current DMG and installed app as `source=Notarized Developer ID`.
- Deterministic missing ffmpeg/ffprobe QA, PATH/default-directory recovery, and Local Pack Library installed-app evidence are recorded; a true clean-Mac pass remains useful external confidence before broad distribution.
```

- [ ] **Step 2: Run a drift scan**

Run:

```bash
rg -n "<stale release blocker phrases>|Local Pack Library|missing ffmpeg|Pending" PRODUCT.md GAME-SPRITE-FORGE-DEVELOPMENT-PLAN-2026-05-31.md docs
```

Expected:

```text
Only historical notes and explicit pending QA rows remain. Current-state sections no longer claim notarization is an active blocker.
```

- [ ] **Step 3: Update the post-release backlog slice labels**

In `docs/architecture/post-release-backlog.md`, replace the `### Slice 1: Local Pack Library` heading and goal with:

```markdown
### Slice 1: Local Pack Library Hardening

Goal: finish the first-pass local `.gsfpack` library by making scanning symlink-safe, adding action feedback, and recording installed-app inspect/validate/re-import evidence.
```

- [ ] **Step 4: Run documentation source guards**

Run:

```bash
npm run test:scripts
```

Expected:

```text
PASS manual QA fixture test
PASS import panel source test
PASS recovery plan source test
PASS ffmpeg resolver source test
PASS local pack library source test
PASS notarization preflight test
```

### Task 2: Harden `.gsfpack` Validation Against Symlink Escapes

**Files:**
- Modify: `packages/pack/src/lib.rs`
- Modify: `packages/pack/tests/pack_tests.rs`

- [ ] **Step 1: Write failing pack symlink tests**

Add these imports in `packages/pack/tests/pack_tests.rs`:

```rust
#[cfg(unix)]
use std::os::unix::fs::symlink;
```

Add these tests below `missing_required_layout_file_fails_validation`:

```rust
#[cfg(unix)]
#[test]
fn symlinked_required_file_fails_validation() {
    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    write_pack_fixture(&pack, 1);
    let outside = temp.path().join("outside-forgepack.json");
    fs::write(&outside, fs::read(pack.join("forgepack.json")).unwrap()).unwrap();
    fs::remove_file(pack.join("forgepack.json")).unwrap();
    symlink(&outside, pack.join("forgepack.json")).unwrap();

    let error = validate_pack_layout(&pack).unwrap_err();

    assert!(matches!(error, PackError::InvalidAssetPath(path) if path == "forgepack.json"));
}

#[cfg(unix)]
#[test]
fn symlinked_frames_directory_fails_validation() {
    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    write_pack_fixture(&pack, 1);
    let outside_frames = temp.path().join("outside-frames");
    fs::create_dir(&outside_frames).unwrap();
    fs::write(outside_frames.join("frame_001.png"), b"png").unwrap();
    fs::remove_dir_all(pack.join("assets/frames")).unwrap();
    symlink(&outside_frames, pack.join("assets/frames")).unwrap();

    let error = validate_pack_layout(&pack).unwrap_err();

    assert!(matches!(error, PackError::InvalidAssetPath(path) if path == "assets/frames"));
}
```

- [ ] **Step 2: Run tests to verify the gap**

Run:

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml symlinked_required_file_fails_validation symlinked_frames_directory_fails_validation
```

Expected before implementation:

```text
Both tests fail because validation follows symlinked required entries.
```

- [ ] **Step 3: Add safe pack-entry helpers**

In `packages/pack/src/lib.rs`, add these helpers near `expect_asset_path`:

```rust
fn require_regular_pack_file(pack_path: &Path, relative: &str) -> Result<PathBuf, PackError> {
    require_pack_child(pack_path, relative, true)
}

fn require_pack_directory(pack_path: &Path, relative: &str) -> Result<PathBuf, PackError> {
    require_pack_child(pack_path, relative, false)
}

fn require_pack_child(
    pack_path: &Path,
    relative: &str,
    must_be_file: bool,
) -> Result<PathBuf, PackError> {
    let root = fs::canonicalize(pack_path)?;
    let path = pack_path.join(relative);
    let metadata = fs::symlink_metadata(&path).map_err(|_| PackError::MissingFile(relative.to_string()))?;
    if metadata.file_type().is_symlink() {
        return Err(PackError::InvalidAssetPath(relative.to_string()));
    }
    if must_be_file && !metadata.is_file() {
        return Err(PackError::MissingFile(relative.to_string()));
    }
    if !must_be_file && !metadata.is_dir() {
        return Err(PackError::MissingFile(relative.to_string()));
    }
    let canonical = fs::canonicalize(&path)?;
    if !canonical.starts_with(&root) {
        return Err(PackError::InvalidAssetPath(relative.to_string()));
    }
    Ok(canonical)
}
```

- [ ] **Step 4: Use safe helpers in layout validation**

In `validate_pack_layout`, replace the initial existence checks with:

```rust
let pack_metadata = fs::symlink_metadata(pack_path).map_err(|_| PackError::MissingPack(pack_path.to_path_buf()))?;
if pack_metadata.file_type().is_symlink() || !pack_metadata.is_dir() {
    return Err(PackError::NotDirectory(pack_path.to_path_buf()));
}

for relative in REQUIRED_FILES {
    if *relative == "assets/frames" {
        require_pack_directory(pack_path, relative)?;
    } else {
        require_regular_pack_file(pack_path, relative)?;
    }
}
```

Replace the optional Godot helper existence check with:

```rust
require_regular_pack_file(pack_path, godot_helper)?;
```

Replace `require_page_image_file` with:

```rust
fn require_page_image_file(pack_path: &Path, image: &str) -> Result<(), PackError> {
    let relative = format!("assets/{image}");
    require_regular_pack_file(pack_path, &relative)?;
    Ok(())
}
```

- [ ] **Step 5: Make frame enumeration skip symlinks**

Replace `frame_pngs` with:

```rust
fn frame_pngs(pack_path: &Path) -> Result<Vec<PathBuf>, PackError> {
    let frames_dir = require_pack_directory(pack_path, "assets/frames")?;
    let mut paths = fs::read_dir(&frames_dir)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if file_type.is_symlink() || !file_type.is_file() {
                return None;
            }
            let path = entry.path();
            let is_png = path
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| value.eq_ignore_ascii_case("png"))
                .unwrap_or(false);
            is_png.then_some(path)
        })
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}
```

- [ ] **Step 6: Verify Task 2**

Run:

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml symlinked_required_file_fails_validation symlinked_frames_directory_fails_validation
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml --package forge_pack
```

Expected:

```text
symlink validation tests pass
forge_pack tests pass
```

### Task 3: Harden Local Pack Library Scanning And UI Wiring

**Files:**
- Modify: `apps/mac/src-tauri/src/lib.rs`
- Modify: `scripts/test-local-pack-library-source.mjs`
- Modify: `apps/mac/src/App.tsx`
- Modify: `apps/mac/src/styles/app.css`
- Create: `docs/qa/local-pack-library-evidence-2026-06-05.md`

- [ ] **Step 1: Add behavioral tests for local pack listing**

In the `#[cfg(test)]` module of `apps/mac/src-tauri/src/lib.rs`, add:

```rust
fn write_library_pack_fixture(pack: &Path, name: &str, frame_count: usize) {
    fs::create_dir_all(pack.join("previews")).unwrap();
    fs::create_dir_all(pack.join("assets/frames")).unwrap();
    fs::write(pack.join("previews/preview.gif"), b"gif").unwrap();
    fs::write(pack.join("assets/sprite_sheet.png"), b"png").unwrap();
    for index in 1..=frame_count {
        fs::write(pack.join("assets/frames").join(format!("frame_{index:03}.png")), b"png").unwrap();
    }
    fs::write(
        pack.join("forgepack.json"),
        serde_json::json!({
            "id": name.to_lowercase().replace(' ', "-"),
            "name": name,
            "version": "0.1.0",
            "previews": { "gif": "previews/preview.gif" },
            "assets": {
                "frames": "assets/frames",
                "spriteSheet": "assets/sprite_sheet.png",
                "atlas": "assets/atlas.json",
                "manifest": "assets/manifest.json",
                "qualityReport": "quality-report.json"
            }
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        pack.join("assets/atlas.json"),
        serde_json::json!({
            "image": "sprite_sheet.png",
            "frameWidth": 16,
            "frameHeight": 16,
            "columns": 1,
            "rows": frame_count,
            "frames": (0..frame_count)
                .map(|index| serde_json::json!({
                    "index": index,
                    "name": format!("frame_{:03}.png", index + 1),
                    "x": 0,
                    "y": index * 16,
                    "width": 16,
                    "height": 16
                }))
                .collect::<Vec<_>>()
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        pack.join("assets/manifest.json"),
        serde_json::json!({
            "name": name,
            "sheet": {
                "image": "assets/sprite_sheet.png",
                "frameWidth": 16,
                "frameHeight": 16,
                "columns": 1,
                "rows": frame_count
            },
            "animations": [{
                "name": "idle",
                "frames": (0..frame_count).collect::<Vec<_>>(),
                "fps": 12.0,
                "loop": true
            }],
            "anchor": { "type": "feet", "x": 8.0, "y": 16.0 }
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        pack.join("quality-report.json"),
        serde_json::json!({
            "verdict": "game_ready",
            "metrics": {
                "bboxStability": "stable",
                "loopMatch": "consistent",
                "alphaCoverage": 0.5,
                "frameCount": frame_count,
                "frameSizeConsistency": "consistent",
                "spriteSheetCellSafety": "safe"
            },
            "warnings": []
        })
        .to_string(),
    )
    .unwrap();
}

#[test]
fn list_local_packs_lists_direct_and_nested_valid_packs() {
    let temp = tempfile::tempdir().unwrap();
    let exports = temp.path().join("exports");
    let direct = exports.join("direct.gsfpack");
    let nested_parent = exports.join("Nested-Export");
    let nested = nested_parent.join("nested.gsfpack");
    let broken = exports.join("broken.gsfpack");
    write_library_pack_fixture(&direct, "Direct Pack", 2);
    write_library_pack_fixture(&nested, "Nested Pack", 3);
    fs::create_dir_all(&broken).unwrap();

    let packs = list_local_packs(ListLocalPacksParams {
        exports_dir: exports.display().to_string(),
    })
    .unwrap();

    let names = packs.iter().map(|pack| pack.name.as_str()).collect::<Vec<_>>();
    assert_eq!(names, vec!["Direct Pack", "Nested Pack"]);
}

#[cfg(unix)]
#[test]
fn list_local_packs_skips_symlinked_pack_and_nested_directory() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().unwrap();
    let exports = temp.path().join("exports");
    fs::create_dir_all(&exports).unwrap();
    let outside = temp.path().join("outside");
    let outside_pack = outside.join("outside.gsfpack");
    write_library_pack_fixture(&outside_pack, "Outside Pack", 1);
    symlink(&outside_pack, exports.join("linked.gsfpack")).unwrap();
    symlink(&outside, exports.join("linked-dir")).unwrap();

    let packs = list_local_packs(ListLocalPacksParams {
        exports_dir: exports.display().to_string(),
    })
    .unwrap();

    assert!(packs.is_empty());
}
```

- [ ] **Step 2: Run tests to verify current scanner gap**

Run:

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml list_local_packs
```

Expected before implementation:

```text
The symlink test fails because the scanner follows symlinked directories or packs.
```

- [ ] **Step 3: Replace local pack scanner with canonical-root scanning**

In `apps/mac/src-tauri/src/lib.rs`, replace `list_local_packs` with:

```rust
#[tauri::command]
fn list_local_packs(
    params: ListLocalPacksParams,
) -> Result<Vec<forge_pack::PackInspectSummary>, String> {
    let exports_dir = normalize_user_export_directory(Path::new(&params.exports_dir))?;
    let mut packs = Vec::new();

    if !exports_dir.exists() {
        return Ok(packs);
    }

    let root = fs::canonicalize(&exports_dir).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(&root).map_err(|error| error.to_string())?.filter_map(Result::ok) {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if is_gsfpack_path(&path) {
            push_local_pack_candidate(&root, &path, &mut packs);
            continue;
        }
        if file_type.is_dir() {
            let Ok(nested_entries) = fs::read_dir(&path) else {
                continue;
            };
            for nested in nested_entries.filter_map(Result::ok) {
                let Ok(nested_file_type) = nested.file_type() else {
                    continue;
                };
                if nested_file_type.is_symlink() {
                    continue;
                }
                let nested_path = nested.path();
                if nested_file_type.is_dir() && is_gsfpack_path(&nested_path) {
                    push_local_pack_candidate(&root, &nested_path, &mut packs);
                }
            }
        }
    }

    packs.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.root.cmp(&right.root))
    });
    Ok(packs)
}

fn push_local_pack_candidate(
    root: &Path,
    candidate: &Path,
    packs: &mut Vec<forge_pack::PackInspectSummary>,
) {
    let Ok(metadata) = fs::symlink_metadata(candidate) else {
        return;
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return;
    }
    let Ok(canonical) = fs::canonicalize(candidate) else {
        return;
    };
    if !canonical.starts_with(root) {
        return;
    }
    if let Ok(summary) = forge_pack::inspect_pack(&canonical) {
        packs.push(summary);
    }
}
```

- [ ] **Step 4: Upgrade the source guard**

In `scripts/test-local-pack-library-source.mjs`, add these assertions:

```js
assertContains(tauriSource, "symlink_metadata", "Local pack library must inspect symlinks without following them.");
assertContains(tauriSource, "canonicalize(&exports_dir)", "Local pack library must canonicalize the export root.");
assertContains(tauriSource, "canonical.starts_with(root)", "Local pack library must keep candidates inside the export root.");
assertContains(tauriSource, "list_local_packs_skips_symlinked_pack_and_nested_directory", "Local pack library must have a symlink regression test.");
assertContains(appSource, "queuedGsfpackImportPath", "Local Pack Library re-import must queue a .gsfpack path for Forge.");
assertContains(appSource, "setQueuedGsfpackImportPath(path)", "Re-import Pack must set the queued .gsfpack path.");
```

- [ ] **Step 5: Add action feedback in `App.tsx`**

Add state near the existing Local Pack Library state:

```tsx
const [libraryStatus, setLibraryStatus] = useState("Refresh the library to scan the default export folder.");
```

Pass it into `ExportsRoute`:

```tsx
libraryStatus={libraryStatus}
```

Extend `ExportsRoute` props:

```tsx
libraryStatus: string;
```

Render the status under the panel title:

```tsx
<p className="library-status">{libraryStatus}</p>
```

Update the action handlers:

```tsx
async function handleRefreshLibrary() {
  setLibraryStatus("Scanning local export folder...");
  try {
    const packs = await listLocalPacks(settings.defaultOutputFolder);
    setLibraryPacks(packs);
    setLibraryStatus(`Found ${packs.length} local pack${packs.length === 1 ? "" : "s"}.`);
  } catch (error) {
    setLibraryStatus(error instanceof Error ? error.message : String(error));
  }
}

async function handleInspectPack(path: string) {
  setLibraryStatus("Inspecting local pack...");
  try {
    const pack = await inspectLocalPack(path);
    setLibraryPacks((current) => [pack, ...current.filter((item) => item.root !== pack.root)]);
    setLibraryStatus(`Inspected ${pack.name}.`);
  } catch (error) {
    setLibraryStatus(error instanceof Error ? error.message : String(error));
  }
}

async function handleValidatePack(path: string) {
  setLibraryStatus("Validating local pack...");
  try {
    await validateGsfpack(path);
    await handleInspectPack(path);
    setLibraryStatus("Pack validation passed.");
  } catch (error) {
    setLibraryStatus(error instanceof Error ? error.message : String(error));
  }
}
```

- [ ] **Step 6: Keep action controls stable in CSS**

Add to `apps/mac/src/styles/app.css`:

```css
.library-status {
  color: var(--muted);
  font-size: 12px;
  line-height: 1.35;
  margin: 8px 0 0;
  max-width: 720px;
}

.export-history-actions {
  align-items: center;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: flex-end;
}

.export-history-actions .secondary-button {
  min-height: 34px;
  white-space: nowrap;
}
```

- [ ] **Step 7: Verify Task 3**

Run:

```bash
node scripts/test-local-pack-library-source.mjs
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml list_local_packs
npm --workspace apps/mac run build
npm --workspace apps/mac run smoke:ui:mvp
```

Expected:

```text
PASS local pack library source test
Rust list_local_packs tests pass
frontend build passes
MVP UI smoke passes and still finds Local Pack Library, Validate Pack, Re-import Pack
```

### Task 4: Computer Use Installed-App Verification

**Files:**
- Modify: `docs/qa/forge-post-rc-ui-smoke-2026-06-05.md`
- Create: `docs/qa/local-pack-library-evidence-2026-06-05.md`
- Modify: `docs/architecture/release-candidate-verification.md`

- [ ] **Step 1: Launch with deterministic missing ffmpeg environment**

Use Computer Use MCP against the installed app launched from Terminal:

```bash
env GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS=/tmp/forge-empty-tools \
  GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS=1 \
  PATH=/usr/bin:/bin \
  "/Applications/Game Sprite Forge.app/Contents/MacOS/Game Sprite Forge"
```

Expected UI observation:

```text
Check FFmpeg reports: Install ffmpeg or choose an ffmpeg binary in Settings.
The app does not claim ffmpeg is available.
Settings still allows selecting configured ffmpeg/ffprobe binaries.
```

- [ ] **Step 2: Launch with PATH fallback restored**

Run:

```bash
env PATH="/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin" \
  "/Applications/Game Sprite Forge.app/Contents/MacOS/Game Sprite Forge"
```

Expected UI observation:

```text
Check FFmpeg reports ffmpeg and ffprobe are available when Homebrew tools are reachable through the launch environment.
```

- [ ] **Step 3: Verify Local Pack Library inspect/validate/re-import**

Use a known exported pack:

```text
/Users/kartz/Game Sprite Forge/Exports/Multiline-PNG-UI-Test-1780626245133/Multiline-PNG-UI-Test.gsfpack
```

Expected UI observation:

```text
Exports route shows Local Pack Library.
Refresh Library lists the pack.
Inspect updates the pack row without navigation.
Validate Pack reports validation passed.
Re-import Pack opens Forge, imports the .gsfpack, shows frames, and preserves the original frame count.
```

- [ ] **Step 4: Record evidence**

Create `docs/qa/local-pack-library-evidence-2026-06-05.md`:

````markdown
# Local Pack Library Evidence - 2026-06-05

## Installed-App Build

```text
App: /Applications/Game Sprite Forge.app
Candidate: target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg
```

## Computer Use Result

```text
Refresh Library:
Inspect:
Validate Pack:
Re-import Pack:
Frame count:
Result:
```

## Commands

```bash
npm run test:scripts
npm --workspace apps/mac run build
npm --workspace apps/mac run smoke:ui:mvp
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml list_local_packs
```
````

Fill the result fields with the exact Computer Use observations from the installed app.

### Task 5: Exporter Evidence And Godot Helper Validation

**Files:**
- Modify: `packages/core/src/export/mod.rs`
- Modify: `docs/qa/godot-helper-evidence-2026-06-05.md`
- Create: `docs/qa/godot-editor-helper-evidence-2026-06-06.md`
- Modify: `docs/architecture/post-release-backlog.md`

- [ ] **Step 1: Keep schema-level Godot helper coverage green**

Run:

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml godot_helper_lists_all_multipage_textures validates_multipage_atlas_assets
```

Expected:

```text
Both tests pass and prove multi-page texture metadata remains present.
```

- [ ] **Step 2: Add editor/sample-project evidence**

Create `docs/qa/godot-editor-helper-evidence-2026-06-06.md`:

````markdown
# Godot Editor Helper Evidence - 2026-06-05

## Pack Under Test

```text
Pack:
Godot helper:
Texture count:
Frame count:
```

## Godot Version

```text
Version:
Command:
```

## Result

```text
Helper JSON loads:
All textures are present:
Animation metadata matches manifest:
Manual import result:
```

## Follow-Up Decision

```text
Godot editor plugin is deferred. The MVP keeps Godot support as helper metadata and documentation only.
```
````

- [ ] **Step 3: Update backlog evidence row**

In `docs/architecture/post-release-backlog.md`, under `Slice 2: Exporter Refinement`, add:

````markdown
Evidence required before adding more engines:

```text
generic export validates against schemas
Godot helper metadata references every sprite-sheet page
Godot editor/sample-project evidence is recorded in docs/qa/godot-editor-helper-evidence-2026-06-06.md
```
````

- [ ] **Step 4: Verify Task 5**

Run:

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml godot_helper_lists_all_multipage_textures validates_multipage_atlas_assets
npm run test:scripts
```

Expected:

```text
Godot helper tests pass
source/script guards pass
```

### Task 6: Release Package Regression After Any App Change

**Files:**
- Modify: `docs/architecture/release-candidate-verification.md`
- Modify: `docs/architecture/mvp-scope.md`

- [ ] **Step 1: Build the app**

Run:

```bash
npm --workspace apps/mac run build
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml
scripts/build-signed-release-dmg.sh
```

Expected:

```text
frontend build passes
Rust tests pass
signed DMG is produced under target/release/bundle/dmg/
```

- [ ] **Step 2: Run notarization preflight**

Run:

```bash
bash scripts/notarization-preflight.sh \
  --keychain-profile GameSpriteForgeNotary \
  "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
```

Expected before a rebuilt DMG is submitted:

```text
Preflight checks signing, timestamp, hardened runtime, current stapled-ticket state, and prints exact submit/staple commands without printing secrets.
```

- [ ] **Step 3: Submit, staple, verify, and package only if the DMG changed**

Run:

```bash
xcrun notarytool submit "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg" \
  --keychain-profile "GameSpriteForgeNotary" \
  --wait
xcrun stapler staple "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
xcrun stapler validate -v "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
scripts/verify-release-package.sh "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
scripts/package-release-candidate.sh
```

Expected:

```text
notarytool status: Accepted
stapler validate: pass
Gatekeeper DMG assessment: accepted, source=Notarized Developer ID
Gatekeeper installed app assessment: accepted, source=Notarized Developer ID
release package directory includes the new SHA prefix
```

- [ ] **Step 4: Update release evidence docs**

In `docs/architecture/release-candidate-verification.md` and `docs/architecture/mvp-scope.md`, update:

```text
DMG SHA-256
notarization submission id
release-candidates directory name
verification command results
Computer Use installed-app evidence summary
```

### Task 7: CLI/MCP Feasibility Decision Without Product Expansion

**Files:**
- Create: `docs/architecture/cli-mcp-feasibility.md`
- Modify: `docs/architecture/post-release-backlog.md`

- [ ] **Step 1: Create the feasibility document**

Create `docs/architecture/cli-mcp-feasibility.md`:

````markdown
# CLI And MCP Feasibility

Date: 2026-06-05

## Current Decision

CLI and MCP remain feasibility work only. They are not part of the public MVP product surface.

## Core Operations That Are Ready For Headless Mapping

```text
validate .gsfpack
inspect .gsfpack
import .gsfpack into a local job
export processed job assets
verify release package
```

## Operations That Need More UI/Core Separation First

```text
interactive video source selection
interactive PNG multi-select
background tuning preview
manual anchor editing
quality decisions requiring visual inspection
```

## Proposed CLI Command Map

```text
forge pack inspect <path.gsfpack>
forge pack validate <path.gsfpack>
forge export verify <path.gsfpack>
forge release verify <dmg-path> --sha256 <hash>
```

## Proposed MCP Command Map

```text
inspect_pack(path) -> pack metadata and paths
validate_pack(path) -> pass/fail plus schema/layout errors
list_local_packs(exports_dir) -> local pack summaries
verify_release_package(dmg_path, sha256) -> notarization/Gatekeeper result
```

## Safety Rules

```text
No cloud upload.
No AI provider calls.
No marketplace publishing.
No write action without explicit destination path.
No destructive delete commands.
No automatic export into a game project without user-selected output.
```

## Entry Criteria For Product MCP

```text
Local Pack Library hardening complete.
Exporter evidence complete.
At least one release candidate passes installed-app and release-package verification after the hardening work.
The user explicitly asks to build product MCP or CLI.
```
````

- [ ] **Step 2: Add backlog gate**

In `docs/architecture/post-release-backlog.md`, under `Slice 4: CLI/MCP Feasibility Spike`, add:

````markdown
Implementation rule:

```text
This slice produces a feasibility document and command map first. It does not create a product MCP server until the user explicitly requests implementation after local library and exporter evidence are complete.
```
````

- [ ] **Step 3: Verify no product MCP surface was introduced**

Run:

```bash
rg -n "MCP|mcp|server" apps packages scripts docs/architecture docs/superpowers/plans
```

Expected:

```text
Matches only appear in planning, feasibility, and scope-guard documentation unless a later user request explicitly starts implementation.
```

## Multi-Agent Execution Map

Use this map only after the user chooses execution.

```text
Agent A / Docs worker:
  Owns PRODUCT.md, GAME-SPRITE-FORGE-DEVELOPMENT-PLAN-2026-05-31.md, docs/architecture/*

Agent B / Pack safety worker:
  Owns packages/pack/src/lib.rs and packages/pack/tests/pack_tests.rs

Agent C / Local library worker:
  Owns apps/mac/src-tauri/src/lib.rs, scripts/test-local-pack-library-source.mjs, apps/mac/src/App.tsx, apps/mac/src/styles/app.css

Agent D / Exporter evidence worker:
  Owns packages/core/src/export/mod.rs and docs/qa/godot*evidence*

Main thread:
  Owns Computer Use verification, release notarization checks, integration review, and final docs reconciliation
```

Each worker must state changed file paths and verification commands in its final message. After each worker returns, run a spec review and quality review before moving to release packaging.

## Final Completion Gate

The plan is complete only when all of the following are true:

```text
Documentation current-state drift is resolved.
Local Pack Library scanner is symlink-safe and covered by Rust tests.
Local Pack Library UI actions provide visible feedback and pass UI smoke.
Installed-app Computer Use evidence covers missing ffmpeg, PATH fallback, inspect, validate, and re-import.
Godot helper has schema-level evidence and a recorded editor/sample-project evidence row.
Current or rebuilt DMG passes notarization preflight and release verifier.
CLI/MCP feasibility is documented while product MCP remains deferred.
```
