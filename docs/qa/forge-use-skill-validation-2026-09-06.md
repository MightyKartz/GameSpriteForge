# Forge 产品使用 skill 验证

日期：2026-09-06

结论：`forge-use` 已完成格式与引用校验，并通过独立的离线产品操作演练。
本结论适用于 skill 和 fixture 流程，不代表真实模型或视觉素材验收。

## 实现

- 新增 `.agents/skills/forge-use/SKILL.md`、UI metadata 和三份按需参考：基础流程、
  执行与交付、可选功能。
- 明确 Plan camelCase 与 Job snake_case、持久授权和环境门禁、审核及重试费用边界。
- `AGENTS.md` 增加使用入口；收窄 `forge-dev` 描述，避免日常素材操作触发工程开发流程。
- 根据独立演练补充 fixture 占位语义、可缺失的工作流图、Job/step 区别及安装路径解析。

## 已执行验证

| 检查 | 结果 |
| --- | --- |
| `quick_validate.py`，forge-use / forge-dev | 通过 |
| 本地引用、JSON 示例、UI metadata、空白 | 通过 |
| `cargo build --locked --offline -p forge-cli` | 通过；用于取得独立演练的 CLI |
| 独立 agent 按 skill 操作新临时资产/Godot 项目 | 通过 |
| Style、两项图标与安装 Job | 三个 Job 均 succeeded |
| Pack 结构校验、安装纹理与源 Pack 哈希 | 通过 |
| Godot 4.6.3 headless 加载两张 128×128 安装纹理 | 通过 |

独立使用者只获得 skill、CLI 路径和离线素材请求，没有复制现有验收脚本。
它先创建计划，再执行对应令牌，读取 Job/报告并安装到新临时 Godot 项目。
原始命令、退出码、报告及素材保留于 `/tmp/forge-use-fixture-25Sc84/`。
独立记录反映初稿的实际表现；随后补充解释缺口并复核最终文档，没有修改 CLI 行为。

Style 请求 1 次、图标请求 2 次、安装请求 0 次，均为 fixture；真实 Provider 请求为 0。
费用字段为 null，未解释为实际金额 0。未执行真实授权消费、可选功能的完整流程、
Godot GUI 渲染或人工批准。本轮没有修改 Rust 行为，也未重跑完整工作区测试。

## 演练发现及范围

- 两个 fixture 图标均为相同紫色占位形状；流程通过不等于药水/钥匙视觉语义通过，
  没有记录人工批准。
- Style/Static 的 `job graph` 返回 `workflow_graph_missing`；报告和产物仍可检查。
- Static Job 的部分 step 仍 pending，且 plan effects 与实际报告 profile 不同；
  未以这些字段取代生命周期/产物验证，也未修改历史记录。
- usage 中的 texture 路径在 Pack 内有效，安装位置需按实际资源引用解析；
  已从真实安装路径完成引擎加载。
- 本次 icon_set 只交付 PNG，没有 `.tres/.tscn`，相应大小与内嵌像素检查记为不适用。
- 独立检查脚本首次误用 Godot API，修正脚本后加载检查通过；原始错误日志仍保留。

精简证据：[机器报告](artifacts/forge-use-skill-2026-09-06/summary.json)、
[独立使用记录](artifacts/forge-use-skill-2026-09-06/independent-result.md)、
[fixture contact sheet](artifacts/forge-use-skill-2026-09-06/fixture-contact-sheet.png)。
仅保留摘要、哈希和单张代表性预览，完整 JobStore、Plan/token 和 Godot 缓存不入库。
精简证据的凭据/带签名 URL 模式扫描为 0 匹配。
