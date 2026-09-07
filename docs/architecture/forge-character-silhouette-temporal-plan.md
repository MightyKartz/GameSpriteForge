# Forge Character 轮廓时间稳定与多背景预览实施计划

日期：2026-08-07

## 问题

真实四方向 Character 在透明或深色背景上暴露了两类问题：

- 模型视频会在相邻帧重新绘制兜帽、头肩和身体外轮廓。
- 旧质量门禁只检查中心、脚底、循环和语义，没有把稳定轮廓与正常摆臂、披风、腿部动作分离。

绿色背景消失不是错误。绿色只用于 Provider 输出的 chroma key；matting 后的游戏资产必须是透明 RGBA。为了让用户检查抠图边缘，Forge 需要另外生成绿色、深色和棋盘格调试预览。

## 实施范围

### 1. 归一化与 Alpha

- 全动画共享缩放比例。
- 每帧用身体中心与脚底做确定性锚点校正，允许的中心和脚底相邻步进上限均为 2px。
- 使用预乘 Alpha 双线性采样，避免透明边缘出现暗边。
- `temporal-alpha@1.0.0` 只删除孤立单帧闪点、填补被七个以上邻域像素包围的单帧小孔，并平滑连续前景 Alpha；不做全帧多数投票。
- `rigid-head-alpha@1.0.0` 锁定每个动画首帧的中央头肩 Alpha 轮廓，按当前帧身体中心和头顶对齐，只作用于身体顶部 42%。内部 RGB、表情、手臂、披风和腿部动作不冻结。

### 2. `silhouette-temporal@1.0.0`

每个动画输出：

- 相邻头肩核心 Mask IoU 的最小值和中位数。
- 相邻轮廓距离最大值。
- 身体宽高变化率。
- 身体中心和脚底相邻步进最大值。
- 8 个逐过渡明细，包含末帧到首帧。

硬门禁：

- 最小 IoU `>= 0.90`，中位 IoU `>= 0.93`。
- 最大轮廓距离 `<= 4px`。
- 身体中心和脚底相邻步进 `<= 2px`。
- 缺帧、缺少身体、任一硬门禁失败均为 `blocked`，不能人工强制导出。

形状比较在身体中心和头顶对齐后执行，避免把正常走路起伏误判为形状漂移。手臂、披风和腿部仍通过原有动画/循环门禁和宽高诊断观察。

### 3. Pack、CLI 与 Godot

- Job 和 Pack 写入 `character-silhouette-temporal-report.json`。
- `.gsfpack` `assets.characterSilhouetteTemporalReport` 指向固定报告路径并接受 JSON Schema 校验。
- `forge job report` 返回该报告。
- 硬失败不能通过 `forge job review` 接受。
- Pack provenance 和 Godot `forge_usage.json` 保存 profile、报告路径和 verdict。
- 每个动画额外生成 checkerboard、chroma-green、dark 三种调试 GIF；正式帧仍保持透明。

## 验收门槛

- 稳定合成动画与 2px 中心移动通过。
- 3px 中心移动、单帧头肩缺口被拦截。
- 时间 Alpha 小孔/闪点修复和刚性头肩修复有单元测试。
- 冻结的真实 xAI 四方向素材零 Provider 请求重放，四动画全部 `game_ready`。
- Pack 校验、Godot 4.6.3 无头导入和场景加载通过。
- `.tres/.tscn < 1 MiB`，无 `PackedByteArray`、内嵌 Image 或凭据。
- 完整 Rust、CLI 产品和 Stage 3 静态资产回归通过。

