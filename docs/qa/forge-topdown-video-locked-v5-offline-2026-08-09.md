# Forge topdown-video-locked V5 离线验收

日期：2026-08-09  
结论：**离线实现与 Godot 契约通过；真实 xAI 视觉验收尚未执行。**

## 验收范围

本次验收覆盖 `topdown-video-locked@5.0.0`：

1. 创建或复用一张四方向 `DirectionLock`。
2. 确定性清除参考图底部的细线、地面和阴影残留。
3. 将 front/rear/right 方向锚点分别用于 idle、walk_down、walk_up、walk_right 的 image-to-video 首帧。
4. 对完整视频抽取候选帧，使用 `loop@2.0.0` 选择闭合区间并导出每动作 8 帧。
5. 对全部动作使用一个共享尺度，仅平移对齐，不逐帧缩放。
6. 生成 Character Pack，并安装为 Godot 原生 `SpriteFrames` / `AnimatedSprite2D` 外部纹理资源。

## 结果

- 复用 V4 DirectionLock 的 V5 fixture 运行只发出 4 次视频请求、0 次图片编辑请求。
- 四个动作均导出 8 帧，共 32 张 PNG。
- Pack 布局、DirectionLock workflow provenance 和 SHA-256 校验通过。
- Godot 4.6.3 headless 安装与项目加载通过。
- Godot 资源含 idle、walk_up、walk_right、walk_down，`.tres` 小于 1 MiB，不含 `PackedByteArray` 或内嵌 Image。
- Ground-line 清理测试覆盖“移除悬空细线且保留靴子”和“不误删有垂直支撑的角色轮廓”。
- 本次未调用真实 xAI，模型费用为 0。

机器摘要：[summary.json](artifacts/forge-topdown-video-locked-v5-offline-20260809/summary.json)

## 已通过命令

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p providers --test locked_frames_generation_contract -- --nocapture
scripts/test-cli-product.sh
scripts/test-consistency-v2.sh
```

## 仍需真实验收

下一门槛不是继续改 fixture，而是使用 Ayla 的现有 SubjectLock/StyleLock 真实运行 V5：优先只生成 walk_up 与 walk_right，确认视角、地面清除、手部/装备和真实步态；通过后再生成 idle 与 walk_down，并执行 Pack、Godot 与安全审计。该步骤会产生 xAI 费用，需要单独授权。
