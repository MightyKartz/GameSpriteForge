# Forge Stage 3：项目级静态视觉资产实施计划

状态：实施中  
分支：`codex/stage3-static-assets`  
范围：CollectionLock、Portrait、Equipment V1、Decal、editable/replace-item、跨 Pack audit  
非范围：自然语言地图、UI/VFX、Unity/Unreal、MCP、新 Provider、逐帧装备挂件

## 1. 目标与发布边界

Stage 3 把现有 Icon/Prop 的“逐项生成”升级为项目级静态视觉资产系统。所有资产必须锁定 Provider、模型、Style revision 与 Collection revision；Forge 负责确定性归一化、一致性检测、Pack、Catalog、Godot 安装与可审计来源，模型只负责像素生成。

本阶段先完成完全离线、可重复的实现和 fixture 验收。真实 xAI 验收会产生费用，必须在执行前单独获得用户明确授权；未授权不影响离线实现结论，但不能宣称真实模型门槛通过。

## 2. 公共 CLI

```text
forge collection create --project <path> --spec <collection.json> [--wait|--plan-only] --json
forge collection inspect --project <path> --id <id> [--revision <rev>] --json

forge generate portrait-set --project <path> --spec <portrait.json> [--wait|--plan-only] --json
forge generate equipment-set --project <path> --spec <equipment.json> [--wait|--plan-only] --json
forge generate decal-set --project <path> --spec <decal.json> [--wait|--plan-only] --json

forge asset export-editable --project <path> --id <asset-id> --output <dir> --json
forge asset replace-item --id <job-id> --item <item-id> --path <png> [--wait] --json

forge project audit --project <path> \
  [--scope completeness|consistency|quality|provenance|godot|all] --json
```

命令继续遵守单 JSON stdout、stderr 诊断、稳定错误码、耐久 Job、一次性 plan token 和异步默认行为。`replace-item` 必须创建子 Job、Provider 请求数为 0，绝不修改源 Job 或源 Pack。

## 3. 数据契约

### CollectionLock

- `CollectionSpecV1`：集合 ID/名称/种类、Style revision、集合提示、材质、尺度、视角/落地规则、画布、0–1 张本地 anchor、许可证。
- `CollectionLockV1`：不可变 revision、Provider/profile/model、Style revision 与 board SHA、anchor/medoid 路径和 SHA、基线签名、outlier profile、许可证。
- 文件位置：`.forge/collections/<collection-id>/<revision>/collection-lock.json`。
- 无本地 anchor 时由 Provider 从 Style board 派生；有本地 anchor 时只做本地规范化，不产生 Provider 请求。

### 静态集合

- Icon/Prop V1 保持可读；V2 与 Portrait/Equipment/Decal 显式引用 Collection revision。
- Portrait 初始表情固定为 `neutral/happy/angry/hurt/surprised`，并引用一个 SubjectLock。
- Equipment item 输出 inventory icon、world prop 与可选 static equipped preview；V1 不生成动画帧挂件。
- Decal item 输出透明 PNG、footprint、blend recommendation，可选 atlas，不输出 gameplay trigger。
- Pack 继续使用 V2 静态兼容布局，并附加 `collection-lock-ref.json` 与 `collection-consistency-report.json`；旧读取器仍可读基础 item。

## 4. 一致性与离群检测

固定 profile `collection-consistency@1.0.0`：

1. 每个 item 先通过既有硬门禁：PNG、画布、Alpha、无裁切、单主体/单对象与 SHA。
2. 以通过硬门禁的候选计算 pairwise 相似度矩阵，指标为前景调色板、边缘密度、尺度、感知哈希和锚点/落地。
3. 集合 medoid 是平均距离最小的 item；并与 Collection anchor 同时作为基线。
4. item 综合相似度 `>=0.70` 为通过，`0.55–0.70` 为灰区，`<0.55` 为 outlier 并定向重试。
5. 每 item 最多两次 Provider 尝试；第二次仍为 outlier 时进入 `awaiting_review`，硬失败不可人工越过。
6. 报告保存 medoid、pairwise 摘要、每项分数/原因/尝试数与 native-size contact sheet 路径。

## 5. 可编辑替换与审计

- editable export 复制 Pack item PNG、spec 快照、lock 引用、SHA manifest、许可证和编辑说明到新目录；不包含凭据或临时 URL。
- replace-item 校验路径边界、PNG、画布、Alpha 和 SHA，复用源 Job 的未失效 item，仅重跑该 item 的归一化、集合一致性、Pack 和 Catalog；报告明确 `providerRequests: 0`。
- project audit 读取 project、catalog、Pack、GameArtManifest 与 Godot 安装记录，检查完整性、锁/依赖哈希、质量/集合一致性、Pack 布局、license/provenance、安装陈旧、孤儿/缺失文件、凭据模式、临时 URL、资源体积和内嵌图像。

## 6. 实施顺序

1. 建立 `collection-assets` feature、schema 与 CollectionLock Core。
2. 扩展静态生成请求和 runner，加入 CollectionLock 引用、medoid/outlier 和局部第二次尝试。
3. 接入 Portrait、Equipment、Decal CLI/spec/Pack/Godot。
4. 实现 editable export 与 replace-item 子 Job。
5. 实现 ProjectAuditReportV1 与 CLI。
6. 扩展 GameArtManifest/Catalog lock provenance。
7. 完成 fixture、schema、CLI、Pack、Godot、安全与回归证据。

## 7. 离线验收门槛

- Collection：本地 anchor 零 Provider 请求；Provider anchor 的 plan 估算准确；revision 对相同输入稳定、文件篡改可检测。
- Fixture：Icon、Prop、Portrait、Equipment、Decal 覆盖 pass/gray/fail/second-success/outlier。
- Retry：只重试失败 item；replace-item 只失效目标 item 和下游，源 Job/Pack SHA 不变。
- Pack/Godot：五类静态 Pack 均验证并安装；外部 PNG、无内嵌 Image/PackedByteArray、文本资源 `<1 MiB`。
- Audit：覆盖缺失 Pack、SHA 漂移、过期安装、缺 license、outlier、凭据/URL 泄漏与 clean project。
- 回归：默认 workspace、Stage 2 manifest、Character V1/V2、安装器与既有静态/world contract 不退化。

## 8. 真实 Provider 门槛（需另行授权）

- 至少 5 个 Style；每 Style 生成 1 个 Icon Pack、1 个 Prop Pack、1 个 Portrait Pack。
- Icon/Prop 每 Pack 至少 10 items；只允许失败 item 定向重试。
- 所有集合离群项必须被机器报告捕获，native-size contact sheet 全量人工复核。
- Pack、Godot、来源、license、凭据扫描通过率 100%，错误 Pack 导出数为 0。
- 执行前输出预计/最大 xAI 请求数和预算，用户明确接受费用后才启动。

