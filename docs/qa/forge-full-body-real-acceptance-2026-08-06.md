# Forge full-body Portrait 真实 xAI 验收（2026-08-06）

## 结论

本轮真实生成证明：`full_body@1.0.0` 已能阻止“半身图被当作完整全身图”的原始 P0，
五个表情的真实结果均完整包含头部、躯干、双腿和双脚，且单项 geometry verdict 全部为
`game_ready`。但是整套自动验收 **未通过**：Collection anchor 带法杖、轮廓更宽且面部
被兜帽阴影遮挡，而五个表情按规格移除了法杖，因此全部被现有 anchor-relative framing
指标标记为 `portrait_framing_drift`，Job 停在 `awaiting_review`。

严格门槛要求无需人工审核即可导出，因此本轮没有人工接受、没有导出 Pack、没有安装
Godot，也没有继续消耗剩余五次重试额度。当前结论是：

- 缺腿假阳性的安全问题已经修复并通过真实像素验证。
- full-body Portrait 的端到端可用性仍被 Collection anchor 语义和 framing 基线设计阻断。
- 不能把本轮结果宣称为 full-body Portrait 自动发布门槛通过。

机器摘要：
[`artifacts/forge-full-body-real-20260806/summary.json`](artifacts/forge-full-body-real-20260806/summary.json)。

## 授权与费用

用户授权范围为复用现有 StyleLock/SubjectLock，只生成一个 full-body Collection anchor 和
`neutral/happy/angry/hurt/surprised` 五个全身表情；总计最多 11 次图片请求和
11,000,000,000 cost ticks，禁止生成其他资产。

Forge 将额度拆成两个持久授权账本，以避免锚点重试挤占表情预算：

| 授权 | 目标 | 请求上限 | 费用上限 | 实际请求 | 实际费用 |
| --- | --- | ---: | ---: | ---: | ---: |
| `fullbody-20260806-anchor` | `collection:full-body-portraits` | 1 | 1,000,000,000 | 1 | 600,000,000 |
| `fullbody-20260806-portraits` | 五个表情，每项最多 2 次 | 10 | 10,000,000,000 | 5 | 4,000,000,000 |
| 合计 | 仅上述六个目标 | **11** | **11,000,000,000** | **6** | **4,600,000,000** |

- Provider/Profile：`xai/default`
- 认证：OAuth Device Code Preview；健康检查通过，本轮未重新登录。
- 模型：只允许 `grok-imagine-image-quality`
- 五个未使用的表情重试请求保持未消费。
- 没有生成 Icon、Prop、Character animation、地图或其他资产。

## 锁与 Job

- 复用 Style revision：`397c05ac46ca8209`
- 复用 Subject：`ayla-ranger@023c1d579b0eecad`
- 新 Collection：`full-body-portraits@r-888de4eb3c4cd763`
- Collection Job：`5a71a202-ede9-492f-b127-fcc451c4586c`
- Portrait Job：`46eb19a1-bd47-47be-b9f0-09e27019f667`
- Collection plan：预计/最大 `1/1`，实际 1。
- Portrait plan：预计/最大 `5/10`，实际 5。

## 真实视觉结果

### Collection anchor

[原始 anchor](artifacts/forge-full-body-real-20260806/anchor.png)

- 头部至双脚完整，顶部/底部安全边距约为 11.1%/11.3%。
- `lowerBodyPresenceProxy=0.8527`，geometry verdict 为 `game_ready`。
- 人工检查发现法杖和被阴影遮挡的脸；规格已经要求无额外对象和可识别身份，但
  `collection-anchor@1.1.0` 只把与主体连通的法杖视为主体轮廓，未能阻止该语义缺陷。

### 五个表情

[Contact sheet](artifacts/forge-full-body-real-20260806/contact-sheet.png)

- 五张都清楚显示双腿和双脚，没有半身裁切、平台、场景或多主体。
- 五张之间的 pairwise mean similarity 为 `0.9925437`。
- 相对 medoid similarity 为 `0.9903648–1.0`，人工检查身份、服装、比例和姿态稳定。
- 每张 geometry verdict 均为 `game_ready`，bottom margin 为 `6.54%–9.96%`。
- 所有单项最终仍为 `awaiting_review`，共同原因是
  `portrait_framing_drift`；`neutral` 另有非阻断的
  `edge_density_advisory`。
- anchor-relative framing similarity 只有 `0.8930356–0.8960882`。主因是 anchor
  的法杖扩大了宽度和轮廓，而五个规范表情均没有法杖；这不是五张之间的构图漂移。

## 为什么没有使用剩余重试

剩余五次额度允许每个表情再请求一次，但失败原因对五项完全一致，且来自同一个坏 anchor。
现有定向重试提示会要求结果靠近 anchor，继续请求更可能重新引入法杖或损伤已经稳定的身份，
不能修复基线语义。为了避免用 Provider 费用掩盖确定性门禁缺陷，本轮停止于 5 次表情请求。

也没有执行 `job review --accept`。本轮发布门槛明确要求自动 `game_ready`，人工接受会
改变验收问题而非解决问题。

## 安全与交付检查

- Job/Plan/Authorization/Project/Godot 目录凭据扫描：0 个文件命中。
- 临时或签名媒体 URL 扫描：0 个文件命中。
- 请求账本只保存非秘密的目标、模型、Job、状态与 cost ticks。
- Pack 数量：0；质量门禁按预期阻止导出。
- Godot 安装：未执行，因为没有合格 Pack。

## 后续修复建议

下一步应先修确定性语义，不应继续付费盲重试：

1. full-body Portrait 的集合 framing 基线改为首个通过硬 geometry 的 `neutral` 项或集合
   medoid；Collection anchor 只作为生成参考，不作为全轮廓强一致性真值。
2. framing 比较使用人体主体的中心、头顶、脚底、身体宽高和安全边距，降低手持道具、披风
   等附属轮廓对构图判断的污染。
3. Collection anchor 增加“脸部可见、禁止手持道具”的语义证据；在没有经过审计的视觉
   组件前，无法可靠验证时应进入 `awaiting_review`，不能写成 `game_ready`。
4. 修复后直接对本轮五张真实像素执行零费用 consistency replay。只有零费用重放全部自动
   `game_ready`，才需要生成一个新的真实样本确认泛化；不应先花掉当前剩余额度。

