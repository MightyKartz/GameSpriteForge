# Forge topdown-video@2.0.0 离线验收（2026-08-09）

## 结论

通过离线晋级门槛；真实 xAI 未运行，仍需单独授权。

`topdown-video@2.0.0` 已成为 Character V2 的稳定视频合同，
`topdown-spritesheet@3.0.0` 保持 experimental。旧 `topdown@1.0.0` 的序列化
camera profile、Pack 和 GameArtManifest 构建均通过回归。

## 本次修复

- 新增显式 camera profile：正交与三分之四不得静默混用。
- 新增 `direction-still-preflight@1.0.0`，在视频请求前检查方向、构图、
  SubjectLock 相对尺度、顶部留白、中心、脚底基线和裁切。
- V2 直接复用已验证 SubjectLock，不再额外重生成 Subject Reference。
- 修复 `validationOnly` 仍生成全部四方向的问题；现在只生成请求的方向。
- 完整 V2 计划为预计 8/最多 16 次媒体请求；单方向为预计 2/最多 4 次。
- V2 walk 播放从旧版 12 FPS 固定为目录声明的 8 FPS；V1 保持 12 FPS。
- Provider manifest、WorkflowGraph、Pack metadata 与 Godot `forge_usage.json`
  均记录 camera profile；预检节点自身不产生 Provider 请求。
- 视频提示明确要求相反接触姿势、经过姿势、无残影、无镜头运动、无新增光效。

## 自动验收

```text
cargo fmt --all -- --check                                      PASS
cargo clippy --workspace --all-targets -- -D warnings          PASS
cargo test --workspace                                         PASS
scripts/test-consistency-v2.sh                                 PASS
scripts/test-cli-product.sh                                    PASS
```

新增端到端 fixture 合同覆盖：

```text
StyleLock
→ SubjectLock
→ topdown-video@2.0.0 四方向
→ 4 个 direction still preflight
→ image-to-video
→ ≤12 FPS / ≤96 候选帧
→ loop@2.0.0
→ V2 Pack
→ Godot 4.6 headless 安装与项目加载
```

断言包括：外部纹理、原生 `AnimatedSprite2D`/`SpriteFrames`、camera profile
provenance、Pack 校验、`forge_usage.json`、旧 GameArtManifest Character 构建、
Sprite Sheet/Keyframe/静态资产/世界资产回归以及零凭据泄漏。

## 真实模型门槛

本报告不代表真实 xAI 视觉晋级。推荐下一步只授权：

1. 复用现有 Ayla SubjectLock/StyleLock；
2. `topdown-video@2.0.0` + `topdown-orthographic@2.0.0`；
3. 先生成 `walk_right`，预计 2、最多 4 次媒体请求；
4. 通过人工原尺寸与 Godot 播放检查后，再单独验收严格背面的 `walk_up`；
5. 两个方向都通过后才生成完整四方向。
