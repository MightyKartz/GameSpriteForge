# Agent resource production: M0 baseline

日期：2026-09-21。对应[开发计划 M0](../architecture/agent-resource-production-roadmap.md)。
本轮完成技术盘点、可重放任务和首个阻塞点定位；没有完成 M1，也没有得出生产力收益结论。

English summary: the default CLI already prepares multi-action characters, static
props and local audio. This experiment found a delivery gap: Godot's default
alpha-border processing changes RGB at nonzero alpha, despite lossless Pack
frames. Disabling that setting in a separate diagnostic copy passes the same
native checks. This is a reproduction and a proposed next fix, not a shipped fix
or a claim of artistic quality or productivity improvement.

## 能力差距表

发行版指 v0.6.4 的 macOS 公共安装入口；源码指 main `cfde241`，均为默认特性。
“盘点”与“本轮执行”分开：未运行的预览与可选生成流程没有新增通过证据。

| 分类 | 能力 / 缺口 | 依据与下一步 |
| --- | --- | --- |
| 已有可复用 | `plan prepare-character` 接收多动作、规则图集及 PNG 序列 | 两种二进制执行本轮配方；复用 schemaVersion 2，无需新命令 |
| 已有可复用 | 共享画布、锁定锚点、preserve_source、保留 alpha | Pack 帧与独立源帧逐字节比较 RGBA；本轮没有逐帧缩放、居中或补帧 |
| 已有可复用 | 非均匀时长、循环、统一动画控制器 | 原生检查 SpriteFrames、seek、advance 和结束状态 |
| 已有可复用 | 静态 prop / icon、WAV 音乐与音效导入、Plan / Job / 安装事务 | 本轮执行 prop 和两项 WAV；icon 与失败回滚沿用已有测试，未在本轮重复测量 |
| 已有可复用 | 随二进制 guide、doctor 能力发现、预览及审核入口 | 核对 guide、doctor 与既有 `test-godot-preview-ui.py` / `test-godot-unified-player.py`；本轮不作视觉审核 |
| 需修复 | Godot 导入会改变软边可见 RGB | 两组角色所有帧及 prop 原生像素检查失败；见下文首个阻塞点，排入 M1 |
| 需修复 | 多动作时长错误没有动作名；缺帧错误没有文件名 / 帧索引 | 两种二进制都拒绝输入，但 Agent 无法直接定位；优先补错误上下文，再决定是否新增错误协议 |
| 需修复 | 音频省略参数时默认转换为 48 kHz / stereo，与规划的“显式转换”目标存在距离 | `packages/core/src/audio.rs` 和 audio guide 明确现有默认值；本轮显式指定 22050 Hz / mono。后续先给保留参数的配方，不能悄悄修改兼容默认值 |
| 确实缺失 | 嵌入 guide 缺少可直接运行的三动作任务配方 | animation guide 有单动作 JSON 和多动作说明；本轮请求生成器可作为 M1 示例来源 |
| 确实缺失 | 本路线的真实三动作素材、两个独立游戏各三轮配对证据 | 雷灵只有 idle / move；attack 为合成回归帧。第二个项目尚未确认，M4 证据不足 |
| 可选实验 | Character V2 / consistency-v2、world 工作流、在线生成 Provider | 依据 Cargo features 和既有实现；本轮不启用、不调用，不宣称已在默认发行版完成验收 |

## 固定任务与素材

运行器：[agent_resource_baseline.py](../../scripts/experiments/agent_resource_baseline.py)。
引擎检查器：[check-agent-resources.gd](../../scripts/experiments/check-agent-resources.gd)。

| 输入 | 独立验收约定 | 来源及限制 |
| --- | --- | --- |
| synthetic | 64×64，锚点 (32,52)；idle / walk / attack 各 3 帧；时长分别为 [70,150,230] / [80,120,160] / [60,240,100] ms；attack 不循环 | 运行器生成；包含 alpha=1 与 alpha=128、伸展手臂、透明边界；用于回归，不证明角色动作自然 |
| thunder | 原图 1254×1254，2×2，行优先；627×627 帧，锚点 (313,580)；idle 200 ms / move 143 ms，均循环 | 重用仓库已公开的 [Codex 雷灵源图和来源记录](../media/showcase/thunder/README.md)；本轮重新处理，不继承旧 Job 的批准；143 ms 是本轮明确约定，不是历史 7 FPS 的精确复刻 |
| prop | 64×64、nearest、保留画布及所有非零 alpha 像素 | synthetic idle 第 0 帧的副本；静态验收仅覆盖纹理内容，不声称检查了场景锚点 |
| music / cue | PCM16 WAV，22050 Hz / mono，分别 1.0 s / 0.2 s；music 循环，cue 不循环 | 合成 440 Hz 正弦波，测试音乐/音效角色元数据，不是真实曲目或试听审核 |

先产生 `acceptance.json`，再执行 Forge。它包含独立源帧、大小、锚点、动作、
时长及音频约定，不读取 Forge manifest 来生成预期。Godot 适配器只补充资源路径。
Pack 帧顺序按 manifest 映射回源帧；原生检查进一步核对每个动作的像素及实际时序。
纹理检查忽略 alpha=0 下的隐藏 RGB，但要求所有非零 alpha 的 RGB 和全部 alpha 不变。
音频检查已保存资源类型、PCM16、采样率、声道、样本数、循环模式与完整循环区间，尚未核对听感、
实际音频输出、循环接缝或 PCM 内容相等。

动画配方从一开始就使用 `requireGameReady: false`，定位为 prototype 技术验证，
没有为了绕过失败降低门槛。visualReview / listeningReview 均保持 `not_assessed`。

## 运行方式与证据

需要 Python 及 [Pillow 依赖](../../scripts/requirements-local-assets.txt)、本机 Godot、
选定 Forge 及可用音频工具。源码验证先按[开发指南](../../.agents/skills/forge-dev/SKILL.md)
构建选定 checkout，并核对实际 Cargo target 目录。

```bash
python3 scripts/experiments/test-agent-resource-baseline.py -v
python3 scripts/experiments/test-agent-resource-native.py \
  --godot /absolute/path/to/Godot \
  --output /tmp/forge-agent-m0-oracle-new-attempt
python3 scripts/experiments/agent_resource_baseline.py \
  --forge /absolute/path/to/forge \
  --godot /absolute/path/to/Godot \
  --output /tmp/forge-agent-m0-new-attempt
```

`--output` 必须不存在，防止覆盖失败尝试。每轮保存独立 inputs、requests、
acceptance、Plans、Jobs、游戏目录、命令日志、耗时、二进制 / 原图哈希及 doctor 身份。
执行顺序为 doctor → 两项错误请求 → 每类 prepare / execute / report / validate /
install → 原生验收 → 错误预期时长负对照。已知原生失败会返回非零，`report.json`
保留 `ok:false`；不应把它作为已通过的 CI 验收。常规 CI 运行验收器和素材完整性单测，以及独立构造原生资源的验收器回归测试；
这些不是 Forge 交付通过证据。M1 修复后再把完整原生交付任务接入门禁。输入哈希检查与命令超时日志也在报告中保留。

原始日志和生成目录保留在执行机器的 `/tmp/forge-agent-m0-*`，不提交 Job 存储与
导入缓存。可移植观察摘录见 [M0 observations](artifacts/agent-resource-m0/observations.json)，
包含完整构建身份、源图哈希、错误输出和原生失败记录；摘录不是完整的可搬迁回执。

## 首个阻塞点：Godot 改写低透明度像素

发行版与源码均成功准备和验证四个 Pack，安装 Job 也成功；两个角色的 17 帧在
Pack 内都与源帧一致。但原生 Godot 4.7.2 检查时，17 帧和 prop 的可见像素检查
全部失败，其余动作数、锚点、时序、循环及音频资源检查没有报告失败。

最小例子：prop 的 (22,20) 像素从 `(120,50,210,1)` 变为 `(150,80,190,1)`。
alpha 没变，RGB 变了。Godot 自动生成的 `.png.import` 使用
`process/fix_alpha_border=true`。这不是 Forge 本地源帧处理失败，也不能仅凭
安装 Job 成功推断满足像素约定。

因果对照在**另外复制的隔离游戏目录**中进行：仅将三个 PNG `.import` 中的
`process/fix_alpha_border` 改为 `false`，重新 headless import，再用同一份
源帧和验收文件运行检查器，得到 `RESOURCE_TASK_PASS:5`。这证明该设置能够
解释本轮不一致；没有修改生产安装器，也没有把对照结果计为发行版通过。

在该副本上另做两项负对照：将预期 idle 第 0 帧时长增加 10 ms，检查器返回
`FAIL:duration:synthetic:idle:0`；仅改预期源帧中 alpha=1 的 RGB，返回
`FAIL:pixels:synthetic:idle:0`。两次均返回非零；恢复预期后检查通过。
这些对照不代表视觉审核，也不表明应对所有采样 / 材质无条件禁用透明边缘修复。

## Agent 卡点与实施顺序

1. **PR-A / M0（本次）**：完成技术盘点、可重放素材与独立验收，保留失败和对照证据。
   第二个真实项目及完整三动作角色仍待确认；先继续技术修复，产品收益保持证据不足。
2. **PR-B / M1**：为需要像素保留的动画 / 静态资源明确 Godot 纹理导入策略，
   让上述任务原样通过。核对导入前设置、重装、回滚和缓存，不直接套用全局开关。
   在真实 Godot 验证，加入现有 macOS / Windows CI；补最短三动作 guide 配方。
3. **PR-C / M1–M2**：补多动作缺帧与非法时长上下文，保证错误指出动作、帧或路径；
   再根据真实坏例补局部修正。不先另建角色框架或通用动画评分系统。
4. **M3**：复用现有音频处理与静态安装，先完善显式保留参数的路径和格式摘要；
   补音乐/音效实际播放与循环验证，保留兼容性。
5. **M4**：确认第二个独立项目，补真实 attack、试听 / 视觉审核与配对任务记录。
   尚无人工时间、模型费用或维护成本数据；这些字段保持 null，不推算“节省 20%”。

本次没有修改公共产品能力声明、游戏消费端 pin 或历史回执，没有发布新版本。
本地原生证据仅覆盖 macOS / Godot 4.7.2，不替代 Windows 或 Godot 4.6 的运行验证。


## PR #59 审查修复

首轮 PR 自审复现了验收器的漏检，已补回归，原始 M0 观察记录保持不变：

- **画布约定未生效**：把独立验收的角色 / 道具尺寸改为 128×128 后，旧检查器仍通过。
  现在同时核对源帧、原生纹理与声明尺寸，不再仅比较两张图彼此相同。
- **循环区间漏检**：把音乐资源改为仅循环一个采样点，旧检查器仍通过。
  现在核对 loop_begin=0、loop_end=完整帧数；非循环资源要求区间为 0。
- **错误资源触发脚本异常**：缺失 AnimatedSprite2D / SpriteFrames 时，先检查再进入
  场景树，避免控制器的初始化抢先报错，改为直接输出资源缺失诊断。
- **失败尝试污染验收文件**：负对照改用独立副本及 finally 恢复原始文件字节；
  超时、错误通过、脚本异常均不能留下修改后的时长。提前失败也核对输入哈希。
- **动作素材不能区分**：旧 synthetic 的 idle / walk 对应帧完全相同，存在互换仍通过的
  盲点。现为每个动作设置不同像素标记，保留原有画布、锚点、alpha、伸展和时长约定。
  因此新合成输入哈希不同，不能当作旧观察的同输入重复运行；历史证据不重写。

5 项 Python 测试和 8 个原生验收器用例通过。原生用例使用微型原生资源和独立预期，
覆盖正常资源、错误角色 / 静态尺寸、循环区间、时长、alpha=1 颜色及两种缺失资源。
完整发布版基准仍复现 18 项已知导入像素失败；隔离导入设置对照再次通过全部 5 项
任务，负对照返回预期错误并恢复验收文件。证据见
[PR review verification](artifacts/agent-resource-m0/pr59-review.json)。
未完成整组验收的结果项以 `nativeAcceptance: null` 表示尚未确认，不将其一律标为失败。
本轮修复的是 M0 验收准确性；生产安装器的透明边缘策略仍由下一阶段 M1 处理。
