# Forge Stage 3 真实 xAI 验收（2026-08-05）

结论：**未通过真实模型发布门槛，不应晋级 Stage 3 商业一致性声明。**

本轮按用户明确授权执行 1 个 Style、1 个 Subject、3 个 Collection，以及 Icon/Prop/Portrait 各 1 个 Pack。计划预计 30 次、最大 55 次 xAI 请求；服务端 usage 记录 37 次、25.8 美元等值 ticks。另有 1 次 `/images/edits` 传输失败，未返回媒体或 usage，按保守口径计为 38/55 次。真实 Provider 已停止，剩余 17 次未使用。

- Provider/Profile：`xai/default`
- 认证：Preview OAuth Device Code
- 图片模型：`grok-imagine-image-quality`
- Style revision：`397c05ac46ca8209`
- Subject：`ayla-ranger@023c1d579b0eecad`
- Godot：`4.6.3.stable.official.7d41c59c4`
- 原始隔离目录：`generated-assets/forge-stage3-real-20260805/`（Git 忽略，约 40 MiB）
- 机器摘要：[summary.json](artifacts/forge-stage3-real-20260805/summary.json)

## 预算执行

| 阶段 | 记录请求 | 成本 ticks | 说明 |
|---|---:|---:|---|
| StyleLock | 1 | 500,000,000 | 成功 |
| SubjectLock | 1 | 600,000,000 | 成功 |
| 三个 Collection anchor | 3 | 1,800,000,000 | 成功 |
| Icon Pack | 14 | 9,800,000,000 | 10 个首次请求，4 个定向第二次尝试 |
| Prop Pack 首次运行 | 2 | 1,400,000,000 | 第三个 item 传输失败；失败请求未进入 usage |
| Prop Pack 重跑 | 11 | 7,700,000,000 | 10 个首次请求，cook-pot 第二次尝试 |
| Portrait Pack | 5 | 4,000,000,000 | 5 个表情均首次通过 |
| 合计 | **37** | **25,800,000,000** | 保守请求口径为 38 |

费用和请求没有达到用户批准的上限。每个真实命令都设置了剩余请求数与剩余成本的双重熔断；没有静默切换 Provider、profile 或模型。

## 资产结果

### Style 与 Subject

- Style Board 在森林角色、背包和路标之间保持一致的苔绿、皮革、黄铜、轮廓和左上光照。
- Subject canonical 保持透明背景，Ayla 的兜帽、围巾、斗篷、皮甲和五官清晰。
- Style board SHA-256：`cb1c04850c367bcb454c32cf31b3fa9845f13ae2e92c62fa9622dddeb278d366`
- Subject canonical SHA-256：`e5f0b134254ddf0d44a8f49e9ecf2eca4de184e31e452c12afa054b444049745`

### Icon Pack — 未通过，未导出 Pack

- Job：`5ee7ae46-0364-4310-823d-201bb441fba5`
- 4 项 `game_ready`、3 项灰区、3 项第二次尝试后仍为 `regenerate`。
- 三个失败 item 均为 `edge_density_drift`；native-size 查看 herb bundle、compass 和 mushroom 没有裁切、多主体或明显风格破坏，显示 edge-density 对不同对象拓扑存在系统性误报。
- xAI 生成的 Collection anchor 同时包含背包、卷轴、药瓶和装饰底座，不符合“一个 canonical item”，但 Collection 创建阶段仍接受了它。
- 基础 consistency 报告为 `blocked`；collection consistency 报告却排除三个失败项，仅检查 7 项并返回 `game_ready`，报告间发生矛盾。
- Forge 正确没有导出错误 Icon Pack，也没有人工绕过硬失败。
- Contact sheet SHA-256：`71c1e34448d8150ff1b98c79072a46d9463d6e6a2fff6b0de84c3d7dd0504c92`

### Prop Pack — 视觉通过，需修复 review 状态链

- 首次 Job：`66273688-61c9-449a-a13c-02186fac8ace`，在第三个 item 因传输错误失败；前两个结果已落盘并计费，但当前 CLI 不能从失败 item 继续。
- 重跑 Job：`614fe9dc-1cb7-427d-964e-43eb613dae54`。
- 8 项直接 `game_ready`；cook-pot 与 foraging-basket 为 edge-density 灰区。
- native-size 人工复核确认两个灰区仍满足单对象、透明度、视角、材质、光照、落地和可用尺度，因此显式接受。
- Pack 校验、Godot 安装和 4.6.3 headless import 通过。
- Pack SHA-256：`b49695d7093eab1ade32437f1f1e049442a2ee6f2da811033f3b6c3b2de629fb`
- Godot install Job：`36b0d25e-fab8-4219-a7da-cf3aace500f5`
- 缺陷：review 后生成 Pack artifact，但没有把 Pack 注册回 Project Catalog；带 `--catalog-project` 的安装计划返回 `catalogProjectPath does not contain the requested Pack`。

### Portrait Pack — 机器假阳性，人工不接受

- Job：`80d44f8f-19d7-4766-a2ac-e6c9e0a981ed`
- 五个表情均首次被机器判为 `game_ready`；collection score 为 `0.948–0.982`。
- native-size 人工复核发现胸像、全身像、裁切比例和身体姿态明显漂移。对话 Portrait Set 应保持固定的头肩构图，因此这不是可接受的表情变化集合。
- Forge 已经导出并注册了该 Pack，构成“错误 Pack 导出数不为零”，真实发布门槛失败。
- Pack 校验及 Godot 安装仅证明交付结构可用，不表示视觉验收通过。
- Pack SHA-256：`4a1c2c300b7b73e13b64f81251ebd4cea3a1ab2c06cab036c0b7922675a479ce`
- Godot install Job：`f1007bbe-f052-4066-a524-37f74897fd76`
- Godot `.forge/assets.json` 错误地把 `portrait_set` 写成 `character`，虽然 Pack manifest 和 `forge_usage.json` 都是 `portrait_set`。

## Godot、安全与审计

- Prop 与 Portrait Pack 都通过 `forge pack validate`。
- 两个 Pack 均安装到外部 PNG；Godot 4.6.3 headless import 无错误。
- 最大 `.tscn` 为 314 bytes；无 `.tres/.tscn >= 1 MiB`。
- 无 `PackedByteArray`、内嵌 Image 或 `ImageTexture.create_from_image`。
- 凭据扫描 0 命中：没有 Authorization header、Bearer、access/refresh token、Device Code 或 API Key。
- 仅发现固定 host `api.x.ai`（失败 Job 的 API 端点）和 `godotengine.org`（Godot banner）；没有临时或签名媒体 URL。
- `project audit --scope all` 返回 clean、0 error，但只审计 Catalog 中的 Portrait。它无法发现 review 后未注册的 Prop，也无法表达人工否决的 Portrait，说明 audit 的输入闭包仍不完整。

## 发布阻断项

1. Collection anchor 缺少“单一 canonical item、无底座/场景/多对象”硬门禁。
2. Collection report 必须覆盖全部声明 item；不能排除基础 consistency 失败项后仍返回 `game_ready`。
3. `edge_density_drift` 对拓扑不同但风格一致的静态集合误报，需要按 asset kind 校准或改为更稳健的轮廓/复杂度特征，不能直接降低阈值。
4. 传输失败的静态 Job 不能从最后未完成 item 恢复，导致已付费结果无法复用。
5. `job review --accept` 导出 Pack 后没有原子注册 Catalog/Pack SHA，破坏 Catalog → Godot 状态链。
6. Portrait 缺少固定 framing、face/body scale、crop、pose 和身份语义门禁；现有颜色/边缘/尺度指标可产生严重假阳性。
7. Godot Project Manifest 将 `portrait_set` 错写为 `character`。
8. Project audit 需要审计 Catalog 与 Godot `.forge/assets.json` 的双向闭包，并支持人工 quarantine/rejection 状态。

## 本轮代码修复与回归

- 修复 `forge style create --plan-only` 被忽略、会直接执行的问题；现在返回未领取 token 和 `1/1` 请求估算，不创建 Job。
- `scripts/test-stage3-static.sh` 增加该回归合同。
- Stage 3 fixture 合同通过。

在上述阻断项修复并通过离线 fixture 后，才应申请新的真实 Provider 预算。当前结果不能用于“Stage 3 已具备商业一致性”的宣传。
