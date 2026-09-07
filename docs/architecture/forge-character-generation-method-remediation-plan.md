# Forge 角色生成方法论纠偏工单

状态：待执行
创建：2026-08-11
执行方：Codex
当前分支：`codex/stage3-static-assets`（工作区有大量未提交改动，见「硬约束」）

## 0. 背景与判断依据

对现有角色图/角色动画生成路径（`topdown-*` V1–V8）做了一次外部对标复核，对标对象：
Sprite Sheet Diffusion（arXiv 2412.03685）、PixelLab、SpriteCook、Scenario、LayerDiffuse、
Astropulse/pixeldetector、Retro-Diffusion/pixel-art-fixer、facebookresearch/AnimatedDrawings。

结论摘要：

- **工程外壳（不可变锁、子 Job、零请求本地重放、来源链、预算守卫、`motion-semantics` 门禁、
  Godot 原生资源边界）正确且强于全部对标产品，本工单不得削弱任何一项。**
- **生成方法论选错了默认路径**：链式逐方向派生 + 图生视频抽帧，成本比对标高一个数量级
  （V8 单角色 12 次付费请求 vs PixelLab/SpriteCook 1 次），而 Stage 1 真实验收
  通过率 0/3、花费 $132.10。
- **像素级实现有确定错误**，且很可能是「离线全绿、真实全挂」的直接原因：交付路径用
  Lanczos3 缩放、无调色板量化/锁定、身份度量以 64-bit pHash 为主。

本工单按「先离线零成本、后需授权」分级。P0 全部离线可验证，不产生任何 Provider 费用。

## 1. 硬约束（违反即判定工单失败）

1. **保留用户未提交改动。** 当前工作区有 60 个已修改文件（约 +29,190 / −2,149）与 173 个
   未跟踪文件。禁止 `git checkout .`、`git stash`、`git clean`、`git reset --hard`，
   禁止「顺手回退」任何与本工单无关的改动。开工前先保存
   `git status --short` 快照到证据目录。
2. **零真实 Provider 调用。** 不得设置 `FORGE_REAL_PROVIDER_ACCEPT`。所有验收使用 fixture。
   P1 的真实 xAI 验收明确不在本工单范围内，需用户单独授权预算后另开工单。
3. **不得降低任何现有阈值来让测试变绿。** 若新指标或新路径达不到门槛，如实记录未达标结论。
   这是本仓库既有纪律（见 Stage 1 未晋级记录），必须延续。
4. **新能力一律走 feature flag，默认关闭。** 默认 release surface 的 `--help` 输出必须逐字节不变。
5. 遵守 `.agents/skills/forge-dev/SKILL.md` 全部核心规则：CLI JSON 信封、耐久 JobStore、
   一次性 plan token、稳定错误码、Provider 中立、凭据零泄漏、源 Job/源 Pack 不可变、
   Godot 外部纹理且 `.tres/.tscn` < 1 MiB 无内嵌图像。
6. **不删除、不回退、不重构 V1–V8 任何现有工作流。** 它们的既有测试必须保持全绿，
   既有 Pack 产物必须逐字节可复现。

## 2. 范围

| 阶段 | 内容 | 成本 | 是否需授权 |
| --- | --- | --- | --- |
| P0-A | 像素交付管线修复（网格感知下采样 + 调色板锁定） | 离线零成本 | 否 |
| P0-B | 一致性/身份度量可信化 + 标定集 | 离线零成本 | 否 |
| P1 | 单次网格生成路径 `topdown-grid@9.0.0`（仅 fixture 验收） | 离线零成本 | 否 |
| P2 | Provider 选型 ADR + 适配器接口对齐（不接真实 key） | 离线零成本 | 否 |
| P3 | 确定性绑定/形变路径可行性 spike（只出报告） | 离线零成本 | 否 |

非范围：真实 xAI 验收、Unity/Unreal、MCP、新增付费 Provider 接入、桌面 UI、
Stage 3 静态资产（`collection-assets`）的功能变更。

---

## P0-A 像素交付管线修复

### 问题

1. **交付路径使用 Lanczos3 重采样**，产生振铃与半透明抗锯齿边缘，破坏像素网格；
   配合 Godot 侧 `sampling: nearest` 会表现为糊边与 1px 光晕。
   已确认调用点：
   - `packages/core/src/asset_project.rs:2013`（`normalize_matted_static_image`）
   - `packages/core/src/animation_sheet.rs:152`、`:355`（`normalize_repaired_frame_to_contract` 等）
   - `packages/core/src/automation/runner.rs:11833`（`run_generate_character_pack` 内）
2. **全仓库无调色板量化或强制映射**（grep 无 `quantize` / `posterize`）。
   `palette` 仅出现在度量代码里（`assess_consistency` 的 `palette_overlap`），
   不用于强制。这是社区公认的头号一致性杀手（palette drift）。

### 要求

**A1. 区分「交付缩放」与「分析缩放」，只改前者。**

以下分析用途的重采样**保持原样，不得修改**——改动会静默移动既有阈值基线：

- `packages/core/src/quality/loop_selection.rs:752`（Nearest，运动分析）
- `packages/core/src/quality/source_cycle_sampling.rs:290`（Triangle，周期采样）
- `packages/core/src/asset_project.rs:3239`（Triangle，9×8 pHash 前置缩放）

本阶段**在范围内**的交付路径：上述三处 Lanczos3 调用点中服务于角色交付图与角色 sheet 的分支。
**本阶段不在范围**（列出但不改，理由写进报告）：`packages/core/src/collection.rs:1418`
（`reframe_portrait_to_anchor`，Portrait 非像素网格资产）、`packages/core/src/world.rs:1437`。
若 Codex 认为应一并纳入，先出证据再改，不要默认扩大范围。

**A2. 实现网格感知下采样。**

新增 `packages/core/src/pixel_grid.rs`：

- 检测源图的像素块网格（参考 Astropulse/pixeldetector 与 Retro-Diffusion/pixel-art-fixer
  的做法：用自适应 k-means 量化**仅用于定位网格**，颜色输出与网格定位解耦）。
- 按检测到的网格做众数/最近邻下采样到目标画布；检测失败时回退到纯 Nearest，
  并在报告中标记 `grid_detection: fallback`，不得静默回退。
- 输出必须满足：alpha 只有 0 或 255（阈值可配置但默认二值），无半透明羽化边。

**A3. 实现 PaletteLock。**

- 新增 `PaletteLockV1`：有序 RGB 条目 + 最大色数 + 来源（Style board 或 canonical idle）
  + 派生算法与版本 + SHA-256。
- **StyleLock 不可变**：禁止就地修改已有 `style-lock.json`。PaletteLock 写入独立路径
  `.forge/palette/<revision>/palette-lock.json`，内含被引用的 style revision 与 board SHA。
- 派生必须确定性：相同输入产出相同 revision 与相同调色板顺序。
- 交付阶段对所有输出图强制最近邻颜色映射到锁定调色板（在 A2 的下采样之后执行）。
- 新增 `schemas/palette-lock.schema.json`。不得修改任何现有 schema 的既有字段。

**A4. Feature 与兼容性。**

- 新增 feature `pixel-delivery-v2`，默认关闭。
- feature 关闭时，所有现有工作流的 Pack 产物必须**逐字节不变**（用 SHA-256 对比证明）。

### P0-A 验收

必须新增测试并全部通过：

1. 交付图颜色数 ≤ PaletteLock 色数上限。
2. 交付图 alpha 通道无中间值（或在声明阈值内），无 1px 羽化边。
3. 相同输入两次运行产出逐字节相同的交付图与相同 PaletteLock revision。
4. 网格检测失败时正确回退并在报告里标记，不静默通过。
5. `pixel-delivery-v2` 关闭时，至少三个现有工作流（建议 V6、V7、V8）的 Pack SHA-256 不变。
6. 新增 `scripts/test-pixel-delivery.sh`，全离线。

---

## P0-B 一致性/身份度量可信化

### 问题

当前身份判定核心在 `packages/core/src/asset_project.rs:2213` `assess_consistency`，
identity 信号是：

```rust
1.0 - (candidate.perceptual_hash ^ reference.perceptual_hash).count_ones() as f32 / 64.0
```

64-bit pHash 对「同一角色换朝向」几乎无判别力（结构本身变了），对「颜色轻微漂移」又不敏感，
正好把两件要区分的事都判反。文献标准是 SSIM / PSNR / LPIPS / DINO 特征相似度四件套
（Sprite Sheet Diffusion 即用此组合）。

**这是「离线 fixture 全绿、真实运行全挂」最可能的技术根因：门禁测量的量与人眼判定的量不是同一个。**

### 要求

**B1. 先建标定集，再改指标。顺序不可颠倒。**

- 从既有真实运行产物构造带标签的标定集。可用素材：`docs/qa/artifacts/` 下 30+ 个
  `*-real-*` 目录（254 张 PNG）与 `generated-assets/` 下的真实运行输出。
- 标签优先从既有 QA 文档中已记录的人工判定恢复（例如 Stage 1 真实门槛记录的
  14/15/22 帧 `game_ready`）。若逐帧标签无法从文档恢复，做一次人工标注：
  生成 native-size contact sheet，人工标注 pass/gray/fail，把标注结果与标注依据一并入库。
- 标定集与标签写入 `docs/qa/artifacts/forge-identity-metric-calibration-<date>/`，
  包含机器可读 JSON。遵守 `docs/qa/forge-qa-artifact-policy.md`（不提交完整 JobStore、
  原始视频、大 Pack、token cache）。

**B2. 先测量现状。**

用标定集跑现有 pHash 指标，输出 ROC / 分离度 / 混淆矩阵。
**这一步的产出本身就是交付物**：它把「门禁骗人」从判断变成可测量事实。

**B3. 实现 IdentityMetricV2。**

新增 `packages/core/src/quality/identity.rs`，多信号融合，建议信号（Codex 可增删，但必须做消融）：

- 前景掩膜后的梯度方向直方图（HOG 类）相似度
- 区域化（建议 3×3）调色板 EMD
- 锚点对齐后的轮廓 IoU
- 现有 pHash 与 `occupancy_profile_similarity`（`asset_project.rs:2199`）降级为辅助权重

要求：

- 纯 Rust、无外部模型权重、完全离线、确定性。
- 必须输出**消融表**：每个信号单独与组合的分离度，证明每个信号都有贡献。
- 保留 DINO/LPIPS 作为可选 component 插槽——`packages/core/src/component.rs:181`
  已有 `VisionOperation::PerceptualDistance` 的 fixture 实现，接口对齐即可，
  不要引入未审查权重（`forge component install` 绝不替换未审查权重的规则不变）。

**B4. Feature 与兼容性。**

- 新增 feature `identity-metric-v2`，默认关闭。关闭时 `assess_consistency` 行为不变。
- 开启时新指标与旧指标**同时**写入报告，便于对比，不要直接替换字段。
- 新增 `schemas/identity-metric-report.schema.json`。

### P0-B 验收

1. 标定集与标签入库，标注方法可复现。
2. 现状测量报告：pHash 在标定集上的分离度数字。
3. IdentityMetricV2 在同一标定集上的分离度显著优于 pHash，附消融表。
4. **若分离度不达标，如实记录未达标，不得调整阈值或裁剪标定集来制造通过。**
5. `identity-metric-v2` 关闭时既有测试全绿、既有报告字段不变。
6. 新增 `scripts/test-identity-metric.sh`，全离线。
7. 证据文档 `docs/qa/forge-identity-metric-calibration-<date>.md`。

---

## P1 单次网格生成路径 `topdown-grid@9.0.0`

### 依据

- PixelLab 的 8 方向 Pro 工具**一次请求出一张 3×3 网格**，8 个朝向同时生成；
  其文档把「每次 45° 增量旋转并更新参考」明确标注为 error accumulation 来源。
- Forge 自己在 V2.4→V2.5 已诊断出链式参考传播漂移，修法是「从 DirectionLock 独立派生」——
  方向正确但只走了一半。终点是**同一次前向传播内生成全部朝向**，靠模型内部注意力保证互一致。
- `topdown-spritesheet@3.0.0` 真实验收的失败描述是「身份与取景很强，但运动相位塌成近似重复姿势」——
  这是缺姿势条件的症状，不是网格法的症状。身份与取景强恰好证明网格法在一致性上有效。
- Grok Imagine 单请求支持最多 3 张参考图，现有链式编辑大多只用 1–2 张，未吃满。

### 要求

**C1. 新工作流，不改旧工作流。**

- feature `grid-generation`，默认关闭。工作流 ID `topdown-grid@9.0.0`。
- 可视图谱：

```text
prompt (+ StyleLock / 可选 SubjectLock 描述)
└── direction_grid            一次请求，2×2 网格 = front / back / right / left
    └── 确定性切片 → 4 个方向 idle 节点
        └── action_grid × 4   每动作一次请求，2×2 四相位网格
            └── 确定性切片 → 每动作 4 关键帧
```

- 请求预算：**预计 5 次图像请求，上限 10，视频请求恒为 0**。
  （对比 V8 的 8 图 + 4 视频 = 12 次，上限 24。）
- 每次 action_grid 请求最多使用 3 张参考：canonical front idle + 对应方向 idle + 姿势引导图。

**C2. 复用而非重写。**

- 切片、对齐、清理必须复用 `topdown-spritesheet@3.0.0` 的既有实现，不要另起一套。
- 关键帧语义沿用 V2.4 的 contact / passing / opposite contact / opposite passing。
- **`motion-semantics@1.1.0` 门禁必须保留并作用于本路径**——这是仓库独有的护城河，
  对标产品与 SSD 论文都没有等价物。
- 保留 V8 的两阶段授权：direction_grid 完成后停在 `awaiting_review`，
  写 approval 绑定源 Job 与全部节点 SHA-256，approval 通过后才允许 action_grid。

**C3. 单元格级重试。**

- `job retry --frame 0-3` 只重生成该单元格，其余单元格 PNG 逐字节复用。
- 重试创建子 Job，源 Job 不可变。

**C4. 姿势引导的处理。**

现有 `write_keypose_guide`（`packages/core/src/automation/runner.rs:6270`）画的是
`draw_disc` + `draw_line` 的两色火柴人 PNG，作为**参考图**传给通用编辑 API。
文献中的姿势条件是注入 latent 的（SSD 的 Pose Guider 用 4 层卷积把姿势编码对齐到
noise latent 后直接加到 noisy latent；ControlNet 同理），当作参考图传给不认识该语义的
模型，效果无保证——这很可能是 V2.4 `walk_right` 近乎静止的原因。

本阶段要求：**保留姿势引导，但做成可一键关闭的独立开关**，并在报告中说明该开关的存在理由，
以便将来真实验收时可低成本 A/B。不要在本阶段就删除它。

### P1 验收（全部 fixture，零真实请求）

1. 切片正确性：网格单元格边界、缺失单元格、重复单元格均被检测。
2. plan 阶段的请求估算准确：5 预计 / 10 上限 / 0 视频。
3. 两阶段授权正确：未 approval 时无法进入 action_grid；approval 证据缺失/被改/被移动时 fail-closed。
4. 单元格重试：只重生成目标单元格，其余逐字节复用，Provider 请求数正确。
5. `motion-semantics@1.1.0` 对本路径生效，能拦住相位塌陷的 fixture。
6. Pack 校验通过，Godot 4.6 安装通过，`.tres/.tscn` < 1 MiB 且无 `PackedByteArray`。
7. 凭据扫描：Pack、日志、CLI JSON、Godot 工程内无 key / token / 临时 URL / 绝对 JobStore 路径。
8. 新增 `scripts/test-grid-generation.sh`，全离线。
9. `grid-generation` 关闭时默认 `--help` 输出逐字节不变。
10. 证据文档 `docs/qa/forge-topdown-grid-v9-offline-<date>.md`，
    **必须显式写明「真实 xAI 验收未执行，不得据此宣称真实模型门槛通过」。**

---

## P2 Provider 选型 ADR（调研 + 接口对齐，不接真实 key）

### 问题

架构宣称 Provider 中立，实际锁死 xAI / Aurora：面向写实、无原生透明、无 LoRA/ControlNet、
非像素专用模型。在不合适的模型上反复打磨编排层是错误的投入方向。

### 要求

产出 `docs/architecture/forge-provider-selection-adr.md`，对下列候选做结构化对比：

| 候选 | 需要评估的维度 |
| --- | --- |
| xAI Grok Imagine（现状基线） | 原生透明 / 参考图数量 / 像素网格保真 / 多视角网格能力 / 价格 / 许可证 |
| PixelLab API | 同上 + 骨架条件化 / 默认透明背景 / 8 方向单请求网格 |
| Retro Diffusion | 同上 + 是否可自托管 |
| Gemini 3.x Flash Image | 同上（SpriteCook 的默认模型） |
| FLUX.2 多参考 | 同上（支持 2–10 张参考注入） |
| Qwen-Image-Edit | 同上（专攻编辑漂移） |

额外要求：

- 评估 LayerDiffuse 式**原生透明生成**替代现有「生成后抠图」的可行性。
  依据：LayerDiffuse 的用户研究中 97% 的情况用户更偏好原生透明输出而非事后抠图；
  现有 `keyframe-background-cleanup@1.3.0` 本质是在补这个洞。
- 实现**一个**新 adapter 的接口对齐 + fixture 实现（不接真实 key、不发真实请求），
  以此证明 core 不需要为换 Provider 而改动。若发现 core 需要改动，把改动点列进 ADR。
- ADR 必须给出明确推荐与理由，不要只罗列。

---

## P3 可选 spike：确定性绑定/形变路径（只出报告）

时间盒，不出实现。评估「AI 只出基础姿势 + 确定性绑定/形变生成动画帧」的可行性，
参考 facebookresearch/AnimatedDrawings（单图 → 分割 → 自动绑定 → BVH 驱动）。

对 top-down 四方向行走这类高度程式化的循环，确定性绑定的潜在优势：完全可控、
边际成本为零、时序 100% 一致、天然契合仓库既有的确定性哲学。

产出 `docs/architecture/forge-deterministic-rig-spike.md`，给出可行/不可行的明确结论与依据。

---

## 3. 执行顺序

P0-A 与 P0-B 可并行。**P1 必须在 P0-A、P0-B 之后**——否则新路径仍然会被不可信的指标评判，
重蹈「离线全绿、真实全挂」的覆辙。P2 可随时并行。P3 最后或跳过。

## 4. 每阶段的发布门槛

每个阶段独立提交、独立 PR。每个 PR 合并前必须通过：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
scripts/test-cli-product.sh
```

加上该阶段自己的 focused 脚本（`test-pixel-delivery.sh` / `test-identity-metric.sh` /
`test-grid-generation.sh`）。

## 5. 交付物清单

- [ ] `packages/core/src/pixel_grid.rs` + `PaletteLockV1` + `schemas/palette-lock.schema.json`
- [ ] `packages/core/src/quality/identity.rs` + `schemas/identity-metric-report.schema.json`
- [ ] `topdown-grid@9.0.0` 工作流（feature `grid-generation`）
- [ ] 三个 focused 测试脚本
- [ ] `docs/qa/forge-identity-metric-calibration-<date>.md` + 标定集 artifacts
- [ ] `docs/qa/forge-pixel-delivery-v2-offline-<date>.md`
- [ ] `docs/qa/forge-topdown-grid-v9-offline-<date>.md`
- [ ] `docs/architecture/forge-provider-selection-adr.md`
- [ ] `docs/architecture/forge-deterministic-rig-spike.md`（P3，可选）
- [ ] 更新 `docs/qa/forge-complete-visual-implementation-status.md` 的阶段总览与变更记录

## 6. 需要用户决策的点（Codex 遇到时停下来问，不要自行决定）

1. **是否愿意更换或增加 Provider。** 涉及成本、许可证、供应商依赖。P2 只出 ADR 与推荐，
   实际接入需用户拍板。
2. **P1 的真实 xAI 验收预算。** 本工单不含真实调用。grid 路径预计 5 请求/角色，
   若要真实验收需用户批准请求数与费用上限。
3. **P0-A 是否扩展到 Portrait 与 World 资产。** 本工单默认只做角色交付路径。
4. **若 P0-B 标定集显示 IdentityMetricV2 仍不达标**，是否接受引入需要模型权重的
   DINO/LPIPS component（涉及既有的 license 与标定审查阻塞）。

## 7. 明确不做

- 不删除、不回退、不重构 V1–V8 任何现有工作流。
- 不改变默认 release surface。
- 不做任何真实 Provider 调用。
- 不降低任何现有阈值。
- 不触碰工作区中与本工单无关的未提交改动。
- 不把 P1 的 fixture 通过表述为真实模型门槛通过。
