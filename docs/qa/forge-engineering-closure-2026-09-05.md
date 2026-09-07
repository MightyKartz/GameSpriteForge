# Forge 工程收尾：重试、Godot 事务与持续集成

日期：2026-09-05

状态：本轮工程修复与适用离线验证全部完成；未进行新发布或真实模型晋级。

本报告针对 `codex/stage3-static-assets` 的本地工作树。分析前已有大量未提交实现，
本轮保留这些改动；历史真实模型报告不作为当前工作树测试通过的替代。

## 修复范围

### V4 单帧重试

分析时默认工作区测试为 549 passed / 1 failed / 1 ignored。失败发生于
`topdown-frames@4.0.0` 成功父 Job 的 `walk_right` 索引 2 帧重试，返回
`animation_sheet_regeneration_required`。

Fixture 虽正确解析了重试相位，但首次 sheet cell 与 repair prompt 进入了不同的腿部
绘制分支。现在两者使用同一套四关键帧姿态，旧八相位流程保留。没有修改真实 Provider
提示词、提高请求预算或降低质量门禁。

新增四动作 × 四相位的逐像素合同；完整 V4 合同同时验证仅请求一次、源 Job 四帧哈希
不变、child 中未选中的 0/1/3 帧逐字节复用。失败断言附具体阶段报告，便于后续诊断。

### Godot 安装事务

安装流程与交付辅助逻辑从大型 runner 提取到
`packages/core/src/automation/runner/godot_install.rs`，runner 减少约 830 行。
原有版本要求、外部纹理验证、方向播放契约和公开 CLI 协议保持。

- Godot 版本检查、源素材 staging 在替换旧资产之前完成。
- 所有目标变动后的普通错误统一恢复旧资源；首次安装失败则删除不完整目标。
- 项目 manifest 和可选资产 catalog 在锁内保存快照；安装、登记、提交或回滚共用
  同一锁范围。公开登记 API 使用相同锁，事务内部采用持锁写入入口。
- 登记或最终 JobStore 写入失败，也恢复两份登记文件；原本不存在的文件不会残留。
- 恢复失败会返回原始错误与恢复错误，并保留 Job 中的备份。非正常进程终止后的自动
  恢复不属于本轮新增承诺；Job 备份仍可用于恢复。
- 对不能由现有目录复制器完整备份的非普通文件，安装在替换前拒绝操作。

定向测试覆盖首次/覆盖安装的十类故障：版本、源 staging、进程启动、import 状态、
安装脚本状态、日志写入、必需资源缺失、嵌入像素、catalog 登记、最终 Job 写入。
另外验证新 manifest 清理、成功提交和第二安装等待首个回滚后再提交。

### CI 与文档

PR workflow 拆成四个独立组，每组有自己的报告与 30 分钟上限：

| 组 | 覆盖 |
| --- | --- |
| release | 保留既有默认发布矩阵 |
| project-assets | GameArtManifest 与 Stage 3 静态资产 CLI/fixture |
| subject-grid | Subject 导入、Grid feature 合同、CLI/schema |
| processing | Pixel Grid、feature-on normalization、Identity 单测与 evaluator 编译 |

新增 `scripts/test-experimental-feature-matrix.sh` 输出机器可读结果和日志。
CI 安装满足 Grid 合同约定的 Godot 路径，避免引擎断言被静默跳过。矩阵入口清除真实
Provider 授权/凭证环境变量；不改变默认 feature 或发布版本。

Identity 人工校准语料未受 Git 跟踪，CI 明确记录该校准为 `not_run`。便携单元测试及
evaluator 编译通过不等于重新完成真实图像校准。

新增双语[工作流与发布边界](../architecture/forge-workflow-boundaries.md)，同步 README、
CONTRIBUTING 和总状态表。阶段 3 的局部真实修复、World 工程验收、V18 专用交付均保留
原范围，不扩展为通用能力或新发布结论。

为通过当前 Rust Clippy，还对已有例程的只读路径参数、一个冗余闭包和测试默认参数
初始化作了无行为变更的整理，未关闭 lint。

## 本轮验证

| 检查 | 结果 |
| --- | --- |
| V4 locked-frame 合同 | 2 passed，含原失败场景 |
| Godot 模块定向测试 | 8 passed，含四个原方向播放测试 |
| 默认 Clippy，全 targets，`-D warnings` | 通过 |
| 全 features Clippy，全 targets，`-D warnings` | 通过 |
| 格式检查 | 通过 |
| 默认工作区完整测试 | 555 passed / 0 failed / 1 ignored |
| 默认 CLI product | 通过，含实际 Godot 安装和凭证扫描 |
| 新实验分组：project-assets | 3/3 门通过：工具、GameArtManifest、Stage 3 |
| 新实验分组：subject-grid | 3/3 门通过；包含 35 个 Grid Provider 合同及 CLI/schema |
| 新实验分组：processing | 4/4 门通过；包含 8 个处理单测及 evaluator 编译 |

完整日志与分组 JSON 输出于 `target/qa/forge-engineering-closure-20260905/`。
该目录为本机测试产物，不把完整临时 JobStore 或生成媒体纳入 Git。
`verification-summary.json` 汇总最终通过状态；`checks.tsv` 保留迭代过程，包含已修复的
首次 Clippy 失败，最终 Clippy 日志及本表均为修复后复验结果。

默认测试中唯一 ignored 用例需要本机保留的历史真实 xAI Job；其忽略条件未修改。

本机环境为 Rust 1.96.0、Godot 4.6.3、FFmpeg 8.0.1。CI 保留其固定的 Godot 4.6.3
与 FFmpeg 8.1.2 构建配置；本机结果不冒充尚未运行的 GitHub 执行结果。

本轮真实 Provider 媒体请求：0。所有生成验收使用离线 fixture，未执行新的真实模型、
付费或人工美术验收；已配置的远端 GitHub CI 仍需提交后由 GitHub 实际运行。

复验命令如下。依赖缓存恢复后，本轮检查均使用离线 Cargo 模式。

```bash
export CARGO_NET_OFFLINE=true
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --offline -- -D warnings
cargo clippy --workspace --all-targets --all-features --offline -- -D warnings
cargo test --workspace --offline --no-fail-fast
bash scripts/test-cli-product.sh
FORGE_EXPERIMENTAL_SUITE=project-assets bash scripts/test-experimental-feature-matrix.sh
FORGE_EXPERIMENTAL_SUITE=subject-grid bash scripts/test-experimental-feature-matrix.sh
FORGE_EXPERIMENTAL_SUITE=processing bash scripts/test-experimental-feature-matrix.sh
```
