# Forge topdown-video-locked V5 离线修复验收

日期：2026-08-09

## 结论

本轮确定性修复已完成并通过回归。没有新增真实 xAI 请求，也没有新增费用。

修复没有把两套已知有缺陷的真实视频“美化为通过”，而是准确选取靠近 DirectionLock
的 idle 周期，并在最终八帧上识别方向漂移、异常步态、轮廓/身份偏移，最终以
`failed` 结束且不导出 `.gsfpack`。这是预期的失败安全结果。

机器摘要见
[summary.json](artifacts/forge-topdown-video-locked-v5-remediation-offline-20260809/summary.json)。

## 已实施

- `loop@2.1.0`：V5 idle 只能从初始锚点窗口开始，并记录锚点相似度。
- `direction-quality@1.2.0`：逐一检查实际导出的八帧，记录匹配比例与最大连续错向。
- `motion-semantics@1.1.0`：检测缺少步态、错误相位、上身闪烁、残脚与运动过度。
- `keyframe-background-cleanup@1.3.0`：只删除脚平面上小而暴露的线状/稀疏绿色残留；
  高饱和绿色靴子、宽披风和装备受到组件形状保护。
- `consistency@1.7.0`：V5 使用最终动作帧与对应 DirectionLock 的调色板、轮廓和语义
  锚点重算报告；Provider 尝试次数来自 manifest。旧 Character/关键帧继续使用
  `consistency@1.6.0`，避免 provenance 伪升级。
- `loop`、`matting`、`consistency` replay 在计划为 0/0 时跳过认证健康检查；即使没有
  API Key/OAuth，也不会访问 Keychain 或产生 Provider 请求。

## 冻结真实样本

两个源 Job 均使用清除 `FORGE_REAL_PROVIDER_*` 授权环境变量后的 CLI 重新执行：

| 源 Job | 最终 replay Job | idle 锚点区间 | 主要拦截 | 结果 |
|---|---|---|---|---|
| `e621d016…` | `f2f96350…` | `1..46`，相似度 `0.9801` | `walk_down` 方向匹配 `0/8`；动作一致性与运动语义失败 | `failed`，无 Pack |
| `c5809f1f…` | `b51b2e1d…` | `3..27`，相似度 `0.9486` | `walk_down` 匹配率 `0.25`、连续错向 3 帧；三项动作一致性要求重生成 | `failed`，无 Pack |

第二套样本的 `walk_down` 最终选中帧删除 35 个脚平面绿色残留像素，报告无残留；
第一套删除 5 个。两次 `provider-usage.json` 均为 0 请求、0 图片、0 视频、0 视频编辑。

原真实授权账本仍为 8 个请求、`26,400,000,000` observed cost ticks、
`nextSequence = 9`，证明离线修复没有消费授权。

## 回归与安全

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`（Core 209 项及全部 CLI/Pack/Provider/世界资产回归）
- V5 fixture：32 个最终帧、有效 Pack、Godot 4.6 headless 安装/资源加载、无认证本地 replay
- `scripts/test-cli-product.sh`
- `scripts/test-consistency-v2.sh`
- JSON Schema、`git diff --check`、凭据/临时 URL/内嵌 Godot Image 扫描

扫描结果为 0 个凭据标记、0 个临时媒体 URL、0 个内嵌 `PackedByteArray`/
`ImageTexture.create_from_image`，冻结坏素材导出 Pack 数为 0。

## 边界

本结论证明的是修复逻辑和失败安全门禁有效，不代表旧的真实视频已经成为可用游戏资产。
下一次真实生成仍应从 DirectionLock 重新生成视频，并单独获得用户费用授权。
