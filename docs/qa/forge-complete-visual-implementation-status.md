# Forge 完整视觉资产系统实施状态

状态：Active — 阶段 0 已验收；阶段 1 未晋级；阶段 2 已合并；阶段 3 与项目审计已有实现和局部验收，仍为可选发布面
维护说明：Kimi 实施、Codex 分阶段验收的历史记录保留；当前范围按本地源码与已存 QA 证据核对
计划文档：`docs/architecture/forge-complete-visual-asset-system-implementation-plan.md`
基线提交：`69e6aec`（`codex/v0.3-character-benchmarks`）
状态核对：2026-09-05，本地 HEAD `23e5b90` 已包含 PR #10；后续未提交实现不等于已发布

本文下方历史测试与验收结果只适用于当时记录的范围和日期，不是当前工作树测试报告。
默认 CLI 为 `default = []`；完整功能开关、Provider 路由与样例边界见
[工作流与发布边界](../architecture/forge-workflow-boundaries.md)。

2026-09-05 工程复验：V4 重试与 Godot 安装事务已修复，默认测试 555 passed / 0 failed /
1 ignored，CLI 与新增可选功能 CI 分组全部通过；详见
[工程收尾报告](forge-engineering-closure-2026-09-05.md)。该结果不改变下表的真实模型与发布门槛。

## 阶段总览

| 阶段 | 状态 | 证据 |
| --- | --- | --- |
| 0 基线冻结与仓库卫生 | ✅ 已验收 | 本文档 + baseline JSON |
| 1 v0.3 Character 真实门槛 | ⚠️ **未晋级；阶段 1 收尾完成待复验** | `forge-character-v2-real-gate-2026-08-04.md` + `forge-stage1-closure-2026-08-04.md` |
| 2 GameArtManifest 与项目 Build Core | ✅ **实现与修复已由 PR #10 合并；`game-art-manifest` 默认关闭** | `23e5b90` + `forge-stage2-game-art-manifest-2026-08-05.md` + `forge-stage2-codex-remediation-2026-08-05.md` |
| 3 CollectionLock、Portrait 与 Static 完整化 | 已实现，离线与局部真实修复验收有记录；完整矩阵与正式发布未闭合，`collection-assets` 默认关闭 | `forge-stage3-offline-acceptance-2026-08-05.md` + `forge-stage3-blocker-remediation-2026-08-05.md` |
| 4 World 资产正式化 | 实验实现已有工程验收；美术质量与正式化门槛未完成 | `forge-world-v1-real-acceptance-2026-08-03.md` |
| 5 Background 与 UI | 未开始 | — |
| 6 VFX 与 EffectLock | 未开始 | — |
| 7 Project Audit、Godot Sync 与 Gallery | 项目审计已有实现与局部验收；项目级 Sync / 渲染 Gallery 完整门槛未完成 | `forge-stage3-offline-acceptance-2026-08-05.md` + `forge-stage3-blocker-remediation-2026-08-05.md` |
| 8 Forge 1.0 冻结视觉项目 | 未开始 | — |

阶段 3 首次真实探针失败与后续三个 Pack 修复验收通过是不同时间、不同范围的记录，
不能将任一结论扩展为冻结的五风格完整矩阵已完成。V18 的
[单条 `walk_right` 生产批准](forge-v18-approved-walk-right-production-2026-08-19.md)
同样只证明其批准素材与 Pack/Godot 交付，不代表通用角色动画 CLI 已完成。

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

阶段 1 收尾已实现代码级 `FORGE_REAL_PROVIDER_ACCEPT`、`FORGE_REAL_PROVIDER_MAX_REQUESTS`
和 `FORGE_REAL_PROVIDER_MAX_COST_TICKS` 守卫；未显式接受或缺少正数上限时，CLI 会在读取
凭据和建立网络请求之前拒绝真实 Provider 执行。收尾修复、定向测试和复验入口见
`docs/qa/forge-stage1-closure-2026-08-04.md`。

## 变更记录

| 日期 | 阶段 | 内容 | 证据 |
| --- | --- | --- | --- |
| 2026-09-05 | 状态核对 | 根据本地 `23e5b90` 修正阶段 2 待合并标记，补齐阶段 3、World、项目审计的实现/验收范围；新增双语工作流边界，保留各历史验收事实；本项未运行测试或真实 Provider | `docs/architecture/forge-workflow-boundaries.md` + 本文档阶段总览 |
| 2026-08-04 | 0 | 基线冻结、README 同步、QA artifact policy、状态文档建立 | `docs/qa/artifacts/forge-complete-visual-baseline-2026-08-04/` |
| 2026-08-04 | 1 | 三次真实 xAI Character V2 闭环，未晋级；费用 167 请求 / $132.10 | `docs/qa/forge-character-v2-real-gate-2026-08-04.md` + `docs/qa/artifacts/forge-character-v2-real-20260804/` |
| 2026-08-04 | 1 收尾 | 五项验收缺口修复；未调用真实 Provider，待 Codex 复验 | `docs/qa/forge-stage1-closure-2026-08-04.md` |
| 2026-08-05 | 2 | GameArtManifestV1 + ProjectCatalogV2 + project diff/plan-build + BuildProject 编排器（父子 Job、取消级联、崩溃恢复）实施完成，CLI 经 `game-art-manifest` feature 放出；验收回归 8/8 通过，focused script CHECK 0–8 全绿；验收发现 CLI `plan execute` 对 BuildProject 不解析 provider 的缺口，已由实现方修复并复验；全程 fixture，0 真实请求 | `docs/qa/forge-stage2-game-art-manifest-2026-08-05.md` + `docs/qa/artifacts/forge-stage2-game-art-manifest-20260805/` |
| 2026-08-05 | 2 Codex 修复 | 输入闭包、Pack 真实性、Style/Subject containment、快照执行、费用上限、严格恢复/CAS、取消与 usage 汇总等 blocker 全部修复；225 项 all-features 测试与 focused contract 通过；0 真实请求 | `docs/qa/forge-stage2-codex-remediation-2026-08-05.md` |
| 2026-08-11 | 角色纠偏 P0-A | `pixel-delivery-v2`：像素网格感知交付缩放、PaletteLock、二值 Alpha、fallback 报告；feature 默认关闭；V6/V7/V8 默认路径回归通过；0 真实请求 | `docs/qa/forge-pixel-delivery-v2-offline-2026-08-11.md` + `scripts/test-pixel-delivery.sh` |
| 2026-08-11 | 角色纠偏 P0-B | `identity-metric-v2`：真实 V8 标定集、pHash 基线测量、区域调色板/HOG/轮廓/占用消融；composite separation +0.0481 / AUC 1.0，显著优于 pHash；0 真实请求 | `docs/qa/forge-identity-metric-calibration-2026-08-11.md` + `docs/qa/artifacts/forge-identity-metric-calibration-2026-08-11/` + `scripts/test-identity-metric.sh` |
| 2026-08-11 | 角色纠偏 P1 | `topdown-grid@9.0.0`：一次方向 2×2 网格、四方向动作 2×2 网格、两阶段 approval、单元格重试、姿势开关；预计 5 / 最多 10 图片请求，视频恒 0；fixture、Pack、Godot、approval fail-closed、motion gate 全通过；0 真实请求 | `docs/qa/forge-topdown-grid-v9-offline-2026-08-11.md` + `scripts/test-grid-generation.sh` |
| 2026-08-11 | V9 Subject 修复 | 新增零 Provider `subject import`、导入 provenance 与 Style board SHA 闭包；Ayla canonical 字节原样导入为 `e807965b18707c76`，0 新请求；Direction Grid 仅完成 1/2 计划预检，未执行 | `docs/qa/forge-subject-import-style-closure-2026-08-11.md` + `scripts/test-subject-import.sh` |
| 2026-08-11 | V9 Direction Grid 真实门槛 | Job `833cbbf2-c8c8-4191-bcc6-4a0215d43b64` 使用 1 请求 / 0.7B ticks 后按合同停在审核；结构门禁通过，但原生审核发现无声明箭袋、箭矢、手杖和披风装饰，未批准、未进入 Action Grid/Pack/Godot | `docs/qa/forge-topdown-grid-v9-real-preflight-2026-08-11.md` |
| 2026-08-11 | V9 Direction Grid 语义修复 | equipment-none / hand-state prompt 合同、`direction-grid-appearance@1.0.0`、`direction-grid-lock@1.1.0`、审批前完整性校验与整张 Grid child retry 已完成；真实失败图零费用复算、1/2 请求 Plan-only 兼容验证及 9 项 Grid 合同通过；未执行新真实请求 | `docs/qa/forge-topdown-grid-v9-equipment-hand-remediation-2026-08-11.md` |
| 2026-08-11 | V9 Direction Grid 真实重试 | child Job `9ea9cae9-dd94-4b4a-a277-4b5d0345b2ad` 使用 1 次图片编辑 / 0.7B ticks；法杖、箭袋、箭矢及不一致手套已移除，正面站稳、背面兜帽与完整披风保留；Lock/报告/节点哈希和凭据扫描通过；按合同停在人工审核，无 Action Grid/Pack/Godot | `docs/qa/forge-topdown-grid-v9-equipment-hand-remediation-2026-08-11.md` |
| 2026-08-11 | V9 Action Grid 真实门槛 | Direction Grid 经用户批准；child Job `e7c14d79-33a6-4c4c-bdfe-93d3ac6d15d8` 仅提交 `walk_down:action_grid` 两次图片编辑 / 1.6B ticks，随后因上身闪烁、接触姿势近似、相位错误及姿势不足被硬阻断；按异常即停合同未消费 up/right/left，无 Pack/Godot/视频，安全扫描通过 | `docs/qa/forge-topdown-grid-v9-equipment-hand-remediation-2026-08-11.md` |
| 2026-08-12 | V9 Action Grid 四相位修复 | 固定 contact/passing 四相位；第二次请求改为编辑失败 Sheet；聚合动作与逐帧装备诊断；严格引用顺序、容量、输入/输出哈希和失败清单；Grid 合同 11/11 通过，0 真实请求 | `docs/architecture/forge-topdown-grid-v9-action-phase-remediation-plan.md` + `docs/qa/forge-topdown-grid-v9-equipment-hand-remediation-2026-08-11.md` |
| 2026-08-12 | V9 walk_down 单方向真实探测 | 新增 V9 validation-only 单方向闭包并将 Grid 合同扩为 12/12；真实 Job `0eae6ee6-d635-4958-ae91-14223045f39c` 仅调用 walk_down 两次 / 1.6B ticks；第一次相位仍不合格，第二次引入黑色 Grid 分隔线并被四格触边门禁拒绝；无其他方向、Pack/Godot/视频，安全扫描通过 | `docs/qa/forge-topdown-grid-v9-equipment-hand-remediation-2026-08-11.md` |
| 2026-08-12 | V9.1 独立关键帧离线交付 | 新增 `topdown-grid@9.1.0`：复用已批准 V9.0 DirectionGridLock，每方向从对应 idle 独立生成 contact/passing 四帧；4/8 单方向、16/32 全量、视频恒 0；帧级 child retry 三帧按哈希复用；失败报告、WorkflowGraph、相对路径 Pack 溯源、篡改拦截、Godot 4.6 headless 与默认 CLI 隔离均通过；0 真实请求 | `docs/architecture/forge-topdown-grid-v91-independent-keyframes-plan.md` + `docs/qa/forge-topdown-grid-v91-independent-keyframes-offline-2026-08-12.md` |
| 2026-08-12 | V9.1 walk_down 独立关键帧真实探测 | 真实 Job `7097f67e-0e85-4ead-b5bc-c6c908f4abda` 使用 7 次图片编辑 / 4.5B ticks，仅生成 walk_down 四帧并定向重试 1/2/3；身份、完整披风、空手及四个像素 distinct pose 保持，但原尺寸审核确认四帧均为同一侧靴子前伸，左右接触相位未交替，故拒绝；无其他方向、Pack/Godot/视频，安全与哈希扫描通过 | `docs/qa/forge-topdown-grid-v91-independent-keyframes-real-acceptance-2026-08-12.md` |
| 2026-08-12 | V9.2 结构化步态离线修正 | 新增 `topdown-grid@9.2.0` validation-only walk_down：viewer-space `gait-laterality@1.0.0`、透明灰度 PoseStructure、3-reference 预检、@1.1 报告/Manifest/Graph 哈希闭包及稳定错误码；真实 V9.1 同侧帧被拒且仅建议重试 2/3；下半身镜像虽翻转信号但分割缝变化 42.96%，保持禁用；Grid 19/19、Core 279、CLI 16、默认产品/Clippy 全绿，0 真实请求 | `docs/architecture/forge-topdown-grid-v92-structured-gait-plan.md` + `docs/qa/forge-topdown-grid-v92-structured-gait-offline-2026-08-12.md` |
| 2026-08-12 | V9.2 walk_down 结构化步态真实探测 | 真实 Job `0048ce3b-a0ea-4a63-8ed9-9063d01e9043` 使用 6 次图片编辑 / 4.3B ticks；frame 3 已正确切换至 screen-right，但 frame 2 定向编辑后仍为 screen-left 且清晰度下降，`gait-laterality@1.0.0` 以 `walk_laterality_not_alternating` 正确拒绝；35 个 artifact 哈希、授权账本、安全扫描与批准源 Lock SHA 通过；无其他方向、视频、Pack/Godot | `docs/qa/forge-topdown-grid-v92-structured-gait-real-acceptance-2026-08-12.md` |
| 2026-08-12 | V9.2 laterality fresh retry 离线修正 | laterality-only 失败不再携带错误 EditTarget，改用 DirectionAnchor + 灰度 PoseStructure fresh retry；新 `@1.2.0` 报告禁止失败帧 SHA；严格 child validation 仅接受来源报告唯一推荐帧，Plan/授权固定 1/1，其他三帧字节复用；V9.2 4/4、完整 Grid 20/20 通过，0 真实请求 | `docs/qa/forge-topdown-grid-v92-laterality-fresh-retry-offline-2026-08-12.md` |
| 2026-08-12 | V9.2 frame-2 fresh retry 真实探测 | 独立授权 child Job `d080b43c-bb35-4fc0-8f46-ba33aa1a8062` 严格使用 1 次图片编辑 / 0.7B ticks；失败帧像素未作为输入、其余三帧字节复用，身份/兜帽/完整披风/空手保持且清晰度恢复，但 frame 2 仍为 screen-left 接触，laterality 与 motion 门禁一致拒绝；15 个 artifact 哈希、来源锁、账本和安全扫描通过，无其他方向、视频、Pack/Godot | `docs/qa/forge-topdown-grid-v92-laterality-fresh-retry-real-acceptance-2026-08-12.md` |
| 2026-08-12/13 | V9.3 非对称步态离线修正与真实来源预检 | 新增 `topdown-grid@9.3.0`：仅从原始 V9.2 laterality 失败来源重试 `walk_down` frame 2；V1.1 guide 以明/粗/宽底与暗/细/抬靴编码 screen-right 支撑与 screen-left 抬起，prompt 移除解剖学左右歧义；其余三帧及 V1.0 guides 字节复用。Action Report @1.3、选中节点 @1.2、Manifest/Graph/源帧 SHA 闭包、exact 1/1/1 授权及审阅 manifest 摘要、并发单 worker/单 source child、guide-ignore、源证据/授权清单篡改负例通过；中途清单漂移不会恢复 HTTP 重试或 URL 回退。2026-08-13 只读真实来源预检确认 1/1 Pending Plan 与空账本精确授权，来源树/Job 聚合哈希和 10 个 Job 均未变；未创建/claim Job、未启动 worker、未调用 Provider/网络、未改变真实授权库。最终全 workspace/all-features、Clippy warnings-as-errors、Grid 26/26 与 CLI 产品合同均通过。真实探测仍需独立用户授权。 | `docs/architecture/forge-topdown-grid-v93-asymmetric-gait-plan.md` + `docs/qa/forge-topdown-grid-v93-asymmetric-gait-offline-2026-08-12.md` + `docs/qa/forge-topdown-grid-v93-real-source-preflight-2026-08-13.md` |
| 2026-08-13 | V9.3 非对称步态真实单帧探测 | 独立授权并执行 `walk_down:frame:2`，严格 1 image request / 1 Provider operation，实际 0.7B ticks；frames 0/1/3 字节复用、来源树未变、无视频/Pack/Godot。自动 laterality、motion、identity、equipment 与 cleanup 门禁均通过并停在 `awaiting_review`，但原生审查发现 frame 2 右靴下出现与宽底 PoseStructure 对应的灰色平台/滑板状语义泄漏，因此人工拒绝且未批准、未追加请求。 | `docs/qa/forge-topdown-grid-v93-asymmetric-gait-real-acceptance-2026-08-13.md` |
| 2026-08-13 | V9.4 平台安全步态离线修正 | 新增 `topdown-grid@9.4.0`：仅接受哈希闭合且由新轮廓门禁唯一复现 frame 2 平台泄漏的 V9.3 `awaiting_review` 来源；frames 0/1/3 字节复用，frame 2 使用无横向鞋底的 PoseStructure V1.2 fresh 生成。`footwear-platform@1.0.0` 以颜色无关的脚下薄台面/腿干倍率硬阻断平台或滑板；真实失败图测得 46/16=2.875×，其余三帧通过。Action Report @1.4、选中节点 @1.3、Manifest/Graph/Job 报告闭包与 fixture 单请求回归通过；0 真实请求。 | `docs/architecture/forge-topdown-grid-v94-platform-safe-gait-plan.md` + `docs/qa/forge-topdown-grid-v94-platform-safe-gait-offline-2026-08-13.md` |
| 2026-08-13 | V9.4 平台安全步态真实单帧执行（后续拒绝） | Job `c96788ba-e9de-43d6-8f30-c11611282f42` 使用 1 次图片编辑 / 0.7B ticks；自动 motion/laterality/equipment/旧 footwear 门禁通过，但用户原尺寸复核发现 frame 2 靴边仍有灰白抠图残留，已记录 `manual_rejected`，未导 Pack/Godot。 | `docs/qa/forge-topdown-grid-v94-platform-safe-gait-real-acceptance-2026-08-13.md` |
| 2026-08-13 | V9.5 鞋靴清理真实探测与确定性接受 | 新 `footwear-platform@1.1.0` 捕获灰色连通残留；V9.5 真实 Job `1ce7ade1-9a8e-40a2-8d11-cdf745f9ba50` 严格消费 1 次图片编辑 / 0.7B ticks，但仍残留 50 灰像素并把 frame 2 改回 screen-left，自动拒绝。最终本地 Job `4175b3ea-7181-47c0-a910-f8c567f8f042` 从正确 V9.4 姿势确定性移除 87 个掩码像素，其他像素与 frames 0/1/3 字节不变；motion/laterality/equipment/footwear、原尺寸审核及 0 请求用量复验通过，已接受；无视频、Pack/Godot。 | `docs/qa/forge-topdown-grid-v95-footwear-cleanup-real-2026-08-13.md` |
| 2026-08-13/14 | V10 连续四方向步态离线闭环 | 新增 `topdown-cycle@10.0.0`：复用已批准 V9 DirectionGridLock 与四个 idle，每个 walk 由一次连续媒体请求生成，并按原生 PTS 发现完整周期后自适应选取 8/10/12 帧；统一 `character-scale-lock@1.0.0` 禁止人物大小、中心与脚底基线漂移。validation `walk_down` 为 1/1，完整 fixture 为 4/4，8 动画 Pack、四方向 gait、V9 Lock/approval 溯源与 Godot 4.6 headless 导入通过；篡改来源在请求前拒绝。非 fixture 默认 fail-closed，仅另行授权的精确 Video 1.5 `walk_down` 1/1 探针可执行。 | `docs/architecture/forge-topdown-cycle-v10-plan.md` + `docs/qa/forge-topdown-cycle-v10-offline-2026-08-13.md` |
| 2026-08-14 | V10 walk_down Video 1.5 真实 1/1 探针 | Job `f80b6e47-9f58-4d8d-9dc3-4bdd239a9e6c` 严格消费 1 次 `grok-imagine-video-1.5` 生成 / 3.3B ticks；原生周期选择取得 1,166 ms、12 帧且 gait/身份/朝向/空手通过，但人物中心漂移 9 px、脚底基线漂移 10 px，并伴随上半身轮廓/边缘闪烁，`character-scale-lock@1.0.0` 正确拒绝。无重试、Pack 或 Godot，批准来源树未变。 | `docs/qa/forge-topdown-cycle-v10-real-acceptance-2026-08-14.md` |
| 2026-08-14 | xAI Image 2.0 候选模型离线接入 | 当前团队只读模型目录确认 `grok-imagine-image-2.0` 可用；Forge 保留 Image Quality 默认，仅开放“已批准 V9 DirectionGrid → Image 2.0 单次对照 child”。Plan/Runner 固定 `direction_grid`、1 request / 1 operation / 1.4B cap、新空账本独立授权、来源/manifest 哈希闭包、无重试/回退/Pack/Godot；伪 xAI scope 证明失败为 0 请求。真实来源只读 Plan 为 1/1，未执行付费生成。 | `docs/architecture/forge-xai-image2-candidate-plan.md` + `docs/qa/forge-xai-image2-candidate-offline-2026-08-14.md` |
| 2026-08-15 | xAI Image 2.0 DirectionGrid 真实 1/1 对照探针 | 当前团队模型目录仍返回 Image 2.0；独立空账本授权后，Job `686d40f4-3f18-4101-8780-fa1132ac7552` 的唯一 `edit_image` 操作收到 HTTP 429 并以 `provider_rate_limited` 失败。账本恰 1 条且保守标为 `ambiguous`，exact-one 路径无 429 重试；0 成功响应、0 图片、0 Pack/Godot，批准来源树未变。结果不具备视觉比较价值，Image Quality 继续保持默认。 | `docs/qa/forge-xai-image2-candidate-real-acceptance-2026-08-15.md` |
| 2026-08-15 | Codex 内置 imagegen DirectionGrid 实验 | 以获批 V9 Contact Sheet 为唯一参考生成 1 个四方向候选，并做 1 次仅移除背景的定向修正。四方向、空手、静止姿势、完整兜帽/披风和视觉尺寸基本通过，但两次输出均为 1254×1254 RGB、无 Alpha，棋盘格被烘入像素；按停止条件拒绝，未进入 Forge Lock/Pack/Godot。内置 imagegen 仅保留为外部概念工具，不替换生产 Provider。 | `docs/qa/forge-codex-imagegen-direction-grid-experiment-2026-08-15.md` |
| 2026-08-15 | 外部 DirectionGrid 导入与确定性棋盘抠图 | 新增零请求 `forge job import-direction-grid`、`direction-grid-import@1.0.0` 与 `checkerboard-sheet-matting@1.0.0`。真实本地 child `01447a14-791d-4ced-8f5f-990076e6b200` 移除 1,278,326 背景像素和 1,551 边缘中性色，4 主体、0 边界/中性残留；人物比例漂移 0.54%、中心 ≤0.5 px、脚底 ≤0.4 px，来源树未变、0 Provider/Pack/Godot。候选因外观启发式与身份细节漂移停在人工审核，未批准。 | `docs/architecture/forge-external-direction-grid-import-plan.md` + `docs/qa/forge-external-direction-grid-import-real-2026-08-15.md` |
| 2026-08-15 | 外部 DirectionGrid 身份一致性修正 | 新增 `direction-grid-lock@1.2.0` producer/downstream 分离、同方向相对一致性、共享来源比例/基线对齐、soft Alpha 与三底色 halo 证据、四文件输入和固定原生审核包。零请求重放 Job `a045e4fc-b457-4618-b5fe-2aaf3bc33ac0` 将四方向高度校正至 0.990–1.010×、中心 ≤0.5 px、脚底 0 px；Alpha 全门禁通过，但 front/back 各出现 3 个来源中不存在的躯干线性细节，被稳定硬阻断。来源树未变，未授权、批准、导 Pack 或进入 Godot。 | `docs/architecture/forge-external-direction-grid-identity-remediation-plan.md` + `docs/qa/forge-external-direction-grid-identity-remediation-2026-08-15.md` |
| 2026-08-16 | Codex imagegen 四方向独立生成 | 经单独授权完成严格 4/4：front/back/right/left 各只引用对应批准帧，未串联且无重试。四张输出方向、静止姿势和空手通过，但均为 1254×1254 RGB 并烘入棋盘格；本地四文件导入 Job `8cff3609-1117-424b-8e70-8b499929480d` 零 Forge Provider 请求，确定性抠图后发现 406 个暗色轮廓断裂像素（预算 8）而硬阻断。原生复核另见 front 新斜带/扣件及各方向服装细节重绘；来源树未变，无批准、第五次请求、动作、Pack 或 Godot。 | `docs/qa/forge-codex-imagegen-independent-directions-real-2026-08-16.md` |
| 2026-08-16 | 披风下摆合同与 V10.1 零请求稳定化 | 新增 `cape-hem-consistency@1.0.0` 四方向服装拓扑门禁；批准源零请求重放 Job `76172efc-4dd6-49cf-badc-0d9ddfea80c6` 精确拒绝 `front_idle` 缺少连续金色下摆，其余三向通过。新增 `topdown-cycle@10.1.0` 仅从原始 V10 scale-lock 失败来源执行整数平移；最终本地 child `ff618c4c-0332-4347-b82d-bbe387e95b06` 将中心/脚底漂移 9/10 px 降至 1/0 px，scale lock 通过，并写入零请求 `anchor_stabilization` Graph 节点；上身闪烁与轮廓/边缘漂移仍正确阻断。两条路径总计 0 Provider 请求，无批准、Pack 或 Godot。 | `docs/architecture/forge-cape-hem-v101-godot-plan.md` + `docs/qa/forge-cape-hem-v101-offline-2026-08-16.md` |
| 2026-08-16 | 正面权威 DirectionGrid 生成修正 | 新增 `front_authoritative_no_skirt_hem`：仅把已批准 `front_idle` 作为唯一图片引用，一次 2×2 编辑生成四向旋转；不再四方向独立生成，也不传 Style 图片。Plan/Runner 固定 1/1、来源/引用/报告/Lock/Manifest SHA 闭包、返回后引用复核、精确独立授权和无 Pack/Godot 边界；披风门禁同时支持“连续金边”与“正面无裙边”两个显式合同。离线 fixture 与两组拓扑单测通过，0 真实请求。 | `docs/architecture/forge-front-authoritative-direction-grid-plan.md` + `docs/qa/forge-front-authoritative-direction-grid-offline-2026-08-16.md` |
| 2026-08-16 | Codex 正面权威四方向真实生成 | 按用户要求停止 xAI，使用 Codex 内置图像生成：以批准 `front_idle` 为唯一权威生成 1 张 2×2 网格，再做侧面宽度与身份两次定向修正。三版均为烘入棋盘格的 1254×1254 RGB，Forge 零请求确定性抠图成功。最终四向尺寸/Alpha通过，但身份/躯干细节与后侧披风轮廓仍漂移，披风合同亦拒绝正面误匹配；保留候选但未批准、Pack 或进入 Godot。 | `docs/qa/forge-codex-imagegen-front-authoritative-real-2026-08-16.md` |
| 2026-08-11 | 角色纠偏 P2/P3 | Provider 选型 ADR 与 PixelLab loopback adapter；确定性 rig/deformation 可行性报告；均不接真实 key、不发真实请求 | `docs/architecture/forge-provider-selection-adr.md` + `packages/providers/src/pixellab.rs` + `docs/architecture/forge-deterministic-rig-spike.md` |

## 角色生成方法论纠偏（2026-08-11）

依据 `docs/architecture/forge-character-generation-method-remediation-plan.md`
完成 P0-A、P0-B、P1、P2 与 P3 的全部离线范围。关键结论：

1. P0-A 证明角色交付可以从 Lanczos3/半透明抗锯齿切换到 feature-gated
   像素网格交付与 PaletteLock；默认构建保持既有行为。
2. P0-B 证明现有 pHash 基线在真实 V8 标定集上确实反向（separation
   -0.0359 / AUC 0.175），IdentityMetricV2 消融后显著改善（separation
   +0.0481 / AUC 1.0）；该结论仍只代表当前小标定集。
3. P1 证明 `topdown-grid@9.0.0` 可以在 fixture 下完成单次方向网格、
   动作网格、两阶段授权、Pack 与 Godot 闭环，且视频请求恒为 0。
4. P2 建议真实验收时将 xAI 作为对照，将 PixelLab 风格网格 Provider
   作为首选候选；真实接入和预算仍需用户单独批准。
5. P3 决定不进入确定性 rig 实现，只保留后续单方向原型建议。

最终验证：

- `cargo fmt --all -- --check`：通过。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过。
- `cargo test --workspace`：全部通过。
- `scripts/test-cli-product.sh`：通过。
- `scripts/test-pixel-delivery.sh`：通过。
- `scripts/test-identity-metric.sh`：通过。
- `scripts/test-grid-generation.sh`：通过。

真实 xAI 验收未执行，不得据此宣称真实模型门槛通过。
