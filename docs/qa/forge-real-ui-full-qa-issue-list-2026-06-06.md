# Forge Real UI Full QA Issue List

Date: 2026-06-06 22:18 CST

Status: DONE_WITH_CONCERNS

Scope: 使用真实 macOS UI 和 Computer Use 测试已安装版本 `/Applications/Game Sprite Forge.app`。本轮只做验证和问题记录，不实施修复。

## Tested App

```text
App: /Applications/Game Sprite Forge.app
Bundle ID: dev.gamespriteforge.desktop
Version shown in UI: v0.1.0
Process ids observed: 7669 -> 26106
Latest verified export:
/Users/kartz/Game Sprite Forge/Exports/Green-Box-Character-Pack-1780755461163/
```

最新导出目录包含 `Green-Box-Character-Pack.gsfpack/forgepack.json`、`quality-report.json`、`frames/frame_001.png` 到 `frame_024.png`、`sprite_sheet.png`、`atlas.json`、`manifest.json`、`godot_import.json` 和 `preview.gif`。

## Passed In This Pass

- 已安装 App 能启动，首屏为中文 UI，页脚版本显示 `v0.1.0`。
- `检查工具链` 可用，底部状态显示 `ffmpeg 和 ffprobe 可用`。
- `运行示例流程` 可完成一键导入、处理、质量检查、导出和重新导入验证。
- 分步路径可走通：`填入示例路径` -> `导入所选` -> `抽取帧` -> `处理并检查质量` -> `导出资源包` -> `验证重新导入`。
- 抽取后帧数修正为 24，处理后质量报告显示 `可用于游戏，1 条建议`。
- 导出后生成 `.gsfpack` 目录、通用文件、Godot 辅助文件和精灵表预览。
- `验证重新导入` 成功，状态显示 `已验证并重新导入 Green Box Character Pack，共 24 帧`。
- 工作流顶部标签可切换；点击 `导出` 后中央说明切换为 `导出检查`。
- 设置页可见语言、ffmpeg/ffprobe 路径、默认导出文件夹、默认 FPS、默认精灵表尺寸、应用版本和安装路径。
- 导出资源库页面可见本地导出列表，视觉上展示 `刷新资源包库`、`检查`、`验证资源包`、`重新导入资源包` 和 `打开` 操作。

## Issues

### P1: 设置页和导出资源库内容没有进入可访问性树，键盘也无法进入主要表单/卡片操作

Repro:

1. 打开已安装 App。
2. 点击左侧 `设置`。
3. 使用 Computer Use 读取窗口状态，或连续按 `Tab`。
4. 点击左侧 `导出`，重复读取状态和 Tab 测试。

Observed:

设置页截图中能看到语言下拉、路径输入和按钮，但可访问性树只暴露顶部导航和侧边栏。导出资源库同样只暴露顶部导航，列表卡片、刷新、检查、验证、重新导入、打开按钮都没有元素编号。Tab 焦点只在左侧导航之间移动，无法进入设置表单或资源包卡片操作。

Expected:

所有可见表单字段、按钮、资源包卡片和卡片操作都应进入 macOS 可访问性树，并按合理顺序支持键盘 Tab/Enter 操作。

Impact:

辅助技术用户无法操作设置和资源库；真实 UI 自动化也无法稳定验证资源库的刷新、检查、验证和重新导入功能。

Suggested fix:

检查 SettingsRoute 和 ExportsRoute 的 DOM 结构、`aria-hidden`、自定义滚动容器、卡片按钮层级和 focus 样式。为表单控件提供可聚焦原生控件或显式 label/role，并增加 Playwright/AX 快照回归测试：设置页必须暴露语言下拉和路径输入，资源库必须暴露每张卡的四个操作按钮。

### P1: 分步导入后中间状态显示 64 帧和骑士示例，和 Green Box 视频实际内容不一致

Repro:

1. 首屏点击 `填入示例路径`。
2. 点击视频行的 `导入所选`。
3. 观察左侧当前来源、运行摘要、中央预览和时间线。

Observed:

导入后左侧显示 `Green Box Character Pack`，但 `工作区内 64 帧`；运行摘要中又显示视频预计 24 帧；中央仍显示骑士 `示例预览 - 不是活动工作区`，时间线仍像示例占位。只有继续点击 `抽取帧` 后，状态才修正为 24 帧并显示绿色背景素材。

Expected:

导入视频但尚未抽帧时，应明确显示“来源已选择，等待抽取帧”，不应把占位 64 帧当作实时工作区展示；预览也不应继续显示骑士示例，除非标记为纯占位。

Impact:

用户会误以为 Green Box 视频已经导入为 64 帧，且会怀疑 App 把错误素材或旧示例混入当前项目。

Suggested fix:

把 `source selected` 和 `live frames available` 状态拆开。导入视频后只显示视频元数据和抽帧 CTA；抽帧完成前不要创建 64 帧示例时间线，也不要把当前来源标记为实时工作区。

### P1: 导出后运行摘要提前写“本次会话的导出已验证”

Repro:

1. 走分步路径完成处理。
2. 点击 `导出资源包`。
3. 在点击 `验证重新导入` 前观察左侧运行摘要和右侧导出状态。

Observed:

导出后左侧运行摘要已经显示 `本次会话的导出已验证。`，并把验证状态展示为 `就绪`。但右侧仍然显示可点击的 `验证重新导入`，且此时还没有 `上次验证：Green Box Character Pack，24 帧。` 文案。点击验证后才真正出现验证结果。

Expected:

导出完成但未验证时，应显示 `已导出，等待重新导入验证` 或类似状态；只有点击验证并成功后才显示“已验证”。

Impact:

用户可能跳过重新导入验证，以为导出已经被验证过。

Suggested fix:

拆分 `exported` 和 `validated` 状态机。运行摘要里的验证字段只读取真实的 `lastValidation`/validation result，不要由 export success 间接置为 verified。

### P2: 高级元数据面板在处理/导出后自动展开，挤压主要导出操作区

Repro:

1. 首屏高级元数据默认是收起状态。
2. 分步导入并点击 `处理并检查质量`。
3. 观察右侧导出区域。

Observed:

`资源包元数据和精灵表设置` 自动变为展开状态，作者、许可、列数、内边距、外边距、复选框全部显示。右侧主要按钮被推到更低位置；在较小窗口中会更容易需要滚动。

Expected:

高级设置应保持用户显式选择的展开/收起状态。处理或导出流程不应自动展开高级区域，除非其中有错误需要用户处理。

Impact:

主流程按钮和状态被不必要的高级配置挤压，新用户会觉得导出区比实际更复杂。

Suggested fix:

让 `advancedOpen` 只受用户点击或错误聚焦影响。流程状态更新不要重置该 UI 状态；若需要提醒高级设置，可用轻量提示而不是自动展开。

### P2: 首屏样例叙事仍混合骑士素材和 Green Box 输出

Repro:

1. 启动 App，停留在未选择来源状态。
2. 观察中央预览、示例时间线、默认资源包名。
3. 运行示例或分步导入内置示例。

Observed:

首屏中央预览和示例时间线是骑士素材，默认资源包名称已经是 `Green Box Character Pack`，实际内置视频是 `green-box-character.mp4`，最终输出是白色方块/Green Box 风格。

Expected:

首屏样例、默认资源包名、内置视频、导出结果应讲同一个故事。要么全部使用骑士样例，要么首屏就展示 Green Box 素材，并把骑士仅作为历史/外部示例移出主流程。

Impact:

第一次使用时很容易误解“运行示例流程”为什么输出和首屏预览完全不同。

Suggested fix:

统一 bundled sample 资产和文案。短期可把首屏预览替换为 Green Box 的第一帧/抽帧预览；长期可提供明确的样例选择器。

### P2: 应用重启后工作区会话不恢复

Repro:

1. 运行示例流程或分步完成导出验证，工作区处于 24 帧实时状态。
2. 应用进程重启或被重新打开。
3. 回到工作台。

Observed:

本轮真实 UI 中进程从 `7669` 变成 `26106` 后，工作台回到初始未选择来源状态。没有崩溃报告，但活动工作区、处理结果、导出结果和验证状态都没有跨进程恢复。

Expected:

至少应恢复最近一次活动工作区摘要、最近导出的资源包路径和验证状态；如果暂不支持恢复，应在重启后提供“打开最近资源包/重新导入最近导出”的入口。

Impact:

用户重启 App 后无法从上次进度继续，只能去导出资源库或重新跑流程。

Suggested fix:

把最近活动 workspace summary、source path、last export path、validation result 保存到本地设置或 workspace state。启动时提供恢复最近会话或重新导入最近 `.gsfpack` 的明确 CTA。

### P2: 导出资源库视觉可见但无法通过真实 UI 自动化验证刷新/检查/重新导入

Repro:

1. 点击左侧 `导出`。
2. 尝试通过可访问性元素点击 `刷新资源包库`、`检查`、`验证资源包`、`重新导入资源包`。
3. 尝试键盘 Tab 进入卡片操作。

Observed:

视觉上按钮存在，但可访问性树没有对应元素。Computer Use 坐标点击返回 `noWindowsAvailable`，键盘 Tab 只进入侧边栏导航，无法进入卡片按钮。

Expected:

资源库的核心按钮应能被辅助技术、键盘和自动化工具稳定访问。

Impact:

资源库是本地工作台的重要后续入口，但当前真实 UI 自动化无法完成它承诺的验证和重新导入路径。

Suggested fix:

优先修复资源库页面可访问性。修复后补一条真实 UI 回归：刷新资源库 -> 选择最新 Green Box 导出 -> 检查 -> 验证资源包 -> 重新导入资源包 -> 回到工作台确认 24 帧。

### P3: 本轮截图采集受当前桌面窗口枚举限制影响

Observed:

Computer Use 能读取 Forge 窗口并返回 UI 截图；但本机 `Quartz.CGWindowListCopyWindowInfo` 当前只枚举到其他前台窗口，未枚举到 Forge，因此 `screencapture -l` 没能保存 Forge 窗口截图文件。

Expected:

QA 环境应能稳定保存真实 UI 证据截图。

Impact:

不影响产品功能判断，但影响 QA 证据归档。

Suggested fix:

后续真实 UI QA 前关闭/隐藏全屏前台应用，或增加通过 Computer Use 截图落盘的辅助脚本。

## Recommended Next Fix Order

1. 修复 SettingsRoute 和 ExportsRoute 的可访问性/键盘可达性，这是当前最大阻塞。
2. 修复分步导入状态机：导入视频后不要显示 64 帧示例工作区。
3. 修复 export/validate 状态文案，确保未验证时不显示“已验证”。
4. 保持高级元数据面板收起状态，减少主流程 UI 干扰。
5. 统一首屏样例素材和 Green Box 示例流程。
6. 增加最近会话恢复或最近导出重新导入入口。

