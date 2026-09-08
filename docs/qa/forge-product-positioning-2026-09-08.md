# Product positioning refresh — 2026-09-08

## Product message

The English and Chinese READMEs now position Forge as a 2D game asset pipeline
for AI-assisted development. They explain the value of batch production,
processing controls, quality evidence, traceable Packs and native Godot delivery.
The four existing showcase images remain unchanged.

The primary claims were checked against the current
[CLI protocol](../automation/forge-cli.md) and
[v0.3.2 release scope](../releases/v0.3.2.md):

- Style references, consistency reports and selected-item regeneration are
  described under online Provider generation. Local PNG preparation does not
  claim style-consistency checks or targeted retries.
- Static origins, sampling, animation timing, native Godot resources and install
  recovery reflect the shipped workflows. Existing animation remains experimental.
- Source and processing records support traceability. Pack integrity checks and
  visual approval are separate; no automatic production-quality claim is made.
- `forge guide` remains the entry point. No skill installation instructions were
  reintroduced into the main READMEs.

`PRODUCT.md` now describes the public Rust CLI. The retained Tauri desktop MVP
and its old release/signing evidence are explicitly historical, rather than
the current product and distribution status.

## GitHub About

The repository uses one bilingual description:

> A 2D game asset pipeline for AI-assisted development: generation, processing, validation and native Godot delivery. 面向 AI 协作开发的 2D 游戏资产生产工具链：生成、处理、质量检查与 Godot 原生交付。

Topics: `rust`, `godot`, `gamedev`, `game-assets`, `sprite-sheet`, `cli`, `codex`,
`macos`. The old `react` and `tauri` product tags are removed; the retained desktop
source is still documented in the project.

## Verification

- Checked 40 local links across both READMEs and `PRODUCT.md` before this note.
- Confirmed both READMEs retain four matching media references, the direct guide,
  the v0.3.2/macOS Apple Silicon/Godot 4.6.x scope and the animation boundary.
- Independent claim review passed after changing a `PRODUCT.md` sentence so that
  visual approval is a workflow step, not a guaranteed tool outcome.
- `git diff --check` passed. Documentation and repository metadata only; no CLI
  rebuild, media regeneration, consumer-project changes or new release required.

Publication verification compares the GitHub About description/topics and main
commit with the intended values. Local evidence is retained under
`target/qa/product-positioning-20260908/`.
