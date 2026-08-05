# Forge Stage 2 Codex 修复与复验（2026-08-05）

状态：**实现修复完成；离线门禁通过；等待 PR #10 最终推送与 CI**

范围：对 Kimi 提交的 Stage 2 `GameArtManifestV1` / Project Build Core 做安全、耐久性与费用边界复验。本轮不调用真实 xAI，所有生成测试均使用 fixture provider。

## 已关闭的验收 blocker

1. **完整输入闭包**：plan token 绑定 manifest、spec、spec 内嵌参考图、ForgeProject、Style/Subject Lock、catalog、复用 Pack、恢复 state/Job/Pack。输入变化在 Job staging 前拒绝；claim 前后各复验一次，post-claim 失败会恢复 pending token。
2. **路径与 Pack 安全**：拒绝 URL、绝对/父目录穿越、项目外 symlink；Style board 与 Subject canonical/mask 在 diff/plan 阶段验证项目 containment 和内容哈希；复用 Pack 必须是非 symlink 的真实目录并通过 `validate_pack_layout`。
3. **稳定确定性**：项目与 manifest 在规划前 canonicalize；目录哈希使用 domain/type/path/content 长度分帧；lock 语义哈希把绝对媒体路径规范化为项目相对路径，因此相同内容跨 checkout 的 plan hash 一致。
4. **不可变执行输入**：StyleLock、SubjectLock 和 spec reference 在 JobStore 内生成 approved-input snapshot，child 只读取快照；每个付费 child 前再次校验 source closure。
5. **Provider 与费用边界**：计划锁定 provider/profile/capabilities/image model/video model；xAI 离线规划不访问 Keychain。缺少接受、请求上限或成本上限时，token 保持 pending 且不创建 Job。Character 最大请求数修正为 17（1 + 4×2×2）。
6. **严格恢复**：只允许从不同的、`Failed + recoverable + worker_pid=None` 的 BuildProject Job 恢复；恢复创建新 Job，不修改来源 Job；`.resume-claimed` CAS 租约阻止第二个继任者；已成功 child 的 Pack 必须存在且哈希、布局有效，禁止静默付费重建。
7. **取消与用量**：父取消动态级联到未终止子 Job；父 Job 不再写重复 provider usage，项目报告从完整 child 历史汇总并对缺失/损坏 usage 给出警告。
8. **Catalog provenance**：复用同时验证 kind、workflow、完整依赖集及依赖 Pack hash、完整 lock 集、provider/profile/model；成功 Pack 注册 V2 provenance 和来源链。

## 新增反例覆盖

- 计划后 spec/reference/StyleLock/catalog/Pack/恢复 state 漂移。
- 普通文件或 Pack 根/子项 symlink 伪装 `.gsfpack`。
- Style board、Subject canonical/mask 指向项目外。
- manifest 移除 lock、修改 dependency/kind/workflow/provider/style。
- 运行中父 Job 恢复、第二继任者、成功 child 尚未 catalog 注册的 crash window。
- 符号链接项目路径 plan→execute 自失效。
- 父子 usage 去重、取消级联、real-provider guard 不 claim token。

## 验证结果

| 门禁 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` + `git diff --check` | PASS |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS，0 warning |
| `cargo test --workspace --all-features --no-fail-fast` | PASS，225 passed / 0 failed |
| `scripts/test-cli-product.sh` | PASS |
| `scripts/test-cli-installer.sh` | PASS |
| `scripts/test-cli-signing-contract.sh` | PASS |
| `scripts/test-game-art-manifest.sh` | PASS，CHECK 0–8 及新增 CHECK 6A |
| `scripts/test-v03-release-matrix.sh` | PASS，6/6；releaseBlockingPassed=true；绑定 commit `54b8639` |

## 费用与凭据

- 真实 Provider 请求：0。
- 真实 Provider 费用：0。
- focused contract 对 JobStore/PlanStore 扫描 API key、OAuth token、Device Code、Authorization header 和临时签名 URL：0 命中。
- xAI 离线 plan 使用 secret sentinel 验证，sentinel 未写入 JobStore/PlanStore。

## 交付

- 实现 commit：`54b8639`（`fix: harden project build review boundaries`）。
- PR：#10，分支 `codex/stage2-game-art-manifest`。
- 机器报告：`docs/qa/artifacts/forge-stage2-codex-remediation-20260805/remediation.json`。

本报告只说明 Stage 2 工程合同通过，不代表新的真实模型视觉质量验收；Stage 3 的 CollectionLock、Portrait、Equipment、Decal 与跨 Pack audit 仍需单独工单和费用批准。
