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
- 使用外部纹理、原子覆盖、所有权检查和失败回滚的 Godot 安装。

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

角色发布门槛已经完成：连续三次真实 xAI Character → Pack → Godot 运行均无需人工
审核，四个动作全部达到 `game_ready`。Provider 费用、重试方式和 Godot 证据记录在
[`docs/qa/forge-character-loop-v2-2026-08-03.md`](docs/qa/forge-character-loop-v2-2026-08-03.md)。
`v0.2.0-cli.1` 已在 GitHub Releases 正式发布（2026-08-03，未签名、未公证，附带 SBOM 与
Artifact Attestation）。最后一项发布操作检查——从该 Release 进行全新账户安装验证——尚未
记录于 `docs/qa/`。

## 尚未发布的角色一致性 V2

源码树包含面向 v0.3 CLI 发布线的可选 `consistency-v2` 构建：不可变 Subject Lock、语义化图片
参考、每动作 8 帧的显式关键帧、类型化 WorkflowGraph 重放、内容寻址缓存和
`.forge/catalog.json`。在真实 xAI 验收完成之前，这些命令不会进入默认发布
二进制。v0.3 fixture/合同矩阵已于 2026-08-04 六门全过
（[`docs/qa/forge-v03-test-matrix.md`](docs/qa/forge-v03-test-matrix.md)）；真实模型
身份晋级门槛仍未完成。SAM/DINO/LPIPS 组件在许可证和阈值校准审计通过前保持未发布，
`forge component install` 不会静默安装未经审计的权重。
离线合同、Godot 验证和仍待完成的外部门槛记录在
[`docs/qa/forge-consistency-v2-and-world-implementation-2026-08-03.md`](docs/qa/forge-consistency-v2-and-world-implementation-2026-08-03.md)。

## 尚未发布的世界资产流水线

后续 CLI 里程碑通过尚未发布的世界功能特性和 `.gsfpack` V3 合同实现；这些命令不会
进入默认发布二进制，并保持 experimental；v0.3 矩阵已记录其 fixture 级通过。Terrain、Building、Map 会在角色一致性 V2 之后分版发布：

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
