# Forge 仓库协作约定

本文件适用于整个仓库。开发或验证 Forge 时，先阅读
[forge-dev skill](.agents/skills/forge-dev/SKILL.md)；通过 CLI 制作素材和交付时，使用
[forge-use skill](.agents/skills/forge-use/SKILL.md)。再按任务查阅相关资料。

## 产品边界

Forge 是面向智能体的本地视觉资产 CLI。默认 Rust 工作区由 `packages/cli`、
`packages/core`、`packages/providers`、`packages/pack` 组成，分别负责命令与编排入口、
确定性处理与 Job/质量规则、媒体 Provider、Pack 交付合同。
Forge 负责素材与 Godot 交付；游戏代码和玩法逻辑属于消费素材的项目。

- [CLI feature 定义](packages/cli/Cargo.toml) 的默认值为 `default = []`。
  先确认任务涉及的 feature 和工作流；源码、example 或局部验收不代表默认发布能力。
- 保留的桌面/MCP 源码不参与默认产品路线。任务明确涉及这些模块时再修改。
- 真实 Provider 执行需要任务中已有的明确授权和请求/费用上限；fixture 成功不构成
  真实调用授权。已有授权仍然有效，按其范围执行。

## 修改与协作

- 先检查工作树状态，保留用户和其他 agent 的已有改动。围绕当前任务局部编辑，
  不为整理仓库而重置、覆盖或删除无关文件。
- 保持 CLI JSON 协议、Job/Pack 来源链和质量门禁；重试、加工与 Godot 安装的具体
  约束见 skill。沿已有模块职责修改，独立工作流优先放入对应模块。
- 并行任务明确文件负责范围。共用同一构建目录时，切换 feature 且使用
  `target/debug/forge` 的验收脚本顺序执行，避免不同构建互相覆盖。
- 不将凭证、临时媒体 URL、完整临时 JobStore 或大型生成媒体写入提交。
  验收产物按 [QA 产物政策](docs/qa/forge-qa-artifact-policy.md) 保存。

## 资料入口

| 需要确认的内容 | 来源 |
| --- | --- |
| 产品操作：生成、审核、重试与 Godot 交付 | [forge-use skill](.agents/skills/forge-use/SKILL.md) |
| 模块路径、重试、安装事务与验证方法 | [forge-dev skill](.agents/skills/forge-dev/SKILL.md) |
| 当前 feature、Provider 与发布边界 | [工作流与发布边界](docs/architecture/forge-workflow-boundaries.md)，并核对对应源码 |
| 构建、签名、环境依赖和适用检查 | [CONTRIBUTING.md](CONTRIBUTING.md) |
| CI 实际检查范围 | [质量矩阵 workflow](.github/workflows/v03-quality.yml) 及其调用脚本 |
| 当前阶段与剩余工作 | [实施状态](docs/qa/forge-complete-visual-implementation-status.md) |
| 某次验收的素材、版本与结论 | [按日期保存的 QA 报告](docs/qa/) |

## 验证与记录

- 按改动选择验证：Rust/CLI 修改执行适用默认门禁，feature 修改补上对应分组。
  纯文档或指令修改检查内容、路径、链接与 skill 格式，不要求重跑完整 Rust 测试。
- 缓存完整时可用 Cargo 离线模式；生成验收使用 fixture。需要 Godot/FFmpeg 的检查
  必须确认工具可用，不能把缺工具、零用例、ignored 或未执行记为通过。
- 区分默认构建、可选功能、真实 Provider、人工视觉审核与远端 CI 的证据。
  汇报本轮实际执行的检查及限制，不沿用历史通过数量。
- 本文件和 skill 保留稳定约束与入口。提交号、测试数量、阶段进度和个别素材的
  验收结论写入对应状态/QA 文档，避免在多份指令中重复维护。
