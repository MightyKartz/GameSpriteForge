# Forge PortraitBaseLock 与局部一致性 P0 实施计划

## 目标

解决 2026-08-06 真实 full-body Portrait 验收暴露的三类问题：

1. Subject canonical 与 Collection anchor 的围巾颜色冲突，Provider 在两者之间随机取舍。
2. 每个表情都重新生成完整人物，导致肤色、服装颜色和局部纹理漂移。
3. 整图 palette/hash 指标无法发现占比很小的脸部、肤色和围巾局部缺陷。

本计划不调用真实 Provider。冻结的五张 xAI 图片作为只读回归输入；所有生成测试使用
fixture，所有真实像素复验使用零费用 consistency replay。

## 产品契约

- Portrait V1 继续使用现有独立参考行为，保持兼容。
- Portrait V2 必须包含且以 `neutral` 为第一项。
- StyleLock、SubjectLock 与 Collection anchor 只用于生成 neutral。
- neutral 必须通过格式、Alpha、单主体、framing 和 geometry 硬门禁，之后写入不可变
  `PortraitBaseLockV1`。
- happy/angry/hurt/surprised 均从同一个 neutral 星形派生；不得从上一个表情链式派生。
- 对 xAI 等没有 mask 能力的 Provider，每个表情请求只发送 neutral 这一张
  `edit_target`，不再发送角色互相冲突的三图片数组。
- Provider 候选仍完整落盘并计算 SHA-256；游戏输出只接纳确定性 face scope 内的像素，
  face scope 外必须与 neutral 完全一致。
- 任何局部硬失败都不能人工强制导出 Pack。

## 新增类型

### PortraitBaseLockV1

- profile：`portrait-base-lock@1.0.0`
- assetId、itemId（固定 neutral）
- base PNG 路径与 SHA-256
- Provider/profile/model、Style/Subject/Collection revision
- framing profile 与确定性 face scope
- 创建时间

### PortraitConsistencyReportV1

- profile：`portrait-local@1.0.0`
- base item 与 base SHA-256
- 每项：
  - Provider 原图在 face scope 外的变化比例
  - 最终输出在 face scope 外的变化比例
  - face scope 内变化比例
  - 基于 neutral 皮肤样本的 CIELAB 色差
  - face edge-density ratio
  - face perceptual similarity
  - verdict 与原因

## 确定性 face scope

- 从 neutral 的 Alpha bbox 推导一个中心椭圆区域。
- full-body profile 使用 bbox 上部的脸部椭圆；V1 dialogue bust 暂不启用该新流程。
- 椭圆边缘采用固定 feather；mask、坐标和实现版本写入 Lock。
- 表情输出先与 neutral 对齐到同一画布，再仅在 mask 内合成。
- mask 外每个 RGBA 像素必须与 neutral 完全相同。

## 门禁

- neutral：沿用 geometry 与硬质量门禁，不用 Collection anchor 的 palette/framing
  similarity 决定最终集合一致性。
- 非 neutral：
  - Provider 原图必须仍满足完整 geometry，防止从 bust 中截取脸部。
  - 最终 mask 外变化比例必须为 0。
  - 肤色 CIELAB 平均色差小于等于校准阈值。
  - face edge-density 与 perceptual similarity 必须落入版本化区间。
  - 表情区域必须产生最低变化，防止原样复制 neutral。
- Portrait V2 不再使用通用 Collection anchor-relative report 决定 verdict；改用
  Portrait local report。CollectionLock 仍作为初始构图 provenance 保留。

## Pack、Godot 与回放

- Job 写入 `portrait-base/neutral.png`、`portrait-base-lock.json` 和
  `portrait-consistency-report.json`。
- Pack 附加上述两个 JSON，并在 source metadata 保存 profile、base SHA 与 base item。
- Godot `forge_usage.json` 保存非秘密 PortraitBase provenance。
- `job retry --stage consistency` 从源 Job 的 normalized PNG 重新建立 neutral base、
  重新合成其余表情并运行新门禁，Provider 请求必须为 0。
- 原 Job、原图片与原 Pack永不修改。

## 验收

1. fixture 证明 V2 Provider 请求顺序为：
   - neutral：Style + Collection + Subject
   - 其余四项：只有 neutral edit target
2. fixture 注入围巾/身体漂移后，最终输出的非脸部像素与 neutral 完全相同。
3. fixture 注入肤色漂移与脸部伤痕后，本地报告分别阻断。
4. V1 Portrait、Icon、Prop、Equipment、Decal 合同不变。
5. 对 Job `46eb19a1-bd47-47be-b9f0-09e27019f667` 的五张真实 PNG 零费用 replay：
   - neutral 与 happy 可接受；
   - angry/surprised 的围巾漂移被确定性消除；
   - 肤色漂移和 hurt 新增脸部伤痕必须被局部门禁发现；
   - 在缺陷仍存在时不得导出 Pack。
6. Stage 3、CLI product、workspace、Godot 4.6 与凭据扫描全部通过。

