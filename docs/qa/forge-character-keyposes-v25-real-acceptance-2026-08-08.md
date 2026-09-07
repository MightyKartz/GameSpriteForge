# Forge Character Key Poses V2.5 真实 xAI 验收

日期：2026-08-08

## 结论

`topdown-keyposes@2.5.0` 完成 Ayla 四方向真实生成，但未通过游戏资产门槛。
Job 正确进入 `failed` / `character_motion_semantics_failed`，没有生成候选 Pack，
也没有进入 Godot 安装。

- Job：`5a529bd7-2cac-4f17-9c39-a33b80ca5025`
- 实际请求：17 次图片生成；无视频、视频编辑或私有文件上传
- 实际费用：11,900,000,000 cost ticks
- 唯一 Provider 重试：`walk_down:frame:0`
- 授权上限：32 次请求、25,600,000,000 cost ticks
- V2.5 来源合同核对：frame 1–3 均为 `edit_target, pose_structure`，没有上一帧参考

## 关键发现

V2.5 修复了 V2.4 的状态覆盖 bug 和上一帧串行依赖，但双色 Pose guide 被模型直接复制：

- `idle/frame-02`：两条腿出现大块青色/品红。
- `walk_up/frame-01`、`walk_up/frame-03`：腿部出现青色 guide 色块。
- `walk_right/frame-03`：两条大腿出现青色/品红色块。
- `walk_down/frame-00`：手臂和腿部出现青色/品红关节点。

这说明直接把人工着色的语义骨架作为 xAI 图片参考并不可靠。更严重的是，所有 16 个
`consistency-report.json` item 都被判为 `game_ready`，现有 `pose_structure_leak` 只比较
单一主 guide 色和整前景占比，漏掉了双色、局部大块和稀疏关节点复制。

## Motion gate

| 动画 | 结果 | 原因 |
| --- | --- | --- |
| `idle` | blocked | `foot_lobe_count_exceeded` |
| `walk_up` | blocked | 足部额外连通块、contact 过近、相位顺序错误 |
| `walk_right` | blocked | 足部额外连通块、上半身闪烁、contact 过近、相位和多样性不足 |
| `walk_down` | blocked | 步态能量不足、contact 过近、只有一个明显姿势 |

Motion gate 最终阻止了错误资产，但 guide 色泄漏本应在单帧 consistency 阶段更早阻断。

## 安全与交付

- JobStore 未发现凭据、Bearer、OAuth Token、Device Code 或临时媒体 URL。
- `exports` 为空；没有 Pack，也没有 Godot 安装。
- 原始 Provider 图片、净化帧、报告、WorkflowGraph 和 SHA-256 provenance 均已保留。

## 下一修复方向

1. 禁止将彩色骨架直接作为 xAI 图片参考；改用不携带可复制颜色的灰度/轮廓控制图，
   或先验证 xAI 是否真正支持结构而非外观参考。
2. 将 `pose_structure_leak` 改为多色集合、局部区域与连通块检测；任何 guide 专属颜色
   在角色前景内出现都应硬失败，不再依赖 4% 整图占比。
3. 在下一次付费前，用本次冻结产物做零费用回归，要求五个泄漏帧全部被 consistency gate 捕获。
4. 只对离线门禁通过后的 `walk_right` 单方向进行下一次真实探针，不直接重跑四方向。

机器摘要：`docs/qa/artifacts/forge-character-keyposes-v25-real-20260808/acceptance-summary.json`
