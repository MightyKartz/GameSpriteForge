# Ayla `topdown-video@2.0.0` 四方向真实 xAI 验收

日期：2026-08-09  
结论：**未通过，安全失败（failed safe）**

## 授权与实际消耗

- 授权：最多 16 次媒体请求、55,000,000,000 cost ticks（约 5.50 美元）。
- 实际：10 次媒体请求、27,000,000,000 ticks（约 2.70 美元）。
- 请求构成：6 次 `edit_image`、4 次 `generate_video`。
- 未调用：Subject、Style、`edit_video`、私有文件上传或其他资产生成。
- OAuth 凭据保存在 owner-only 文件凭据库；普通日志、Job、Pack 与 QA 文件不含 Token。

## 逐方向结果

| 方向 | 图片 | 视频 | 结论 |
|---|---:|---:|---|
| `idle` | 1 | 1 | 阻断。静帧和视频出现未声明法杖；Spec 明确为 `equipment: none`。 |
| `walk_up` | 1 | 1 | 静帧方向与构图通过，原始视频已生成；因后续方向失败，没有完成整包下游质量验收。 |
| `walk_right` | 2 | 0 | 阻断。两张静帧均出现未声明武器，且确定性预检发现脚锚漂移；第一次还存在尺度漂移。没有浪费视频请求。 |
| `walk_down` | 2 | 2 | 静帧预检通过；两次视频的方向/发光语义通过，但都被 `silhouette-temporal@2.0.0` 硬门禁阻断。 |

`walk_down` 两次视频共同失败原因为：`upper_body_mask_flicker`、`upper_body_contour_drift`、`persistent_core_boundary_drift`、`unsupported_core_edge_drift`。第二次仍有 18.97% 的身体宽度变化，最大轮廓距离 5.19px，不能作为稳定游戏动画导出。

## 本次修复验证

1. 将绘制型脸部的兼容检测限制在 canonical、`idle` 与 `walk_down` 正面场景；后视 `walk_up` 仍使用严格身份规则，避免把围巾/兜帽误判为脸。
2. `equipment: none` 的方向静帧提示显式禁止 staff、wand、spear、weapon、tool、glow 与 detached object。
3. 修复了失败 Job 只有已付费静帧时，`--stage video` 错误回退到 still+video 的问题；新的 Job 只复用静帧并调用 image-to-video。
4. validation-only 的显式 video 重试禁用 `edit_video`；第二次仍从同一静帧重新 image-to-video，符合本次授权。
5. 第二次 `walk_down` 视频前后，账本只新增 `walk_down:video`，未新增 `walk_down:still`。

## 交付门禁

- `.gsfpack`：未生成。
- Godot 安装：未执行。四方向不完整且存在硬缺陷时安装会制造错误资产。
- 安全扫描：4 个相关 Job 目录中，Token、Bearer、Device Code、Client Secret 模式命中文件数为 0。
- 机器摘要：[`summary.json`](artifacts/forge-topdown-video-v2-real-20260809/summary.json)

## 预览顺序

`direction-stills.png` 从左到右、从上到下为：`idle`、`walk_up`、`walk_right` 第一次、`walk_right` 第二次、`walk_down` 第一次、`walk_down` 第二次。

- [`direction-stills.png`](artifacts/forge-topdown-video-v2-real-20260809/previews/direction-stills.png)
- [`idle-raw-video-contact.png`](artifacts/forge-topdown-video-v2-real-20260809/previews/idle-raw-video-contact.png)
- [`walk_up-raw-video-contact.png`](artifacts/forge-topdown-video-v2-real-20260809/previews/walk_up-raw-video-contact.png)
- [`walk_down-attempt-1.gif`](artifacts/forge-topdown-video-v2-real-20260809/previews/walk_down-attempt-1.gif)
- [`walk_down-attempt-2.gif`](artifacts/forge-topdown-video-v2-real-20260809/previews/walk_down-attempt-2.gif)

## 后续建议

当前主要阻断不是抽帧阈值，而是 xAI 视频在“保持严格正面视角、固定上身轮廓、完整步态”三项约束间发生漂移。下一次真实费用不应继续盲重试。应先把 `equipment: none` 做成静帧硬门禁，再增加视频前的方向/姿态适配检查；只有静帧完全合格后，允许视频生成。视频路径继续保留，但需在冻结样本上验证 `walk_down` 正面步态和上身轮廓稳定性后再授权真实重跑。
