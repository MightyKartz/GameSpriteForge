# Forge topdown-keyposes V2.4 / Motion Semantics 离线验收

日期：2026-08-08

## 结论

- 新增 `topdown-keyposes@2.4.0`：每个动作固定四个显式关键姿势，6 FPS，正常 16 次、最大 32 次图片请求。
- 新增 `motion-semantics@1.0.0` 硬门禁，区分真实运动与颜色闪烁，并检测步态相位、足部残影、第三足轮廓和稳定区域闪烁。
- 新增零费用命令：`forge pack audit-motion --path <pack> --json`。
- 旧 V2.3 Pack 的复审结论为 `blocked`，与人工观察一致；未调用 Provider、未产生费用。

## 真实 V2.3 Pack 零费用复审

源 Job：`8b5248a0-c944-4c84-a4a9-80d020ae0396`

源 Pack：`Ayla-Ranger-Keyframes-V2-3.gsfpack`

| 动作 | 有效姿势数 | 下半身动态 | 相位分数 | 上半身闪烁最大值 | 最大足部轮廓数 | 结论 |
|---|---:|---:|---:|---:|---:|---|
| idle | 2 | 0.0134 | 0.5143 | 0.0162 | 3 | blocked |
| walk_down | 6 | 0.0478 | 0.4430 | 0.0205 | 3 | blocked |
| walk_right | 2 | 0.0267 | 0.2859 | 0.0151 | 3 | blocked |
| walk_up | 3 | 0.0210 | 0.1071 | 0.0374 | 2 | blocked |

关键诊断：

- `walk_right`：接触姿势过于相似、姿势多样性不足、相位错误。
- `walk_up`：运动量不足、接触姿势相似、相位错误。
- `walk_down`：相位错误、下缘残影、第三足轮廓。
- 稳定区域颜色指标排除了轮廓重采样与局部纹理边界；该指标没有把本 Pack 的闪烁单独判为硬失败，视觉闪烁仍由姿势不足和相位错误解释。

## 离线回归

- `motion_semantics` 单元夹具：正确四相位、静止但颜色闪烁、乱序相位、第三足/残影、轻微 idle 呼吸、冻结 idle。
- Fixture Provider 合同：V2.4 完整四方向计划为预计 16 次、最大 32 次；16 个 Provider 节点；后续姿势只依赖 DirectionLock 与上一帧；无双邻帧 AI 插值。
- Fixture 的四个动作全部通过 motion、collection consistency 与兼容 silhouette 门禁，Pack 验证和 Godot 安装通过；`forge_usage.json` 保留 motion provenance。
- 旧 `topdown-keyframes@2.3.0` 合同继续保留并回归。

## 尚未执行

本轮没有新的真实 xAI 授权，因此没有生成新的 V2.4 实图。进入真实验收前，应先使用单方向 4 帧、最大 8 次请求验证 `walk_down`；通过后再授权四方向 16/32 次。
