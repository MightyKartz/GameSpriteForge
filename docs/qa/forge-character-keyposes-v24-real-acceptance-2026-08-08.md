# Forge Character Key Poses V2.4 真实 xAI 验收

日期：2026-08-08

## 结论

`topdown-keyposes@2.4.0` 完成了 Ayla 四方向的真实图片生成，但未通过运动语义硬门禁，
因此没有导出 Pack，也没有进入 Godot 安装。真实门槛结论为 **未通过**。

- Job：`63a91d35-bccb-4e50-b15f-b9520517ca80`
- Provider / model：`xai` / `grok-imagine-image-quality`
- 实际请求：18 张图片；无视频、视频编辑或私有文件上传
- 实际费用：14,000,000,000 cost ticks
- 定向重试：`idle:frame:3`、`walk_down:frame:3` 各一次
- 授权上限：32 次请求、25,600,000,000 cost ticks

## 运动语义结果

| 动画 | 结论 | 主要原因 |
| --- | --- | --- |
| `idle` | blocked | `foot_lobe_count_exceeded`、`stable_upper_body_flicker` |
| `walk_up` | blocked | `walk_phase_order_invalid`、`walk_pose_diversity_missing` |
| `walk_right` | blocked | 几乎无步态、contact 过近、相位与姿势多样性不足、脚部连通块异常 |
| `walk_down` | blocked | `walk_pose_diversity_missing` |

人工大图检查与门禁结论一致：`walk_right` 四帧几乎相同；`walk_up` 后半周期重复；
`walk_down` 主要变化集中在手臂和斗篷而非足部；`idle` 最后一帧产生明显姿态/腿部漂移。

## 交付与安全

- `character-motion-semantics-report.json` 的总体 verdict 为 `blocked`。
- Pack export 被 `every animation must be game_ready` 阻断，exports 目录为空。
- 因无合格 Pack，Godot 安装按协议跳过。
- JobStore 文本扫描未发现凭据、Bearer、OAuth Token、Device Code 或临时媒体 URL。
- 生成 Job 本身显示 `awaiting_review`，但 motion hard failures 不可人工越过；后续应修正
  顶层错误摘要和 next actions，避免把不可审核的运动失败误导成普通 consistency 灰区。

机器摘要：
`docs/qa/artifacts/forge-character-keyposes-v24-real-20260808/acceptance-summary.json`
