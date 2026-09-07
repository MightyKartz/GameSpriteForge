# 零生成角色复用：main 分批集成验证

本次从远端 main `23e5b905024942024f3f461d14cdb589c603c444` 建立独立工作树，
只集成现有三方向复用工具所需的 Pack/Godot 支持、辅助脚本和使用文档。
原工作区的其他历史改动、源素材和已完成候选保持原样。

## 批次与边界

1. [PR #11](https://github.com/MightyKartz/GameSpriteForge/pull/11)：Pack 逐帧时长、
   可选人工审核记录格式、渲染合同校验，以及 Godot 原生时长和渲染参数导入。
2. [PR #12](https://github.com/MightyKartz/GameSpriteForge/pull/12)：零生成准备、
   运行验证和打包工具、Godot 审核场景、7 项离线完整性测试及 CI 入口。
3. 使用入口、[操作说明](../architecture/forge-existing-animation-reuse-plan.md)和本次精简 QA 证据。

这些提交提供实验辅助流程。源人工批准只适用于对应源帧，不批准新的运行时校准。
up 方向原始尺度门禁失败仍保留；本次固定倍率约 0.939902 是数学拟合，
`visualReview = pending`、`productionPromoted = false`。没有新增视频或其他真实 Provider 请求。
未扩展默认 CLI 的生成工作流，也未发布新的二进制 Release。

## CI 兼容修复

首次远端运行使用 Rust 1.98，新增的 `chunks_exact_to_as_chunks` Clippy 规则
命中了 main 原有代码。本轮将三处固定四字节分组改为等价的 `as_chunks` /
`as_chunks_mut` 写法，涉及组件 fixture、Provider fixture 和其测试 fixture。
本地 Rust 1.98 的默认及 Consistency/World Clippy、完整 workspace 测试均通过。
首次远端其余五组矩阵已通过；最终远端结果见各 PR 的最新 Checks。

## 本轮检查

| 检查 | 本轮结果 |
| --- | --- |
| 默认 `cargo fmt --all -- --check` | 通过 |
| 默认 workspace Clippy，warnings denied | 通过 |
| 默认 `cargo test --workspace` | 通过；不把零用例分组算成独立覆盖 |
| Pack 定向回归 | 37 项通过 |
| 零生成 Python 完整性回归 | 7 项通过 |
| `bash scripts/test-cli-product.sh` | 通过；本机 FFmpeg、ffprobe、Godot 可用 |
| 三个真实已有 Pack 的独立 CLI 安装 | 通过；本次请求计数 0 |
| Godot 4.6.3 headless 和有界面运行 | 均通过；有界面运行产生截图 |
| 52 帧实际 AtlasTexture 区域 RGBA 对照 | 全部通过 |
| 13 张安装纹理的字节哈希 | 与源素材一致 |
| 7 个 Godot 文本资源 | 外部纹理引用、每个小于 1 MiB |
| ZIP 全新解压、无缓存首次导入和运行 | 通过 |

源周期保留 right 2041 ms、down 1833 ms、up 2000 ms，`speed_scale = 1`。
四段移动均覆盖对应方向全部帧，最大位移误差约 0.00281 px，原始 pivot 到角色锚点
最大距离为 0。以上为工程检查；仍需人工判断动作、轮廓、脚底接触与方向切换效果。

[机器摘要](artifacts/forge-directional-reuse-main-2026-09-07/summary.json)记录代码、
本轮产物哈希及计数。[运行截图](artifacts/forge-directional-reuse-main-2026-09-07/review.png)
来自本次独立构建。源 Pack、完整 JobStore、Godot 项目和约 24 MiB ZIP 按
[QA 产物政策](forge-qa-artifact-policy.md)留在本地，未进入 Git。

远端质量矩阵以各 PR 的 Checks 为准，与此处本地证据分开。历史报告的通过数量未计入本轮。

## 重现命令

在仓库根目录运行；本地依赖缓存完整时可加 `CARGO_NET_OFFLINE=true`。

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p pack
bash scripts/test-cli-product.sh
python3 scripts/character/test_prepare_directional_reuse.py -v
cargo build -p forge-cli
```

集成运行需要本地保留的、具有原始批准和哈希来源链的恢复素材；仓库不提供这套完整素材。
`SOURCE_ROOT` 指向 `recovered-pack-exports`，`REUSE_GATE` 指向已有
`existing-video-reuse-gate.json`，`CANDIDATE` 必须是新的绝对路径：

```bash
python3 scripts/character/prepare_directional_reuse.py prepare \
  --source-root "$SOURCE_ROOT" --reuse-gate "$REUSE_GATE" \
  --output "$CANDIDATE" --forge "$PWD/target/debug/forge" --godot "$GODOT"
"$GODOT" --headless --path "$CANDIDATE/godot-project" -- --qa-auto-quit
"$GODOT" --path "$CANDIDATE/godot-project" -- --qa-auto-quit
python3 scripts/character/prepare_directional_reuse.py verify --output "$CANDIDATE"
python3 scripts/character/prepare_directional_reuse.py package --output "$CANDIDATE"
```

可移植检查将 ZIP 解压到新的目录，确认其中没有 `.godot`，再运行：

```bash
zsh /absolute/extracted/forge-directional-reuse-review/open-review.command --headless -- --qa-auto-quit
```

启动脚本先导入外部纹理再运行场景；成功要求日志包含 `RUNTIME_REVIEW PASS`，
且新生成的 `qa-output/runtime-report.json` 中 `runtimePassed` 为 `true`。
