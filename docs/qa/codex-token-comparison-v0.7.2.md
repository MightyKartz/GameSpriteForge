# Codex resource-lookup token comparison / Codex 素材查询 Token 对比

This is an **exploratory demonstration**, not a measured productivity gain for a real game. It records two independent, read-only Codex CLI comparisons on 2026-09-23. The selected usage events and normalized answers are in [the evidence snapshot](artifacts/codex-token-comparison-v0.7.2.json). The original local session logs and generated fixture media are not published.

这是一次**探索性演示**，不是已验证的真实游戏开发效率收益。2026-09-23 使用 Codex CLI 进行了两轮独立、只读的对比。[证据摘要](artifacts/codex-token-comparison-v0.7.2.json)保留了用量记录及标准化结果；本机原始会话日志和生成的测试媒体未公开。

## Task / 任务

A synthetic library contained 73 records assembled from the repository's public example artwork and a synthetic sound. Both arms received the same four requirements: a thunder enemy with approved visual review, a thunder attack effect, forest background music, and a fire sound effect. The ordinary-tool arm could read an inventory and use shell/Python/jq; the Forge arm could query the library with `forge asset check-requirements`. Both returned the same result: two covered, one needing review, one missing.

测试库由仓库公开示例素材和合成声音组成，共 73 条记录。两组收到相同的四条需求：已通过视觉审核的雷灵、雷击特效、森林背景音乐和火焰音效。普通工具组可读取清单并使用 shell/Python/jq；Forge 组可通过 `forge asset check-requirements` 查询资源库。两组均得出相同结论：已覆盖 2 项、待审核 1 项、缺失 1 项。

Both arms used Codex CLI `0.155.0-alpha.9.2`, `gpt-6-astra`, low reasoning, and independent ephemeral read-only sessions. The Forge arm used the official macOS Apple Silicon v0.7.2 release binary from commit `d0fb39b3d26a3a6d383306c7fa4359d2fe545b9e`, default features (`[]`), SHA-256 `f6c7a238b072adfd7c23e5f01b3d02daab3b5c668f19275e10e1a0e14ac4d888`. The binary reported `project_asset_requirements_check` in `forge doctor --json`. No asset-generation Provider, Godot import, or tool-initiated web request was part of this comparison; Codex itself used its model service.

两组使用相同的 Codex CLI、模型和推理等级，并分别启动临时只读会话。Forge 组使用上述官方 macOS Apple Silicon v0.7.2 发布包二进制。此对比不涉及素材生成 Provider、Godot 导入或网络请求。

## Recorded usage / 记录用量

`input_tokens` is the Codex CLI `turn.completed.usage.input_tokens` value. It includes cached input; cached and output tokens are shown separately so the figures are not mistaken for billed cost. Elapsed time is retained for context but was not controlled as a performance measurement.

`input_tokens` 直接取自 Codex CLI 的 `turn.completed.usage.input_tokens`，包含缓存输入。下表另列缓存与输出 Token，避免将输入用量误当作账单金额。耗时仅供参考，未按性能测试要求控制。

| Run / 轮次 | Arm / 组别 | Input tokens | Cached input | Output tokens | Elapsed |
| --- | --- | ---: | ---: | ---: | ---: |
| 1 | Codex with ordinary tools / 普通工具 | 103,290 | 73,984 | 962 | 110.31 s |
| 1 | Codex with Forge / 配合 Forge | 100,732 | 78,208 | 363 | 103.07 s |
| 2 | Codex with ordinary tools / 普通工具 | 103,096 | 58,368 | 1,062 | 131.87 s |
| 2 | Codex with Forge / 配合 Forge | 39,982 | 18,944 | 195 | 68.97 s |

In these runs, the Forge-guided arm recorded **2,558 fewer input tokens (2.5%)** and **63,114 fewer (61.2%)**, respectively. Both arms answered the four requirements correctly in each run.

这两轮中，Forge 组记录的输入 Token 分别少 **2,558（2.5%）** 和 **63,114（61.2%）**；每轮两组对四条需求的回答均正确。

## Limits / 限制

- Run 1's ordinary-tool arm hit a local Python/Xcode shim problem, although it still produced the correct answer. Run 2 supplied an explicit interpreter path to that arm and the exact Forge command to the Forge arm. The prompts and tool guidance therefore changed between runs and were not symmetric.
- Cached input and model/tool paths varied substantially. Two synthetic runs cannot isolate Forge's causal effect or establish an average saving, a fixed percentage, faster real-game development, or a billing discount.
- The measured task was only **finding existing resources**. It did not measure generating artwork, processing media, installing assets in Godot, or revising a game. Real-project benefits remain unmeasured under the [agent-delivery comparison protocol](asset-delivery-comparison.md).

- 第 1 轮普通工具组遇到本机 Python/Xcode shim 路径问题，但最终答案正确。第 2 轮给普通工具组提供了明确的解释器路径，同时给 Forge 组提供了准确命令。两轮的提示和工具指引并不对称。
- 缓存输入及模型/工具调用路径变化明显。两次合成任务不能隔离 Forge 的因果收益，也不能得出平均节省率、固定百分比、真实游戏开发提速或账单折扣。
- 本次只测量**查找已有资源**，未测素材生成、媒体加工、Godot 安装或游戏修订。真实项目收益仍需按[智能体交付对照协议](asset-delivery-comparison.md)验证。
