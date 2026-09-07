# Forge 构图语义与持久授权 P0 离线验收

日期：2026-08-06  
结论：通过  
真实 xAI 请求：0

## 验收范围

本轮修复 Stage 3 Portrait 验收中暴露的两个 P0：对话半身结果被误读为“全身完整”，以及
新的 CLI/Provider 实例会重置逐 item 请求计数。验收严格使用合成 PNG、`fixture` Provider
和本机 loopback HTTP，不读取或调用真实 xAI 凭据。

## 构图合同

| 门槛 | 结果 | 权威证据 |
|---|---|---|
| Portrait V1 继续是 dialogue bust | 通过 | `portrait_v1_resolves_to_legacy_dialogue_bust_profile`；Stage 3 旧 Portrait Pack 成功 |
| 同一明显 bust 在 full-body 下阻断 | 通过 | `obvious_bust_is_blocked_by_full_body_profile` |
| 合成完整全身通过 | 通过 | `valid_synthetic_full_body_passes_the_proxy` |
| 长袍/遮挡不虚假自动通过 | 通过 | `ambiguous_robe_requires_review_instead_of_claiming_leg_semantics` |
| full-body 端到端不导出错误 Pack | 通过 | `full_body_portrait_contract_blocks_a_bust_before_pack_export`；`happy` 为 `blocked`，无 `gsfpack` |
| dialogue bust 兼容 | 通过 | `fixture_delivers_stage3_static_types_and_zero_cost_replacement` |
| Character 使用独立 sprite 合同 | 通过 | Character video、keyframe 与 GameArt Build 回归全部通过，报告 profile 为 `character_sprite@1.0.0` |
| Pack / Godot 保留 profile 与报告 SHA | 通过 | Stage 3 contract 校验 `forgepack.json` 与 `forge_usage.json` |

报告只包含 `lowerBodyPresenceProxy`、轮廓拓扑、尺寸和安全边距，不包含或暗示
`legsVerified`。全身门禁在 matting 后、归一化前运行；对话半身可以 reframe，全身禁止。

## 持久授权合同

| 门槛 | 结果 | 权威证据 |
|---|---|---|
| 未授权 target 发网前拒绝 | 通过 | `durable_authorization_rejects_missing_target_before_network` |
| 多 Provider 实例共享额度 | 通过 | `durable_authorization_is_shared_across_xai_instances` |
| 并发不能突破总额度 | 通过 | `reservations_are_atomic_across_independent_sessions` |
| parent/child/replay 共用逐 item 上限 | 通过 | `parent_child_and_replay_share_one_target_cap` |
| submitted 后断连仍占额度 | 通过 | `submitted_transport_failure_remains_ambiguously_consumed` |
| 实际费用替换预留 | 通过 | `settlement_replaces_reservation_with_observed_cost` |
| 持久文件无凭据字段 | 通过 | `persisted_documents_have_no_credential_fields` |
| CLI 可创建、查看并绑定授权 | 通过 | fixture CLI smoke；成功 Job 保存 `authorization_id=style-auth` |

授权账本记录 target、operation、model、Job/lineage、状态、请求数和 cost ticks；不记录
prompt、Token、Device Code、授权头、临时 URL 或媒体。

## 已运行命令

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
FORGE_REQUIRE_GODOT=1 FORGE_GODOT_PATH=/Applications/Godot.app/Contents/MacOS/Godot scripts/test-stage3-static.sh
scripts/test-cli-product.sh
scripts/test-real-provider-budget-guard.sh
```

全部通过。Stage 3 脚本完成 Godot 4.6 headless 安装合同、外部纹理和无内嵌图片检查；CLI
product 与旧环境预算 guard 兼容合同均通过。

## 尚未声称的能力

- 纯 Alpha 轮廓不能证明左右腿或脚的语义存在；模糊样本仍需人工或未来经许可审计的视觉组件。
- 本轮没有重新调用真实 xAI，因此它不是新的商业真实模型门槛，也不改变此前真实产物。
- 下一次真实 Portrait 定向修复必须先创建持久授权，并由账本约束全部 parent/child/replay 请求。

机器可读摘要：
[`docs/qa/artifacts/forge-framing-authorization-p0-20260806/summary.json`](artifacts/forge-framing-authorization-p0-20260806/summary.json)。
