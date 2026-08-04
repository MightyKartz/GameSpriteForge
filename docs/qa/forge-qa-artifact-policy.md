# Forge QA Artifact Policy

状态：Active（自 2026-08-04 起生效，随完整视觉资产系统实施计划阶段 0 建立）

本策略约束 `docs/qa/` 及其 `artifacts/` 子目录中提交到 Git 的证据范围，目标是让
验收证据可审查、可复现，同时不把大型瞬态产物带入仓库历史。

## 提交到仓库的内容

每个 QA 运行或验收记录保留：

- Markdown 验收报告（结论、门槛判定、精确命令、失败定位）。
- 机器可读 JSON 报告（gate 结果、质量/consistency/loop 报告摘要、费用与请求计数）。
- 代表性视觉证据：contact sheet、少量 preview PNG/GIF（每个运行少量精选，而非全集）。
- 关键输入与输出的 SHA-256 哈希（用于对账，而非大文件本体）。

## 不提交到仓库的内容

- 完整 JobStore / PlanStore 目录（`jobs/`、`plans/` 树、`.job.lock`、source 视频、
  normalized-frames 等工作区文件）。
- Provider 原始视频与原始大尺寸生成图（保留抽样帧或 contact sheet 代替）。
- 完整 `.gsfpack` 大文件（保留 `forge pack validate` 报告与 manifest 哈希代替）。
- Token cache、凭据、Authorization header、Device Code、临时签名 URL——这些
  永远不得进入仓库，发现即视为安全事故处理。
- 完整 Godot 导入目录、`.godot/` 缓存。

大型瞬态产物应放在已被忽略的位置（如 `target/qa/` 或本地未跟踪目录），仓库中只留
上述精简证据与指向它们的哈希。

## 既有证据目录的保护

2026-08-04 之前，`docs/qa/artifacts/` 下已存在大量未跟踪的历史证据目录（如
`forge-loop-v2-real-20260803/`、`forge-xai-real-acceptance-20260803/` 等）。
这些是用户保留的在途证据：

- 不得删除、移动或覆盖。
- 不得新增会把这些既有目录变为 ignored 的 `.gitignore` 规则。新增规则前必须用
  `git status --ignored` 与 `git check-ignore` 验证不会覆盖任何既有证据路径。
- 因此本策略通过「新证据放精简、旧证据保持原样」实现，而不是通过追溯式忽略规则。

## 新运行的证据最小化流程

1. 运行时将 JobStore/PlanStore 重定向到临时目录或 `target/qa/`。
2. 验收后只把报告 JSON、contact sheet、精选 preview 与哈希复制进
   `docs/qa/artifacts/<run-name>/`。
3. 提交前执行凭据扫描（token/URL/Authorization 模式），扫描结果写入验收报告。
