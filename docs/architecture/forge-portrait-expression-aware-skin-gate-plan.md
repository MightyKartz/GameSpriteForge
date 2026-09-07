# Forge Portrait 表情感知肤色门禁修改计划

状态：已实施  
日期：2026-08-06

## 目标

降低大幅表情对肤色指标的误触发，但不放宽身体像素、裁切、Alpha、多主体、身份漂移和
严重脸部伪影硬门禁。修改必须能对现有真实 xAI Job 做零 Provider 请求 replay。

## 问题

`portrait-local@1.0.0` 的肤色均值会比较 neutral 中的所有皮肤像素，即使这些位置在
`surprised` 中合理地变成扩大后的眼睛或张开的嘴。它也不要求候选像素仍然是皮肤，导致
预期表情结构被计入肤色 ΔE。普通暗化比例超过 8% 又会直接 `regenerate`，无法让人工审核
区分可接受的整体表情变化与真正涂鸦。

## `portrait-local@1.1.0` 合同

1. 为 `happy`、`angry`、`hurt`、`surprised` 定义确定性的眼眉/嘴部排除区；大幅表情使用
   更宽但仍受 Face Scope 限制的区域。
2. 肤色只比较 neutral 与候选图中都满足皮肤启发式的对应像素。
3. 对匹配 ΔE 排序，去掉最高和最低各 10%，使用截尾均值。
4. 新增 `skinCorrespondenceRatio`：候选图仍为皮肤的数量除以 neutral 合格皮肤数量。
   `<0.65` 为 `regenerate`，`0.65–0.80` 为 `awaiting_review`，防止通过丢弃异常像素伪造
   低色差。
5. 稳健肤色 ΔE：`>18` 为 `regenerate`，`10–18` 为 `awaiting_review`。
6. 保护肤色区域的比例判定同时要求最小异常像素数，避免小图中少数像素放大百分比：
   普通暗化 `>25%` 且至少 32 px，或严重暗化 `>4%` 且至少 20 px，才进入
   `regenerate`；普通暗化 `>5%` 且至少 24 px，或严重暗化 `>2.5%` 且至少 12 px，进入
   `awaiting_review`。
7. `outputOutsideFaceChangedRatio > 0`、严重身份/边缘漂移、缺少有效表情等既有硬门禁不变。
8. 只有灰区能由 `forge job review --accept` 晋级；`regenerate`/`blocked` 继续禁止强制导出。

Schema 同时接受 `portrait-local@1.0.0` 与 `portrait-local@1.1.0`，新字段保持可选以读取旧
Pack；新生成报告固定写入 1.1。

## 验收

- Synthetic：扩大眼睛/嘴巴不得被计为肤色漂移；非表情区换成非肤色必须被对应关系门禁
  拦截；深色脸颊涂鸦仍必须 `regenerate`。
- Fixture：Stage 3、Pack、Godot provenance 和 CLI schema 全部使用/识别 1.1，同时兼容
  1.0。
- 真实冻结样本：从 Expressions Job 仅重跑 `consistency`，Provider 请求必须为 0；
  `surprised` 只能降为 `awaiting_review`，不得自动 `game_ready`。
- 用户原尺寸审核接受后才允许导出 Pack 和安装 Godot。
