# Forge 阶段 2：GameArtManifest 与项目 Build Core 验收（2026-08-05）

状态：**原始实施完成；Codex 复验 blocker 已由后续修复关闭，见 `forge-stage2-codex-remediation-2026-08-05.md`**

范围：`GameArtManifestV1` 校验、ProjectCatalogV2、`project diff` / `project plan-build`、
BuildProject 编排器（父子 Job、取消级联、崩溃恢复），以及 `game-art-manifest` feature 后的
CLI `forge project diff|plan-build`。本波为验收证据波：只新增 focused script、示例与 QA
文档；验收中发现的一处 CLI 派发缺陷已由实现方修复并复验（见第 5 节）。

> 后续验收说明：本文件保留 Kimi 原始验收证据与历史数字。Codex 深度复验发现并修复了
> 输入闭包、Pack 真实性、TOCTOU、恢复并发、请求上限和 usage 重复等问题；当前结论、
> commit 与新回归结果以 `docs/qa/forge-stage2-codex-remediation-2026-08-05.md` 及其机器报告为准。

## 1. 实施摘要（Implementation summary）

阶段 2 让现有已稳定的 Character / Icon / Prop 资产可以被项目级 manifest 统一规划和构建：

- `GameArtManifestV1`：JSON Schema（`schemas/game-art-manifest.schema.json`）+ Rust 类型；
  严格 path/symlink/URL/traversal 检查；依赖解析（重复 id、未知引用、自依赖、cycle、
  required→optional 依赖拒绝）；normalized manifest 与 graph SHA-256（键序/空白无关）。
- `project diff`：对照 `.forge/catalog.json` 与 Style/Subject Lock 计算每资产
  reuse/build/rebuild/orphan 判定与机器可读 reason codes；阶段 2 只报告 delete candidates，
  从不删除。
- `project plan-build`：离线 plan（workflow 静态分派：character→`topdown@1.0.0`，
  icon_set/prop_set→`static-set@1.0.0`；provider 请求数估计；`catalog_reuse` cache 计数；
  cost ticks 显式为 null），并签发 automation 层 single-use plan token；执行走既有
  `forge plan execute`。
- BuildProject 编排器：durable 父 Job + 每资产一个子 Job（拓扑序）；父 Job 自身零 provider
  调用；`build-state.json` 逐转换原子落盘用于崩溃恢复；取消为协作式（每个子 Job 前检查父
  标记，`request_cancellation_cascade` 级联到活动子 Job）；required/optional 语义
  （只有 required 失败才使父 Job 失败；依赖失败沿 `dependsOn` 毒害）；
  `reconcile_interrupted_builds` 把 worker 已死的 Running build Job 标记为可恢复失败。
- ProjectCatalogV2：兼容读取 V1 catalog、原子写入、并发注册不丢更新；阶段 2 provenance
  （specSha256、locks、workflow、provider/profile/model、质量 verdict、gameReady）完整。

## 2. 变更文件（Changed files）

快照：`docs/qa/artifacts/forge-stage2-game-art-manifest-20260805/git-status.txt` 与
`git-diff-stat.txt`（在验收修复之后捕获）。清单：

- 修改：`packages/cli/Cargo.toml`（`game-art-manifest` feature）、`packages/cli/src/main.rs`
  （`project diff|plan-build` 命令 + 验收修复）、`packages/core/src/automation/{mod,plan,
  repair,runner,types}.rs`（`AutomationOperation::BuildProject`、`BuildProjectRequestV1`、
  派发、步骤/估算）、`packages/core/src/catalog.rs`（Catalog V2）、
  `packages/core/src/job/{store,types}.rs`（`JobOperationKind::BuildProject`、取消级联）、
  `packages/core/src/lib.rs`、`packages/core/tests/automation_tests.rs`。
- 新增：`packages/core/src/game_art/`（types/manifest/diff/plan/build）、
  `packages/core/tests/game_art_build_tests.rs`（9 个编排器集成测试）、
  `schemas/game-art-manifest.schema.json`、`examples/cli/complete-visual/game-art.json`
  及 `specs/{hero,hud-icons,forest-props}.json`、
  `scripts/test-game-art-manifest.sh`、本 QA 文档与证据目录
  `docs/qa/artifacts/forge-stage2-game-art-manifest-20260805/`。

## 3. 公共契约变更（Public contract changes）

- 新 feature gate：`forge-cli` 的 `game-art-manifest`（默认关闭；`--all-features` 覆盖）。
- 新 CLI 命令（仅该 feature 下）：`forge project diff --project P --manifest M --json`、
  `forge project plan-build --project P --manifest M --json`（plan-only，返回
  `project_build_plan` + single-use token；执行复用既有 `forge plan execute --token T`）。
- 新 automation operation：`AutomationOperation::BuildProject` /
  `BuildProjectRequestV1 { schemaVersion, projectPath, manifestPath }`；
  新 `JobOperationKind::BuildProject`（JSON `build_project`）。
- 新 schema id：`https://game-sprite-forge.local/schemas/game-art-manifest.schema.json`。
- ProjectCatalogV2 文件格式：`.forge/catalog.json` 升级为 V2（新增 specSha256、
  dependencies、locks、workflowProfile/workflowVersion、provider、qualityVerdict/
  qualityProfile、gameReady、generatedAt、license、provenanceSummary 等字段）；
  V1 catalog 仍可读取，写入为原子写。
- 父 build Job 新增 `project_build_report` artifact（`project-build-report.json`），并在父
  Job 目录维护 `build-state.json` 崩溃恢复记录。
- 验收修复（见第 5 节）：`forge plan execute` 现在从 manifest 解析 BuildProject 的
  provider 并执行真实 provider 费用守卫。

## 4. 兼容性声明（Compatibility statement）

- 默认构建面不变：未启用 `game-art-manifest` 时不出现新命令；
  `scripts/test-cli-product.sh`（默认 feature）通过。
- V1 pack 与 V1 catalog 保持可读；既有 v0.3 六门矩阵 6/6 通过。
- 本阶段验收全部使用 fixture provider 与离线 plan，无真实 provider 调用，未设置
  `FORGE_REAL_PROVIDER_ACCEPT`。
- 阶段 2 不删除任何 orphan 资产（delete candidates 仅报告）。

## 5. 验收中发现并修复的缺陷（Bugs found and fixed during acceptance）

**BUG-STAGE2-ACCEPT-1（已在验收波内修复并复验）**

- 现象：`forge project plan-build` 签发的 token 经 `forge plan execute --token T --wait`
  执行时，只要 plan 含 build/rebuild 动作即失败：
  `automation_failed: asset processing failed: provider fixture must be resolved before
  running this plan`。纯 reuse plan 不受影响。
- 根因：CLI 的 `run_plan_operation`（`packages/cli/src/main.rs`）为所有 operation 解析
  provider，唯独 `AutomationOperation::BuildProject` 落入 `_ => None`；
  `operation_provider_id` 同样缺少该分支（真实 provider 费用守卫也被绕过）。父 build Job
  因此以 `provider=None` 进入编排器。Rust 集成测试直接以 `Some(provider)` 调用
  `run_operation_with_provider`，绕开了 CLI 派发层，所以实现波测试全绿却未发现。
- 发现方式：本验收波 focused script 的 CHECK 2（CLI 端到端首次 build）失败定位；
  验收方按波规则只报告不修复，附最小复现（fixture 项目 + 示例 manifest + plan-build +
  plan execute）与根因定位。
- 修复：实现方在 `packages/cli/src/main.rs` 的 `run_plan_operation` 增加
  `AutomationOperation::BuildProject` 分支——`GameArtManifestV1::load_validated(
  &request.manifest_path)` 读取 manifest 的 `provider.id/profileId`，先经
  `ensure_real_provider_execution` 费用守卫，再 `resolve_provider` 注入；
  manifest 错误经 `game_art_error` 映射为稳定错误码。
- 复验：修复后 focused script 全绿（CHECK 0–8 + PASS），完整回归 8/8 通过
  （见第 6 节与机器报告）。流程归属：验收波（Kimi）报告 + 复验，实现方（Codex）修复。

## 6. 测试执行（精确命令与结果）

完整 §16.5 回归 + 本阶段 focused script，机器可读报告：
`docs/qa/artifacts/forge-stage2-game-art-manifest-20260805/stage2-regression.json`
（每步退出码同目录 `step-*-exit-code.txt`；完整日志在瞬态目录
`target/qa/stage2-regression-20260805T022316Z/`，按 QA artifact policy 不提交）。

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | PASS（无 diff） |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS，0 warning（含 `game-art-manifest`） |
| `cargo test --workspace --no-fail-fast` | PASS，210 passed / 0 failed，19 个测试二进制 |
| `bash scripts/test-cli-product.sh` | PASS |
| `bash scripts/test-v03-release-matrix.sh` | PASS，6/6 门，verdict=pass（报告 `target/qa/v0.3-release-matrix-20260805T022701Z/v0.3-test-report.json`） |
| `bash scripts/test-cli-installer.sh` | PASS |
| `bash scripts/test-cli-signing-contract.sh` | PASS |
| `bash scripts/test-game-art-manifest.sh` | PASS（CHECK 0–8 + `PASS Forge game art manifest contract`） |

Focused script 检查行（证据：`focused-script-output.txt`）：

- CHECK 0：示例 manifest + specs 解析——`forge project diff` 对 fixture 项目报告 3 个
  `new_asset` build（示例同时通过 `schemas/game-art-manifest.schema.json` 的
  jsonschema 校验）。
- CHECK 1：确定性 plan hash `53c5e1c0ac844301…`——同 manifest/spec 两次
  `project plan-build` 的 `.data.plan.planSha256` 一致；`jq -S` 改写键序/空白后仍一致
  （改写前后文件字节不同，检查非空转）。
- CHECK 2：reuse 不产生 provider job——首次 build 后 JobStore 恰为 5 个 Job
  （1 style + 1 parent + 3 children）；同 manifest 再 plan-build + execute，3/3 全部
  `reuse`，`providerUsage.requests == 0`，第二个父 Job 0 个子 Job，Job 总数 5→6
  （仅新增父 Job；0 个新 provider/子 Job）。
- CHECK 3：定向失效——改一个 icon 的 prompt 后仅 `hud-icons` `rebuild`
  （reason `spec_changed`），`hero`/`forest-props` `reuse`；执行后恰新增 1 个子 Job
  （总数 6→8：1 parent + 1 child）。
- CHECK 4：required/optional——required `hud-icons` 失败使父 Job
  `project_build_failed`（recoverable），依赖它的 required `hero` 被跳过
  （`dependency_failed`），无依赖的 optional `bonus-props` 仍构建；镜像场景中唯一失败的
  是 optional `bonus-props`，required `hud-icons` 照常构建且父 Job 成功。
  经由 `forge job report --id <parent>` 上的 `project_build_report` artifact 与
  `forge job get` 记录字段断言。
- CHECK 5：取消传播——`FORGE_FIXTURE_POLL_PENDING_ONCE=1` 下无 `--wait` 执行
  （worker 派生），轮询到 character 子 Job running 后 `forge job cancel --id <parent>`；
  活动子 Job 的 `job.json` `cancellation_requested=true`，父与子最终均为 `cancelled`；
  cancel 命令输出经 jq 验证为单 JSON ok envelope。
- CHECK 6：崩溃恢复——`cargo test -p core --test game_art_build_tests reconcile` 与
  `cargo test -p core concurrent`（`catalog::tests::concurrent_registers_do_not_lose_updates`
  及 job-store 并发取消测试）通过。
- CHECK 7：单 JSON stdout——脚本中每条 forge 命令都经「恰好一个 JSON 值 + `.ok`」
  守卫（jq 验证）。
- CHECK 8：凭据扫描——`$FORGE_JOB_STORE` + `$FORGE_PLAN_STORE` 上
  `authorization:[[:space:]]*bearer|access[_-]?token|refresh[_-]?token|device[_-]?code|xai[_-]?api[_-]?key`
  模式 0 命中；本波新增文件（示例、脚本、schema、证据目录）同模式 0 命中。

## 7. 机器可读 QA 报告路径（Machine-readable QA report path）

- 回归机器报告：`docs/qa/artifacts/forge-stage2-game-art-manifest-20260805/stage2-regression.json`
- Focused script 检查行：`docs/qa/artifacts/forge-stage2-game-art-manifest-20260805/focused-script-output.txt`
- v0.3 矩阵报告（瞬态）：`target/qa/v0.3-release-matrix-20260805T022701Z/v0.3-test-report.json`
- 本阶段不产生新的视觉 preview：全部 fixture 确定性像素（96×96 绿色块 + GIF），
  不代表真实模型视觉质量，故按 policy 不提交 preview。

## 8. Codex 验收逐条核对（§15 阶段 2）

| 验收条 | 证据 | 结果 |
| --- | --- | --- |
| 相同 manifest/spec/hash 得到相同 plan hash | CHECK 1；Rust `game_art::plan` 测试 | ✅ |
| 已满足资产为 reuse，不创建 Provider Job | CHECK 2；Rust 测试 (b) `build_project_reexecution_reuses_everything_without_child_jobs` | ✅ |
| 修改一个 icon spec 只失效其 collection 和下游 pack | CHECK 3；Rust 测试 (c) `build_project_rebuilds_only_the_asset_whose_spec_changed` | ✅ |
| required dependency 失败阻断依赖资产 | CHECK 4（场景一）；Rust 测试 (d) | ✅ |
| optional asset 失败不阻断无依赖的 required 资产 | CHECK 4（镜像场景二）；同一代码路径（build.rs 仅对 required 集合计失败） | ✅ |
| cancel parent 会请求取消所有 active child | CHECK 5；Rust 测试 (e) `request_cancellation_cascade_flags_non_terminal_descendants_only` | ✅ |
| 崩溃后可从 JobStore 恢复 | CHECK 6（`reconcile` 测试：死 worker 标记 + build-state 续跑 + catalog 自愈） | ✅ |
| 并发 build 不损坏 catalog | CHECK 6（`catalog::tests::concurrent_registers_do_not_lose_updates`） | ✅ |
| 所有 stdout 保持单 JSON | CHECK 7（每条 forge 命令单值 + ok 守卫） | ✅ |

## 9. 已知限制（Known limitations）

- plan 的 `estimatedCostTicks`/`maximumCostTicks` 恒为 null：现有估算只计请求数，无成本模型。
- `project audit` / 项目级 status 命令属后续阶段（阶段 7）；本阶段只有 diff/plan-build/build。
- delete candidates 仅报告，阶段 2 不删除任何 catalog 资产。
- 阶段 2 manifest 只支持 `character` / `icon_set` / `prop_set` 与 `style` / `subject` lock
  引用；其余 kind 在 parse 期以 `invalid_kind` 拒绝。
- `job report` 的 `reports` 内联汇总尚未收录 `project_build_report` kind；报告通过
  `job report`/`job get` 记录上的 artifact 路径读取（script 即采用此方式）。
- macOS 优先：`pid_alive` 在非 Unix 平台恒返回 alive（崩溃恢复探活在其他平台不生效）。

## 10. Provider 请求与费用（Provider requests and cost）

0 请求 / 0 费用。全部 fixture provider；未设置 `FORGE_REAL_PROVIDER_ACCEPT`，未读取 xAI
凭据；reuse 路径另经 `providerUsage.requests == 0` 显式断言（未变化项目 rebuild 零
provider 请求）。

## 11. 凭据扫描结果（Credential scan result）

- Focused script CHECK 8：临时 JobStore + PlanStore 上
  `authorization:[[:space:]]*bearer|access[_-]?token|refresh[_-]?token|device[_-]?code|xai[_-]?api[_-]?key`
  （rg，大小写不敏感）0 命中。
- 本波新增文件（`examples/cli/complete-visual/`、`scripts/test-game-art-manifest.sh`、
  `schemas/game-art-manifest.schema.json`、本证据目录）同模式 0 命中。
- 证据目录不含 JobStore/PlanStore 拷贝、token cache 或大产物；日志在瞬态 `target/qa/`。

## 12. Commit SHA

HEAD：`43e7ab3f4d81b9f39ab8b316b067ade44d10b014`（分支
`codex/stage2-game-art-manifest`）。按波规则本阶段不做 git commit；全部变更以未提交
工作树变更集交付，快照见证据目录 `git-status.txt` / `git-diff-stat.txt`。
