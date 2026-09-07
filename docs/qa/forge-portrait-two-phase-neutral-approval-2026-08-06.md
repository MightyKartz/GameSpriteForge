# Forge Portrait V2 两阶段 Neutral 审批验收

日期：2026-08-06  
结论：离线实现与兼容性验收通过；本轮未调用真实 xAI。

机器可读摘要：
[`summary.json`](artifacts/forge-portrait-two-phase-neutral-approval-20260806/summary.json)

## 已实现合同

Portrait V2 不再一次生成 neutral 和四个表情，而是建立明确的费用与人工审批边界：

1. `--phase base` 只生成 neutral，预计 1 次、最多 2 次图片请求；生成
   `PortraitBaseLock` 后停在 `awaiting_review`，不导出 Pack。
2. `forge job review --accept` 把审批绑定到源 Job、Lock SHA-256、neutral PNG
   SHA-256 和 reference policy。任何一个输入改变都会让审批失效。
3. `--phase expressions --base-job <id>` 只生成 `happy`、`angry`、`hurt`、
   `surprised`，预计 4 次、最多 8 次请求；审批验证在 Provider 请求之前完成。
4. Expressions Job 不能重试 neutral。要更换基准图，必须创建新的 Base Job 并重新审批。
5. 表情仍以获批 neutral 作为唯一 edit target，并用确定性 face scope 恢复脸部范围外像素。

Neutral reference policy 已版本化：

- `subject-style@1.0.0`：默认；Subject identity 在前、Style board 在后，Collection
  约束写入 prompt/provenance，不再把可能冲突的 anchor 图传给模型。
- `subject-edit@1.0.0`：只把 Subject canonical 作为 edit target。
- `legacy-three-reference@1.0.0`：保留 Style、Collection anchor、Subject 的旧顺序，
  仅用于显式对照和兼容。

## 离线 Fixture 验收

`packages/providers/tests/stage3_static_contract.rs` 和
`scripts/test-stage3-static.sh` 已验证：

- 三种 policy 的参考角色和顺序准确；
- Base 计划预算固定为 1/2，且不导出不完整 Pack；
- 未审批、审批内容篡改或 neutral/Lock SHA 不匹配时，在零 Provider 请求处失败；
- Expressions 计划预算固定为 4/8，实际 Fixture 请求恰好为 4；
- Pack 含 approval、policy 与来源哈希，并通过校验；
- Godot 4.6.3 headless 安装、场景加载、外部纹理和 provenance 通过；
- 从 Expressions Job 重试 neutral 被 `portrait_base_immutable` 拒绝。

## 冻结真实产物零费用回放

旧 schema V4 xAI Job `46eb19a1-bd47-47be-b9f0-09e27019f667` 以
`--stage consistency` 重放为 Job `6800de64-eb2a-44c2-8baa-6c09750ba2c4`。

- Provider 请求：0；
- neutral、happy：`game_ready`；
- angry：`awaiting_review`；
- hurt：因保护肤色区域伪影进入 `regenerate`；
- surprised：因肤色漂移进入 `regenerate`；
- 最终 Pack：未导出；
- JobStore 敏感信息与临时媒体 URL 扫描：通过。

这证明两阶段新增字段保持了旧 Job 可读性，并且本地重检不会误产生费用或把已知坏表情导出。

## 回归门禁

- `cargo fmt --all -- --check`：通过；
- `cargo clippy --workspace --all-targets -- -D warnings`：通过；
- `cargo test --workspace`：通过（261 项测试）；
- `scripts/test-stage3-static.sh`：通过；
- `scripts/test-cli-product.sh`：通过；
- Godot：`4.6.3.stable.official.7d41c59c4`。

## 尚未执行

真实新图验收不属于本轮授权范围。下一次真实验收应拆成两份最小权限授权：Base 只允许
`neutral`（预计 1、最多 2 次），Expressions 只允许四个表情（预计 4、最多 8 次）；合计
预计 5、最多 10 次图片请求。只有新的 neutral 经人工批准后才能启动第二阶段。
