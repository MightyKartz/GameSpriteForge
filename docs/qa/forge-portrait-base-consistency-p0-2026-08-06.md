# Forge PortraitBaseLock 与局部一致性 P0 验收

日期：2026-08-06  
结论：离线实现与冻结真实产物回放通过；真实 xAI 再生成未执行。

## 验收范围

- Portrait V1 保持兼容。
- Portrait V2 强制 `neutral` 为第一项。
- neutral 使用 Style、Collection、Subject 建立不可变 `PortraitBaseLockV1`。
- 其余四个表情只引用同一个 neutral `edit_target`，不再重复发送互相冲突的参考图。
- 最终输出仅接纳确定性 face scope 内的候选像素，范围外 RGBA 必须逐像素等于 neutral。
- 局部报告覆盖肤色 CIELAB 色差、脸部边缘密度、感知相似度和脸颊保护区暗色伪影。
- Pack、CLI review、Godot usage、Project audit 和 retry/replay 使用同一报告闭包。

## Fixture 与回归结果

- `cargo fmt --all -- --check`：通过。
- `cargo clippy --workspace --all-targets -- -D warnings`：通过。
- `cargo test --workspace`：通过；新增 Portrait 测试包含保护区逐像素保持、肤色/脸部伪影拦截、neutral 首位和 Provider reference-role 合同。
- `scripts/test-stage3-static.sh`：通过；3/3 Stage 3 合同测试通过，并完成 Godot 可用环境下的安装闭环。
- `scripts/test-cli-product.sh`：通过。
- V2 fixture Pack 同时通过 `.gsfpack` 校验、去本机路径检查、Project audit 与 Godot `forge_usage.json` provenance 检查。

## 冻结真实 xAI 产物零费用回放

源 Job：`46eb19a1-bd47-47be-b9f0-09e27019f667`  
回放 Job：`9038956a-033c-43c9-9a3b-7c8c93cd9d35`  
运行方式：`forge job retry --stage consistency --wait --json`  
Provider 请求：`0`  
回放结论：`awaiting_review`，未导出 Pack。

| 表情 | 局部结论 | 主要证据 |
|---|---|---|
| neutral | `game_ready` | 建立 BaseLock，SHA-256 保持不变 |
| happy | `game_ready` | 肤色色差 5.92，保护区伪影比 0.0131 |
| angry | `awaiting_review` | 肤色色差 11.01，脸部细节灰区 |
| hurt | `regenerate` | 脸颊保护区伪影比 0.0871、严重伪影比 0.0424 |
| surprised | `regenerate` | 肤色色差 13.56，脸部细节灰区 |

五个输出的 face scope 外变化比例均为 `0.0`。因此围巾、服装、身体、手、腿和鞋全部
继承 neutral；原始候选中这些区域的漂移不会进入游戏资产。用户指出的 hurt 脸部划痕和
surprised 肤色漂移已被机器门禁拒绝，且硬失败不能通过 `job review` 强制导出。

机器摘要见
[`artifacts/forge-full-body-real-20260806/portrait-base-replay-summary.json`](artifacts/forge-full-body-real-20260806/portrait-base-replay-summary.json)。
完整本地报告位于回放 Job 的 `portrait-consistency-report.json`，合成 contact sheet 的
SHA-256 为 `9a4eb76e38382fe1b0dd07968fb94dcdf2f21da0361707f9147d4ce2b455a889`。

## 安全与发布判断

- 本次实现与验收没有访问 xAI 网络端点，也没有消耗此前真实 Provider 授权。
- `provider-usage.json` 明确记录 `requests: 0`。
- 缺陷仍存在时没有 `.gsfpack` 目录产生。
- Job/Pack/Godot provenance 不包含 Token、Authorization header、Device Code 或临时 URL；
  Pack 与 Godot provenance 额外移除了 JobStore 的本机工作路径。
- 下一次真实 Provider 验收应作为新的、逐目标与费用上限明确的授权执行；本次不自动沿用旧授权。
