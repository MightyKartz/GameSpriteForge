# Forge workflow and release boundaries / 工作流与发布边界

Reviewed against the local source tree on 2026-09-05. This is a capability map,
not a new test report. Dated QA reports retain their original scope and verdict.

本文按 2026-09-05 本地源码核对能力边界，不代表本轮测试结果。历史 QA 的通过或失败
只适用于各自记录的版本、素材、范围和日期。

## English

### Default CLI and opt-in source

Forge is a local, agent-oriented asset pipeline. The default workspace contains
`cli`, `core`, `providers`, and `pack`; retained desktop/MCP sources are excluded.
The CLI orchestrates plans and durable Jobs, core owns deterministic processing
and quality gates, providers supply media, and pack validates delivery contracts.
Godot installation produces external textures and native engine resources.

[`packages/cli/Cargo.toml`](../../packages/cli/Cargo.toml) declares `default = []`.
An implementation in core or an example does not automatically add a public command.

| Surface | Build boundary | Current evidence and limit |
| --- | --- | --- |
| Style Locks, Character V1, Icon/Prop V1, Jobs, Packs, Godot installation | Default CLI | Published v0.2 baseline; the [2026-08-03 Character gate](../qa/forge-character-loop-v2-2026-08-03.md) covers its four-action workflow, not later Character versions. |
| Subject Locks and Character V2 | `consistency-v2` | Offline contracts and dated real probes exist; general real-model promotion remains incomplete. |
| Subject import and Grid generation | `subject-import`; `grid-generation` includes it | Opt-in workflows with version-specific validation and review contracts; success must be attributed to the exact version and action. |
| GameArtManifest and project diff/build plans | `game-art-manifest` | Stage 2 and remediation merged in PR #10, present in local HEAD `23e5b90`; the feature remains off by default. |
| Collection Locks, Icon/Prop V2, Portrait, Equipment, Decal, editable export/replacement, project audit | `collection-assets` includes `game-art-manifest` and `consistency-v2` | Implemented with fixture/Godot evidence and scoped real remediation; not a default release capability. |
| Environment, Terrain, Building, JSON Map | `terrain-assets` → `building-assets` → `map-compiler`; `world-assets` is the development umbrella | Experimental. [Real engineering acceptance](../qa/forge-world-v1-real-acceptance-2026-08-03.md) passed, while terrain repetition and building art quality prevented release promotion. |
| Later animation delivery experiments, including V18 | Core modules and dedicated examples | The [V18 approved delivery](../qa/forge-v18-approved-walk-right-production-2026-08-19.md) proves one reviewed `walk_right` animation and its Pack/Godot delivery; it does not establish a general multi-action CLI workflow. |

Production Provider resolution currently accepts only `xai` and offline `fixture`
([resolver](../../packages/providers/src/lib.rs)).
[`PixelLabLoopbackProvider`](../../packages/providers/src/pixellab.rs) is an offline
interface experiment: it reads no key, makes no network request, and is not registered
in the CLI resolver. Imported media and browser experiments do not add Provider support.

### Evidence and remaining release work

- Stage 3's [initial real probe](../qa/forge-stage3-real-acceptance-2026-08-05.md)
  failed. The later [remediation report](../qa/forge-stage3-blocker-remediation-2026-08-05.md)
  records acceptance of the repaired three-Pack scope, including Portrait repair and
  project audit. Preserve both verdicts; neither substitutes for evidence covering
  the full frozen five-style matrix or an intentional release decision.
- Stage 7 has project-audit implementation and local Godot delivery evidence. The
  planned project-wide sync and rendered Gallery acceptance are not closed by those
  narrower checks. Background/UI, VFX, and the complete frozen Forge 1.0 project
  remain unfinished roadmap work.
- Fixture success proves orchestration and contracts. Real image quality, native-size
  human review where required, full workflow acceptance, and release packaging are
  separate gates. Historical passing counts do not describe the current working tree.
- Update this map when features, resolver routes, or release decisions change. Keep
  new verification in dated [QA reports](../qa/) and the stage summary in the
  [implementation status](../qa/forge-complete-visual-implementation-status.md).
  Use [CONTRIBUTING.md](../../CONTRIBUTING.md) for applicable local checks.

## 中文

### 默认 CLI 与可选源码能力

Forge 是面向智能体的本地资产流水线。默认工作区包含 `cli`、`core`、`providers` 和
`pack`，保留的桌面/MCP 源码不参与默认构建。CLI 编排计划与耐久 Job，core 负责
确定性加工和质量门禁，providers 提供媒体，pack 验证交付合同；Godot 安装输出外部
纹理与原生引擎资源。

[`packages/cli/Cargo.toml`](../../packages/cli/Cargo.toml) 的默认功能为 `default = []`。
core 模块或 example 中存在实现，不会自动成为公开 CLI 命令。

| 能力 | 构建边界 | 已有证据与限制 |
| --- | --- | --- |
| Style Lock、Character V1、Icon/Prop V1、Job、Pack、Godot 安装 | 默认 CLI | 已发布的 v0.2 基线；[2026-08-03 角色门槛](../qa/forge-character-loop-v2-2026-08-03.md) 仅覆盖当时的四动作工作流，不代表后续 Character 版本。 |
| Subject Lock 与 Character V2 | `consistency-v2` | 有离线合同与分日期真实探测；通用真实模型晋级尚未完成。 |
| Subject 导入与 Grid 生成 | `subject-import`；`grid-generation` 包含前者 | 可选工作流采用各版本的验证与审核合同；通过结论必须指向具体版本和动作。 |
| GameArtManifest、项目 diff/build plan | `game-art-manifest` | 阶段 2 及修复已由 PR #10 合并，本地 HEAD `23e5b90` 已包含；默认功能仍关闭。 |
| Collection Lock、Icon/Prop V2、Portrait、Equipment、Decal、可编辑导出/替换、项目审计 | `collection-assets` 包含 `game-art-manifest` 和 `consistency-v2` | 已有实现、fixture/Godot 证据和局部真实修复验收；尚非默认发布能力。 |
| Environment、Terrain、Building、JSON Map | `terrain-assets` → `building-assets` → `map-compiler`；`world-assets` 为开发用集合开关 | 保持实验性。[真实工程验收](../qa/forge-world-v1-real-acceptance-2026-08-03.md) 已通过，但地形重复纹理与建筑美术质量仍阻止发布晋级。 |
| V18 等后续动画交付实验 | core 模块与专用 examples | [V18 已批准交付](../qa/forge-v18-approved-walk-right-production-2026-08-19.md) 证明一条人工审核的 `walk_right` 动画及其 Pack/Godot 交付，不证明通用多动作 CLI 完成。 |

正式 Provider resolver 目前只接入 `xai` 和离线 `fixture`
（[源码](../../packages/providers/src/lib.rs)）。
[`PixelLabLoopbackProvider`](../../packages/providers/src/pixellab.rs) 仅验证接口，
不读取密钥、不发网络请求，也未注册到 CLI resolver。导入素材和浏览器实验不等于
新增正式 Provider 支持。

### 验收证据与待完成发布工作

- Stage 3 的[首次真实探针](../qa/forge-stage3-real-acceptance-2026-08-05.md)失败；
  后续[修复报告](../qa/forge-stage3-blocker-remediation-2026-08-05.md)记录三个 Pack
  范围的通过，包括 Portrait 修复和项目审计。两个历史结论均保留，不能替代冻结的
  五风格完整矩阵证据或正式发布决定。
- 阶段 7 已有项目审计实现和局部 Godot 交付证据；项目级同步及实际渲染 Gallery 的
  完整验收仍未闭合。Background/UI、VFX 和冻结 Forge 1.0 视觉项目仍属未完成路线。
- fixture 通过证明编排与合同。真实美术质量、需要时的原尺寸人工审核、完整工作流
  验收及发布打包分别核对；历史测试数量不代表当前工作树结果。
- 功能开关、resolver 路由或发布决定变化时同步本文。新增验证写入按日期命名的
  [QA 报告](../qa/)，阶段摘要写入[实施状态](../qa/forge-complete-visual-implementation-status.md)，
  适用的本地检查见 [CONTRIBUTING.md](../../CONTRIBUTING.md)。
