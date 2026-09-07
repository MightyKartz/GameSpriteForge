# Forge Stage 3 离线验收（2026-08-05）

结论：**离线实现与 fixture/Godot 门槛通过；真实 xAI 门槛未执行，等待费用授权。**

> 后续状态：本文件保留当时的离线基线。用户授权后已完成 1 风格、3 Pack 的真实
> xAI 探针，但未通过发布门槛；结果与 blocker 见
> [Forge Stage 3 真实 xAI 验收](forge-stage3-real-acceptance-2026-08-05.md)。

- 分支：`codex/stage3-static-assets`
- 基线：`23e5b905024942024f3f461d14cdb589c603c444`
- Godot：`4.6.3.stable.official.7d41c59c4`
- 实施计划：[forge-stage3-static-assets-implementation-plan.md](../architecture/forge-stage3-static-assets-implementation-plan.md)
- 机器摘要：[summary.json](artifacts/forge-stage3-offline-20260805/summary.json)

## 已验收范围

- `CollectionSpecV1` / 不可变 `CollectionLockV1`：本地 anchor 为 0 Provider 请求，无 anchor 为 1 次请求；revision、Style/Provider/model、anchor/medoid SHA 与 outlier profile 可验证。
- `collection-consistency@1.0.0`：pairwise matrix、medoid、anchor/medoid 综合分数、灰区/离群结论、定向第二次尝试和机器报告。
- Icon/Prop V2、Portrait（固定五表情）、Equipment V1（icon/world/static preview）、Decal（footprint/blend）。
- `.gsfpack` V2 兼容扩展、Catalog V2 来源、GameArtManifest 新 kind 与 `collection:<id>@<revision>` 锁引用。
- Godot 外部 PNG 安装：Portrait 映射、Equipment/Decal 场景、usage/Catalog install link、所有权与回滚保持原合同。
- `asset export-editable` 与零费用 `asset replace-item`：创建 child Job，不修改源 Job/Pack，手工 item provenance 为 `manual_replacement`。
- `project audit`：完整性、Pack/SHA、锁一致性、质量/离群、license/provenance、Godot 陈旧/体积/内嵌图像、凭据与临时 URL。

## 自动化结果

| 门禁 | 结果 |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo clippy -p forge-cli --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --workspace` | PASS，228 tests |
| `scripts/test-stage3-static.sh` | PASS |
| `scripts/test-cli-product.sh` | PASS |
| `scripts/test-game-art-manifest.sh` | PASS（Checks 0–8） |
| `scripts/test-real-provider-budget-guard.sh` | PASS |
| 全部 `schemas/*.json` 经 `jq empty` | PASS |
| `git diff --check` | PASS |

Stage 3 专项合同真实运行了 fixture Style/Subject/Collection → Icon V2/Portrait/Equipment/Decal → Pack → Godot，并验证：

- 四类新/升级 Pack 全部 `game_ready`；
- Collection report 和无主机路径的 lock ref 均进入 Pack；
- 离群/第二次成功样本产生 `provider_item_orb_attempt_2`；
- replace-item 的 plan estimate/max 均为 0，实际 usage.requests 为 0；
- Godot `.tres/.tscn < 1 MiB`，无 `PackedByteArray` 或 `ImageTexture.create_from_image`；
- `project audit --scope all` 的 errorCount 为 0；
- 临时 JobStore/PlanStore/Pack/Godot 项目凭据扫描为 0 命中。

## 真实 xAI 门槛（未执行）

按冻结门槛：5 个 Style；每个 Style 各 1 个 Icon Pack（≥10 items）、Prop Pack（≥10 items）和 Portrait Pack（5 expressions）。若 Style、Subject、三类 Collection anchor 均由 xAI 生成：

- 预计请求：`5 Style + 5 Subject + 15 Collection anchor + 125 items = 150`
- 最大请求：`25 setup + 250 item attempts = 275`

执行前必须由用户明确接受 xAI 费用，并设置与 275 次最大请求相匹配的请求/费用守卫。未获得授权前，不运行真实 Provider，也不把 Stage 3 标记为真实模型晋级。
