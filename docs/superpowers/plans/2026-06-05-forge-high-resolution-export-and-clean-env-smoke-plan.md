# Forge High Resolution Export And Clean Environment Smoke Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove the notarized release candidate in a clean or scrubbed macOS environment, then resolve QA-004 by exporting high-resolution animations as valid multi-page sprite sheets instead of blocking on a single oversized sheet.

**Architecture:** Keep the app import-only and local-first. Preserve the existing single-page export contract (`sprite_sheet.png`, `atlas.json`, `manifest.json`, `.gsfpack`) for ordinary packs, and add multi-page atlas metadata only when a sheet would exceed the selected max texture size. Put layout planning in Rust core, keep pack validation schema-aware, and expose only small export controls in the React UI.

**Tech Stack:** Tauri 2, React 18, TypeScript, Vite, Rust workspace, `image`, `serde`, JSON Schema, macOS Developer ID notarized DMG, ffmpeg/ffprobe, Computer Use for real UI smoke verification.

---

## Current Evidence

Recorded on 2026-06-05:

```text
notarized release candidate: release-candidates/GameSpriteForge-0.1.0-aarch64-f2060117105d-notarized
DMG SHA-256: f2060117105df8477ab8e3eb91f1ba7f1a3a38db5a05f99bbe61bdf630a8116d
zip SHA-256: 4e58969cea1fdf5b2998d02b6cfa22b2c46461ad5bcaece6a9e9e5806e89b100
release verifier: pass from root DMG and package DMG
manual installed-app QA: pass for sample video, real short video, PNG sequence, sprite sheet, and .gsfpack re-import
QA-004: /Users/kartz/Development/social-auto-upload/videos/demo.mp4 processed 30 frames, then export failed with sprite sheet 5226x10450 exceeds max texture size 2048
```

## Scope Guard

Do not add these in this slice:

```text
AI image generation
AI video generation
BYOK settings
website
online pack registry
marketplace
MCP implementation
Codex Skill integration
creator publish
cloud upload
cloud processing
bundled ffmpeg
```

MCP/tooling guidance for execution:

```text
Use Computer Use only for the clean-environment and installed-app UI smoke rows.
Use local shell commands for Rust, TypeScript, schema, packaging, and release verifier checks.
Use no Figma/Product Design MCP for this slice; there is no new screen design system to import.
```

## Current Code Truths

```text
packages/core/src/export/sheet.rs builds one RgbaImage and returns TextureTooLarge when sheet_width or sheet_height exceeds max_texture_size.
packages/core/src/export/manifest.rs hardcodes manifest.sheet.image to assets/sprite_sheet.png.
packages/core/src/export/mod.rs copies exactly one sprite_sheet.png into the .gsfpack assets folder.
packages/pack/src/lib.rs requires assets/sprite_sheet.png and validates atlas/manifest schemas with additionalProperties=false.
schemas/atlas.schema.json has no page/image field per frame and no images array.
schemas/manifest.schema.json has no sheet.images field.
apps/mac/src/tauriCommands.ts sends sheet.maxTextureSize and sheet.columns but no multi-sheet option.
apps/mac/src/components/ExportPanel.tsx exposes Sheet Columns, Sheet Padding, and Sheet Margin only.
apps/mac/src/routes/ForgeRoute.tsx currently routes oversized export errors through generic sprite sheet recovery copy.
```

## File Structure

- Create: `docs/qa/forge-clean-env-smoke-2026-06-05.md`
- Create: `packages/core/src/export/sheet_layout.rs`
- Modify: `packages/core/src/export/sheet.rs`
- Modify: `packages/core/src/export/mod.rs`
- Modify: `packages/core/src/export/manifest.rs`
- Modify: `packages/core/src/lib.rs` only if the new module needs re-exporting through the crate root
- Modify: `schemas/atlas.schema.json`
- Modify: `schemas/manifest.schema.json`
- Modify: `packages/pack/src/lib.rs`
- Modify: `packages/pack/tests/pack_tests.rs`
- Modify: `apps/mac/src/tauriCommands.ts`
- Modify: `apps/mac/src/components/ExportPanel.tsx`
- Modify: `apps/mac/src/routes/ForgeRoute.tsx`
- Modify: `apps/mac/src/styles/app.css`
- Modify: `apps/mac/scripts/smoke-ui.mjs`
- Modify: `scripts/test-recovery-plan-source.mjs`
- Modify: `docs/architecture/mvp-scope.md`
- Modify: `docs/qa/forge-manual-mvp-checklist-2026-06-05.md`
- Modify: `docs/architecture/post-release-backlog.md`

---

### Task 1: Record Clean Environment Release Smoke

**Files:**
- Create: `docs/qa/forge-clean-env-smoke-2026-06-05.md`
- Reference: `release-candidates/GameSpriteForge-0.1.0-aarch64-f2060117105d-notarized/README.md`
- Reference: `docs/architecture/release-candidate-verification.md`

- [ ] **Step 1: Create the clean environment smoke document**

Create `docs/qa/forge-clean-env-smoke-2026-06-05.md` with this content:

````markdown
# Forge Clean Environment Smoke - 2026-06-05

Artifact under test:

```text
release-candidates/GameSpriteForge-0.1.0-aarch64-f2060117105d-notarized.zip
DMG SHA-256: f2060117105df8477ab8e3eb91f1ba7f1a3a38db5a05f99bbe61bdf630a8116d
Zip SHA-256: 4e58969cea1fdf5b2998d02b6cfa22b2c46461ad5bcaece6a9e9e5806e89b100
```

Environment:

```text
Machine:
macOS version:
Apple Silicon or Intel:
Fresh user account or scrubbed PATH:
ffmpeg source: missing / PATH / configured setting
ffprobe source: missing / PATH / configured setting
```

Release package verification:

```bash
cd /Users/kartz/Development/Forge/release-candidates/GameSpriteForge-0.1.0-aarch64-f2060117105d-notarized
shasum -a 256 -c SHA256SUMS
scripts/verify-release-package.sh "./Game Sprite Forge_0.1.0_aarch64.dmg" \
  f2060117105df8477ab8e3eb91f1ba7f1a3a38db5a05f99bbe61bdf630a8116d
```

Expected:

```text
SHA256SUMS: all OK
verify-release-package.sh: PASS release package verification script completed.
Gatekeeper DMG assessment: accepted, source=Notarized Developer ID
Gatekeeper app assessment: accepted, source=Notarized Developer ID
mounted-DMG launch: pass
```

Manual app rows:

| ID | Check | Expected | Result | Evidence |
| --- | --- | --- | --- | --- |
| CE-001 | Open DMG without bypassing Gatekeeper | App launches from mounted DMG | Not run | |
| CE-002 | Install to `/Applications` | Installed app opens normally | Not run | |
| CE-003 | Missing ffmpeg/ffprobe | `Install ffmpeg or choose an ffmpeg binary in Settings.` | Not run | |
| CE-004 | PATH ffmpeg/ffprobe fallback | Settings check reports `ffmpeg and ffprobe are available.` | Not run | |
| CE-005 | Invalid configured ffmpeg | `ffmpeg: configured tool path must point to ffmpeg` | Not run | |
| CE-006 | Invalid configured ffprobe | `ffprobe: configured tool path must point to ffprobe` | Not run | |
| CE-007 | Bundled sample video | Import, process, export, validate re-import succeeds | Not run | |

Decision:

```text
Clean-environment confidence: Not run
Blocking issues: none recorded yet
Follow-ups:
```
````

- [ ] **Step 2: Run package verification on the target environment**

Run:

```bash
cd /Users/kartz/Development/Forge/release-candidates/GameSpriteForge-0.1.0-aarch64-f2060117105d-notarized
shasum -a 256 -c SHA256SUMS
scripts/verify-release-package.sh "./Game Sprite Forge_0.1.0_aarch64.dmg" \
  f2060117105df8477ab8e3eb91f1ba7f1a3a38db5a05f99bbe61bdf630a8116d
```

Expected:

```text
Game Sprite Forge_0.1.0_aarch64.dmg: OK
scripts/verify-release-package.sh: OK
docs/release-candidate-verification.md: OK
README.md: OK
PASS release package verification script completed.
```

- [ ] **Step 3: Use Computer Use for manual clean-environment UI rows**

Use Computer Use to open the installed app and record CE-001 through CE-007 in `docs/qa/forge-clean-env-smoke-2026-06-05.md`.

Expected final document rows:

```text
CE-001 through CE-007 are Pass, Fail, or Blocked with exact visible evidence.
Any Fail or Blocked row has a concrete error string or missing path.
```

- [ ] **Step 4: Commit**

Run:

```bash
git add docs/qa/forge-clean-env-smoke-2026-06-05.md
git commit -m "docs: add clean environment release smoke checklist"
```

If `/Users/kartz/Development/Forge` is still not a git repository, record this command in the handoff notes instead of committing.

---

### Task 2: Add Sprite Sheet Layout Planner Tests

**Files:**
- Create: `packages/core/src/export/sheet_layout.rs`
- Modify: `packages/core/src/export/mod.rs`

- [ ] **Step 1: Create layout planner module with tests first**

Create `packages/core/src/export/sheet_layout.rs` with the public types and tests below. For the first failing run, make the function return `Err(SpriteSheetLayoutError::NoFrames)`; Step 4 replaces that deliberately failing body with the planner.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpriteSheetLayoutParams {
    pub frame_count: usize,
    pub frame_width: u32,
    pub frame_height: u32,
    pub requested_columns: u32,
    pub padding_px: u32,
    pub margin_px: u32,
    pub max_texture_size: u32,
    pub allow_multi_sheet: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteSheetLayout {
    pub frame_width: u32,
    pub frame_height: u32,
    pub page_columns: u32,
    pub page_rows: u32,
    pub frames_per_page: usize,
    pub pages: Vec<SpriteSheetPageLayout>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteSheetPageLayout {
    pub page_index: usize,
    pub image_name: String,
    pub columns: u32,
    pub rows: u32,
    pub width: u32,
    pub height: u32,
    pub frames: Vec<SpriteSheetFramePlacement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteSheetFramePlacement {
    pub frame_index: usize,
    pub page_index: usize,
    pub name: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpriteSheetLayoutError {
    NoFrames,
    InvalidColumns,
    FrameExceedsMaxTexture { width: u32, height: u32, max_texture_size: u32 },
    SheetExceedsMaxTexture { width: u32, height: u32, max_texture_size: u32 },
}

pub fn plan_sprite_sheet_layout(
    params: SpriteSheetLayoutParams,
) -> Result<SpriteSheetLayout, SpriteSheetLayoutError> {
    let _ = params;
    Err(SpriteSheetLayoutError::NoFrames)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_small_exports_on_one_legacy_page() {
        let layout = plan_sprite_sheet_layout(SpriteSheetLayoutParams {
            frame_count: 6,
            frame_width: 64,
            frame_height: 64,
            requested_columns: 4,
            padding_px: 2,
            margin_px: 2,
            max_texture_size: 2048,
            allow_multi_sheet: true,
        })
        .unwrap();

        assert_eq!(layout.pages.len(), 1);
        assert_eq!(layout.page_columns, 4);
        assert_eq!(layout.page_rows, 2);
        assert_eq!(layout.pages[0].image_name, "sprite_sheet.png");
        assert_eq!(layout.pages[0].frames.len(), 6);
        assert_eq!(layout.pages[0].width, 262);
        assert_eq!(layout.pages[0].height, 130);
    }

    #[test]
    fn splits_high_resolution_exports_across_pages() {
        let layout = plan_sprite_sheet_layout(SpriteSheetLayoutParams {
            frame_count: 30,
            frame_width: 1304,
            frame_height: 696,
            requested_columns: 4,
            padding_px: 2,
            margin_px: 2,
            max_texture_size: 2048,
            allow_multi_sheet: true,
        })
        .unwrap();

        assert_eq!(layout.page_columns, 1);
        assert_eq!(layout.page_rows, 2);
        assert_eq!(layout.frames_per_page, 2);
        assert_eq!(layout.pages.len(), 15);
        assert_eq!(layout.pages[0].image_name, "sprite_sheet.png");
        assert_eq!(layout.pages[1].image_name, "sprite_sheet_002.png");
        assert!(layout.pages.iter().all(|page| page.width <= 2048));
        assert!(layout.pages.iter().all(|page| page.height <= 2048));
        assert_eq!(layout.pages[14].frames[1].frame_index, 29);
    }

    #[test]
    fn preserves_current_error_when_multi_sheet_is_disabled() {
        let error = plan_sprite_sheet_layout(SpriteSheetLayoutParams {
            frame_count: 30,
            frame_width: 1304,
            frame_height: 696,
            requested_columns: 4,
            padding_px: 2,
            margin_px: 2,
            max_texture_size: 2048,
            allow_multi_sheet: false,
        })
        .unwrap_err();

        assert_eq!(
            error,
            SpriteSheetLayoutError::SheetExceedsMaxTexture {
                width: 5226,
                height: 10450,
                max_texture_size: 2048,
            }
        );
    }

    #[test]
    fn rejects_a_single_frame_larger_than_the_limit() {
        let error = plan_sprite_sheet_layout(SpriteSheetLayoutParams {
            frame_count: 1,
            frame_width: 4096,
            frame_height: 256,
            requested_columns: 1,
            padding_px: 0,
            margin_px: 0,
            max_texture_size: 2048,
            allow_multi_sheet: true,
        })
        .unwrap_err();

        assert_eq!(
            error,
            SpriteSheetLayoutError::FrameExceedsMaxTexture {
                width: 4096,
                height: 256,
                max_texture_size: 2048,
            }
        );
    }
}
```

- [ ] **Step 2: Expose the module**

Modify the top of `packages/core/src/export/mod.rs`:

```rust
pub mod gif;
pub mod manifest;
pub mod sheet;
pub mod sheet_layout;
```

Add this export near the existing `pub use` lines:

```rust
pub use sheet_layout::{
    plan_sprite_sheet_layout, SpriteSheetFramePlacement, SpriteSheetLayout,
    SpriteSheetLayoutError, SpriteSheetLayoutParams, SpriteSheetPageLayout,
};
```

- [ ] **Step 3: Run the failing tests**

Run:

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml sheet_layout
```

Expected:

```text
keeps_small_exports_on_one_legacy_page ... FAILED
splits_high_resolution_exports_across_pages ... FAILED
preserves_current_error_when_multi_sheet_is_disabled ... FAILED
rejects_a_single_frame_larger_than_the_limit ... FAILED
```

- [ ] **Step 4: Implement the minimal planner**

Replace `plan_sprite_sheet_layout` with logic that:

```text
validates frame_count > 0
validates requested_columns > 0
computes the legacy single sheet using requested_columns
returns SheetExceedsMaxTexture when allow_multi_sheet is false and the legacy sheet is too large
when allow_multi_sheet is true, computes max columns and max rows that fit inside max_texture_size
uses requested_columns.min(max_columns) for page columns
uses max_rows for page rows
assigns frame placements page by page
names page 0 sprite_sheet.png and later pages sprite_sheet_002.png, sprite_sheet_003.png, ...
```

Use this helper formula for fitting cells:

```rust
fn max_cells_that_fit(cell_size: u32, padding_px: u32, margin_px: u32, max_texture_size: u32) -> u32 {
    if cell_size.saturating_add(margin_px.saturating_mul(2)) > max_texture_size {
        return 0;
    }
    let available = max_texture_size - margin_px.saturating_mul(2);
    ((available + padding_px) / (cell_size + padding_px)).max(1)
}
```

- [ ] **Step 5: Run the planner tests**

Run:

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml sheet_layout
```

Expected:

```text
test result: ok
```

- [ ] **Step 6: Commit**

Run:

```bash
git add packages/core/src/export/mod.rs packages/core/src/export/sheet_layout.rs
git commit -m "test: add sprite sheet layout planner coverage"
```

If the workspace is still not a git repository, record this command in the handoff notes instead of committing.

---

### Task 3: Write Multi-Page Sprite Sheets In Rust Core

**Files:**
- Modify: `packages/core/src/export/sheet.rs`
- Modify: `packages/core/src/export/mod.rs`
- Modify: `packages/core/src/export/manifest.rs`

- [ ] **Step 1: Extend sprite sheet parameters and outputs**

Modify `SpriteSheetParameters` in `packages/core/src/export/sheet.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpriteSheetParameters {
    pub columns: u32,
    pub padding_px: u32,
    pub margin_px: u32,
    pub max_texture_size: u32,
    #[serde(default)]
    pub allow_multi_sheet: bool,
}
```

Update `Default`:

```rust
allow_multi_sheet: false,
```

Extend `Atlas`:

```rust
pub struct Atlas {
    pub image: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<String>,
    pub frame_width: u32,
    pub frame_height: u32,
    pub columns: u32,
    pub rows: u32,
    pub frames: Vec<AtlasFrame>,
}
```

Extend `AtlasFrame`:

```rust
pub struct AtlasFrame {
    pub index: usize,
    pub name: String,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub page: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

fn is_zero_usize(value: &usize) -> bool {
    *value == 0
}
```

Extend `SpriteSheetOutput`:

```rust
pub struct SpriteSheetOutput {
    pub sprite_sheet_path: PathBuf,
    pub sprite_sheet_paths: Vec<PathBuf>,
    pub atlas_path: PathBuf,
    pub atlas: Atlas,
}
```

- [ ] **Step 2: Add a failing core export test**

Add this test in the `#[cfg(test)]` module of `packages/core/src/export/mod.rs`:

```rust
#[test]
fn export_pack_splits_high_resolution_sprite_sheets() {
    let temp = tempfile::tempdir().unwrap();
    let mut sources = Vec::new();
    for index in 0..30 {
        let source = temp.path().join(format!("source_{index:03}.png"));
        let image = RgbaImage::from_pixel(1304, 696, Rgba([index as u8, 0, 255, 255]));
        image.save(&source).unwrap();
        sources.push(source);
    }

    let output = export_pack(ExportPackParams {
        exports_dir: temp.path().join("exports"),
        export_id: "high_res".to_string(),
        frame_paths: sources,
        sheet: SpriteSheetParameters {
            columns: 4,
            padding_px: 2,
            margin_px: 2,
            max_texture_size: 2048,
            allow_multi_sheet: true,
        },
        gif: PreviewGifParameters {
            fps: 12.0,
            loop_animation: true,
            background: GifBackground::Checkerboard,
        },
        metadata: PackMetadataParams {
            id: "high-res".to_string(),
            name: "High Res".to_string(),
            version: "0.1.0".to_string(),
            creator_name: "Game Sprite Forge".to_string(),
            license_type: "private".to_string(),
            source_kind: "import_video".to_string(),
            source_name: Some("demo.mp4".to_string()),
            animation_name: "demo".to_string(),
            animation_frames: Some((0..30).collect()),
            fps: 12.0,
            loop_animation: true,
            anchor: default_foot_anchor(1304, 696, 0),
            quality_report: quality_report(30),
        },
    })
    .unwrap();

    assert_eq!(output.frame_paths.len(), 30);
    assert_eq!(output.sprite_sheet_paths.len(), 15);
    assert!(output.export_dir.join("sprite_sheet.png").is_file());
    assert!(output.export_dir.join("sprite_sheet_002.png").is_file());
    assert!(output.pack_dir.join("assets/sprite_sheet.png").is_file());
    assert!(output.pack_dir.join("assets/sprite_sheet_002.png").is_file());

    let atlas: serde_json::Value =
        serde_json::from_slice(&fs::read(&output.atlas_path).unwrap()).unwrap();
    assert_eq!(atlas["images"].as_array().unwrap().len(), 15);
    assert_eq!(atlas["frames"].as_array().unwrap().len(), 30);
    assert_eq!(atlas["frames"][2]["page"], serde_json::json!(1));
    assert_eq!(atlas["frames"][2]["image"], serde_json::json!("sprite_sheet_002.png"));
}
```

- [ ] **Step 3: Run the failing high-resolution export test**

Run:

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml export_pack_splits_high_resolution_sprite_sheets
```

Expected:

```text
FAILED
```

The expected initial failure can be a missing `allow_multi_sheet` field, missing `sprite_sheet_paths`, or `TextureTooLarge`.

- [ ] **Step 4: Implement multi-page sheet writing**

In `packages/core/src/export/sheet.rs`, update `build_sprite_sheet` to call `plan_sprite_sheet_layout`. For each page:

```text
create one RgbaImage sized to the page layout
copy only that page's frames
save the page image in output_dir
push the page path into sprite_sheet_paths
write atlas.json once after all pages are saved
```

Use this mapping for atlas frames:

```rust
AtlasFrame {
    index: placement.frame_index,
    name: placement.name.clone(),
    page: placement.page_index,
    image: if layout.pages.len() > 1 {
        Some(page.image_name.clone())
    } else {
        None
    },
    x: placement.x,
    y: placement.y,
    width: frame_width,
    height: frame_height,
}
```

Set atlas page fields like this:

```rust
let images = layout
    .pages
    .iter()
    .map(|page| page.image_name.clone())
    .collect::<Vec<_>>();

let atlas = Atlas {
    image: "sprite_sheet.png".to_string(),
    images: if images.len() > 1 { images } else { Vec::new() },
    frame_width,
    frame_height,
    columns: layout.page_columns,
    rows: layout.page_rows,
    frames: atlas_frames,
};
```

- [ ] **Step 5: Copy every sheet page into `.gsfpack`**

Modify `ExportPackOutput` in `packages/core/src/export/mod.rs`:

```rust
pub struct ExportPackOutput {
    pub export_dir: PathBuf,
    pub frames_dir: PathBuf,
    pub frame_paths: Vec<PathBuf>,
    pub sprite_sheet_path: PathBuf,
    pub sprite_sheet_paths: Vec<PathBuf>,
    pub atlas_path: PathBuf,
    pub manifest_path: PathBuf,
    pub godot_helper_path: PathBuf,
    pub quality_report_path: PathBuf,
    pub preview_gif_path: PathBuf,
    pub pack_dir: PathBuf,
}
```

Change `write_pack_directory` to accept `sprite_sheet_paths: &[PathBuf]`. Copy the first page to `assets/sprite_sheet.png` and later pages by their file names:

```rust
for sprite_sheet_path in sprite_sheet_paths {
    let Some(file_name) = sprite_sheet_path.file_name() else {
        continue;
    };
    fs::copy(sprite_sheet_path, pack_dir.join("assets").join(file_name))?;
}
```

Keep `assets/sprite_sheet.png` as a required first-page file for old importers.

- [ ] **Step 6: Keep manifest compatible and add optional images**

Modify `ManifestSheet` in `packages/core/src/export/manifest.rs`:

```rust
pub struct ManifestSheet {
    pub image: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<String>,
    pub frame_width: u32,
    pub frame_height: u32,
    pub columns: u32,
    pub rows: u32,
}
```

In `engine_manifest`, set:

```rust
images: atlas
    .images
    .iter()
    .map(|image| format!("assets/{image}"))
    .collect(),
```

Keep `image: "assets/sprite_sheet.png".to_string()`.

- [ ] **Step 7: Run Rust export tests**

Run:

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml export_pack
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml sheet
```

Expected:

```text
test result: ok
```

- [ ] **Step 8: Commit**

Run:

```bash
git add packages/core/src/export/sheet.rs packages/core/src/export/mod.rs packages/core/src/export/manifest.rs
git commit -m "feat: export oversized animations across sprite sheet pages"
```

If the workspace is still not a git repository, record this command in the handoff notes instead of committing.

---

### Task 4: Update Schemas And Pack Validation

**Files:**
- Modify: `schemas/atlas.schema.json`
- Modify: `schemas/manifest.schema.json`
- Modify: `packages/pack/src/lib.rs`
- Modify: `packages/pack/tests/pack_tests.rs`

- [ ] **Step 1: Extend atlas schema**

Modify `schemas/atlas.schema.json`:

```json
"images": {
  "type": "array",
  "minItems": 2,
  "items": {
    "type": "string",
    "minLength": 1
  }
}
```

Add optional frame fields inside the frame item properties:

```json
"page": {
  "type": "integer",
  "minimum": 0
},
"image": {
  "type": "string",
  "minLength": 1
}
```

Do not add `images`, `page`, or frame `image` to `required`. Single-page exports must remain valid.

- [ ] **Step 2: Extend manifest schema**

Modify `schemas/manifest.schema.json` under `sheet.properties`:

```json
"images": {
  "type": "array",
  "minItems": 2,
  "items": {
    "type": "string",
    "minLength": 1
  }
}
```

Do not add `images` to `required`.

- [ ] **Step 3: Add a multi-page pack fixture test**

Add this test to `packages/pack/tests/pack_tests.rs`:

```rust
#[test]
fn validates_multipage_atlas_assets() {
    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    write_pack_fixture(&pack, 4);
    fs::write(pack.join("assets/sprite_sheet_002.png"), b"png").unwrap();
    fs::write(
        pack.join("assets/atlas.json"),
        json!({
            "image": "sprite_sheet.png",
            "images": ["sprite_sheet.png", "sprite_sheet_002.png"],
            "frameWidth": 16,
            "frameHeight": 16,
            "columns": 1,
            "rows": 2,
            "frames": [
                { "index": 0, "name": "frame_001.png", "x": 0, "y": 0, "width": 16, "height": 16 },
                { "index": 1, "name": "frame_002.png", "x": 0, "y": 16, "width": 16, "height": 16 },
                { "index": 2, "name": "frame_003.png", "page": 1, "image": "sprite_sheet_002.png", "x": 0, "y": 0, "width": 16, "height": 16 },
                { "index": 3, "name": "frame_004.png", "page": 1, "image": "sprite_sheet_002.png", "x": 0, "y": 16, "width": 16, "height": 16 }
            ]
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        pack.join("assets/manifest.json"),
        json!({
            "name": "Hero",
            "sheet": {
                "image": "assets/sprite_sheet.png",
                "images": ["assets/sprite_sheet.png", "assets/sprite_sheet_002.png"],
                "frameWidth": 16,
                "frameHeight": 16,
                "columns": 1,
                "rows": 2
            },
            "animations": [{
                "name": "idle",
                "frames": [0, 1, 2, 3],
                "fps": 12.0,
                "loop": true
            }],
            "anchor": { "type": "feet", "x": 8.0, "y": 16.0 }
        })
        .to_string(),
    )
    .unwrap();

    validate_pack_layout(&pack).unwrap();
}
```

Add this negative test:

```rust
#[test]
fn missing_multipage_atlas_image_fails_validation() {
    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    write_pack_fixture(&pack, 2);
    fs::write(
        pack.join("assets/atlas.json"),
        json!({
            "image": "sprite_sheet.png",
            "images": ["sprite_sheet.png", "sprite_sheet_002.png"],
            "frameWidth": 16,
            "frameHeight": 16,
            "columns": 1,
            "rows": 2,
            "frames": [
                { "index": 0, "name": "frame_001.png", "x": 0, "y": 0, "width": 16, "height": 16 },
                { "index": 1, "name": "frame_002.png", "page": 1, "image": "sprite_sheet_002.png", "x": 0, "y": 0, "width": 16, "height": 16 }
            ]
        })
        .to_string(),
    )
    .unwrap();

    let error = validate_pack_layout(&pack).unwrap_err();

    assert!(matches!(error, PackError::MissingFile(path) if path == "assets/sprite_sheet_002.png"));
}
```

- [ ] **Step 4: Run the failing pack tests**

Run:

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml validates_multipage_atlas_assets missing_multipage_atlas_image_fails_validation
```

Expected:

```text
FAILED
```

- [ ] **Step 5: Validate atlas image pages in pack validator**

In `packages/pack/src/lib.rs`, after JSON schema validation of `assets/atlas.json`, read atlas images:

```rust
let atlas: serde_json::Value = read_json(pack_path.join("assets/atlas.json"))?;
if let Some(images) = atlas.get("images").and_then(|value| value.as_array()) {
    for image in images {
        let Some(image) = image.as_str() else {
            continue;
        };
        let relative = format!("assets/{image}");
        if !pack_path.join(&relative).exists() {
            return Err(PackError::MissingFile(relative));
        }
    }
}
```

This keeps old packs valid because `images` is optional.

- [ ] **Step 6: Run pack tests**

Run:

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml -p forge_pack
```

Expected:

```text
test result: ok
```

- [ ] **Step 7: Commit**

Run:

```bash
git add schemas/atlas.schema.json schemas/manifest.schema.json packages/pack/src/lib.rs packages/pack/tests/pack_tests.rs
git commit -m "feat: validate multipage sprite sheet packs"
```

If the workspace is still not a git repository, record this command in the handoff notes instead of committing.

---

### Task 5: Add Export UI Controls And Recovery Copy

**Files:**
- Modify: `apps/mac/src/tauriCommands.ts`
- Modify: `apps/mac/src/components/ExportPanel.tsx`
- Modify: `apps/mac/src/routes/ForgeRoute.tsx`
- Modify: `apps/mac/src/styles/app.css`
- Modify: `scripts/test-recovery-plan-source.mjs`
- Modify: `apps/mac/scripts/smoke-ui.mjs`

- [ ] **Step 1: Update TypeScript command types**

Modify `ExportPackOutput` in `apps/mac/src/tauriCommands.ts`:

```ts
export type ExportPackOutput = {
  exportDir: string;
  framesDir: string;
  framePaths: string[];
  spriteSheetPath: string;
  spriteSheetPaths: string[];
  atlasPath: string;
  manifestPath: string;
  godotHelperPath: string;
  qualityReportPath: string;
  previewGifPath: string;
  packDir: string;
};
```

Extend `exportPack` args:

```ts
  allowMultiSheet: boolean;
```

Send it to Rust:

```ts
sheet: {
  columns: Math.max(1, Math.round(args.sheetColumns)),
  paddingPx: Math.max(0, Math.round(args.sheetPaddingPx)),
  marginPx: Math.max(0, Math.round(args.sheetMarginPx)),
  maxTextureSize: args.sheetSize,
  allowMultiSheet: args.allowMultiSheet,
},
```

- [ ] **Step 2: Add ExportPanel props**

In `apps/mac/src/components/ExportPanel.tsx`, add props:

```ts
  allowMultiSheet: boolean;
  onAllowMultiSheetChange: (value: boolean) => void;
```

Render this checkbox inside `.export-advanced-body` after the sheet margin field:

```tsx
<label className="export-toggle">
  <input
    checked={allowMultiSheet}
    disabled={disabled}
    onChange={(event) => onAllowMultiSheetChange(event.target.checked)}
    type="checkbox"
  />
  <span>Split sheets when needed</span>
</label>
```

Render a compact output note when `exportOutput.spriteSheetPaths.length > 1`:

```tsx
{exportOutput && exportOutput.spriteSheetPaths.length > 1 ? (
  <p className="export-path">
    Sprite sheet pages: {exportOutput.spriteSheetPaths.length}
  </p>
) : null}
```

- [ ] **Step 3: Wire state in ForgeRoute**

In `apps/mac/src/routes/ForgeRoute.tsx`, add state near the existing sheet settings:

```ts
const [allowMultiSheet, setAllowMultiSheet] = useState(true);
```

Pass `allowMultiSheet` into `exportPack`:

```ts
allowMultiSheet,
```

Pass props into `ExportPanel`:

```tsx
allowMultiSheet={allowMultiSheet}
onAllowMultiSheetChange={setAllowMultiSheet}
```

- [ ] **Step 4: Route oversized texture errors before generic sprite-sheet recovery**

In `recoveryPlanFor`, add this branch before the current sprite sheet/grid branch:

```ts
if (/(exceeds max texture size|texture too large|max texture)/.test(lower)) {
  return {
    actions: [
      { key: "export", label: "Review export" },
      { key: "settings", label: "Sheet size" },
    ],
    detail: "Enable sheet splitting, raise the max texture size, or reduce columns before exporting again.",
    title: "Sprite sheet is too large",
  };
}
```

- [ ] **Step 5: Update source tests**

Modify `scripts/test-recovery-plan-source.mjs` to assert the oversized branch exists before the generic sprite sheet branch:

```js
assertContains(
  "Sprite sheet is too large",
  "Oversized export recovery must name the texture-size problem.",
);
assertContains(
  "Enable sheet splitting, raise the max texture size, or reduce columns before exporting again.",
  "Oversized export recovery must give sheet splitting guidance.",
);
assertBefore(
  "exceeds max texture size",
  "/(sprite sheet|grid|slice|cell|atlas)/",
  "Oversized export recovery must run before generic sprite-sheet recovery.",
);
```

Modify `apps/mac/scripts/smoke-ui.mjs` so `mvpRequired` contains:

```js
"Split sheets when needed",
```

- [ ] **Step 6: Run frontend checks**

Run:

```bash
npm run test:scripts
npm --workspace apps/mac run build
npm --workspace apps/mac run smoke:ui:mvp
```

Expected:

```text
PASS recovery plan source test
tsc --noEmit: pass
vite build: pass
UI smoke passed (mvp)
```

- [ ] **Step 7: Commit**

Run:

```bash
git add apps/mac/src/tauriCommands.ts apps/mac/src/components/ExportPanel.tsx apps/mac/src/routes/ForgeRoute.tsx apps/mac/src/styles/app.css apps/mac/scripts/smoke-ui.mjs scripts/test-recovery-plan-source.mjs
git commit -m "feat: add multipage sheet export controls"
```

If the workspace is still not a git repository, record this command in the handoff notes instead of committing.

---

### Task 6: Verify QA-004 With Real High-Resolution Video

**Files:**
- Modify: `docs/qa/forge-manual-mvp-checklist-2026-06-05.md`
- Modify: `docs/architecture/mvp-scope.md`
- Modify: `docs/architecture/post-release-backlog.md`

- [ ] **Step 1: Build and launch the app**

Run:

```bash
npm --workspace apps/mac run tauri -- build
open -n "/Applications/Game Sprite Forge.app"
```

Expected:

```text
Tauri build: pass
Game Sprite Forge opens
```

If the Tauri build does not install to `/Applications`, open the generated app bundle from `target/release/bundle/macos/Game Sprite Forge.app`.

- [ ] **Step 2: Use Computer Use to reproduce the high-resolution export**

Use Computer Use against the real app UI:

```text
Import video: /Users/kartz/Development/social-auto-upload/videos/demo.mp4
Extract frames: use the same 30-frame path previously used for QA-004
Process & Quality: complete
Export settings:
  Sheet Columns: 4
  Sheet Padding: 2
  Sheet Margin: 2
  Split sheets when needed: enabled
  Max texture size: 2048
Export Pack: click
Validate Re-import: click
```

Expected:

```text
Export Pack completes
Sprite sheet pages count is greater than 1
Every generated sprite_sheet*.png page is <= 2048 x 2048
Validate Re-import succeeds
Re-imported frame count is 30
```

- [ ] **Step 3: Verify exported files from the shell**

Set `EXPORT_DIR` to the export directory shown by the app, then run:

```bash
find "$EXPORT_DIR" -maxdepth 1 -name 'sprite_sheet*.png' -print | sort
python3 - <<'PY'
from pathlib import Path
from PIL import Image
import json
import os

export_dir = Path(os.environ["EXPORT_DIR"])
pages = sorted(export_dir.glob("sprite_sheet*.png"))
assert len(pages) > 1, f"expected multiple pages, got {len(pages)}"
for page in pages:
    image = Image.open(page)
    assert image.width <= 2048, (page, image.width)
    assert image.height <= 2048, (page, image.height)

atlas = json.loads((export_dir / "atlas.json").read_text())
assert len(atlas["frames"]) == 30
assert len(atlas["images"]) == len(pages)
print("PASS high-resolution multipage export evidence")
PY
```

Expected:

```text
PASS high-resolution multipage export evidence
```

If the bundled Python environment does not have Pillow, use macOS `sips -g pixelWidth -g pixelHeight "$page"` for each page instead.

- [ ] **Step 4: Update QA-004**

In `docs/qa/forge-manual-mvp-checklist-2026-06-05.md`, replace the QA-004 row with:

```markdown
| QA-004 | High-resolution video export sizing | Manual app QA | resolved | `/Users/kartz/Development/social-auto-upload/videos/demo.mp4` exported with multi-page sprite sheets; every `sprite_sheet*.png` page stayed within 2048 x 2048; `.gsfpack` validation and re-import preserved 30 frames | Resolved by multi-page sheet export and explicit oversized-sheet recovery guidance |
```

Update the release gate summary:

```text
Non-blocking issues: none currently open
```

- [ ] **Step 5: Update MVP scope**

In `docs/architecture/mvp-scope.md`, replace the QA-004 follow-up paragraph with:

```text
High-resolution video sizing follow-up: resolved; /Users/kartz/Development/social-auto-upload/videos/demo.mp4 exports with multi-page sprite sheets under the selected max texture size, validates as a .gsfpack, and re-imports with 30 frames.
```

Replace:

```text
Non-blocking follow-up: QA-004.
```

with:

```text
Non-blocking follow-up: none currently open.
```

- [ ] **Step 6: Update post-release backlog**

In `docs/architecture/post-release-backlog.md`, add this completed note under Slice 2:

````markdown
Resolved before Slice 2 start:

```text
QA-004 high-resolution video export sizing now uses multi-page sprite sheets and no longer blocks large local videos at the first 2048 texture limit.
```
````

- [ ] **Step 7: Commit**

Run:

```bash
git add docs/qa/forge-manual-mvp-checklist-2026-06-05.md docs/architecture/mvp-scope.md docs/architecture/post-release-backlog.md
git commit -m "docs: record high resolution export QA evidence"
```

If the workspace is still not a git repository, record this command in the handoff notes instead of committing.

---

### Task 7: Final Verification And Release Candidate Refresh

**Files:**
- Verify: `release-candidates/GameSpriteForge-0.1.0-aarch64-f2060117105d-notarized`
- Optional modify after rebuild: `docs/architecture/distribution-mvp.md`
- Optional modify after rebuild: `docs/architecture/release-candidate-verification.md`

- [ ] **Step 1: Run full local verification**

Run:

```bash
npm run test:scripts
npm --workspace apps/mac run build
npm --workspace apps/mac run smoke:ui:mvp
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml
```

Expected:

```text
PASS manual QA fixture test
PASS import panel source test
PASS recovery plan source test
PASS notarization preflight test
vite build: pass
UI smoke passed (mvp)
cargo tests: pass
```

- [ ] **Step 2: Build the release app and DMG**

Run:

```bash
npm --workspace apps/mac run tauri -- build
shasum -a 256 "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
```

Expected:

```text
Finished 1 bundle at target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg
new DMG SHA-256 printed
```

- [ ] **Step 3: Notarize and staple the rebuilt DMG**

Run:

```bash
xcrun notarytool submit "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg" \
  --keychain-profile "GameSpriteForgeNotary" \
  --wait
xcrun stapler staple -v "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
xcrun stapler validate -v "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
```

Expected:

```text
notarytool status: Accepted
The staple and validate action worked!
```

- [ ] **Step 4: Run release verifier**

Run with the new SHA printed in Step 2:

```bash
scripts/verify-release-package.sh "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg" \
  "$(shasum -a 256 "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg" | awk '{print $1}')"
```

Expected:

```text
PASS release package verification script completed.
```

- [ ] **Step 5: Create refreshed release candidate package**

Run:

```bash
scripts/package-release-candidate.sh
```

Expected:

```text
Release candidate package created:
Directory: /Users/kartz/Development/Forge/release-candidates/GameSpriteForge-0.1.0-aarch64-{first 12 characters of the new DMG SHA}-notarized
Zip: /Users/kartz/Development/Forge/release-candidates/GameSpriteForge-0.1.0-aarch64-{first 12 characters of the new DMG SHA}-notarized.zip
```

- [ ] **Step 6: Update release docs with the new SHA**

If the DMG SHA changed, update:

```text
docs/architecture/distribution-mvp.md
docs/architecture/release-candidate-verification.md
docs/architecture/mvp-scope.md
docs/qa/forge-clean-env-smoke-2026-06-05.md
```

Replace the previous SHA and package path with the new SHA-prefixed package created in Step 5.

- [ ] **Step 7: Commit**

Run:

```bash
git add docs/architecture/distribution-mvp.md docs/architecture/release-candidate-verification.md docs/architecture/mvp-scope.md docs/qa/forge-clean-env-smoke-2026-06-05.md release-candidates
git commit -m "chore: refresh notarized release candidate after multipage export"
```

If the workspace is still not a git repository, record this command in the handoff notes instead of committing.

---

## Verification Commands

Run before claiming this slice complete:

```bash
npm run test:scripts
npm --workspace apps/mac run build
npm --workspace apps/mac run smoke:ui:mvp
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml
scripts/verify-release-package.sh "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg" \
  "$(shasum -a 256 "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg" | awk '{print $1}')"
```

Expected final state:

```text
clean environment smoke rows are recorded
small exports still create one sprite_sheet.png and validate as existing packs
high-resolution exports create multiple sprite_sheet*.png files within max texture size
atlas.json references every page used by frames
manifest.json remains valid and includes optional sheet.images for multi-page exports
.gsfpack validation passes for single-page and multi-page packs
demo.mp4 re-import preserves 30 frames
notarized release verifier passes after rebuild
```

## Self-Review

Spec coverage:

- Clean-environment confidence is covered by Task 1.
- QA-004 high-resolution failure is covered by Tasks 2, 3, 4, 5, and 6.
- Backward compatibility for single-page packs is covered by preserving `sprite_sheet.png`, keeping optional schema fields optional, and running existing pack tests.
- UI recovery copy is covered by Task 5.
- Release candidate refresh is covered by Task 7.

Placeholder scan:

```text
No unresolved placeholder labels remain.
Every code-facing task names the exact files, commands, expected outputs, and core snippets.
```

Type consistency:

```text
Rust uses allow_multi_sheet and TypeScript sends allowMultiSheet through serde camelCase.
Rust output uses sprite_sheet_paths and TypeScript reads spriteSheetPaths.
Atlas keeps legacy image and adds optional images, page, and frame image fields.
Manifest keeps legacy sheet.image and adds optional sheet.images.
```

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-06-05-forge-high-resolution-export-and-clean-env-smoke-plan.md`.

Two execution options:

1. Subagent-Driven (recommended): dispatch a fresh subagent per task, review between tasks, and use Computer Use only for Task 1 and Task 6 UI verification.
2. Inline Execution: execute tasks in this session with `superpowers:executing-plans`, batching Rust/schema work first and UI/QA verification second.
