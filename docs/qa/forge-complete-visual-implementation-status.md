# Forge 完整视觉资产系统实施状态

状态：Active — 阶段 0 完成，等待验收
负责人：Kimi 实施，Codex 分阶段验收
计划文档：`docs/architecture/forge-complete-visual-asset-system-implementation-plan.md`
基线提交：`69e6aec`（`codex/v0.3-character-benchmarks`）

## 阶段总览

| 阶段 | 状态 | 证据 |
| --- | --- | --- |
| 0 基线冻结与仓库卫生 | ✅ 完成（待 Codex 验收） | 本文档 + baseline JSON |
| 1 v0.3 Character 真实门槛 | ⚠️ 已执行 — **未晋级** | `forge-character-v2-real-gate-2026-08-04.md` |
| 2 GameArtManifest 与项目 Build Core | 未开始 | — |
| 3 CollectionLock、Portrait 与 Static 完整化 | 未开始 | — |
| 4 World 资产正式化 | 未开始 | — |
| 5 Background 与 UI | 未开始 | — |
| 6 VFX 与 EffectLock | 未开始 | — |
| 7 Project Audit、Godot Sync 与 Gallery | 未开始 | — |
| 8 Forge 1.0 冻结视觉项目 | 未开始 | — |

## 阶段 0：基线冻结与仓库卫生（2026-08-04）

### 完成项

1. **git 快照保存**：`git status --short` 完整快照存于
   `docs/qa/artifacts/forge-complete-visual-baseline-2026-08-04/git-status.txt`。
   未清理、未删除、未覆盖任何用户未提交文件。
2. **v0.3 基线回归**：七项全部通过，零预先存在失败。机器可读报告：
   `docs/qa/artifacts/forge-complete-visual-baseline-2026-08-04/baseline.json`。

   | 步骤 | 结果 |
   | --- | --- |
   | `cargo fmt --all -- --check` | pass |
   | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | pass（0 warning） |
   | `cargo test --workspace --no-fail-fast` | pass（135 passed / 0 failed，21 个测试二进制） |
   | `scripts/test-cli-product.sh` | pass |
   | `scripts/test-v03-release-matrix.sh` | pass（6/6 门，verdict=pass；报告 `target/qa/v0.3-release-matrix-20260804T050933Z/`） |
   | `scripts/test-cli-installer.sh` | pass（全离线沙箱） |
   | `scripts/test-cli-signing-contract.sh` | pass |

   运行前已审查全部脚本：无真实 Provider 调用（xai 仅出现在离线成本估算与被预期拒绝的
   费用守卫中），未设置 `FORGE_REAL_PROVIDER_ACCEPT`。
3. **README 同步**：修复 `README.md` / `README.zh-CN.md` 过期 v0.2 发布状态描述——
   `v0.2.0-cli.1` 已于 2026-08-03 发布；consistency-v2 / world 的发布线锚点更新为
   v0.3，并注明六门矩阵已于 2026-08-04 通过、真实模型晋级门槛仍未完成。
   如实保留：全新账户安装验证尚未在 `docs/qa/` 记录，未夸大为已通过。
4. **QA artifact policy**：新建 `docs/qa/forge-qa-artifact-policy.md`。仓库保留报告、
   contact sheet、精选 preview 与哈希；不提交完整 JobStore、原始视频、大 Pack、
   Token cache。
5. **`.gitignore` 决定**：不新增任何规则。既有 `docs/qa/artifacts/` 下的历史证据目录
   处于未跟踪状态，任何追溯式忽略规则都会覆盖它们（计划硬约束禁止）；新证据通过
   「JobStore 重定向到 `target/qa/` + 精简证据入库」的流程控制，而非忽略规则。
6. **本状态文档**建立。

### 阶段 0 验收核对

- 六门 v0.3 matrix 在本提交上不变：✅（基线 6/6 pass，与
  `docs/qa/forge-v03-release-matrix-2026-08-04.json` 记录一致）
- v0.2 CLI product surface 无新命令：✅（默认构建 `--help` 快照
  `cli-help-baseline.txt`：doctor/asset/pack/job/project/style/generate/godot/
  profile/provider/repair/plan，未变）
- 没有删除现有用户文件：✅（仅新增 QA 文档与证据，仅编辑两个 README 的过期段落）

### 基线运行备注

- 基线代理记录了一次"运行中外部修改"：README 两文件在回归运行期间被修改。
  已核实为本次阶段 0 的 README 同步编辑（任务 3），非异常。
- 未运行任何真实（付费）xAI 调用；全部 fixture / 离线。

## 阶段 1 预算协议

阶段 1（v0.3 Character 真实 xAI A/B 晋级门槛）需要真实 Provider 费用。按协议：
默认拒绝真实调用，预算由 Codex/用户批准后执行，超出即阻断。
## 阶段 1 结果（2026-08-04）

用户批准 3 次真实闭环（上限 250 请求 / 150B ticks）。三次全新目录真实 xAI
Character V2 → Pack → Godot 运行均**未通过一致性门禁**（14/15/22 帧 game_ready，
Pack 成功率 0/3），`topdown-keyframes@2.0.0` 未晋级：video 保持默认，keyframe 保持
experimental，未降低阈值。实际费用 167 请求 / 132.1B ticks（$132.10），预算内。
完整证据与逐门槛核对：`docs/qa/forge-character-v2-real-gate-2026-08-04.md`。
冻结 20×5 真实 benchmark 暂缓执行（成功率门槛数学上不可达），待 Codex 决策。

备注：计划中预设的 `FORGE_REAL_PROVIDER_*` 环境变量守卫在当前代码库尚不存在；
本次运行通过「运行前离线 plan + 逐次核对 provider-usage.json」的程序化预算守卫执行。

## 变更记录

| 日期 | 阶段 | 内容 | 证据 |
| --- | --- | --- | --- |
| 2026-08-04 | 0 | 基线冻结、README 同步、QA artifact policy、状态文档建立 | `docs/qa/artifacts/forge-complete-visual-baseline-2026-08-04/` |
| 2026-08-04 | 1 | 三次真实 xAI Character V2 闭环，未晋级；费用 167 请求 / $132.10 | `docs/qa/forge-character-v2-real-gate-2026-08-04.md` + `docs/qa/artifacts/forge-character-v2-real-20260804/` |
