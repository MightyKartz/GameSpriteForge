# Forge 项目理解与定向验证

日期：2026-09-12。范围：本地源码、产品文档、默认 CLI 构建与基础契约测试；不是完整代码审计或发布验收。

此记录对应清理前提交 `4e67b5d`。随后已按纯 CLI 范围清理桌面源码和工具；当前状态见[CLI 仓库清理验收](forge-cli-only-cleanup-2026-09-12.md)。下文保留当时的分析基线。

## 基线

- 分支：`main`，提交：`4e67b5d5c123cba932bb8e3dfcabe99dd1f673d7`。
- 开始分析时工作区干净；本次只新增此报告，没有修改实现。
- Cargo workspace 版本：`0.3.2`。
- 构建：`cargo build --locked -p forge-cli --no-default-features`，通过。
- 二进制：`/Users/kartz/Development/Forge/target/debug/forge`。
- SHA-256：`6eac96ca979213f4fb96a3e25957bdb89db28d9b7829fd98732682f06a64c623`。
- doctor 的编译身份：上述提交，`dirty=false`，`aarch64-apple-darwin`，`debug`，`features=[]`。这是新增本报告之前构建的身份。
- 本机发现 Godot `4.6.3.stable.official.7d41c59c4` 和 Homebrew FFmpeg/FFprobe；未验证安装包内置 helper。

## 产品与架构

Forge 面向 AI 协作开发，将 2D 美术源素材处理成可验证、可追溯、可交付 Godot 的资产。公开产品是 Rust CLI；`apps/mac` 保留 React/TypeScript/Vite/Tauri 桌面实现，但不在默认 Cargo workspace 和 CLI 发布范围内。

| 层 | 入口 | 职责 |
| --- | --- | --- |
| CLI | `packages/cli/src/main.rs` | Clap 命令、JSON 协议、功能开关、执行预检、后台 worker |
| Core | `packages/core/src/lib.rs` | 素材处理、计划、任务、质量、修复、生成编排和引擎交付 |
| Pack | `packages/pack/src/lib.rs` | Pack 布局、JSON Schema、文件路径、动画时间与渲染契约校验 |
| Providers | `packages/providers/src/lib.rs` | xAI 与离线 fixture；认证及媒体生成适配 |
| 契约与集成 | `schemas/`、`profiles/`、`scripts/godot/` | 数据格式、处理配置、Godot 原生资源构建 |

依赖方向：CLI → Core / Pack / Providers；Providers → Core 中的 `MediaGenerationProvider` trait；Core → Pack。生成接口在核心定义，服务商实现独立。

主流程：请求 → 校验与输入指纹 → Plan → 执行预检与单次 token 消费 → Job → 素材处理或服务商生成 → 质量报告 / 预览 / Pack → 独立 Godot 安装计划。

- Plan 有 15 分钟有效期，执行时检查输入指纹，拒绝源文件发生变化的请求。
- JobStore 使用目录和 JSON 保存任务、源素材、处理结果、预览、导出、日志与备份。异步 CLI 启动同一可执行文件的隐藏 `__worker` 子命令。
- Job 同时记录 `JobState` 处理阶段和 `JobLifecycleState` 执行生命周期。两者语义不同；后续修改必须同步考虑取消、失败、等待审核与终态。
- 静态入口保留源 alpha，根据前景边界及边缘留白统一画布；图标居中，道具采用地面原点。导出记录源图与结果指纹。
- Pack 是含 `forgepack.json`、资产、预览和报告的目录。验证除结构外还覆盖路径约束、符号链接和跨文件时间/渲染一致性。
- Godot 安装具有目标归属检查、备份及多个失败分支的恢复逻辑，并调用实际 Godot 执行导入和原生资源生成；本次未全面验证所有失败路径。

## 能力边界

- 稳定主线：本地透明 PNG 图标/道具集 → 静态 Pack → Godot。
- 默认 CLI 也包含本地动画、保留源坐标、逐帧时长和整张图集预处理；产品文档仍将角色动画定位为实验能力。
- 在线风格参考与静态素材生成经 Provider 执行；Codex 图像工具产生的 PNG 属于外部源图，不应记作 Forge Provider 生成。
- `consistency-v2`、`terrain-assets`、`building-assets`、`map-compiler`、`game-art-manifest` 在 CLI 通过 Cargo feature 控制；不能因 Core 中存在实现就认为默认命令可用。
- `guide` 和 `skill` 共享 `.agents/skills/forge-use/` 一份内嵌内容，升级可执行文件与更新已安装 skill 是不同操作。
- 本地静态 `game_ready` 证明结构检查通过，不证明视觉效果、风格或动画动作已经获得人工认可。

## 维护观察

1. `packages/cli/src/main.rs` 约 3,002 行，`automation/runner.rs` 约 6,283 行，生成、处理、导出和安装编排高度集中。未来改动宜按业务操作逐步分拆，保留现有契约测试；此项是维护性观察，并非已确认缺陷。
2. 旧桌面代码、早期规划和当前 CLI 同库存在。判断当前产品应优先核对 workspace、feature、实际二进制能力和近期发布说明。
3. 项目已有较多输入完整性、取消并发、兼容性、Pack 校验和真实 Godot 测试；CI 还包含角色、静态、多风格、世界资产及安装分发检查。测试存在不等于本次全部通过。
4. 本地静态导入尚不支持资产项目 catalog 注册与单素材定向 retry，修改时需要新建请求；应与 Provider 生成路径区分。

## 本次验证

以下命令均成功退出：

```sh
cargo build --locked -p forge-cli --no-default-features
/Users/kartz/Development/Forge/target/debug/forge --version
/Users/kartz/Development/Forge/target/debug/forge plan --help
/Users/kartz/Development/Forge/target/debug/forge doctor --json
cargo test --locked -p core --test prepare_static_tests --test static_delivery_tests --test job_store_tests
cargo test --locked -p pack
```

结果：JobStore 8 项、本地静态输入 4 项、静态交付契约 3 项、Pack 37 项，共 52 项通过；1 项真实 Godot 静态安装测试按默认设置忽略。未执行完整 workspace 测试、实验 feature 矩阵、桌面测试、真实 Provider 请求、实际游戏交付或发布安装包验收。

本报告仅包含源码引用、构建身份与测试摘要，不包含凭据、消费者素材或完整任务目录。
