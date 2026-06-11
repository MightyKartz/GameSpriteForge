# game-sprite-forge 项目方向

日期：2026-05-31

## 1. 项目一句话

`game-sprite-forge` 是一个面向 Codex 和 2D 游戏开发者的游戏动画资产锻造工具链：它接收任意生成源或导入源，包括文本、角色参考图、AI 视频、AI 生成的连续帧、已有 sprite sheet、手工视频和创作者 recipe，再通过本地后处理、质量检查、打包和引擎导出，把这些输入变成游戏引擎可用的 sprite sheet、单帧 PNG、GIF/WebM 预览、manifest、atlas 和 `.gsfpack`。

核心目标不是押注某一种模型路线，也不是做一个孤立的网页生成器，而是做一个可以被 Codex 调用、可以落地到主流 2D 游戏项目、可以长期演进为 MCP/CLI/插件的开放资产生产工具链。

当前个人开发者执行版应先收缩为：

```text
本地导入源 -> Forge Pipeline -> 游戏可用资产
```

第一版不调用图像模型或视频模型，也不做轻网站。它先把用户导入的视频、PNG 序列、已有 sprite sheet 或 `.gsfpack`，通过本地后处理、质量检查和导出，变成可复用的游戏动画资源。

## 2. 背景与机会

Codex 已经可以辅助游戏开发脚本、场景、资源导入和工具链脚本，但 2D 游戏角色动画仍然是瓶颈。传统方式需要动画师逐帧绘制或绑定骨骼；AI 图像模型可以快速产出角色外观；AI 视频模型当前更擅长时间连续性；未来图像模型也可能直接生成连续动画帧或完整 sprite sheet。

因此项目不能绑定在“图像模型不够强，所以必须用视频模型”这个短期假设上。更合理的长期链路是：

1. 允许用户从多种 `Source Provider` 获得原始动画素材。
2. 用确定性的 Forge Pipeline 做切帧、去背景、对齐、锚点、循环检测、质量报告和打包。
3. 输出游戏引擎、创作者生态和 Codex 自动化都能复用的资产格式。

当前的“角色参考图 -> 视频动作母片 -> 抽帧”只是第一批 source provider 组合。未来如果图像模型能直接生成连续动画图或 sprite sheet，它应该替换 source provider，而不是替换整个项目。项目的价值在于把任意 AI/非 AI 输出锻造成可工程化复用的游戏资源。

## 3. 相关项目与参照

已有相邻项目说明方向成立，但端到端开发链仍有空间：

- Agent Sprite Forge：开源 Codex Skill，做 prompt 到 sprite sheet/GIF 的本地生成和清理流程。  
  https://github.com/0x0funky/agent-sprite-forge
- Sorceress Auto-Sprite：采用 AI 角色图、AI 视频、sprite sheet 的链路，并面向 Godot/Unity 导出。  
  https://sorceress.games/pages/auto-sprite
- EZ Animate：提供 reference/text 到 AI frames、透明 sprite sheet 的网页工具。  
  https://www.ezanimate.com/
- Sprite Smithy / Frame Extractor：偏视频转帧、去背景、sprite sheet 和 JSON manifest。
- Sprite Video Lab：开源本地网页工具，覆盖导入视频/图片/序列帧、区间截取、抽帧、chroma/BiRefNet/Luma/CorridorKey 抠图、统一画布、动画预览、sprite sheet、zip 和 manifest 导出。  
  https://github.com/sparklecatta-lang/sprite-video-lab
- Godot 官方支持 `AnimatedSprite2D`、`SpriteFrames` 和 sprite sheet 动画导入。  
  https://docs.godotengine.org/en/stable/tutorials/2d/2d_sprite_animation.html

`game-sprite-forge` 的差异点应该放在 Codex/Godot 工程集成、可复现的本地后处理、开放 CLI/MCP 工具边界，而不是只做一个素材生成网页。

### 3.1 Sprite Video Lab 的借鉴边界

2026-06-04 对 `sparklecatta-lang/sprite-video-lab` 的调研结论：它对 Forge 有很强的工程借鉴意义，但不是 Forge 的产品终点。

可吸收部分：

- 本地 job 工作流：每次导入、处理、导出都形成可追踪任务目录。
- 四步体验：导入素材、预览与截取区间、抽帧与抠图、检查帧并导出。
- 单帧参数预览：先看一帧的抠图、despill、halo、画布效果，再跑整段。
- 多抠图模式概念：chroma、AI matting、luma、边缘精修可以作为 Forge 的 matting provider。
- 输出结构：frames、sprite sheet、preview、manifest、zip 是 Forge `.gsfpack` 的基础组成。

不能照搬的部分：

- 它没有 Source Provider 生成链路，不能覆盖“角色参考图 -> 动作源 -> 游戏资产”的创建目标。
- 它没有面向游戏动画资产的质量门，如脚底锚点、循环匹配、中心漂移、cell boundary safety 和 game-ready verdict。
- 它没有 `.gsfpack`、创作者 recipe、license、资源包安装、发布草稿和生态分发。
- 它不是 macOS 原生工作台，也没有 CLI/MCP/引擎导出作为长期自动化边界。

因此 Forge 应把它视为“后处理实验室参考实现”，吸收其本地处理与交互经验，同时把差异化继续放在生成源、质量门、资源包格式、引擎导出和创作者生态。

## 4. 产品定位

### 4.1 面向用户

- 独立游戏开发者。
- 使用 Godot 制作 2D 游戏的开发者。
- 使用 Codex 开发游戏项目、希望自动生成占位或可用角色动画的人。
- 想把 AI 视频、AI 连续帧、sprite sheet 或手工视频转换成可用游戏动画资产的技术美术或工具工程师。

### 4.2 核心使用场景

- 从视频、连续帧或已有 sprite sheet 中自动抽取稳定动画帧。
- 生成 Godot、Unity、Phaser、Cocos、GameMaker 或通用 pipeline 可使用的 sprite sheet、atlas 和动画 manifest。
- 创建、导入、校验和重新导出 `.gsfpack`。
- 后续阶段从一句角色描述生成角色参考图、连续帧、动作视频或 sprite sheet。
- 后续阶段从角色参考图生成 idle、walk、run、attack、hurt、death 等动作源。
- 在 Codex 开发游戏项目时，一步生成、校验并导入角色动画资源。

### 4.3 非目标

- 第一版不做完整商业 SaaS。
- 第一版不做图像模型或视频模型调用。
- 第一版不做轻网站和线上 registry。
- 第一版不做 BYOK provider 设置。
- 第一版不做 MCP server 或 Codex Skill。
- 第一版不做多人角色互动动画。
- 第一版不做复杂场景镜头动画。
- 第一版不承诺生成专业像素美术最终成品。
- 第一版不依赖 Flow 私有 workspace 状态链或内部业务模型。

## 5. 推荐技术路线

推荐路线应分成两层：可替换的 Source Provider 和稳定的 Forge Pipeline。

```text
Source Provider
  -> text-to-reference-image
  -> reference-image-to-motion-video
  -> text/reference-to-image-sequence
  -> text/reference-to-sprite-sheet
  -> imported video / frames / sprite sheet
  -> creator recipe

Forge Pipeline
  -> 本地持久化 source artifact
  -> ffmpeg 抽帧
  -> 背景移除或绿幕 key
  -> bounding box 统一
  -> 脚底/中心点锚定
  -> 帧筛选和循环段检测
  -> sprite sheet / frames / GIF / manifest
  -> .gsfpack / atlas / quality report
  -> Godot / Unity / Phaser / Cocos / GameMaker / generic 导出
```

### 5.1 Source Provider 抽象

Source Provider 是“素材从哪里来”的抽象，不是产品核心边界。它可以是 AI，也可以是用户导入，也可以是创作者 recipe。

个人开发者 MVP 启用 provider：

- `import_video`：用户导入 MP4/MOV/WebM。
- `import_frames`：用户导入 PNG 序列。
- `import_sprite_sheet`：用户导入已有 sheet。
- `import_gsfpack`：用户导入 Forge 自己导出的资源包。

未来可新增 provider：

- `image_reference_provider`：生成或导入角色参考图。
- `motion_video_provider`：从参考图生成短动作视频。
- `image_sequence_provider`：图像模型一次生成连续动画帧。短期应标记为 experimental / prototype provider。
- `sprite_sheet_provider`：图像模型直接生成完整 sprite sheet。短期应标记为 experimental / prototype provider。
- `pose_guided_provider`：用 walk/run/attack 姿势模板约束图像或视频生成。
- `rigged_animation_provider`：骨骼动画或 2D rig 输出 frames。
- `recipe_provider`：从 `.gsfpack` 或 creator recipe 复用生成参数。
- `community_provider`：创作者 marketplace 提供的可复用生成链。

只要 provider 输出能进入 Forge Pipeline，底层模型路线变化就不会推翻项目。

### 5.2 图像模型角色参考图

图像模型负责生成角色 identity，而不是每一帧动画。

建议输出：

- 全身角色图。
- 单角色。
- 干净背景或透明背景。
- 明确视角：side-view、front-view、top-down、3/4 view。
- 固定服装、比例和颜色。
- 尽量避免复杂道具和漂浮特效。

两轮本地测试表明，纯图像模型直接生成横向 walk cycle sprite sheet 时，即使角色外观和绿幕都不错，也容易出现动作规律不正确、帧间节奏不稳定、bbox 宽度变化大、披风/武器触碰 cell 边界等问题。它适合生成候选素材、角色 identity、key pose 或 prototype sheet，不应在第一版承诺为 production-ready walk cycle。

因此图像生成应分级：

```text
Level 1: Quick Prototype
  prompt/reference -> candidate sprite sheet
  用于占位、灵感和 pipeline 测试

Level 2: Pose-guided Generation
  pose template / action recipe -> constrained frames
  用于更稳定的 walk/run/attack 结构

Level 3: Video-to-Frames
  reference image -> short motion video -> frames
  当前更适合连续动作候选

Level 4: Rig / Template Recipe
  skeleton / action template / creator recipe -> frames
  长期更接近可复用生产能力
```

产品文案和 UI 不应写成 `Generate game-ready walk cycle`，而应写成 `Generate Candidates`、`Quality Check`、`Send to Frames`、`Fix / Regenerate / Export`。

### 5.3 视频模型动作母片

视频模型当前更适合用于生成动作连续性。以 Seedance 等 provider 为例，它们支持参考图、首帧图生视频、指定时长和多模态输入，因此理论上适合做角色动作母片。

但游戏动画不应默认生成 10 秒长视频。推荐时长：

- idle：2-3 秒。
- walk：1-2 秒循环。
- run：1-2 秒循环。
- attack：0.8-1.5 秒。
- hurt：0.5-1 秒。
- death：2-4 秒。

视频模型的输出不是最终资产，必须经过后处理质量门。未来如果图像模型能直接生成连续帧或 sprite sheet，这一节可以变成可选 provider，而不是项目主路径。

### 5.4 游戏化 Prompt 约束

视频 prompt 应该像动画生产说明，而不是影视分镜说明。

示例：

```text
single 2D game character, full body visible, locked side-view orthographic camera,
no camera movement, no zoom, no cuts, solid green background,
character stays centered, feet remain on the same ground line,
clean readable silhouette, looping walk cycle, 2 seconds
```

关键约束：

- locked camera。
- no zoom。
- no cuts。
- single character。
- full body visible。
- solid color background。
- feet remain on same ground line。
- readable silhouette。

## 6. 输出格式

第一版至少输出：

```text
output/
  character.json
  preview.gif
  sprite_sheet.png
  manifest.json
  frames/
    idle_000.png
    idle_001.png
    ...
```

### 6.1 manifest.json 建议结构

```json
{
  "name": "hero_knight",
  "source": {
    "characterImage": "character.png",
    "motionVideo": "walk.mp4",
    "provider": "byok"
  },
  "sheet": {
    "image": "sprite_sheet.png",
    "frameWidth": 256,
    "frameHeight": 256,
    "columns": 4,
    "rows": 3
  },
  "animations": {
    "walk": {
      "fps": 12,
      "loop": true,
      "frames": [0, 1, 2, 3, 4, 5, 6, 7]
    }
  },
  "anchor": {
    "type": "feet",
    "x": 128,
    "y": 232
  }
}
```

### 6.2 Godot 导出

第一版可先导出 Godot 可读资源描述，后续再自动生成：

- `.tres` SpriteFrames。
- `.tscn` 角色场景。
- `AnimatedSprite2D` 节点。
- 简单状态机脚本。

## 7. 开源策略

可以开源，但应开源独立工具链，不应开源 Flow 本体。

### 7.1 建议开源内容

- CLI。
- Codex Skill。
- 本地抽帧脚本。
- 去背景/绿幕 key。
- bbox 裁切和对齐。
- sprite sheet 排版。
- GIF 预览生成。
- manifest 生成。
- Godot exporter。
- MCP server 的公开工具接口。
- 示例素材和 mock provider。

### 7.2 不应开源内容

- Flow workspace 状态链。
- Flow 的角色、场景、分镜 canonical truth。
- Flow 的私有 prompt 编排。
- Flow 的 provider routing 和 fallback 策略。
- Flow 内部 API key、请求日志、真实用户素材。
- 私有测试夹具和生产数据。

### 7.3 仓库边界

推荐新建独立仓库：

```text
game-sprite-forge/
  packages/
    core/
    cli/
    godot-exporter/
    codex-skill/
    mcp-server/
    providers/
  examples/
  docs/
```

Flow 只作为私有上游集成方：

```text
Flow -> 调用 game-sprite-forge CLI/MCP
game-sprite-forge -> 不反向依赖 Flow
```

这样可以保护 Flow 的核心技术，同时让 `game-sprite-forge` 成为可开源、可复用、可被社区贡献的项目。

## 8. 使用成本与计费建议

生成类 provider 费用和能力变化都很快，因此个人开发者 MVP 不接入生成 provider。未来进入生成阶段时，应采用 BYOK，并把成本控制设计在 Source Provider 层，而不是写死某个模型。

BYOK：Bring Your Own Key。用户自己配置模型 API key：

```bash
OPENAI_API_KEY=...
ARK_API_KEY=...
```

工具默认不使用项目维护者的账号代付模型费用。

### 8.1 三种运行模式

1. Local mode  
   用户提供现成角色图和视频，只做本地抽帧、对齐和导出。适合免费开源。

2. BYOK generation mode  
   用户提供自己的 OpenAI / Ark / Seedance 或其他 provider key。工具负责调用、缓存、溯源和后处理。

3. Hosted credits mode  
   未来如需商业化，可提供托管服务、额度、限流和计费。不要在开源 skill 里默认使用维护者 API key。

### 8.2 Cost preflight

每次触发模型调用前必须显示成本预估：

```text
即将生成：
- 1 张角色参考图
- 1 个 2 秒动作视频或 1 组连续动画帧
- 抽取 12 帧

预计消耗：
- image generation: ...
- video generation: ...

是否继续？
```

Codex Skill 和 MCP 工具都应默认 require confirmation，除非用户显式传入 `--yes` 或配置自动化预算。

## 9. MVP 范围

第一版只做最短闭环：

- 导入一个本地 MP4/MOV/WebM 视频，或导入 PNG 序列、已有 sprite sheet、`.gsfpack`。
- 支持用户手动命名动作，例如 idle、walk、attack。
- 支持 side-view 横版角色。
- 支持纯色背景视频。
- 支持抽帧、去背景、裁切、统一尺寸、脚底锚点对齐。
- 输出 `sprite_sheet.png`、`frames/*.png`、`preview.gif`、`manifest.json`。
- 提供 Godot 导入说明或基础 exporter。
- 支持 `.gsfpack` 创建、导入、校验和重新导出。

第一版不做：

- 图像模型调用。
- 视频模型调用。
- BYOK provider 设置。
- 轻网站。
- 八方向动画。
- 多角色互动。
- 骨骼绑定。
- 完整 Godot 编辑器插件。
- 商业托管计费平台。
- Flow 私有工作台集成。

## 10. 质量门

生成 sprite sheet 前应自动检查：

- 是否只有一个主要角色。
- 是否全身可见。
- 背景是否可移除。
- 每帧 bbox 是否稳定。
- 角色中心点是否漂移过大。
- 脚底锚点是否稳定。
- 动作是否适合循环。
- 帧间差异是否过大或过小。
- 输出帧尺寸是否一致。

质量失败时，不应静默输出劣质 sheet，应返回可操作建议：

- 缩短视频时长。
- 换成 locked side-view prompt。
- 换成 pose-guided provider 或 creator recipe。
- 使用纯色背景。
- 重新生成角色参考图。
- 降低动作幅度。
- 手动选择循环区间。
- 拒绝把 prototype candidate 标记为 game-ready。

## 11. Codex Skill 形态

Skill 应提供清晰的工作流：

```text
用户：为我的 Godot 横版游戏生成一个骑士 walk 动画。

Codex Skill：
1. 检查是否已有角色参考图。
2. 如无参考图，生成或要求用户提供。
3. 生成 Seedance 动作视频前做成本确认。
4. 下载视频。
5. 抽帧和后处理。
6. 生成 sprite sheet 和 manifest。
7. 可选：导入 Godot 项目并创建 AnimatedSprite2D。
```

Skill 第一版可以只调用本地 CLI。MCP server 等接口稳定后再补。

## 12. MCP 工具方向

后续 MCP server 可以暴露这些工具：

- `generate_character_reference`
- `generate_motion_video`
- `extract_frames`
- `remove_background`
- `normalize_sprite_frames`
- `build_sprite_sheet`
- `export_godot_spriteframes`
- `estimate_generation_cost`

MCP 工具必须明确区分：

- 本地免费操作。
- 会产生模型费用的操作。
- 会写入游戏项目文件的操作。

## 13. Godot 集成方向

第一阶段：导出文件，用户手动导入。

第二阶段：生成 `.tres` / `.tscn`。

第三阶段：Codex 直接修改 Godot 项目：

- 把 sheet 放入 `res://assets/characters/<name>/`。
- 创建 `SpriteFrames`。
- 创建 `AnimatedSprite2D`。
- 写入基础角色控制脚本。
- 在测试场景中实例化角色。

## 14. 风险与对策

### 14.1 角色漂移

对策：

- 使用单张强约束角色参考图。
- 缩短动作视频。
- 使用 locked camera。
- 后处理时检测 bbox 和颜色漂移。

### 14.2 背景移除不稳定

对策：

- 默认使用纯绿/纯蓝背景。
- 提供 chroma key。
- 复杂背景只作为高级模式。

### 14.3 动作不适合循环

对策：

- 默认生成短循环动作。
- 自动寻找首尾相似帧。
- 允许用户手动指定 frame range。
- 对纯图像 sheet provider 增加 `prototype` 标签，必须经过 Quality Gate 后才能导出为 game-ready pack。
- 优先研发 pose template / action recipe，而不是单纯堆 prompt。

### 14.4 成本失控

对策：

- BYOK。
- cost preflight。
- 默认 local mode。
- 默认短视频或短序列。
- 加缓存和复用。

### 14.5 模型路线被替代

风险：未来图像模型可能直接生成连续动画帧或完整 sprite sheet，使“视频模型生成动作母片”不再是最佳路线。

对策：

- 产品定位为 `source -> forge pipeline -> game-ready asset`，而不是 `video -> sprite sheet`。
- 把图像序列、视频、sprite sheet、手工素材和 creator recipe 都视为 source provider。
- `manifest` 和 `.gsfpack` 中记录 source/provider/cost/recipe，方便未来迁移。
- UI 第一入口命名为 `Source` 或 `Generate`，而不是 `Video`。
- 质量检查、锚点、packing、engine exporters 和生态分发保持模型无关。

### 14.6 纯图像生成动作不可靠

风险：图像模型能画出好看的多帧角色图，但不一定理解真实 walk cycle 的接触、重心、passing pose、回环和帧间节奏。生成结果可能看起来漂亮，却不能直接作为游戏动画。

对策：

- 纯图像 sheet generation 默认标记为 `Quick Prototype`。
- 生成按钮命名为 `Generate Candidates`，而不是 `Generate Final Animation`。
- 必须经过 bbox、脚底线、loop、frame diff 和 cell-boundary 检查。
- 提供 `Fix / Regenerate / Send to Frames`，而不是一键导出。
- 中期优先做 pose-guided generation 和 creator action recipe。
- 视频模型、导入视频、导入帧序列和 rig/template 输出继续作为一等 source provider。

### 14.7 开源暴露 Flow 核心

对策：

- 新建独立项目。
- 不复制 Flow 私有代码。
- 只公开通用后处理和接口。
- Flow 通过 adapter 调用公开 CLI/MCP。

## 15. 推荐开发路线

### Phase 0：独立项目骨架

- 新建 `game-sprite-forge` 仓库。
- 建立 macOS App 工程和 core package。
- 支持输入本地 MP4，输出 frames、sheet、manifest、GIF 和 `.gsfpack`。

### Phase 1：本地后处理闭环

- ffmpeg 抽帧。
- 纯色背景去除。
- bbox 裁切。
- 统一 canvas。
- 锚点对齐。
- sprite sheet 排版。
- `.gsfpack` 创建、导入、校验。

### Phase 2：macOS App 工作台

- Import / Frames / Background / Anchor / Sheet / Export workflow。
- 单帧 chroma 预览。
- 时间轴帧选择。
- 右侧 Quality Report。
- Settings 配置 ffmpeg path 和默认输出目录。

### Phase 3：引擎导出

- Generic PNG + JSON。
- Godot helper metadata 或导入说明。
- Phaser/PixiJS atlas。

### Phase 4：本地资源包库

- 本地安装 `.gsfpack`。
- 检查 metadata / license / quality report。
- 重新导出。

### Phase 5：轻网站

- 下载页。
- 文档。
- 示例 pack。
- 创作者 waitlist。

### Phase 6：BYOK 模型调用

- 接入图像生成 provider。
- 接入视频 provider。
- 加成本确认。
- 加缓存。

### Phase 7：Codex Skill / MCP server

- 编写 `SKILL.md`。
- 封装本地 CLI/MCP。
- 支持从游戏项目上下文推断输出路径。
- 暴露稳定工具接口。
- 加权限和成本 guard。

## 16. 最小验收标准

MVP 完成的定义：

- 导入一个 1-2 秒动作视频和一个动作名 `walk`。
- 自动抽取 8-12 帧。
- 输出透明背景或纯色背景已移除的 PNG 帧。
- 生成统一尺寸 sprite sheet。
- 生成 `manifest.json`。
- 生成 `preview.gif`。
- 生成并重新导入 `.gsfpack`。
- 在 Godot 中能用 `AnimatedSprite2D` 正常播放。

## 17. 当前推荐结论

`game-sprite-forge` 应该作为独立开源项目启动。它可以吸收 Flow 的经验，但不依赖 Flow 的内部实现。

最健康的边界是：

- Flow 是高级生产工作台。
- `game-sprite-forge` 是开放的游戏动画资产锻造工具。
- macOS 本地 App 是第一阶段入口。
- CLI、Codex Skill、MCP/Plugin 是本地 App 和 pack 格式稳定后的自动化入口。
- 未来模型调用默认 BYOK，避免维护者承担视频模型成本。

第一版只追求一个小而硬的闭环：单角色、横版 side-view、本地导入视频或帧序列、sprite sheet、quality report、`.gsfpack`、Godot 可用。AI 生成、轻网站和 MCP 都在这个闭环稳定之后再做。
