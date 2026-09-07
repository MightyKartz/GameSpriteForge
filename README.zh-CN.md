# Forge CLI

[English README](README.md)

Forge 是一个开源、面向智能体的命令行游戏资产流水线。Codex、Claude、脚本和
CI 可以通过同一套稳定 JSON 协议生成一致的 2D 游戏资产，并安装到 Godot 4.6.x。

当前 macOS Apple Silicon 版本支持：

- 不可变的项目 Style Lock；
- 带 `idle`、`walk_up`、`walk_right`、`walk_down` 的一致性俯视 Character Pack；
- 从同一风格板和 anchor 派生的图标集与道具集；
- 通过 API Key 或 Preview OAuth 直连 xAI REST，不依赖 Grok Build CLI；
- 确定性抠图、规范化、一致性门禁、阶段级重试、Loop Selection V2、来源记录和 `.gsfpack` 验证；
- 使用外部纹理、备份、所有权检查和事务回滚的 Godot 安装。

## 当前源码与发布范围

默认 CLI 的功能开关为 `default = []`。Character V2/Grid、项目构建、Collection/Portrait
资产、项目审计和世界生成需要显式启用对应 feature。正式 Provider resolver 支持 xAI
和离线 fixture；PixelLab 目前仅有离线 loopback 实验。

使用默认能力可从下方安装与工作流开始。双语[工作流与发布边界](docs/architecture/forge-workflow-boundaries.md)
整理了可选功能、已验证样例和剩余发布门槛，[实施状态](docs/qa/forge-complete-visual-implementation-status.md)
记录阶段进度。后续 V18 验收仅覆盖一条经过人工审核的 `walk_right` 交付，不代表
通用多动作 CLI 已完成发布。

## 安装

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/MightyKartz/GameSpriteForge/main/install.sh | sh
```

重新打开终端后验证：

```bash
forge doctor --json
```

安装器会把 `forge`、`ffmpeg`、`ffprobe` 安装到版本化用户目录，只把 `forge`
暴露到 `PATH`；切换版本前会同时验证 Release 压缩包和包内逐文件 SHA-256 清单。
首个 CLI Release 不带 Apple 签名或公证。Forge 不附带 Godot；引擎安装功能需要
Godot 4.6.x。

## 五分钟 xAI → Godot

通过隐藏输入保存 API Key，不把密钥写入 shell 历史：

```bash
forge provider login --provider xai --method api-key
forge project init --path "$PWD/game-assets" --name "My Game"
```

创建 `game-assets/specs/style.json`：

```json
{
  "schemaVersion": "1",
  "prompt": "带深色轮廓的紧凑宝石色像素风",
  "referenceImages": [],
  "perspective": "topdown",
  "lighting": "upper_left",
  "outline": "dark",
  "background": "transparent",
  "sampling": "nearest",
  "characterCanvasSize": 256,
  "iconCanvasSize": 128,
  "propCanvasSize": 256
}
```

```bash
forge style create \
  --project "$PWD/game-assets" \
  --spec "$PWD/game-assets/specs/style.json" \
  --wait --json
```

创建 `game-assets/specs/ranger.json`：

```json
{
  "schemaVersion": "1",
  "kind": "character",
  "id": "forest-ranger",
  "name": "Forest Ranger",
  "prompt": "一个戴绿色兜帽的紧凑森林游侠",
  "license": "private"
}
```

```bash
forge generate character \
  --project "$PWD/game-assets" \
  --spec "$PWD/game-assets/specs/ranger.json" \
  --wait --json

forge godot plan-install \
  --pack /absolute/path/Forest-Ranger.gsfpack \
  --project /absolute/path/my-godot-game \
  --asset-key forest_ranger --json

forge plan execute --token <返回的-token> --wait --json
```

## 图标集与道具集

```json
{
  "schemaVersion": "1",
  "kind": "icon_set",
  "id": "inventory-icons",
  "name": "Inventory Icons",
  "items": [
    { "id": "potion", "name": "Potion", "prompt": "红色治疗药水" },
    { "id": "key", "name": "Key", "prompt": "一把小黄铜钥匙" }
  ],
  "license": "private"
}
```

```bash
forge generate icon-set --project "$PWD/game-assets" --spec /absolute/icons.json --json
forge generate prop-set --project "$PWD/game-assets" --spec /absolute/props.json --json
forge job report --id <job-id> --json
forge job retry --id <job-id> --item potion --wait --json

# 使用当前 Style Lock 本地重评既有图标/道具像素，不调用 Provider。
forge job retry --id <static-job> --stage consistency --wait --json

# 角色专用重试：loop / matting 只重跑本地阶段，不调用 Provider。
forge job retry --id <character-job> --item walk_right --stage loop --wait --json
forge job report --id <new-job-id> --json
```

生成默认作为可恢复的异步任务运行；增加 `--wait` 可同步等待。公开命令只在
stdout 写一个 JSON envelope，诊断和交互认证进入 stderr/TTY。

Style Lock 使用版本化的 `style-baseline@2.3.0` 前景感知调色板。基线升级时，Forge
保留旧的不可变 revision，并在校验通过后复用原风格板，因此迁移无需重新生成图片。

角色生成会覆盖完整视频搜索真实闭合周期，只导出选中 `[start, end)` 内的帧；
用于证明闭合的边界帧不会重复进入动画。`job report` 会直接返回选中索引、评分组成、
重试方法，以及本次重试是否产生了 Provider 请求和费用。

v0.2 角色发布门槛已于 2026-08-03 完成：连续三次真实 xAI Character → Pack → Godot 运行均无需人工
审核，四个动作全部达到 `game_ready`。Provider 费用、重试方式和 Godot 证据记录在
[`docs/qa/forge-character-loop-v2-2026-08-03.md`](docs/qa/forge-character-loop-v2-2026-08-03.md)。
`v0.2.0-cli.1` 已在 GitHub Releases 正式发布（2026-08-03，未签名、未公证，附带 SBOM 与
Artifact Attestation）。最后一项发布操作检查——从该 Release 进行全新账户安装验证——尚未
记录于 `docs/qa/`。

## 尚未发布的角色一致性 V2

构建时启用 `consistency-v2` 才会放出这组可选命令；Grid 工作流还需要 `grid-generation`。
这些版本与默认发布分开管理。下方保留各版本合同及其对应日期的验收历史。

<details>
<summary>角色工作流历史与各版本验收证据</summary>

`topdown-video@2.0.0` 定义了版本化的 Character V2 合同。Spec 必须显式选择
`topdown-orthographic@2.0.0` 或 `topdown-three-quarter@1.0.0`；Forge 会在产生
图生视频费用之前，硬性检查正面/背面/右侧方向、角色尺度、顶部留白、中心、脚底基线、
相对 SubjectLock 的构图漂移和裁切。随后覆盖完整本地视频抽取候选帧、选择真实闭合周期，
最后只向 Godot 输出外部 PNG/atlas 与原生 `SpriteFrames`。旧 `topdown@1.0.0`
继续用于历史 Character V1 Job 的兼容读取。详见[实施计划](docs/architecture/forge-topdown-video-v2-plan.md)
与[离线验收](docs/qa/forge-topdown-video-v2-offline-2026-08-09.md)。

可选构建支持在生成完整 32 帧 Character 之前，把付费关键帧验收限定为单个方向：

```bash
forge generate character --project /absolute/assets --spec character-v2.json \
  --validation-animation walk_right --plan-only --json
```

对于 `topdown-video@2.0.0`，该模式预计一次方向静帧编辑和一次图生视频请求；两次尝试
最多四次媒体请求。它输出方向、构图、循环、质量与播放证据，但绝不会导出或登记不完整的
Character Pack。

第一版修复合同为 `topdown-keyframes@2.2.0`：不再把多对象 Style board 作为图片参考
发送，Pose guide 改为透明紧凑步态，并强制显式声明 `none` 或 `staff_like` 装备。
对应的 [离线验收报告](docs/qa/forge-character-reference-isolation-equipment-offline-2026-08-08.md)
记录 fixture、CLI 计划、Godot 与仍需完成的真实模型门槛。
后续的 [四方向真实 xAI 验收](docs/qa/forge-character-keyframes-v22-real-acceptance-2026-08-08.md)
被正确拦截：法杖参考泄漏已修复，但方向、Alpha 背景、身份和下半身一致性仍未达到发布要求。

后续 `topdown-keyframes@2.3.0` 保持 2.2 Job 可读，并修复这次验收暴露的两个传播缺陷。
每个方向用 frame 0 建立版本化的正面、背面或右侧 DirectionLock，不增加额外付费请求；
其余帧必须通过该方向的局部基线。每张 Provider 图片还会先经过
`keyframe-background-cleanup@1.3.0`，净化后才允许归一化或成为下游参考；原图与净化图
分别保留 SHA-256。[V2.3 离线 QA 报告](docs/qa/forge-character-direction-lock-background-cleanup-offline-2026-08-08.md)
已覆盖 fixture、Pack 与 Godot 门槛，但升级真实 xAI 路径仍需下一次单独授权的真实验收。

`topdown-keyposes@2.4.0` 是下一条可选修复路径：每个动作以 6 FPS 生成四个明确姿势
（接触、经过、反向接触、反向经过），保持不可变 DirectionLock，并移除导致闪烁和足部残影的
双邻帧 AI 插值。`motion-semantics@1.1.0` 会拦截只有换色、没有真实步态、接触顺序错误、
下缘残影或额外足部轮廓的 Pack。旧 Pack 可用
`forge pack audit-motion --path <pack> --json` 零费用复审；详见
[离线报告](docs/qa/forge-topdown-keyposes-v24-motion-semantics-offline-2026-08-08.md)。

V2.4 真实门槛证明“上一张已接受帧”仍会压过新的 Pose guide：`walk_right` 几乎静止，
串行参考还会传播漂移。`topdown-keyposes@2.5.0` 保留 V2.4 的可复现性，但让第 1–3 帧
分别只从不可变 DirectionLock 与双色左右腿语义 Pose guide 独立派生。下游 motion
硬失败现在保持 `failed` 终态，不再被可审核的 consistency 灰区覆盖；详见
[V2.5 修复报告](docs/qa/forge-topdown-keyposes-v25-isolation-offline-2026-08-08.md)。

`topdown-spritesheet@3.0.0` 保留为低成本实验对照路径：锁定的图片 Provider
每个动作只生成一张包含四阶段的 2×2 完整角色图，随后由 Forge 确定性拆帧、净化、
对齐并执行硬门禁。完整角色预计 4 次、最多 8 次图片编辑；单方向探针为 1/2 次。
重复帧或只有闪烁、没有步态的结果会被 `motion-semantics@1.1.0` 拦截。真实 xAI
验收表明它的身份和构图较稳定，但行走阶段会坍缩为近似重复姿势，因此不再作为移动动画的
默认生成方式。Godot 只接收
外部纹理和原生 `AnimatedSprite2D`/`SpriteFrames`/`AtlasTexture`，不安装扩展，
不使用骨骼、零件或 ControlNet。详见[设计](docs/architecture/forge-topdown-spritesheet-v3-plan.md)
与[离线验收](docs/qa/forge-topdown-spritesheet-v3-offline-2026-08-08.md)。

`topdown-frames@4.0.0` 是下一条实验性纯图片路径。它先生成并验证一张由 Forge
固定排序的 2×2 DirectionLock（正面、背面、右侧、左侧），再把对应方向图作为每个
四阶段动作表唯一的外观和镜头锚点。Style board 只以文字元数据进入提示词，避免风格图中的
道具、特效、裁切或视角污染角色。完整运行预计 5 次、最多 10 次图片编辑；共享尺度、仅平移的
脚底对齐和 `onion-skin@1.0.0` 会保留真实漂移供门禁判断。`job retry --frame 0-3`
只替换指定格，另外三张 PNG 保持逐字节复用。离线 fixture、Pack、CLI、安全和 Godot 4.6.3
门禁已通过；真实 xAI 视觉验收仍需单独授权。详见 [V4 设计](docs/architecture/forge-topdown-frames-v4-plan.md)
与[离线验收](docs/qa/forge-topdown-frames-v4-offline-2026-08-09.md)。

`topdown-video-locked@5.0.0` 是实验性的混合后继路线：保留 V4 已验证的
DirectionLock，本地删除模型画出的细地面线/基线，把对应方向锚点合成到精确纯绿色背景，
然后直接作为图生视频首帧；不再生成每方向静帧，也不使用视频编辑回退。Forge 以最高
12 FPS 覆盖完整视频抽取候选，从闭合的 `[start, boundary)` 区间选择 8 帧且不导出重复
边界帧，最后只使用一个全角色共享缩放和逐帧平移式躯干/脚底锚定。全新角色预计 5 次、
最多 10 次媒体请求（1 次 DirectionLock + 4 个视频）；复用 DirectionLock 时为 4/8。
它必须先通过单独授权的真实 xAI `walk_up`、`walk_right` 探针和完整四动作 Pack → Godot
门槛，才能晋级。详见 [V5 设计](docs/architecture/forge-topdown-video-locked-v5-plan.md)。

`topdown-video-cycle@6.0.0` 是针对 V5 输入清晰度、步态周期与播放速度问题的实验性
修复路线。它把 512px 以上的 DirectionLock 生成母版与 256px 交付精灵分开，使用 720p
图生视频并要求模型输出多个恒速重复周期；只有在 700–1200ms 的源步行周期中证明两次相反
着地和两个 passing pose 后，Forge 才导出 8 帧。所有 walk 固定 800ms，idle 固定
1600ms，Debug GIF、Pack 与 Godot 共用同一份 `frameDurationsMs`。详见
[V6 设计](docs/architecture/forge-topdown-video-cycle-v6-plan.md)；冻结视频离线门禁和另行授权的
真实 xAI 验收通过前，它不会晋级为默认工作流。

`topdown-direction-motion@8.0.0` 是针对方向来源错误和抽帧丢失动态的实验性路线。
它先文生图建立唯一的 `front_idle`，再图生图派生三个方向 idle 和四个同方向动作姿势，
以八张图片的 SHA-256 绑定人工批准；随后另开视频授权，只允许从四张动作姿势分别生成
四个方向视频。V8.1 以原生解码 PTS 保留最高 24 FPS / 120 帧，先在完整视频中选择闭合
源周期，再确定性精简为 8、10 或 12 张原始姿势；闭合边界帧不会重复导出，原始时序直接
进入预览、Pack 与 Godot，也不再逐帧抹掉自然身体起伏。Godot 得到四个显式 idle 和四个
显式 walk，左向也不再水平翻转右向。详见 [V8 设计与 CLI
合同](docs/architecture/forge-character-direction-motion-v8-plan.md)。

源码树包含面向 v0.3 CLI 发布线的可选 `consistency-v2` 构建：不可变 Subject Lock、语义化图片
参考、显式关键帧/关键姿势、类型化 WorkflowGraph 重放、内容寻址缓存和
`.forge/catalog.json`。在真实 xAI 验收完成之前，这些命令不会进入默认发布
二进制。v0.3 fixture/合同矩阵已于 2026-08-04 六门全过
（[`docs/qa/forge-v03-test-matrix.md`](docs/qa/forge-v03-test-matrix.md)）；真实模型
身份晋级门槛仍未完成。SAM/DINO/LPIPS 组件在许可证和阈值校准审计通过前保持未发布，
`forge component install` 不会静默安装未经审计的权重。
离线合同、Godot 验证和仍待完成的外部门槛记录在
[`docs/qa/forge-consistency-v2-and-world-implementation-2026-08-03.md`](docs/qa/forge-consistency-v2-and-world-implementation-2026-08-03.md)。

2026-08-08 的真实 xAI `walk_right` 关键帧定向验收在 14 次图片编辑后被正确拦截。
结果证明不透明、多对象 Style board 的语义内容会泄漏到角色帧中；Forge 未导出 Pack。
详见 [`docs/qa/forge-character-walk-right-keyframe-real-2026-08-08.md`](docs/qa/forge-character-walk-right-keyframe-real-2026-08-08.md)。

</details>

## 尚未发布的 Stage 3 静态资产系统

源码中的 `collection-assets` feature 增加不可变 Collection Lock，并把项目级一致性扩展到
Icon/Prop V2、Portrait、Equipment V1 和 Decal。该能力尚未进入已发布的 v0.2 二进制。
2026-08-05 完成的 1 风格、3 Pack 真实 xAI
探针**未通过**发布门槛，暴露了 Collection anchor、报告闭包、Portrait 构图、
review/Catalog 状态链和 Godot kind 映射问题；详见
[`docs/qa/forge-stage3-real-acceptance-2026-08-05.md`](docs/qa/forge-stage3-real-acceptance-2026-08-05.md)。

后续[修复报告](docs/qa/forge-stage3-blocker-remediation-2026-08-05.md)记录了三个 Pack
范围的通过，包括 Portrait 和项目审计。该局部结论不代表冻结的五风格完整矩阵通过，
也不代表 `collection-assets` 已晋级为默认发布功能。

<details>
<summary>静态资产合同、可选命令与验收历史</summary>

Portrait 构图现在是显式的离线契约：旧 Portrait V1 继续兼容
`dialogue_bust@1.0.0`；Portrait V2 必须选择该半身契约或
`full_body@1.0.0`。全身候选会在裁切、缩放之前检查，必须引用以脚部为锚点的
Collection Lock，并禁止套用半身重构图。明显缺少下半身的结果会在 Pack 导出前被
阻断。报告只称该确定性指标为 `lowerBodyPresenceProxy`，不会虚假宣称已进行语义腿部识别。

Portrait V2 现在把合格的 `neutral` 图锁定为不可变 Portrait Base，其余表情均从同一基准
独立派生，不再分别重绘完整角色。Forge 会确定性恢复版本化脸部范围之外的每个像素，并用
局部肤色与脸颊保护区伪影门禁补足整图 palette 指标。对 2026-08-06 冻结 xAI 产物的零费用
回放已证明：围巾、服装、身体和双腿漂移被消除；hurt 脸部划痕与 surprised 肤色漂移会在
Pack 导出前被拒绝。

新的 Portrait V2 CLI 改为两阶段：`--phase base` 只为 neutral 付费（预计 1、最多 2 次），
不会导出不完整 Pack；`forge job review --accept` 写入哈希绑定 approval 后，
`--phase expressions --base-job <id>` 才生成四个表情（预计 4、最多 8 次）。默认
`subject-style` 策略让身份参考优先，并从 neutral 请求中移除可能与角色配色冲突的
Collection anchor 图片。

`portrait-local@1.1.0` 采用表情感知肤色审核：扩大后的眼睛和嘴巴不参与肤色采样，匹配
皮肤使用截尾均值，并以独立的皮肤对应关系指标拦截异常换色。非严重且仅涉及肤色的问题可
暂停等待显式审核；身体像素变化、严重划痕、身份漂移等硬缺陷仍不可越过。

真实付费运行可使用 `forge provider authorize` 和 `--authorization <id>`。不含凭据的
manifest 与原子请求账本，会跨后台 worker、子重试和 replay Job 共同执行逐目标、总请求数与
费用上限；`forge job report` 可审计账本，但不包含凭据、prompt、授权头或临时 URL。详见
[`docs/automation/forge-cli.md`](docs/automation/forge-cli.md) 与
[`examples/cli/stage3`](examples/cli/stage3) 中的全身示例。

```bash
forge collection create --project /absolute/assets --spec /absolute/collection.json --wait --json
forge collection inspect --project /absolute/assets --id inventory --json
forge generate portrait-set --project /absolute/assets --spec /absolute/portraits.json --phase base --wait --json
forge job review --id <base-job-id> --accept --reason "neutral approved" --json
forge generate portrait-set --project /absolute/assets --spec /absolute/portraits.json --phase expressions --base-job <base-job-id> --wait --json
forge generate equipment-set --project /absolute/assets --spec /absolute/equipment.json --json
forge generate decal-set --project /absolute/assets --spec /absolute/decals.json --json

forge asset export-editable --project /absolute/assets --id inventory-icons --output /absolute/editable --json
forge asset replace-item --id <source-job-id> --item potion --path /absolute/potion.png --wait --json
forge project audit --project /absolute/assets --scope all --json
```

`replace-item` 始终创建子 Job，Provider 请求数为 0。项目审计覆盖 Pack/SHA 完整性、
Style/Subject/Collection revision、质量与集合离群、license/provenance、Godot 安装、
内嵌图像、凭据特征和临时媒体 URL。可通过 `forge schema list/show` 获取规范；示例位于
[`examples/cli/stage3`](examples/cli/stage3)，冻结范围与真实门槛见
[`docs/architecture/forge-stage3-static-assets-implementation-plan.md`](docs/architecture/forge-stage3-static-assets-implementation-plan.md)。

</details>

## 尚未发布的世界资产流水线

后续 CLI 里程碑通过尚未发布的世界功能特性和 `.gsfpack` V3 合同实现；这些命令不会
进入默认发布二进制，并保持 experimental；v0.3 矩阵已记录其 fixture 级通过。
[真实工程验收](docs/qa/forge-world-v1-real-acceptance-2026-08-03.md)也已通过，但地形
重复纹理和建筑美术质量仍阻止发布晋级。Terrain、Building、Map 分别管理发布里程碑。

<details>
<summary>实验性世界资产范围与命令</summary>


- 不可变的俯视 Environment Lock；
- 从两张 Provider 材质板确定性合成的 16/32px dual-grid Terrain Set；
- 使用固定屋顶、墙体、门窗模块的外观 Building Kit；
- 不调用 Provider、输出自包含 Godot 世界的 JSON Map Compiler。

```bash
forge environment create --project /absolute/assets --spec /absolute/environment.json --wait --json
forge generate terrain-set --project /absolute/assets --spec /absolute/terrain.json --wait --json
forge generate building-kit --project /absolute/assets --spec /absolute/buildings.json --wait --json
```

Map 只接受 JSON。Forge 不调用文本模型，也不把自然语言转换为地图；Codex、Claude
或用户负责生成 `MapSpecV1`，Forge 只负责校验和确定性编译：

```bash
forge map schema --json
forge map compile --project /absolute/assets --spec /absolute/map.json --wait --json
forge map validate --pack /absolute/Forest-Village.gsfpack --json
```

V1 只覆盖俯视户外地图、dual-grid Terrain、3×3–8×6 的矩形建筑外观、南向入口和
Godot 4.6.x；不包含室内、等距、平台跳跃、3D、Tiled、Unity 或 Unreal。可运行 JSON
示例位于 [`examples/cli/world`](examples/cli/world)。

</details>

## 安全与来源

- Provider 输出先落地、校验格式并计算 SHA-256，之后才进入本地处理。
- 每个 Job 锁定一个 Provider、Profile、模型选择和 Style revision。
- 凭据保存在 Keychain，不进入 Job、Pack、日志或普通 JSON 输出。
- OAuth 为 Preview；API Key 是稳定商业认证路径。
- Godot 写入限定在 `addons/forge_assets`，只覆盖 Forge-owned 目录。

## 开发与许可证

开发说明见 [CONTRIBUTING.md](CONTRIBUTING.md)。Forge 使用
[MIT License](LICENSE)。随 CLI 分发的 FFmpeg helper 使用独立 LGPL 声明并在每个
Release 同时提供对应源码，详见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
