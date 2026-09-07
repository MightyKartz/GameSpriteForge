# Forge Character、Icon、Prop 真实 xAI 验收（2026-08-06）

## 裁决

真实 Provider 执行已完成并严格停留在用户授权范围内，但本轮发布门槛 **未通过**：Character、Icon Set、Prop Set 均未产出可验证的 `.gsfpack`，因此没有执行虚假的 Godot 安装。

这次运行成功证明了费用授权、耐久 Job、逐目标账本、失败拦截和零费用 replay；同时发现三个发布阻断问题：

1. xAI 异步视频创建响应不含 `costInUsdTicks`，Forge 在拿到可持久化 request ID 之前就要求结算，导致 Character 视频请求被记为 `ambiguous` 并丢失可轮询票据。
2. 普通 Icon/Prop 工作流把异构 Style board 的边缘密度直接作为硬基线，造成真实静态资产系统性 `edge_density_drift` 误报。
3. Prop anchor 会把物体语义带入后续 item：本轮 `crate` 实际看起来像带锁的第二个宝箱，现有检测没有发现 prompt/物体语义错误。

Character 还暴露了独立的视觉问题：Style board 里的兜帽角色没有五官，新的 Character reference 和两个 idle still 也继承了空白脸。它虽然有完整四肢，但不具备可锁定的角色身份。

## 授权与实际使用

用户授权：最多 37 次媒体请求，总费用上限 55,000,000,000 cost ticks；禁止生成其他资产。

Forge 创建了三个互不挪用预算的耐久授权：

| 类型 | 请求上限 | 费用上限 | 实际请求 | 已观测费用 | 保守账本费用 |
|---|---:|---:|---:|---:|---:|
| Character | 17 | 35.0B | 5 | 2.0B | 12.0B |
| Icon Set | 10 | 10.0B | 10 | 6.0B | 6.0B |
| Prop Set | 10 | 10.0B | 8 | 5.5B | 5.5B |
| 合计 | 37 | 55.0B | 23 | 13.5B | 23.5B |

保守账本费用把两个无法从创建响应取得费用的 Character 视频请求各按 5.0B 预留额计算，因此是当前应采用的安全口径，约 2.35 美元。实际账单可能低于该值，但不能在缺少 Provider 结算字段时声称精确费用。

所有请求均被 target allowlist 限定：

- Character：`subject_reference` 与四个动作的 `still/video`；每个 target 最多两次。
- Icon：`potion/key/coin/scroll/gem`；每项最多两次。
- Prop：`chest/barrel/crate/signpost/campfire`；每项最多两次。
- 模型：只允许 `grok-imagine-image-quality` 与 Character 所需的 `grok-imagine-video-1.5`。

## Character

- Job：`9bf6e2f7-3831-40f8-8a48-cfaff79c38d6`
- Lifecycle：`failed`
- 请求：5
  - 3 个图片请求已结算：600M + 700M + 700M ticks。
  - 2 个 `idle:video` 请求状态为 `ambiguous`，各保守占用 5B ticks。
- 错误：`provider_cost_budget_exceeded: real Provider response omitted costInUsdTicks; the durable reservation remains consumed`
- Pack：未生成。

问题发生在视频创建阶段，不是视频质量门禁。xAI 的创建响应成功提交了异步任务，但 Forge 当前在返回 `VideoTicket` 之前结算授权；由于创建响应没有费用字段，票据没有进入 JobStore，后续无法轮询或取消。第二次自动尝试同样失败后，`idle:video` 的两次额度耗尽，系统正确停止。

人工原尺寸审查：

- 角色全身、双腿和靴子完整。
- 兜帽下的脸是无眼、鼻、嘴的空白肤色区域。
- reference、idle attempt 1、idle attempt 2 均复现该问题。
- 不应在修复视频结算后直接继续付费生成；应先要求可见五官或绑定合格 SubjectLock。

## Icon Set

- Job：`b42347cd-a74f-4108-9385-ad89c9dde457`
- Lifecycle：`awaiting_review`
- 请求：10/10，费用 6.0B ticks。
- Pack：未生成。
- 报告：`consistency@1.4.0`，聚合判定 `blocked`。

五个 item 的画布、Alpha、裁切、主体数、前景尺度、锚点和调色板均通过；唯一失败理由均为 `edge_density_drift`：

| Item | Edge ratio | 判定 |
|---|---:|---|
| potion | 0.462 | regenerate |
| key | 0.507 | regenerate |
| coin | 0.508 | regenerate |
| scroll | 0.458 | regenerate |
| gem | 0.321 | regenerate |

人工审查认为五个图标的对象语义正确，材质、轮廓、光照和调色板具有良好集合一致性。误报来自 Style board 基线 `edgeDensity=57.007698`：该 board 同时包含角色、包、盾牌路牌和背景，不能直接代表 128px 单图标的对象复杂度。

零费用 consistency replay：

- Child Job：`efb91cce-2929-45c7-9512-92096d49c2a2`
- Provider 请求：0
- 费用：0
- 报告 SHA 与原报告一致，证明结论可重复且不是 Provider 随机性。

## Prop Set

- Job：`ca27fde0-6708-49b2-914d-954a9a403101`
- Lifecycle：`awaiting_review`
- 请求：8/10，费用 5.5B ticks。
- Pack：未生成。
- 报告：`consistency@1.4.0`，聚合判定 `blocked`。

| Item | Edge ratio | 判定 |
|---|---:|---|
| chest | 0.768 | game_ready |
| barrel | 0.426 | regenerate |
| crate | 0.656 | awaiting_review |
| signpost | 0.311 | regenerate |
| campfire | 0.681 | awaiting_review |

人工审查：

- `chest`、`barrel`、`signpost`、`campfire` 的视觉风格基本一致，硬 edge 判定不符合肉眼结果。
- `crate` 虽然画面质量好，但带弧形盖、锁孔和金属护角，实际是第二个宝箱，不满足“square wooden supply crate”的 item 语义。
- 这说明不能简单把 edge density 全部降为 advisory 后导出；还必须加入 item 语义/anchor identity leakage 门禁。

零费用 consistency replay：

- Child Job：`a3dfbba7-8f4a-4113-9c11-86685bee6dfd`
- Provider 请求：0
- 费用：0
- 报告 SHA 与原报告一致。

## Pack、Godot 与项目审计

- 验收 JobStore 中 `.gsfpack` 数量：0。
- 因三类资产均未达到可导出状态，没有执行 Godot plan-install；这属于正确的失败封锁，不是遗漏。
- 使用带 `collection-assets` feature 的当前 CLI 执行 `project audit --scope all`：现有 xAI 项目 1 个已登记 Pack，0 error、0 warning、`clean: true`。本轮失败 Job 未污染项目 Catalog。
- 验收 JobStore 与授权账本的敏感字段扫描无命中：未发现 API Key、OAuth Token、Device Code、Bearer/Authorization header 或临时媒体 URL。

## 必须先修复的顺序

### P0：异步视频授权结算

- 创建请求成功后立即持久化 xAI request ID 和授权 reservation ID。
- `submitted` reservation 随 `VideoTicket` 进入 JobStore。
- 只在 `/v1/videos/{id}` 完成响应获得 usage 后 `settled`。
- 完成响应仍无费用时保守记为 `ambiguous`，但必须保留 request ID、轮询结果和本地媒体，禁止重复提交同一 target。
- 取消、超时、重启恢复都必须能找到远程 request ID。

### P0：Character 身份基准

- 没有 SubjectLock 时，canonical reference 必须通过“可见脸部/身份区域”门禁；空白脸不得进入方向和视频阶段。
- Style board 只提供风格，不应把其中的空白脸当作身份模板。
- 对需要可见脸部的角色，在 spec/SubjectLock 中提供明确策略；全身完整性与脸部身份分别报告。

### P1：静态集合一致性

- Style board edge density 只作风格诊断，不直接对异构 Icon/Prop 触发重生成。
- 同类 edge 基线应来自已接受 anchor、Collection medoid 或按 asset kind 校准的冻结样本。
- 保留 Alpha、裁切、画布、多主体、锚点和严重调色板漂移为硬门禁。
- 增加 item 语义门禁，至少能拦截本轮 `crate` 被 anchor 变成宝箱的问题。
- 修复后只需对现有源 Job 做零费用 consistency replay；不得再次调用 Provider，除非语义错误 item 获得新的定向费用授权。

## 证据路径

- 验收根目录：`generated-assets/forge-core-real-validation-20260806`
- Character reference SHA-256：`80c1dc17ca8a0fea9355e95485f0df683192bc3f0e16fee75eb032c6b6678486`
- Icon contact sheet SHA-256：`306fcf5160b24505db7e24a150fd001c4a65516bde31c5ca980b14bdabf27617`
- Prop contact sheet SHA-256：`955472611f2b1e5c29e6d455defd2f6adea3103ea95d43f0de69857bb1efa8a7`
- 机器摘要：`docs/qa/artifacts/forge-core-character-icon-prop-real-20260806/summary.json`

