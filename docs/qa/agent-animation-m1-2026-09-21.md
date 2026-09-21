# Agent animation production — M1 technical delivery

日期：2026-09-21。范围：默认特性的源码实现；不是新版本发布或视觉质量批准。
对应[路线图 M1](../architecture/agent-resource-production-roadmap.md)，复用
[M0 基准](agent-resource-baseline-2026-09-21.md)的原始输入和独立验收。

## 本次交付

- 平面 animation / character 和 icon / prop 的 Godot 安装，在首次导入前写入
  lossless `.import` 配置：不修复透明边缘 RGB、不预乘 alpha、不生成 mipmaps、
  不缩小纹理。nearest / linear 采样及原有混合方式仍按 Pack 设置。
- 原生验证在新 Godot 进程中读取已保存纹理，检查导入设置和完整 RGBA，与 Pack
  中 PNG 比较；不一致按现有事务失败并回滚。验证阶段的 `verifiedSpriteTextures`
  列出检查过的纹理；不把 GPU 滤波后的屏幕颜色等同于存储纹理像素。
- `.import` 与 PNG 一起位于受管理安装目标内，复用既有备份、缓存快照和恢复机制。
  重装会重新应用设置。layered / world / audio 的导入策略未扩展到本能力。
- 多动作错误包含动作名；缺失 PNG 和保留画布不匹配包含零起算帧索引及路径。
  时长数量不匹配给出期望及实际数量，零时长指出 `frameDurationsMs[index]`。
  沿用已有 JSON 错误结构，不引入第二套任务协议。
- `forge guide animation-example` 提供 idle / walk / attack 三动作请求，
  `forge guide animation` 连接准备、报告、Pack 校验、安装和验证。
  重复交付复用现有 `local-delivery-example`，不新增编排命令。
- 新能力标识 `godot_lossless_sprite_import` 用于区分已实现的源码构建与旧发行版。
  旧安装不会自动改变；消费端应显式选择、验证新工具链后再重装。

English scope: the default-feature source build now imports flat sprite and static
PNG textures losslessly and verifies saved native RGBA against the Pack. Existing
sampling, animation timing and transactional recovery remain in place. The embedded
guide includes a three-action recipe and actionable input errors identify the
animation, frame or path. This is source implementation evidence, not a release,
visual approval, or a productivity claim. Layered/world import policy is outside
this capability.

## 技术验收

验收复用已有机制而非新增一套格式。三动作基准使用 PNG 序列及规则图集、共享
64×64 画布、指定锚点、非均匀时长、alpha=1 / alpha=128 及伸展手臂；另有已公开
雷灵 idle / move、prop 和 WAV 音乐 / 音效角色。原图哈希与 PR #59 审查后的
基准完全一致，未通过更换输入消除原有失败。

| 检查 | 验收要求 |
| --- | --- |
| M0 完整原生任务 | 原 18 项像素差异消失；5 项资源任务通过；负时长对照仍被拒绝；输入不变 |
| 原生安装事务 | 五个既有真实 Godot 用例全部执行；覆盖重复安装、失败更新、原生缓存恢复、资源引用和保留版本回滚 |
| 新增故障注入 | 修改导入设置后验证必须失败；alpha=1 输入同时用于 linear 动画和 nearest 静态资源 |
| 动画 / Pack 回归 | 坐标、时序、源变换、质量、旧请求及 Pack 合同保留；新增动作/帧/路径错误上下文用例 |
| 嵌入 guide | 独立 CLI 在隔离目录读取原始示例并执行三动作准备、验证 Pack；不依赖仓库、skill 安装或 Provider |
| 原生平台 CI | 既有 macOS / Windows × Godot 4.6.3 / 4.7.2 矩阵加入完整 M0 任务；保留报告、请求与命令日志 |

可移植本地摘录见 [verification.json](artifacts/agent-animation-m1/verification.json)。
记录实际构建身份、dirty 状态、特性、二进制 SHA-256、Godot 版本和运行结果；
原始 Job / Plan / 原生缓存留在临时目录，不入库。CI 以本 PR 的实际检查和上传证据
为准，macOS 本地通过不能替代 Windows 验证。

复现入口（`FORGE`、`GODOT` 为选定绝对路径，输出目录必须新建）：

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked -p pack
cargo test --locked -p core --test animation_delivery_tests --test animation_pixel_quality_tests --test anchor_tests --test godot_install_transaction_tests --test static_delivery_tests
python3 scripts/test-cli-skill.py --forge FORGE --output NEW_GUIDE_OUTPUT
python3 scripts/test-native-godot-transactions.py --godot GODOT --output NEW_TRANSACTION_OUTPUT
python3 scripts/test-local-animation-delivery.py --forge FORGE --godot GODOT --output NEW_ANIMATION_OUTPUT
python3 scripts/experiments/agent_resource_baseline.py --forge FORGE --godot GODOT --output NEW_BASELINE_OUTPUT
```

## 使用边界及后续

这是角色动画**技术闭环**：合成三动作和已有公开素材的坐标、像素、时序、安装及
原生播放可验证。未生成新的真实 attack，没有做逐帧美术审核、试听或设备测试。
示例明确选择 prototype 质量门槛；没有将 `prototype_usable` 改称 `game_ready`。

关闭 Godot 透明边缘处理会改变旧安装的纹理预处理行为，尤其应在升级时检查
linear 滤波边缘与实际背景的组合。像素保留不保证观感更好，也不修复模型缺帧。
源图预处理、引擎运行、视觉判断继续分开。旧发布版本缺少新能力标识，不会因阅读
新 guide 获得新行为；本次不升级游戏 pin、不修改历史回执、不发布版本。

M2 的局部坏帧修正、系统化诊断与审核，M3 的音频 / 静态使用体验，以及 M4 的
两个真实项目配对收益验证仍按路线图另行实施。人力、模型费用、维护成本和节省
比例没有新增测量，结论继续是证据不足。
