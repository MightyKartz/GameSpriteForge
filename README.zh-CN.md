# Forge

[English](./README.md) | 简体中文

**面向智能体的 2D 游戏资产 CLI：生成、处理与复用素材，保留来源记录并交付 Godot。**

Forge 为 Codex、Claude、脚本和 CI 提供统一的 JSON 接口。你可以生成角色、图标和道具，
在本地处理已有媒体，也可以复用已批准的动画帧而不再生成新视频，最终交付可检查的
`.gsfpack` 资产与 Godot 原生资源。

[最新发布](https://github.com/MightyKartz/GameSpriteForge/releases/latest) ·
[CLI 参考](docs/automation/forge-cli.md) ·
[示例](examples/cli) ·
[参与开发](CONTRIBUTING.md)

## 核心能力

- **一致性资产：**使用不可变 Style Lock 引导角色、图标集和道具集生成。
- **本地处理：**抠图、帧规范化、循环选取、精灵图集与 Pack 校验。
- **已有动画复用：**保留已批准的源像素、帧序、原生时长和来源记录，组合成独立的方向动画审核候选。
- **可检查任务：**持久化 Job、结构化报告、计划与执行流程，以及定向重试。
- **Godot 交付：**外部 PNG/atlas 纹理、原生动画资源、安装所有权检查和使用记录。
- **智能体集成：**stdout 输出单个 JSON envelope，诊断与交互式认证使用 stderr/TTY。

Forge 负责视觉资产和引擎交付；玩法与游戏逻辑由消费这些资产的项目负责。

## 安装

已发布 CLI 面向 **macOS Apple Silicon**。需要引擎交付时，另行安装 **Godot 4.6.x**。

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/MightyKartz/GameSpriteForge/main/install.sh | sh
```

重新打开终端，检查安装：

```bash
forge --version
forge doctor --json
```

安装器会验证 SHA-256 清单，将 `forge`、`ffmpeg` 和 `ffprobe` 安装到版本化用户目录，
仅把 `forge` 暴露到 `PATH`。当前已发布版本为
[`v0.2.0-cli.1`](https://github.com/MightyKartz/GameSpriteForge/releases/tag/v0.2.0-cli.1)，
尚未签名或公证。`main` 上的新能力可能需要从源码构建。

## 生成第一个角色

参考 [Style 示例](examples/cli/style.json)和[角色示例](examples/cli/character.json)，
保存自己的规格文件，并替换下面的绝对路径。API Key 是稳定的 xAI 认证方式，OAuth 为
Preview。真实 Provider 生成可能产生费用；执行前核对计划及请求上限，详见
[CLI 参考](docs/automation/forge-cli.md)。

```bash
forge provider login --provider xai --method api-key
forge project init --path "$PWD/game-assets" --name "My Game"

# 先规划 Style Lock，再使用返回的 token 执行。
forge style create --project "$PWD/game-assets" \
  --spec /absolute/style.json --plan-only --json
forge plan execute --token STYLE_PLAN_TOKEN --wait --json

# 单独规划角色，再使用对应的 token 执行。
forge generate character --project "$PWD/game-assets" \
  --spec /absolute/character.json --plan-only --json
forge plan execute --token CHARACTER_PLAN_TOKEN --wait --json
```

用每次计划响应中的实际值替换 token 占位符。生成默认使用持久化异步 Job；`--wait`
会等待完成。从 Job 的 artifacts 获取真实 Pack 路径：

```bash
forge job report --id JOB_ID --json
forge pack validate --path /absolute/Character.gsfpack --json
```

制作背包和场景资产时，使用 `forge generate icon-set` 或 `forge generate prop-set`，
并提供对应的[图标规格](examples/cli/icons.json)或[道具规格](examples/cli/props.json)。

## 交付到 Godot

使用已有 Godot 项目，以及已完成 Job 返回的 Pack 路径：

```bash
forge godot plan-install \
  --pack /absolute/Character.gsfpack \
  --project /absolute/my-godot-game \
  --asset-key my_character --json

forge plan execute --token INSTALL_PLAN_TOKEN --wait --json
```

Forge 使用外部纹理，安装到 `addons/forge_assets`，并追踪由 Forge 管理的输出。
用于游戏前，检查安装资源并在 Godot 中审核动画效果。

## 不再生成新视频的动画复用

源码仓库提供实验性的[三方向复用流程](docs/architecture/forge-existing-animation-reuse-plan.md)：
将已有、已批准的 right/down/up 恢复 Pack 组合到独立 Godot 审核项目中，保留源帧和原生
时序，并按方向应用整段固定缩放。安装计划必须明确预计与最大 Provider 请求均为零。

辅助工具需要 Python 3.9+、Pillow、从当前源码构建的 Forge CLI、Godot 4.6.x，以及符合
输入约定的来源批准和尺度证据。它适用于现有三方向恢复素材；完整源媒体不随仓库分发。

可查阅[集成验证](docs/qa/forge-directional-reuse-main-integration-2026-09-07.md)与
[人工审核通过记录](docs/qa/forge-directional-human-review-2026-09-07.md)。批准绑定具体候选；
修改后的候选需要独立审核。工具不会补出缺失的左向或 idle 动画。

## 发布版本与源码能力

以 [CLI feature 定义](packages/cli/Cargo.toml)为准，默认构建使用 `default = []`。

| 能力 | 可用范围 |
| --- | --- |
| Style Lock、角色/图标/道具生成、本地资产准备、Job、Pack 校验、Godot 安装 | 默认 CLI |
| 三方向复用辅助工具、原生逐帧时长与渲染参数支持 | 已进入 `main`，与当前发布二进制分开 |
| Subject Lock 与 Character 一致性 V2 | 按需启用 `consistency-v2` 源码 feature |
| Environment、Terrain、Building、Map 工作流 | 按需启用 World features；`world-assets` 启用整组 |
| 资产清单驱动的项目 diff 与构建计划 | 按需启用 `game-art-manifest` 源码 feature |

可选功能和 fixture 检查不能代替真实 Provider 或人工视觉验收。当前产品重心是 CLI 和
Rust 工作区；保留的桌面/MCP 代码不属于默认发布范围。

## 开发

```bash
cargo build -p forge-cli
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 scripts/character/test_prepare_directional_reuse.py -v
```

环境配置与适用检查见 [CONTRIBUTING.md](CONTRIBUTING.md)，CI 范围见
[质量矩阵](.github/workflows/v03-quality.yml)，证据保存遵循
[QA 产物政策](docs/qa/forge-qa-artifact-policy.md)。不提交凭证或临时媒体 URL；
Provider 输出应在本地落盘并计算哈希，重试和派生结果须保留源 Job/Pack 的来源链。

## 许可证

[MIT](LICENSE)。附带 FFmpeg 工具有独立 LGPL 声明和对应源码分发，详见
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
