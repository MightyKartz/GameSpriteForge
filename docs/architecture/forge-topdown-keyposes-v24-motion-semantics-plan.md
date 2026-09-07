# Forge Top-down Key Poses V2.4 与运动语义门禁实施计划

日期：2026-08-08

## 目标

修复 `topdown-keyframes@2.3.0` 可以把“几乎静止但逐帧闪烁”的图片序列判为
`game_ready`，以及 locomotion 的足部残影只记录为诊断值、不阻断 Pack 的问题。

V2.4 新增独立工作流 `topdown-keyposes@2.4.0`。旧 V2.0–V2.3 Job、Pack、retry
和 replay 继续可读，不改变其不可变证据；通过新的零费用 Pack audit 给历史结果追加复审结论。

## 公共接口

```text
forge generate character --spec character-v2.json --wait --json
forge pack audit-motion --path <character.gsfpack> --json
forge job retry --id <job> --item <animation> --frame <0-3> --stage frame --json
```

Character spec 使用：

```json
{
  "workflow": { "id": "topdown-keyposes", "version": "2.4.0" },
  "generation": { "targetFrameCount": 4 }
}
```

- 正常四方向请求数为 16，最大 32。
- validation-only 单方向为 4，最大 8。
- retry/replay 继续创建子 Job；motion audit、matting、consistency 和 Pack audit 为零 Provider 请求。

## 生成合同

Walk 固定四个相位：

```text
left_contact -> left_passing -> right_contact -> right_passing -> loop
```

Idle 固定四个相位：

```text
neutral_hold -> inhale -> neutral_return -> exhale
```

- 每个相位都从不可变 DirectionLock 派生。
- 第一张参考始终是 DirectionLock；第二张是上一张已验收相位（首相位除外）；最后一张是当前方向、当前相位的 pose structure。
- Style 使用文本 descriptor，不占用图片参考槽。
- 禁止把前后两个生成结果同时交给 Provider 生成 in-between。
- `walk_right` 使用水平步幅；`walk_up`/`walk_down` 使用垂直深度、遮挡与左右脚接触区，不复用同一正弦 X 位移模板。
- 首发只导出四个清晰相位；八帧插值必须等序列级 Provider 能力完成后另行版本化。

## Motion Semantics V1

新增 `motion-semantics@1.0.0`：

- `lowerBodyDynamicDegree`：对齐后的髋部以下 Alpha 变化中位数。
- `opposingContactChangeRatio`：左右 contact 相位的下半身差异。
- `distinctPoseCount`：下半身轮廓达到最小分离度的姿势数。
- `phaseOrderScore`：与版本化 pose guide 的相位顺序一致性。
- `stableUpperBodyFlickerRatioMax`：稳定区域的逐帧 RGB 闪烁。
- `unsupportedLowerEdgeRatioMax`：下半身无法由相邻相位运动解释的边缘变化。
- `lowerEdgeOutlierRatioMax`：单帧异常边缘相对全周期中位数的离群程度。
- `footLobeCountMax`：脚部区域显著 Alpha 连通块数量。

硬失败原因：

```text
walk_motion_missing
walk_contact_poses_too_similar
walk_pose_diversity_missing
walk_phase_order_invalid
lower_body_edge_ghost
foot_lobe_count_exceeded
stable_upper_body_flicker
```

Motion hard failure 不得通过 `job review` 强制导出。Loop closure 仅在 motion semantics
通过之后决定最终质量，禁止用高 loop score 补偿动作缺失。

阈值使用 fixture 与冻结真实样本校准；报告保存 threshold profile，不能针对单个 Provider
结果临时放宽。

## Pack、Godot 与历史复审

- Job 写入 `character-motion-semantics-report.json`。
- Pack 写入同名报告并在 `assets.characterMotionSemanticsReport` 注册。
- `source.metadata.characterMotionSemantics` 保存 profile、verdict、报告路径和 SHA-256。
- `forge_usage.json` 复制相同 provenance。
- `forge pack audit-motion` 只读取 Pack 帧，输出单 JSON，不修改 Pack。
- 当前 Ayla V2.3 Pack 必须被新 audit 识别为运动不合格；原 Job/Pack 不原地修改。

## 测试门禁

- 静止但颜色闪烁的 walk：`walk_motion_missing`。
- 四个正确 contact/passing 相位：`game_ready`。
- contact 相位重复：`walk_contact_poses_too_similar`。
- 相位倒序：`walk_phase_order_invalid`。
- Walk Down 第三只脚或拖尾：`lower_body_edge_ghost` 或 `foot_lobe_count_exceeded`。
- 合法斗篷/手臂运动不因全身边缘变化误伤。
- Idle 的轻微呼吸通过；纯颜色闪烁失败。
- frame retry 只生成指定相位；local replay/audit 为零 Provider 请求。
- Pack schema、Godot 4.6.3、外部纹理、文本资源体积和安全扫描持续通过。

真实 Provider 重生成不包含在未授权的离线实施阶段。离线门禁通过后，先申请只生成
`walk_down`、`walk_right` 的 8 个相位，最多 16 次图片请求，再决定是否生成完整四方向。
