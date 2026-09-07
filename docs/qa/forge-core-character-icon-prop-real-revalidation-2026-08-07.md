# Forge Character、Icon、Prop 真实 xAI 复验（2026-08-07）

## 裁决

本次限定范围真实复验 **通过**：Character、Icon Set、Prop Set 均达到 `game_ready`，三份 `.gsfpack` 均通过校验并成功安装到 Godot 4.6.3。项目级 `project-audit@1.1.0` 最终为 0 error、0 warning、`clean: true`。

用户授权最多 29 次媒体请求、47,000,000,000 cost ticks；实际使用 17 次、36,700,000,000 ticks（约 3.67 美元）。没有生成授权范围外的资产。

| 类型 | 授权请求/费用 | 实际请求 | 实际费用 | 结果 |
|---|---:|---:|---:|---|
| Character | 17 / 35.0B | 11 | 32.6B | 4/4 动作 game_ready |
| Icon Set | 10 / 10.0B | 5 | 3.4B | 5/5 item game_ready |
| Prop `crate` 定向重试 | 2 / 2.0B | 1 | 0.7B | 5/5 item game_ready |
| 合计 | 29 / 47.0B | 17 | 36.7B | 通过 |

三个授权账本中的 17 条请求全部为 `settled`。Character 的最终本地 loop replay 使用 0 次 Provider 请求；其 `provider-usage.json` 明确记录 `requests: 0`。

## Character

- 最终 Job：`1e927928-0ec5-45f1-abce-c0e44fbf417a`
- 来源链：`323a1c5a-8af3-467a-95de-b093d5004edd` → `3f24fd72-7577-4cd7-af41-f6fe4920e8b1` → `1dc8847d-be05-4062-90b3-2f71a3d1c0d1` → 最终零费用 replay
- 身份门禁：`character-identity@1.1.0`，`game_ready`
- 一致性：`consistency@1.5.0`，4/4 `game_ready`
- 动画质量：`animation-quality@2.0.0`，4/4 `game_ready`
- 循环：`loop@2.0.0`，4/4 `game_ready`
- Pack SHA-256：`9a28eb694bbb9641357dde3e88a393bd78a037ddd796048094f548ee70811a5a`
- Godot install Job：`5392e49f-4bfb-4a28-91e8-2d08596cb04e`

本次真实运行发现并修复：

1. 明亮眼白被错误选为肤色 seed，导致存在五官的 canonical reference 被误判为空白脸；seed 规则已修复并升级身份 profile。
2. `idle` 正面方向静帧真实生成了空白脸；新增视频前方向身份门禁，失败静帧不会再产生视频费用。
3. 第二次 idle still 仍为空白脸时，Forge 改为复用已经通过身份门禁的 canonical reference，只重新生成 idle 视频。
4. `loop/matting/consistency` replay 过去仍解析 xAI Keychain；现在使用 fail-closed 的本地 replay Provider，不读取凭据，任何意外网络媒体调用都会报错。
5. Pack 追加一致性报告后原 artifact/Catalog SHA 会陈旧；现在在最终写入后重新计算目录哈希并同步 Job/Catalog。

最终 loop composite score 为 idle `0.899`、walk_up `0.918`、walk_right `0.912`、walk_down `0.965`；锚点闭合均不超过 0.5px。

## Icon Set

- 最终零费用 replay Job：`daff18db-632e-4c06-965e-ffad9dee4d2d`
- 真实来源 Job：`51ff27d4-5297-463b-b652-1c25b4f606c0`
- 一致性：`consistency@1.5.0`，5/5 `game_ready`
- Pack SHA-256：`deffa1583ad21ae50616bb6aec290d7553b64fbdc5839a0041f7bf94f067f857`
- Godot install Job：`7aac7cfd-aca5-473a-82ef-fa63d47050b5`

真实图标的对象语义、材质、调色板与轮廓一致。`gem` 与 anchor 的几何相似曾触发 Prop 专用的 identity-leakage 规则；已修正为 Icon 只记录该相似度诊断，不把正常的居中图标构图误判为语义复制。Alpha、裁切、多主体、画布等硬门禁未降低。

## Prop Set

- 定向重试 Job：`7601592a-e5e5-461f-bf0e-2baed8a0e4e8`
- 仅调用目标：`crate`
- 实际请求：1，费用 700,000,000 ticks
- 一致性：`consistency@1.5.0`，5/5 `game_ready`
- Pack SHA-256：`242ccb2fcb473fad3c749ef6fdbe24b875a0a34d81e52c5482622e0b281dffa4`
- Godot install Job：`a84b015a-1d1b-42d6-b9b6-6fb6fb42cd31`

新 `crate` 是平顶、方形木板补给箱，没有弧形宝箱盖或锁孔。由于宝箱与木箱天然共享木材和箱体几何，机器规则保守进入灰区；原尺寸人工审核接受并留下可审计理由，未越过 Alpha、裁切、损坏媒体等硬失败。

## Godot、Pack 与安全

- Godot：`4.6.3.stable`
- 三份 Pack：全部 `forge pack validate` 通过。
- Godot headless editor import：通过。
- 项目审计：4 个 Catalog asset、4 个 Pack、3 个 Godot installed asset；0 error、0 warning。
- 最大文本资源：Character `.tscn` 7,655 bytes；远小于 1 MiB。
- `.tres/.tscn` 未发现 `PackedByteArray`、内嵌 Image 或 `ImageTexture.create_from_image`。
- JobStore、Pack、授权账本和 Godot 项目的凭据扫描：0 命中。
- 临时媒体 URL 扫描：0 命中。

## 回归门禁

- `cargo fmt --all --check`：通过。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过。
- `cargo test --workspace --all-features`：通过。
- `scripts/test-cli-product.sh`：通过。
- `scripts/test-stage3-static.sh`：通过。
- 新增本地 replay fail-closed 单元测试：通过。

## 证据

- Character contact sheet SHA-256：`5ce3f21ec1fd2da5e8d8db67da7d144c0c65e7dedb10c80434352bde15920465`
- Icon contact sheet SHA-256：`1f9721f1a6de531a4170bc928c50ee536d571a7e6e4a1341fc8ceb315742172c`
- Prop contact sheet SHA-256：`8df8ee260f5794bc2a25490005b9acb2cb4658e6d6af7a1da9f5e41bca052034`
- 机器摘要：`docs/qa/artifacts/forge-core-character-icon-prop-real-revalidation-20260807/summary.json`

