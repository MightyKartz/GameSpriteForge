# M3：音乐、音效与静态素材的 Agent 使用闭环

日期：2026-09-21。范围：默认特性源码；不发布版本、不升级消费项目、不推进 M4。

English scope: M3 reuses existing local preparation, preview, native Godot
installation and receipts. It adds audio to the embedded delivery example,
identifies malformed batch inputs and corrects the audio Pack review domain.
Synthetic native playback is technical evidence, not listening, visual or license
approval. No new CLI command, request schema or orchestration framework is added.

## 基线与实际缺口

基线 `f24fb810a78676929ae7a74eeebd6b582ba14be8`，本地 main 与 origin/main 一致、
工作树干净、无开放 PR。已核对 PR #59/#60/#61 合并及 M0/M1/M2 的验收记录。
重新编译默认特性 CLI 后，用隔离项目实际执行既有音频与静态素材脚本：

| 工作 | 已有能力 / 实测 | 本阶段处理 |
| --- | --- | --- |
| 音频准备 | WAV inspect、Plan/Job、PCM16、显式裁切/增益/交叉淡化、源字节与回执；原有音频任务通过 | 保持请求默认值兼容；guide 要求按输入显式选声道/采样率，不把省略字段当作保真模式 |
| 图标/道具 | 规范化画布、源画布保留、中心/落脚点、nearest/linear、真实 Godot 安装；三组既有静态任务通过 | 复用，不新增规范化命令或质量评分 |
| 批量诊断 | 损坏 WAV/PNG 的底层格式错误可能缺少 item ID 或原输入路径 | 在现有错误消息中加入 item ID 与原路径；错误 envelope/code 不变 |
| 已审核素材交付 | 嵌入 `local-delivery-example` 已保留准备结果、安装失败证据，但只允许图像/动画操作 | 增加已有 `prepare-audio` 操作；检查音频能力、显式输出格式；继续复用相同安装/回执代码 |
| 音频预览 | 现有页面已复制并播放原始/处理后 WAV | 修复 `audio_set` Pack 被建议使用 `visual` 审核域的问题；建议 `auditory`，不自动批准 |
| 中断恢复 | Job 状态、安装事务与可移植回执已具备 | guide 统一三条最短路径与按阶段恢复表；真实项目解析失败后只重试安装 |
| 引擎播放 | 既有测试已检查保存资源、采样率、声道、时长及循环范围 | 增加独立 AudioStreamPlayer 检查：超过一次完整时长后音乐仍播放，音效已结束 |

不增加新来源格式、自动降混、响度归一化、模型安装、源文件覆盖或通用任务框架。
请求的历史 48000 Hz/双声道默认值保持兼容；新的交付示例要求显式字段。
已有 WAV `smpl`/cue 区域不自动导入，guide 明确 whole-clip loop 和源工具确认边界。

## 可复现的任务与验收

使用 Python 3.10+、当前检出构建的绝对 CLI 路径与已验证 Godot 路径。输出目录
必须全新且位于消费项目之外；每个脚本创建自己的项目与 stores。

```sh
cargo build --locked -p forge-cli --no-default-features
python3 scripts/test-local-audio-cli.py --forge /ABS/forge --godot /ABS/godot --output /NEW/audio-baseline
python3 scripts/test-local-static-cli.py --forge /ABS/forge --godot /ABS/godot --output /NEW/static-baseline
python3 scripts/experiments/agent_supporting_resources.py --forge /ABS/forge --godot /ABS/godot --output /NEW/m3
python3 scripts/test-cli-skill.py --forge /ABS/forge
```

`CARGO_TARGET_DIR` 非默认时应解析实际产物路径。M3 runner 从该 CLI 的 `guide`
导出并运行示例，不绕过嵌入版本；检查二进制 SHA-256、doctor 构建身份和示例哈希。

任务合同由脚本生成的合成素材固定，不读取 Survival 或其他消费项目：

1. 22050 Hz 单声道循环音乐、48000 Hz 双声道一次性音效分别准备，显式选择各自
   输出格式；中性处理后的 PCM16 样本逐字节一致，时长一秒，源文件保持不变。
2. 在同一隔离项目安装规范化 64×64 道具，核对 `(32,60)` 落脚点与 nearest。
   既有静态脚本另覆盖图标中心原点和 linear/nearest。
3. 混合批次中损坏的 WAV/PNG 返回 `broken` item 与路径，且不创建 Plan/Job。
   音频示例缺少显式格式时在生产前拒绝；隔离无 FFmpeg/FFprobe 场景返回依赖原因。
4. 音乐准备成功后，真实 Godot autoload 解析失败导致安装失败。保留原准备 Job、
   Pack、失败 progress 和准备回执；修复隔离项目后从原 Pack 新建安装 Plan，
   导出新的恢复回执，验证旧证据不变，不重复生成/准备。
5. 用已有资源库预览精确版本：WAV 副本哈希匹配原始/处理后文件，音频推荐
   auditory、静态推荐 visual；预览前后资源库字节不变，没有产生任何审核批准。
6. 独立 Godot 脚本加载保存的 `.res`/道具场景，检查格式、循环范围、锚点和纹理
   采样，再实际启动两个 AudioStreamPlayer 检查循环/结束行为。
7. 验证安装与回执，保留输入哈希、所有命令/错误及源/游戏逻辑/隔离锁文件不变证据。

CI 复用 Godot agent workflow，在 macOS/Windows × Godot 4.6.3/4.7.2 四个组合执行
同一 M3 任务。原有 M0/M1/M2、音频、静态、安装事务与指南检查继续运行。
最终提交的 CI 状态和原生平台 artifacts 以 PR 检查为准；本地记录不冒充 Windows 结果。

## 验证层级与限制

- **结构/数据：** Pack、回执、来源哈希、PCM 样本、输出时长、画布、锚点及循环意图。
- **真实引擎：** 保存资源加载、引擎采样设置、播放器启动、音乐跨循环继续、音效结束。
  Headless 音频驱动不提供扬声器/设备试听或无缝循环观感证据。
- **试听：** `not_assessed`；需实际完整试听、重复循环接缝、游戏音量和设备审核。
- **视觉：** `not_assessed`；合成道具的技术正确性不代表美术风格/可读性批准。
- **许可：** 仅测试作者的合成素材来源声明；技术报告、浏览器预览及 receipt 不确认许可。
- **收益：** 证明复用同一交付/恢复代码可行，未测量真实项目节省时间、费用或维护成本。
  两项目配对实验仍属于 M4，本次不实施。

运行身份、测试结果与保留的失败记录见
[本地验收摘要](artifacts/agent-supporting-resources-m3/verification.json)。
