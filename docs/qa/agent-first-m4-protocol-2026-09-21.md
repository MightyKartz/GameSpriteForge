# M4 Agent 定位与观测协议修订验收

日期：2026-09-21。基线 `fefff956cc0f4bb396bb6c1e76d01f9ae36f0d49`。
类型：文档与未测模板验收；无 CLI 运行或游戏收益结论。

## 交付

[分步实施计划](../architecture/agent-first-m4-plan.md)明确直接使用者为 Agent，后续以
真实任务、自主完成、有效诊断、可靠交付及维护成本安排投入。中英文 README、PRODUCT、
产品聚焦决策和路线图同步。v1 协议归档，原 JSON 模板及所有历史实测记录字节不改。
[新协议](asset-delivery-comparison.md)及 v2 模板仅用于随后预注册的新观察。

## 自审与修订

- 保留旧人工门槛及变更原因，不把历史 PNG subprocess 时间当作 Agent 全任务耗时。
- 分开技术通过与完整任务验收：必要创意/许可审核未完成时，自主完成不能为 true。
- 所有失败及中断保留在成功率与净投入中；成功配对计时中位数的选择范围必须披露。
- setup/task/maintenance 不重复计时；外部等待有起止与原因；费用未知保持 null。
- 修正 Sword 候选说明：旧音乐验证入口错误不是现行 v6 校验失败；不修改冻结工具。
- 明确两个候选任务不等于六轮，旧动画不是当前必做需求；M4 保持未完成。

## 本地验证

本轮核对所有改动 Markdown 的本地链接目标存在、JSON 可解析、v2 两组字段一致及
未测量值为空；核对原 v1 模板与基线逐字节一致、改动文件范围只含文档/未测模板，
`git diff --check` 通过。没有运行 Rust、真实 Godot 或素材实验，因为未改可执行输入。

最小复核（从仓库根执行）：

```sh
git diff --check fefff956cc0f4bb396bb6c1e76d01f9ae36f0d49
python3 -m json.tool docs/qa/artifacts/delivery-comparison-v2-template.json >/dev/null
git diff --exit-code fefff956cc0f4bb396bb6c1e76d01f9ae36f0d49 -- docs/qa/artifacts/delivery-comparison-template.json
```

现有 quality、Godot native、Windows portable PR 工作流按路径触发，本轮文件不匹配其
触发路径；发布工作流不在本轮范围。PR checks 的实际状态另在 PR 中核对，未触发不能
写为 CI 通过，不为纯文档宣称 macOS/Windows 运行覆盖。

## 剩余验收

真实任务预注册、两个项目各三轮、动画修订闭环、实际时间/模型费用、复用和投入决策
均未完成。本轮没有修改消费项目、工具链锁、原始素材或历史批准，也没有发布版本。
