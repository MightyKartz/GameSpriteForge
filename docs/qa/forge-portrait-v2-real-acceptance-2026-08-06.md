# Forge Portrait V2 两阶段真实 xAI 验收

日期：2026-08-06  
最终结论：**未通过质量门槛；Forge 正确阻止 Pack 导出。**

后续说明：该结论准确记录 `portrait-local@1.0.0` 当时的自动门禁。表情感知的
`portrait-local@1.1.0` 修复、零费用 replay、用户审核与 Godot 交付结果见
[`forge-portrait-expression-aware-skin-gate-2026-08-06.md`](forge-portrait-expression-aware-skin-gate-2026-08-06.md)。原报告不覆盖或改写。

机器可读结果：
[`summary.json`](artifacts/forge-portrait-v2-real-20260806/summary.json)

原始可视结果：
[`contact-sheet.png`](../../generated-assets/forge-portrait-v2-real-20260806/jobs/33f2ac78-4f63-451b-871b-18b6da6b8c0f/contact-sheet.png)

## 范围与费用边界

本次仅验收 Ayla 全身 Portrait V2，不生成角色动画、图标、道具或其他资产：

- Base 授权：仅 `neutral`，最多 2 次、2,000,000,000 cost ticks；
- Expressions 授权：仅 `happy`、`angry`、`hurt`、`surprised`，每项最多 2 次，
  总计最多 8 次、8,000,000,000 cost ticks；
- 总授权：最多 10 次、约 1.00 美元；
- 实际使用：7 次、4,300,000,000 cost ticks，即约 0.43 美元。

所有请求使用同一 Provider/profile/model：xAI、`default`、
`grok-imagine-image-quality`。Neutral 使用默认 `subject-style@1.0.0` reference policy。

## Base 阶段

Job：`c60b229d-736f-437e-9636-3e1e618b3f2d`

Neutral 第一次生成即通过硬门禁：

- palette overlap：0.9943；
- foreground scale ratio：1.0004；
- anchor drift：0.5 px；
- perceptual similarity：0.9844；
- 双腿、双脚、绿色兜帽/斗篷、橙色围巾、装备和画布边距完整；
- 实际 1 次请求、约 0.07 美元。

经原尺寸人工检查后写入哈希绑定的 `portrait-base-approval@1.0.0`。Base 阶段没有
导出不完整 Pack。

## Expressions 阶段

Job：`33f2ac78-4f63-451b-871b-18b6da6b8c0f`

每个表情均从获批 neutral 独立派生。确定性 face composite 使所有最终图片的脸部范围外
变化率保持为 0，因此围巾、斗篷、服装、身体、双腿和双脚没有继续漂移。

逐项结果：

- `happy`：2 次，`awaiting_review`。视觉上基本可用，但 skin-tone ΔE 为 9.26，保护肤色
  区域异常比例为 7.81%，没有自动晋级；
- `angry`：1 次，`game_ready`；
- `hurt`：1 次，`game_ready`；
- `surprised`：2 次，`regenerate`。skin-tone ΔE 为 14.57，保护肤色区域异常比例
  29.52%，其中严重异常 20.53%。原图和最终图均可观察到整张脸变亮，不是单纯阈值误报。

由于 `surprised` 已达到单项两次上限且仍为 `regenerate`，Forge 未允许人工强制接受，
没有导出 `.gsfpack`，也没有继续 Godot 安装。这符合安全合同。

## 结论

两阶段设计和 PortraitBaseLock 已解决此前最明显的全身构图、围巾颜色、服装、身体和腿部
一致性问题；当前真实 blocker 已收敛为“大幅表情下的脸部肤色与局部皮肤纹理保持”。

下一步不应降低阈值。建议针对 `surprised` 实施更受约束的面部编辑或确定性肤色保持，再用
新的独立授权只重试该项。当前 Job 和 Pack 均保持未通过状态，错误导出数为 0。

## 安全审计

- JobStore 中未发现 API Key、OAuth Token、Device Code 或 Authorization header；
- 未发现临时媒体 URL；
- 请求账本仅含非敏感目标、模型、Job ID、状态和费用 ticks。
