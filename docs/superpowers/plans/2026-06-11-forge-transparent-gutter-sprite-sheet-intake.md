# Forge Transparent-Gutter Sprite Sheet Intake Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a local automatic sprite sheet import mode that splits frames by transparent gutters and is implemented inside Forge's own media pipeline.

**Architecture:** Keep the feature inside Forge's existing local media pipeline. `forge_core` owns pixel scanning and frame writing, Tauri exposes a job-scoped command, TypeScript wraps the command, and the React import panel lets users choose manual grid or transparent-gutter split before normalization and quality reporting continue unchanged.

**Tech Stack:** Rust `image` crate, Tauri 2 commands, React 18, TypeScript, existing Forge i18n/source-guard scripts, existing `ExtractFramesResult` and job directory layout.

---

## Scope

This plan implements only the first transparent-gutter slice:

```text
automatic transparent-gutter splitting for sprite sheet import
```

It does not implement:

```text
frame repair/nudge/crop UI
GIF/WebP import/export utilities
RPG Maker presets
AI matting
cloud workers
external project source reuse
```

Broader rationale is documented in:

```text
docs/architecture/sprite-tooling-followups.md
```

## File Structure

- Modify: `packages/core/src/video/sprite_sheet.rs` - add transparent-gutter split parameters, algorithm, and unit tests.
- Modify: `packages/core/src/video/mod.rs` - export the new params and function.
- Modify: `apps/mac/src-tauri/src/lib.rs` - expose a job-scoped `slice_sprite_sheet_transparent` command.
- Modify: `apps/mac/src/tauriCommands.ts` - add TypeScript types and command wrapper.
- Modify: `apps/mac/src/components/ImportPanel.tsx` - add split mode controls and transparent split parameters.
- Modify: `apps/mac/src/routes/ForgeRoute.tsx` - store split mode state and call the selected command.
- Modify: `apps/mac/src/i18n.ts` - add English and Simplified Chinese labels.
- Modify: `scripts/test-import-panel-source.mjs` - guard that the import panel exposes both split modes and command wiring.
- Create: `docs/qa/transparent-gutter-sprite-split-evidence-2026-06-11.md` - record command/test/UI evidence after implementation.
- Create: `docs/qa/sprite-sheet-intake-multi-agent-execution-2026-06-11.md` - record skill/MCP research, agent roles, and execution evidence.

## Skill And MCP Operating Model

Use these local skills for this plan:

```text
superpowers:subagent-driven-development
superpowers:dispatching-parallel-agents
superpowers:verification-before-completion
```

Use these MCP tools:

```text
multi_agent_v1.spawn_agent for worker/explorer/reviewer agents
multi_agent_v1.wait_agent to collect blocking agent results
multi_agent_v1.close_agent after results are integrated
```

Do not use these for this slice:

```text
Figma MCP: no design import or Figma source
GitHub MCP: no PR/issue/CI task requested
Ark docs skills: inspected for coding skills; no Forge-relevant local implementation skill found
Chrome/Browser MCP: not needed until local UI visual QA is requested or smoke detects a UI issue
```

Agent split:

```text
worker: Rust core transparent split implementation
explorer: Tauri/TS/UI/source-guard integration research
reviewer: spec compliance review after implementation
reviewer: code quality review after spec compliance
```

## Verification Commands

Run before marking the plan complete:

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml sprite_sheet_transparent
npm --workspace apps/mac run build
npm run test:scripts
npm --workspace apps/mac run smoke:ui:mvp
```

Expected:

```text
Rust transparent split tests: pass
frontend build: pass
script source guards: pass
MVP UI smoke: pass
```

---

### Task 1: Add Transparent Split To `forge_core`

**Files:**
- Modify: `packages/core/src/video/sprite_sheet.rs`
- Modify: `packages/core/src/video/mod.rs`

- [ ] **Step 1: Add failing tests**

Append these tests inside the existing `#[cfg(test)] mod tests` in
`packages/core/src/video/sprite_sheet.rs`:

```rust
#[test]
fn sprite_sheet_transparent_splits_guttered_regions() {
    let temp = tempfile::tempdir().unwrap();
    let sheet_path = temp.path().join("transparent-sheet.png");
    let output_dir = temp.path().join("job");
    write_transparent_gutter_sheet(&sheet_path);

    let result = slice_sprite_sheet_transparent(&SliceSpriteSheetTransparentParams {
        sheet_path,
        output_directory: output_dir,
        alpha_threshold: 0,
        min_gap_px: 1,
    })
    .unwrap();

    assert_eq!(result.frames.len(), 4);
    let colors = [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 0, 255],
    ];
    for (frame, expected) in result.frames.iter().zip(colors) {
        let image = image::open(frame).unwrap().to_rgba8();
        assert_eq!(image.width(), 4);
        assert_eq!(image.height(), 4);
        assert_eq!(image.get_pixel(1, 1).0, expected);
    }
}

#[test]
fn sprite_sheet_transparent_rejects_empty_sheet() {
    let temp = tempfile::tempdir().unwrap();
    let sheet_path = temp.path().join("empty.png");
    RgbaImage::new(8, 8).save(&sheet_path).unwrap();

    let error = slice_sprite_sheet_transparent(&SliceSpriteSheetTransparentParams {
        sheet_path,
        output_directory: temp.path().join("job"),
        alpha_threshold: 0,
        min_gap_px: 1,
    })
    .unwrap_err();

    assert_eq!(error.message, "sprite sheet contains no opaque regions");
}

#[test]
fn sprite_sheet_transparent_removes_stale_pngs_before_slicing() {
    let temp = tempfile::tempdir().unwrap();
    let sheet_path = temp.path().join("transparent-sheet.png");
    let output_dir = temp.path().join("job");
    let raw_dir = output_dir.join("raw");
    fs::create_dir_all(&raw_dir).unwrap();
    fs::write(raw_dir.join("frame_99999.png"), b"stale").unwrap();
    write_transparent_gutter_sheet(&sheet_path);

    let result = slice_sprite_sheet_transparent(&SliceSpriteSheetTransparentParams {
        sheet_path,
        output_directory: output_dir,
        alpha_threshold: 0,
        min_gap_px: 1,
    })
    .unwrap();

    assert_eq!(result.frames.len(), 4);
    assert!(!raw_dir.join("frame_99999.png").exists());
}

fn write_transparent_gutter_sheet(sheet_path: &std::path::Path) {
    let mut sheet = RgbaImage::new(10, 10);
    paint_rect(&mut sheet, 0, 0, 4, 4, Rgba([255, 0, 0, 255]));
    paint_rect(&mut sheet, 6, 0, 4, 4, Rgba([0, 255, 0, 255]));
    paint_rect(&mut sheet, 0, 6, 4, 4, Rgba([0, 0, 255, 255]));
    paint_rect(&mut sheet, 6, 6, 4, 4, Rgba([255, 255, 0, 255]));
    sheet.save(sheet_path).unwrap();
}

fn paint_rect(image: &mut RgbaImage, x0: u32, y0: u32, width: u32, height: u32, color: Rgba<u8>) {
    for y in y0..y0 + height {
        for x in x0..x0 + width {
            image.put_pixel(x, y, color);
        }
    }
}
```

- [ ] **Step 2: Run tests and confirm the missing API**

Run:

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml sprite_sheet_transparent
```

Expected before implementation:

```text
cannot find function `slice_sprite_sheet_transparent`
cannot find struct `SliceSpriteSheetTransparentParams`
```

- [ ] **Step 3: Add params and implementation**

Add this public type below `SliceSpriteSheetParams`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SliceSpriteSheetTransparentParams {
    pub sheet_path: PathBuf,
    pub output_directory: PathBuf,
    pub alpha_threshold: u8,
    pub min_gap_px: u32,
}
```

Add this implementation below `slice_sprite_sheet_grid`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SpriteRegion {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

pub fn slice_sprite_sheet_transparent(
    params: &SliceSpriteSheetTransparentParams,
) -> Result<ExtractFramesResult, VideoError> {
    let sheet = image::open(&params.sheet_path)
        .map_err(|error| VideoError::invalid_params(error.to_string()))?
        .to_rgba8();
    let regions = transparent_regions(&sheet, params.alpha_threshold, params.min_gap_px.max(1));
    if regions.is_empty() {
        return Err(VideoError::invalid_params(
            "sprite sheet contains no opaque regions",
        ));
    }

    let frame_width = regions.iter().map(|region| region.width).max().unwrap_or(1);
    let frame_height = regions.iter().map(|region| region.height).max().unwrap_or(1);
    let raw_directory = params.output_directory.join("raw");
    fs::create_dir_all(&raw_directory)?;
    remove_existing_png_frames(&raw_directory)?;

    let mut frames = Vec::with_capacity(regions.len());
    for (index, region) in regions.iter().enumerate() {
        let source = image::imageops::crop_imm(
            &sheet,
            region.x,
            region.y,
            region.width,
            region.height,
        )
        .to_image();
        let mut frame = image::RgbaImage::new(frame_width, frame_height);
        let dx = (frame_width - region.width) / 2;
        let dy = frame_height - region.height;
        image::imageops::replace(&mut frame, &source, dx.into(), dy.into());
        let path = raw_directory.join(format!("frame_{:05}.png", index + 1));
        frame
            .save(&path)
            .map_err(|error| VideoError::invalid_params(error.to_string()))?;
        frames.push(path);
    }

    Ok(ExtractFramesResult {
        raw_directory,
        frames,
    })
}
```

Add these private helpers:

```rust
fn transparent_regions(
    image: &image::RgbaImage,
    alpha_threshold: u8,
    min_gap_px: u32,
) -> Vec<SpriteRegion> {
    let row_gaps = transparent_row_runs(image, alpha_threshold, min_gap_px);
    let row_regions = content_regions_from_gaps(&row_gaps, image.height());
    let mut regions = Vec::new();

    for (y0, y1) in row_regions {
        let col_gaps = transparent_col_runs(image, y0, y1, alpha_threshold, min_gap_px);
        for (x0, x1) in content_regions_from_gaps(&col_gaps, image.width()) {
            if region_has_opaque_pixel(image, x0, y0, x1, y1, alpha_threshold) {
                regions.push(SpriteRegion {
                    x: x0,
                    y: y0,
                    width: x1 - x0,
                    height: y1 - y0,
                });
            }
        }
    }

    regions
}

fn transparent_row_runs(
    image: &image::RgbaImage,
    alpha_threshold: u8,
    min_gap_px: u32,
) -> Vec<(u32, u32)> {
    let mut rows = Vec::new();
    for y in 0..image.height() {
        let transparent = (0..image.width()).all(|x| image.get_pixel(x, y).0[3] <= alpha_threshold);
        if transparent {
            rows.push(y);
        }
    }
    runs_with_min_len(&rows, min_gap_px)
}

fn transparent_col_runs(
    image: &image::RgbaImage,
    y0: u32,
    y1: u32,
    alpha_threshold: u8,
    min_gap_px: u32,
) -> Vec<(u32, u32)> {
    let mut cols = Vec::new();
    for x in 0..image.width() {
        let transparent = (y0..y1).all(|y| image.get_pixel(x, y).0[3] <= alpha_threshold);
        if transparent {
            cols.push(x);
        }
    }
    runs_with_min_len(&cols, min_gap_px)
}

fn runs_with_min_len(values: &[u32], min_gap_px: u32) -> Vec<(u32, u32)> {
    if values.is_empty() {
        return Vec::new();
    }
    let mut runs = Vec::new();
    let mut start = values[0];
    let mut last = start;
    for value in values.iter().copied().skip(1) {
        if value == last + 1 {
            last = value;
        } else {
            if last - start + 1 >= min_gap_px {
                runs.push((start, last + 1));
            }
            start = value;
            last = value;
        }
    }
    if last - start + 1 >= min_gap_px {
        runs.push((start, last + 1));
    }
    runs
}

fn content_regions_from_gaps(gaps: &[(u32, u32)], total: u32) -> Vec<(u32, u32)> {
    let mut regions = Vec::new();
    let mut cursor = 0;
    for &(start, end) in gaps {
        if cursor < start {
            regions.push((cursor, start));
        }
        cursor = cursor.max(end);
    }
    if cursor < total {
        regions.push((cursor, total));
    }
    regions
}

fn region_has_opaque_pixel(
    image: &image::RgbaImage,
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
    alpha_threshold: u8,
) -> bool {
    (y0..y1).any(|y| (x0..x1).any(|x| image.get_pixel(x, y).0[3] > alpha_threshold))
}
```

- [ ] **Step 4: Export the new API**

In `packages/core/src/video/mod.rs`, replace the sprite sheet export line with:

```rust
pub use sprite_sheet::{
    slice_sprite_sheet_grid, slice_sprite_sheet_transparent, SliceSpriteSheetParams,
    SliceSpriteSheetTransparentParams,
};
```

- [ ] **Step 5: Run Rust tests**

Run:

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml sprite_sheet_transparent
```

Expected:

```text
test sprite_sheet_transparent_splits_guttered_regions ... ok
test sprite_sheet_transparent_rejects_empty_sheet ... ok
test sprite_sheet_transparent_removes_stale_pngs_before_slicing ... ok
```

### Task 2: Expose A Job-Scoped Tauri Command

**Files:**
- Modify: `apps/mac/src-tauri/src/lib.rs`
- Modify: `apps/mac/src/tauriCommands.ts`

- [ ] **Step 1: Add the Tauri command**

In `apps/mac/src-tauri/src/lib.rs`, add this command next to `slice_sprite_sheet`:

```rust
#[tauri::command]
fn slice_sprite_sheet_transparent(
    params: forge_core::video::SliceSpriteSheetTransparentParams,
) -> Result<forge_core::video::ExtractFramesResult, String> {
    let input_job = ensure_job_scoped_existing_path(&params.sheet_path)?;
    let output_job = ensure_job_scoped_output(&params.output_directory)?;
    ensure_same_job(&input_job, &output_job)?;

    let result = forge_core::video::slice_sprite_sheet_transparent(&params)
        .map_err(|error| error.message)?;
    mark_job_state(
        &output_job,
        forge_core::job::types::JobState::FramesExtracted,
    )?;
    Ok(result)
}
```

Add it to `tauri::generate_handler!` immediately after `slice_sprite_sheet`:

```rust
slice_sprite_sheet_transparent,
```

- [ ] **Step 2: Add the TypeScript wrapper**

In `apps/mac/src/tauriCommands.ts`, add this wrapper after `sliceSpriteSheet`:

```ts
export function sliceSpriteSheetTransparent(args: {
  sheetPath: string;
  jobDir: string;
  alphaThreshold: number;
  minGapPx: number;
}) {
  return invoke<ExtractFramesResult>("slice_sprite_sheet_transparent", {
    params: {
      sheetPath: args.sheetPath,
      outputDirectory: args.jobDir,
      alphaThreshold: args.alphaThreshold,
      minGapPx: args.minGapPx,
    },
  });
}
```

- [ ] **Step 3: Run command-level compilation**

Run:

```bash
cargo check --manifest-path /Users/kartz/Development/Forge/apps/mac/src-tauri/Cargo.toml
npm --workspace apps/mac run build
```

Expected:

```text
cargo check: pass
frontend build: pass
```

### Task 3: Add Import UI Mode Selection

**Files:**
- Modify: `apps/mac/src/components/ImportPanel.tsx`
- Modify: `apps/mac/src/routes/ForgeRoute.tsx`
- Modify: `apps/mac/src/i18n.ts`

- [ ] **Step 1: Add state and wrapper import**

In `apps/mac/src/routes/ForgeRoute.tsx`, add `sliceSpriteSheetTransparent` to the existing import from `../tauriCommands`.

Add state next to `spriteGrid`:

```ts
const [spriteSheetSliceMode, setSpriteSheetSliceMode] = useState<"grid" | "transparent">("grid");
const [transparentSplitAlphaThreshold, setTransparentSplitAlphaThreshold] = useState(0);
const [transparentSplitMinGapPx, setTransparentSplitMinGapPx] = useState(1);
```

- [ ] **Step 2: Branch sprite sheet import**

In `runSpriteSheetImportPipeline`, replace the `sliceSpriteSheet` call with:

```ts
const sliced = spriteSheetSliceMode === "transparent"
  ? await sliceSpriteSheetTransparent({
      sheetPath,
      jobDir: current.job_dir,
      alphaThreshold: transparentSplitAlphaThreshold,
      minGapPx: transparentSplitMinGapPx,
    })
  : await sliceSpriteSheet({
      sheetPath,
      jobDir: current.job_dir,
      frameWidth: spriteGrid.frameWidth,
      frameHeight: spriteGrid.frameHeight,
      columns: spriteGrid.columns,
      rows: spriteGrid.rows,
    });
```

Keep all downstream calls the same:

```ts
setExtractResult(sliced);
const processed = await processAndNormalize(current, sliced.frames);
```

- [ ] **Step 3: Pass mode props into `ImportPanel`**

Add these props to the `ImportPanel` call:

```tsx
onSpriteSheetSliceModeChange={setSpriteSheetSliceMode}
onTransparentSplitAlphaThresholdChange={setTransparentSplitAlphaThreshold}
onTransparentSplitMinGapPxChange={setTransparentSplitMinGapPx}
spriteSheetSliceMode={spriteSheetSliceMode}
transparentSplitAlphaThreshold={transparentSplitAlphaThreshold}
transparentSplitMinGapPx={transparentSplitMinGapPx}
```

- [ ] **Step 4: Extend `ImportPanelProps`**

In `apps/mac/src/components/ImportPanel.tsx`, add:

```ts
onSpriteSheetSliceModeChange: (value: "grid" | "transparent") => void;
onTransparentSplitAlphaThresholdChange: (value: number) => void;
onTransparentSplitMinGapPxChange: (value: number) => void;
spriteSheetSliceMode: "grid" | "transparent";
transparentSplitAlphaThreshold: number;
transparentSplitMinGapPx: number;
```

Destructure those props in `ImportPanel`.

- [ ] **Step 5: Render split mode controls**

Replace the unconditional `sprite-grid-fields` block with:

```tsx
<div className="sprite-split-mode" role="group" aria-label={t("import.spriteSheetSplitMode")}>
  <button
    className={spriteSheetSliceMode === "grid" ? "active" : ""}
    disabled={disabled}
    onClick={() => onSpriteSheetSliceModeChange("grid")}
    type="button"
  >
    {t("import.spriteSheetSplitGrid")}
  </button>
  <button
    className={spriteSheetSliceMode === "transparent" ? "active" : ""}
    disabled={disabled}
    onClick={() => onSpriteSheetSliceModeChange("transparent")}
    type="button"
  >
    {t("import.spriteSheetSplitTransparent")}
  </button>
</div>
{spriteSheetSliceMode === "grid" ? (
  <div className="sprite-grid-fields">
    <GridField label="W" onChange={(value) => onSpriteGridChange({ ...spriteGrid, frameWidth: value })} value={spriteGrid.frameWidth} />
    <GridField label="H" onChange={(value) => onSpriteGridChange({ ...spriteGrid, frameHeight: value })} value={spriteGrid.frameHeight} />
    <GridField label={t("import.grid.columns")} onChange={(value) => onSpriteGridChange({ ...spriteGrid, columns: value })} value={spriteGrid.columns} />
    <GridField label={t("import.grid.rows")} onChange={(value) => onSpriteGridChange({ ...spriteGrid, rows: value })} value={spriteGrid.rows} />
  </div>
) : (
  <div className="sprite-grid-fields">
    <GridField label={t("import.transparent.alpha")} onChange={onTransparentSplitAlphaThresholdChange} value={transparentSplitAlphaThreshold} />
    <GridField label={t("import.transparent.gap")} onChange={onTransparentSplitMinGapPxChange} value={transparentSplitMinGapPx} />
  </div>
)}
```

- [ ] **Step 6: Add translations**

In `apps/mac/src/i18n.ts`, add these keys to both language dictionaries:

```ts
"import.spriteSheetSplitMode": "Sprite sheet split mode",
"import.spriteSheetSplitGrid": "Grid",
"import.spriteSheetSplitTransparent": "Transparent gutters",
"import.transparent.alpha": "Alpha",
"import.transparent.gap": "Gap",
```

Simplified Chinese:

```ts
"import.spriteSheetSplitMode": "精灵表切分模式",
"import.spriteSheetSplitGrid": "固定网格",
"import.spriteSheetSplitTransparent": "透明间隔",
"import.transparent.alpha": "Alpha",
"import.transparent.gap": "间隔",
```

- [ ] **Step 7: Add minimal CSS if needed**

If the buttons render unstyled, add this to `apps/mac/src/styles/app.css` near the sprite grid styles:

```css
.sprite-split-mode {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 6px;
}

.sprite-split-mode button {
  min-height: 32px;
}

.sprite-split-mode button.active {
  border-color: rgba(22, 198, 244, 0.58);
  background: linear-gradient(180deg, #1e3a47, #152b35);
  color: #dff7ff;
}
```

- [ ] **Step 8: Build the UI**

Run:

```bash
npm --workspace apps/mac run build
```

Expected:

```text
TypeScript: pass
Vite build: pass
```

### Task 4: Add Source Guards And Evidence

**Files:**
- Modify: `scripts/test-import-panel-source.mjs`
- Create: `docs/qa/transparent-gutter-sprite-split-evidence-2026-06-11.md`

- [ ] **Step 1: Add source guard assertions**

In `scripts/test-import-panel-source.mjs`, add this source read next to the
existing `source`, `forgeRouteSource`, and `i18nSource` reads:

```js
const tauriCommandsSource = readFileSync("apps/mac/src/tauriCommands.ts", "utf8");
```

Add this helper near the existing `assertSourceContains`, `assertI18nContains`,
and `assertForgeRouteContains` helpers:

```js
function assertTauriCommandsContains(needle, message) {
  if (!tauriCommandsSource.includes(needle)) {
    throw new Error(message);
  }
}
```

Add these assertions near the existing import panel assertions:

```js
assertSourceContains(
  "spriteSheetSliceMode",
  "ImportPanel must expose sprite sheet split mode.",
);
assertSourceContains(
  "import.spriteSheetSplitTransparent",
  "ImportPanel must expose transparent-gutter split mode.",
);
assertTauriCommandsContains(
  "sliceSpriteSheetTransparent",
  "TypeScript must wrap transparent sprite sheet splitting.",
);
assertForgeRouteContains(
  "sliceSpriteSheetTransparent",
  "ForgeRoute must call transparent sprite sheet splitting.",
);
assertForgeRouteContains(
  "transparentSplitAlphaThreshold",
  "ForgeRoute must keep transparent split alpha threshold state.",
);
assertForgeRouteContains(
  "transparentSplitMinGapPx",
  "ForgeRoute must keep transparent split gap state.",
);
```

- [ ] **Step 2: Create QA evidence document**

Create `docs/qa/transparent-gutter-sprite-split-evidence-2026-06-11.md`:

````markdown
# Transparent-Gutter Sprite Split Evidence

Date: 2026-06-11

## Scope

This evidence covers Forge's local automatic sprite sheet split mode for sheets
with transparent gutters. It does not use or copy external project code.

## Commands

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml sprite_sheet_transparent
npm --workspace apps/mac run build
npm run test:scripts
npm --workspace apps/mac run smoke:ui:mvp
```

## Result

```text
Rust transparent split tests: pass
frontend build: pass
script source guards: pass
MVP UI smoke: pass
```

## Manual Notes

```text
Manual grid import remains available.
Transparent-gutter import writes raw frames into the same job raw directory.
Downstream normalization, quality report, and export use the existing pipeline.
Non-transparent sheets should use fixed grid mode.
```
````

- [ ] **Step 3: Run all verification**

Run:

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml sprite_sheet_transparent
npm --workspace apps/mac run build
npm run test:scripts
npm --workspace apps/mac run smoke:ui:mvp
```

Expected:

```text
all pass
```

- [ ] **Step 4: Commit**

Run:

```bash
git add packages/core/src/video/sprite_sheet.rs packages/core/src/video/mod.rs apps/mac/src-tauri/src/lib.rs apps/mac/src/tauriCommands.ts apps/mac/src/components/ImportPanel.tsx apps/mac/src/routes/ForgeRoute.tsx apps/mac/src/i18n.ts apps/mac/src/styles/app.css scripts/test-import-panel-source.mjs docs/qa/transparent-gutter-sprite-split-evidence-2026-06-11.md
git commit -m "feat: add transparent sprite sheet split import"
```

Expected:

```text
commit created
```

## Follow-Up Plan Seeds

After this plan lands and is dogfooded, write separate plans for:

```text
Frame repair workbench: frame nudge, crop/expand, delete, duplicate, reset
GIF intake: import animated GIF as PNG sequence through local job directories
Format presets: named RPG Maker and Godot import/export settings
```
