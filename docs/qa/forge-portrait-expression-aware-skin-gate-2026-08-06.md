# Forge Portrait 表情感知肤色门禁验收

日期：2026-08-06  
结论：**通过，需显式人工审核的真实 Portrait Pack 已成功导出并安装 Godot。**

机器可读证据：
[`summary.json`](artifacts/forge-portrait-expression-aware-skin-gate-20260806/summary.json)

最终 contact sheet：
[`contact-sheet.png`](../../generated-assets/forge-portrait-v2-real-20260806/jobs/65f17f65-5951-4ae7-a23a-9e4fd77e754a/contact-sheet.png)

实施合同：
[`forge-portrait-expression-aware-skin-gate-plan.md`](../architecture/forge-portrait-expression-aware-skin-gate-plan.md)

## 修改结果

新增 `portrait-local@1.1.0`：

- 按 `happy`、`angry`、`hurt`、`surprised` 排除确定性的眼眉和嘴部表情区；
- 只比较 neutral 与候选图中都仍为皮肤的对应像素；
- 使用去掉两端各 10% 的截尾均值计算 CIE76；
- 新增 `skinCorrespondenceRatio`，避免异常颜色通过“不再被识别为皮肤”逃避检测；
- 普通与严重暗化同时使用比例和最小异常像素数，减少小图百分比失真；
- 只有非严重肤色问题会降为 `awaiting_review`；硬缺陷仍不可人工越过。

Schema/Pack 继续接受旧 `portrait-local@1.0.0`，新报告与 Godot provenance 使用 1.1。

## 真实零费用 replay

源真实 Expressions Job：`33f2ac78-4f63-451b-871b-18b6da6b8c0f`  
最终 replay Job：`65f17f65-5951-4ae7-a23a-9e4fd77e754a`

Replay Provider 请求数为 0。自动结论：

- neutral：`game_ready`；
- happy：`awaiting_review`，肤色 ΔE 从旧算法的 9.26 降至 3.86；
- angry：`game_ready`；
- hurt：`game_ready`；
- surprised：`awaiting_review`，稳健肤色 ΔE 13.21、皮肤对应关系 0.976、身份相似度
  0.908；严重暗化比例仅 1.75%，没有自动放行。

所有最终图的脸部范围外变化率为 0。用户完成原尺寸视觉审核后，happy 与 surprised 写入
`accepted_by_review`，整包晋级 `game_ready`。

## Pack 与 Godot

- Pack：
  `generated-assets/forge-portrait-v2-real-20260806/jobs/65f17f65-5951-4ae7-a23a-9e4fd77e754a/exports/ayla-full-body-portraits/ayla-full-body-portraits.gsfpack`
- Pack 校验：通过；
- Godot 安装 Job：`e31c74e1-6cb8-4ca1-b11e-b70f34555ce8`；
- Godot 4.6.3 headless 项目加载：通过；
- 五个 item 均为外部 PNG；
- 无内嵌 Image/PackedByteArray，无超过 1 MiB 的文本资源；
- `forge_usage.json` 记录 `portrait-local@1.1.0`、Base approval 和 reference policy。

## 回归与安全

- 扩大眼睛/嘴巴不再被当成肤色漂移；
- 非表情区换成非肤色会被 `skinCorrespondenceRatio` 拦截；
- 深色脸颊涂鸦仍由严重肤色或 face artifact 门禁进入 `regenerate`；
- Stage 3、CLI 产品、Pack、Godot、workspace、Clippy 和格式门禁通过；
- Job、Pack、Godot 目录未发现凭据、Authorization header 或临时媒体 URL。

本次修改与 replay 没有新增 xAI 请求；真实生成阶段原费用仍为 7 次请求、约 0.43 美元。
