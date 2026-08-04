# Forge 阶段 1 收尾（2026-08-04）

状态：**实施完成，等待 Codex 复验**

范围：只修复阶段 1 验收指出的五项缺口；不降低既有一致性阈值，不将
`topdown-keyframes@2.0.0` 晋级为默认，不执行新的真实 xAI 请求。

## 五项收尾结果

1. **真实样本硬缺陷门禁**：新增 `keyframe-hard-defects@1.0.0`，在关键帧一致性评分前
   阻断不透明矩形背景、低 Alpha 离散噪点、极端轮廓比例漂移和姿势引导图颜色泄漏。
   硬失败不能进入 Pack，也不能由人工审核越过。归一化阶段把 Alpha `< 32` 的插值残留
   确定性清零，避免透明背景正常采样产生同类误报。
2. **真实 Provider 费用守卫**：CLI 和 xAI Provider 均要求
   `FORGE_REAL_PROVIDER_ACCEPT=1`，并要求正数
   `FORGE_REAL_PROVIDER_MAX_REQUESTS`、`FORGE_REAL_PROVIDER_MAX_COST_TICKS`。
   请求数在发送前原子预留；累计响应费用超过上限或响应缺少费用字段时返回稳定错误码，
   不继续执行后续生成。fixture 和 loopback 合同测试不受影响。
3. **模型来源闭合**：Provider 统一解析默认图片/视频模型；plan、Style/Subject/Environment
   Lock、WorkflowGraph、Provider manifest 和 Catalog 均写入实际解析后的模型名，不再以
   `null` 代表 xAI/fixture 默认模型。Provider 或模型不同仍不得命中远程产物缓存。
4. **QA 证据更正**：运行 3 的 `edge_density_drift` 实际为 8/10，已同步修正门槛报告和
   `run-3/run-report.json`；三次真实运行的费用、verdict 和“未晋级”结论均未改变。
5. **干净分支交付**：从当前 `origin/main` 建立 `codex/stage1-closure`，仅依次承接阶段 0、
   阶段 1 证据提交并增量加入本收尾；原始脏工作树、桌面端和 MCP 源码均未修改或删除。

## 复验命令

以下命令全部使用隔离工作树内的 workspace 二进制和临时 JobStore/PlanStore：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --no-fail-fast
bash scripts/test-cli-product.sh
bash scripts/test-v03-release-matrix.sh
bash scripts/test-cli-installer.sh
bash scripts/test-cli-signing-contract.sh
```

定向合同：

```bash
bash scripts/test-real-provider-budget-guard.sh
cargo test -p core keyframe_hard_gates_block_background_pose_leak_noise_and_extreme_silhouette
cargo test -p providers --test keyframe_generation_contract
cargo test -p providers xai::tests::real_provider_budget_requires_acceptance_and_enforces_request_and_cost_limits
cargo test -p providers xai::tests::xai_resolves_default_models_for_provenance
```

## 安全与费用

- 本收尾未设置真实 Provider 接受环境变量，未读取 xAI 凭据，未发起真实 xAI 请求，费用为 0。
- 测试只使用 fixture、localhost 伪服务和在网络前预期拒绝的 xAI CLI 路径。
- Job、Pack、报告和普通日志不得包含 Token、Authorization header、Device Code 或临时媒体 URL。

## 本次执行结果

| 门禁 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | PASS |
| Clippy（workspace / all targets / all features / warnings denied） | PASS，0 warning |
| `cargo test --workspace --no-fail-fast` | PASS，138 passed / 0 failed |
| `scripts/test-cli-product.sh` | PASS |
| `scripts/test-v03-release-matrix.sh` | PASS，6/6（含 Character V2、静态资产与实验世界资产） |
| `scripts/test-cli-installer.sh` | PASS |
| `scripts/test-cli-signing-contract.sh` | PASS |
| 五项定向合同 | PASS |
| 当前 diff 的凭据形态扫描 | PASS，0 match |

v0.3 矩阵机器报告由测试写入隔离工作树的 `target/qa/`，按 artifact policy 不提交临时
JobStore、完整帧集或大型 Pack。

## 复验裁决边界

- 阶段 1 仍为 `not_promoted`；video 继续默认，keyframe 继续 experimental。
- 本提交只请求验收五项收尾，不请求启动阶段 2，也不请求执行冻结 20×5 付费 benchmark。
- Codex 复验通过后，本工单停止，由用户决定是否把该分支合入 `main` 并启动阶段 2。
