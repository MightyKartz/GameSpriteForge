# Sword 使用经验对 Forge CLI 的反馈

日期：2026-09-12。Forge 基线：`4a870811c43ca4aa087e8f2a4113fca699b7b838`。Sword 本地主项目：`/Users/kartz/Development/Games/Sword`，分支 `codex/pr61-run-cultivation`，提交 `a8b93f4a753a84dbbd68365eac5258a9927617dd`。

本轮分析开发记录、导入脚本、运行资源及相关个人技能，并在隔离临时目录复现两个问题；未修改 Sword、调用真实 Provider、重新生成美术、导入正式游戏或升级消费者锁。本文列出待办，不表示已实现。精简复现结果见 [findings.json](artifacts/forge-sword-feedback-20260912/findings.json)。

## 当前消费情况与已完成的回补

Sword 静态/动画两锁都指向同一已验收 v0.3.0 发布二进制，提交 `60904c02a52f6831626ebbc4475f9c25db162931`。本轮只读核对确认实际 SHA 与两锁一致，doctor 为干净 release、默认 features。当前安装 17 个 Pack：4 个静态集、13 个动画 Pack。不能把旧回执中的旧生产者误当成尚未升级的工具入口。

开发过程从首批透明 PNG、主角/游魂动画，扩展到法术、场景、法器和烟尘效果。初期的低 alpha 残留导致主体缩小、落地位置不准，推动了静态 alpha bounds/edge padding；共同画布、锚点、逐帧时长和不能整除的图集推动了 preserve_source 与整图无损变换。这些已在默认 CLI 实现。Sword 的 T01 迁移记录证明当时 12 Packs / 20 PNG / 48 帧的原生资源、RGBA 和 metadata 严格一致；该历史验收不代表后来新增的所有素材。

本轮依据包括 Sword `docs/pr/T01-forge-v030-toolchain.md:13–28`、`asset-specs/expansion-v1/README.md`、`docs/pr/S01-animation-feedback.md` 和后续法术/沙潮 QA，以及 Forge 既有 Sword 审计。应保留现有回归，不重复开发这些已支持的合同，也不为了读取新版 guide 改动 Sword 的固定版本。

## P1：优先修复的可靠性与入口问题

### 1. 安装失败的恢复没有覆盖所有错误分支（已复现）

`packages/core/src/automation/runner.rs:5014–5028` 在验证 Godot 版本之前备份并删除旧目标。随后 Godot 定位、版本验证、进程启动、日志写入和部分资源验证通过 `?` 直接退出，未统一执行恢复；只有若干显式分支回滚。

隔离复现：使用真实 Godot 首次安装有效动画 Pack；第二次指定仅返回 `4.5.0.fixture` 的测试 Godot。计划仍成功建立，执行返回 `automation_failed`，但目标从 6 文件变为 1 文件，丢失 ownership、scene、SpriteFrames、usage 和导入元数据。旧内容在任务备份中，目标未自动恢复；无 ownership 还会妨碍后续正常覆盖。

建议先把版本/依赖预检移到目标变更之前，再以统一事务守卫覆盖所有失败路径；验证完整暂存结果后替换。用失败注入验证不支持版本、spawn/写入失败、资源验证失败、取消等情况保持旧目标与登记一致。不能只修此单一版本检查分支。

### 2. 单动作非循环动画未使用已有非循环质检逻辑（已复现）

`runner.rs:4858` 调用 `compute_quality_report`，未传递 `metadata.loop`。`quality/metrics.rs:58–74` 已存在 `compute_quality_report_for_animation`，会对非循环关闭首尾循环判定。

隔离输入：两个共同画布/共同底点的扩张帧，`loop:false`、`preserve_source`、`requireGameReady:true`。结果仍为 `awaiting_review`、`needs_cleanup`，唯一建议是 `trim_loop_range`；底点与横向中心漂移均为 0。这与一次性爆发效果的语义冲突。

建议单动作路径使用已有 animation-aware helper，并补 CLI 运行级回归。Sword fire/frost/lightning/impact 等单动作非循环素材直接对应此类用法。不要将所有质检阈值一起放宽。

### 3. 显式稳定 asset key 没有用于默认安装目录（已核对代码与历史案例）

`packages/cli/src/main.rs:1336–1349` 仍从 Pack 文件名推导省略的 target，未优先采用显式 `asset_key`。Sword `docs/pr/S01-animation-feedback.md:19` 记录中文名字默认同落 `pack`，后续全部 importer 显式指定稳定目录并验证 `usage.assetKey`。

建议显式 target 优先；未给 target 时使用经过验证的显式 asset key，再考虑可用的 Pack 稳定 ID。保留 ownership 和冲突保护，补中文名、同名 Pack、不同稳定 ID 的测试。已确认的是默认目录易冲突，不是无条件覆盖其他资源。

### 4. 旧个人动画技能与当前 CLI 主线不兼容（文件依赖检查）

`/Users/kartz/.codex/skills/forge-character-animation/scripts/forge_character_animation.py` 的前检仅要求 Cargo 与安装脚本存在，但其生产命令引用当前仓库没有的 6 个 Rust examples：`extract_locked_video_source`、`select_locked_video_cycle`、`select_locked_idle_cycle`、`replay_locked_gait_diagnostic`、`replay_locked_idle_diagnostic`、`create_pending_animation_review`。

该问题不影响 Sword 现有的 v0.3.0 PNG 导入脚本。建议技能默认走已验证 CLI capabilities；源码专用路线必须检查实际 example/固定历史 checkout，并清楚标注范围。不要为满足旧技能恢复整批废弃源码，也不要把历史候选/晋级能力描述成默认 CLI 功能。

## P2：将重复的消费端工作沉淀为产品能力

| 改进 | Sword 的实际证据 | 建议边界 |
| --- | --- | --- |
| 可移出 Job store 的统一回执及只读验证 | 6 个 importer 共 968 行；loadout/bog/sand 重复 prepare-only、Pack 全文件清单、install-prepared、request/build/source/installed hashes 核对 | 组合已有 Job/report/usage，关联 prepare 与 install，保留 source-transform/quality；追加独立 review，不覆盖历史生产者。命令及 schema 需另行设计 |
| 请求绑定已审核源图 | 每套 wrapper 在计划前校验批准 SHA；现有 Plan 指纹只能保证计划建立后未变化 | 增加兼容的可选 expected-source-hash 或输入锁；保留源图 hash 与归一化 hash 区别 |
| 特效质检类型 | bog/sand 明确用 requireGameReady:false；烟尘自然翻卷被推荐 adjust_anchor | 显式区分角色脚点和特效形变；给出 alpha 边界、相邻/首尾像素、亮度诊断。透明消散帧需明确语义，不将正常终帧当丢失素材，也不取消真正损坏的检查 |
| 已安装资源审计 | 动画 importer 167–197 行重复核对类型、尺寸、锚点、采样、动作时序和纹理 SHA | 在已有安装验证上增加事后只读检查，发现手改/漂移；可选真实 Godot 加载并与原 Pack 比对 |
| 源图预检与预览说明 | 生成结果实际尺寸不一定等于提示词；存在跨格身体、烘焙棋盘格、低 alpha 残留与正常透明 RGB | 输出实测尺寸、alpha、多阈值 bounds、格线触边、双色底预览；诊断不能冒充美术批准。GIF 应披露量化误差，原生 timing 仍权威 |

沙潮记录特别说明：8 fps 原生每帧 125ms、循环 1000ms；GIF 量化为每帧 130ms、循环 1040ms。这是预览限制，不是 Godot timing 丢失。现有 guide 已说明非均匀时长不能以 GIF 为准，可进一步提供实际预览时长/误差。

## 新沉淀技能可带回 Forge 的内容

`godot-roguelike-iteration/references/validation-delivery.md:7–10,26–34` 的经验适合补入 forge-use/forge-dev：

- Godot 成功判断同时依据实际检查结果和明确解析/脚本错误；不将普通 warning 一概当失败。
- 安装、导入、导出等共享 Godot 缓存的操作应序列化。
- 分开记录结构通过、安装成功、实际渲染、人工接受及真机验证。源/派生 hash、帧数、时序、运行引用可关联。
- 给基于外部模拟时间采样 SpriteFrames 的通用示例，展示 pause/non-loop/per-frame timing；实际战斗时钟与规则仍归游戏。

当前 Forge 安装 runner 主要依据进程退出和产物路径判断，`install_forge_pack.gd` 的 `_fail()` 只调用 `quit(1)`，调用者并非始终停止，顶层还可能继续 PASS/quit(0)。这是源码确认的验证协议风险；本轮没有将其完整误通过链当成已执行复现。应与统一事务一起补结构化完成结果和负例测试。前一轮清理已修好独立 smoke 的同类问题，默认安装器还需单独处理。

不应搬进 Forge：Sword 的成长/存档、奖励、危险区域、沙墙安全缺口、actor/action 映射、显示高度、手机安装备份及音频许可流程。Godot 技能中的这些经验继续属于游戏开发。`effect-image-asset-pipeline` 的动态语义分层与实际游戏尺度审核可作为指南原则，不能成为自动批准画面的算法。

## 实施顺序

1. 先修安装事务、单动作非循环判定、默认 target 和旧技能能力探测，补隔离负例与现有消费合同回归。
2. 再产品化回执/来源锁/已安装资源验证，减少消费端重复代码。
3. 最后分开设计特效质检、预检和预览提示，以角色与特效夹具检验语义。

升级 Sword 必须是独立迁移：先用确切新二进制隔离重放并比对，再更新两锁；本轮不更改固定 v0.3.0 和既有素材回执。历史批准不能自动变成新输出的批准。
