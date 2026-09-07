# Forge Top-down Key Poses V2.5 隔离与终态修复计划

日期：2026-08-08

## 真实问题

V2.4 真实 xAI Job `63a91d35-bccb-4e50-b15f-b9520517ca80` 完成 18 次图片请求后被
motion gate 拦截。大图与确定性报告共同证明：上一帧图片参考压过了新的 Pose guide，
导致 `walk_right` 几乎不动、`walk_up` 后半周期重复、`walk_down` 主要只摆动上半身，
`idle` 的最后一帧还产生串行漂移。

同时，Pack 层已经写入 `character_motion_semantics_failed`，外层关键帧逻辑却又把 Job
覆盖为 `keyframe_review_required`，错误暴露了可以人工审核的 next action。

## 修复合同

- 保留 `topdown-keyposes@2.4.0`，不改变历史缓存、Job 或 Provider provenance。
- 新增 `topdown-keyposes@2.5.0`，四方向仍为每动作 4 帧、6 FPS、预计 16/最多 32 次图片请求。
- frame 0 建立 DirectionLock；frame 1–3 只引用 DirectionLock 和当前 Pose guide，禁止上一帧图片参考。
- `topdown-keyposes@2.1.0` guide 用青色表示角色左肢、品红表示角色右肢，并扩大 contact/passing 足部间距。
- Provider manifest 和 WorkflowGraph 记录 `direction_lock_plus_pose_only`；frame 1–3 只依赖 frame 0。
- motion hard failure 保持 `failed`、`character_motion_semantics_failed`，next actions 不包含 review。
- motion 报告新增兼容可选的 `recommendedRetryFrames`，为后续定向付费重试提供范围。

## 验收

- V2.4 仍可执行并保持原三参考角色。
- V2.5 frame 1–3 的 Provider references 固定为 `edit_target, pose_structure`。
- fixture 完整四方向可以导出 Pack；静止关键姿势 fixture 必须失败且不得产生 candidate Pack。
- 使用真实 V2.4 Job 的本地规范化帧做零费用复审，验证推荐重试帧与大图结论一致。
- 所有真实 Provider 调用继续需要独立次数与费用授权；本修复阶段不调用 xAI。
