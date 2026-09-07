# Character V2.3 四方向真实 xAI 验收

日期：2026-08-08

## 结论

`topdown-keyframes@2.3.0` 的 Ayla 四方向角色资产已通过真实 xAI 生成、质量门禁、显式人工灰区审核、`.gsfpack` 校验和 Godot 4.6.3 场景加载验收。最终 Job `8b5248a0-c944-4c84-a4a9-80d020ae0396` 为 `succeeded`。

本次只生成 `idle`、`walk_up`、`walk_right`、`walk_down` 图片帧；未生成 Subject、Style、视频或其他资产。

## Provider 使用量

- Authorization：`character-v23-four-directions-20260808`
- 实际请求：40 / 64
- 实际费用：30,200,000,000 / 51,200,000,000 cost ticks，约 3.02 美元
- 本地一致性重放、循环裁剪、Pack、Godot 和安全审计均为零 Provider 请求

## 动画结果

| 动画 | 帧数 | FPS | loop score | 结论 |
| --- | ---: | ---: | ---: | --- |
| idle | 6 | 8 | 0.9688848 | game_ready |
| walk_up | 8 | 8 | 0.76339984 | game_ready |
| walk_right | 8 | 8 | 0.90009606 | game_ready |
| walk_down | 8 | 8 | 0.9638875 | game_ready |

idle 使用 `keyframe-loop-trim@1.0.0` 在本地选择源帧 1–6，未产生 Provider 请求。Pack 内报告与 Job 报告 SHA-256 均为 `12f06222d8a1675efd78d8c170cf69fae6603556f4e33a5ee66e7efc4dab2c88`。

一致性报告只有 `idle/frame-00` 位于可审核灰区，原因是 `edge_density_drift`。其 edge density ratio 为 0.7056891，但调色板重合度 0.9951572、感知相似度 0.96875、锚点漂移 1px，且完整身体、Alpha、单主体、边界、方向和时序轮廓硬门禁全部通过。原生尺寸联系表检查后显式接受，没有降低全局阈值。

## Pack 与 Godot

- Pack SHA-256：`879569546694b252768bea0dd0ddbe27abe3c47ffd8342c0a02d461234f1d1c7`
- `forge pack validate`：通过
- Godot：4.6.3 stable
- Godot 安装 Job：`0bef21cb-48b0-4fa0-a8fd-b9df6befa379`
- 场景实际加载：通过；动画帧数为 `idle=6, walk_up=8, walk_right=8, walk_down=8`
- 方向映射：左向复用 `walk_right` 并水平翻转
- 外部纹理：2 个 PNG；没有内嵌 Image、`PackedByteArray` 或 `ImageTexture.create_from_image`
- 最大 `.tres/.tscn`：6,629 bytes，低于 1 MiB
- `forge_usage.json` 已记录 `keyframe-loop-trim@1.0.0` 及其报告 SHA

## 安全与回归

- Job、Pack 和 Godot 工程凭据/Authorization/Bearer/Token 命中：0
- 临时 xAI 媒体 URL 命中：0
- `cargo fmt --check`：通过
- Core Clippy all-targets：通过
- Pack：20/20
- direction anchor：2/2
- keyframe cleanup：4/4
- keyframe Provider fixture contract：1/1
- consistency V2 fixture contract：通过

机器可读摘要、最终联系表、GIF 和报告位于 `docs/qa/artifacts/forge-character-keyframes-v23-real-20260808/final/`。
