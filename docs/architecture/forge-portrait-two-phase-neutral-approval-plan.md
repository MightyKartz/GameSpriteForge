# Forge Portrait V2 两阶段 Neutral 审核与参考策略计划

## 目标

解决 `PortraitBaseLock` P0 仍未覆盖的问题：Forge 可以让所有表情一致地继承 neutral，
但如果 neutral 自身身份、围巾、肤色或装备错误，后续会稳定地复制错误。本阶段把 neutral
从普通的第一项提升为独立、可审核、哈希绑定的费用检查点。

本阶段不调用真实 Provider。真实 xAI 对比必须在离线实现通过后由新的逐目标授权执行。

## CLI 合同

Portrait V1 保持原有一次性生成。Portrait V2 的公开 CLI 改为两阶段：

```text
forge generate portrait-set \
  --project <path> --spec <portrait-v2.json> \
  --phase base \
  [--neutral-reference-policy subject-style|subject-edit|legacy-three-reference] \
  [--plan-only|--wait] --json

forge job review \
  --id <base-job-id> --accept \
  --reason "neutral identity and equipment approved" --json

forge generate portrait-set \
  --project <path> --spec <portrait-v2.json> \
  --phase expressions --base-job <approved-base-job-id> \
  [--plan-only|--wait] --json
```

- V2 默认 `--phase base`；`expressions` 必须引用已成功审核的 base Job。
- Base 阶段只授权 `neutral`，预计 1 次、最多 2 次请求。
- Expressions 阶段只授权 `happy/angry/hurt/surprised`，预计 4 次、最多 8 次请求。
- Base 阶段即使机器门禁为 `game_ready` 也不会生成其余表情或导出 Pack。
- `job review --accept` 对 base Job 只写入 approval，不导出不完整 Pack。
- 旧 recipe 缺少 phase 时继续按 `legacy_all` 读取，保证历史 Job 可回放。
- 新两阶段 recipe 使用 schemaVersion `5`；旧 schemaVersion `4` 只能读取
  `legacy_all`，防止新调用静默绕过审核检查点。

## 版本化参考策略

`PortraitNeutralReferencePolicyV1` 固定三种策略：

- `subject-style@1.0.0`（默认）：有序引用 Subject identity、Style board；Collection 仅作为
  构图 provenance 与 prompt 约束，不发送其可能冲突的 anchor 图片。
- `subject-edit@1.0.0`：Subject canonical 作为唯一 edit target；Style 只通过版本化 prompt
  与锁定描述提供。
- `legacy-three-reference@1.0.0`：Style、Collection anchor、Subject 三图，保留用于明确的
  A/B 对比和旧行为复验，不作为默认。

Provider 可能不理解 Forge 的 ReferenceRole，因此 Lock 和报告必须保存 policy、引用顺序与
SHA-256，禁止静默重排或回退。

## Approval 闭包

新增 `PortraitBaseApprovalV1`：

- profile：`portrait-base-approval@1.0.0`
- source Job ID、asset ID
- PortraitBaseLock 文件 SHA-256
- neutral PNG SHA-256
- neutral reference policy
- 审核结果、原因和时间

Expressions 计划和执行都必须验证：

1. base Job 为 `succeeded`；
2. approval 为 accepted；
3. approval 的 Job、asset、policy、Lock SHA 和 neutral SHA 与磁盘闭包一致；
4. Style、Subject、Collection、Provider/profile/model 与新 spec 完全一致；
5. base Job 没有 Pack，源 Job 和 approval 不被子 Job 修改。

任何不一致返回稳定错误，不调用 Provider。

## 执行与重试

- Base Job 只运行 neutral → matting/normalize → geometry/consistency → BaseLock → reports。
- Expressions Job 复用 neutral 与 Provider edit source，只生成四个表情，然后执行局部合成、
  完整一致性、Pack 和 Godot 闭环。
- 重试 neutral 创建新的 Base Job，并要求重新审核；旧 approval 不可复用。
- 单个表情重试沿用已经批准的 base，不重新生成 neutral。
- consistency replay 保持零费用；旧 `legacy_all` Job 继续可回放。

## 验收门槛

1. Fixture 验证 Base 计划为 1/2 请求，Expressions 为 4/8，且授权目标精确。
2. Base 成功后保持 `awaiting_review`、没有 Pack；批准后写入哈希绑定 approval。
3. 未批准、已拒绝、Lock/PNG/approval 被篡改、spec/Provider/model/policy 不匹配时，
   Expressions 在计划阶段失败且 Provider 请求为 0。
4. 三种 neutral policy 的引用角色、顺序和 SHA-256 均有合同测试。
5. Expressions 只引用已批准 neutral；最终 Pack/Godot usage 记录 policy 与 approval provenance。
6. Portrait V1、旧 V2 recipe、其他静态资产、retry/replay、Project audit 全部回归通过。
7. 对 2026-08-06 冻结真实产物只执行零费用 replay，不执行新的 xAI 请求。
