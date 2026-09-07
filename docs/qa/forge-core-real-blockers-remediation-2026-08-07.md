# Forge Character、Icon、Prop 真实阻断项修复验收 — 2026-08-07

## 裁决

2026-08-06 真实验收发现的四个代码阻断项已修复。离线合同、旧真实像素零费用 replay、Icon Pack、Godot 4.6.3 和安全门禁均通过；本轮没有调用 xAI，也没有产生新费用。

机器摘要：[`artifacts/forge-core-real-blockers-remediation-20260807/summary.json`](artifacts/forge-core-real-blockers-remediation-20260807/summary.json)

Character 的代码门禁已通过，但新的真实 Character 四动作端到端生成不在本轮授权范围内，因此仍需单独费用授权后才能恢复“真实 Character 发布门槛通过”的结论。

## 修复结果

| 原阻断项 | 修复 | 验收结果 |
| --- | --- | --- |
| 视频创建响应无费用导致 ticket 丢失 | 授权账本持久化 `providerRequestId`；提交后绑定，终态再结算；轮询可跨重启和授权过期恢复 | mock xAI 与授权合同覆盖正常 usage、缺 usage、重开、幂等结算；全部通过 |
| JobStore 没有可恢复视频票据 | 每次提交写入 `provider_video_ticket`，终态同步 JSON 与 artifact SHA | Character fixture 4 个动作得到 4 个唯一 succeeded ticket，哈希一致 |
| Character 空白脸仍进入视频阶段 | `character-identity@1.0.0` 在 reference/SubjectLock 阶段阻断 | 合成门禁通过；旧真实 reference 被只读诊断为 `blocked / character_face_detail_missing`，0 请求 |
| Icon/Prop edge-density 系统性误报 | `consistency@1.5.0` 将静态 edge 单项降为 diagnostic，硬门禁不变 | Icon 真实 replay 5/5 `game_ready` |
| `crate` 复制 chest 语义 | prompt 防复制 + perceptual/geometry 双指标 review 门禁 | Prop 真实 replay 仅 crate 命中 `anchor_identity_leakage_review`，其余 4/5 `game_ready` |

## 旧真实产物零费用复验

### Character

- 源 Job：`9bf6e2f7-3831-40f8-8a48-cfaff79c38d6`
- reference SHA-256：`80c1dc17ca8a0fea9355e95485f0df683192bc3f0e16fee75eb032c6b6678486`
- 方式：`forge job report` 对本地 canonical reference 做只读 legacy diagnostic
- 结果：`blocked`
- 原因：`character_face_detail_missing`
- enclosed feature count：1
- Provider 请求：0
- 结论：新门禁能够在任何 direction/video 前拦截这张空白脸。

### Icon Set

- 源 Job：`b42347cd-a74f-4108-9385-ad89c9dde457`
- 最终 replay Job：`69839985-9e8c-4f26-b5e5-e441135f977d`
- 结果：5/5 `game_ready`
- Provider 请求/费用：0 / 0
- consistency report SHA-256：`10baa8d8ac510d0635ec5260cb022b07dd5bbdb096ea8cd125a21d30614c6812`
- contact sheet SHA-256：`306fcf5160b24505db7e24a150fd001c4a65516bde31c5ca980b14bdabf27617`
- Pack SHA-256：`d98db87235bffb7b0aaacfc16ddc211c360970fcdab06ca9544b5ecb2349802c`
- Pack validation：通过。

### Prop Set

- 源 Job：`ca27fde0-6708-49b2-914d-954a9a403101`
- 最终 replay Job：`a9145818-1743-43b6-a99d-a75fcd56c2b6`
- Provider 请求/费用：0 / 0
- 结果：chest、barrel、signpost、campfire 为 `game_ready`；crate 为 `awaiting_review`
- crate perceptual similarity：0.8125
- crate geometry similarity：0.9711294
- consistency report SHA-256：`5f91917a6673b67c3ee538a1f2f29e031c24566e3dec49e2617528ad1246cc40`
- contact sheet SHA-256：`955472611f2b1e5c29e6d455defd2f6adea3103ea95d43f0de69857bb1efa8a7`
- Pack：未导出，符合失败封锁合同。

## Godot 4.6.3

- 安装对象：最终 Icon Pack
- Godot Job：`7bec9641-3a42-4289-991d-31174c3d72bf`
- 目标：`addons/forge_assets/validation-inventory-icons`
- 结果：plan token 单次领取、安装、验证、`.forge/assets.json` 注册全部成功。
- Godot 4.6.3 headless editor import 成功，五张外部 PNG 均生成 `.ctex`。
- 静态 Icon 类型没有生成 `.tres/.tscn`；项目中不存在 `PackedByteArray`、内嵌 Image 或 `ImageTexture.create_from_image`。

## 回归门禁

通过：

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p providers --test character_generation_contract
scripts/test-cli-product.sh
scripts/test-stage3-static.sh
forge pack validate (real Icon Pack)
Godot 4.6.3 --headless --editor --quit
```

完整 workspace 首次运行发现两个 build orchestrator fixture 使用无五官占位图；Fixture 随后改为生成明确眼睛与嘴部，没有对真实 Provider 或身份门禁加白名单。视频 ticket 测试同时暴露并修复了 ticket 目录未预建、终态 artifact SHA 未更新，以及 fixture ticket ID 重用问题。最终回归全绿。

## 安全与费用

- 本轮 xAI 请求：0。
- 本轮新增 cost ticks：0。
- 在验收 JobStore、授权账本、Pack 和 Godot 项目扫描：API Key、OAuth Token、Device Code、Bearer/Authorization header、临时签名 URL 均为 0 命中。
- 原始 2026-08-06 授权账本和源 Job 保持不变；所有重评创建 child Job。

## 发布裁决

- Icon：代码与真实 Pack/Godot 门槛通过。
- Prop：检测修复通过，但 crate 源像素语义错误，保持阻断；若要完成该 Pack，只能在新的定向费用授权下重生成 crate。
- Character：异步视频和身份基准代码门槛通过；新的真实四动作端到端测试仍待独立授权。
