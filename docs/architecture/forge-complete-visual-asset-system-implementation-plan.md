# Forge 完整 2D 图像资产系统实施计划

状态：Implementation handoff / 待 Kimi 实施、Codex 分阶段验收  
版本：1.0  
日期：2026-08-04  
目标仓库：`MightyKartz/GameSpriteForge`  
当前基线：`v0.2.0-cli.1` 已发布，`v0.3` Character consistency V2 与跨资产 fixture matrix 已实现但尚未完成真实模型晋级门槛

## 1. 文档用途

本文档是可以直接交给 Kimi 执行的完整实施规格，不是方向性建议。

Kimi 应按本文档中的阶段顺序实施。每完成一个阶段：

1. 提交独立、可审查的 commit。
2. 写入对应的 `docs/qa/` 验收记录。
3. 保留机器可读 JSON 报告和少量代表性预览。
4. 停止进入下一阶段，等待 Codex 验收。

Codex 负责最终验收，包括代码审查、测试执行、真实模型费用审批、Godot 4.6.x 加载、视觉证据检查、凭据扫描和发布判断。

本计划不得被解释为一次性大重写。必须增量开发，保留已经发布的 CLI、Pack 和自动化兼容性。

## 2. 产品边界

### 2.1 Forge 负责的内容

Forge 是面向 Codex、Claude 和其他编码代理的完整 2D 图像资产编译系统，负责：

- 生成游戏所需的 2D 图像资产。
- 锁定并维护项目级视觉风格。
- 保持角色、集合和世界资产一致性。
- 执行确定性后处理、抠图、对齐、切帧、atlas 和预览。
- 执行按资产类型划分的质量门禁。
- 支持局部重生成和人工修图往返。
- 保存版本、来源、Provider、模型、费用、哈希和许可证信息。
- 输出版本化 `.gsfpack`。
- 安装为 Godot 4.6.x 原生资源并进行引擎验收。

### 2.2 Forge 不负责的内容

- 不生成 GDScript、C# 或其他游戏代码。
- 不实现玩家控制、战斗、任务、存档、AI、UI 行为或游戏逻辑。
- 不生成音频、音乐或配音。
- 不实现自然语言地图；Map 只接受 JSON。
- 不实现桌面 UI。
- 本路线不实现 MCP、Unity、Unreal 或 3D。
- 不自动下载或执行任意第三方插件、自定义节点或脚本。

Codex/Claude 负责：

- 编写 JSON Spec 和项目级 `GameArtManifestV1`。
- 根据 `forge_usage.json` 和项目 catalog 引用资源。
- 编写游戏代码、场景逻辑和 UI 行为。
- 决定地图玩法语义；Forge 只编译 JSON 中的视觉结构。

### 2.3 首个完整产品的限制

- 游戏类型：俯视 2D 游戏。
- 引擎：Godot `4.6.x`。
- 像素网格：首期重点支持 16px/32px Tile 与 64–512 的 2 次幂画布。
- Provider：稳定路径为 xAI API Key；xAI OAuth 保持 Preview；fixture 用于离线测试。
- 平台：当前 macOS Apple Silicon；Linux 在视觉闭环完成后实施。
- 视角：top-down；isometric、platformer 和 3D 不进入 Forge 1.0。

## 3. 完整性的产品定义

Forge 1.0 必须覆盖以下视觉资产域：

| 资产域 | 必须支持的资产 | Godot 主要交付物 |
| --- | --- | --- |
| Character | 主角、NPC、敌人、Boss、方向视图、头像 | `SpriteFrames`、`AnimatedSprite2D` 场景、PNG/atlas |
| Animation | idle、walk、run、attack、hurt、death、cast、interact | 动画资源、方向映射、循环/非循环元数据 |
| Static | icon、prop、weapon、equipment、loot、resource、decoration | 独立 PNG、atlas、`Sprite2D` 场景、usage 映射 |
| World | Terrain、道路、水体、Building、植被、墙体、门窗、Decal | `TileSet`、`TileMapLayer` 数据、场景和语义 manifest |
| Background | 场景背景、远景、水平循环 Parallax 图层 | 外部纹理、Parallax 层场景和循环元数据 |
| UI | 九宫格面板、按钮状态、槽位、血条、光标、对话框、徽标 | `Theme.tres`、`StyleBoxTexture`、图标和演示场景 |
| VFX | slash、hit、explosion、fire、heal、magic、smoke、dust、pickup | flipbook、`SpriteFrames` 或 `GPUParticles2D` 场景 |
| Map Visual | JSON 驱动的 Terrain、Building、Prop、Decal 组合 | 自包含 world pack、视觉场景和渲染预览 |

Forge 1.0 的核心验收不是“拥有这些命令”，而是一个冻结的项目级视觉清单能够在全新目录中被完整生成、审计、修复、安装和渲染。

## 4. 不可破坏的现有合同

以下合同在所有阶段持续生效：

- 公开二进制名保持 `forge`。
- stdout 每次只输出一个 JSON envelope。
- stderr/TTY 仅用于诊断、进度和交互式授权。
- 高层生成默认返回耐久 Job；`--wait` 同步等待。
- 所有有副作用的批量操作继续使用单次 plan token。
- 一个 Job 锁定一个 Provider、profile、模型和视觉 revision。
- 禁止在 Job 中静默更换 Provider 或模型。
- Provider 结果必须先落地本地、验证格式并计算 SHA-256。
- Token、API Key、Authorization header、Device Code、临时媒体 URL 不得进入 Job、Pack、日志或普通 JSON 输出。
- `.gsfpack` V1/V2/V3 继续可读。
- 当前 Character、Icon Set、Prop Set 命令和 JSON 自动化继续兼容。
- 桌面与 MCP 源码不进入默认 workspace/release；不得删除其现有源码。
- Godot 写入只允许位于 `addons/forge_assets` 和 `.forge` 的 Forge-owned 路径。
- Godot 安装保持原子覆盖、所有权检查、备份与失败回滚。
- `.tres/.tscn` 必须小于 1 MiB。
- Godot 文本资源禁止出现内嵌 Image、`PackedByteArray` 或 `ImageTexture.create_from_image`。
- Release README 只保留 curl 安装方式，不增加 Homebrew 或手动下载说明。

## 5. 当前基线

Kimi 开始前必须阅读：

- `README.md`
- `README.zh-CN.md`
- `.agents/skills/forge-dev/SKILL.md`
- `docs/qa/forge-character-loop-v2-2026-08-03.md`
- `docs/qa/forge-consistency-v2-and-world-implementation-2026-08-03.md`
- `docs/qa/forge-v03-test-matrix.md`
- `docs/qa/forge-v03-release-matrix-2026-08-04.json`
- `docs/architecture/model-neutral-media-generation.md`

已有基础：

- Style Lock、Subject Lock、Environment Lock。
- Character V1 稳定视频路径与 Loop Selection V2。
- Character V2 32 帧实验路径。
- Icon Set、Prop Set、集合内一致性 V1。
- `WorkflowGraphV1`、内容寻址缓存、JobStore、PlanStore。
- `.forge/catalog.json` 与 Godot `.forge/assets.json` 分离。
- `.gsfpack` V1/V2/V3。
- Terrain、Building、JSON Map feature-gated 实验实现。
- xAI 与 fixture Provider。
- Godot 4.6.3 headless import/load 测试。

已知缺口：

- v0.3 Character V2 尚未完成真实 xAI A/B 晋级门槛。
- fixture matrix 证明工作流正确，不证明真实视觉优越性。
- 缺少项目级期望状态清单和多 Pack 依赖协调器。
- 缺少 Collection Lock 的项目级实现。
- 缺少 Portrait、Background、UI、VFX、Decal 工作流。
- 缺少通用人工修图往返。
- 缺少 Godot 实际渲染 Gallery 验收。
- 当前 `.forge/assets.json` 的类型和字段只适合 Character/Static。
- README 中仍有部分 v0.2 发布状态描述需要同步。
- 工作树包含大量本地 QA 产物；必须建立精简证据策略。

## 6. 总体架构

```text
GameArtManifestV1
→ schema/path/dependency validation
→ immutable dependency fingerprint
→ desired-state diff against ProjectCatalogV2
→ ProjectBuildPlanV1 with cost/request/cache estimate
→ single-use plan token
→ durable parent build Job
→ typed child asset Jobs
→ Provider media materialization
→ deterministic processing
→ asset-specific quality gates
→ typed .gsfpack
→ project catalog registration
→ Godot sync plan
→ Godot import/load/render verification
→ installation catalog registration
```

### 6.1 期望状态与执行状态分离

- `GameArtManifestV1` 描述用户想要什么。
- `ProjectBuildPlanV1` 描述本次需要执行什么。
- JobStore 描述实际发生了什么。
- `.forge/catalog.json` 描述已成功生成什么。
- Godot `.forge/assets.json` 描述已成功安装什么。

禁止将以上概念合并为一个可变 JSON 文件。

### 6.2 类型化 DAG

所有新工作流必须使用 `WorkflowGraphV1` 或其兼容升级版本，不得新增临时流程 JSON。

每个节点记录：

- node ID 和 node type。
- implementation version。
- 规范化参数。
- 输入 SHA-256。
- 输出 SHA-256。
- Provider/model/profile。
- Style/Subject/Environment/Collection/UI/Effect revision。
- cache key。
- 是否发生 Provider 请求。
- 失效后代。
- 费用/请求计数。

父 Project Build Job 不直接调用 Provider；Provider 调用发生在锁定后的子资产 Job 中。

### 6.3 Provider 边界

Provider 保持媒体能力接口，不感知 Godot、Pack 或游戏资产业务类型。

Provider capability 至少包含：

```text
GenerateImage
EditImage
GenerateVideo
EditVideo
MultiReferenceImage
PrivateFileInput
NativeAlpha
SeamlessTexture
StartKeyframe
EndKeyframe
```

capability constraints 至少包含：

- 最大图片参考数。
- 最大视频参考数。
- 支持的输入媒体类型。
- 最大输入尺寸和字节数。
- 支持的输出画布。
- 是否可返回远程 private asset/file ID。

工作流在 plan 阶段检查能力。能力不足时直接返回结构化错误，不得静默降级。

### 6.4 锁定体系

保留 Style Lock 名称以兼容当前项目，在其上增加领域锁：

```text
StyleLock
├── SubjectLock
├── EnvironmentLock
├── CollectionLock
├── UiLock
└── EffectLock
```

- `StyleLock`：项目级视角、调色板族、描边、像素密度、光照、背景、画布和缩放规则。
- `SubjectLock`：角色身份、多视角 canonical、mask、装备特征与身份基线。
- `EnvironmentLock`：地表材质、建筑材料、植被、环境光照与世界比例。
- `CollectionLock`：同一 icon/prop/equipment/decal 集合的 anchor、medoid、材质、尺度和 outlier 基线。
- `UiLock`：UI 材质、边框、九宫格、状态色、文本安全区和可读性基线。
- `EffectLock`：VFX 调色板、混合模式、发光/Alpha、时间和尺度基线。

所有 lock 不可变。修改生成新 revision，旧 revision 永不覆盖。

## 7. 公共 CLI 目标

### 7.1 保留的现有命令

现有命令继续兼容：

```text
forge doctor
forge provider ...
forge project init|inspect
forge style create|inspect
forge subject create|list|inspect
forge generate character|icon-set|prop-set
forge job get|cancel|report|retry|review|graph|replay
forge pack validate
forge godot plan-install
forge plan execute
```

### 7.2 新增项目级命令

```text
forge project plan-build \
  --project <path> \
  --manifest <game-art.json> \
  --json

forge project diff \
  --project <path> \
  --manifest <game-art.json> \
  --json

forge project audit \
  --project <path> \
  [--scope completeness|consistency|quality|provenance|godot|all] \
  --json

forge project status \
  --project <path> \
  --json
```

`plan-build` 只生成未领取的单次 plan token，不启动生成。返回：

- normalized manifest hash。
- dependency graph hash。
- 新建、重建、复用、删除候选列表。
- Provider/profile/model 锁定。
- cache hits/misses。
- 预计与最大 Provider 请求数。
- 预计与最大费用 ticks。
- 不产生费用的本地节点数。
- 不满足的 capability。
- plan token 和过期时间。

执行统一使用：

```bash
forge plan execute --token <token> --wait --json
```

不得增加绕过 plan token 的批量生成入口。

### 7.3 新增锁定命令

```text
forge collection create|inspect
forge ui-lock create|inspect
forge effect-lock create|inspect
```

### 7.4 新增资产命令

```text
forge generate portrait-set
forge generate equipment-set
forge generate decal-set
forge generate background-set
forge generate ui-kit
forge generate effect-set
```

World 命令保持当前 feature-gated 设计：

```text
forge environment create|inspect
forge generate terrain-set
forge generate building-kit
forge map schema|compile|validate
```

### 7.5 人工修图往返

```text
forge asset export-editable \
  --project <path> --id <asset-id> --output <dir> --json

forge asset replace-frame \
  --id <job-id> --item <animation> --frame <index> \
  --path <png> [--wait] --json

forge asset replace-item \
  --id <job-id> --item <item-id> \
  --path <png> [--wait] --json

forge asset replace-tile \
  --id <job-id> --tile <tile-id> \
  --path <png> [--wait] --json

forge asset replace-ui-slice \
  --id <job-id> --slice <slice-id> \
  --path <png> [--wait] --json

forge asset replace-effect-frame \
  --id <job-id> --effect <effect-id> --frame <index> \
  --path <png> [--wait] --json
```

所有 replace 命令：

- 创建来源链新 Job，不修改原 Job。
- 校验 PNG、画布、Alpha、像素上限和 SHA-256。
- 只失效对应下游节点。
- 不产生 Provider 请求。
- 重新执行质量、Pack、catalog 等下游步骤。
- 原始 Provider 产物继续保留。

### 7.6 Godot 项目级交付

```text
forge godot plan-sync \
  --project <asset-project> \
  --godot-project <godot-project> \
  [--manifest <game-art.json>] --json

forge godot verify \
  --project <godot-project> \
  [--scope assets|gallery|all] --json

forge godot render-gallery \
  --project <godot-project> \
  --output <dir> --json
```

`plan-sync` 返回一次性 token；实际写入仍由 `forge plan execute` 完成。

## 8. Schema 与存储设计

### 8.1 GameArtManifestV1

新增：`schemas/game-art-manifest.schema.json`。

推荐结构：

```json
{
  "schemaVersion": "1",
  "kind": "game_art_manifest",
  "projectId": "forest-rpg",
  "name": "Forest RPG",
  "styleRevision": "style-revision-id",
  "provider": {
    "id": "xai",
    "profileId": "default"
  },
  "defaults": {
    "outputDirectory": "packs",
    "godotRoot": "addons/forge_assets",
    "license": "private"
  },
  "assets": [
    {
      "id": "forest-ranger",
      "kind": "character",
      "spec": "specs/characters/forest-ranger.json",
      "required": true,
      "dependsOn": ["subject:forest-ranger@subject-revision"],
      "tags": ["player", "forest"]
    }
  ]
}
```

约束：

- `projectId` 与 asset ID 必须是稳定 filesystem-safe ID。
- asset ID 全局唯一。
- `kind` 必须是受支持的联合类型。
- spec 路径相对 manifest 文件解析。
- 禁止绝对路径、URL、父目录穿越、符号链接逃逸和设备文件。
- plan 阶段保存 canonical path、文件大小和 SHA-256。
- `dependsOn` 必须引用已声明 asset 或不可变 lock revision。
- dependency graph 必须无环。
- 未知字段默认拒绝。
- asset `required=false` 可失败而不阻塞必需资产，但不得被 required asset 依赖。

### 8.2 新增 Spec

新增版本化类型与 JSON Schema：

- `CollectionSpecV1` / `CollectionLockV1`
- `PortraitSetSpecV1`
- `EquipmentSetSpecV1`
- `BackgroundSetSpecV1`
- `UiSpecV1` / `UiLockV1` / `UiKitSpecV1`
- `EffectSpecV1` / `EffectLockV1` / `EffectSetSpecV1`
- `DecalSetSpecV1`
- `ProjectBuildPlanV1`
- `ProjectBuildReportV1`
- `ProjectAuditReportV1`
- `GodotSyncPlanV1`
- `GodotGalleryReportV1`

所有 Schema 必须可通过：

```text
forge schema list --json
forge schema show --id <schema-id> --json
```

### 8.3 ProjectCatalogV2

`.forge/catalog.json` 升级为 V2，同时继续读取 V1。

每个资产记录：

- asset ID、kind、name、revision。
- spec path 与 SHA-256。
- dependency IDs/revisions/hashes。
- Style/Subject/Environment/Collection/UI/Effect revision。
- Provider/profile/model。
- workflow/profile/version。
- source Job、parent Job、Pack path/hash/version。
- quality verdict/profile。
- game-ready 状态。
- generatedAt、reviewedAt。
- Godot installation link（仅引用，不复制安装记录）。
- license 与 provenance 摘要。

更新必须原子写入。并发 build 必须使用 project lock，避免 catalog 丢失更新。

### 8.4 Godot 安装 Catalog V2

`.forge/assets.json` 升级为 V2，同时读取 V1。

修复当前固定字段只适合 Character 的问题。改为类型化 payload：

```json
{
  "assetId": "forest-ui",
  "kind": "ui_kit",
  "revision": 1,
  "packSha256": "...",
  "paths": {
    "theme": "res://addons/forge_assets/.../forge_theme.tres",
    "galleryScene": "res://addons/forge_assets/.../gallery.tscn"
  }
}
```

V1 字段继续为旧 Character/Static 生成。

### 8.5 `.gsfpack` V4

新增 `.gsfpack` V4，用于新资产域和统一 typed payload；V1/V2/V3 不迁移、不重写。

通用 V4 布局：

```text
<Asset>.gsfpack/
├── forgepack.json
├── assets/
│   ├── manifest.json
│   ├── atlas.json                 optional by kind
│   ├── atlas.png                  optional by kind
│   ├── items/                     optional by kind
│   ├── frames/                    optional by kind
│   ├── editable/                  optional export-only metadata
│   └── type-specific files
├── quality/
│   ├── summary.json
│   └── type-specific reports
├── provenance/
│   ├── workflow.json
│   ├── provider.json
│   └── licenses.json
└── previews/
    ├── contact-sheet.png
    ├── preview.gif                optional
    └── preview.png                optional
```

要求：

- `forgepack.json` 使用严格 Schema。
- V4 asset kind 是封闭枚举。
- 所有相对路径必须 canonical、无穿越、无 symlink。
- manifest 中记录每个文件 SHA-256。
- Pack 不保存 Token、临时 URL、Authorization header 或 Device Code。
- Pack validator 必须按 kind 校验条件字段。
- `forge pack validate`、`forge asset inspect` 支持 V1–V4。

## 9. 资产工作流规格

### 9.1 Character 与 Animation

#### 默认路径

保留已通过真实 xAI 验收的 `topdown@1.0.0` 视频路径作为默认，直至真实 A/B 数据证明其他路径更优。

#### 实验路径

- `topdown-keyframes@2.0.0`：现有 32 独立帧基线，保持 experimental。
- `topdown-hybrid@2.1.0`：能力允许时使用 2–4 关键帧、时间生成和指定帧修复。

#### SubjectLock V2

增加多视角 canonical：

```text
down/front
up/back
right
mask per view
palette/silhouette/equipment features
```

输入分别传递，不得用 collage 替代多参考。

#### 动作 Profile

新增版本化动作 profile：

- `topdown-player-core@1.0.0`：idle、walk、run、attack、hurt、death、interact、cast。
- `topdown-enemy-core@1.0.0`：idle、walk、attack、hurt、death。
- `topdown-npc-core@1.0.0`：idle、walk、interact。

左向继续由 Godot 水平翻转，除非动作或装备声明 `mirrorSafe=false`。

#### 质量

- 保留现有硬门禁和 Loop Selection V2。
- 新增 `animation-quality@3.0.0` 维度：identity、pose、flicker、smoothness、dynamic degree、alpha stability、anchor、foot-contact phase、loop closure。
- 单项硬失败不得被 composite score 掩盖。
- attack/hurt/death 等非循环动作不使用 loop closure；改用 start/end、动作能量、裁切和结束姿态门禁。

### 9.2 Portrait Set

流程：

```text
SubjectLock view
→ StyleLock portrait profile
→ anchor portrait
→ expression edits
→ foreground/crop normalization
→ identity + collection consistency
→ portrait pack
```

首期支持：neutral、happy、angry、hurt、surprised。

硬门禁：统一画布、统一裁切、单主体、无裁切、身份通过、透明或声明背景模式。

Godot 输出：独立 PNG、atlas、item→`res://` 映射。不生成对话逻辑。

### 9.3 Icon、Prop、Equipment、Decal

在现有 Static Set 基础上引入 `CollectionLockV1`：

```text
StyleLock
→ collection anchor
→ initial items
→ medoid selection
→ outlier detection
→ targeted retry
→ pack
```

集合级检查：

- 调色板族。
- 光照方向。
- 描边/边缘密度。
- 前景尺度。
- 透视和落地基线。
- pairwise/medoid outlier。
- native-size legibility（Icon）。

Equipment V1 只负责：

- inventory icon。
- world prop。
- 可选静态 equipped preview。

可穿戴逐帧 attachment layer 延后到 1.0 之后，不允许在首期伪装成稳定能力。

Decal 输出透明 PNG、落地尺寸、blend 建议和可选 atlas，不输出 gameplay trigger。

### 9.4 Terrain、Building、Map Visual

保留当前世界实现的核心边界：

- 模型生成材质和细节。
- Forge 生成 mask、tile 顺序、邻接、碰撞/遮挡建议与确定性布局。
- Map 只接受 JSON，不调用文本模型。
- WFC/局部约束只用于装饰，不参与道路、Spawn、Exit 或全局拓扑。

Terrain 晋级要求：

- `dual-grid@2.0.0` 15 mask × 4 variants。
- 全部合法邻接验证。
- 周期纹理闭合。
- variant 重复率报告。
- 随机压力图无 seam、hole、misalignment。
- Godot `TileSetAtlasSource`、terrain peering bits、物理/自定义数据加载成功。

Building 晋级要求：

- 固定模块语义、锚点、入口、碰撞建议、遮挡和 Y-sort metadata。
- 3×3 至 8×6 footprint。
- 八个确定性示例。
- 实际组合预览，而非仅 atlas 预览。

Map Visual 晋级要求：

- 编译 20 个确定性候选。
- 先过滤硬失败，再按版本化 rubric 选优。
- 依赖 Pack/hash 全部锁定。
- 自包含复制依赖资源。
- 输出实际 Terrain/Building/Prop/Decal 合成画面。
- 不输出玩法脚本。

### 9.5 Background Set

新增 profile：`topdown-parallax@1.0.0`。

Spec 声明：

- viewport aspect/resolution。
- layer 数量（1–5）。
- 每层 depth、scroll factor、repeat mode。
- 是否水平循环、垂直循环或固定画面。
- camera safe area。
- Environment revision。

流程：

```text
EnvironmentLock
→ background anchor
→ layer decomposition/generation
→ edge loop normalization
→ parallax composition test
→ pack
```

Godot 输出外部纹理、Parallax 层场景和 `forge_usage.json`。不生成相机代码。

硬门禁：尺寸、边缘闭合、层间透明度、无明显裁切、无意外主体、循环预览通过。

### 9.6 UI Kit

新增固定 profile：`godot-topdown-ui@1.0.0`。

V1 组件：

```text
panel
dialog_panel
tooltip_panel
button_normal
button_hover
button_pressed
button_disabled
slot_normal
slot_selected
health_bar_bg/fill
mana_bar_bg/fill
cursor_default
cursor_action
separator
portrait_frame
```

模型负责材质、装饰和图像外观；Forge 负责九宫格 mask、边距、状态组织和 Godot Theme。

UiLock 保存：

- border/margin rules。
- palette 与状态色。
- text-safe area。
- minimum component size。
- icon/stylebox mapping。
- font reference（Forge 不生成字体文件）。

Godot 输出：

- 外部 PNG/atlas。
- `StyleBoxTexture`。
- `Theme.tres`。
- 包含各 Control 状态的 Gallery scene。
- 320×180、640×360、1280×720 三种测试尺寸截图。

质量门禁：

- 九宫格边框在 1×、2×、4× 拉伸后不得被拉伸或断裂。
- normal/hover/pressed/disabled 必须可区分。
- 文本安全区域不得被装饰覆盖。
- 原生像素网格和 nearest sampling 保持稳定。
- Theme 引用不得缺失。

### 9.7 Effect Set

新增 profile：

- `sprite-effect-loop@1.0.0`
- `sprite-effect-burst@1.0.0`
- `particle-texture-set@1.0.0`

V1 效果：slash、hit、explosion、fire、heal、magic、smoke、dust、pickup、sparkle。

输出：

- 透明 PNG frames。
- sprite sheet/atlas。
- `SpriteFrames` 或无脚本 `GPUParticles2D` 场景。
- fps、loop/one-shot、blend mode、origin、bounds。
- contact sheet 与 GIF。

EffectLock 保存：调色板、发光、Alpha、blend、尺度和 duration 基线。

硬门禁：

- Alpha halo 和背景污染。
- 裁切。
- burst 必须从低能量开始并在结束时衰减。
- loop 必须闭合。
- 多帧运动能量不能为零。
- blend mode 与背景测试通过。

## 10. 统一质量与视觉组件

### 10.1 质量层级

```text
hard media validity
→ deterministic geometry/alpha rules
→ asset-specific visual metrics
→ collection/project consistency
→ Godot render verification
→ optional human review for gray band
```

损坏媒体、Alpha 丢失、路径越界、缺主体、多主体、裁切、缺资源和 Schema 失败永远不能人工绕过。

### 10.2 视觉组件

保留现有 `VisionComponentProtocolV1`，不要将 Python、SAM、DINO、LPIPS 或模型权重直接加入基础 CLI。

组件要求：

- 独立签名可执行程序。
- stdio JSON 协议。
- manifest Ed25519 签名。
- executable/model SHA-256。
- 许可证与再分发审计。
- 断网可运行。
- timeout、崩溃和畸形响应 fail-closed。

视觉模型指标在人工冻结数据集完成校准前只作 advisory，不替代确定性硬门禁。

### 10.3 项目级 Audit

`forge project audit --scope all` 至少检查：

- Manifest completeness。
- spec/lock/Pack dependency hash。
- 所有 required asset game-ready。
- 跨 Pack Style/Environment/Collection/UI/Effect consistency。
- Pack V1–V4 validity。
- provenance/license 完整性。
- Godot 安装状态与 stale asset。
- missing/orphaned files。
- credential/temporary URL 泄漏。
- 资源大小和嵌入图像。

## 11. 可编辑中间格式

`forge asset export-editable` 输出：

```text
editable/
├── editable.json
├── frames/
├── layers/
├── items/
├── palette.png
├── animation-tags.json
├── slices.json
├── pivots.json
└── source-hashes.json
```

`editable.json` 记录：

- source asset/job/revision。
- canvas、sampling、palette。
- frame/item/tile/slice ID。
- pivot、duration、tag、layer。
- expected SHA-256。
- allowed replacements。

格式必须是公开、版本化 JSON + PNG，不依赖 Aseprite 专有格式。用户可使用 Aseprite、Pixelorama 或任意图像编辑器。

## 12. Godot 安装与 Gallery

### 12.1 安装流程

```text
copy external PNG/atlas to staging
→ write .import settings where needed
→ Godot --headless --import
→ ResourceLoader load external textures
→ create/save native resources
→ verify resources
→ atomic directory swap
→ update .forge/assets.json
```

不得先创建 `ImageTexture` 再嵌入资源。

### 12.2 Asset Gallery

Forge 必须生成 QA Gallery，不生成游戏逻辑：

```text
ForgeAssetGallery
├── Characters
├── StaticAssets
├── Terrain
├── Buildings
├── Backgrounds
├── UI
└── Effects
```

Gallery 只用于视觉验收和资源加载：

- Character 自动切换动作/方向。
- Static 显示原生尺寸和放大尺寸。
- Terrain 显示压力图。
- Building 显示八个示例。
- Background 显示 Parallax 合成。
- UI 显示所有 Control 状态和三种分辨率。
- Effect 显示 loop/burst 和不同背景。

`forge godot render-gallery` 保存 PNG/GIF 和机器报告。不得依赖人工打开 Godot 编辑器才能完成自动验收。

## 13. 代码落点

Kimi 应优先新增小模块，不继续把所有逻辑堆进 `packages/cli/src/main.rs` 或 `packages/pack/src/lib.rs`。

### 13.1 Core

建议新增：

```text
packages/core/src/game_art/
├── mod.rs
├── manifest.rs
├── plan.rs
├── build.rs
├── diff.rs
├── audit.rs
└── types.rs

packages/core/src/collection.rs
packages/core/src/ui.rs
packages/core/src/effect.rs
packages/core/src/background.rs
packages/core/src/editable.rs

packages/core/src/quality/
├── animation_v3.rs
├── collection.rs
├── terrain.rs
├── building.rs
├── background.rs
├── ui.rs
└── effect.rs
```

修改：

- `packages/core/src/lib.rs`
- `packages/core/src/automation/types.rs`
- `packages/core/src/automation/runner.rs`
- `packages/core/src/catalog.rs`
- `packages/core/src/project/mod.rs`
- `packages/core/src/workflow_graph.rs`
- `packages/core/src/export/*`
- `packages/core/src/provider/mod.rs`

### 13.2 CLI

建议新增：

```text
packages/cli/src/commands/
├── mod.rs
├── project_build.rs
├── visual_generate.rs
├── editable.rs
└── godot_sync.rs
```

`main.rs` 只负责 Clap 路由、JSON envelope 和薄调用。现有行为必须通过回归测试保持不变。

### 13.3 Pack

建议新增：

```text
packages/pack/src/v4.rs
packages/pack/src/path_safety.rs
packages/pack/src/inspect.rs
```

如果拆分现有 `lib.rs`，必须是纯重构 commit，并先通过当前 Pack 全部测试。

### 13.4 Provider

只在 capability/constraints 确实需要时修改：

- `packages/providers/src/xai.rs`
- `packages/providers/src/fixture.rs`
- `packages/providers/src/lib.rs`

新资产域优先复用中立 `GenerateImage/EditImage/GenerateVideo`，不得为每个资产类型增加 Provider 专有函数。

### 13.5 Schema/Examples

新增对应 `schemas/*.schema.json` 和：

```text
examples/cli/complete-visual/
├── game-art.json
├── collections/
├── portraits/
├── backgrounds/
├── ui/
├── effects/
└── world/
```

### 13.6 Godot Scripts

新增：

```text
scripts/godot/sync_forge_project.gd
scripts/godot/build_forge_gallery.gd
scripts/godot/verify_forge_gallery.gd
scripts/godot/render_forge_gallery.gd
```

## 14. Feature flags 与发布面

现有 feature 保留：

```toml
consistency-v2 = []
terrain-assets = []
building-assets = ["terrain-assets"]
map-compiler = ["building-assets"]
world-assets = ["map-compiler"]
```

新增：

```toml
game-art-manifest = []
collection-consistency = []
portrait-assets = ["collection-consistency"]
equipment-assets = ["collection-consistency"]
background-assets = []
ui-assets = []
effect-assets = []
complete-visual-assets = [
  "consistency-v2",
  "game-art-manifest",
  "collection-consistency",
  "portrait-assets",
  "equipment-assets",
  "world-assets",
  "background-assets",
  "ui-assets",
  "effect-assets"
]
```

`complete-visual-assets` 只用于开发和综合测试，不直接作为所有 Release 的默认 feature。

每个版本的 release workflow 必须明确写出启用 feature。未晋级功能不得出现在公开 `forge --help`。

## 15. 分阶段实施

### 阶段 0：基线冻结与仓库卫生

目标：建立可安全增量开发的基线。

任务：

1. 保存并报告当前 `git status --short`；不得清理用户未提交文件。
2. 运行当前完整 v0.3 matrix 并保存基线 JSON。
3. 修复 README 中过期的 v0.2 发布状态描述。
4. 明确 QA artifact policy：
   - repo 保留报告、contact sheet、必要 preview 和 hash。
   - 不提交完整 JobStore、原始视频、大 Pack、Token cache。
5. 增加 `.gitignore` 精确规则时不得覆盖用户已有证据目录。
6. 建立本计划的执行状态文档：`docs/qa/forge-complete-visual-implementation-status.md`。

验收：

- 当前六门 v0.3 matrix 不变。
- v0.2 CLI product surface 不出现新命令。
- 没有删除现有用户文件。

### 阶段 1：完成 v0.3 Character 真实门槛

目标：在继续扩展资产域前，完成已有 Character consistency V2 的真实判断。

任务：

1. 运行三次全新目录真实 xAI Character V2 → Pack → Godot。
2. 运行冻结的 20 Character / 5 Style A/B benchmark。
3. 同一 case 比较 video 与 keyframe；不得改变 Provider/model/Style/Subject。
4. 保存请求数、费用、延迟、重试、身份、动作和 Godot 结果。
5. 如果 32 帧路径未达到全部晋级门槛，保持 experimental，不降低阈值。
6. 可实现 `topdown-hybrid@2.1.0` fixture/contract，但不得在没有真实数据时设为默认。

真实 Provider 费用保护：

- 默认脚本拒绝真实调用。
- 只有 `FORGE_REAL_PROVIDER_ACCEPT=1` 才允许执行。
- 运行前必须输出 plan、预计请求数和最大费用。
- 必须设置显式 `FORGE_REAL_PROVIDER_MAX_REQUESTS` 和 `FORGE_REAL_PROVIDER_MAX_COST_TICKS`。
- 超出预算立即阻断，不得自动继续。

晋级门槛：

- 20 Character、至少 5 Style。
- 自动尝试内 Pack 成功率 ≥90%。
- 硬缺陷拦截率 100%。
- 错误 Pack 导出数 0。
- 身份一致性比当前 video 提升至少 10 个百分点。
- 每 Pack Provider 图片请求中位数 ≤40。
- Godot 加载率 100%。

未全部达到时：video 保持默认，v0.3 只发布已经证明稳定的部分。

### 阶段 2：GameArtManifest 与项目 Build Core

目标：先让现有 Character/Icon/Prop 能被项目级 manifest 统一规划和构建。

任务：

1. 实现 `GameArtManifestV1` Schema 和 Rust 类型。
2. 实现严格 path/symlink/URL/traversal 检查。
3. 实现依赖解析、重复 ID、未知引用和 cycle 检查。
4. 实现 normalized manifest 与 graph SHA-256。
5. 实现 `project diff`。
6. 实现 `project plan-build` 与 single-use plan token。
7. 实现 durable parent Job 与 child Job 协调。
8. 实现取消传播、失败聚合和恢复。
9. 实现 ProjectCatalogV2 兼容读取/原子写入。
10. 首期只支持已稳定的 Character/Icon/Prop。

验收：

- 相同 manifest/spec/hash 得到相同 plan hash。
- 已满足资产为 reuse，不创建 Provider Job。
- 修改一个 icon spec 只失效其 collection 和下游 pack。
- required dependency 失败阻断依赖资产。
- optional asset 失败不阻断无依赖的 required 资产。
- cancel parent 会请求取消所有 active child。
- 崩溃后可从 JobStore 恢复。
- 并发 build 不损坏 catalog。
- 所有 stdout 保持单 JSON。

### 阶段 3：CollectionLock、Portrait 与 Static 完整化

目标：形成项目级静态资产体系。

任务：

1. 实现 CollectionLock。
2. 将 Icon/Prop 升级为 anchor → medoid/outlier 流程。
3. 实现 Portrait Set。
4. 实现 Equipment V1（icon/world prop/static preview）。
5. 实现 Decal Set。
6. 实现 replace-item 与 editable export。
7. 实现跨 Pack project audit。
8. 加入 fixture 的 pass/gray/fail/second-success/outlier 样本。

真实门槛：

- 至少 5 Style。
- 每 Style 1 Icon Pack、1 Prop Pack、1 Portrait Pack。
- Icon/Prop 每 Pack 至少 10 items。
- 单 item 失败只重试该 item。
- 集合 outlier 全部被拦截。
- 真实原生尺寸 contact sheet 人工验收。
- Pack/Godot/credential scan 100% 通过。

### 阶段 4：World 资产正式化

目标：将已有 Terrain/Building/Map 从 compile-gated experiment 晋级为可发布视觉能力。

任务：

1. 补齐 Environment/Style/Collection 关联。
2. Terrain 增加 variant repetition 与真实渲染压力图。
3. Building 增加组合预览和模块定向替换。
4. Map preview 改为真实资源合成，不再使用抽象色块作为主要视觉证据。
5. 实现 replace-tile。
6. 加入 `GameArtManifestV1` 支持。
7. 升级 ProjectCatalog/Godot assets type payload。
8. 通过 Godot 4.6.x 实际资源和 Gallery 加载。

真实门槛：

- 森林、沙漠、雪地 3 个 Environment。
- 16px 与 32px。
- Terrain 全 mask/variant/邻接通过。
- 每个 Building Kit 八个组合示例。
- 每 Environment 至少 30 个 Map seed。
- 同一 spec/dependency/compiler version 输出相同 layout hash。
- 不导出任何 validation fail Map。

### 阶段 5：Background 与 UI

目标：补齐游戏画面和界面视觉资产。

任务：

1. 实现 BackgroundSetSpec/Workflow/Pack/Godot。
2. 实现 UiLock、UiKitSpec 与固定 Godot UI profile。
3. 实现九宫格确定性切片和 `Theme.tres` 生成。
4. 实现 UI Gallery 三分辨率渲染。
5. 实现 replace-ui-slice。
6. 加入 manifest/build/audit/sync。

验收：

- Background 水平循环测试通过。
- Parallax Gallery 加载和渲染通过。
- UI 三种分辨率无边框拉伸、缺图或文本区覆盖。
- Button 四状态均存在且视觉可区分。
- UI Pack 无字体版权内容；只引用用户提供的 font path/hash。
- 所有纹理外部加载、无 embedded Image。

### 阶段 6：VFX 与 EffectLock

目标：补齐战斗和交互视觉效果。

任务：

1. 实现 EffectLock。
2. 实现 burst/loop/particle texture profiles。
3. 实现 effect pack 与 Godot `SpriteFrames`/`GPUParticles2D` 输出。
4. 实现 replace-effect-frame。
5. 实现透明/黑/白/游戏背景多背景预览。
6. 加入 manifest/build/audit/sync。

验收：

- 10 类冻结 effect fixture。
- burst 与 loop 判定正确。
- 静止伪动画被拒绝。
- Alpha halo/crop/background contamination 被拒绝。
- one-shot/loop/fps/blend metadata 正确。
- Godot Gallery 自动播放并成功渲染。

### 阶段 7：Project Audit、Godot Sync 与 Gallery 完整化

目标：形成单清单到完整 Godot 图像资源库的闭环。

任务：

1. `project audit --scope all` 覆盖所有资产域。
2. `godot plan-sync` 安装全部 required game-ready Pack。
3. 删除/替换只允许 Forge-owned stale resource。
4. 实现完整 Gallery。
5. 实现真实渲染截图和机器报告。
6. 实现 Pack/catalog/install 三方 hash 对账。
7. 实现 orphan、stale、missing dependency 报告。

验收：

- 增量 sync 只修改变化的资产目录。
- 失败 sync 保留旧资源。
- catalog 与安装记录一致。
- Gallery 全部资源加载且截图存在。
- `.tres/.tscn < 1 MiB`。
- 无 embedded Image/PackedByteArray。
- 无 credential/temporary URL。

### 阶段 8：Forge 1.0 冻结视觉项目

创建 `examples/complete-visual/forest-village/`：

- 1 主角。
- 3 类敌人。
- 2 NPC。
- Character 使用适用的 5–8 个动作 profile。
- 24 icons。
- 20 props/equipment/decal items。
- 2 Terrain Sets。
- 1 Building Kit。
- 1 JSON Map。
- 1 UI Kit。
- 10 VFX。
- 3 层 Parallax Background。

连续三次在全新目录中运行：

```text
GameArtManifest
→ plan-build
→ execute
→ project audit
→ pack validate
→ godot plan-sync
→ execute
→ godot verify
→ render-gallery
```

Forge 1.0 硬门槛：

- required asset completeness 100%。
- game-ready Pack 100%。
- 错误 Pack 导出数 0。
- Godot resource load 100%。
- project audit 通过。
- 所有失败可定位到具体 asset/item/frame/stage。
- 局部替换不产生 Provider 请求。
- 相同输入和本地确定性节点输出 hash 可复现。
- Provider 使用量、费用、重试和来源链可重建。
- 无凭据或临时 URL。
- Codex/Claude 只读取 manifest、catalog 和 usage 就能引用全部图像资源。

## 16. 测试矩阵

### 16.1 Rust 单元/集成测试

覆盖：

- 所有 Schema valid/invalid/unknown field。
- 路径穿越、绝对路径、URL、symlink、设备文件。
- DAG cycle、重复 ID、未知引用。
- normalized manifest/hash 稳定性。
- ProjectCatalog V1→V2 读取。
- Godot assets V1→V2 读取。
- Pack V1–V4 读取和条件字段。
- cache hit、corruption、Provider/model isolation。
- parent/child Job cancel/recovery。
- replace 操作失效传播和零 Provider 请求。
- 每种质量 profile。

### 16.2 Fixture 测试

每个新资产域必须包含：

- 第一次成功。
- 灰区待审核。
- 硬失败。
- 第一次失败、第二次成功。
- 定向重试。
- 本地 replay 零 Provider 请求。
- cancel/timeout/malformed media。
- Pack/Godot/credential scan。

### 16.3 Godot 测试

- 固定 Godot `4.6.3`。
- `--headless --import`。
- ResourceLoader 加载所有外部纹理和资源。
- Gallery scene instantiate。
- UI 三尺寸。
- VFX 播放。
- Terrain/Building/Map 视觉场景。
- `.tres/.tscn` 大小和嵌入扫描。

### 16.4 CI

新增 `.github/workflows/complete-visual-quality.yml`，按 feature matrix 分 Job：

```text
core-and-pack
character-and-static
world
background-and-ui
effects
project-build-and-godot-gallery
```

CI 默认只用 fixture，不读取 xAI 凭据。上传：

- gate JSON。
- 日志。
- contact sheets。
- Gallery screenshots。
- secret scan 结果。

真实 Provider workflow 必须手动触发、受 environment approval 和预算限制，不得在普通 PR 自动运行。

### 16.5 持续回归命令

每阶段至少执行：

```bash
cargo fmt --manifest-path /Users/kartz/Development/Forge/Cargo.toml --all -- --check
cargo clippy --manifest-path /Users/kartz/Development/Forge/Cargo.toml --workspace --all-targets --all-features -- -D warnings
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml --workspace --no-fail-fast
bash scripts/test-cli-product.sh
bash scripts/test-v03-release-matrix.sh
bash scripts/test-cli-installer.sh
bash scripts/test-cli-signing-contract.sh
```

再运行当前阶段的 focused script。

## 17. 安全、许可证与商业门槛

- 输入图像必须限制最大字节数、像素数和解码后内存。
- Pack/spec/manifest 路径不得穿越或通过 symlink 逃逸。
- 远程 Provider ID 可以保存；signed download URL 不可保存。
- 人工替换图像记录输入 SHA-256 和用户声明许可证。
- 每个资产记录 prompt、Provider、model、workflow、输入引用和 license。
- UI Font 只接受用户提供路径和 hash；Forge 不生成或捆绑未知许可证字体。
- 外置视觉模型组件在权重商业再分发审计完成前不得发布。
- ComfyUI 后续只能作为 Provider adapter；禁止自动安装 arbitrary custom node。
- xAI OAuth 在取得商业许可前保持 Preview；API Key 是稳定商业路径。
- 不宣称生成内容绝对无版权风险；提供可审计 provenance。

## 18. 性能与成本预算

每个 plan/report 必须记录：

- Provider request estimated/max/actual。
- cost ticks estimated/max/actual。
- cache hits/misses。
- local processing duration。
- Provider latency。
- Pack size。
- Godot installed size。

目标：

- `project diff` 对 500 个 asset entry 在常规开发机上应保持秒级，不读取 Provider。
- 未变化项目 rebuild 不产生 Provider 请求。
- 单 item/frame/tile/slice 替换不重建无关资产。
- atlas 和外部纹理避免重复像素嵌入。
- JobStore 大文件可按 retention policy 清理，但报告和 Pack provenance 不丢失。

## 19. 文档与示例

每个稳定资产域必须提供：

- JSON Schema。
- 最小 JSON 示例。
- CLI 生成示例。
- plan-only/cost 示例。
- targeted retry/replace 示例。
- Pack inspect/validate 示例。
- Godot sync/usage 示例。
- 常见错误与 `nextActions`。

README 只展示稳定 Release 命令。未发布能力写入独立 architecture/QA 文档，不混入五分钟公开流程。

## 20. Kimi 每阶段交付格式

每阶段交付必须包含：

```text
Implementation summary
Changed files
Public contract changes
Compatibility statement
Tests executed with exact commands
Machine-readable QA report path
Representative preview paths
Known limitations
Provider requests and cost
Credential scan result
Commit SHA
```

Kimi 不得：

- 删除或覆盖用户未提交修改。
- 使用 `git reset --hard`、`git checkout --` 或广泛清理命令。
- 将真实 Token 写入环境报告或命令输出。
- 在未获费用批准时运行真实 xAI benchmark。
- 为通过测试降低现有质量门禁。
- 将 fixture 结果描述为真实模型视觉质量。
- 在实现过程中顺带恢复桌面、MCP、Unity、Unreal 或自然语言地图。
- 复制 SpriteCook、ComfyUI、Aseprite、Tiled 或其他项目源码。

## 21. Codex 分阶段验收协议

Codex 在每阶段执行：

1. 读取 Kimi 提交摘要和 commit diff。
2. 检查 `git status`，确认未覆盖用户修改。
3. 检查 Schema、CLI envelope、退出码和兼容默认值。
4. 运行 focused Rust/CLI/Pack 测试。
5. 运行完整默认 workspace 与 v0.3 regression。
6. 使用临时 JobStore/PlanStore 运行 fixture E2E。
7. 验证 Pack 与 Godot 4.6.3 headless import/load。
8. 检查 `.tres/.tscn` 大小和嵌入像素。
9. 扫描 JobStore、Pack、日志和 Godot 项目中的凭据/URL。
10. 查看 contact sheet、GIF 和 Gallery screenshots。
11. 对真实 Provider 阶段先审批 plan 和费用，再执行验收。
12. 只有全部硬门槛通过才批准进入下一阶段。

若失败，Codex 返回：

- blocker/error code。
- 精确文件/命令/证据。
- 是否需要代码修复、数据校准或真实 Provider 重跑。
- 允许重跑的最小范围。

## 22. 发布顺序建议

| Release | 公共能力 |
| --- | --- |
| v0.3 | 经真实门槛确认的 Character consistency V2；未通过的 keyframe 路径继续 experimental |
| v0.4 | `GameArtManifestV1` 对稳定 Character/Icon/Prop 的 plan/diff/build/audit |
| v0.5 | CollectionLock、Portrait、Equipment V1、Decal |
| v0.6 | Terrain、Building、JSON Map 正式化 |
| v0.7 | Background、UI Kit、Godot Theme |
| v0.8 | VFX、EffectLock、Particle/Flipbook |
| v0.9 | 全资产 Godot sync、Gallery、完整项目 audit |
| v1.0 | Forest Village 冻结视觉项目三次真实闭环通过 |

版本可以因实现结果调整，但不得跳过阶段硬门槛，也不得因为版本号压力降低质量标准。

## 23. 研究与工程参考

仅参考思想和协议，不复制源码：

- SpriteCook Documentation: <https://www.spritecook.ai/docs>
- SpriteCook API: <https://www.spritecook.ai/api-docs>
- SpriteCook Frame Animation: <https://www.spritecook.ai/docs/guide-frame-animation>
- Graphical Game Asset Generation Review: <https://arxiv.org/abs/2311.10129>
- Sprite Sheet Diffusion: <https://arxiv.org/abs/2412.03685>
- ComfyUI: <https://github.com/Comfy-Org/ComfyUI>
- Diffusers: <https://github.com/huggingface/diffusers>
- Aseprite CLI: <https://github.com/aseprite/docs/blob/main/cli.md>
- Pixelorama: <https://github.com/Orama-Interactive/Pixelorama>
- Tiled: <https://github.com/mapeditor/tiled>
- Godot image import: <https://docs.godotengine.org/en/4.6/tutorials/assets_pipeline/importing_images.html>
- Godot CLI: <https://docs.godotengine.org/en/stable/tutorials/editor/command_line_tutorial.html>
- Godot Theme: <https://docs.godotengine.org/en/stable/classes/class_theme.html>
- Godot 2D particles: <https://docs.godotengine.org/en/4.6/tutorials/2d/particle_systems_2d.html>

## 24. 最终产品验收语句

只有当以下语句在真实项目中成立时，Forge 才能宣称为完整图像资产系统：

> Codex 或 Claude 提供一个版本化 JSON 视觉清单；Forge 在受控费用内生成、验证、修复、打包并同步完整的俯视 2D 游戏图像资源库；Godot 4.6.x 能加载和渲染所有资源；任何失败都能定位并局部修复；全流程不依赖桌面 UI，不泄漏凭据，也不需要 Forge 编写游戏代码。
