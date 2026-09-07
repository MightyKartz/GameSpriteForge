# Forge

[English](./README.md) | 简体中文

**生成 2D 游戏素材，整理精灵资源，交付到 Godot。**

Forge 是一款游戏资产 CLI，可生成图标和道具集、处理已有图片与动画，并将资源交付到游戏项目。你可以直接在终端使用，也可以让 Codex、Claude 配合完成素材制作。

[最新发布](https://github.com/MightyKartz/GameSpriteForge/releases/latest) ·
[安装](#安装) ·
[CLI 指南](docs/automation/forge-cli.md) ·
[示例](examples/cli)

## 生成风格协调的图标与道具

用文字描述制作背包图标、拾取物和场景道具。复用同一套风格设定，让整组素材更协调；某个素材不满意，可以单独重试，保留其余结果。

![森林主题图标与道具示例素材](docs/media/showcase/gallery.png)

*为本页单独用 AI 生成的森林主题示例素材，随后使用 Forge 处理。*

从[图标集](examples/cli/icons.json)、[道具集](examples/cli/props.json)和[风格设定](examples/cli/style.json)示例开始。

## 把已有素材整理成精灵资源

导入自己的视频、PNG 序列或精灵图集，在本地完成背景移除、帧对齐、循环选取和图集制作，让已有美术素材更方便地用于游戏。

![将已有素材处理为透明精灵帧](docs/media/showcase/processing.png)

*从原始图集中切分的一张精灵图，展示 Forge 去除背景前后的实际效果。*

## 将资源交付到 Godot

把纹理、图集和原生动画资源交付到 Godot 项目，保留逐帧时长和渲染设置。直接在引擎中预览，再根据游戏效果继续调整。

![使用处理后的 PNG 精灵搭建的 Godot 演示场景](docs/media/showcase/godot.png)

*使用 Forge 处理后的 PNG，在 Godot 中搭建的演示场景。*

### 按需继续制作

查看任务进度、检查结果，只重试需要调整的素材。Forge 支持终端、脚本和编程智能体，既适合处理几张素材，也能重复执行批量任务。

## 安装

当前发布版本为 [v0.2.1](https://github.com/MightyKartz/GameSpriteForge/releases/tag/v0.2.1)，支持 **macOS Apple Silicon**。需要引擎交付时，另行安装 **Godot 4.6.x**。当前二进制文件尚未签名或公证。

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/MightyKartz/GameSpriteForge/main/install.sh | sh
```

重新打开终端，检查安装：

```bash
forge --version
forge doctor --json
```

制作第一组素材，可结合 [CLI 指南](docs/automation/forge-cli.md)与[图标](examples/cli/icons.json)或[道具](examples/cli/props.json)示例。在线生成需要使用自己的服务商账号，可能产生费用；本地素材处理无需重新生成。

## 开发中的功能

**角色动画目前仍处于测试开发阶段。** 角色生成、动画复用和多方向动画仍需检查实际画面。实验性的动画复用辅助工具随源码提供，CLI 安装器不包含该工具。

高级角色一致性和世界素材工作流也需要启用可选源码功能。当前下载包的具体范围见[发布说明](docs/releases/v0.2.1.md)。

## 文档

- [CLI 指南](docs/automation/forge-cli.md)：命令、素材生成与处理、Godot 交付。
- [示例规格](examples/cli)：以现有示例开始制作自己的素材。
- [发布说明](docs/releases/v0.2.1.md)：平台支持和版本范围。
- [展示素材](docs/media/showcase/README.md)：图片来源与可复现的 Godot 演示。
- [参与开发](CONTRIBUTING.md)：源码构建与开发检查。

## 许可证

[MIT](LICENSE)。附带 FFmpeg 工具有独立 LGPL 声明和对应源码分发，详见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
