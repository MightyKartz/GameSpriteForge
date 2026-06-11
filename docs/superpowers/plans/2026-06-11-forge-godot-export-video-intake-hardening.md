# Forge Godot Export And Video Intake Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Forge exports directly usable in Godot and make video frame extraction and character-position correctness visible before users export.

**Architecture:** Keep `.gsfpack` as the portable pack format, then add a local Godot export layer that converts an existing pack into Godot-native `SpriteFrames` and a smoke scene. Keep video extraction decisions in the existing Tauri/React workflow, but move frame-count math into tested helpers and expose bbox/anchor overlays in the preview UI.

**Tech Stack:** Rust `forge_core` / `forge_pack`, Tauri 2 commands, React 18, TypeScript, Godot 4.6 headless smoke testing, existing Forge source-guard scripts.

---

## Implementation Status - 2026-06-11

Status: implemented and verified.

Important for future agents: the detailed steps below are the original
task-by-task implementation instructions. Do not overwrite the current
generalized files with the older hardcoded smoke snippets. The current
implementation intentionally preserves these differences from the original
draft:

```text
scripts/godot/import_forge_pack.gd derives resource names from forgepack.json.
scripts/godot/import_forge_pack.gd supports multi-sheet texture pages through atlas frame image fields.
packages/core/examples/generate_godot_smoke_pack.rs only cleans known smoke children under godot-pack-smoke-*.
apps/mac/src-tauri/src/lib.rs validates .gsfpack layout before writing a Godot project.
CanvasPreview overlays are anchored to preview-frame-stage, not the whole canvas container.
ForgeRoute uses activeKeepEveryNFrames derived from the target frame count for extraction.
```

Multi-agent implementation record:

```text
Skills: forge-dev, superpowers:subagent-driven-development, superpowers:dispatching-parallel-agents
MCP: tool_search discovered multi_agent_v1
Agents: Turing reviewed Godot export/smoke risks; Cicero reviewed video segment and overlay boundaries; Dewey reviewed plan consistency and dirty worktree risks.
QA: docs/qa/godot-export-video-intake-hardening-2026-06-11.md
```

Verified commands:

```bash
npm --workspace apps/mac run build
npm run test:scripts
cargo fmt --manifest-path /Users/kartz/Development/Forge/Cargo.toml --all -- --check
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml -p core
cargo check --manifest-path /Users/kartz/Development/Forge/apps/mac/src-tauri/Cargo.toml
npm run smoke:godot
```

## Scope

Implement these user-visible improvements:

```text
one-click Godot export folder from a completed Forge pack
repeatable Godot headless smoke test
target frame-count controls for video extraction
preview overlays for bbox, foot anchor, and character center
export result copy that reports pack validation and Godot readiness
```

Do not implement these in this slice:

```text
cloud export
online registry
Godot asset-library publishing
AI generation
Unity/Phaser/Cocos/GameMaker exporters
```

## File Structure

- Create: `packages/core/src/export/godot.rs` - writes a Godot import project/folder from a `.gsfpack`.
- Modify: `packages/core/src/export/mod.rs` - exports Godot export types and calls.
- Create: `packages/core/examples/generate_godot_smoke_pack.rs` - creates the deterministic smoke pack from the manual QA sprite sheet.
- Create: `scripts/godot/import_forge_pack.gd` - Godot 4 script that converts Forge metadata into `SpriteFrames` and a scene.
- Create: `scripts/run-godot-pack-smoke.sh` - generates the pack and runs Godot headless.
- Modify: `package.json` - adds `smoke:godot` and includes source guards in `test:scripts`.
- Create: `scripts/test-godot-export-source.mjs` - guards that Godot export files and commands remain wired.
- Modify: `apps/mac/src-tauri/src/lib.rs` - exposes `export_godot_project`.
- Modify: `apps/mac/src/tauriCommands.ts` - adds TypeScript types and wrapper.
- Modify: `apps/mac/src/components/ExportPanel.tsx` - adds Godot export action and status.
- Modify: `apps/mac/src/routes/ForgeRoute.tsx` - stores Godot export result and calls the command.
- Modify: `apps/mac/src/components/VideoSegmentPanel.tsx` - adds target frame count controls and clearer extraction reasoning.
- Create: `apps/mac/src/videoSegment.ts` - pure helpers for frame-count estimation and interval derivation.
- Modify: `apps/mac/src/components/CanvasPreview.tsx` - renders bbox, anchor, and center overlays.
- Modify: `apps/mac/src/i18n.ts` - English and Simplified Chinese labels.
- Modify: `apps/mac/src/styles/app.css` - overlay and Godot export UI styles.
- Create: `docs/qa/godot-export-video-intake-hardening-2026-06-11.md` - implementation evidence.

## Task 1: Productize The Godot Import Script

**Files:**
- Create: `scripts/godot/import_forge_pack.gd`
- Create: `scripts/test-godot-export-source.mjs`
- Modify: `package.json`

- [ ] **Step 1: Create the Godot import script file**

Create `scripts/godot/import_forge_pack.gd` by moving the proven script from:

```text
docs/qa/artifacts/godot-pack-smoke-2026-06-11/godot-project/import_forge_pack.gd
```

The committed script must keep these observable behaviors:

```gdscript
extends SceneTree

func _initialize() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() < 1:
		_fail("Expected a .gsfpack directory path.")

	var pack_path := String(args[0]).simplify_path()
	var import_root := "res://imported/GodotSmokeWalk"
	var texture_res_path := import_root.path_join("sprite_sheet.png")
	var sprite_frames_res_path := import_root.path_join("GodotSmokeWalk.spriteframes.tres")
	var scene_res_path := "res://GodotSmokeWalkSmoke.tscn"
```

Keep the later assertions that:

```gdscript
loaded_sprite.play(first_animation_name)
if !loaded_sprite.is_playing():
	_fail("Loaded AnimatedSprite2D could not play the imported animation.")
```

- [ ] **Step 2: Add source guard coverage**

Create `scripts/test-godot-export-source.mjs`:

```js
import { readFileSync } from "node:fs";

const script = readFileSync("scripts/godot/import_forge_pack.gd", "utf8");
const packageJson = readFileSync("package.json", "utf8");

function assertContains(source, needle, message) {
  if (!source.includes(needle)) {
    throw new Error(message);
  }
}

assertContains(script, "SpriteFrames.new()", "Godot importer must create SpriteFrames.");
assertContains(script, "AtlasTexture.new()", "Godot importer must create AtlasTexture frames.");
assertContains(script, "AnimatedSprite2D.new()", "Godot importer must create AnimatedSprite2D.");
assertContains(script, "loaded_sprite.play", "Godot importer must verify the loaded animation can play.");
assertContains(packageJson, "smoke:godot", "package.json must expose the Godot smoke command.");

console.log("PASS Godot export source test");
```

- [ ] **Step 3: Wire the guard into `package.json`**

Add `node --check scripts/test-godot-export-source.mjs` and
`node scripts/test-godot-export-source.mjs` to the existing `test:scripts`
chain next to the other source guards.

- [ ] **Step 4: Run source checks**

Run:

```bash
npm run test:scripts
```

Expected:

```text
PASS Godot export source test
```

- [ ] **Step 5: Commit**

Run:

```bash
git add scripts/godot/import_forge_pack.gd scripts/test-godot-export-source.mjs package.json
git commit -m "test: guard Godot import script"
```

Expected:

```text
commit created
```

## Task 2: Add A Core Godot Export Writer

**Files:**
- Create: `packages/core/src/export/godot.rs`
- Modify: `packages/core/src/export/mod.rs`

- [ ] **Step 1: Write the failing Rust test**

Create `packages/core/src/export/godot.rs` with this test first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn godot_project_export_writes_project_script_and_readme() {
        let temp = tempfile::tempdir().unwrap();
        let pack_dir = temp.path().join("Godot-Smoke-Walk.gsfpack");
        fs::create_dir_all(pack_dir.join("assets")).unwrap();
        fs::write(pack_dir.join("forgepack.json"), r#"{"name":"Godot Smoke Walk"}"#).unwrap();

        let output = export_godot_project(GodotProjectExportParams {
            pack_dir: pack_dir.clone(),
            output_dir: temp.path().join("godot-export"),
            project_name: "Godot Smoke Walk".to_string(),
        })
        .unwrap();

        assert!(output.project_dir.join("project.godot").is_file());
        assert!(output.project_dir.join("addons/game_sprite_forge/import_forge_pack.gd").is_file());
        assert!(output.project_dir.join("README.md").is_file());

        let readme = fs::read_to_string(output.project_dir.join("README.md")).unwrap();
        assert!(readme.contains("Godot Smoke Walk"));
        assert!(readme.contains("Godot-Smoke-Walk.gsfpack"));
    }
}
```

- [ ] **Step 2: Run the focused test to verify it fails**

Run:

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml -p core godot_project_export_writes_project_script_and_readme
```

Expected:

```text
cannot find function `export_godot_project`
```

- [ ] **Step 3: Implement the core writer**

Add the implementation above the test in `packages/core/src/export/godot.rs`:

```rust
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::ExportError;

const GODOT_IMPORT_SCRIPT: &str = include_str!("../../../../scripts/godot/import_forge_pack.gd");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GodotProjectExportParams {
    pub pack_dir: PathBuf,
    pub output_dir: PathBuf,
    pub project_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GodotProjectExportOutput {
    pub project_dir: PathBuf,
    pub import_script_path: PathBuf,
    pub project_file_path: PathBuf,
    pub readme_path: PathBuf,
}

pub fn export_godot_project(
    params: GodotProjectExportParams,
) -> Result<GodotProjectExportOutput, ExportError> {
    let project_slug = sanitize_project_name(&params.project_name);
    let project_dir = params.output_dir.join(project_slug);
    let addon_dir = project_dir.join("addons/game_sprite_forge");
    fs::create_dir_all(&addon_dir)?;

    let project_file_path = project_dir.join("project.godot");
    let import_script_path = addon_dir.join("import_forge_pack.gd");
    let readme_path = project_dir.join("README.md");

    fs::write(&project_file_path, project_file(&params.project_name))?;
    fs::write(&import_script_path, GODOT_IMPORT_SCRIPT)?;
    fs::write(&readme_path, readme(&params.project_name, &params.pack_dir))?;

    Ok(GodotProjectExportOutput {
        project_dir,
        import_script_path,
        project_file_path,
        readme_path,
    })
}

fn project_file(project_name: &str) -> String {
    format!(
        "; Engine configuration file.\n\nconfig_version=5\n\n[application]\n\nconfig/name=\"{}\"\nconfig/features=PackedStringArray(\"4.6\")\n",
        project_name.replace('"', "'")
    )
}

fn readme(project_name: &str, pack_dir: &std::path::Path) -> String {
    format!(
        "# {project_name}\n\nRun this command from the project folder:\n\n```bash\n/Applications/Godot.app/Contents/MacOS/Godot --headless --path . --script addons/game_sprite_forge/import_forge_pack.gd -- {}\n```\n",
        pack_dir.display()
    )
}

fn sanitize_project_name(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "Godot-Export".to_string()
    } else {
        trimmed.to_string()
    }
}
```

- [ ] **Step 4: Export the module**

Modify `packages/core/src/export/mod.rs`:

```rust
pub mod godot;
```

Add public exports next to the existing `pub use` lines:

```rust
pub use godot::{export_godot_project, GodotProjectExportOutput, GodotProjectExportParams};
```

- [ ] **Step 5: Run tests**

Run:

```bash
cargo fmt --manifest-path /Users/kartz/Development/Forge/Cargo.toml --all
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml -p core godot_project_export
```

Expected:

```text
godot_project_export_writes_project_script_and_readme ... ok
```

- [ ] **Step 6: Commit**

Run:

```bash
git add packages/core/src/export/godot.rs packages/core/src/export/mod.rs
git commit -m "feat: add Godot project export writer"
```

Expected:

```text
commit created
```

## Task 3: Add Godot Export To Tauri And React

**Files:**
- Modify: `apps/mac/src-tauri/src/lib.rs`
- Modify: `apps/mac/src/tauriCommands.ts`
- Modify: `apps/mac/src/components/ExportPanel.tsx`
- Modify: `apps/mac/src/routes/ForgeRoute.tsx`
- Modify: `apps/mac/src/i18n.ts`
- Modify: `apps/mac/src/styles/app.css`
- Modify: `scripts/test-godot-export-source.mjs`

- [ ] **Step 1: Add Tauri command**

In `apps/mac/src-tauri/src/lib.rs`, add:

```rust
#[tauri::command]
fn export_godot_project(
    params: forge_core::export::GodotProjectExportParams,
) -> Result<forge_core::export::GodotProjectExportOutput, String> {
    forge_core::export::export_godot_project(params).map_err(|error| error.to_string())
}
```

Add `export_godot_project` to the `tauri::generate_handler![]` list.

- [ ] **Step 2: Add TypeScript wrapper**

In `apps/mac/src/tauriCommands.ts`, add:

```ts
export type GodotProjectExportOutput = {
  projectDir: string;
  importScriptPath: string;
  projectFilePath: string;
  readmePath: string;
};

export function exportGodotProject(args: {
  packDir: string;
  outputDir: string;
  projectName: string;
}) {
  return invoke<GodotProjectExportOutput>("export_godot_project", {
    params: {
      packDir: args.packDir,
      outputDir: args.outputDir,
      projectName: args.projectName,
    },
  });
}
```

- [ ] **Step 3: Wire route state and handler**

In `apps/mac/src/routes/ForgeRoute.tsx`, import the new wrapper:

```ts
import { exportGodotProject, type GodotProjectExportOutput } from "../tauriCommands";
```

Add state near `exportOutput`:

```ts
const [godotExportOutput, setGodotExportOutput] = useState<GodotProjectExportOutput | null>(null);
```

Add handler:

```ts
async function handleExportGodotProject() {
  if (!exportOutput) {
    return;
  }
  await runStep(t("status.exportingGodotProject"), async () => {
    const output = await exportGodotProject({
      packDir: exportOutput.packDir,
      outputDir: exportOutput.exportDir,
      projectName: packName || "Game Sprite Forge Export",
    });
    setGodotExportOutput(output);
    return t("status.exportedGodotProject");
  });
}
```

Pass `godotExportOutput` and `onExportGodotProject={handleExportGodotProject}` to `ExportPanel`.

- [ ] **Step 4: Add ExportPanel action**

In `apps/mac/src/components/ExportPanel.tsx`, extend props:

```ts
godotExportOutput: GodotProjectExportOutput | null;
onExportGodotProject: () => void;
```

Add a button beside validate/re-export after `exportOutput` exists:

```tsx
<button className="secondary-button preview-export" disabled={disabled || !exportOutput} onClick={onExportGodotProject} type="button">
  <PackageCheck size={17} />
  {t("export.exportGodotProject")}
</button>
```

Add a status card when `godotExportOutput` exists:

```tsx
{godotExportOutput ? (
  <div className="godot-export-card" role="status">
    <strong>{t("export.godotReady")}</strong>
    <code>{godotExportOutput.projectDir}</code>
  </div>
) : null}
```

- [ ] **Step 5: Add i18n keys**

In `apps/mac/src/i18n.ts`, add English:

```ts
"export.exportGodotProject": "Export Godot Project",
"export.godotReady": "Godot project export ready",
"status.exportingGodotProject": "Writing Godot project export...",
"status.exportedGodotProject": "Godot project export is ready.",
```

Add Simplified Chinese:

```ts
"export.exportGodotProject": "导出 Godot 项目",
"export.godotReady": "Godot 项目导出已就绪",
"status.exportingGodotProject": "正在写入 Godot 项目导出...",
"status.exportedGodotProject": "Godot 项目导出已就绪。",
```

- [ ] **Step 6: Extend source guard**

In `scripts/test-godot-export-source.mjs`, add reads and assertions:

```js
const tauriSource = readFileSync("apps/mac/src-tauri/src/lib.rs", "utf8");
const commandsSource = readFileSync("apps/mac/src/tauriCommands.ts", "utf8");
const exportPanelSource = readFileSync("apps/mac/src/components/ExportPanel.tsx", "utf8");
const forgeRouteSource = readFileSync("apps/mac/src/routes/ForgeRoute.tsx", "utf8");

assertContains(tauriSource, "export_godot_project", "Tauri must expose export_godot_project.");
assertContains(commandsSource, "exportGodotProject", "TypeScript must wrap Godot project export.");
assertContains(exportPanelSource, "export.exportGodotProject", "ExportPanel must show the Godot export action.");
assertContains(forgeRouteSource, "handleExportGodotProject", "ForgeRoute must wire the Godot export handler.");
```

- [ ] **Step 7: Run verification**

Run:

```bash
npm --workspace apps/mac run build
npm run test:scripts
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml -p game-sprite-forge-mac export_godot_project
```

Expected:

```text
frontend build: pass
source guards: pass
Tauri command tests: pass or no focused tests matched
```

- [ ] **Step 8: Commit**

Run:

```bash
git add apps/mac/src-tauri/src/lib.rs apps/mac/src/tauriCommands.ts apps/mac/src/components/ExportPanel.tsx apps/mac/src/routes/ForgeRoute.tsx apps/mac/src/i18n.ts apps/mac/src/styles/app.css scripts/test-godot-export-source.mjs
git commit -m "feat: expose Godot project export"
```

Expected:

```text
commit created
```

## Task 4: Formalize The Godot Smoke Command

**Files:**
- Create: `packages/core/examples/generate_godot_smoke_pack.rs`
- Create: `scripts/run-godot-pack-smoke.sh`
- Modify: `package.json`
- Create: `docs/qa/godot-export-video-intake-hardening-2026-06-11.md`

- [ ] **Step 1: Create the core example**

Create `packages/core/examples/generate_godot_smoke_pack.rs` from the proven temporary generator used for:

```text
docs/qa/artifacts/godot-pack-smoke-2026-06-11/generated-pack-summary.json
```

The example must print these lines:

```rust
println!("PACK_DIR={}", output.pack_dir.display());
println!("FRAME_COUNT={}", summary.frame_count);
println!("SPRITE_SHEET={}", output.sprite_sheet_path.display());
println!("GODOT_HELPER={}", output.godot_helper_path.display());
```

- [ ] **Step 2: Create the smoke shell script**

Create `scripts/run-godot-pack-smoke.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

ROOT="/Users/kartz/Development/Forge"
ARTIFACT_ROOT="$ROOT/docs/qa/artifacts/godot-pack-smoke-$(date +%Y%m%d-%H%M%S)"
GODOT="${GODOT_BIN:-/Applications/Godot.app/Contents/MacOS/Godot}"

mkdir -p "$ARTIFACT_ROOT"
cargo run --manifest-path "$ROOT/Cargo.toml" -p core --example generate_godot_smoke_pack -- "$ARTIFACT_ROOT" | tee "$ARTIFACT_ROOT/generate.log"

PACK_DIR="$(awk -F= '/^PACK_DIR=/{print $2}' "$ARTIFACT_ROOT/generate.log")"
PROJECT_DIR="$ARTIFACT_ROOT/godot-project"
mkdir -p "$PROJECT_DIR/addons/game_sprite_forge"
cp "$ROOT/scripts/godot/import_forge_pack.gd" "$PROJECT_DIR/addons/game_sprite_forge/import_forge_pack.gd"
cat > "$PROJECT_DIR/project.godot" <<'PROJECT'
config_version=5

[application]

config/name="Forge Godot Smoke"
config/features=PackedStringArray("4.6")
PROJECT

"$GODOT" --headless --path "$PROJECT_DIR" --script "$PROJECT_DIR/addons/game_sprite_forge/import_forge_pack.gd" -- "$PACK_DIR" | tee "$ARTIFACT_ROOT/godot.log"
grep -q "PASS Forge Godot import smoke" "$ARTIFACT_ROOT/godot.log"
echo "PASS Godot pack smoke: $ARTIFACT_ROOT"
```

- [ ] **Step 3: Make the script executable and add npm script**

Run:

```bash
chmod +x scripts/run-godot-pack-smoke.sh
```

Add to `package.json`:

```json
"smoke:godot": "scripts/run-godot-pack-smoke.sh"
```

- [ ] **Step 4: Run the smoke**

Run:

```bash
npm run smoke:godot
```

Expected:

```text
PASS Forge Godot import smoke
PASS Godot pack smoke
```

- [ ] **Step 5: Record QA evidence**

Create `docs/qa/godot-export-video-intake-hardening-2026-06-11.md`:

````markdown
# Godot Export And Video Intake Hardening QA

Date: 2026-06-11

## Commands

```bash
npm run smoke:godot
npm --workspace apps/mac run build
npm run test:scripts
```

## Result

```text
Godot smoke: pass
frontend build: pass
source guards: pass
```

## Notes

The smoke creates a real Forge `.gsfpack`, imports it into a clean Godot project,
creates `SpriteFrames`, saves a scene, loads the scene back, and calls `play()`.
```
````

- [ ] **Step 6: Commit**

Run:

```bash
git add packages/core/examples/generate_godot_smoke_pack.rs scripts/run-godot-pack-smoke.sh package.json docs/qa/godot-export-video-intake-hardening-2026-06-11.md
git commit -m "test: add Godot pack smoke command"
```

Expected:

```text
commit created
```

## Task 5: Make Video Frame Selection Explicit

**Files:**
- Create: `apps/mac/src/videoSegment.ts`
- Modify: `apps/mac/src/components/VideoSegmentPanel.tsx`
- Modify: `apps/mac/src/routes/ForgeRoute.tsx`
- Modify: `apps/mac/src/i18n.ts`
- Create: `scripts/test-video-segment-source.mjs`

- [ ] **Step 1: Add pure frame-count helpers**

Create `apps/mac/src/videoSegment.ts`:

```ts
export type FrameSelectionMode = "target_frames" | "target_fps" | "manual_interval";

export function estimateSelectedFrames(selectedDuration: number, fps: number, keepEveryNFrames: number, fallback: number) {
  if (fps > 0 && selectedDuration > 0) {
    return Math.max(1, Math.round((selectedDuration * fps) / Math.max(1, keepEveryNFrames)));
  }
  return Math.max(0, Math.round(fallback));
}

export function keepEveryForTargetFrames(selectedDuration: number, fps: number, targetFrameCount: number) {
  if (selectedDuration <= 0 || fps <= 0 || targetFrameCount <= 0) {
    return 1;
  }
  return Math.max(1, Math.round((selectedDuration * fps) / targetFrameCount));
}

export function targetFpsForInterval(fps: number, keepEveryNFrames: number) {
  if (fps <= 0) {
    return 0;
  }
  return fps / Math.max(1, keepEveryNFrames);
}
```

- [ ] **Step 2: Update VideoSegmentPanel props**

Add props to `VideoSegmentPanelProps`:

```ts
frameSelectionMode: FrameSelectionMode;
targetFrameCount: number;
onFrameSelectionModeChange: (value: FrameSelectionMode) => void;
onTargetFrameCountChange: (value: number) => void;
```

Import helpers:

```ts
import { estimateSelectedFrames, keepEveryForTargetFrames, type FrameSelectionMode } from "../videoSegment";
```

- [ ] **Step 3: Add 12/24/custom target controls**

Add controls before the interval field:

```tsx
<div className="segment-target-controls" role="group" aria-label={t("segment.targetFrames")}>
  {[12, 24].map((count) => (
    <button
      className={frameSelectionMode === "target_frames" && targetFrameCount === count ? "active" : ""}
      disabled={disabled}
      key={count}
      onClick={() => {
        onFrameSelectionModeChange("target_frames");
        onTargetFrameCountChange(count);
        onKeepEveryChange(keepEveryForTargetFrames(selectedDuration, fps, count));
      }}
      type="button"
    >
      {t("segment.targetFramePreset", { count })}
    </button>
  ))}
  <button
    className={frameSelectionMode === "manual_interval" ? "active" : ""}
    disabled={disabled}
    onClick={() => onFrameSelectionModeChange("manual_interval")}
    type="button"
  >
    {t("segment.manualInterval")}
  </button>
</div>
```

- [ ] **Step 4: Add route state**

In `ForgeRoute.tsx`, add:

```ts
const [frameSelectionMode, setFrameSelectionMode] = useState<FrameSelectionMode>("target_frames");
const [targetFrameCount, setTargetFrameCount] = useState(24);
```

Pass the state and setters to `VideoSegmentPanel`.

- [ ] **Step 5: Add copy**

In `apps/mac/src/i18n.ts`, add English:

```ts
"segment.targetFrames": "Target frames",
"segment.targetFramePreset": "{count} frames",
"segment.manualInterval": "Manual interval",
"segment.extractionRationale": "{count} frames is an estimate for this selected segment, not a quality guarantee.",
```

Add Simplified Chinese:

```ts
"segment.targetFrames": "目标帧数",
"segment.targetFramePreset": "{count} 帧",
"segment.manualInterval": "手动间隔",
"segment.extractionRationale": "{count} 帧是当前片段的预计抽取结果，不代表质量自动正确。",
```

- [ ] **Step 6: Add source guard**

Create `scripts/test-video-segment-source.mjs`:

```js
import { readFileSync } from "node:fs";

const panel = readFileSync("apps/mac/src/components/VideoSegmentPanel.tsx", "utf8");
const route = readFileSync("apps/mac/src/routes/ForgeRoute.tsx", "utf8");
const helper = readFileSync("apps/mac/src/videoSegment.ts", "utf8");
const i18n = readFileSync("apps/mac/src/i18n.ts", "utf8");

for (const [source, needle, message] of [
  [helper, "keepEveryForTargetFrames", "videoSegment helper must derive interval from target frame count."],
  [panel, "segment.targetFrames", "VideoSegmentPanel must expose target frame controls."],
  [route, "targetFrameCount", "ForgeRoute must store target frame count."],
  [i18n, "segment.extractionRationale", "i18n must explain frame count is an estimate."],
]) {
  if (!source.includes(needle)) {
    throw new Error(message);
  }
}

console.log("PASS video segment source test");
```

Add it to `package.json` `test:scripts`.

- [ ] **Step 7: Run verification**

Run:

```bash
npm --workspace apps/mac run build
npm run test:scripts
```

Expected:

```text
frontend build: pass
PASS video segment source test
```

- [ ] **Step 8: Commit**

Run:

```bash
git add apps/mac/src/videoSegment.ts apps/mac/src/components/VideoSegmentPanel.tsx apps/mac/src/routes/ForgeRoute.tsx apps/mac/src/i18n.ts scripts/test-video-segment-source.mjs package.json
git commit -m "feat: clarify video frame selection"
```

Expected:

```text
commit created
```

## Task 6: Add Position And Anchor Overlays

**Files:**
- Modify: `apps/mac/src/components/CanvasPreview.tsx`
- Modify: `apps/mac/src/routes/ForgeRoute.tsx`
- Modify: `apps/mac/src/styles/app.css`
- Modify: `apps/mac/src/i18n.ts`
- Create: `scripts/test-preview-overlay-source.mjs`

- [ ] **Step 1: Extend CanvasPreview props**

In `CanvasPreview.tsx`, import types:

```ts
import type { FootAnchor, FrameBbox, FrameSize } from "../tauriCommands";
```

Extend props:

```ts
overlay?: {
  anchor: FootAnchor;
  bbox: FrameBbox;
  size: FrameSize;
} | null;
```

- [ ] **Step 2: Compute overlay styles**

Add this helper:

```ts
function overlayPercent(overlay: { anchor: FootAnchor; bbox: FrameBbox; size: FrameSize }) {
  const width = Math.max(1, overlay.size.width);
  const height = Math.max(1, overlay.size.height);
  return {
    anchorY: `${(overlay.anchor.y / height) * 100}%`,
    bboxHeight: `${(overlay.bbox.height / height) * 100}%`,
    bboxLeft: `${(overlay.bbox.left / width) * 100}%`,
    bboxTop: `${(overlay.bbox.top / height) * 100}%`,
    bboxWidth: `${(overlay.bbox.width / width) * 100}%`,
    centerX: `${(overlay.bbox.centerX / width) * 100}%`,
  };
}
```

Render overlays above `frame-tag`:

```tsx
{overlay && imageSrc ? (
  <div className="preview-measure-overlays" aria-hidden="true">
    <span className="preview-bbox" style={{
      height: overlayPercent(overlay).bboxHeight,
      left: overlayPercent(overlay).bboxLeft,
      top: overlayPercent(overlay).bboxTop,
      width: overlayPercent(overlay).bboxWidth,
    }} />
    <span className="preview-anchor-line" style={{ top: overlayPercent(overlay).anchorY }} />
    <span className="preview-center-line" style={{ left: overlayPercent(overlay).centerX }} />
  </div>
) : null}
```

- [ ] **Step 3: Pass selected normalized frame overlay**

In `ForgeRoute.tsx`, compute:

```ts
const selectedOverlay = normalizeResult?.frames[selectedFrameIndex]
  ? {
      anchor: normalizeResult.frames[selectedFrameIndex].anchor,
      bbox: normalizeResult.frames[selectedFrameIndex].bbox,
      size: normalizeResult.frames[selectedFrameIndex].size,
    }
  : null;
```

Pass `overlay={selectedOverlay}` to `CanvasPreview`.

- [ ] **Step 4: Add CSS**

In `apps/mac/src/styles/app.css`, add:

```css
.preview-measure-overlays {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.preview-bbox {
  position: absolute;
  border: 1px solid rgba(80, 220, 150, 0.9);
  box-shadow: 0 0 0 1px rgba(5, 20, 20, 0.45);
}

.preview-anchor-line,
.preview-center-line {
  position: absolute;
  background: rgba(255, 220, 90, 0.85);
}

.preview-anchor-line {
  left: 0;
  right: 0;
  height: 1px;
}

.preview-center-line {
  bottom: 0;
  top: 0;
  width: 1px;
}
```

- [ ] **Step 5: Add source guard**

Create `scripts/test-preview-overlay-source.mjs`:

```js
import { readFileSync } from "node:fs";

const canvas = readFileSync("apps/mac/src/components/CanvasPreview.tsx", "utf8");
const route = readFileSync("apps/mac/src/routes/ForgeRoute.tsx", "utf8");
const styles = readFileSync("apps/mac/src/styles/app.css", "utf8");

for (const [source, needle, message] of [
  [canvas, "preview-measure-overlays", "CanvasPreview must render measurement overlays."],
  [canvas, "preview-bbox", "CanvasPreview must render bbox overlay."],
  [route, "selectedOverlay", "ForgeRoute must pass selected normalized overlay data."],
  [styles, ".preview-anchor-line", "CSS must style anchor line."],
]) {
  if (!source.includes(needle)) {
    throw new Error(message);
  }
}

console.log("PASS preview overlay source test");
```

Add it to `package.json` `test:scripts`.

- [ ] **Step 6: Run real UI verification**

Run:

```bash
npm --workspace apps/mac run build
npm run test:scripts
npm --workspace apps/mac run smoke:ui:mvp
```

Expected:

```text
frontend build: pass
source guards: pass
MVP UI smoke: pass
```

Record screenshot evidence under:

```text
docs/qa/artifacts/
```

- [ ] **Step 7: Commit**

Run:

```bash
git add apps/mac/src/components/CanvasPreview.tsx apps/mac/src/routes/ForgeRoute.tsx apps/mac/src/styles/app.css scripts/test-preview-overlay-source.mjs package.json docs/qa/artifacts
git commit -m "feat: show frame position overlays"
```

Expected:

```text
commit created
```

## Task 7: Final Verification And Documentation

**Files:**
- Modify: `docs/qa/godot-export-video-intake-hardening-2026-06-11.md`
- Modify: `docs/architecture/post-release-backlog.md`
- Modify: `README.md`
- Modify: `README.zh-CN.md`

- [ ] **Step 1: Update backlog**

In `docs/architecture/post-release-backlog.md`, add a completed or active slice:

````markdown
### Slice 7: Godot Export And Video Intake Hardening

Goal: make exported Forge packs directly usable in Godot and make frame-count
and position correctness visible in the local app.

Evidence:

```text
docs/architecture/godot-export-video-intake-hardening.md
docs/superpowers/plans/2026-06-11-forge-godot-export-video-intake-hardening.md
docs/qa/godot-export-video-intake-hardening-2026-06-11.md
```
````

- [ ] **Step 2: Update README feature list**

In `README.md`, add:

```markdown
- Godot project export smoke-tested with native `SpriteFrames` and `AnimatedSprite2D`.
- Video extraction controls show the selected range, estimated frame count, and extraction interval.
```

In `README.zh-CN.md`, add:

```markdown
- Godot 项目导出经过 `SpriteFrames` 和 `AnimatedSprite2D` 原生资源验证。
- 视频抽帧控制会显示选中片段、预计帧数和抽帧间隔。
```

- [ ] **Step 3: Run full verification**

Run:

```bash
npm --workspace apps/mac run build
npm run test:scripts
npm --workspace apps/mac run smoke:ui:mvp
npm run smoke:godot
cargo fmt --manifest-path /Users/kartz/Development/Forge/Cargo.toml --all -- --check
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml
```

Expected:

```text
frontend build: pass
source guards: pass
MVP UI smoke: pass
Godot smoke: pass
Rust format: pass
Rust tests: pass
```

- [ ] **Step 4: Record final QA**

Append the final command output summary to:

```text
docs/qa/godot-export-video-intake-hardening-2026-06-11.md
```

Use this result block:

````markdown
## Final Verification

```text
frontend build: pass
source guards: pass
MVP UI smoke: pass
Godot smoke: pass
Rust format: pass
Rust tests: pass
```
````

- [ ] **Step 5: Commit**

Run:

```bash
git add README.md README.zh-CN.md docs/architecture/post-release-backlog.md docs/qa/godot-export-video-intake-hardening-2026-06-11.md
git commit -m "docs: record Godot export hardening evidence"
```

Expected:

```text
commit created
```

## Self-Review

Spec coverage:

```text
Godot one-click usability: Tasks 1-4
Video frame extraction correctness: Task 5
Character position correctness: Task 6
QA evidence and documentation: Task 7
```

Placeholder scan:

```text
No placeholder markers.
No unfinished implementation notes.
All tasks include concrete files, commands, and expected results.
```

Type consistency:

```text
Rust command: export_godot_project
TypeScript wrapper: exportGodotProject
Output type: GodotProjectExportOutput
Video helper: keepEveryForTargetFrames
Preview route variable: selectedOverlay
```
