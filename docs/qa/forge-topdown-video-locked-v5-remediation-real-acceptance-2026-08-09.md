# Forge `topdown-video-locked@5.0.0` 修复后真实 xAI 验收

日期：2026-08-09

结果：**FAILED SAFE**。四方向均已真实生成，三个 walk 又各完成一次定向重试；`idle` 达到 `game_ready`，但三个 walk 仍未同时通过方向、运动语义和动作一致性硬门禁，因此没有导出 Pack，也没有安装到 Godot。

机器摘要：[summary.json](artifacts/forge-topdown-video-locked-v5-remediation-real-20260809/summary.json)

## 授权与费用

- 复用 Ayla 既有 SubjectLock、StyleLock 和 DirectionLock。
- 允许目标仅为 `idle:video`、`walk_up:video`、`walk_right:video`、`walk_down:video`。
- 授权上限：8 次请求、48,000,000,000 ticks。
- 实际使用：7 次视频请求、23,100,000,000 ticks（约 2.31 美元）。
- 没有生成 Subject、Style、DirectionLock、图片或其他资产。

## 最终结果

| 动作 | 方向 | 运动语义 | 循环 | 动作一致性 | 结论 |
| --- | --- | --- | --- | --- | --- |
| `idle` | game_ready | game_ready | game_ready | game_ready | 可用 |
| `walk_up` | blocked | blocked | game_ready | awaiting_review | 背向过程中检测到脸、脚部多轮廓及上身闪动 |
| `walk_right` | blocked | blocked | game_ready | regenerate | 顶部对齐漂移、上身闪动、步态相位和锚点漂移 |
| `walk_down` | blocked | blocked | game_ready | regenerate | 正面方向漂移、脚部多轮廓、上身闪动、步态相位和锚点漂移 |

V5 修复本身在真实数据上生效：`idle` 被限制在初始锚点窗口并通过；所有动作都选出了闭合循环；最终帧一致性报告会随重生成结果变化；脚底 chroma 清理的所有选中帧均为 `game_ready`，没有残留标记。失败来自视频内容的方向与运动稳定性，而不是 Pack 或 Godot 交付阶段。

## 证据

- 首轮 Job：`generated-assets/forge-topdown-video-locked-v5-remediation-real-20260809/jobs/09ac9e42-c6dc-4e2f-b709-2551657f2cda`
- 定向重试 Job：`generated-assets/forge-topdown-video-locked-v5-remediation-real-20260809/jobs/605d08af-c5a3-4a32-98b7-849332dcce7d`
- 最终 contact sheet：`generated-assets/forge-topdown-video-locked-v5-remediation-real-20260809/jobs/605d08af-c5a3-4a32-98b7-849332dcce7d/contact-sheet.png`
- 四个最终 GIF 位于定向重试 Job 的 `previews/debug/*-dark-1x.gif`。

## 安全与交付

- 凭据、Token、Device Code、API Key 和 Authorization header 标记：0 命中。
- 临时媒体 URL：0 命中。
- Pack 数量：0；Godot 安装未执行。
- 授权仍剩 1 次 `idle:video` 容量，但本次任务已停止，不会继续调用 Provider。
