# Forge CLI MCP Feasibility Spike Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` to execute this plan. Run Task 1 and Task 5 with real UI or Computer Use verification where specified.

**Goal:** Add a small, local-only Forge automation surface that proves CLI feasibility and defines the MCP contract without changing the current import/export user experience.

**Architecture:** Keep the Tauri app as the primary product surface. Move reusable pack-library scanning out of the Tauri command layer into `packages/pack`, add a new `packages/cli` binary that calls `packages/core` and `packages/pack`, and document MCP as a thin local adapter over the CLI contract. Do not build a product MCP server in this spike.

**Tech Stack:** Rust workspace, `forge_core`, `forge_pack`, Tauri command wrappers, `clap`, `serde_json`, existing npm script guards, macOS real UI verification.

---

## Current Code Truth

The current Tauri command layer in `apps/mac/src-tauri/src/lib.rs` exposes the working local workflow: toolchain check, sample path resolution, job creation, source import, video probing, frame extraction, sprite-sheet slicing, chroma preview, chroma processing, frame normalization, quality report, pack export, `.gsfpack` import, re-import validation, local pack listing, and pack inspection.

`apps/mac/src/tauriCommands.ts` owns UI-facing defaults and workflow parameter shaping, including chroma defaults, export metadata defaults, and current source assumptions. The CLI must not copy these defaults silently as product behavior.

`packages/core` already contains reusable processing modules for video, matting, preview, frames, quality, export, and job storage. `packages/pack` already contains pack import, validation, summary, and inspection logic. The missing bridge is a small shared local-pack listing function and a CLI command contract.

The first CLI slice should be read-mostly:

- `toolchain --json`
- `inspect-pack --path <pack.gsfpack> --json`
- `validate-pack --path <pack.gsfpack> --json`
- `list-packs --exports-dir <dir> --json`

End-to-end video import/export automation is deferred until the write confirmation rules are verified.

---

## Scope Guard

This plan does not add cloud sync, model generation, a public MCP server, or unattended write workflows.

This plan does not replace the Tauri app. The CLI exists to make local automation testable and to prepare a clear MCP contract.

This plan does not bundle FFmpeg. The CLI uses the same configured/PATH resolver behavior already evaluated for the app.

---

## Task 1: Add CLI/MCP Source Guard

**Purpose:** Make future CLI/MCP work hard to accidentally drift into a product MCP server or duplicated UI orchestration.

**Files:**

- `scripts/test-cli-mcp-feasibility-source.mjs`
- `package.json`

**Steps:**

1. Create `scripts/test-cli-mcp-feasibility-source.mjs`.

```js
#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const requiredFiles = [
  "docs/superpowers/plans/2026-06-06-forge-cli-mcp-feasibility-spike.md",
  "docs/architecture/cli-mcp-feasibility.md",
  "packages/cli/Cargo.toml",
  "packages/cli/src/main.rs",
];

for (const relative of requiredFiles) {
  const absolute = path.join(root, relative);
  if (!fs.existsSync(absolute)) {
    throw new Error(`Missing required CLI/MCP feasibility artifact: ${relative}`);
  }
}

const plan = fs.readFileSync(
  path.join(root, "docs/superpowers/plans/2026-06-06-forge-cli-mcp-feasibility-spike.md"),
  "utf8",
);

const architecture = fs.readFileSync(
  path.join(root, "docs/architecture/cli-mcp-feasibility.md"),
  "utf8",
);

const combinedDocs = `${plan}\n${architecture}`;

for (const phrase of [
  "Do not build a product MCP server in this spike.",
  "write confirmation",
  "local-only",
  "toolchain --json",
  "inspect-pack --path",
  "validate-pack --path",
  "list-packs --exports-dir",
]) {
  if (!combinedDocs.includes(phrase)) {
    throw new Error(`CLI/MCP docs must include: ${phrase}`);
  }
}

const packageJson = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8"));
const scriptsTest = packageJson.scripts?.["test:scripts"] ?? "";
if (!scriptsTest.includes("test-cli-mcp-feasibility-source.mjs")) {
  throw new Error("package.json test:scripts must run test-cli-mcp-feasibility-source.mjs");
}

console.log("CLI/MCP feasibility source guard passed.");
```

2. Add the source guard to `package.json` in `scripts["test:scripts"]`.

3. Run the guard once after Task 4, because Task 1 intentionally references files created by later tasks.

**Validation:**

```bash
npm run test:scripts
```

---

## Task 2: Move Local Pack Listing Into `packages/pack`

**Purpose:** Let the app and CLI share one local pack discovery implementation.

**Files:**

- `packages/pack/src/lib.rs`
- `packages/pack/tests/pack_tests.rs`
- `apps/mac/src-tauri/src/lib.rs`

**Steps:**

1. Add a public function to `packages/pack/src/lib.rs`:

```rust
pub fn list_packs_in_exports_dir(exports_dir: &Path) -> Result<Vec<PackInspectSummary>, PackError> {
    if !exports_dir.exists() {
        return Ok(Vec::new());
    }

    let exports_root = exports_dir.canonicalize()?;
    let mut summaries = Vec::new();

    for entry in fs::read_dir(&exports_root)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("gsfpack") {
            if let Some(summary) = inspect_pack_candidate(&exports_root, &path)? {
                summaries.push(summary);
            }
            continue;
        }

        if path.is_dir() {
            for nested in fs::read_dir(&path)? {
                let nested = nested?;
                let nested_path = nested.path();
                if nested_path.is_file()
                    && nested_path.extension().and_then(|ext| ext.to_str()) == Some("gsfpack")
                {
                    if let Some(summary) = inspect_pack_candidate(&exports_root, &nested_path)? {
                        summaries.push(summary);
                    }
                }
            }
        }
    }

    summaries.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(summaries)
}

fn inspect_pack_candidate(
    exports_root: &Path,
    candidate: &Path,
) -> Result<Option<PackInspectSummary>, PackError> {
    let canonical = candidate.canonicalize()?;
    if !canonical.starts_with(exports_root) {
        return Ok(None);
    }

    inspect_pack(&canonical).map(Some)
}
```

2. Keep symlink safety: a `.gsfpack` symlink that resolves outside the exports root must not be listed.

3. Update `apps/mac/src-tauri/src/lib.rs` so `list_local_packs` calls `forge_pack::list_packs_in_exports_dir` instead of keeping private duplicate scan logic.

4. Add tests in `packages/pack/tests/pack_tests.rs` covering:

- Missing exports directory returns an empty list.
- Direct `.gsfpack` under the exports directory is listed.
- Nested `.gsfpack` one folder deep is listed.
- A `.gsfpack` symlink escaping the exports directory is skipped.

**Validation:**

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml -p forge_pack list_packs
```

---

## Task 3: Add Minimal Read-Only CLI

**Purpose:** Create the local CLI surface that MCP can later wrap.

**Files:**

- `Cargo.toml`
- `packages/cli/Cargo.toml`
- `packages/cli/src/main.rs`
- `packages/cli/tests/cli_tests.rs`

**Steps:**

1. Add `packages/cli` to the root Rust workspace members.

2. Create `packages/cli/Cargo.toml`:

```toml
[package]
name = "forge_cli"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "forge-cli"
path = "src/main.rs"

[dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
forge_core = { path = "../core" }
forge_pack = { path = "../pack" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"
```

3. Create `packages/cli/src/main.rs` with these command names and JSON-first behavior:

```rust
use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use forge_core::video::ffmpeg::{resolve_ffmpeg_paths, FfmpegSearch};
use forge_pack::{inspect_pack, list_packs_in_exports_dir, validate_pack_layout};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "forge-cli")]
#[command(version)]
#[command(about = "Local automation helpers for Game Sprite Forge")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Toolchain {
        #[arg(long)]
        ffmpeg_path: Option<PathBuf>,
        #[arg(long)]
        ffprobe_path: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    InspectPack {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    ValidatePack {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    ListPacks {
        #[arg(long)]
        exports_dir: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Serialize)]
struct ToolchainStatus {
    ok: bool,
    ffmpeg_path: Option<PathBuf>,
    ffprobe_path: Option<PathBuf>,
    error: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Toolchain {
            ffmpeg_path,
            ffprobe_path,
            json,
        } => {
            let status = match resolve_ffmpeg_paths(&FfmpegSearch {
                configured_ffmpeg_path: ffmpeg_path,
                configured_ffprobe_path: ffprobe_path,
                bundled_resource_path: None,
            }) {
                Ok(paths) => ToolchainStatus {
                    ok: true,
                    ffmpeg_path: Some(paths.ffmpeg),
                    ffprobe_path: Some(paths.ffprobe),
                    error: None,
                },
                Err(error) => ToolchainStatus {
                    ok: false,
                    ffmpeg_path: None,
                    ffprobe_path: None,
                    error: Some(error.to_string()),
                },
            };
            print_json_or_text(json, &status)?;
        }
        Command::InspectPack { path, json } => {
            let summary = inspect_pack(&path)?;
            print_json_or_text(json, &summary)?;
        }
        Command::ValidatePack { path, json } => {
            let report = validate_pack_layout(&path)?;
            print_json_or_text(json, &report)?;
        }
        Command::ListPacks { exports_dir, json } => {
            let summaries = list_packs_in_exports_dir(&exports_dir)?;
            print_json_or_text(json, &summaries)?;
        }
    }

    Ok(())
}

fn print_json_or_text<T>(json: bool, value: &T) -> Result<()>
where
    T: Serialize + std::fmt::Debug,
{
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{value:#?}");
    }
    Ok(())
}
```

4. Add integration tests in `packages/cli/tests/cli_tests.rs`:

- `forge-cli --help` includes `toolchain`, `inspect-pack`, `validate-pack`, and `list-packs`.
- `forge-cli list-packs --exports-dir <missing-dir> --json` returns `[]`.
- `forge-cli toolchain --json` returns JSON with `ok` as a boolean.

**Validation:**

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml -p forge_cli
cargo run --manifest-path /Users/kartz/Development/Forge/Cargo.toml -p forge_cli -- toolchain --json
```

---

## Task 4: Write Architecture And MCP Contract

**Purpose:** Define what MCP can wrap after the CLI exists and make the write-safety model explicit.

**Files:**

- `docs/architecture/cli-mcp-feasibility.md`
- `docs/architecture/post-release-backlog.md`

**Steps:**

1. Create `docs/architecture/cli-mcp-feasibility.md` with these sections:

- Current app command surface
- CLI command contract
- MCP feasibility contract
- Write confirmation rules
- Deferred workflows
- Verification checklist

2. Include this exact product boundary sentence:

```text
Do not build a product MCP server in this spike.
```

3. Define MCP as a local adapter over CLI commands:

```text
toolchain() -> wraps `forge-cli toolchain --json`
inspect_pack(path) -> wraps `forge-cli inspect-pack --path <path> --json`
validate_pack(path) -> wraps `forge-cli validate-pack --path <path> --json`
list_packs(exports_dir) -> wraps `forge-cli list-packs --exports-dir <dir> --json`
```

4. Define write confirmation rules:

```text
Any future command that creates jobs, extracts frames, processes pixels, imports packs, exports packs, overwrites files, or deletes files must support a dry-run response first. The write command must require an explicit confirmation token that includes the resolved absolute output directory and the operation kind.
```

5. Update `docs/architecture/post-release-backlog.md` under the CLI/MCP feasibility gate to link this plan and architecture doc.

**Validation:**

```bash
rg -n "Do not build a product MCP server|write confirmation|toolchain --json|inspect-pack --path|validate-pack --path|list-packs --exports-dir" /Users/kartz/Development/Forge/docs/architecture/cli-mcp-feasibility.md
npm run test:scripts
```

---

## Task 5: Real UI Cross-Check

**Purpose:** Verify the CLI does not change the existing Tauri product path.

**Files:**

- `docs/qa/cli-mcp-feasibility-ui-evidence-2026-06-06.md`

**Steps:**

1. Launch the installed app:

```bash
open -a "/Applications/Game Sprite Forge.app"
```

2. Use Computer Use to verify:

- Forge opens to the local workbench.
- The Exports view or export panel can still show the existing local pack library.
- The app can still inspect or validate a known `.gsfpack`.
- Settings still reports the current FFmpeg/FFprobe path configuration.

3. Run CLI commands against the same known pack path used in the UI:

```bash
cargo run --manifest-path /Users/kartz/Development/Forge/Cargo.toml -p forge_cli -- inspect-pack --path "/Users/kartz/Game Sprite Forge/Exports/Hero-Knight-Pack-1780644870371/Hero-Knight-Pack.gsfpack" --json
cargo run --manifest-path /Users/kartz/Development/Forge/Cargo.toml -p forge_cli -- validate-pack --path "/Users/kartz/Game Sprite Forge/Exports/Hero-Knight-Pack-1780644870371/Hero-Knight-Pack.gsfpack" --json
cargo run --manifest-path /Users/kartz/Development/Forge/Cargo.toml -p forge_cli -- list-packs --exports-dir "/Users/kartz/Game Sprite Forge/Exports" --json
```

4. Record the UI and CLI evidence in `docs/qa/cli-mcp-feasibility-ui-evidence-2026-06-06.md`.

**Validation:**

```bash
test -f /Users/kartz/Development/Forge/docs/qa/cli-mcp-feasibility-ui-evidence-2026-06-06.md
rg -n "Computer Use|inspect-pack|validate-pack|list-packs|FFmpeg" /Users/kartz/Development/Forge/docs/qa/cli-mcp-feasibility-ui-evidence-2026-06-06.md
```

---

## Task 6: Full Verification Gate

**Purpose:** Confirm the CLI/MCP feasibility slice is stable across Rust, scripts, app build, and smoke UI.

**Commands:**

```bash
cargo fmt --all -- --check
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml
npm run test:scripts
npm --workspace apps/mac run build
npm --workspace apps/mac run smoke:ui:mvp
cargo run --manifest-path /Users/kartz/Development/Forge/Cargo.toml -p forge_cli -- toolchain --json
```

**Pass Criteria:**

- All commands above pass.
- The CLI returns JSON for all four planned commands.
- Tauri local pack listing still works after moving scan logic into `packages/pack`.
- The MCP architecture doc keeps all write operations behind dry-run plus explicit confirmation token rules.
- Real UI evidence exists and names the installed app, pack path, CLI commands, and observed result.

---

## Execution Order

1. Task 1: Add source guard, but run it only after Task 4 artifacts exist.
2. Task 2: Extract local pack listing and keep Tauri behavior unchanged.
3. Task 3: Add `forge-cli` read-only commands.
4. Task 4: Write architecture and MCP contract.
5. Task 5: Verify with real UI and matching CLI commands.
6. Task 6: Run the full verification gate.

---

## Recommended Subagent Split

- **Worker A:** Task 2 pack-library extraction and Tauri reuse.
- **Worker B:** Task 3 CLI crate and integration tests.
- **Worker C:** Task 1 and Task 4 docs/source guard.
- **Worker D:** Task 5 real UI evidence and Task 6 final verification.

The workers should merge results only after `npm run test:scripts` and the relevant Rust package tests pass locally.
