# Forge Top-down Key Poses V2.5 离线验收

日期：2026-08-08

## 结论

V2.5 隔离式关键姿势合同与 Job 终态修复通过离线门槛。没有调用真实 Provider。

- `topdown-keyposes@2.4.0` 保持兼容。
- `topdown-keyposes@2.5.0` 仍为 16 次预期、32 次最大图片请求。
- frame 1–3 只使用 DirectionLock 与当前 Pose guide，不引用上一帧。
- WorkflowGraph 的后续姿势只依赖 `frame_image:<action>:0`。
- Pose guide 使用左右肢双色语义并扩大 contact/passing 差异。
- motion hard failure 返回 `failed` / `character_motion_semantics_failed`，不暴露 review action 或 candidate Pack。
- `recommendedRetryFrames` 在静止/相位错误/足部异常 fixture 上非空。

## 测试证据

- `cargo test -p core motion_semantics --lib`：5/5 通过。
- `cargo test -p providers --test keyframe_generation_contract --no-fail-fast`：1/1 通过；
  单个合同覆盖 V2.3、V2.4、V2.5、Pack、可用时的 Godot、单帧 retry、local replay、
  direction failure、review candidate 和 motion hard failure。
- `cargo check -p forge-cli --features consistency-v2`：通过。
- `cargo clippy -p core -p providers -p forge-cli --features consistency-v2 --all-targets -- -D warnings`：通过。
- `cargo test --workspace`：通过。
- `scripts/test-consistency-v2.sh`：通过。
- `scripts/test-cli-product.sh`：通过。

## 真实 V2.4 产物零费用复审

只读取 Job `63a91d35-bccb-4e50-b15f-b9520517ca80` 的 16 张本地规范化帧，没有修改
原 Job，也没有 Provider 请求：

| 动画 | 结论 | 推荐重试帧 |
| --- | --- | --- |
| `idle` | blocked | 0, 1, 2, 3 |
| `walk_up` | blocked | 1, 2, 3 |
| `walk_right` | blocked | 0, 1, 2, 3 |
| `walk_down` | blocked | 1, 2, 3 |

这与人工大图一致：V2.4 不应人工放行。原 Job 仍保留历史 `awaiting_review` 字段作为
不可变证据；同类新 Job 会正确进入 `failed`。

## 未完成的外部门槛

V2.5 尚未进行真实 xAI 生成，因此不能宣称解决了模型侧步态。下一次真实验收应先只生成
`walk_right` 与 `walk_down` 的 8 个姿势（预计 8、最多 16 次图片请求），确认独立参考确实
服从 Pose guide 后，再决定是否重跑完整四方向。
