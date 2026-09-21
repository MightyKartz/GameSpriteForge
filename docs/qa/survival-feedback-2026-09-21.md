# Survival 使用反馈与 Forge 改进建议

日期：2026-09-21。状态：已核实的调查报告，改进项尚未实施。

Survival demonstrates working local PNG preparation and Godot delivery with Forge
v0.6.3. The highest-priority follow-ups are filesystem-aware publication,
complete portable delivery evidence, and alignment of the Godot lock schema with
runtime support. This audit does not establish animation quality or player approval.

## 范围和身份

- Forge 源码基线：`546bcefaf145b32f6d5f27b1e509a93b071037f8`，开始检查时工作区干净。
- Survival：本机 `/Volumes/Untitled/Dev/Survival`，HEAD `7054eb9`；有未跟踪的
  `asset-sources/portrait-combat-v1/`、`asset-specs/portrait-combat-v1/` 和
  `docs/PORTRAIT_COMBAT_IMPLEMENTATION.md`，未改动，也未纳入正式实现结论。
  审计收尾时观察到另一轮开发更新了 registry、UI 和 assets 配置，并新增
  `portrait_combat_v1` 安装及其 Pack/日志。本报告统计以读取时的 19 键快照为准，
  不包含该新增安装，也不将并发变更视为本次修改。
- 实测 executable：`/Users/kartz/.local/share/forge/versions/v0.6.3/bin/forge`。
  消费者入口 `~/.local/bin/forge` 是指向该文件的符号链接。
- `doctor --json`：0.6.3，同上述 Forge commit，`dirty:false`，`features:[]`，
  `aarch64-apple-darwin`，release；Godot `4.7.2.stable.official.ed1daf0bf`。
- binary SHA-256：`618cdad7ef1981cf046852baad1a34c5e435847ab2ab2356a6eeb51cf5a5e342`，
  与 Survival weapon-lab 和 visual-v2 文档一致。未将本机入口当作远端最新版证明。
- 本次使用已安装发行版做只读核验及隔离合成实验，未重新构建源码、重导入游戏或修改消费项目。

机器可读摘要：[survival-feedback-2026-09-21.json](artifacts/survival-feedback-2026-09-21.json)。
不复制消费者原画、完整 Pack、JobStore 或项目缓存到 Forge。

## 已验证的成果

消费项目保留 23 份自定义交付日志和 23 个 Pack，其中日志包含 19 次静态准备、
4 次动画准备、23 次安装的成功 report；记录的 Provider 请求数全部为 0。
这与“Codex 创作原图，Forge 做本地整理和交付”的产品路径一致。
另有两次安装失败记录保存在 `asset-specs/visual-v2/`，没有被当作成功。

按审计快照 `.forge/assets.json` 中的 `lastJobId` 匹配保留 Pack，对当时 19 个安装键逐一运行
`godot verify-install --project ... --asset-key ... --pack ... --json`：全部退出 0，
文件、Pack、安装快照及已有导入缓存通过校验，`cacheCheck.status=verified`。
这是当前字节一致性核验；全部报告明确 `nativeLoad=not_run`、`visualReview=not_assessed`。
它不证明历史拷贝过程具备事务性，也不是新的 iOS 或视觉验收。

值得保留的消费端实践：

- 原图、提示词、派生哈希、失败候选和失败报告分别留存。
- 不修改 Forge 所有权资源，游戏读取 SpriteFrames 和纹理。
- 100/80/100/160 ms 非均匀帧时长、非循环动作和共享画布保留到引擎。
- 战斗规则决定命中时刻，表现层按规则时钟采样，暂停与重播不改变结算。
- “原型可用”“结构成功”“真人认可”明确区分，四帧姿势跳变与脚位漂移仍如实记录。

## P1：外置文件系统能力与交付终点

**实证。** `diskutil info` 确认消费项目在 ExFAT。相同合成 16×16 RGB PNG 和
`source matte --stdin --json` 请求在本机临时目录成功；输出到 ExFAT 独立临时目录时报
`invalid_source_matte: cannot publish new output PNG: Operation not supported (os error 45)`，
没有输出文件。独立 `os.link` 探针在 ExFAT 同样失败；临时测试目录均已清理。

源码 `source_matte.rs` 使用 `persist_noclobber`，标准 receipt 发布还直接使用
`fs::hard_link`。library、preview、Godot 配置等处也有 no-clobber 发布，应逐个检查，
不能由这次实验宣称所有这些命令均失败。此次没有复现完整 ExFAT 原生安装。

**消费端绕行。** `tools/forge-import.mjs` 把整个游戏复制到 APFS 缓存（排除 `.godot`），
在那里安装，再用 `fs.cpSync` 把目标目录和整个 `.forge` 复制回 ExFAT。
这段返回拷贝不在 Forge 的安装锁和回滚边界内，也不校验最终目标。它还采用目录合并，
新版本删除的文件可能残留；并发编辑时可能覆盖旧 registry。这些是代码层风险，
本次 19/19 核验未发现当前安装损坏，不能写成已发生的数据损失。

**建议。** 首先提供明确的路径/文件系统能力诊断及结构化错误；如探针需要写入，必须
显式展示其检查范围。然后为无法满足原子发布的平台设计可恢复的本机暂存与目标提交协议，
覆盖最终目标锁、库存、删除项、安装 registry、失败恢复和最终审计。
不要简单替换为“先判断不存在，再复制”，也不要静默降低无覆盖与完整文件可见性保证。

**验收。** APFS/ExFAT 原生矩阵；已有输出不被覆盖；并发写入、拷贝中断、更新删除文件、
缓存再导入均有明确结果；最终审计针对消费者真实目录。其他操作系统需另做原生验证。

## P1：把完整证据流程做成容易复用的路径

**实证。** 自定义“receipt”实际格式为 `schema/generatedBy/steps/sources/delivery`，
包含 CLI 响应但不是 Forge 标准 receipt。包装器没有调用 `receipt export/verify`，
也没有复制后的 `godot verify-install`；`copiedUnchanged:true` 是声明而非实际哈希比较。
registry 的 Pack 路径仍指向缓存 JobStore，尽管项目已保留另一份完整 Pack。

Forge 已具备 producer 身份、完整库存、迁移后的 Pack/project override 和标准 receipt，
无需再造证据格式。可以先提供受测试的消费端示例：读取已固定 executable 的指南，
核对身份 → 准备 → 校验 → 安装 → 标准 receipt 导出 → 在最终位置验证 → 保存 receipt hash。
待文件系统协议解决后，再评估是否合并常见命令步骤。

**固定版本的缺口。** Survival 的 toolchain lock 仅保存版本，公共导入包装器默认读取
会随升级移动的 `~/.local/bin/forge`；只有 weapon-lab 预处理脚本检查 SHA。
应让生产入口统一验证真实 binary hash、features 和能力，而不是仅在 README 记录。
消费者 specs 未使用 `sourceLocks`；已记录的人工/AI 审阅哈希应在明确审阅边界后绑定到请求。
Plan 自身仍会检查计划前后的输入，这不是“没有任何输入保护”。

**验收。** 用合成资产走完整示例；移走 JobStore 后仍能以保留 Pack 和最终项目验证；
入口指向不同二进制时明确拒绝或要求显式升级；失败历史和审阅状态不被重写。

## P1：修复 Godot 锁 Schema 与运行时不一致

Survival 的有效锁记录 Godot `4.7.2.stable.official.ed1daf0bf`，但仓库
`schemas/godot-toolchain-lock.schema.json` 的 `godotVersion.pattern` 仍是
`^4\\.6\\.[^\\r\\n]+$`。直接匹配该真实值失败，而 runtime 支持 4.6.x 和 4.7.x。
这是公开数据契约问题，不是 Survival 操作错误。

建议同步 Schema，并补“运行时产生的锁能通过公开 Schema”的合同测试；
覆盖 4.6、4.7 和不支持的版本，避免只测试 Rust 内部判断。
本轮仅记录问题，未修改 Schema 或生产代码。

## P2：将现有素材检查提前接入工作流

**实证。** weapon-lab 首版 `linqiao-blade-v1.png` 被项目拒绝为不透明棋盘底。
本次 `source inspect` 测得 1254×1254，1,572,516 个不透明像素，透明/半透明均为 0。
alpha 检查能确认其没有真实透明度，但“棋盘图案”本身来自项目视觉记录，不由数字推断。
最终 `zhouye-blade-matte-v4.png` 的 2×2、627×627 网格四格均未触边
（alpha 1 和 32 两阈值）；项目另保留了旧靴子/刀尖触边失败候选。

Forge 已有 `source inspect --frame-width/--frame-height`、深浅底预览、alpha 统计、
源图锁和图像合同；不要把“新增透明度/网格检测”再列为缺失功能。
真正缺口是入口只测量并存档，没有把“背景允许不透明、角色要求透明、动画需要边距”等
用途要求连接到导入决策。`preserve_source` 合法允许不透明背景，不能一律拒绝 RGB。

建议在示例/指南中增加按用途的预检流程与明确的失败动作，并复用现有图像合同。
动画先整张去底，再检查网格/共享画布/边缘，避免逐帧重新居中掩盖真实漂移。
用合成图覆盖“假透明、武器触边、软 alpha、非均匀帧时长”；美术质量仍需另行审阅。

## P2：按实际显示尺寸审阅与更清楚的批处理诊断

- visual-v2 首次 `preserve_source` 混合画布请求被拒绝，项目改为按尺寸分包。
  这符合现有契约。改进错误信息时应返回每项真实尺寸与建议分组，保持显式计划，
  不自动缩放或偷偷分包。
- 1254 原图切出的图标为 418×418，项目后来用 Forge 重采样到 128；物件采用 256。
  项目记录大幅缩小的锯齿得到改善。Forge 已有 Lanczos 和 linear 选项，下一步应是
  将“源尺寸预览”和“游戏显示尺寸预览”一起呈现；mipmap/导入策略需另做原生对比，
  不能由本次文档阅读断言根因。
- `source matte` 拒绝覆盖旧输出是既定保护。复现脚本曾因此失败，改用独立缓存输出。
  示例应提供新输出目录/显式缓存复用模式，而不是鼓励删除旧证据。

## P2：把消费项目错误与素材错误区分开

两份失败报告分别包含 `_draw_prop_hotspot()` 不存在和 GDScript 缩进解析错误，
均发生在开发中的游戏副本。Forge 正确识别了“Godot 退出码成功但日志有 SCRIPT ERROR”，
并把 Job 标为失败；不能把放宽错误检查作为修复。

当前外层错误为 `automation_failed`。建议添加阶段、日志文件和诊断原因，明确是
项目脚本解析、Pack 无效、工具不可用还是安装验证失败，并给出“修复项目后重试安装，
复用已成功准备的 Pack”的动作。若增加预检，应在隔离副本里运行，避免提前导入修改
正式缓存；最终安装仍要完整核验。

## 适合归纳为示例，不应扩大为生成承诺

1. 扩展现有 `examples/godot/forge-external-clock` 文档，说明非循环动作、命中事件、
   暂停/seek 和恢复段如何衔接。Survival 的武器路由与战斗结算属于游戏逻辑，
   无需迁入 Forge。可用合成四帧动作验证时序，不复制游戏代码与私有原画。
2. 当前已有外部时钟采样器，不能称为缺失播放器；新增价值是消费场景说明与回归覆盖。
3. 静态物件“单独看可用，贴到场景上突兀”导致用户改用绿色热点和物件面板。
   审阅建议应覆盖实际构图、尺寸和用途；素材 Pack 通过不等于场景组合通过。
4. 四姿势、整帧武器族动作不等于骨骼换装。持握点、遮挡、独立武器层和脚位漂移
   需要独立研究与验收，不能从本项目成功导入推导出已支持通用角色生成。
5. 项目保留大量历史 Pack，却未使用 Forge V3 library。优先提供保留旧证据的
   渐进登记示例，连接修订、拒绝原因、选择和交付；不要强制迁移或清除历史文件。

## 推荐顺序与验证边界

1. 小范围修复 Godot Schema，并建立与 runtime 的合同测试。
2. 完成文件系统能力诊断与 no-clobber 发布设计，再开展 ExFAT 原生交付验证。
3. 提供完整消费示例，连接版本固定、标准 receipt、最终位置审计和可恢复交付。
4. 完善用途预检、尺寸分组错误、游戏尺寸预览与安装失败诊断。
5. 扩展外部时钟与渐进资源库示例。

本次没有 Rust 改动，未运行 Rust 测试；未跑游戏新构建、Godot 渲染、iOS 或 Windows。
历史成功/失败次数来自保留日志；本次新增运行证据仅为 doctor、19 次只读安装核验、
两幅 PNG 检查、文件系统合成探针和 Schema pattern 对照。
消费项目持续开发中的未跟踪文件保持原样，所有改进建议尚未实施。
