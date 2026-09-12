# Forge CLI 仓库清理

日期：2026-09-12。起点：`main@4e67b5d5c123cba932bb8e3dfcabe99dd1f673d7`，开始时与 `origin/main` 一致。

## 清理范围

仓库现在只维护 Rust CLI 及其处理、Pack、Provider 和 Godot 工具链。

- 删除 React/Tauri 桌面应用、根 npm workspace/lockfile、前端依赖与旧 MCP 构建缓存。
- 删除桌面 UI source guard、截图工具、旧 DMG 打包/公证/安装脚本及无活跃消费者的手工 QA fixtures。
- 删除旧根目录设计方案、Figma 原型、桌面实施计划、UI 架构和纯 UI QA 记录。历史仍可在起点 Git 提交中恢复。
- 保留有效 CLI、Pack、Provider、Godot 和动画文档、发布记录、测试与实验 feature。混合文档说明历史范围，当前说明更新到 CLI 契约。
- 删除仅桌面使用的 `core::preview` 和 `core::export::godot`；CLI Godot 交付继续使用 automation runner 和原生导入脚本。
- 将任务结果中的旧桌面后续操作换成 `job_report`，Godot 安装继续提供 `inspect_project`。
- 将唯一仍需使用的图集移至 `examples/inputs/forge-walk-sheet.png`，保持源文件字节不变。
- Godot smoke 使用仓库相对路径，并复用实际 `SpriteFrames` 时长检查器；移除会在报错后继续打印 PASS 的旧 helper 检查器。
- 更新 Cargo workspace、开发文档、forge-dev 技能与忽略规则。保留 Job/Plan 历史目录和序列化兼容，不改变用户资产库位置。

本地旧 UI 截图和工具状态已保留在仓库外的 `/Users/kartz/Development/Forge-cleanup-archive-20260912/`。已有 `docs/qa/` 下未跟踪或忽略的历史素材保持原位；Rust 构建产物和本轮完整 QA 输出留在 `target/`。这些本机数据不上传 GitHub。

## 验证

结果：全部通过。机器可读记录见 [validation.json](artifacts/forge-cli-only-cleanup-20260912/validation.json)。

| 检查 | 结果 |
| --- | --- |
| v0.3 完整质量矩阵 | 8/8 阶段通过，0 失败 |
| Rust workspace | fmt、Clippy 和 275 项测试通过；默认忽略的真实 Godot 测试在交付阶段单独执行并通过 |
| CLI 与生成契约 | 角色一致性/完整矩阵、静态 CLI/五种风格矩阵、实验世界资源通过；使用 fixture Provider |
| 本地引擎交付 | Godot 4.6.3 静态、动画、旧 Pack 兼容与原生资源验证通过 |
| 安装及指南 | CLI 安装器、签名契约通过；内嵌指南/skill 33 个场景、118 次命令通过 |
| 独立 Godot smoke | 从仓库外工作目录运行，8 帧导入/播放/逐帧时长通过；错误时长负例退出 1 且不输出 PASS |
| 完整性 | 14 个 shell 语法检查通过；Markdown 相对路径和 Rust 内嵌资源均无缺失 |

主要复现命令：

```sh
FORGE_GODOT_PATH=/Applications/Godot.app/Contents/MacOS/Godot \
GODOT_BIN=/Applications/Godot.app/Contents/MacOS/Godot \
FORGE_V03_REPORT_DIR="$PWD/target/qa/cli-only-cleanup-20260912/matrix" \
bash scripts/test-v03-release-matrix.sh
cargo build --locked -p forge-cli --no-default-features
python3 scripts/test-cli-skill.py --forge "$PWD/target/debug/forge" \
  --output "$PWD/target/qa/cli-only-cleanup-20260912/skill"
python3 scripts/character/test_prepare_directional_reuse.py -v
bash scripts/test-cli-installer.sh
bash scripts/test-cli-signing-contract.sh
bash scripts/run-godot-pack-smoke.sh
```

测试对象是上述起点上的清理工作树。默认二进制编译身份为 `0.3.2`、`aarch64-apple-darwin`、`debug`、`features=[]`、`dirty=true`；SHA-256 为 `5be5fb79cd31c377e9a553edeb48eb76cdf8d7ba10a3141c27636ec45db94a88`。提交后的构建身份会随 Git 提交改变。

只提交精简报告，不提交测试 JobStore、PlanStore、完整 Pack 或临时 Godot 项目。新增验收文件已检查，无凭据和授权响应。

本轮不调用真实 Provider，不发布新的版本安装包，也不修改现有游戏项目。
