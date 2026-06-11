# GAME-SPRITE-FORGE UI Design Spec

日期：2026-05-31

## 1. 文档目的

这份文档把当前已经导入 Figma、并在本地 HTML 中 1:1 还原的 `GAME-SPRITE-FORGE` UI，整理成后续 macOS App 开发可以直接使用的设计规范。

它不是单纯的视觉说明，而是面向开发的 UI 合同：

- 明确 App 的信息架构和页面范围。
- 把现有 Forge Editor 高保真界面拆成可实现组件。
- 把“生成”保留为长期一等入口，但个人开发者 MVP 先隐藏生成能力，优先实现导入型本地工作台。
- 定义设计 token、布局尺寸、状态、交互和质量标准。
- 说明 Figma、HTML 原型和真实 macOS App 之间的职责边界。
- 为 Tauri + React 或 SwiftUI 实现提供组件映射。

### 1.1 2026-06-05 实现状态更新

当前生产实现已经转向“真实本地功能优先”，不再以 Figma 截图 1:1 还原为第一目标。HTML/Figma 仍是视觉气质和布局密度参考，但生产 UI 必须服从以下更高优先级：

- 所有可见主按钮必须对应真实本地动作，或明确说明不可用原因。
- 不显示 app 内容区内的假 macOS 红黄绿窗口按钮；窗口控制由系统窗口负责。
- 不显示 MVP 未实现的 Marketplace、Creator Publish、BYOK、provider API、cloud cost、online registry、Generate 等入口。
- Demo/sample 数据必须以 `Demo`、`Sample preview - not an active workspace`、`Sample timeline` 等方式清楚标识。
- 导出区必须显示 export readiness blockers，而不是只给一个灰色按钮。
- 进入真实工作区后，左侧显示 `Run Summary`：Source、Frames、Quality、Export、Validate。
- 失败状态必须显示恢复卡片，例如 Open Settings、Select source、Extract frames、Tune background、Process again、Choose output。

当前已实现的主界面路由是：

```text
Forge
Exports
Settings
```

`Library`、`Packs`、`Marketplace`、`Creator Publish` 仍属于后续扩展，不应在当前 MVP UI 中以可点击主导航出现。

## 2. 当前设计资产

### 2.1 Figma 文件

```text
https://www.figma.com/design/dmeE2JAZ62Fz6TrvZzWyyu/Magpic?node-id=94-597&p=f&t=yb0PtaHM6c57kkVK-0
```

用途：

- 设计评审。
- 页面拆解。
- 组件命名。
- 视觉标注。
- 与设计师、合作者协作。

注意：Figma 由 `html.to.design` 导入，图层可能较碎，不建议直接把 Figma 层级当作生产代码结构。

### 2.2 本地 HTML 原型

```text
/Users/kartz/Development/Forge/figma-import/game-sprite-forge-app.html
```

本地预览：

```text
http://127.0.0.1:8765/game-sprite-forge-app.html
```

用途：

- 高保真视觉基准。
- CSS token 和布局参考。
- 后续 Tauri + React UI 的实现基线。
- 对比截图和回归检查。

### 2.3 项目产品方案

```text
/Users/kartz/Development/Forge/GAME-SPRITE-FORGE-DEVELOPMENT-PLAN-2026-05-31.md
```

UI 设计必须服从该产品方向：

- macOS-first。
- 官网直发版优先。
- 个人开发者 MVP 先不做轻网站。
- 导出格式覆盖多数 2D 游戏引擎。
- Godot 是重点 exporter，但不是产品边界。
- `.gsfpack` 从第一版就是核心资产格式。
- 创作者生态保留为数据结构和未来页面，不进入第一版可见导航。

## 3. 产品 UI 定位

`GAME-SPRITE-FORGE` 的 UI 应该像一个专业游戏资产工作台，而不是普通 SaaS 后台，也不是营销型网页。

关键词：

- 工具型。
- 高密度。
- 低干扰。
- 文件和资产优先。
- 可视化检查优先。
- 导出确定性优先。
- 创作者生态内置。

界面气质：

- 类似游戏引擎、视频剪辑软件、像素编辑器和资产管理工具的结合。
- 中央永远服务于当前资产。
- 右侧永远解释当前资产是否“可用、可导出、可出售”。
- 底部时间轴承担动画生产判断。
- 左侧承担项目、资源包和生态导航。

## 4. 核心用户和任务

### 4.1 主要用户

1. 独立游戏开发者。
2. 需要把本地视频、序列帧或 sprite sheet 转成游戏资产的 2D 游戏创作者。
3. 素材包创作者。
4. 小型游戏团队技术美术。
5. 需要把视频或角色图转成游戏资产的开发者。

### 4.2 核心任务

- 导入角色图、本地视频、连续帧、手工视频或已有 sprite sheet。
- 抽帧并清理背景。
- 检查 bbox、alpha edge、脚底锚点和循环连续性。
- 编辑动画分段，例如 idle、walk、attack。
- 生成 sprite sheet、frames、atlas、manifest、preview。
- 导出 `.gsfpack` 和各引擎目标格式。
- 导入、校验、管理本地资源包。
- 后续通过 Source/Generate 从文本、参考图、AI 视频、连续帧、sprite sheet 或 creator recipe 创建动画源。
- 后续为 marketplace 准备 metadata、license 和 preview。

## 5. 应用信息架构

### 5.1 一级导航

当前 HTML 原型中的一级导航是长期产品参考，不代表当前 MVP 必须全部显示：

```text
Library
Forge
Packs
Exports
Settings
```

建议后续扩展：

```text
Marketplace
Creator
```

如果第一版不想增加导航复杂度，`Creator` 可以先作为 `Packs` 或 `Marketplace` 内的二级页面。

当前 2026-06-05 生产 MVP 导航应保持为：

```text
Forge
Exports
Settings
```

原因：

- `Forge` 承担导入、处理、质量检查、导出 readiness 和 Run Summary。
- `Exports` 只记录本机 recent exports。
- `Settings` 只管理 ffmpeg/ffprobe 路径和默认输出设置。
- `Library`、`Packs`、`Marketplace`、`Creator Publish` 尚未具备完整真实功能，暂不出现在主导航。

### 5.2 页面定义

| 页面 | 目的 | MVP 是否需要 |
| --- | --- | --- |
| Library | 本地项目和最近资产入口 | 需要 |
| Forge | 核心编辑和导出工作台 | 必须 |
| Packs | 本地 `.gsfpack` 管理 | 必须 |
| Marketplace | 早期生态资源展示和安装 | MVP 隐藏 |
| Exports | 最近导出、目标路径、导出历史 | 需要 |
| Settings | ffmpeg、路径、license | 必须 |
| Creator Publish | 创建资源包发布草稿 | 后置 |
| Onboarding | 首次启动、依赖检查、示例包导入 | 必须 |

### 5.3 推荐路由

如果使用 Tauri + React：

```text
/
/library
/forge/:packId
/packs
/packs/:packId
/exports
/settings
/onboarding
```

如果使用 SwiftUI：

```text
NavigationSplitView
  SidebarDestination.library
  SidebarDestination.forge(packId)
  SidebarDestination.packs
  SidebarDestination.marketplace
  SidebarDestination.exports
  SidebarDestination.settings
```

## 6. Forge Editor 页面结构

当前高保真图对应的是核心页面：`Forge Editor / Frames Step`。

### 6.1 页面骨架

```text
Mac Window
  Top Toolbar
    Project Selector
    Workflow Step Tabs
    Undo / Redo / Save / Quality Check

  Sidebar Navigation
    Library / Forge / Packs / Marketplace / Exports / Settings

  Pack List
    Current pack
    Installed packs
    Validation status

  Main Canvas
    Tool strip
    Checkerboard preview
    Selected sprite
    Bounding box
    Foot anchor line
    Frame scrubber

  Bottom Workspace
    Active Assets
    Timeline
    Sprite Sheet Preview

  Right Inspector
    Quality Report
    Pack Metadata
    Export Targets
    Export action

  Status Bar
    Frames / Size / FPS / Duration / Quality state / Version
```

### 6.2 视觉层级

第一视觉焦点：

```text
中央角色画布
```

第二视觉焦点：

```text
底部时间轴 + 右侧 Quality Report
```

第三视觉焦点：

```text
导出目标 + Export Pack
```

低优先级但常驻：

```text
左侧导航、pack list、status bar
```

## 7. 设计 Token

以下 token 来自本地 HTML 原型，后续应进入 `tokens.ts`、CSS variables 或 SwiftUI Theme。

### 7.1 颜色

#### Primitive Colors

```text
bg             #0B1014
bgDeep         #070B0E
panel          #11181D
panel2         #151D23
panel3         #1B252C
line           #25323A
text           #DCE5EA
muted          #8F9BA3
faint          #5F6E76
cyan           #16C6F4
blue           #2F90D8
green          #35C85A
amber          #F6AB22
pink           #E64796
violet         #9657D9
```

#### Semantic Colors

```text
surface.app             bg
surface.sidebar         #0F171B
surface.panel           panel
surface.panelRaised     panel2
border.default          line
border.subtle           rgba(139, 166, 180, 0.16)
text.primary            text
text.secondary          muted
text.tertiary           faint
accent.primary          cyan
accent.selection        cyan
status.success          green
status.warning          amber
status.error            #FF6258
status.info             blue
timeline.idle           pink
timeline.walk           blue
timeline.attack         violet
```

### 7.2 字体

推荐：

```text
macOS App: SF Pro / system font
Web fallback: -apple-system, BlinkMacSystemFont, "SF Pro Display", "Inter", "Segoe UI", sans-serif
```

字号层级：

```text
Window title            15px / 650
Project title           16px / 650
Toolbar label           10px-12px / 500
Panel title             11px / uppercase
Body                    12px-13px
Inspector row           12px
Primary button          16px / 600
Status bar              10px-12px
```

原则：

- 不使用 viewport-based font size。
- 不使用负 letter spacing。
- 面板内避免大标题。
- 工具型 UI 优先可扫描，而不是视觉冲击。

### 7.3 间距

```text
space.2      2px
space.4      4px
space.6      6px
space.8      8px
space.10     10px
space.12     12px
space.14     14px
space.16     16px
space.18     18px
space.22     22px
space.26     26px
```

布局规则：

- 工具栏高度：38px。
- 顶部标题栏高度：56px。
- 状态栏高度：43px。
- 右侧 inspector tab 高度：50px。
- 时间轴 track 高度：74px 左右。
- pack card 高度：57px。
- export target card 高度：82px。

### 7.4 圆角

```text
radius.xs      3px
radius.sm      5px
radius.md      6px
radius.lg      7px
radius.window  8px
```

规则：

- 卡片和按钮优先 6px-7px。
- 页面大框架最大 8px。
- 不使用过度圆润的 SaaS 风格大圆角。

### 7.5 边框和阴影

```text
border.default     1px solid #25323A
border.active      1px solid #16C6F4
shadow.window      0 18px 50px rgba(0, 0, 0, 0.42)
shadow.activeGlow  0 0 0 1px rgba(22, 198, 244, 0.12)
```

规则：

- 面板分隔主要依赖 1px 线。
- 阴影只用于窗口、浮层、选中对象，不做装饰性光晕。
- 高亮必须有功能含义，例如 selected、active、quality pass。

## 8. Layout Spec

### 8.1 原型尺寸

当前 HTML 原型：

```text
1568 x 1003
```

这是设计基准，不等于真实 App 最小窗口。

### 8.2 推荐窗口尺寸

```text
Recommended: 1568 x 1003
Minimum:     1280 x 820
Comfort:     1440 x 900
Large:       1728 x 1117
```

### 8.3 桌面布局比例

当前原型 grid：

```text
columns:
  Sidebar Nav      122px
  Pack List        182px
  Main Workspace   844px
  Inspector        420px

rows:
  Top Toolbar      56px
  Main Canvas      524px
  Bottom Timeline  380px
  Status Bar       43px
```

### 8.4 窗口缩小时的适配

当宽度小于 1440px：

- 左侧一级导航保持。
- Pack List 可折叠为图标/缩略图列。
- Right Inspector 宽度最低 360px。
- Sprite Sheet Preview 可以隐藏到 Inspector 的 Sheet tab。
- Timeline 保持可横向滚动。

当宽度小于 1280px：

- 进入 compact mode。
- Pack List 默认折叠。
- Inspector 变成右侧 drawer。
- Canvas + Timeline 优先保留。

## 9. 组件拆解

### 9.1 AppShell

职责：

- 管理 macOS window chrome 感知区域。
- 承载 top toolbar、sidebar、main content、inspector、status bar。
- 控制全局主题和布局模式。

Props 建议：

```ts
type AppShellProps = {
  activeSection: "library" | "forge" | "packs" | "exports" | "settings";
  compactMode: boolean;
  projectDirty: boolean;
};
```

### 9.2 TopToolbar

子组件：

- `ProjectSelector`
- `WorkflowTabs`
- `ToolbarActionGroup`
- `QualityCheckButton`
- `MoreMenu`

状态：

- clean / dirty。
- saving。
- quality checking。
- quality pass。
- quality warnings。
- disconnected helper。

### 9.3 SidebarNav

导航项：

```text
Library
Forge
Packs
Exports
Settings
```

状态：

- active。
- disabled。
- badge，例如 marketplace updates、export warnings。

实现注意：

- 文字过长时不可撑开布局。
- 图标和文字 baseline 统一。
- active state 需要左侧 cyan indicator。

### 9.4 PackListPanel

职责：

- 展示当前项目可用的 packs。
- 展示安装状态、版本和质量状态。
- 支持导入、创建、搜索、筛选。

组件：

- `PackListHeader`
- `PackCard`
- `PackStatusBadge`
- `PackThumbnail`

PackCard 状态：

```text
selected
valid
warning
invalid
missingAssets
updateAvailable
unlicensed
syncing
```

### 9.5 WorkflowTabs

当前流程：

```text
Import → Frames → Background → Anchor → Sheet → Export
```

每一步含义：

| Step | 目的 |
| --- | --- |
| Import | 导入本地视频、序列帧、sprite sheet 或 `.gsfpack` |
| Frames | 抽帧、选择帧、动画段切分 |
| Background | 去背景、alpha edge、透明检查 |
| Anchor | bbox、脚底线、origin、pivot |
| Sheet | sprite sheet、atlas、packing |
| Export | `.gsfpack`、engine exporters、预览 |

长期 Source step 不应被设计成简单文件导入页。它是资产生产流水线的起点，未来应包含三个模式：

```text
Generate
Import
Use Recipe
```

个人开发者 MVP 先只实现 Import 模式：

```text
Import:
  - import_video
  - import_frames
  - import_sprite_sheet
  - import_gsfpack
```

后续 Source provider 类型：

```text
Generate:
  - text_to_reference_image
  - reference_to_motion_video
  - text_or_reference_to_image_sequence experimental
  - text_or_reference_to_sprite_sheet experimental
  - pose_guided_generation

Use Recipe:
  - creator_recipe
  - installed_pack_recipe
  - marketplace_recipe
```

UI 原则：

- 不把第一步命名为 `Video`，避免产品被误解成视频转帧工具。
- 个人开发者 MVP 的第一屏可以是 `Import`，但内部模型仍使用 `Source Provider`。
- Generate 不出现在 MVP 导航和主要按钮中。
- 后续启用 Generate 时，不把生成藏在 `Import` 的二级按钮里。
- 后续生成结果必须通过 `Send to Frames` 进入后续质量控制。
- 后续 `Generate` 的默认产物叫 candidate，不叫 final asset。
- 后续纯图像 sprite sheet provider 必须显示 `Experimental` 或 `Prototype` 标签。
- 未来图像模型直接生成连续帧或 sprite sheet 时，只新增 Source provider，不改变 Frames/Background/Anchor/Sheet/Export。

状态：

```text
inactive
active
completed
warning
blocked
processing
```

### 9.6 CanvasEditor

职责：

- 显示当前帧。
- 显示透明棋盘格。
- 显示 bbox、裁剪框、脚底线、锚点。
- 支持缩放、平移、选择、对齐。

子组件：

- `CanvasToolbar`
- `CheckerboardCanvas`
- `SpritePreview`
- `BoundingBoxOverlay`
- `AnchorLineOverlay`
- `FrameScrubber`
- `ZoomControl`
- `PixelPreviewToggle`

关键状态：

```text
empty
loadingFrame
frameReady
processing
selectionActive
anchorEditing
backgroundPreview
exportPreview
error
```

交互：

- 拖动 bbox handles。
- 拖动 foot anchor line。
- 切换 frame。
- 缩放 25% / 50% / 100% / 200% / Fit。
- Pixel Preview on/off。
- Space + drag pan。

### 9.7 TimelinePanel

职责：

- 展示多个动画 clip。
- 分段、预览和检查循环。
- 提供帧级选择。

子组件：

- `ActiveAssetsPanel`
- `TimelineToolbar`
- `TimelineRuler`
- `AnimationTrack`
- `FrameStrip`
- `Playhead`
- `TransportControls`
- `FpsControl`
- `LoopRangeControl`

AnimationTrack 状态：

```text
idle
walk
run
attack
hurt
death
custom
```

每条 track 应支持：

- frame count。
- lock / visibility。
- warning indicator。
- drag reorder。
- trim start/end。
- loop marker。
- active frame selection。

### 9.8 SpriteSheetPreview

职责：

- 预览最终 packed sheet。
- 显示 size、margin、padding。
- 切换 packing profile。

状态：

```text
notGenerated
generating
ready
oversized
hasTransparentWaste
packingFailed
```

建议操作：

- 选择尺寸：1024、2048、4096。
- 选择 margin/padding。
- 显示 atlas cell boundaries。
- 打开大图预览。

### 9.9 InspectorPanel

Tab：

```text
Quality Report
Generation Recipe
Pack Metadata
Export Targets
```

Inspector 是这个产品非常关键的区域，它不只是属性面板，而是把“素材是否能用于游戏”和“素材是否能成为资源包”解释清楚。

### 9.10 QualityReport

当前指标：

```text
BBox Stability
Background Removal
Foot Anchor Drift
Loop Match (Start/End)
Alpha Edge Quality
Frame Consistency
```

建议扩展：

```text
Transparent Bounds
Frame Size Consistency
Pivot Consistency
Silhouette Jitter
Loop Seam Score
Color Palette Stability
Engine Compatibility
License Metadata Completeness
Preview Availability
Cell Boundary Safety
BBox Width Variation
Game-ready Verdict
```

状态等级：

```text
pass      green
warning   amber
fail      red
info      blue
unknown   muted
```

质量结论：

```text
Quick Prototype
Prototype usable
Needs cleanup
Needs regeneration
Game-ready
```

`Game-ready` 只能由质量检查得出，不能由生成 provider 直接声明。纯图像生成的 sprite sheet 默认应落在 `Quick Prototype` 或 `Prototype usable`，除非 bbox、脚底线、loop、frame diff、cell boundary 等检查全部通过。

#### 9.10.1 后处理实验室交互模式

参考 `sprite-video-lab` 后，Forge 的 Background / Frames / Anchor 不能只做参数表单，必须提供“先单帧验证，再批处理，再质量解释”的闭环。

Background step 推荐布局：

```text
Source Frame Preview
  显示原始抽帧
  支持缩放和平移
  显示采样时间和帧号

Processed Frame Preview
  显示当前 matting 参数处理后的结果
  支持棋盘格、纯色和游戏场景色背景
  显示 key color、threshold、softness、despill、halo

Parameter Controls
  matting mode
  key color mode
  threshold
  softness
  despill strength
  halo pixels
  luma black/white/gamma/strength

Actions
  Preview Current Frame
  Apply to Selected Range
  Apply to Full Clip
  Send to Quality Check
```

Frames step 应提供：

```text
video segment trim
keep every N frames
selected frame count
odd/even/invert selection
animation preview
reverse preview/export
```

Anchor step 应提供：

```text
union bbox overlay
per-frame bbox ghost overlay
foot line
center line
pivot point
drift chart
manual lock foot line
manual fixed anchor
```

交互原则：

- 参数变化不应立即跑全量视频，先更新单帧预览。
- 批处理完成后必须自动更新 Quality Report。
- 如果质量失败，右侧 inspector 显示具体修复入口，例如 `Open Anchor Step`、`Trim Loop Range`、`Regenerate Shorter Motion`。
- 处理过程中的中间产物必须可追踪到 job，不只是 UI 临时状态。

每条质量项结构：

```ts
type QualityCheck = {
  id: string;
  label: string;
  status: "pass" | "warning" | "fail" | "info" | "unknown";
  summary: string;
  metric?: string;
  affectedAssets?: string[];
  action?: {
    label: string;
    command: string;
  };
};
```

### 9.11 PackMetadata

字段：

```text
Pack name
Version
Creator
License
Tags
Compatible engines
Animation list
Preview GIF/WebM
Source recipe
AI provider disclosure
Commercial usage notes
```

MVP 里至少要有：

- name。
- version。
- creator。
- license type。
- compatible engines。
- animations。

### 9.12 GenerationRecipe

职责：

- 记录当前资产从哪个 source provider 而来。
- 保存 prompt、参考图、动作参数、provider、成本和后处理参数。
- 支持用户复用、remix、发布和 marketplace 分发。

Recipe 信息：

```text
Source type
Provider
Prompt
Reference image
Actions
Duration / FPS
View lock
Background strategy
Postprocess preset
Cost estimate
Generated artifacts
```

状态：

```text
draft
ready
missingProvider
needsCostConfirmation
generating
failed
saved
publishReady
```

GenerationRecipe 是创作者生态的关键对象。用户未来购买或安装的不应只是最终图片，也可以是可复用的 production recipe。

### 9.13 ExportTargets

第一批目标：

```text
Generic PNG + JSON
Godot 4
Unity
Phaser 3
Cocos Creator
GameMaker
```

导出格式：

```text
.gsfpack
frames/*.png
sprite_sheet.png
atlas.json
manifest.json
preview.gif
quality-report.json
```

状态：

```text
available
selected
requiresConfig
notSupportedYet
exporting
exported
failed
```

Primary action：

```text
Export Pack
```

Secondary action：

```text
Preview Export
Open Exports Folder
```

### 9.14 StatusBar

展示：

```text
Frames
Sheet size
FPS
Duration
Quality summary
App version
```

状态：

- 所有检查通过。
- 有 warnings。
- 正在处理。
- 最近导出失败。
- helper disconnected。

## 10. 页面级设计规范

### 10.1 Library

目的：

- 让用户快速进入最近项目、最近 pack、示例包。

模块：

- Recent Projects。
- Recent Packs。
- Sample Packs。
- Quick Actions。

空状态：

```text
Import Video
Import PNG Sequence
Import Sprite Sheet
Create Pack
Install Sample Pack
Open Existing Pack
```

### 10.2 Forge

目的：

- 核心生产工作台。

必须支持：

- Import 入口。
- 多工作流步骤。
- 帧预览。
- 动画时间轴。
- 质量检查。
- 导出。

MVP 默认应落在 `Import` step。后续接入 AI/BYOK 或 creator recipe 后，可以把默认新建项目改回长期 `Source` step。

### 10.2.1 Import 页面（MVP）

目的：

- 让用户从本地视频、PNG 序列、已有 sprite sheet 或 `.gsfpack` 创建素材源。
- 把本地文件变成后续 Forge Pipeline 的输入。
- 避免第一版出现 provider、API key、成本确认和生成失败处理。

布局建议：

```text
Import Page
  Import Cards
    Import Video
    Import PNG Sequence
    Import Sprite Sheet
    Import .gsfpack

  Source Metadata
    file name
    resolution
    duration
    fps
    source kind

  Video Segment
    start time
    end time
    keep every N frames
    selected frame count

  Dependency Status
    ffmpeg detected / missing
    ffprobe detected / missing
    open Settings
```

关键按钮：

```text
Import Video
Import Frames
Import Sprite Sheet
Import .gsfpack
Preview Current Frame
Send to Frames
```

Import 模式的状态：

```text
ffmpegMissing
waitingForSource
sourceReady
probingSource
probeFailed
segmentReady
previewReady
sentToFrames
```

注意：

- 第一版不显示 `Generate` tab、Provider、BYOK、Cost Preflight。
- Source metadata 仍要保留 `sourceProviderKind`，为后续生成 provider 留接口。
- 如果用户导入 `.gsfpack`，应直接进入 Pack preview 和 Quality Report。
- 后续启用 Generate 时，单独恢复 `Source / Generate` 页面，不能把它混进 Import 表单。

### 10.3 Packs

目的：

- 管理本地 `.gsfpack`。

模块：

- Installed Packs。
- Pack Details。
- Validation Report。
- License Info。
- Export / Reveal / Remove。

状态：

- installed。
- corrupted。
- missing files。
- unlicensed。
- update available。

### 10.4 Marketplace（后置）

目的：

- 未来生态入口，不属于个人开发者 MVP。

未来内容：

- Featured Packs。
- Free Sample Packs。
- Creator Spotlight。
- Submit Pack / Join Waitlist。
- Open Website。

注意：

- App 内 marketplace 可以在本地 App 闭环稳定后再做轻量 registry。
- 不要第一版就做复杂支付、分账、审核后台。

### 10.5 Exports

目的：

- 管理导出历史和目标路径。

模块：

- Recent Exports。
- Export Presets。
- Engine Targets。
- Open Folder。
- Re-export。

### 10.6 Settings

模块：

- General。
- Processing。
- FFmpeg。
- Export Paths。
- License。
- Updates。
- Diagnostics。

关键状态：

- ffmpeg detected / missing。
- helper installed / missing。
- update available。
- app notarized / signature info。

### 10.7 Creator Publish（后置）

目的：

- 在本地 pack 格式稳定后让生态可见。
- 先生成 publish draft，不必完整上架。

流程：

```text
Select Pack
Validate Quality
Fill Metadata
Choose License
Generate Preview
Create Publish Draft
Open Website Submission
```

输出：

```text
publish-draft.json
preview assets
creator metadata
license summary
quality-report.json
```

## 11. 关键状态覆盖

### 11.1 Source Empty

当没有任何资产时：

- 中央 canvas 显示 drop zone。
- 左侧 pack list 显示 Create / Import / Open Pack。
- 右侧 inspector 显示 import checklist。

Primary action：

```text
Import Video
```

Secondary：

```text
Import PNG Sequence
Import Sprite Sheet
Open Existing Pack
```

### 11.2 Processing

用于：

- ffmpeg 抽帧。
- 背景移除。
- bbox 扫描。
- sheet packing。
- export。

要求：

- 显示进度。
- 显示当前处理文件。
- 支持 cancel。
- 处理期间禁用危险操作。
- 不要阻塞基础浏览。

### 11.3 Warning

例子：

```text
Foot anchor drift in knight_walk (1.2px)
```

要求：

- warning 不阻止导出。
- 但导出前需要在 Quality Report 中可见。
- 可提供 Fix / Ignore / Explain。

### 11.4 Error

常见错误：

- ffmpeg missing。
- video decode failed。
- output folder not writable。
- alpha removal failed。
- sheet too large。
- pack manifest invalid。
- license metadata missing。

要求：

- 错误信息要能指导用户下一步。
- 不要只显示 technical stack trace。
- Diagnostics 可以另存完整日志。

### 11.5 Export Success

导出后展示：

- 输出路径。
- 生成文件列表。
- compatible engines。
- quality summary。
- Open Folder。
- Copy Import Instructions。

## 12. Interaction Spec

### 12.1 拖拽

支持拖入：

- mp4 / mov / webm。
- png sequence。
- sprite_sheet.png。
- `.gsfpack`。
- zip。

拖拽反馈：

- 全局 drop overlay。
- 显示可导入类型。
- 无效文件显示 reject state。

### 12.2 键盘快捷键

建议：

```text
Space              play / pause
Left / Right       previous / next frame
Shift + Left/Right jump 10 frames
Cmd + S            save
Cmd + E            export
Cmd + Z            undo
Cmd + Shift + Z    redo
Cmd + Plus         zoom in
Cmd + Minus        zoom out
Cmd + 0            fit canvas
```

UI 内不要大段展示快捷键说明，放到菜单或 help overlay。

### 12.3 右键菜单

Canvas：

- Set anchor here。
- Reset bbox。
- Fit to sprite。
- Copy frame。
- Reveal source frame。

Timeline：

- Rename animation。
- Duplicate track。
- Trim to selection。
- Mark loop start。
- Mark loop end。
- Delete animation。

Pack：

- Reveal in Finder。
- Validate pack。
- Export。
- Duplicate。
- Remove。

## 13. Accessibility

虽然这是专业工具界面，也必须保证基础可访问性：

- 所有 icon button 必须有 tooltip 和 aria-label。
- active tab 不只依赖颜色，需有位置/下划线/边框。
- warning 不只依赖颜色，需有图标。
- 文本对比度至少达到工具型界面可读标准。
- 键盘可操作核心流程。
- 时间轴 frame strip 可以横向滚动并支持键盘移动。
- 状态和错误可被 screen reader 读取。

## 14. Figma 使用规范

Figma 当前角色：

```text
Design review source
Component naming source
Visual alignment source
Collaboration source
```

Figma 不承担：

```text
Production source of truth
Runtime layout source
State logic source
Export logic source
```

建议在 Figma 中建立页面：

```text
00 Design Tokens
01 App Shell
02 Import
03 Forge Editor
04 Pack Library
05 Export Flow
06 Empty / Error / Processing States
07 Future Source / Generate
08 Future Marketplace / Creator Publish
```

导入的 html.to.design 页面可以作为 `03 Forge Editor` 的视觉参考。`02 Import` 应先服务本地文件导入和 ffmpeg 依赖检查；`07 Future Source / Generate` 只做后续概念，不进入 MVP 开发。

## 15. HTML 原型到生产代码映射

当前 HTML 文件：

```text
figma-import/game-sprite-forge-app.html
```

建议 React 组件映射：

```text
App
  AppShell
    TopToolbar
      ProjectSelector
      WorkflowTabs
      ToolbarActions
    SidebarNav
    PackListPanel
      PackCard
    ForgeEditor
      ImportSourcePanel
      CanvasEditor
      FrameScrubber
      TimelinePanel
      SpriteSheetPreview
    InspectorPanel
      QualityReport
      PackMetadata
      ExportTargets
    StatusBar
```

建议目录：

```text
src/
  app/
    routes/
    AppShell.tsx
  ui/
    tokens.ts
    components/
      Button.tsx
      IconButton.tsx
      Tabs.tsx
      Panel.tsx
      StatusBadge.tsx
  features/
    forge/
      ForgePage.tsx
      ImportSourcePanel.tsx
      CanvasEditor.tsx
      TimelinePanel.tsx
      QualityReport.tsx
      ExportTargets.tsx
    packs/
    exports/
    settings/
```

## 16. 数据模型对 UI 的最低要求

### 16.1 SourceProvider

```ts
type SourceProviderKind =
  | "import_video"
  | "import_frames"
  | "import_sprite_sheet"
  | "import_gsfpack"
  | "text_to_reference_image"
  | "reference_to_motion_video"
  | "text_or_reference_to_image_sequence"
  | "text_or_reference_to_sprite_sheet"
  | "pose_guided_generation"
  | "creator_recipe"
  | "installed_pack_recipe"
  | "marketplace_recipe";

type SourceProvider = {
  id: string;
  kind: SourceProviderKind;
  label: string;
  requiresApiKey: boolean;
  supportsCostPreflight: boolean;
  maturity: "stable" | "experimental" | "prototype";
  status: "available" | "missingConfig" | "unsupported" | "offline";
};
```

MVP 只允许启用：

```ts
const MVP_ENABLED_SOURCE_PROVIDERS: SourceProviderKind[] = [
  "import_video",
  "import_frames",
  "import_sprite_sheet",
  "import_gsfpack",
];
```

其它 provider 可以存在于类型定义中，但 UI 不显示，设置页也不提供 API key 字段。

### 16.2 GenerationRecipe（后置）

`GenerationRecipe` 是未来生成和生态能力的对象。MVP 可以保留类型设计，但不需要实现表单、队列、provider 状态或成本确认 UI。

```ts
type GenerationRecipe = {
  id: string;
  sourceProviderKind: SourceProviderKind;
  prompt?: string;
  referenceImageId?: string;
  actions: string[];
  generationLevel?: "quickPrototype" | "poseGuided" | "videoToFrames" | "rigOrTemplateRecipe";
  durationSec?: number;
  fps?: number;
  viewLock?: "side" | "front" | "topDown" | "threeQuarter";
  backgroundStrategy?: "transparent" | "chromaKey" | "solid" | "autoRemove";
  providerMetadata?: {
    providerId: string;
    model?: string;
    estimatedCost?: string;
  };
  postprocessPresetId?: string;
};
```

### 16.3 Pack

```ts
type Pack = {
  id: string;
  name: string;
  version: string;
  creator?: string;
  license?: LicenseSummary;
  thumbnail?: string;
  sourceProviderKind?: SourceProviderKind;
  generationRecipeId?: string;
  animations: AnimationAsset[];
  compatibleEngines: EngineTarget[];
  qualityStatus: "pass" | "warning" | "fail" | "unknown";
};
```

### 16.4 AnimationAsset

```ts
type AnimationAsset = {
  id: string;
  name: string;
  frameCount: number;
  fps: number;
  durationMs: number;
  loop: boolean;
  status: "ready" | "warning" | "error" | "processing";
};
```

### 16.5 ExportTarget

```ts
type ExportTarget = {
  id: "generic" | "godot4" | "unity" | "phaser3" | "cocos" | "gamemaker";
  label: string;
  status: "available" | "requiresConfig" | "unsupported";
  selected: boolean;
};
```

### 16.6 QualityReport

```ts
type QualityReport = {
  summary: "pass" | "warning" | "fail" | "unknown";
  checks: QualityCheck[];
  lastRunAt?: string;
};
```

## 17. MVP UI 范围

### 17.1 必须实现

```text
AppShell
SidebarNav
Forge Editor
Import step
PackList
Canvas Preview
Timeline
Quality Report
Export Targets
Export Pack action
Settings: FFmpeg path / Output folder
Onboarding: dependency check
```

### 17.2 可以简化

```text
Pack Metadata 先支持基础字段
Timeline editing 先支持选择和预览，不急着做复杂剪辑
Packs 先只做本地导入和校验，不做线上安装
```

### 17.3 暂不实现

```text
Generate / BYOK
Marketplace
Creator Publish
完整付费 marketplace
自动分账
复杂资源审核后台
多人协作
云端视频处理
完整在线编辑器
```

## 18. 开发优先级

### Phase 1: UI Skeleton

- AppShell。
- Sidebar。
- Forge route。
- Import step 静态结构。
- 静态 PackList。
- 静态 Canvas。
- 静态 Inspector。
- 静态 Timeline。

目标：让真实 App 先长得像当前 HTML 原型。

### Phase 2: Local Asset Loop

- 导入视频或 png sequence。
- Source metadata 记录。
- 单帧处理预览。
- 区间抽帧参数。
- 生成帧列表。
- Canvas 显示真实帧。
- Timeline 显示真实 frame strip。
- Pack metadata 存储到本地。

目标：让 UI 绑定真实数据。

### Phase 3: Quality Pipeline

- bbox stability。
- alpha edge quality。
- foot anchor drift。
- frame consistency。
- loop match。

目标：右侧 Quality Report 从静态变成真实检查结果。

### Phase 4: Export Pipeline

- `.gsfpack`。
- sprite_sheet.png。
- atlas.json。
- manifest.json。
- preview.gif。
- quality-report.json。

目标：Export Pack 真的可用。

### Phase 5: Ecosystem UI

- local pack import。
- local pack validate。
- local pack re-export。

目标：本地资源包管理闭环。线上生态、website handoff、creator publish draft 后置。

## 19. 视觉验收标准

开发实现完成后，需要和 HTML/Figma 做视觉对比。

验收区域：

```text
top-toolbar
sidebar-nav
pack-list
canvas-editor
timeline
sprite-sheet-preview
quality-report
export-targets
statusbar
```

每个区域检查：

- 尺寸是否接近。
- 文字层级是否正确。
- active / selected / warning 状态是否明显。
- 线条和边框是否稳定。
- 图片资源是否不变形。
- 时间轴是否不会撑破布局。
- 右侧导出按钮是否始终可见。

## 20. 关键设计原则

1. **Forge 页面优先真实生产，不做营销感界面。**
2. **Godot 是 exporter，不是产品身份。**
3. **`.gsfpack` 是第一等 UI 对象。**
4. **质量检查是核心卖点，不能藏在二级菜单。**
5. **导出目标必须清楚表达“这个资产能进哪个引擎”。**
6. **创作者生态从数据结构开始，第一版 UI 只做本地 pack，不做线上生态。**
7. **Figma 用于设计协作，HTML 用于实现基准，App 才是最终产品。**

## 21. 下一步建议

建议按这个顺序继续：

1. 在 Figma 中把导入页面拆成 `App Shell / Forge Editor / Timeline / Inspector / Export Targets` 组件。
2. 基于本规范创建 `tokens.ts` 和 React 组件目录。
3. 用当前 HTML 原型作为第一轮 Tauri + React UI 的视觉基准。
4. 先实现静态 Forge Editor，再接入真实本地文件和 ffmpeg pipeline。
5. 先设计 `Import`、`Packs`、`Exports` 和 `Settings`，`Marketplace` 与 `Creator Publish` 后置。
