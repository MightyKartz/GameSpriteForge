# Forge Character、Icon、Prop 核心能力验收（2026-08-06）

## 结论

本轮限定验证 `Character`、`Icon Set`、`Prop Set`，其余资产类型未进入范围。

- 离线 Fixture 全链路：**通过**。
- 三类 `.gsfpack` 校验：**通过**。
- Godot 4.6.3 安装、导入和资源注册：**通过**。
- CLI/workspace 回归：**通过**。
- 验收产物安全扫描：**通过**。
- 真实 xAI：**未执行**；仅完成 `--plan-only` 请求预算核对，因此本报告不把 Fixture 占位图视为真实视觉质量证据。

当前可以确认 Forge 的 CLI、JobStore、质量报告、Pack 和 Godot 交付闭环对这三类资产可工作。真实模型的角色/图标/道具视觉一致性仍需要独立、显式费用授权后验收。

## 验收环境

- Git HEAD：`23e5b9050249`
- 分支：`codex/stage3-static-assets`
- CLI：`0.2.0-cli.1`（本地 debug build）
- Godot：`4.6.3.stable.official.7d41c59c4`
- Provider：`fixture`（执行），`xai`（仅 plan-only）
- 验收根目录：`generated-assets/forge-core-capability-validation-20260806`
- Style revision：`2525c32236cfd61a`

工作树在验收前已经包含大量未提交的 Stage 3 修改和 QA 资产；本轮没有清理、覆盖或提交这些既有修改。

## 回归门禁

以下命令全部通过：

```text
cargo build -p forge-cli
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./scripts/test-cli-product.sh
```

`forge doctor --json` 返回 `ok: true`，并确认 Godot 4.6.3、FFmpeg、FFprobe、fixture 与 xAI Provider 能力可被 CLI 正确发现。Doctor 按设计不主动读取 Keychain。

## Fixture 端到端结果

### Character

- Job：`510de8ea-c1ab-4c88-acb3-b9dc5f8b396d`
- Lifecycle：`succeeded`
- Pack：`jobs/510de8ea-c1ab-4c88-acb3-b9dc5f8b396d/exports/validation-ranger/Validation-Ranger.gsfpack`
- `forge pack validate`：`valid: true`
- 质量：`animation-quality@2.0.0`，总判定 `game_ready`
- 动作：`idle`、`walk_up`、`walk_right`、`walk_down`
- 每个动作：8 帧；`loop@2.0.0` 判定均为 `game_ready`
- Godot：生成 `AnimatedSprite2D` 场景并注册四个动画；左向继续由运行时水平翻转策略处理
- 安装 Job：`a8bb266d-6319-4211-a6a8-500e02ff8b8f`

### Icon Set

- Job：`668cfefe-d289-4065-9605-2d9237754f79`
- Lifecycle：`succeeded`
- Items：`potion`、`key`、`coin`、`scroll`、`gem`
- Pack：`jobs/668cfefe-d289-4065-9605-2d9237754f79/exports/validation-inventory-icons/validation-inventory-icons.gsfpack`
- `forge pack validate`：`valid: true`
- 质量：`consistency@1.4.0`，5/5 `game_ready`
- Godot：5 个独立外部 PNG/资源路径已注册
- 安装 Job：`8ceec837-cc5d-41ec-8478-9ea814b9e4ed`

### Prop Set

- Job：`70764816-1951-4728-8714-251b04f70f08`
- Lifecycle：`succeeded`
- Items：`chest`、`barrel`、`crate`、`signpost`、`campfire`
- Pack：`jobs/70764816-1951-4728-8714-251b04f70f08/exports/validation-forest-props/validation-forest-props.gsfpack`
- `forge pack validate`：`valid: true`
- 质量：`consistency@1.4.0`，5/5 `game_ready`
- Godot：5 个独立场景/外部纹理已注册
- 安装 Job：`535b504e-9906-42c9-832e-adc520f91e6c`

Fixture contact sheet 是确定性占位图，只用于证明帧数、透明度、画布、报告、打包和安装链路，不用于判断真实模型美术质量。

## Godot 验收

Godot 项目：`generated-assets/forge-core-capability-validation-20260806/godot`

使用 Forge Doctor 解析出的绝对路径执行：

```text
/Applications/Godot.app/Contents/MacOS/Godot --headless --editor --quit --path <project>
```

结果为退出码 0。`.forge/assets.json` 注册了三类资产；另外验证：

- 所有 `.tres/.tscn` 小于 1 MiB。
- 未发现 `PackedByteArray`、内嵌 `Image` 或 `ImageTexture.create_from_image`。
- Character 使用外部纹理和 `AnimatedSprite2D` 场景。
- Icon/Prop 的 item 映射和场景路径已写入 Godot 资产清单。

## xAI plan-only 预算

本轮只生成计划，没有领取 plan token、创建真实 Job 或发起网络生成。三个计划均锁定 `xai/default` 与 `grok-imagine-image-quality`，缓存命中为 0。

| 类型 | 预计 Provider 请求 | 最大 Provider 请求 | 说明 |
|---|---:|---:|---|
| Character | 9 | 17 | 1 个身份参考 + 4 个方向静帧/视频及最多一次定向重试 |
| Icon Set（5 items） | 5 | 10 | 每个 item 最多两次 |
| Prop Set（5 items） | 5 | 10 | 每个 item 最多两次 |
| 合计 | 19 | 37 | 未执行 |

计划 fingerprint：

- Character：`790bd36bb72a2ab3a97a8304806018cb87430598982da47c98d34f7db1274fa5`
- Icon：`89a6dc442b49703a7328cf80dbba0691e43c9284ee1d030448e9275459ef504b`
- Prop：`5b82da0ead9d1fcb8c73fe084edd0ad207e764e2ef64c61040ef6d7cb630b32e`

历史 Character 实测约为 26.2B–29.4B cost ticks（约 2.62–2.94 美元）；静态图片以近期记录粗估约 0.6B–0.7B ticks/次。若下一步执行三类真实验收，建议分别授权并设置：Character 35B、Icon 10B、Prop 10B，总上限 55B ticks（约 5.50 美元）。这是安全上限，不是 xAI 报价或实际费用承诺。

## 安全与完整性

- 在本轮 JobStore、Pack 和 Godot 项目中按敏感字段与 Bearer 形式扫描，未发现 API Key、Access/Refresh Token、Device Code 或 Authorization header。
- 未发现待上传/临时媒体 URL。
- `real-plan-jobs` 未创建，证明 plan-only 没有启动真实执行。
- Godot 安装清单 SHA-256：
  - Character：`72031b2b4fe14e29681335a5ac3ecb6b0c6c8d44d3af58769caeab8017cf7084`
  - Icon：`fa3d117f2ea52fefb1ca4a9fbc78b5cfddca1528ea2c29d54dbb7eac0d42d7c6`
  - Prop：`cd4c338471db3dbc71c4d317ddd1ff66c520f1292f34619a00db5f123b833719`

## 裁决

本轮核心功能验收通过，但只代表“可靠执行和交付能力”通过，不代表三类资产的真实模型视觉门槛已经同时通过。下一步应使用三个独立费用授权依次执行 Character、Icon、Prop 的真实 xAI 验收；任一类型失败时只修复该类型，不扩大到 Portrait、Equipment、Decal、Terrain、Building 或 Map。

机器可读摘要：`docs/qa/artifacts/forge-core-character-icon-prop-validation-20260806/summary.json`
