# Forge

[English](./README.md) | 简体中文

**让 AI 智能体更顺畅地开发 Godot 游戏。**

Forge 是**免费、MIT 开源的 CLI 工具**，连接 Codex 等智能体、素材创作工具与 Godot。将生成或已有的 2D 图片、动画和音频加工为可复用的游戏资源，统一管理并交付到项目，再协助完成运行检查、截图和本机桌面导出。

[最新发布](https://github.com/MightyKartz/GameSpriteForge/releases/latest) · [安装](#安装) · [主要功能](#主要功能) · [CLI 指南](docs/automation/forge-cli.md)

![雷灵与雷击在 Godot 中的实际回放](docs/media/showcase/thunder/godot-demo.gif)

*Codex 创作素材，Forge 加工、管理并交付到 Godot。[观看回放](docs/media/showcase/thunder/godot-demo.mp4) · [原始图集与制作说明](docs/media/showcase/thunder/README.zh-CN.md)。*

## 安装

[v0.5.0](https://github.com/MightyKartz/GameSpriteForge/releases/tag/v0.5.0) 支持 **macOS Apple Silicon**，并提供**实验性 Windows x64 便携包**。使用 Godot 工作流时，可以选择已有的 **Godot 4.6.x**，或按下方说明通过 Forge 下载固定版本的官方引擎。安装包尚未签名，macOS 包尚未公证。

macOS 安装：

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/MightyKartz/GameSpriteForge/main/install.sh | sh
```

重新打开终端，然后运行：

```bash
forge --version
forge doctor --json
forge guide
```

Windows 用户请按[便携包安装指南](docs/releases/windows-portable.md)操作。已有游戏应先验证升级，再修改固定的 CLI 版本。

已有 Godot 时，运行 `forge setup godot --path PATH`，将 `PATH` 替换为引擎路径。尚未安装时，可下载并配置固定的官方 Godot 4.6.3：

```bash
forge setup godot --download
```

需要导出模板时再加上 `--templates`，模板会单独下载。配置与导出示例见 `forge guide godot-workflow`。

## 主要功能

| 工作流 | 可以完成的事情 |
| --- | --- |
| **资源库** | 按名称或标签查找素材，预览、对比版本，保留可复用文件，在 macOS 与 Windows 间转移选定资源。 |
| **图片** | 批量处理 PNG 图标和道具，设置背景、画布、锚点与采样方式；需要时可使用自己的 xAI 账号生成素材集。 |
| **动画与分层** | 加工已有动画帧或精灵图集，保留源坐标和时长，将已配准图层及变换、透明度轨道封装成 Pack。 |
| **音频** | 导入 WAV 音乐、音效和环境声，进行裁剪、增益调整、淡入淡出和循环处理。 |
| **Godot 交付** | 验证 Pack，安装原生纹理、场景、动画和音频资源；锁定选定版本、保留交付回执，并在更新失败时回滚。 |
| **Godot 开发协作** | 配置引擎、锁定项目工具版本、检查游戏启动、截图，并导出和检查当前电脑平台的桌面游戏。 |

### 音频交付

Forge 将本地 WAV 加工并交付为 Godot 原生音频资源。这个示例在 Godot 演示场景中播放一段合成的三音提示音。

![Forge 交付的合成提示音在 Godot 中回放](docs/media/showcase/v040/native-delivery.gif)

*GIF 本身无声。[观看有声演示](docs/media/showcase/v040/native-delivery.mp4) · [素材来源与复现](docs/media/showcase/v040/README.md)。*

### 检查并导出 Godot 游戏

交付素材后，可以让智能体在独立项目副本中检查游戏运行、截图供你查看，并在出现问题时读取日志。项目工具链锁可以帮助不同电脑保持一致的 Forge 和 Godot 版本。

Forge 还能使用已有导出预设，构建当前电脑平台的桌面游戏，并检查导出的程序能否启动。画面、声音和玩法仍由你实际验收。命令与详细检查机制见 [Godot 工作流指南](.agents/skills/forge-use/references/godot-workflow.md)。

## 配合 Codex 使用

在 Codex 中打开游戏项目并提出需求：

> 请用 Forge 加工和管理这个游戏的源图、动画帧和 WAV 音频。先运行 `forge guide`，保留原始素材并登记到项目资源库，向我展示加工结果供审核，再安装到 Godot。然后用 Forge 检查游戏启动并截图，告诉我发现的问题，以及还需要我实际验收的内容。

内置指南支持离线读取。通过 `forge guide project-assets`、`forge guide static`、`forge guide animation`、`forge guide audio` 或 `forge guide delivery` 查看对应主题。macOS/Linux 还可选择[安装 Codex skill](docs/automation/forge-cli.md#optional-bundled-codex-skill)，让 Codex 发现它。

## 实际项目：Sword

Sword 是一款修仙题材的生存游戏原型。源图由 Codex 创作，Forge 将这些法术和怪物动画加工并交付到 Godot。

![Sword 法术：离火、寒霜、落雷](docs/media/showcase/sword-spells.gif)

![Sword 怪物：游魂、石傀、妖藤与守卫落击](docs/media/showcase/sword-enemies.gif)

*使用早期 Forge 构建导入的已有资源，在独立素材展示场景中回放。详见[来源与录制说明](docs/media/showcase/README.md)。*

## 能力范围

角色动画仍属实验性功能；高级角色及世界资产命令需要启用可选的源码构建功能。使用 `forge doctor --json` 核对当前 CLI 的能力。

音乐和音效由外部应用生成。ACE-Step 和 Stable Audio 3 提供可选的源代码目录只读检测，Forge 不安装或运行其模型。在线 xAI 请求使用你的账号，可能产生费用。

技术检查不能代替美术审查、试听或许可核对，使用前请在游戏中验收。已验证的版本范围见 [v0.5.0 发布说明](docs/releases/v0.5.0.md)。

## 文档

- [CLI 指南](docs/automation/forge-cli.md)：命令与自动化。
- [项目资源库](docs/automation/project-asset-library.md)：登记、版本、审核与复用。
- [Godot 工作流](.agents/skills/forge-use/references/godot-workflow.md)：引擎配置、工具链锁定、游戏检查与桌面导出。
- [本地美术](docs/automation/codex-local-assets.md)、[音频](.agents/skills/forge-use/references/audio.md)和[分层 Pack](docs/automation/layered-packs.md)：素材加工流程。
- [示例规格](examples/cli)：素材请求的起点。
- [参与开发](CONTRIBUTING.md)：源码构建与开发检查。

## 许可证

[MIT](LICENSE)。附带 FFmpeg 工具有独立 LGPL 声明和对应源码分发，详见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
