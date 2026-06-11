# game-sprite-forge 项目开发方案

日期：2026-05-31

## 1. 项目定位

`game-sprite-forge` 是一个 macOS-first 的 2D 游戏动画资产创作、打包和分发生态工具。它帮助游戏开发者和创作者把文本、角色图、AI 视频、AI 连续帧、已有 sprite sheet、手工视频或创作者 recipe 转成游戏引擎可用的动画素材：单帧 PNG、透明 sprite sheet、atlas、GIF/WebM 预览、manifest、资源包和引擎导出文件。

项目不应被定义为 Godot 专用工具。Godot 是第一批重点验证目标之一，但核心输出应覆盖大部分 2D 游戏开发场景，包括 Godot、Unity、Phaser、PixiJS、Cocos、GameMaker、LÖVE、自研引擎和通用 asset pipeline。

推荐定位：

```text
Game Sprite Forge V1 = macOS local asset forge app + open .gsfpack format
Future = lightweight website + BYOK generation + creator ecosystem + MCP automation
```

## 2. 核心结论

1. 先做官网直发 macOS App，不首发 Mac App Store。
2. 个人开发者第一版不做轻网站；发布和说明可以先用本地文档、README、GitHub Release 或手动分发。
3. 第一版只实现本地 `.gsfpack` 创建、导入、校验和导出；创作者生态保留数据结构，但不做线上 registry、提交、审核和付费 marketplace。
4. 第一版导出通用游戏素材，Godot 只是可选 exporter，不是产品边界。
5. MCP 不作为第一入口，等 core/app/pack 格式稳定后再做。
6. Codex Skill 只作为后期轻封装，不承担核心生成和处理能力；个人版 MVP 不实现 Skill/MCP。
7. 底层必须保持 headless core，macOS App、CLI、MCP、网站后台都调用同一套能力。
8. 不把项目绑定为 `video-to-sprite`。视频模型只是第一批 source provider；未来图像模型直接生成连续帧或 sprite sheet 时，只替换 source provider，不替换 Forge Pipeline。
9. 第一版不调用图像模型或视频模型，不处理 API key、模型费用、额度、失败重试和云端缓存。

### 2.1 2026-06-04 调研补充：Sprite Video Lab 的启发

调研 `sparklecatta-lang/sprite-video-lab` 后，开发路线需要更明确地分层：

```text
Sprite Video Lab 可借鉴：
  本地导入 -> 区间预览 -> 单帧参数预览 -> 抽帧/抠图 -> 帧选择 -> sheet/zip/manifest

Forge 必须新增：
  Source Provider -> 质量门 -> 脚底/锚点/循环检测 -> .gsfpack -> 引擎导出 -> 创作者生态 -> CLI/MCP
```

这说明第一阶段不应该急着做完整生成模型集成，而应该先做一个足够硬的本地后处理内核和工作台。只有当本地视频、序列帧和 sprite sheet 都能稳定锻造成 `.gsfpack` 后，AI 生成源才有可靠落点。

具体吸收：

- 保留“单帧预览参数效果”作为 Background step 的核心交互。
- 使用 job 目录记录 source、raw frames、processed frames、preview、export 和 report。
- 将 chroma、luma、AI matting、edge refinement 设计为可替换 matting provider。
- 让用户可以从已有序列帧直接进入动画预览和导出，不强迫所有人走生成链路。

明确不照搬：

- 不采用“视频处理工具”作为产品定位。
- 不把 Python/Flask/OpenCV 网页服务作为 macOS App 的最终核心架构。
- 不允许没有质量报告的结果直接被标记为 game-ready。
- 不把 frames/sheet/manifest 作为终点，而是继续打包为 `.gsfpack` 并服务生态。

### 2.2 2026-06-04 个人开发者 MVP 收缩

为了降低个人开发者第一版的复杂度和成本，当前执行范围收缩为：

```text
只做 macOS 本地系统应用
只支持用户导入视频、PNG 序列、已有 sprite sheet 和 .gsfpack
不调用图像模型
不调用视频模型
不做 BYOK 设置
不做轻网站
不做 MCP
不做线上 marketplace
```

这个收缩不会推翻长期架构。`Source Provider` 仍然保留，但 MVP 只启用：

```text
import_video
import_frames
import_sprite_sheet
import_gsfpack
```

暂时隐藏或禁用：

```text
text_to_reference_image
reference_to_motion_video
image_sequence_generation
sprite_sheet_generation
pose_guided_generation
marketplace_recipe
```

当前第一目标不是“AI 生成角色动画”，而是：

```text
把用户已有的视频或帧素材，稳定锻造成游戏可用的 2D 动画资产。
```

只要这个本地闭环足够好，未来再接入 BYOK 生成时，模型输出就有可靠的后处理落点；如果这个闭环做不硬，过早接入生成只会放大模型成本和质量不确定性。

### 2.3 2026-06-05 当前开发进展

当前项目已经从方案阶段进入可运行的本地 macOS MVP 阶段。现有实现不再只是 UI 原型：

```text
Tauri + React macOS App
Rust core / pack packages
本地 job store
ffmpeg/ffprobe 检查与视频 probe
视频抽帧
PNG 序列导入
sprite sheet 切帧导入
.gsfpack 导入、校验、重导出
chroma 单帧预览与批处理
square-bottom 归一化
手动脚底 anchor
loop range
quality report
frames / sprite_sheet / atlas / manifest / quality-report / preview.gif / .gsfpack 导出
Run Summary、Export readiness、错误恢复卡片
```

当前本机安装状态：

```text
/Applications/Game Sprite Forge.app
CFBundleExecutable: Game Sprite Forge
CFBundleIdentifier: dev.gamespriteforge.desktop
Developer ID codesign: pass
Gatekeeper spctl: rejected, source=Unnotarized Developer ID
```

因此，当前阶段的判断是：

```text
本地手动测试候选：可用
公开分发候选：未完成，阻塞于 Apple notarization 凭据和 stapling / Gatekeeper 验证
```

最新验证命令：

```bash
npm --workspace apps/mac run build
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml
npm --workspace apps/mac run smoke:ui:mvp
node /Users/kartz/.codex/skills/impeccable/scripts/detect.mjs --json apps/mac/src/App.tsx apps/mac/src/routes/ForgeRoute.tsx apps/mac/src/components/QualityInspector.tsx apps/mac/src/components/ExportPanel.tsx apps/mac/src/styles/app.css
npm --workspace apps/mac run tauri -- build
```

下一阶段不应扩大到 AI 生成、网站、marketplace 或 MCP。优先顺序应保持：

1. 手动跑通真实用户素材导入、处理、导出、重导入。
2. 补齐 notarization / stapling / Gatekeeper release gate。
3. 再考虑 CLI、MCP、网站或生成 provider。

## 3. 为什么选择官网直发 macOS App

### 3.1 选择 macOS App 的原因

这个产品的核心体验需要本地文件、视频处理、PNG 批处理、预览和导出：

- 拖入角色图、视频、sprite sheet 或资源包。
- 本地执行 ffmpeg 抽帧。
- 去背景、裁切、bbox 检测、锚点对齐。
- 可视化检查帧序列、脚底线、循环区间和透明背景。
- 导出到本地游戏项目目录。
- 管理本机素材库、创作者资源包和 license。
- 后期安装 CLI/MCP helper，让 Codex 使用本地能力。

这些能力在纯网站里也能做一部分，但会遇到上传成本、隐私、浏览器文件权限、WASM 性能和本地项目写入限制。macOS App 是更自然的创作工作台。

### 3.2 选择官网直发的原因

官网直发比 Mac App Store 更适合本项目：

- 允许更自由地集成 CLI、MCP server、helper tool 和本地处理工具。
- 更适合资源包、插件、创作者分成和未来 marketplace。
- 可以使用 Stripe、Paddle、Lemon Squeezy 等支付和授权方案。
- 迭代速度更快，不需要每个版本等待 App Review。
- 可以避免一开始就被 App Store 内购规则限制资源生态设计。

需要承担的事项：

- Developer ID 签名和 notarization。
- 自动更新机制，例如 Sparkle。
- 崩溃上报和日志诊断。
- License 校验。
- 安装信任教育和下载安全说明。

参考：

- Apple Developer ID: https://developer.apple.com/developer-id/
- Apple App Review Guidelines: https://developer.apple.com/app-store/review/guidelines/

## 4. 轻网站定位（后置）

轻网站仍然是长期需要的生态入口，但不属于个人开发者 MVP。第一版可以先用 README、本地文档、GitHub Release、手动下载链接和示例文件说明替代。

### 4.1 未来网站 V1 功能

- 官网首页。
- macOS App 下载页。
- 安装和安全说明。
- 文档和快速开始。
- 示例 `.gsfpack` 下载。
- 资源包展示页。
- 创作者展示页。
- 提交资源包 / waitlist。
- 更新日志。
- 简单资源索引。

### 4.2 网站暂不做

- 云端视频处理。
- 完整在线编辑器。
- 完整付费 marketplace。
- 自动分账系统。
- 大规模 UGC 审核后台。
- Hosted credits 模型调用平台。

轻网站的目标是让生态可见，让 app 下载、资源包传播、创作者展示和文档闭环成立。但在本地 App 的导入、后处理、质检、导出闭环完成之前，网站不应抢占实现时间。

## 5. 创作者生态设计

生态是项目护城河，不是后期附加功能。第一版就应该让用户能创建、导入、预览、校验和分享资源包。

### 5.1 生态里可以流通的内容

- 成品角色动画包。
- 动作模板，如 idle、walk、run、attack、hurt、death。
- 风格包，如像素风、手绘风、横版平台、俯视 RPG。
- prompt / recipe。
- 后处理参数。
- 调色板和像素化配置。
- 引擎导出模板。
- Godot / Unity / Phaser 示例项目片段。

最有价值的不只是最终图片，而是可复用的 production recipe：用户购买或安装后，可以生成同风格、同规格、同导出格式的新素材。

### 5.2 `.gsfpack` 资源包格式

建议从第一版定义开放资源包格式：

```text
hero-knight.gsfpack/
  forgepack.json
  previews/
    idle.gif
    walk.gif
    attack.gif
  assets/
    frames/
      walk_000.png
      walk_001.png
    sprite_sheet.png
    atlas.json
    manifest.json
  recipes/
    character.recipe.json
    walk.recipe.json
    postprocess.recipe.json
  exports/
    godot/
    unity/
    phaser/
  LICENSE.md
  quality-report.json
```

`forgepack.json` 示例：

```json
{
  "schemaVersion": "0.1.0",
  "id": "creator.hero-knight",
  "name": "Hero Knight",
  "version": "1.0.0",
  "creator": {
    "name": "Creator Name",
    "url": "https://example.com"
  },
  "license": {
    "type": "personal-commercial",
    "summary": "Use in personal and commercial games; redistribution of raw assets prohibited."
  },
  "assetTypes": ["frames", "sprite_sheet", "atlas", "recipe", "engine_export"],
  "animations": ["idle", "walk", "attack"],
  "tags": ["side-view", "fantasy", "platformer"],
  "compatibleEngines": ["generic", "godot", "unity", "phaser", "cocos", "gamemaker"],
  "preview": {
    "gif": "previews/walk.gif"
  }
}
```

### 5.3 App 内生态功能

App V1 应包含：

- Create Pack。
- Import Pack。
- Preview Pack。
- Validate Pack。
- Export Pack。
- Install Pack。
- Share Pack。
- Publish Draft。

`Publish Draft` 可以先生成一个可上传到官网的包和 metadata，不必第一版就完成全自动上架。

## 6. 通用导出目标

第一版应优先保证通用格式，而不是深度绑定某个引擎。

### 6.1 必须支持

```text
frames/*.png
sprite_sheet.png
atlas.json
manifest.json
preview.gif
quality-report.json
```

### 6.2 建议支持

```text
preview.webm
texturepacker.json
aseprite-compatible layout
```

### 6.3 第一批引擎适配

优先级建议：

1. Generic PNG + JSON manifest。
2. Godot `AnimatedSprite2D` / `SpriteFrames` 导入说明或 exporter。
3. Unity 2D sprite sheet / sliced sprites 导入说明。
4. Phaser/PixiJS atlas JSON。
5. Cocos / GameMaker 通用 atlas。

Godot 可以作为第一批 exporter，但文案和架构都不要把项目说成 Godot 专用。

## 7. 产品架构

推荐架构：

```text
game-sprite-forge/
  packages/
    core/              # 抽帧、去背景、bbox、锚点、sheet、manifest、pack 校验
    providers/         # BYOK/本地 source provider adapter：图像、视频、连续帧、sprite sheet、recipe
    exporters/         # generic/godot/unity/phaser/cocos exporter
    pack/              # .gsfpack schema、读写、校验、版本迁移
    cli/               # gsf 命令行
    mcp-server/        # 后期 gsf-mcp
  apps/
    mac/               # macOS Creator App
    website/           # 官网、下载、资源索引、文档
  examples/
    packs/
    videos/
    outputs/
  docs/
```

### 7.1 技术栈建议

为了兼顾 macOS App、未来跨平台和网站复用，建议优先考虑：

```text
macOS App: Tauri + React/TypeScript + Rust
Core: Rust
Image processing: Rust image stack + optional native tools
Video processing: bundled ffmpeg / user-provided ffmpeg
Website: Next.js / Astro
Pack schema: JSON Schema + semantic versioning
Auto update: Sparkle or Tauri updater
```

备选方案：

- SwiftUI + Rust/CLI core：macOS 原生体验更好，但与网站 UI 复用较弱。
- Electron + Node core：开发快，但体积更大，本地处理和分发体验更重。

当前推荐：`Tauri + Rust core`。它比较适合官网直发、本地文件处理、未来 Windows/Linux 延展和 CLI/MCP 复用。

## 8. 本地处理流水线

基础流程应分成可替换的 Source 层和稳定的 Forge Pipeline：

```text
Source
  -> import_video / import_frames / import_sprite_sheet
  -> image_reference_provider
  -> motion_video_provider
  -> image_sequence_provider
  -> sprite_sheet_provider
  -> creator_recipe_provider

Forge Pipeline
  -> source 持久化和溯源
  -> 抽帧
  -> 背景移除或 chroma key
  -> bbox 检测
  -> 帧筛选
  -> 脚底/中心锚点对齐
  -> 统一 canvas
  -> 循环段检测
  -> sprite sheet / frames / atlas / preview
  -> pack 生成
  -> 引擎导出
```

质量门：

- 是否只有一个主要角色。
- 是否全身可见。
- 背景是否可移除。
- 每帧 bbox 是否稳定。
- 中心点是否漂移过大。
- 脚底锚点是否稳定。
- 动作是否适合循环。
- 帧间差异是否过大或过小。
- 输出尺寸是否一致。
- alpha 边缘是否存在明显脏边。

质量失败时，App 不应静默输出劣质资源，应显示原因和修复建议。

## 9. Source Provider、AI 生成和 BYOK 策略

第一版不应依赖 AI 生成能力才能成立，也不应绑定某一种模型能力。推荐先做 Local Mode：

```text
用户提供角色图和 MP4 -> App 完成本地后处理 -> 输出游戏素材
```

随后增加 BYOK generation mode：

```text
用户配置自己的 provider key
  -> 生成角色参考图
  -> 生成短动作视频、连续帧或 sprite sheet
  -> 下载到本地并记录 source metadata
  -> 进入同一套本地后处理流水线
```

长期 provider 分类：

```text
Input providers:
  - import_video
  - import_frames
  - import_sprite_sheet
  - import_gsfpack

AI providers:
  - text_to_reference_image
  - reference_to_motion_video
  - text_or_reference_to_image_sequence experimental
  - text_or_reference_to_sprite_sheet experimental
  - pose_guided_generation

Recipe providers:
  - creator_recipe
  - installed_pack_recipe
  - marketplace_recipe
```

产品护城河不在 provider 本身，而在 provider 之后的质量检查、锚点稳定、sprite sheet packing、manifest、engine exporter、`.gsfpack` 和生态复用。

两轮本地 walk cycle 生成测试说明：纯图像模型可以产出漂亮的角色候选图，也能配合绿幕进入 pipeline，但很难只靠 prompt 生成动作规律正确、bbox 稳定、cell 边界干净的生产级 walk cycle。因此第一版应把纯 image sheet generation 定义为 `Quick Prototype / Candidate Provider`，而不是核心质量承诺。

生成能力分级：

```text
Level 1: Quick Prototype
  prompt/reference -> candidate sprite sheet
  适合占位、灵感、demo 和 pipeline 测试

Level 2: Pose-guided Generation
  pose template / action recipe -> constrained frames
  适合提高 walk/run/attack 动作结构稳定性

Level 3: Video-to-Frames
  reference image -> short motion video -> frames
  当前更适合连续动作候选

Level 4: Rig / Template Recipe
  skeleton / action template / creator recipe -> frames
  长期最接近可复用生产能力
```

模型调用前必须做 cost preflight：

```text
即将生成：
- 1 张角色参考图
- 1 个 2 秒动作视频或一组连续动画帧
- 抽取 12 帧

预计消耗：
- image generation: ...
- video generation: ...

是否继续？
```

默认原则：

- 默认 local mode。
- 默认 BYOK。
- 不用项目维护者的 API key 代付。
- 短视频或短序列优先。
- 纯图像 sprite sheet 生成默认标记为 experimental。
- 生成结果缓存。
- 所有模型调用都要可追踪 source/provider/cost。

## 10. MCP 与 Codex 集成

MCP 不应作为第一阶段核心入口。它适合在 core、pack、exporter 稳定后，作为 Codex 和其他 agent 的自动化接口。

推荐 MCP server：

```bash
gsf-mcp
```

MCP 工具分三类：

本地免费操作：

- `list_installed_packs`
- `inspect_pack`
- `inspect_video`
- `extract_frames`
- `remove_background`
- `normalize_frames`
- `build_sprite_sheet`
- `validate_pack`
- `export_pack`

会产生模型费用：

- `estimate_generation_cost`
- `generate_character_reference`
- `generate_motion_video`

会写入项目文件：

- `export_to_project`
- `create_engine_import_files`
- `install_pack_to_project`

MCP 不负责商店 UI、支付、创作者后台或复杂视觉调参。遇到需要人工视觉判断的步骤，应返回报告并提示用户在 macOS App 中打开。

参考：

- Model Context Protocol: https://modelcontextprotocol.io/

## 11. 阶段路线

### Phase 0：项目骨架与资源包标准

目标：建立项目长期边界。

- 建立 monorepo。
- 定义 `.gsfpack` schema。
- 定义 generic manifest。
- 建立示例 pack。
- 建立质量报告格式。
- 建立官网占位页。

验收：

- 一个示例 `.gsfpack` 能被校验。
- 文档说明 pack 结构、license、动画、预览和导出字段。

### Phase 1：macOS App 本地闭环

目标：不用 AI，不用云端，先把本地视频转成游戏素材。

- 导入 MP4。
- 抽帧。
- chroma key 去背景。
- bbox 裁切。
- 统一 canvas。
- 锚点对齐。
- 生成 frames、sprite sheet、atlas、manifest、preview.gif。
- 生成 `.gsfpack`。

验收：

- 输入一个 1-2 秒 walk 视频。
- 输出 8-12 帧透明 PNG。
- 输出统一尺寸 sprite sheet。
- 输出 manifest/atlas。
- App 内能预览循环动画。

### Phase 2：App 内创作者生态

目标：让资源可以被创建、安装、分享。

- 创建资源包流程。
- 导入和安装资源包。
- 编辑 metadata。
- 选择 license。
- 生成预览。
- 运行质量校验。
- 导出可分享 `.gsfpack`。
- Publish Draft 到官网提交入口。

验收：

- 创作者能在 App 中制作一个完整资源包。
- 用户能导入别人分享的资源包并导出通用素材。

### Phase 3：引擎 Exporter

目标：覆盖主流 2D 引擎。

- Generic exporter。
- Godot import helper metadata。
- Phaser/PixiJS atlas。
- Unity 导入说明。

验收：

- 同一个 `.gsfpack` 能导出到至少 2 类目标：generic、Godot helper。

### Phase 4：本地资源包库

目标：让 App 内部可以管理本地 `.gsfpack`，但不接入线上 registry。

- 本地 pack install。
- 本地 pack inspect。
- 本地 pack validate。
- 最近导出记录。
- 示例 pack 手动导入。

验收：

- 用户能导入自己导出的 `.gsfpack`。
- 用户能在 App 内重新预览、校验和导出这个 pack。

### Phase 5：轻网站与免费 Registry

目标：在本地 App 可用后，让下载、文档、示例资源和早期生态可见。

- App 下载页。
- 文档。
- 示例 pack 下载。
- 免费 pack registry metadata。
- 创作者 waitlist。

验收：

- 官网可以展示至少 3 个示例 pack。
- App 可以手动导入从网站下载的 pack。

### Phase 6：BYOK AI 生成与 Source Provider 扩展

目标：把 AI 生成接入已有后处理链路，同时保持 provider 可替换。

- 图像 provider adapter。
- 视频 provider adapter。
- 连续帧 provider adapter，experimental。
- sprite sheet provider adapter，experimental。
- pose template / action recipe adapter。
- 成本预估。
- 用户确认。
- 本地缓存。
- 错误报告。
- 生成结果进入同一套处理流水线。

验收：

- 用户能使用自己的 key 生成角色图、动作视频或连续帧候选。
- App 能显示费用确认。
- 生成失败时有明确可操作建议。
- 纯图像 sheet provider 的结果会显示 `Prototype / Needs quality check`，不能绕过质量门直接标记为 game-ready。

### Phase 7：CLI / MCP / Codex Skill 自动化

目标：让 Codex 和其他 agent 使用本机已稳定能力。

- `gsf` CLI。
- `gsf-mcp`。
- 本地 pack 查询。
- 资源导出。
- 项目写入前确认。
- 生成任务前成本确认。

验收：

- Codex 能列出本机已安装 pack。
- Codex 能把资源导出到指定游戏项目目录。
- Codex 能调用本地处理工具生成 quality report。

### Phase 8：付费 Marketplace

目标：在资源包标准和免费生态稳定后，再做商业化。

- 用户账号。
- 创作者账号。
- 资源上传。
- 审核流程。
- 授权同步。
- 价格和结算。
- 退款和投诉。
- 版本更新。

验收：

- 创作者可以发布付费资源。
- 用户购买后能在 App 中安装已授权 pack。
- App/MCP 只能使用已授权资源。

## 12. MVP 范围

MVP 应聚焦“小而硬”的闭环。

包含：

- macOS 官网直发 App。
- 本地 MP4 导入。
- 本地 MOV / WebM 导入。
- PNG 序列帧导入。
- 已有 sprite sheet 导入。
- 抽帧。
- 纯色背景去除。
- bbox 裁切。
- 统一尺寸。
- 手动或半自动锚点调整。
- sprite sheet。
- frame sequence。
- preview.gif。
- manifest.json。
- atlas.json。
- `.gsfpack` 创建、导入、校验。
- Generic export。
- Godot helper metadata 或导入说明。

不包含：

- 图像模型调用。
- 视频模型调用。
- BYOK provider 设置。
- 轻网站下载页和示例 pack。
- 完整云端 SaaS。
- 完整 marketplace。
- 复杂 AI provider 编排。
- Hosted credits。
- Mac App Store 首发。
- iOS app。
- 完整 MCP。
- 深度 Unity/Godot 编辑器插件。

## 13. 主要风险

### 13.1 Source Provider 质量和模型演进

风险：当前视频模型可能出现角色漂移、镜头变化、肢体变形和衣服细节变化；未来图像模型可能直接生成连续帧或完整 sprite sheet，使视频模型路线不再是最佳路径。

对策：

- 第一版不依赖 AI。
- 后处理质量门先做好。
- AI 生成只作为可替换 source provider 输入。
- UI 和文档使用 `Source` / `Generate`，避免把产品命名为 `Video` 工具。
- `providers/` 支持 video、image sequence、sprite sheet、recipe 等多源输入。
- 失败时给出 prompt、provider 切换和重生成建议。
- 把 `.gsfpack`、quality report、manifest、exporter 做成模型无关资产层。

### 13.1.1 纯图像生成动作不可靠

风险：纯图像模型能生成漂亮的多帧角色图，但很难稳定遵守 walk cycle 的 contact / passing / recovery 节奏、重心变化、脚触地、cell gutter 和 bbox 约束。它可能是好看的角色 sheet，但不是可直接进游戏的动画。

对策：

- `text_or_reference_to_sprite_sheet` 默认标记为 experimental。
- UI 使用 `Generate Candidates`，不使用 `Generate Final Animation`。
- 生成结果必须经过 bbox、foot anchor、loop match、frame diff、cell boundary 检查。
- Quality Report 可以给出 `Prototype usable`、`Needs cleanup`、`Game-ready` 三类结论。
- 优先研发 pose-guided generation、动作模板和 creator recipe。
- 保留视频、导入帧序列、导入 sprite sheet 和 rig/template 输出作为一等 source。

### 13.2 本地处理质量

风险：去背景毛边、bbox 抖动、脚底线不稳、循环不自然。

对策：

- 先支持纯色背景/chroma key。
- 允许手动调整帧范围和锚点。
- 生成 quality report。
- 用 golden sample 做回归测试。

### 13.3 生态内容版权

风险：用户上传侵权素材、盗版资源包、AI 生成风格争议。

对策：

- pack metadata 必须包含 creator 和 license。
- 官网 registry 做审核。
- 付费 marketplace 前先做免费精选资源。
- 提供投诉和下架机制。

### 13.4 官网直发信任成本

风险：用户担心安装安全。

对策：

- Developer ID 签名。
- notarization。
- 明确下载说明。
- 自动更新透明。
- 公布隐私和本地处理策略。

### 13.5 过早做 marketplace

风险：支付、分账、审核和税务拖慢核心产品。

对策：

- 第一版只做资源包格式和免费分享。
- 先验证创作与安装链路。
- 付费系统放到 Phase 7。

## 14. 下一步建议

最优先的三件事：

1. 写 `.gsfpack` schema v0.1。
2. 做 macOS App 原型：导入 MP4 -> 抽帧 -> 预览 -> 导出 sprite sheet。
3. 做质量报告和 `.gsfpack` re-import：确认导出的包能被 App 自己重新打开。

第一个可执行里程碑：

```text
在 macOS App 中导入一个本地 walk.mp4，
生成 frames/*.png、sprite_sheet.png、preview.gif、manifest.json、atlas.json，
并打包成一个可重新导入的 .gsfpack。
```

只要这个闭环跑通，后续轻网站、AI provider、MCP 和付费 marketplace 都能围绕它逐步生长。

## 15. 可执行实施计划

2026-06-04 已将个人开发者 MVP 收缩为当前执行计划：

```text
/Users/kartz/Development/Forge/docs/superpowers/plans/2026-06-04-forge-solo-local-app-mvp.md
```

当前执行计划把第一版限定为：

1. macOS 本地系统应用。
2. 导入视频、PNG 序列、sprite sheet、`.gsfpack`。
3. ffmpeg 抽帧。
4. chroma key 单帧预览和批处理。
5. bbox、脚底锚点、loop range 和 quality report。
6. 导出 frames、sprite sheet、preview.gif、manifest、atlas 和 `.gsfpack`。
7. 重新导入 `.gsfpack` 校验。
8. 不做图像模型调用、视频模型调用、轻网站、MCP 和 marketplace。

较完整的长期路线图保留在：

```text
/Users/kartz/Development/Forge/docs/superpowers/plans/2026-06-04-forge-local-postprocess-and-ecosystem.md
```

长期路线图把实施拆成 14 个任务：

1. 定义 `.gsfpack`、manifest、quality report、provider-run schema。
2. 建立 core job model。
3. 实现 ffmpeg/ffprobe 导入和抽帧。
4. 实现 chroma/luma matting 和单帧预览。
5. 实现 bbox、脚底锚点、统一 canvas 和质量指标。
6. 实现 loop candidate 选择。
7. 生成 sheet、GIF、manifest 和 `.gsfpack`。
8. 增加 `gsf` CLI。
9. 实现 macOS App 工作台 MVP。
10. 增加 creator pack loop。
11. 增加轻网站 registry。
12. 增加 BYOK provider contract。
13. 在 CLI 稳定后增加 MCP/Codex Skill。
14. 建立 golden samples 和质量回归测试。

执行优先级应以 `Solo Local App MVP / Gate 1: Import and Preview` 为准：本地视频输入，能查看 metadata、截取单帧、预览 chroma key，并继续走到 `.gsfpack` 导出与 re-import。

## 2026-06-06 Current-State Correction

```text
The notarized/stapled local MVP release candidate exists and is package-ready.
The next active development slice is Local Workbench UX Hardening: improve visible feedback for Local Pack Library, first-run sample actions, export validation, and installed-app real UI evidence.
CLI/MCP remains a deferred automation option, not the next product development priority.
```
