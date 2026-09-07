# Forge Character、Icon、Prop 真实验收阻断项修复计划

日期：2026-08-07  
状态：已实施  
真实 Provider：本计划禁止新增调用；只允许现有真实产物的只读诊断、零费用 replay、Pack/Godot 和安全审计。

## 1. 目标

修复 2026-08-06 Character、Icon、Prop 真实 xAI 验收暴露的四类发布阻断项：

1. 异步视频在提交响应缺少费用字段时丢失远程 request ID，无法恢复轮询；
2. Character canonical reference 具有完整身体但没有可见五官，仍进入付费动画阶段；
3. Icon/Prop 把异构 Style board 的 edge density 当作硬基线，系统性误报；
4. Prop 派生项复制 anchor 的物体拓扑，`crate` 变成第二个宝箱却未被拦截。

不降低 Alpha、画布、裁切、多主体、前景、锚点等硬质量门禁；不修改 Provider/model；不人工接受语义错误资产；不新增 xAI 费用。

## 2. 异步视频状态合同

- Provider 提交成功后先解析并持久化远程 request ID，再返回本地 ticket。
- 授权账本把本地 reservation ID 与 `providerRequestId` 绑定；绑定在进程重启后仍可读取。
- 轮询已提交的 ticket 不属于新请求，因此授权过期后仍允许恢复轮询。
- 只在终态响应取得 usage 后结算；终态仍无费用时标为 `ambiguous`，保守占用 reservation，但媒体仍可落地。
- 401 仍只允许一次刷新；轮询、下载和结算保持幂等，重复 poll 不重复记费。
- JobStore 在 `source/provider/tickets/` 保存非秘密 ticket 记录；每次状态变化原子更新 JSON 和 artifact SHA-256。
- 取消或超时保留远程 ID 与终态，不在日志、Pack 或 Godot provenance 保存授权头和临时 URL。

## 3. Character 可见身份合同

新增 `character-identity@1.0.0`：

- 在 canonical reference 完成 matting 后、任何 direction still/video 前执行；
- 默认要求上半部可见肤色区域以及至少两类封闭深色身份特征；
- 低分辨率 fixture 允许 1 px 特征，真实 1K 输出要求更稳定的局部像素规模；
- 明确声明 faceless、mask、helmet、robot 或 hidden-face 的角色可以版本化 opt-out；
- `blocked` reference 写入 `character-identity-report.json` 并立即停止下游 Provider 请求；
- SubjectLock 创建使用同一门禁，空白身份不能成为不可变基准；
- `forge job report` 对旧 Character Job 做只读本地诊断，不解析凭据、不修改源 Job、不产生请求。

## 4. 静态集合一致性合同

`consistency@1.5.0` 固定以下路由：

- Icon/Prop 的 Style board edge density 只作为 `edge_density_advisory`，不能单独触发 regenerate；
- Alpha、画布、裁切、主体数、前景尺度、锚点和严重调色板漂移仍保持硬门禁；
- 非 anchor item 记录 perceptual similarity 与前景 geometry/occupancy similarity；
- 只有对象 prompt 不共享身份名词，同时两项相似度都异常高时，才进入 `anchor_identity_leakage_review`；
- 该灰区不可自动导出，也不允许被 edge advisory 掩盖；
- Provider prompt 明确 anchor 仅提供外观风格，不得复制盖子、锁、把手、五金或对象拓扑，除非 item spec 明确要求。

## 5. 实施顺序

1. 扩展授权账本和 xAI 异步视频实现，加入重启、缺 usage、幂等结算测试。
2. 增加耐久 Provider video ticket artifact，并验证终态哈希。
3. 在 Character 与 SubjectLock 接入可见身份门禁；加入合成空白脸、合法五官和显式 opt-out 测试。
4. 把静态 edge density 改为诊断字段，加入 anchor 语义泄漏双指标门禁。
5. 对旧真实 Character reference 运行只读身份诊断；对 Icon/Prop 运行零费用 consistency replay。
6. 只安装达到 `game_ready` 的 Pack 到 Godot 4.6.3；Prop 的错误 crate 保持 review 阻断。
7. 运行完整 Rust、CLI、Stage 3、Pack、Godot 和敏感信息门禁，归档机器摘要。

## 6. 验收门槛

- 异步 request ID 跨 Provider 实例/进程重开可恢复；终态 usage 只结算一次。
- 缺终态费用不会丢 ticket 或媒体，账本保持保守 consumed。
- 空白脸在首个动画视频请求前阻断；真实旧 reference 的只读诊断必须命中。
- Icon 真实 PNG 零费用 replay 5/5 `game_ready` 并导出有效 Pack。
- Prop 真实 PNG 零费用 replay 只拦截 crate，其他四项通过；不得导出错误 Pack。
- 合格 Icon Pack 通过 Godot 4.6.3 安装和 headless import，使用外部 PNG。
- JobStore、授权账本、Pack 与 Godot 项目无 API Key、OAuth Token、Device Code、Bearer/Authorization header 或临时签名 URL。
- `cargo fmt`、Clippy、workspace tests、CLI product 和 Stage 3 contract 全绿。

## 7. 后续真实门槛

本计划不消耗新的 xAI 授权。代码与旧真实像素复验通过后，Character 仍需一张独立费用工单完成新的 canonical reference → 四动作视频 → Pack → Godot 端到端测试；在该测试完成前，不把“真实 Character 全链路已重新通过”写入发布声明。
