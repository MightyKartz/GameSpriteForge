# Product

**A game-resource CLI for AI agents, starting with Godot.**

Coding agents are the direct users; game developers are the beneficiaries. Forge
turns images, animation frames and audio into usable game resources through
repeatable preparation, actionable diagnostics, validation and native delivery.
Design commands, reports and guidance for agents to discover and complete tasks
with little bespoke scripting or intervention. Developers need not learn Forge
protocols; human review and delivery authority follow the actual task.

## Free Forge and Forge PRO

Forge continues as the free MIT CLI. Existing processing, animation-frame import,
resource management, Godot delivery, published guides and examples remain here,
including the accepted workflow in PR #78. Character assets can still be prepared
with free Forge; no existing feature is being moved behind a paywall.

Forge PRO is maintained as a separate private project. It
builds character-production workflows on a pinned Forge CLI. New image-driven
and video-driven authoring guidance, reference/appearance workflows, candidate
comparison and production orchestration belong in Pro. Codex or another chosen
tool generates artwork; Pro coordinates local preparation, review and delivery.
Video inputs can come from local generation, rendering, filming or downloaded
services; Vidu is one optional source. Neither built-in OpenAI generation nor
native integration with every provider is implied. Beta customer access and updates
follow the terms of the selected storefront; repository previews do not themselves
establish a customer license. Third-party generation charges remain separate.

- [Mianbaoduo update subscription](https://mbd.pub/o/bread/YZaVl55tZw==).
- [itch.io purchase](https://mightykartz.itch.io/forge-pro-beta2).

The storefront URL may retain an older edition name; check its actual current
version and download. Platform prices and access terms are independent. UI/title
authoring templates are added in Pro beta.4; game navigation remains game-owned.

Shared processing fixes belong in Forge. Pro-specific orchestration belongs in
its own repository; the projects do not maintain duplicated Rust implementations.
Existing open feature PRs remain subject to their normal scope and compatibility
review; this boundary does not automatically relocate or close them.

**中文：**Forge 继续免费并保持 MIT 许可。现有加工、动画帧导入、资源管理、Godot
交付、公开指导及案例（含 PR #78）继续保留；角色素材仍可用免费 Forge 处理。Forge PRO
是独立私有项目，提供自己的角色/UI 制作流程，复用固定版本的 Forge CLI，负责源视频
审阅、完整动作周期选择、抠图与节奏复核、预览和交付编排。支持以本地可解码视频作为
输入，包括本机生成、渲染、拍摄及在线服务下载的视频；Vidu 只是可选来源，不代表已
对接全部平台 API。测试版客户访问、价格与更新权益按所选店铺条款提供；仓库预览
不等于客户授权。第三方生成费用另计。通用底层修复
继续进入 Forge，Pro 编排单独维护，不复制两份 Rust 核心。后续新增的图片及视频驱动
角色创作经验、动作参考与外观替换流程、候选对照和制作编排进入 Pro；由 Codex 或用户
选定工具生成原图，Pro 协调本地准备、复核和交付，不宣称自带 OpenAI 图片生成服务。
既有开放 PR 按原流程审核，此次不自动搬迁或关闭。面包多更新订阅与 itch.io 一次购买
分别适用各自条款（链接见上）；网址可能沿用旧版名称，应查看实际版本和下载文件。
Pro beta.4 新增 UI/标题制作模板，实际游戏导航仍由游戏维护。

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
against Codex plus Forge on the same tasks. Track autonomous completion, agent
time, retries, bespoke scripts, interventions and verifiable delivery, including
setup and maintenance overhead. Human time is a supporting measure. The
[focus decision](docs/architecture/product-focus.md) and
[comparison protocol](docs/qa/asset-delivery-comparison.md) define the evidence
needed. Current engineering tests do not establish a productivity or market win.

**中文：**当前投入聚焦于 Godot 素材的稳定加工、更新、验证和失败恢复。保留现有命令、
格式与历史记录的兼容性，暂缓扩展通用生成平台、智能体编排和高级角色/世界功能。
直接使用者是 Agent，游戏开发者是最终受益者。Codex 负责意图理解、创作与游戏代码；
Forge 通过提高 Agent 自主完成率、减少耗时、临时脚本、重试和非预期人工介入证明价值，
并计入自身设置与维护成本。人工时间是辅助指标。简单导入允许使用 Godot 原生能力，
不强制引入资源库或整套证据流程。

## Who it serves

Agents developing games need assets with consistent framing, intentional
placement, usable engine resources and recoverable production history. Forge
provides these operations as discoverable tools. Developers specify the game
outcome; agents handle the processing details within that authority.

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
