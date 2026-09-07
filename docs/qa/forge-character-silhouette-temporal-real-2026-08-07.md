# Forge Character 轮廓时间稳定真实验收

日期：2026-08-07  
结论：**无效，已被 V2 复验推翻**

> 该报告保留为历史反例，不能再作为发布或商业宣传依据。`rigid-head-alpha@1.0.0`
> 在头肩区域改写了 10,094 个像素，而 `silhouette-temporal@1.0.0` 随后又在同一区域
> 评分，形成评估泄漏。用户在原生大图中仍能看到边缘漂移。当前结论见
> `docs/qa/forge-character-silhouette-temporal-v2-real-2026-08-07.md`。

## 验收对象

- 冻结来源：既有真实 xAI 四方向静帧和视频。
- 最终 Character Job：`6f66b269-23fe-4abf-ba1f-9a31b55226bd`。
- Godot 安装 Job：`6a0ef0ed-e950-419e-b683-5b8048bed3d9`。
- Pack：`generated-assets/forge-core-real-revalidation-20260807/jobs/6f66b269-23fe-4abf-ba1f-9a31b55226bd/exports/validation-ranger/Validation-Ranger.gsfpack`
- Pack SHA-256：`a2b4422bbd2ac918b07288a1996d0c802a9ff7287be69cdebef18eb5efc71b6e`。
- Pack 文件总大小：`6,042,794` bytes。

本轮只执行本地 assembly、归一化、质量、Pack 和 Godot 操作。授权账本执行前后均为 11 条请求，`nextSequence=12`，因此新增 Provider 请求为 0。

## 旧结果（无效）

使用下列版本化处理：

- `shared-scale+per-frame-body-anchor@1.0.0`
- `premultiplied-bilinear@1.0.0`
- `rigid-head-alpha@1.0.0`
- `temporal-alpha@1.0.0`
- `silhouette-temporal@1.0.0`

确定性处理记录：

- 头肩 Alpha 稳定像素：10,094。
- 删除时间闪点：24。
- 填补单帧小孔：13。

逐动画最终指标：

| 动画 | 最小核心 IoU | 中位核心 IoU | 最大轮廓距离 | 中心步进 | 脚底步进 | 结论 |
|---|---:|---:|---:|---:|---:|---|
| `idle` | 0.9995 | 1.0000 | 0.03px | 0.5px | 1.0px | game_ready |
| `walk_up` | 0.9797 | 0.9905 | 1.01px | 0.5px | 0.0px | game_ready |
| `walk_right` | 0.9808 | 0.9984 | 1.14px | 0.5px | 2.0px | game_ready |
| `walk_down` | 1.0000 | 1.0000 | 0.23px | 0.5px | 0.0px | game_ready |

机器报告：

- `generated-assets/forge-core-real-revalidation-20260807/jobs/6f66b269-23fe-4abf-ba1f-9a31b55226bd/character-silhouette-temporal-report.json`
- SHA-256：`0158d99de581fdaabd06a602d206d091ae057bb282e3d7dbb108c4f86f978584`
- Framing 报告 SHA-256：`25a195b030bc9023ba0b643ae9575be41bed8cc684fe8cf281562807e41e9eb2`

## 绿色背景说明

正式 PNG 和 Godot 纹理继续使用透明 RGBA。绿色不是交付背景，而是 chroma-key 中间态和边缘调试背景。每个动画现在都输出：

- `*-checkerboard.gif`
- `*-chroma-green.gif`
- `*-dark.gif`

它们位于旧 Job 和 Pack 的 `previews/debug/`。当时的人工检查结论现已撤回：缩放预览掩盖了原生尺寸边缘漂移，不能证明素材合格。

## 旧 Godot 记录（不可作为资产通过证据）

- Godot：`4.6.3.stable.official`。
- 无头 editor import：通过。
- `forge_animation_showcase.tscn` 无头加载：通过。
- `forge_usage.json` 包含 `characterSilhouetteTemporal` 和正确四方向映射。
- 项目审计：4 个资产、4 个 Pack、3 个已安装资产，0 error、0 warning、clean=true。
- 最大文本资源：6,995 bytes。
- 无 `PackedByteArray`、`ImageTexture.create_from_image` 或内嵌 Image。
- 凭据值扫描通过；仅 Godot 标准日志含 `https://godotengine.org`，没有临时媒体 URL。

## 当时的回归记录（仅证明代码测试，不证明素材质量）

- `cargo fmt --all --check`：通过。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过。
- `cargo test --workspace --all-features`：通过。
- `scripts/test-cli-product.sh`：通过。
- `scripts/test-stage3-static.sh`：通过。
- `git diff --check`：通过。
