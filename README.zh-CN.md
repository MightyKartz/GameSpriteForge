# Forge

[English](./README.md) | 简体中文

**面向 AI 协作开发的 2D 游戏资产生产工具链。**

Forge 连接美术创作与 Godot 原生资源交付，将批量资产处理、质量检查、可追溯资产包和引擎集成整合到一款 Rust CLI 中。你可以让 Codex、Claude 驱动制作流程，也可以通过终端和脚本批量执行。

从美术需求出发制作图标与道具库，接入你喜欢的工具生成的图片，或将已有动画帧用于游戏原型。Forge 贯通处理、检查与交付流程，并保留源素材，支持持续迭代。

[最新发布](https://github.com/MightyKartz/GameSpriteForge/releases/latest) ·
[安装](#安装) ·
[CLI 指南](docs/automation/forge-cli.md) ·
[示例](examples/cli)

## 用 Codex 驱动资产制作

[安装 CLI](#安装)后，在 Codex 中打开游戏项目，可以这样说：

> 请用 Forge 和可用的图像工具，为这个 Godot 游戏制作森林主题的背包图标和场景道具。先运行 `forge guide`，保留源图，检查制作结果，再交付到游戏项目。

Codex 使用图像工具创作源图，Forge 负责资产处理、验证与 Godot 交付。CLI 内置操作指南、请求示例和结构化 JSON 输出，让编程智能体能够规划任务、查看进度并核对执行结果。

## 围绕美术方向，批量构建资产集

从 AI 生成图片、手绘精灵或已有素材库开始，将透明 PNG 批量制作成适用于背包、拾取物和游戏场景的图标与道具集。本地处理在你的电脑上完成，无需 Forge Provider 账号。

也可以通过 Forge 固定风格参考，调用在线服务商生成图标或道具集。通过一致性报告检查配色、比例等偏差，再针对选定素材重新生成。在线生成使用你自己的服务商账号，可能产生费用。

![森林主题图标与道具示例素材](docs/media/showcase/gallery.png)

*森林主题图标与道具集：源图由 Codex 单独生成，精灵素材由 Forge 在本地处理。*

从[本地美术工作流](docs/automation/codex-local-assets.md)开始，或查看[在线生成示例](examples/cli)。

## 控制精灵在游戏中的呈现

切分精灵图集、清理色键背景、统一画布规格。图标采用居中原点，道具采用落地锚点，并按美术风格选择清晰的像素采样或平滑过滤。在交付前查看预览和质量报告，确认素材的实际表现。

对于已有动画帧，实验工作流可在导出时保留绘制坐标与逐帧时长。源文件和处理记录会保留，方便后续调整与重新制作。

![将已有素材处理为透明精灵帧](docs/media/showcase/processing.png)

*从带色键背景的源图到透明精灵：Forge 本地背景处理的实际效果。*

## 验证资产，交付 Godot 原生资源

Forge 将纹理和资产信息封装为可验证的 Pack，再安装到 Godot 项目。图标集交付 PNG 纹理，道具集提供带锚点和渲染设置的可复用场景；动画 Pack 提供原生 `SpriteFrames` 资源和 `AnimatedSprite2D` 场景。更新时仅替换 Forge 管理的资产，交付失败会恢复上次安装。

通过源文件指纹、处理设置和任务报告，可以追溯资产来源，核对每次迭代的变化。Pack 验证负责检查交付结构与完整性，美术是否适合游戏则由实际画面审查决定。

### 实际项目：Sword

Sword 是一款修仙题材的生存游戏原型，使用 Forge 完成资产处理与交付。法术特效和怪物动画的源图由 Codex 生成，经 Forge 处理后导入 Godot；下方动图在独立的资产展示场景中回放这些已有动画。

![Sword 原型法术：离火、寒霜、落雷](docs/media/showcase/sword-spells.gif)

*离火、寒霜、落雷：已交付到 Godot 的法术帧动画。*

![Sword 原型怪物：游魂、石傀、妖藤，以及守卫落击动作](docs/media/showcase/sword-enemies.gif)

*游魂、石傀、妖藤与守卫落击：游戏原型中的怪物动画素材。*

## 安装

稳定版本为 [v0.3.2](https://github.com/MightyKartz/GameSpriteForge/releases/tag/v0.3.2)，支持 **macOS Apple Silicon**。需要引擎交付时，另行安装 **Godot 4.6.x**。二进制文件尚未签名或公证。已有游戏应先验证升级，再调整固定的 CLI 版本。

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/MightyKartz/GameSpriteForge/main/install.sh | sh
```

重新打开终端，检查安装：

```bash
forge --version
forge doctor --json
forge guide
```

`forge guide` 读取当前 CLI 版本附带的操作指引与请求示例，离线也能使用。

制作第一组资产，请参考[本地美术工作流](docs/automation/codex-local-assets.md)：处理图片、检查结果、验证 Pack，再安装到 Godot。

## 当前能力与开发进展

本地图标与道具处理、Pack 验证和 Godot 交付构成稳定能力。**角色动画仍处于测试开发阶段**，包括生成、复用和多方向工作流。Sword 展示的是具体原型素材的应用，用于游戏前仍需检查动画效果。版本范围见 [v0.3.2 发布说明](docs/releases/v0.3.2.md)。

## 文档

- [CLI 指南](docs/automation/forge-cli.md)：命令、素材生成与处理、Godot 交付。
- [本地美术工作流](docs/automation/codex-local-assets.md)：将 Codex 或其他工具创作的美术处理并交付到 Godot。
- [示例规格](examples/cli)：以现有示例开始制作自己的素材。
- [发布说明](docs/releases/v0.3.2.md)：平台支持和版本范围。
- [展示素材](docs/media/showcase/README.md)：美术来源与 Godot 预览。
- [参与开发](CONTRIBUTING.md)：源码构建与开发检查。

## 许可证

[MIT](LICENSE)。附带 FFmpeg 工具有独立 LGPL 声明和对应源码分发，详见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
