# 本机 ComfyUI → Forge → Godot：多 Agent 素材生成实施计划

日期：2026-09-23。状态：待实施。源码基线：`d0fb39b`（main，v0.7.2）。本文所示新命令是**拟议接口**，当前发布版尚不支持。

## 目标与验收场景

用户在 Codex、Claude Code 或 DeepSeek Harness 的游戏项目对话中提出图片或视频素材需求。Agent 通过**同一个 Forge CLI 命令及 JSON 契约**连接用户已运行的本机 ComfyUI，得到源文件；Forge 保存生成证据、加工、检查、制作 Pack，并在完成所需检查后交付 Godot。Agent 返回可打开的预览、Job/Pack/安装回执和仍需处理的问题。

验收时至少覆盖：Qwen Image 2.1 透明图标、Qwen 图片编辑（后续能力）、MiniMax H3 首帧生成角色动作；两种生成类型都在真实 Godot 项目中完成保存资源验证。基线对照为 Agent 直接调用 ComfyUI、用现有脚本和 Godot 导入同一需求；记录任务总耗时、Agent token/工具调用、临时脚本、失败重试和人工介入。比较遵循 [现有协议](../qa/asset-delivery-comparison.md)，不以模型推理耗时替代完整任务耗时。

```
用户对话 → Agent → forge asset create → ComfyUI 本地 API → Forge Job 源文件
                               ↓
                    本地加工 / Pack / 预览 / 检查
                               ↓
                    明确审核 → Godot 安装 / 验证 / 回执
```

Forge 不分发 ComfyUI、模型权重或用户的工作流；模型选择及使用许可由用户负责。Forge 记录用户声明的来源与模型，不把生成成功、技术通过或用户声明视为视觉、音频、玩法或许可审核通过。已安装游戏的 Forge/Godot 锁不得静默更新。

## 已有基础与实际缺口

| 现有能力 | 实施含义 |
| --- | --- |
| `MediaGenerationProvider` 已定义图片生成/编辑、视频生成、轮询与取消；`packages/providers` 仅注册 xAI 和 fixture | 增加 ComfyUI adapter，不把节点逻辑放进 Core；仍须处理本地配置、能力探测与异步任务恢复。 |
| `generate icon-set/prop-set` 依赖 Provider、项目 Style Lock；`plan prepare-static/prepare-character` 使用本地源文件 | 初期复用本地加工与交付，避免为了单张素材强迫用户先建 Style Lock；之后再让 ComfyUI Provider 参与现有批量生成路径。 |
| Job/Plan、`asset` 库、Pack、Godot 事务与回执已有独立命令 | 新入口应组合现有阶段并返回可恢复的状态，不另造文件格式、安装事务或审核结论。 |
| `forge skill install` 把 `forge-use` 放入 `.agents/skills`；DeepSeek Harness 的项目 skill 也扫描该目录 | Codex 和已启用 skill 的 DeepSeek Harness 可共用受管来源；Claude Code 需另装 `.claude/skills` 入口，内容仍从同一内嵌指南生成。 |

ComfyUI 官方本机 API 提供 `POST /prompt`、`GET /history/{prompt_id}`、`GET /view`、`POST /upload/image`、`/ws` 和 `/object_info`。提交前必须使用**API 格式**工作流，明确可改写的输入及唯一结果节点；不能仅靠“本机装了某模型”猜节点。MiniMax H3 在 ComfyUI 有文生视频、图生视频及参考视频工作流；具体输出字段以用户安装的工作流和真实 `/history` 响应验证。

## 对外契约（拟议）

三个 Agent 均使用同一入口：

```text
forge provider configure --provider comfyui --profile local --config /absolute/path/comfy-profile.json --json
forge provider doctor --provider comfyui --profile local --json
forge asset create --input /absolute/path/request.json --wait --json
forge asset create --resume JOB_ID --review /absolute/path/review.json --wait --json
```

`configure` 保存用户显式选择的本机 profile；不能覆盖同名 profile 而不显示差异。`asset create` 只接收版本化请求文件，不拼接由 Agent 生成的 shell 参数。新增 `schemas/comfyui-workflow-profile.schema.json` 与 `schemas/local-generation-request.schema.json`；请求显式给出 `mediaKind`（image/video）、`workflowProfile`、提示词、可选参考媒体、目标 `assetId`、用途、素材库路径、加工配方、Godot 项目和安装目标；不指定的选项不得从旧项目、环境变量或某一 Agent 的偏好中默默推断。安装目标和审核要求在首次请求中冻结；`--resume` 不可更换目标或源文件，改动必须新建请求。文档提供图片、视频各一个可直接改写的请求示例。

回复继续遵守现有 `--json` 单一 envelope 和非零退出码规则，提供稳定的 `jobId`、`state`、`phase`、下一步动作、源媒体路径及 SHA-256、预览媒体清单、Pack 路径、安装 Job/回执及具体失败代码。长视频默认可异步启动，Agent 用 Job ID 跟踪；`--wait` 是便利选项，不能成为唯一可靠路径。每个返回路径必须是 Agent 可访问的本机绝对路径，并在 Windows/macOS/Linux 上一致表达其含义；对话界面是否能内嵌播放视频由客户端决定。

阶段顺序：

1. `doctor` 检查 ComfyUI 本机地址、工作流 API JSON、必需节点、模型与输出节点；`provider list` 保持无需网络的静态描述。缺失依赖给出具体修复项。
2. Forge 将用户请求写入 Job，保存 profile/workflow 文件哈希、模型标识、seed、参数及参考媒体哈希；生成前完成能力与资源预算检查。
3. Forge 把参考图上传到 ComfyUI，按 profile 中**显式声明**的节点路径填入参数，预先生成并持久化 `prompt_id` 后提交任务。轮询/事件只用于该 `prompt_id`；断线后先查 history，绝不因响应不确定而重复提交。
4. Forge 从明确的结果节点获取图片/视频，限制数量和大小，校验类型/可解码性，把字节复制进 Job 目录并哈希。不能把 ComfyUI 任意路径直接当作已验证结果，也不能取其他用户的最近一次输出。
5. 图片走现有静态准备、质量报告、Pack 与预览；视频先检查 fps、时长、帧范围，再走选定的动画准备流程。H3 的镜头运动、身份漂移和循环端点需要实际观看，不能仅凭技术检查判定可用于角色动画。
6. 对话中的 Agent 打开预览并记录可追溯审核。若请求明确授权安装且审核要求已满足，同一任务通过 `--resume` 继续执行现有 Godot 安装事务、原生验证和回执。否则返回 `awaiting_review`，保留已生成的源文件及 Job，避免重生成。

### Profile 与工作流

本机 profile 由用户显式创建或导入，至少记录 endpoint、允许的媒体任务、API 格式工作流文件、模型标识、可改写节点映射、结果节点/媒体类型、输入上限和超时。`forge provider doctor` 对照 `/object_info` 与用户工作流检查映射，并测试目标节点是否能产生预期类型；不能用节点 ID 的位置或显示名称猜用途。profile 默认只接受 loopback 地址；其他地址需要用户显式配置，并将其标为非本机服务。配置文件与模型文件不随 Forge 二进制打包，也不写入游戏仓库的通用模板。

Qwen T2I、Qwen 编辑、H3 T2V、H3 I2V 是不同 profile/能力。只宣告已通过真实工作流验证的能力；H3 first/last-frame 与 reference-to-video 也分别声明。`ProviderUsage` 记录实际任务/产物数量；本地运行的云端 API 费用字段为 `null`，不伪称为 0 GPU/电费。现有真实 Provider 守卫应区分本机无凭据任务与可收费在线 Provider，但仍保留显式的生成次数、时间/输出预算。

## P0：本地图片端到端（最小可用）

交付通用 ComfyUI HTTP adapter、profile/工作流校验、Qwen T2I 映射、版本化 `asset create` 图片请求、Job 证据及本地图片加工/Pack/预览/审核后 Godot 安装。先做单张透明 PNG 的 `icon_set`/`prop_set` 适配；用户可选传已生成的本地文件作为同一入口的 `source`，用于无 ComfyUI 时的对照和恢复。新入口不改变既有 `generate icon-set/prop-set` 及旧项目 Style Lock。

三个 Agent 在 P0 都要通过**明确点名 Forge**的同一自然语言任务完成图片生成和安装。`forge-use` 增补 ComfyUI 主题；Codex 与 DeepSeek Harness 复用 `.agents/skills`，Claude Code 从该受管内容安装 `.claude/skills/forge-use`，并给出各自的 `skill check`/手动 `forge guide` 兜底。独立测试 skill 是否被发现、是否选择本机 profile、是否调用同一个 `forge asset create`，不能把命令在 shell 中可运行当成自然语言发现成功。

验收：模拟 ComfyUI 的排队、成功、节点错误、断线、缺失输出、非图片内容和重复恢复；真实 Qwen 工作流在 Windows 上生成透明 PNG；Forge 不修改原图，准备后的画布/锚点/alpha 正确；在真实 Godot 项目验证已保存资源和回执。没有权重或 GPU 的 CI 只跑模拟契约与合成媒体。

## P1：本地视频与动画交付

增加 H3 T2V/I2V profile、参考图上传、长任务进度与恢复、视频输出取回/探测，以及与现有动画准备/预览/Pack/Godot 路径的连接。先把 I2V 用于一个明确的角色动作；T2V 可生成源视频，但只有通过现有角色/特效加工合同的任务才能宣称 Godot 动画交付。若现有 `prepare-character` 不支持某视频输出，报告可消费的源视频和明确缺口，不把视频文件本身称为已安装动画。H3 的原始视频及音轨完整保留；只有明确请求音频资产并经过 Forge 音频加工/审核时，才将音轨作为 Godot 音频交付。

处理 ComfyUI 重启、`prompt_id` 历史消失、Agent 会话结束和用户取消：可恢复时沿原任务继续；证据不足时进入 `needs_recovery`，仅在新的显式请求下重生成。取消只能中断 Forge 拥有且确认匹配的任务，不能误停共享 ComfyUI 上其他任务。记录真实 ComfyUI 版本、工作流哈希、模型文件标识和 FFmpeg/Godot 版本；无需上传私有视频素材到仓库。

验收：至少一个真实 H3 I2V 视频被解码、提帧、质量报告和预览；一个可用循环通过人工观看后进入真实 Godot 并检查播放；一个不合格循环被拒绝或标记待修；模拟长任务/重启/取消/超时均无静默重复生成。三 Agent 各完成一次同样的对话任务。

## P2：维护性、分发与真实项目比较

增加 Qwen 编辑和已验证的 H3 参考模式，工作流升级检查、profile 导入/导出（仅配置与哈希，不含模型/凭据/私有媒体）、批量生成预算和失败诊断。将英中 README、内嵌指南、Claude skill 安装说明、DeepSeek Harness 发现前提和真实示例保持一致；示例不承诺所有 ComfyUI 定制节点都兼容。

对至少两个真实游戏需求运行配对比较，覆盖图片和视频、首次交付与一次改版。只有完整任务结果优于 Agent 直接使用 ComfyUI + Godot 的路径，且三 Agent 的失败/人工介入没有明显恶化，才将该能力推荐为默认工作流；否则保持可选。公开发布前在 Windows 原生打包、macOS 本机和 CI 合成服务上分别验收安装后的 CLI，而非只用源码构建。

## PR 拆分与完成定义

| PR | 范围 | 独立可验收的结果 |
| --- | --- | --- |
| A | Profile/schema、`provider doctor`、模拟 ComfyUI 服务与传输 adapter | 本机服务/工作流诊断、提交/轮询/取回/失败契约；不声称已交付游戏素材。 |
| B | P0 图片 `asset create`、Job/Pack/Godot 连接及三 Agent 指引 | Qwen 图片在三个对话客户端通过同一命令完成真实 Godot 交付。 |
| C | P1 H3 视频、恢复/取消、动画处理 | 视频长任务和一个真实角色动作完成原生验证。 |
| D | P2 编辑/参考模式、升级与分发/比较报告 | 已验证的模式有精确能力声明及完整用户文档。 |

每个实现 PR 更新相应 `doctor --json` 能力标识、CLI JSON 契约测试、内嵌 `forge-use` 与中英文用户说明。Rust 改动跑格式、clippy 与聚焦单元/集成测试；安装变更跑 Windows/macOS 原生验证。最终验收报告分别给出源码 commit、构建 features、二进制 SHA-256、ComfyUI/工作流版本、模型配置、Godot 版本与真实媒体哈希。正式发布需另做发布预检、版本文档及打包验证。

## 边界与外部依据

- Forge 不自动安装 ComfyUI/custom nodes/模型，不托管生成服务，不把模型许可判定写成机器审核结论。用户自行安装并核对 [Qwen Image 2.1](https://huggingface.co/Qwen/Qwen-Image-2.1) 与 [MiniMax H3](https://huggingface.co/MiniMaxAI/MiniMax-H3) 的条款；Forge 不随包分发其权重。
- ComfyUI 的 [Server API](https://docs.comfy.org/development/comfyui-server/comms_routes) 与 [H3 工作流](https://docs.comfy.org/tutorials/video/minimax/minimax-h3) 是 adapter 的上游合同；不同定制工作流须逐一验证。
- Claude Code 的 [项目 skill 路径](https://code.claude.com/docs/en/skills) 与 DeepSeek Harness 的 `packages/skill/skill-filesystem/README.md` 决定发现方式；三者最终都只依赖 Forge CLI 的同一协议。
- 现有 [Forge 生成边界](model-neutral-media-generation.md)、[用户工作流](../../.agents/skills/forge-use/SKILL.md) 和 [产品优先级](../../PRODUCT.md) 继续约束实现：以真实游戏任务收益决定是否扩大投入。
