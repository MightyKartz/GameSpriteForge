# Post-Release Backlog

Date: 2026-06-06

## Scope Guard

The first public release remains import-only and local-first. AI generation, BYOK, website, registry, marketplace, MCP, Codex Skill, cloud upload, and creator publishing stay deferred until the local pack format and release pipeline are stable.

## Next Development Slices

## Post-RC Completed Evidence

```text
Completed after RC: deterministic missing ffmpeg QA, first-run sample copy, Local Pack Library, Local Workbench UX Hardening, UI Language Localization, Godot helper schema evidence, Godot editor/sample-project validation, and bundled ffmpeg evaluation.
Current release package: release-candidates/GameSpriteForge-0.1.0-aarch64-0a5d18c3d1c8-notarized
Next decision gate: tester-driven paper-cut fixes from the local app. CLI/MCP remains deferred until repeated local workflows prove what automation layer is actually needed.
```

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

Resolved before Slice 2 start:

```text
QA-004 high-resolution video export sizing now uses multi-page sprite sheets and no longer blocks large local videos at the first 2048 texture limit.
Godot helper schema-level evidence exists in docs/qa/godot-helper-evidence-2026-06-05.md.
Godot editor/sample-project validation exists in docs/qa/godot-editor-helper-evidence-2026-06-06.md.
```

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

Current result:

```text
Decision: keep user-configured/PATH-only for the current release candidate.
Reason: local Homebrew ffmpeg 8.0.1 is GPL-enabled and no dedicated distributable ffmpeg/ffprobe build has been selected.
Evidence: docs/architecture/bundled-ffmpeg-evaluation.md and docs/qa/ffmpeg-bundle-evaluation-2026-06-06.md.
```

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

### Slice 4: Local Workbench UX Hardening

Goal: make the existing local app clearer after each user action before expanding to CLI/MCP, AI generation, website, registry, marketplace, or cloud features.

Status: completed on 2026-06-06.

Entry criteria:

```text
local pack library and exporter refinement are complete
installed-app real UI evidence exists for the current release candidate
```

Deliverables:

```text
Local Pack Library action status
first-run sample action clarity
export validation result feedback
Computer Use installed-app evidence
CLI/MCP deferred until the local workbench UX evidence is complete and stable
```

Execution plan:

```text
docs/superpowers/plans/2026-06-06-forge-local-workbench-ux-hardening.md
docs/qa/local-workbench-ux-evidence-2026-06-06.md
```

Deferred automation reference:

```text
docs/superpowers/plans/2026-06-06-forge-cli-mcp-feasibility-spike.md
```

### Slice 5: UI Language Localization

Goal: make Forge follow the system language by default while allowing users to override the UI language in Settings.

Status: completed on 2026-06-06.

Entry criteria:

```text
Local Workbench UX Hardening is complete
current app shell and primary workflow have real UI evidence
```

Deliverables:

```text
Automatic language mode
English override
Simplified Chinese override
typed local i18n dictionary
Settings language selector
English and Simplified Chinese UI smoke evidence
Computer Use installed-app evidence
zh-CN runtime visible-text guard
quality recommendation i18n mapping guard
```

Execution plan:

```text
docs/superpowers/plans/2026-06-06-forge-ui-language-localization.md
docs/qa/ui-language-evidence-2026-06-06.md
```

### Slice 6: Transparent-Gutter Sprite Tooling

Goal: harden transparent-gutter sprite sheet import and follow-on frame cleanup while keeping Forge local-first, Rust-backed, and release-safe.

Reference:

```text
docs/architecture/sprite-tooling-followups.md
docs/superpowers/plans/2026-06-11-forge-transparent-gutter-sprite-sheet-intake.md
```

Entry criteria:

```text
local sprite sheet import/export remains stable
manual grid import has no open P0/P1 bugs
external project source is not copied into Forge
```

Deliverables:

```text
automatic transparent-gutter sprite sheet splitting
deterministic Rust tests for split behavior
Import Panel mode switch between fixed grid and transparent gutters
QA evidence proving downstream normalization, quality report, and export still use the existing pipeline
```

Deferred followups:

```text
frame repair workbench
GIF/WebP utility import/export
RPG Maker and additional engine presets
```

## Explicit Non-Goals For The Next Slice

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
hosted credits
```
