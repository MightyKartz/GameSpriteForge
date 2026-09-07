# Forge 构图语义与真实 Provider 授权 P0 实施计划

日期：2026-08-06  
状态：离线实施与验收完成；尚未启动新的真实 xAI 验收  
真实 Provider：本计划的实现与验收阶段禁止调用；只允许 fixture、合成 PNG 和 loopback HTTP 服务。

## 1. 结论与目标

Stage 3 的真实 Portrait Pack 证明了 Forge 可以生成、修复、打包并安装一套对话肖像，
但没有证明角色腿脚或全身结构完整。现有 `portrait-reframe@1.0.0` 会把高而窄的候选图
裁向胸像 anchor，而当前质量门禁只比较 Alpha 外框、面积、中心和行占用，因此不能把
`portrait_set` 的 `game_ready` 解释成 Character 全身完整性通过。

同一轮真实验收还暴露了真实 Provider 授权只存在于单个 CLI 进程内：每次创建新的
`XaiProvider` 都会重置请求计数，导致 sibling/child Job 可以累计超过用户声明的单 item
次数。本计划把这两项同时视为下一次真实调用前的 P0。

目标：

- 保留 `.gsfpack` 的 `portrait_set` 类型和 V1 读取兼容性；
- 新增显式、版本化的对话胸像与全身肖像构图 profile；
- 明显缺失下半身的全身肖像不能进入 Pack；
- 纯几何无法证明腿脚语义时不得虚假宣称通过，而应进入审核；
- Provider 授权、请求和费用预留跨进程、parent/child/replay Job 持久生效；
- 计划允许范围外的 target 或超过 per-target/total 上限的调用在网络请求前失败；
- 失败、重试和恢复继续保持源 Job/Pack 不变，本地重放保持零 Provider 请求。

## 2. 公开构图合同

Portrait V1 继续读取为隐式 `dialogue_bust@1.0.0`。新增 Portrait V2：

```json
{
  "schemaVersion": "2",
  "kind": "portrait_set",
  "framingProfile": "full_body@1.0.0"
}
```

V2 支持：

- `dialogue_bust@1.0.0`：允许胸像、允许确定性 reframe，不要求下半身；
- `full_body@1.0.0`：禁止 reframe，使用 feet grounding，要求下半身存在代理指标和安全边距；
- Character 保持独立 `character` 资产类型，内部使用 `character_sprite@1.0.0` 合同。

纯 Alpha 几何不会输出 `legsVerified`。V1 报告只输出诚实的
`lowerBodyPresenceProxy`、底部支撑、前景纵横比和安全边距。明显胸像为 `blocked`；长袍、
俯视遮挡等无法由几何确定的情况为 `awaiting_review`。未来只有经过许可证审计的姿势或
人体分割组件才能提供语义化左右腿/脚门禁。

## 3. 持久化授权合同

授权由不含凭据的 `AuthorizationManifestV1` 与可变 `RequestLedgerV1` 组成。Manifest
固定 Provider/profile/model、lineage、允许 target、per-target/total 请求上限、费用预留、
计划/输入哈希和过期时间。Ledger 在文件锁内原子执行：

```text
reserve -> submitted -> settled | ambiguous
```

- 网络调用前必须 reserve；
- 一旦提交到远端，传输失败或进程崩溃按 `unknown` 保守占用额度；
- 响应成功后以实际 `costInUsdTicks` settle；
- 同一 authorization 的多个 Provider 实例和多个 Job 读取同一 Ledger；
- Ledger 只保存 job/node/target/attempt、请求指纹和费用数字，不保存 prompt、Token、授权头
  或临时 URL。

现有 `FORGE_REAL_PROVIDER_*` 环境变量保留为旧自动化兼容入口。新付费验收使用
`forge provider authorize` 创建不含凭据的持久授权，并通过 `--authorization` 绑定到执行
Job。Job 保存 `authorizationId` 与 `lineageRootJobId`，child/retry/replay 默认继承；显式传入
新的授权时才替换。

## 4. 实施顺序

1. 新增构图类型、V2 schema、CollectionLock profile 和离线几何报告。
2. 在 raw/matted 原分辨率阶段执行 profile 门禁；只有对话胸像允许 reframe。
3. 把 profile、报告 SHA 和 verdict 写入 Job、Pack provenance 与 Godot usage。
4. 新增持久化授权 Manifest/Ledger 和原子 reserve/settle。
5. 给 Provider 调用增加非敏感 target/node/attempt 上下文，并在 CLI plan execute 绑定授权。
6. 增加跨 Provider 实例、跨 Job lineage、崩溃未知状态和定向 target 的离线测试。
7. 运行 Stage 3、CLI product、Pack、Godot、安全扫描和 workspace 回归。

## 5. 验收门槛

- V1 Portrait 无腿按 `dialogue_bust@1.0.0` 继续可读；
- 同一像素按 `full_body@1.0.0` 必须因缺失下半身阻断；
- 合成完整全身图通过；脚部裁切阻断；长袍模糊样本不得自动通过；
- full-body 路径不得执行 `portrait-reframe`，未修改 item 的 SHA 保持不变；
- Collection profile 变化必须产生不同 revision；Pack/Godot 保留 `portrait_set`；
- parent 已消费一次后，child 只能使用剩余额度；第三个 `happy` 请求在网络前拒绝；
- `retryItemIds=[happy]` 时 Provider Ledger 中不得出现 `angry`；
- 两个 CLI/Provider 实例并发使用同一授权时不得突破总上限；
- submitted 后断连的请求仍占额度；Ledger 和所有产物通过凭据/临时 URL 扫描；
- 计划最大请求数与执行器硬上限一致；超限不得导出 Pack、注册 Catalog 或安装 Godot。

## 6. 非目标

- 本轮不调用 xAI，不修改模型或降低现有质量阈值；
- 不新增桌面、MCP、Unity、Unreal、地图或建筑功能；
- 不把纯 Alpha 启发式包装成语义腿脚识别；
- 不修改或删除现有真实源 Job、Pack 和 QA 证据。

## 7. 实施结果

- Portrait V1 保持隐式 `dialogue_bust@1.0.0`；Portrait V2 和 Collection Lock 支持显式
  `full_body@1.0.0`，且 profile 进入不可变 revision/input fingerprint。
- 全身门禁在 provider 原图 matting 后、裁切缩放前执行；全身路径禁止
  `portrait-reframe@1.0.0`。明显 bust 为 `blocked`，长袍代理样本为
  `awaiting_review`，报告没有 `legsVerified`。
- Character 视频方向静帧和关键帧均写入 `character_sprite@1.0.0` geometry 报告。
- Pack provenance 和 Godot `forge_usage.json` 保留 geometry profile、报告 profile 与
  `consistency-report.json` SHA-256。
- `AuthorizationManifestV1` / `RequestLedgerV1` 使用文件锁和原子写入；xAI 图片、视频、视频
  编辑和私有文件上传在发网前 reserve。未知 target、模型或已耗尽额度在发网前失败。
- `forge provider authorize|authorization`、高层生成命令与 `forge plan execute` 支持持久授权；
  `job report` 返回不含秘密的 manifest/ledger。
- parent/child/replay 共享逐 target 上限；两个独立 Provider 实例并发无法突破总额度；提交后
  断连保守记为 `ambiguous` 并占用费用预留。
- 2026-08-06 全部实施验收只使用合成 PNG、fixture 和 loopback HTTP；xAI 请求数为 0。

验收证据见
[`docs/qa/forge-framing-authorization-p0-2026-08-06.md`](../qa/forge-framing-authorization-p0-2026-08-06.md)。
