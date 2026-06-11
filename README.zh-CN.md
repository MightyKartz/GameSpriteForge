# Game Sprite Forge

[English README](README.md)

Game Sprite Forge 是一个本地 macOS 工作台，用于把视频片段、PNG 序列帧、精灵表和 `.gsfpack` 文件夹转换成可用于游戏开发的 2D 精灵动画资产。

第一个测试版本聚焦“导入优先、本地处理”的工作流：选择本地素材，检查帧，移除或规范化背景，运行质量检查，导出精灵资源包，并通过重新导入验证导出结果。它不依赖云服务、账号、AI provider 配置、市场发布流程或在线 registry。

## 项目介绍

Game Sprite Forge 面向独立游戏开发者、2D 资产制作者、技术美术和小型游戏团队。它适合已经有本地动画素材，并希望用确定性的本地流程把素材整理成游戏引擎可用资源的人。

当前版本可以帮助你：

- 导入视频、PNG 序列帧、精灵表和 `.gsfpack` 文件夹；
- 抽取并检查动画帧；
- 处理绿幕或透明帧素材；
- 将帧规范化到稳定画布和脚底锚点；
- 检查边界、循环、Alpha 边缘和帧一致性等质量信号；
- 导出 PNG 帧、精灵表、manifest、atlas、预览素材和 `.gsfpack` 包；
- 通过本地重新导入验证导出的资源包。

## 当前测试版本

第一个公开测试版本面向 macOS Apple Silicon。

请前往 [GitHub Releases](https://github.com/MightyKartz/GameSpriteForge/releases) 下载。

发布资产：

```text
Game.Sprite.Forge_0.1.0_aarch64.dmg
```

这是测试版本，仍可能存在粗糙边角，但核心的本地 `导入 -> 处理 -> 质量检查 -> 导出 -> 验证` 流程已经可用。

## 功能

- 使用 Tauri、React、TypeScript 和 Rust 构建的本地 macOS 桌面应用。
- 基于 `ffmpeg`/`ffprobe` 的视频导入。
- 支持多行路径的 PNG 序列导入。
- 精灵表导入支持固定网格和透明间隔切分。
- `.gsfpack` 导入、导出、验证和重新导入。
- 带恢复操作的质量报告面板。
- Godot 辅助示例，用于导入 Forge 输出。
- 面向发布加固的源码守护脚本和真实 UI QA 证据。

## 仓库结构

```text
apps/mac/                 Tauri macOS 应用和 React UI
packages/core/            Rust 媒体处理核心
packages/pack/            .gsfpack schema 和资源包逻辑
schemas/                  导出数据的 JSON schema
examples/                 示例输入和引擎辅助示例
scripts/                  QA、打包和发布验证脚本
docs/                     架构说明、计划和 QA 证据
.agents/skills/forge-dev/ 项目级 agent 开发约束
```

## 本地开发

前置条件：

- macOS
- Node.js 和 npm
- Rust toolchain
- macOS Tauri 依赖
- 视频工作流需要 `ffmpeg` 和 `ffprobe`

安装依赖：

```bash
npm install
```

运行前端开发服务器：

```bash
npm run dev
```

运行 Tauri 开发应用：

```bash
npm --workspace apps/mac run tauri -- dev
```

构建 macOS app bundle：

```bash
npm --workspace apps/mac run tauri -- build --debug --bundles app
```

## 验证

常用检查：

```bash
npm --workspace apps/mac run build
npm run test:scripts
npm --workspace apps/mac run smoke:ui:mvp
```

涉及 Rust 媒体处理时：

```bash
cargo fmt --manifest-path Cargo.toml --all -- --check
cargo test --manifest-path Cargo.toml sprite_sheet_transparent
```

发布包请使用 `scripts/` 中已有的发布验证脚本。

## 说明

- 透明间隔精灵表导入由 Forge 的本地 Rust/Tauri 流程实现。
- 第一个测试版本刻意避免云端或账号依赖功能。
- 当前仓库以源码为主。Release candidate 压缩包和 DMG 会作为 GitHub Release 资产发布，不写入 Git 历史。

## License

目前尚未选择开源许可证。在许可证加入前，项目所有权利由项目所有者保留。
