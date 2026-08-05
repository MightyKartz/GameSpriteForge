# 3D Asset Forge：兄弟项目机会与技术方向

Date: 2026-07-31

Status: Research / first-pass product hypothesis

Relationship to Forge: 独立兄弟项目，不纳入当前 Forge V1 范围

## 1. 摘要

建议探索一个独立的 **3D Asset Forge**：把 AI 生成、商店下载、DCC 导出或用户自有的 3D 模型，转换成经过检查、修复、优化并可直接导入 Unity、Unreal 和 Web3D 的游戏资产。

核心判断：Unity、Unreal 的 CLI/MCP 并不依赖外部 3D 资产才能操作编辑器，但真正限制 3D 游戏生产效率的通常不是“能否让 Agent 控制引擎”，而是资产进入引擎前后的质量与兼容性。机会更可能位于 **3D 资产编译、质检和跨引擎交付**，而不是再做一个通用引擎 MCP，或从零训练 3D 生成模型。

推荐顺序：

1. GLB 资产检查器和质量报告；
2. 确定性修复、压缩和优化；
3. Unity 导入适配与自动验证；
4. Unreal 导入适配；
5. 生成服务适配器；
6. 在稳定 Job API 之上提供可选 CLI/MCP。

一句话定位：

> Turn generated and downloaded 3D models into validated, game-ready assets.

## 2. 背景与问题

SpriteCook 的有效组合不是单独的图片生成，也不是单独的 Godot MCP，而是形成了完整循环：

```text
生成精灵
→ 获得稳定资产引用
→ Codex 编写游戏逻辑
→ Godot 导入和组装
→ 运行、截图、修正
```

3D 领域也可以形成类似循环，但 3D 资产比 2D 精灵多出一层重要的“资产编译”问题：

- 拓扑、面数和非流形几何；
- UV、贴图尺寸、PBR 通道与材质映射；
- 坐标轴、单位、朝向、原点和枢轴；
- 骨骼、蒙皮、动画片段与重定向；
- LOD、碰撞体、包围盒和运行时内存；
- Unity、Unreal、Web 渲染之间的视觉一致性；
- 来源、生成器、许可证和修改记录。

因此，3D 产品不能只复制“生成工具 + 引擎 MCP”。它需要在生成器与引擎之间建立一个可验证、可恢复、可重复的处理层。

## 3. 调研结论

### 3.1 Unity

Unity 6 官方支持通过命令行以 `-batchmode`、`-executeMethod` 和 `-projectPath` 执行编辑器任务，并通过 `AssetDatabase`、`ModelImporter` 等接口导入和配置模型。

开源项目 CoplayDev Unity MCP 已经覆盖场景、资产、脚本、测试、性能分析和构建等通用编辑器控制。其存在说明 Unity 的 Agent 控制层已经较成熟，也意味着“再做一个通用 Unity MCP”不容易形成差异化。

结论：**Unity 最适合作为第一个引擎适配器，但不应成为产品本身。** 产品应负责输出可导入资产、导入配置和验证结果，再复用现有 CLI/MCP 完成场景组装。

### 3.2 Unreal Engine

Unreal 的 Interchange Framework 提供文件格式无关、异步、可定制的导入管线，并允许使用 C++、Blueprint 和 Python 扩展。它适合承载材质、LOD、碰撞和导入规则，但版本差异、插件兼容性及编辑器自动化成本都高于 Unity。

现有 Unreal MCP 项目证明了编辑器控制的可行性，但其公开说明仍强调实验性质，不建议作为生产基础直接依赖。

结论：**Unreal 是高价值的第二阶段适配器。** MVP 应先做只读预检和明确的导入配置，避免一开始承担完整编辑器自动化的复杂度。

### 3.3 Godot

Godot 并非只能开发 2D 游戏，现有 Godot MCP 也包含 3D 场景与 MeshLibrary 等能力。不过 SpriteCook + Godot 的自然优势仍主要来自 2D 工作流、生态认知和较低的资产复杂度。

结论：Godot 可作为后续兼容目标，但不能据此假设 3D 资产问题会像 2D 精灵一样简单。

### 3.4 Blender

Blender 官方支持后台模式和 Python 脚本，适合处理浏览器端难以可靠完成的工作：格式转换、UV、贴图烘焙、拓扑修复、骨骼、碰撞体及复杂几何操作。成熟的 Blender MCP 也已证明 Agent 驱动 DCC 的可行性。

结论：**Blender 应作为内部的无头编译器和高级修复后端，而不是 MVP 的主要用户界面。**

### 3.5 Web3D

glTF/GLB 是面向传输和运行时加载的开放格式。Three.js 的 GLTFLoader 支持 glTF 2.0、动画、PBR、Draco、Meshopt 和 KTX2；`<model-viewer>` 适合快速预览；Khronos glTF Validator 可输出机器可读的验证结果；glTF-Transform 与 meshoptimizer 可执行结构变换、压缩和网格优化。

结论：**Web3D 非常适合成为检查、对比、配置和任务编排界面，但不能单独替代 DCC。** 浏览器负责快速反馈，本地或服务端 Worker 负责重处理。

### 3.6 3D 生成服务

Meshy 等服务已提供文本/图片生成 3D、拓扑目标、PBR 贴图、常见格式输出和标准人形绑定；Hunyuan3D 等开放模型则允许自托管，但完整形状与纹理流程对显存要求较高。

结论：MVP 不应训练模型，也不应绑定单一供应商。生成能力可以后接为 Provider Adapter；首要价值是让来自任意来源的模型变得可用。

## 4. 产品机会

### 4.1 推荐方向：3D Asset Forge

目标用户：

- 使用 AI 3D 生成器但不熟悉 Blender 的独立开发者；
- 需要把外包或商店资产批量导入 Unity/Unreal 的小团队；
- 需要为 Web、Unity、Unreal 同时交付模型的创作者；
- 通过 Codex 等 Agent 开发 3D 原型，但被资产质量阻塞的开发者。

核心 Job：

> 给我一个来源不确定的 3D 模型，告诉我它有什么问题，安全地自动修复能修的部分，并输出可以直接进入目标引擎的资产包。

首版价值不在“生成一个更漂亮的模型”，而在：

- 快速回答资产能否使用；
- 把隐蔽问题转成可操作报告；
- 自动执行可重复、低风险的修复；
- 减少模型从下载到游戏中可运行的人工步骤；
- 为 Agent 提供稳定的资产 ID、状态和交付清单。

### 4.2 备选方向

| 方向 | 初步评分 | 判断 | 主要风险 |
| --- | ---: | --- | --- |
| 3D Asset Forge | 27/35 | 优先验证 | 用户是否愿意为质检/转换付费尚无直接证据 |
| Unity-first Agent Asset Loop | 25/35 | 可作为切入场景 | 容易被现有 Unity MCP 或生成平台吸收 |
| Unreal AI Asset Import Guard | 24/35 | 第二阶段 | 开发、版本兼容和 QA 成本更高 |

评分仅代表本轮技术与市场信号的初筛，不代表已经验证的商业结论。

## 5. 推荐系统架构

```text
User file / AI provider / asset store / Blender
                       │
                       ▼
              Ingest + provenance
                       │
                       ▼
             Canonical GLB workspace
                       │
           ┌───────────┴───────────┐
           ▼                       ▼
 Web3D inspector + validator   Headless Blender
 preview / report / config     repair / bake / rig
           │                       │
           └───────────┬───────────┘
                       ▼
           Transform + optimization
        glTF-Transform / meshoptimizer
                       │
           ┌───────────┼───────────┐
           ▼           ▼           ▼
       Web GLB     Unity pack   Unreal pack
                       │           │
                       ▼           ▼
                Engine smoke validation
```

### 5.1 组件职责

| 组件 | 职责 |
| --- | --- |
| Three.js / model-viewer | 模型、材质、动画、包围盒与 LOD 的交互预览 |
| glTF Validator | 格式、缓冲区、图片、动画和扩展的机器可读验证 |
| glTF-Transform | glTF 结构操作、贴图和节点变换、确定性处理 |
| meshoptimizer | 网格简化、缓存优化、LOD/压缩相关能力 |
| Blender headless | 拓扑、UV、烘焙、格式转换、绑定、碰撞等重处理 |
| Unity adapter | ModelImporter 配置、材质映射、Prefab 和 PlayMode 冒烟验证 |
| Unreal adapter | Interchange Pipeline 配置、材质/碰撞/LOD 与导入验证 |
| Job service | 稳定资产 ID、异步状态、取消、重试、恢复和结果清单 |

### 5.2 格式策略

- 首个规范格式：GLB；
- FBX：在骨骼或目标引擎兼容确有需要时加入；
- USD/USDZ：在多工具场景、Apple 或更复杂场景交换得到验证后加入；
- 不把某个生成器的私有格式作为内部事实标准。

后续可定义引擎中立的资产包，包含规范 GLB、贴图、LOD、碰撞、骨骼/动画、质量报告、单位/朝向以及来源/许可证信息。包结构应在真实样本验证后再定稿。

### 5.3 Job 与 Agent 契约

建议所有长任务使用统一的异步模型：

```text
start → job_id → status/progress → result manifest
                     ├── cancel
                     ├── retry
                     └── recover recent jobs
```

每个资产至少记录：

- 稳定 `asset_id`；
- 内容 SHA-256；
- 原始来源和导入时间；
- Provider、模型版本和参数（如适用）；
- 单位、坐标轴、朝向和枢轴策略；
- 许可证或用户声明；
- 处理步骤、工具版本和输出清单。

这部分是 SpriteCook 最值得借鉴的模式之一：MCP 提供小而稳定的工具，Skill/文档定义工作流，资产与 Job 可恢复，下载链接只是临时交付手段。

## 6. MVP 范围

### 6.1 应包含

1. 拖入或上传单个 GLB；
2. Web3D 预览，包括动画片段和材质检查；
3. 质量报告：
   - 三角面、节点、材质、贴图和 Draw Call；
   - PBR 贴图完整性与尺寸；
   - 骨骼、蒙皮和动画；
   - Bounds、单位、朝向和枢轴；
   - glTF 规范错误和警告；
   - 粗略显存/下载体积估算；
4. 确定性处理：归一化变换、贴图压缩、网格优化和元数据清理；
5. 导出优化后的 GLB 和 JSON 报告；
6. Unity 导入 Profile/脚本；
7. 使用固定测试工程完成 Unity 无人工导入与场景冒烟验证；
8. 本地任务清单、失败原因和可恢复结果。

### 6.2 暂不包含

- 自研或训练 3D 生成模型；
- 内置资产市场、社区或协作平台；
- 通用 Unity/Unreal 编辑器 MCP；
- 完整 DCC 建模体验；
- 任意格式和任意引擎兼容；
- 在当前 Forge V1 中加入 3D 模式；
- 强依赖某一家生成服务；
- 将 MCP、CLI 或开发者调试控件暴露为首版产品 UI。

## 7. 可选 CLI/MCP 设计

只有当本地/服务端 Job API 稳定后，再增加薄封装：

```text
ingest_asset
inspect_asset
optimize_asset
generate_lods
export_unity
export_unreal
get_job
```

原则：

- 工具返回稳定 ID 和结构化结果，不依赖聊天文本传递路径；
- 长任务必须异步；
- 引擎操作与资产处理分层；
- 支持 Codex 在失败后查询最近任务并继续；
- API Key、供应商 Token 和签名链接不得写入日志或对话；
- MCP 是自动化接口，不是产品 UI 的核心。

## 8. 开发阶段

### Phase 0 — 证据验证

- 收集 30 个真实问题模型，覆盖 AI 生成、商店和手工导出；
- 对 10 位 Unity/Unreal 独立开发者访谈；
- 用人工 + 工具提供一次性 `$49` 资产清理服务；
- 记录从拿到模型到进入引擎的时间、失败点和用户可接受价格。

7 天继续/停止信号：

- 至少处理 20 个真实模型；
- 至少 5 位目标用户实际试用；
- 至少 2 个付费订单；
- 高频问题能被规则化，而不是每个样本都需要独特的美术判断。

### Phase 1 — GLB Inspector

- 建立 Fixture corpus 和金标准报告；
- 完成浏览器预览、glTF 验证和基本资产指标；
- 输出 JSON 与人类可读报告；
- 明确“可自动修复”和“必须人工处理”的边界。

### Phase 2 — Compiler + Unity

- 接入 glTF-Transform、meshoptimizer 与 Blender headless；
- 完成确定性处理和回归对比；
- 输出 Unity 导入配置和 Prefab；
- 自动打开测试场景、运行并保存验证截图/日志。

### Phase 3 — Unreal

- 增加 Interchange 预设；
- 优先做只读预检和可重放导入；
- 对支持的 Unreal 版本建立固定兼容矩阵；
- 增加场景加载、材质和碰撞冒烟验证。

### Phase 4 — Generation providers

- 以统一 Adapter 接入 Meshy、Hunyuan3D 或其他服务；
- 保留原始文件、生成参数和来源；
- 生成结果进入同一检查/修复流程，不绕过质量门槛。

### Phase 5 — CLI/MCP/Plugin

- 在真实工作流稳定后提供 CLI；
- MCP 只包装稳定 Job；
- 优先集成现有 Unity/Unreal/Blender MCP，不重复建设通用编辑器控制层。

## 9. 成功指标

首个技术里程碑：

- 90% Fixture GLB 能通过验证，或产生明确可操作的失败报告；
- 自动识别至少 10 类高频资产缺陷；
- 优化后保持材质、动画和视觉结果在约定阈值内；
- 选定 Fixture 可无人工步骤进入 Unity 测试场景；
- 每一步有输入哈希、工具版本、日志和结果清单，可重放。

首个商业里程碑：

- 7 天内获得至少 2 个付费清理/导入订单；
- 用户愿意重复提交多个资产，而不仅是试用一次；
- 相比手工 Blender + 引擎导入，能量化节省时间或降低失败率。

价格仅作为待验证假设：个人订阅 `$19–39/月`，或按资产包/处理额度收费。不要在付费实验前据此设计复杂套餐。

## 10. 主要风险与应对

| 风险 | 影响 | 初步应对 |
| --- | --- | --- |
| Blender、Validator、meshoptimizer 等免费工具已存在 | 单点功能难收费 | 售卖完整工作流、报告、自动修复和引擎交付结果 |
| 生成平台继续补齐修复和导入 | 能力被上游吸收 | Provider-neutral，聚焦跨来源和跨引擎一致性 |
| Web UI 无法替代 DCC | 自动修复边界有限 | 浏览器做检查/配置，Blender Worker 做重处理 |
| Unreal 版本和插件复杂 | 支持成本高 | Unity-first；Unreal 采用明确版本矩阵和只读预检起步 |
| 自动优化损害视觉或动画 | 用户不信任结果 | Before/after 对比、可逆 Job、阈值和 Fixture 回归 |
| AI 资产许可和来源不清 | 商用与合规风险 | 把 provenance/license 作为资产清单一等字段 |
| 当前付费意愿证据不足 | 可能是技术玩具 | Phase 0 先售卖人工辅助服务，再写完整产品 |

## 11. 与现有 Forge 的边界

现有 Forge 继续聚焦本地优先的 2D 精灵处理、动画整理与导出。3D 方向应：

- 使用独立产品名、路线图和代码边界；
- 复用 Job、资产哈希、可恢复任务、QA Fixture 等设计经验；
- 不把外部项目源码直接复制进 Forge；
- 不以 3D 探索推迟当前 Forge 的发布与质量工作；
- 在需求得到验证后再决定独立仓库，避免过早搭建平台基础设施。

两个项目共享的是“把不稳定媒体输入编译为可验证游戏资产”的产品哲学，而不是相同的数据模型或 UI。

## 12. 已作出的方向性决策

| 决策 | 当前结论 |
| --- | --- |
| 将 3D 加入现有 Forge V1 | 否 |
| 建立独立兄弟项目 | 是，先做验证性原型 |
| 首个规范输入/输出 | GLB |
| 首个 UI | Web3D Inspector |
| 重处理后端 | Blender headless |
| 第一个引擎适配器 | Unity |
| 第二个引擎适配器 | Unreal |
| 先做通用引擎 MCP | 否 |
| 先训练/自托管生成模型 | 否 |
| 后续 MCP | 是，但仅作为稳定 Job API 的薄封装 |

## 13. 尚待验证的问题

1. 用户最愿意为哪一类结果付费：诊断、自动修复、LOD/压缩、绑定动画，还是引擎一键交付？
2. AI 生成资产最常见的失败模式是否足够集中，能形成规则化产品？
3. Unity-first 是否能覆盖足够大的早期用户群，还是 Web 交付更容易获得首批订单？
4. 哪些修复可以在浏览器可靠完成，哪些必须调用 Blender？
5. 资产包应如何表达碰撞、LOD、骨骼、动画和引擎特定元数据？
6. Unreal 需要支持哪些明确版本，Interchange 的稳定边界在哪里？
7. 本地桌面、Web + 本地 Worker、还是纯服务模式最符合目标用户的隐私和算力需求？

## 14. 下一步

建议下一次开发从 Phase 0 开始，而不是直接建立完整应用：

1. 创建 30 个模型的 Fixture corpus 和问题标签；
2. 编写最小 `inspect` 命令，组合 glTF Validator 与基础指标；
3. 做一个只支持 GLB 的 Three.js 报告页面；
4. 手工服务 5 位用户并记录导入全过程；
5. 达到付费与重复问题信号后，再建立独立仓库和 Unity Adapter。

## 15. 参考资料

以下资料用于本轮可行性调研；外部项目仅作为接口、生态和工作流参考，不代表采用其源码。

- SpriteCook Docs: <https://www.spritecook.ai/docs>
- SpriteCook Codex Plugin: <https://github.com/SpriteCook/codex-plugin>
- Unity command-line arguments: <https://docs.unity3d.com/6000.0/Documentation/Manual/EditorCommandLineArguments.html>
- Unity AssetDatabase: <https://docs.unity3d.com/6000.0/Documentation/ScriptReference/AssetDatabase.html>
- Unity ModelImporter: <https://docs.unity3d.com/6000.0/Documentation/ScriptReference/ModelImporter.html>
- CoplayDev Unity MCP: <https://github.com/CoplayDev/unity-mcp>
- Unreal Interchange Framework: <https://dev.epicgames.com/documentation/en-us/unreal-engine/interchange-framework-in-unreal-engine>
- Unreal Importing Assets Using Interchange: <https://dev.epicgames.com/documentation/en-us/unreal-engine/importing-assets-using-interchange-in-unreal-engine>
- Unreal MCP: <https://github.com/chongdashu/unreal-mcp>
- Godot MCP: <https://github.com/Coding-Solo/godot-mcp>
- Blender command-line arguments: <https://docs.blender.org/manual/en/latest/advanced/command_line/arguments.html>
- Blender MCP: <https://github.com/ahujasid/blender-mcp>
- Khronos glTF: <https://github.com/KhronosGroup/glTF>
- Three.js GLTFLoader: <https://threejs.org/docs/pages/GLTFLoader.html>
- model-viewer: <https://modelviewer.dev/>
- Khronos glTF Validator: <https://github.com/KhronosGroup/glTF-Validator>
- glTF-Transform: <https://github.com/donmccurdy/glTF-Transform>
- meshoptimizer: <https://github.com/zeux/meshoptimizer>
- Meshy Text to 3D API: <https://docs.meshy.ai/en/api/text-to-3d>
- Meshy Rigging API: <https://docs.meshy.ai/en/api/rigging>
- Meshy Pricing: <https://www.meshy.ai/pricing>
- Hunyuan3D 2.1: <https://github.com/Tencent-Hunyuan/Hunyuan3D-2.1>

Research snapshot: 2026-07-31. 项目活跃度、价格、接口与引擎支持范围会变化，实施前应重新核验。
