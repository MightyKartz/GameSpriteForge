# Forge Stage 3 真实验收阻断项修复计划

日期：2026-08-05  
状态：实施中  
真实 Provider：本计划禁止调用；全部修复先通过 fixture、合成 PNG 和现有真实产物的零费用重评。

## 1. 目标与非目标

目标是修复 [Stage 3 真实 xAI 验收](../qa/forge-stage3-real-acceptance-2026-08-05.md) 暴露的质量语义、恢复、Catalog、Godot 和 audit 状态链问题，使 Forge 在下一次付费复验前满足：

- 无效 Collection anchor 不能形成不可变 Lock；
- 所有声明 item 必须进入同一份集合报告，任何硬失败都不能被子集 verdict 掩盖；
- 异构静态集合不再因为对象拓扑差异被 Style/anchor edge-density 误杀；
- Portrait 固定构图漂移在 Pack 导出前被确定性拦截；
- Provider 传输失败可创建 child Job，从首个未完成 item 继续，复用已付费产物；
- review 导出、Pack SHA、Catalog 注册保持原子一致；人工否决可 quarantine，不删除证据；
- Godot 保留 `portrait_set` 类型；Project audit 检查 Catalog 与 Godot manifest 的双向闭包。

非目标：不修改 Provider、模型、公开质量阈值；不调用 xAI；不实现视觉大模型组件；不重新生成真实资产。

## 2. 版本化质量合同

### `collection-anchor@1.1.0`

Collection anchor 在写入 Lock 前执行确定性构图门禁：

- Alpha/画布/裁切/前景存在等既有硬门禁；
- 主要前景组件只能为 1；
- 小组件累计占比、底部平台跨度与底部实心带不得表现为装饰底座/场景卡；
- Icon/Portrait anchor 必须满足 kind 对应的 foreground extent、centroid 和 silhouette 约束；
- 失败生成机器报告并阻断 Lock，不允许人工绕过损坏媒体或多主体硬失败。

### `consistency@1.4.0`

- legacy static 与 Character 保留既有 edge-density verdict。
- Collection-backed static 的单 item 阶段只用 edge density 作诊断字段，不用它单独触发 Provider 重试；集合相似性由 anchor + medoid 的 `collection-consistency@1.1.0` 决定。
- Alpha、裁切、多主体、画布、foreground extent/scale、anchor drift 仍是硬门禁。
- 报告必须显式写出 `edge_density_advisory`，不能通过隐藏字段降低阈值。

### `collection-consistency@1.1.0`

- 输入包含 spec 中全部 item；每项带最终 attempt、base verdict、是否可用于 medoid，以及可选 failure reasons。
- medoid 只从可比较 item 选择，但不可比较/硬失败 item 仍进入报告并把整体 verdict 提升为 `blocked`。
- 整体 verdict 的优先级固定为 `blocked > regenerate > awaiting_review > game_ready`。

### `portrait-framing@1.0.0`

每个 Portrait 与 Collection anchor、Subject canonical 比较：

- foreground width、height、area、centroid；
- 对齐后的 silhouette IoU 与横向 occupancy profile；
- 固定头肩构图所需的上下边界和主体宽高比。

任何一项超出硬范围为 `portrait_framing_drift`/`blocked`；灰区可人工审核，但不得把全身像与胸像混合导出。

## 3. Durable retry 与费用合同

扩展现有接口，不新增破坏性命令：

```text
forge job retry --id <failed-static-job> --stage auto [--wait] --json
```

- 只接受 terminal `failed` 的 static Job，且失败发生在 Provider 传输/超时或可恢复服务错误；
- 创建 child Job，设置 `parentJobId`，源 Job 与源产物不变；
- 对已成功 item 的最终 PNG、原 Provider source 和 SHA-256 做闭包验证后复用；
- 从第一个未完成 item 继续；计划估算只计算剩余 item 的正常/最大请求；
- 本地 matting/normalize/consistency 重放为 0 请求；
- 任一复用文件缺失或哈希变化返回 `legacy_artifact_missing`/`input_changed`，不得静默重生成已付费 item。

## 4. Review、Catalog 与 quarantine

- `job review --accept` 完成本地报告更新、Pack 导出、Pack directory SHA 和 Catalog V2 注册；任一步失败不得把 Job 标记为 succeeded。
- `job review` 不带 `--accept` 可对机器已导出的 static Pack记录人工否决：Catalog entry 设置 `gameReady=false` 和 `review.status=quarantined`，保留 Pack/Job/哈希证据。
- `godot plan-install --catalog-project` 拒绝 quarantined 或非 game-ready entry。
- Catalog 新字段全部可选并有默认值，旧 Catalog 保持可读。

## 5. Godot 与 Audit 闭包

- `portrait_set` 在 Pack、`forge_usage.json` 和 Godot `.forge/assets.json` 中保持同一 kind。
- `project audit --scope all` 从 Catalog 中所有 installed Godot root 读取 `.forge/assets.json`：
  - Catalog installed entry 必须在 Godot manifest 存在、kind/Pack SHA/target 一致；
  - 同一 Godot manifest 中任何 Forge asset 若不在 Catalog，报告 `orphan_godot_asset`；
  - quarantined/非 game-ready asset 若已安装，报告硬错误；
  - 人工 review verdict 与 Catalog `gameReady` 不一致，报告硬错误。
- 审计继续做资源体积、嵌入图像、凭据和临时 URL 检查。

## 6. 实施顺序

1. 新增合成构图签名与 Collection anchor/Portrait 单元测试。
2. 修复静态 runner 的全 item 报告闭包和 edge advisory 路由。
3. 实现 failed static child resume 与请求估算。
4. 修复 review Pack/Catalog 原子注册和 quarantine。
5. 修复 Godot kind，扩展 audit 双向闭包。
6. 对既有真实 Job 执行零 Provider 的 consistency 重评；不得改写源 Job。
7. 运行 Stage 3、CLI、Pack、Godot、全 workspace 回归并固化 QA 报告。

## 7. 验收门槛

- 合成 multi-object/decorative-base anchor 被阻断；合法单对象 anchor 通过。
- 合成 5 Portrait 中混入 1 张全身像时不得导出 Pack。
- 异构但同风格的 herb/compass/mushroom 类合成 item 不因 edge density 单独触发重试。
- Collection report item 数量与 spec 完全相等，失败项存在时整体不可能 `game_ready`。
- 传输失败 fixture 在 child Job 中复用前 N 项，只请求剩余项；源 SHA 不变。
- review 成功后 Catalog Pack SHA 与实际目录一致；quarantine 后安装被拒绝。
- Godot manifest kind 为 `portrait_set`；orphan/mismatch/quarantine 均被 audit 捕获。
- `cargo fmt`、Clippy、workspace tests、Stage 3 contract、CLI product、Pack/Godot headless 和凭据扫描全部通过。
