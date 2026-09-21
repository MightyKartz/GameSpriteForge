# Product

**Reliable asset preparation and updates for Godot projects.**

Forge helps independent developers and small teams turn images, animation and
audio into reusable game resources. Its public product is a Rust CLI that people, coding agents and
scripts can use throughout asset production, from local preparation and quality
checks to traceable Packs and native Godot delivery.

## Current investment priorities

Develop the smallest reusable tool that saves work in real game projects. Codex
and other agents handle intent, artwork creation, game code and orchestration;
Forge concentrates on repeatable asset processing and verifiable delivery.

- **Core investment:** preserve pixels, animation coordinates and timing; validate
  engine resources; update owned assets safely; retain useful failure diagnostics,
  recovery and source/version evidence.
- **Maintain compatibility:** existing Provider integrations, library commands,
  previews and Godot environment/export commands remain supported. Extend them
  when a recurring consumer task demonstrates a benefit, not to expand a feature list.
- **Pause expansion:** additional generation platforms, general agent orchestration,
  a replacement game editor, and promotion of optional character/world workflows.
  This is an investment boundary, not removal of existing commands or data.

For simple ready-to-import images, Godot's native import and a reusable script
may be sufficient. A library, Pack history and delivery receipts should not be
prerequisites for understanding the user's task. Offer them when repeated
updates, recovery or reuse justify the extra machinery.

Before broader product investment, compare Codex plus reusable existing tools
against Codex plus Forge on the same tasks. Track user effort, correctness and
ongoing maintenance, including Forge's own overhead. The
[focus decision](docs/architecture/product-focus.md) and
[comparison protocol](docs/qa/asset-delivery-comparison.md) define the evidence
needed. Current engineering tests do not establish a productivity or market win.

**中文：**当前投入聚焦于 Godot 素材的稳定加工、更新、验证和失败恢复。保留现有命令、
格式与历史记录的兼容性，暂缓扩展通用生成平台、智能体编排和高级角色/世界功能。
Codex 负责意图理解、创作与游戏代码；Forge 必须通过实际交付中减少的人力和返工证明
自身价值。简单导入允许使用 Godot 原生能力，不强制引入资源库或整套证据流程。

## Who it serves

Developers working with Codex, other image tools or existing artwork need more
than a collection of generated images. They need assets with consistent framing,
intentional placement, usable engine resources and a record of how each result
was produced. Forge connects that creative work to the game project.

## What Forge provides

- Prepare transparent PNG icons and props on consistent canvases with suitable
  texture sampling and placement anchors.
- Inspect processing results and quality reports, then validate the asset Pack.
  Structural checks and visual approval remain separate decisions.
- Preserve source originals, hashes and processing evidence so revisions can be
  traced back to their inputs instead of replacing the asset's history.
- Deliver textures, prop scenes and usage metadata into Godot, with stable asset
  identities, owned installation targets and recovery after failed installs.
- Reuse existing animation frames through experimental local preparation that
  can preserve shared drawing coordinates, anchors and frame timing through
  Pack and Godot SpriteFrames delivery.
- Prepare layered effects and their playback resources for Godot.
- Import local WAV music and sound effects, validate audio Packs and deliver
  native Godot audio resources. External music and sound-generation tools remain
  optional; Forge does not run their models or download their weights.
- Catalog source media and processed Packs in a project asset library, with
  previews, review records, retained versions, consumer locks and portable
  transfer. Selecting a version and installing it into a game are explicit steps.

- Configure a local Godot engine, lock project toolchain requirements, verify a
  project in an isolated copy and export/test its host-desktop build.

## Artwork and generation

The primary route starts with artwork created in Codex or another image tool,
or transparent PNGs the developer already has. Image creation is a separate
creative step; Forge processes the local files without a Provider account.

Forge also supports controlled online icon and prop generation through a
selected Provider and Style. This route uses the developer's Provider account,
records generation provenance and request usage, and supports targeted retries.
Provider requests can incur charges; local processing has separate usage evidence.

## Working with an AI assistant

Install the CLI and ask the assistant to run `forge guide` before preparing the
assets. The guide and request examples travel with the executable and work
offline. Keep a game's verified Forge version when continuing an existing project.
The workflow supports reviewing and delivering an asset set the game can consume,
with source history and delivery evidence available for later revisions and reuse.

## Current release scope

See the [release overview](README.md#install) for the current version and downloads.
Builds with `godot_version_selection` support Godot 4.6.x and 4.7.x,
with pinned downloads of 4.6.3 or 4.7.2 (default); v0.6.0 includes this
capability. Existing game locks are never upgraded implicitly.
Distribution includes macOS Apple Silicon and an experimental Windows x64 portable
build. Install Godot 4.6.x or 4.7.x separately. CLI binaries are unsigned; the macOS build
is not notarized. The [Windows guide](docs/releases/windows-portable.md) describes
native validation and portable installation.

Forge is free and open source under the [MIT license](LICENSE). Bundled tools have
their own [third-party notices](THIRD_PARTY_NOTICES.md). External model services
may charge separately, and source assets retain their own license conditions.

Character animation remains in development and testing. Existing animation
processing and reviewed project examples do not establish that automatic animation
generation is ready for general production use. Advanced character and world-asset workflows require
optional source-build features and are outside the default release workflow.

Product communication should make actual capabilities, results and review needs
clear. It should help developers create and reuse assets without promising that
generated artwork is automatically ready for every game.

## Historical context

Earlier Forge work produced a desktop MVP focused on local media import and
animation processing. The retired application, its build tooling and design
documents are preserved in Git history. The current repository contains the
CLI toolchain and its supporting contracts, examples and tests. See
[Contributing](CONTRIBUTING.md) for the current development boundaries.
