---
name: forge-use
description: Use the Forge CLI to create or process 2D game assets, plan provider usage, inspect and review jobs, retry selected outputs, validate Packs, and install assets into Godot. Use for Forge product operation; not for changing Forge source code or implementing gameplay.
---

# Forge 产品使用

把用户的素材需求变成可追溯的 Forge Job、可检查的 `.gsfpack`，以及任务要求的
Godot 资源。按当前 CLI 的实际能力执行，保留用户选定的素材、Provider 和交付范围。

## 按任务读取

| 任务 | 参考 |
| --- | --- |
| 发现能力、新项目、Style、图标/道具、已有素材加工 | [基础工作流](references/workflows.md) |
| 执行计划、预算、Job 状态、审核、定向重试、Pack/Godot 交付 | [执行与交付](references/jobs-and-delivery.md) |
| Subject、Character V2、Portrait、Grid、World、项目批量构建 | [可选工作流](references/optional-workflows.md)，只读相关段落 |

## 明确输入与环境

- 从对话和已有文件确定：素材类别与数量、风格/参考、尺寸或帧率等影响结果的约束，
  Forge 资产项目位置，以及是否需要安装到指定 Godot 项目。两种项目路径分别记录。
- 已有项目、Job、Style/Subject/Collection Lock 或 Pack 时先检查并复用。
  只询问影响执行的缺失信息；已有选择和同范围授权可以继续使用。
- 确定同一个 Forge 可执行文件，读取 `--version`、`--help`、`doctor --json`。
  有源码工作区时可以使用其已构建 CLI；缺少命令不等于应该自动修改源码或开启所有 feature。
- fixture 用于明确的演示和离线验证。生产请求不能通过偷偷换成 fixture、别的 Provider、
  模型或旧工作流来完成。

## 规划与执行

1. 根据输入选择生成、复用 Pack 或本地加工路线。已有媒体的加工不必先创建付费 Style。
   仅需要交付现有 Pack 时，直接进入校验与安装。
2. 使用目标命令的 `--plan-only` 或 `plan prepare-*` / `godot plan-install` 预检。
   计划会保存本地令牌，但不会生成媒体、创建执行 Job 或修改 Godot。
   工作流目录只是发现入口，以实际计划预检确认当前二进制能力。
3. 核对返回的 Provider、profile/model、workflow、预计与最大请求量、effects、到期时间。
   预计请求量不是实际账单，也不自动构成价格报价。真实执行遵守用户已有明确预算、
   持久授权与 CLI 环境门禁，见执行参考；只在缺少或需要扩大授权时提出具体范围。
4. 执行同一计划令牌。令牌一次性消费，输入变化或过期时重新规划；执行响应不确定时
   先查 Job，避免重复提交。短任务可 `--wait`，较长任务保存 Job ID 后查询状态。
5. 同时检查退出状态、JSON `ok` 和 Job 的 `lifecycle_state`。命令成功不代表素材完成；
   读取实际 artifacts、报告和 `next_actions` 决定后续步骤。

## 审核与修复

- 展示实际输出、contact sheet/播放预览和报告；按任务核对风格、透明边缘、方向、
  动作连续性与落点。不要仅根据结构校验宣称视觉通过。
- `awaiting_review` 需要审核动作；继续轮询不会完成审核。`job review` 会写决定并可能
  晋级资产。针对同一候选的已有批准可直接记录；生成预算授权不能充当视觉批准。
  工作流要求人工审核而尚无批准时，先展示具体候选和问题，再取得决定。
- 按失败证据选择最小 item/action/frame 或本地重处理范围，先检查重试计划。
  `auto` 和图节点 replay 不保证免费。保留父 Job/Pack 和未选中素材，核对 child 的
  来源、实际请求和授权余额；不降低质量门槛来制造通过。
- 预算耗尽、来源哈希变化、硬门禁失败或能力缺失时停止依赖该条件的执行，返回证据
  和可选下一步；在已授权范围内仍可继续独立的检查或准备工作。

## 验证与交付

- 从 Job artifacts 获取真实 Pack 路径，执行 `pack validate` 并读取 `data.valid`。
  方向验证、底图批准或其他中间 Job 可能成功但没有 Pack，按返回动作继续。
- 安装仅针对任务指定的 Godot 项目：先计划，再执行，最后检查安装 Job、资源、
  `forge_usage.json` 和项目登记。如需更换非 Forge-owned 目录，保留原目录并选择合法新目标。
- Godot 使用外部 PNG/atlas；检查 `.tres/.tscn` 无内嵌像素且小于 1 MiB。
  游戏脚本、碰撞和玩法扩展只在任务明确包含时处理。
- 交付时给出素材预览/文件链接、Job/Pack 路径、实际请求与费用证据、验证结果，
  以及尚待审核或未执行的范围。fixture、真实模型、人工审核和引擎验证分别记录。

## 使用边界

本 skill 不要求其他个人 skill 才能使用标准 CLI。专用角色动画的外部生成、原生帧
选环、人工批准及生产晋级可能有独立流程；需要时读取该流程自身文档，不将某个
历史 example、固定帧数或单动作交付泛化成通用产品能力。
修改 Forge 实现时改用仓库的 `forge-dev` 开发约定。
