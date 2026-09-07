# Forge Character 轮廓独立验收 V2（真实 xAI 素材零费用复验）

日期：2026-08-07  
结论：**门禁修复通过；当前 Character 素材未通过，Pack 正确阻断**

## 验收对象

- 派生复验 Job：`60b24b33-e0ef-460f-89e1-adf0fd7c4fbc`
- 父 Job：`6f66b269-23fe-4abf-ba1f-9a31b55226bd`
- 冻结真实来源：`idle`、`walk_up`、`walk_right`、`walk_down` 的既有 xAI 静帧与视频。
- Provider 请求账本复验前后 SHA-256 均为 `36d6b62aba1a57ad3eb58133631b85a87eccef280363890b6a3e1c10f0d5d6dd`。
- 请求数保持 11，`nextSequence=12`，新增媒体请求为 0。

## 评估泄漏修复

- 已从生产归一化路径移除 `rigid-head-alpha@1.0.0`。
- 未修复帧先写入 `processed/normalized-source/` 并生成独立 source 报告。
- 后处理只运行 `boundary-alpha-repair@1.0.0`：共修改 29 个 1px 边界像素，最大单帧 4 个，总前景改写率 `0.004652%`，结论 `game_ready`。
- source 和 post-repair 分开评分；本次二者都如实判定三个 walk 方向失败。

## 真实结果

| 动画 | source 结论 | 最小核心 IoU | 最大轮廓距离 | 核心不确定率 | 核心无支持边缘 | 全身无支持边缘 |
|---|---|---:|---:|---:|---:|---:|
| `idle` | game_ready | 0.9801 | 0.87px | 0.0209 | 0.0000 | 0.0318 |
| `walk_up` | blocked | 0.9422 | 2.24px | 0.0956 | 0.2350 | 0.4071 |
| `walk_right` | blocked | 0.9192 | 2.82px | 0.1594 | 0.4478 | 0.7077 |
| `walk_down` | blocked | 0.9025 | 4.28px | 0.1654 | 0.7265 | 0.7010 |

`walk_up`、`walk_right`、`walk_down` 的失败来自未修复 source 帧，因此不能通过调 Alpha、人工审核或本地重放解决。`idle` 已证明当前门禁不会把稳定角色误杀。

4× 深色背景接触表的人工复核与机器结论一致：三个 walk 方向存在兜帽、脸部、肩线、披风或躯干轮廓的帧间重绘，属于角色几何变化，不是可由 1px Alpha 清理安全修复的普通抗锯齿抖动。

## 交付与隔离

- Job 状态：`awaiting_review`，错误码 `character_silhouette_temporal_failed`。
- `pack:export`：blocked；新 Job 的 `exports/` 目录为空，没有 Pack 文件。
- 没有执行 Godot 安装或覆盖。
- 旧 Pack 文件未修改；项目 `.forge/catalog.json` 已将 `validation-ranger` 标记为 `gameReady=false`、`quarantined`，因此旧安装不能继续作为合格资产引用。
- 36 个多背景/多尺度真实预览和带 SHA-256 的 manifest 已生成，供原生尺寸人工复核；预览目录共 20 MiB。

## 机器证据

- source 报告 SHA-256：`303b1380c0f3417f1bb63fe38611a77e1c182e0db9cf4a6daf2fc908cdb8d16b`
- post-repair 报告 SHA-256：`337f67286fe25d8eac4f3a39ba5927727478d398eb7eacb7b6d161d512aa3838`
- Alpha repair 报告 SHA-256：`be638bebe5889983be47fc8438371e366668637cbdf6fa57fb4daa7de8f51992`
- preview manifest SHA-256：`0a6c2ad3d1fd277ad21cc93d4d7681c5aeeed93922af2d94d054ffbf576f47fd`

## 回归结果

- `cargo fmt --all --check`：通过。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过。
- `cargo test --workspace --all-features`：通过。
- `scripts/test-cli-product.sh`：通过。
- `scripts/test-stage3-static.sh`：通过。
- Godot `4.6.3.stable.official` 独立 smoke Pack 导入：通过；这只验证安装器回归，不把当前被拦截的真实 Character 安装到 Godot。
- 最终 Job 的凭据、授权头和临时媒体 URL 扫描：通过。
- `git diff --check`：通过。

## 下一步

如需得到可导出的 Character，应只授权 `walk_up`、`walk_right`、`walk_down` 的 still/video 定向重生成。`idle` 可以复用，不应再次付费；新素材仍必须从 source 门禁开始评估。
