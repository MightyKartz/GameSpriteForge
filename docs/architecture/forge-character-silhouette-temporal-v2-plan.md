# Forge Character 轮廓独立验收 V2

日期：2026-08-07

## 决策

旧流程先用首帧 Alpha 重塑后续帧，再在相同区域评分，属于评估泄漏。V2 固定为：

```text
归一化但未做时间修复的 source frames
→ 独立 source silhouette report
→ 仅 1px 边界小缺口修复（带像素预算）
→ 独立 post-repair silhouette report
→ 两份报告与修复预算全部通过
→ Pack
```

生产路径不再运行 `rigid-head-alpha@1.0.0`，也不允许复制首帧 RGB/Alpha 来重塑角色。

## `silhouette-temporal@2.0.0`

- 对齐只使用游戏根锚点：身体中心和脚底；禁止用正在接受检测的头顶边缘参与对齐。
- 对头部/躯干持久核心检查 Mask IoU、双向轮廓距离、边缘支持率和时间占用不确定区。
- 对全身边缘输出诊断率；`idle` 无动作借口，因此全身边缘也是硬门禁；walk 的四肢、披风和装备允许正常运动，结构门禁仍由持久核心负责。
- 对匹配边缘检查 RGB 闪烁，避免透明底上不明显、深色或绿色背景上明显的彩边。
- source 与 post-repair 分别写报告。source 结构失败后，post-repair 不能把 Job 提升为 `game_ready`。

## `boundary-alpha-repair@1.0.0`

- 只删除孤立单帧边缘点，或填补被至少 7 个当前帧邻居包围的单像素孔。
- 填充颜色取当前帧 3×3 邻域中位色，不跨帧复制 RGB。
- 最大边界距离 1px；单帧最多 64 像素且不超过前景的 0.25%，整组不超过 0.15%。
- 超预算为硬失败，不能人工接受。

## 运行时视觉证据

xAI 等真实 Provider 为每个方向输出：

- 1x checkerboard / chroma-green / dark 动画 GIF；
- 2x、4x 同背景的逐帧 contact sheet；
- `previews/debug/manifest.json` 保存背景、缩放、源帧 SHA-256 和预览 SHA-256。

Fixture 保留 1x 动画以控制 CI 时长，并由独立单元/合同测试覆盖缩放和 manifest 合同。

## 发布条件

- source、post-repair、repair budget、语义、循环和原有一致性门禁全部通过。
- 任一硬失败均不得生成 Pack 或覆盖 Godot。
- 本地一致性复验不得产生 Provider 请求。
