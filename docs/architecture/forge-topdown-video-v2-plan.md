# Forge topdown-video@2.0.0 修复计划

状态：已实施离线合同，真实 xAI 验收待单独授权。

## 决策

- 恢复“方向静帧 → xAI 图生视频 → 全视频候选抽帧 → 闭合周期选择 → Godot
  SpriteFrames”为 Character V2 稳定路径。
- `topdown-spritesheet@3.0.0` 保留为实验性 contact sheet/低成本对照，不作为移动
  动画默认作者工具。
- 不安装 Godot 扩展，不引入骨骼、零件、ControlNet 或新的 Provider。

## 公共合同

- 新工作流：`topdown-video@2.0.0`。
- 必须显式选择：
  - `topdown-orthographic@2.0.0`
  - `topdown-three-quarter@1.0.0`
- Character V1 的 `topdown@1.0.0` 与历史 Job 保持可读。
- 单方向 validation 正常为 2 次媒体请求，最多 4 次；本地 loop/matting/consistency
  replay 为零 Provider 请求。

## 流程

```text
StyleLock + SubjectLock
→ canonical identity
→ direction still
→ direction-still-preflight@1.0.0
→ image-to-video
→ ≤12 FPS / ≤96 candidate frames
→ matting + provisional body/foot alignment
→ loop@2.0.0 closed-cycle selection
→ shared normalization
→ direction/equipment/silhouette/quality gates
→ .gsfpack
→ native Godot AnimatedSprite2D/SpriteFrames
```

生成前门禁检查方向语义、顶部留白、角色尺度、水平中心、脚底基线、相对
SubjectLock 的尺度/位置漂移和边界裁切。失败静帧不得产生视频请求。

## 重试与来源

- `still`：重新生成静帧，并使视频和下游失效。
- `video`：复用已通过静帧，只生成或编辑视频。
- `loop`、`matting`、`consistency`：只重放本地阶段。
- Provider manifest、WorkflowGraph、Pack metadata 和 `forge_usage.json` 记录工作流、
  camera profile、选中周期、重试方法和 SHA-256，不保存凭据或临时 URL。

## 验收顺序

1. fixture 完整 Character → Pack → Godot。
2. 使用历史视频零费用重跑当前抽帧和硬门禁。
3. 单独授权 `walk_right` 真实 xAI A/B；通过后再验收严格背面的 `walk_up`。
4. 两个方向通过后才申请四方向真实验收。
