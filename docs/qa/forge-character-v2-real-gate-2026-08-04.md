# Forge Character V2 真实 xAI 晋级门槛：三次全新闭环（2026-08-04）

状态：已执行 — **未晋级（not_promoted）**
范围：实施计划阶段 1 任务 1（三次全新目录真实 xAI Character V2 → Pack → Godot）
基线提交：`33cea99`（`codex/v0.3-character-benchmarks`）
二进制：`target/debug/forge`（`consistency-v2` feature）
认证：xAI Preview OAuth（device code，file 凭据存储）
费用批准：用户批准 3 次闭环，上限 250 请求 / 150,000,000,000 costInUsdTicks（$150）

## 结论

`topdown-keyframes@2.0.0` 在真实 xAI 下**三次运行均未通过一致性门禁，Pack 成功率 0/3**，
远低于晋级门槛（≥90%）。按实施计划：

- video 路径（`topdown@1.0.0`）保持 v0.3 默认；
- keyframe 路径保持 experimental；
- 未降低任何质量阈值；
- 冻结 20×5 A/B 真实 benchmark 暂缓（见末节决策）。

## 三次运行汇总

| 运行 | 风格 / 角色 | game_ready | regenerate | blocked | awaiting_review | 缺失帧 | 总 verdict | 请求 | 费用（ticks） |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | forest-pixel / Forest Warden | 14 | 12 | 4 | 2 | 18/32 | blocked | 62 | 49,100,000,000 |
| 2 | watercolor-fantasy / Storybook Apothecary | 15 | 5 | 0 | 12 | 17/32 | regenerate | 57 | 45,100,000,000 |
| 3 | retro-scifi / Void Mechanic | 22 | 3 | 2 | 5 | 10/32 | blocked | 48 | 37,900,000,000 |

三次合计：**167 请求 / 132,100,000,000 ticks（$132.10）**，预算内（剩 83 请求 / 17.9B ticks 未用）。
三个角色生成 job 均以 `keyframe_regeneration_required`（可恢复）结束；均未进入 Pack 组装，
Pack validate 与 Godot 交付记为 `not_executed`。

风格 spec 逐字取自冻结清单 `benchmarks/character-v2/frozen-20x5.json`（forest-pixel、
watercolor-fantasy、retro-scifi）；三次运行方法论一致，未为通过门禁调整任何参数。

## 失败画像（跨运行一致）

- **`edge_density_drift` 是三次运行的共同首要原因**（运行 1：多数缺失帧；运行 2：15/17；
  运行 3：8/10）。真实 xAI 逐帧生成在像素边缘密度上无法保持集合内一致。
- `identity_similarity_low` 次之（运行 1 多数帧、运行 2 6 帧、运行 3 3 帧）。
- `multiple_subjects` 集中在运行 1（4 blocked 帧全因此）与运行 3（2 blocked 帧）。
- 所有缺失帧均为 attempt 2 —— 工作流内置的第二次尝试无法挽救。
- 风格/主体敏感度：retro-scifi + 粗轮廓外甲（运行 3，22/32）显著好于细线水彩
  （运行 2，15/32）与细碎像素（运行 1，14/32），但门禁结论不变。

## 逐运行证据

- `docs/qa/artifacts/forge-character-v2-real-20260804/run-1/run-report.json`
  （含网络事件记录：系统 TUN 代理 fake-IP 路由对 api.x.ai 中断，首次 style 尝试失败、
  0 请求损失；后续经 localhost-only CONNECT 代理完成，TLS 端到端，无凭据经手）
- `docs/qa/artifacts/forge-character-v2-real-20260804/run-2/run-report.json`
- `docs/qa/artifacts/forge-character-v2-real-20260804/run-3/run-report.json`
- 每次运行附：使用中的 spec 三件套、一帧代表性 game_ready PNG（含 SHA-256）、
  per-job provider usage 分解、凭据扫描结果（三次均 0 匹配）。

## 晋级门槛逐项核对（计划 §15 阶段 1）

| 门槛 | 结果 |
| --- | --- |
| Pack 成功率 ≥90% | **未达：0/3** |
| 硬缺陷拦截率 100% | 门禁按设计拦截（0 错误 Pack 导出）✔ |
| 错误 Pack 导出数 0 | ✔（无任何 Pack 导出） |
| 身份一致性较 video 提升 ≥10pp | 未达评估条件（无 Pack 产出） |
| 每 Pack 图片请求中位数 ≤40 | 未达评估条件；运行实际 46–60 且仍未过门 |
| Godot 加载率 100% | 未达评估条件 |

## 冻结 20×5 benchmark 决策

离线 plan 预估 A/B 双路径常规 845 请求 / 上限 1565 请求（约 $1300–1800）。
在 Pack 成功率门槛数学上已不可达的情况下，本次不执行；如 Codex 需要完整 A/B
数据用于校准 `edge_density` 阈值或 hybrid 路径设计，再单独申请预算。

## 后续建议（供 Codex 验收参考）

1. `edge_density_drift` 是真实模型的首要拦截原因：需复核该指标在真实 xAI 输出上的
   阈值校准是否过严（注意：本记录不建议降阈值，建议用冻结 fixture + 人工灰区样本校准）。
2. `topdown-hybrid@2.1.0`（2–4 关键帧 + 时间生成）是计划预留的折中路径，可在
   fixture/contract 层面先行实现。
3. Subject Lock 多视角 canonical（计划 §9.1 SubjectLock V2）可能改善
   `identity_similarity_low`，属于阶段 3 之后的工作。

凭据扫描：三份证据目录均通过（authorization/token/device_code/api_key 模式 0 匹配）。
未提交 JobStore、原始帧、视频或 Pack；运行工作区在 `target/qa/`（已被 git 忽略）。
