# Forge Character 四方向真实 xAI 重生成验收

日期：2026-08-08  
结论：**四个方向均已生成；仅 `idle` 通过独立轮廓门禁，Pack 与 Godot 交付正确阻断**

## 授权与范围

- Authorization：`character-four-directions-20260808`
- 来源 lineage：`323a1c5a-8af3-467a-95de-b093d5004edd`
- 允许目标：`idle`、`walk_up`、`walk_right`、`walk_down` 的 still/video；每目标最多 2 次。
- 总上限：16 请求、55,000,000,000 cost ticks。
- 禁止目标：Subject Reference 和所有其他资产。
- 最终零费用组合 Job：`3ab5034e-be5b-434b-a9cc-49408212f3e8`。

## 请求与费用

- 四个方向各完成 1 次图片编辑和 1 次视频生成，共 8 个实际媒体生成请求。
- `walk_up`、`walk_right`、`walk_down` 各额外发生 1 次私有文件上传，共计 11 条账本记录。
- 观察费用：25,600,000,000 ticks，约 2.56 美元。
- 保守预留总额：55,000,000,000 ticks，没有超过用户授权。
- 没有 Subject Reference 请求，没有 Provider/model 切换。

三个 walk 视频的首次本地诊断都建议视频编辑。当前授权实现把私有文件上传计入同一个 `*:video` 的两次上限，因此 `generate_video + private_file_upload` 已用尽额度，真正的 `edit_video` 在网络请求前被阻止。Forge 随后只复用已经落地、哈希校验通过的首次视频进行零费用 `loop` 重放，没有扩大授权。

## 最终质量结果

| 动画 | 语义 | source 轮廓 | 最小核心 IoU | 最大轮廓距离 | 核心不确定率 | 无支持核心边缘 |
|---|---|---|---:|---:|---:|---:|
| `idle` | game_ready | game_ready | 0.9653 | 1.54px | 0.0460 | 0.0747 |
| `walk_up` | game_ready | blocked | 0.8898 | 4.82px | 0.1260 | 0.7956 |
| `walk_right` | blocked | blocked | 0.8493 | 4.64px | 0.2183 | 0.7383 |
| `walk_down` | game_ready | blocked | 0.9254 | 4.39px | 0.1035 | 0.7746 |

`walk_right` 另有 `body_scale_drift` 和 `body_top_alignment_drift`。三个 walk 的 source 轮廓失败都不能通过 Alpha 修复或人工审核强制导出。

`boundary-alpha-repair@1.0.0` 只修改 31 个像素，最大距离 1px，结论 `game_ready`；它没有改变 source 门禁结论。

## 4× 人工验收

- `idle`：头肩、脸部、躯干、法杖与整体构图稳定。
- `walk_up`：后向语义基本成立，但披风、围巾、躯干宽度和手臂轮廓发生明显帧间重绘。
- `walk_right`：身体尺度、顶部位置、侧向姿态和手臂/披风形状明显变化。
- `walk_down`：脸型、围巾尾部、法杖相对位置和躯干轮廓存在帧间变化。

这些问题属于模型生成的角色几何漂移，不是透明边缘的 1px 抗锯齿问题。

## 交付与安全

- 最终 Job：`awaiting_review`，`exports/` 中 0 个文件。
- 没有生成 `.gsfpack`，没有执行 Godot 安装。
- 项目 Catalog 中 `validation-ranger` 继续为 `gameReady=false`、`quarantined`。
- 生成 36 个多背景/多尺度预览和 1 个哈希 manifest，共 20 MiB。
- Job、授权清单和账本的 Token、Authorization header、API Key 与临时媒体 URL 扫描通过。

## 机器证据

- Authorization manifest SHA-256：`4f92d445e6e34eda6ca7cc44dff63f03e5cefae5c6d42b29e3b3b03a77b22e3d`
- Request ledger SHA-256：`300eaf5bf380a13aade7d36c711a4acc6bfad3447ccca3a60df9a9761ba884c8`
- source silhouette report SHA-256：`5a7fa43990fd6b8cee021b6a87497d1a8e4ee7bca130e756cf8d7013b59a1e50`
- semantic report SHA-256：`912a192ef7672bfabf97000897eb4ef86834ff5513bb099747f9b85dfb91a034`
- Alpha repair report SHA-256：`1019918d36579fc9d1ee3f067b679672d4c9b3fefa2db2dea04beb2587dae39f`
- preview manifest SHA-256：`2f1e5cedd89921fb61f12cd9f0a6bd6dc0631540162a201f1e002f1532193d77`

## 后续修复项

1. 将一次视频修复尝试定义为原子复合操作，或把零费用私有上传从每目标“模型尝试次数”中分离；总请求和总费用账本仍需逐请求记录。
2. 在视频编辑前先验证剩余额度足以覆盖 `upload + edit`，不足时不要上传孤立临时文件。
3. 当前 xAI image-to-video 对三个 walk 的角色几何保持仍不足；在再次付费前，应优先验证显式关键帧或确定性 rig/hybrid 路径，而不是降低 `silhouette-temporal@2.0.0` 门禁。
