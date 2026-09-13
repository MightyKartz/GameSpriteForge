# 资源库整组审核问题集中修复

日期：2026-09-13。范围固定为
[整组审核](asset-library-pr-stack-review-2026-09-13.md) 确认的 R1–R5。
修复更新现有 #35–#40；#34 无需修改。未合并 PR、发布版本或改动 Sword。

## 修复及针对性回归

| 问题 | 实施 | 验证 |
| --- | --- | --- |
| R1 / #40 | 追加历史保留共同前缀，以拓扑顺序保持双方分支内因果关系；不兼容顺序报告冲突 | 两种哈希顺序、两种合并方向、三类历史数组；真实 approved→rejected 与独立 auditory 合并仍保留 rejected；既有同域并发审核仍冲突 |
| R2 / #35–#40 | 对象、head、local 和待恢复发布记录按实际序列化字节执行 16 MiB 上限；intake 在发布本地绑定前预检 head | 上限内/边界/越界对象，head/local 原子保留，超大批量登记和 17 MiB origin 拒绝后旧资源仍可查询；生产及安装回归通过 |
| R3 / #36 | 单独持久化发生变化的本机 root 映射，不依赖共享 head 是否变化 | 整份 local 丢失和仅缺一个 root 均恢复；原 revision/head 和其他 root 不变 |
| R4 / #39–#40 | 预览、审计、导出和移机校验共用普通文件及摘要验证，消除不安全索引 | 证据丢失、空目录、单文件目录和内容篡改均受控处理；预览输出问题，导出和 bundle 校验拒绝 |
| R5 / #40 | canonical intake Schema 共享 item 定义，原路径作为兼容引用；文档与测试统一 | 两路径离线验证真实 scan、全部可选字段、单项/批量输入；未知字段被 Schema 和 CLI 同时拒绝 |

`jsonschema` 仅为测试依赖，CLI 运行不需要 Python 或网络。

## 本地集中验收

- Rust：4 项 library 单元测试、18 项 library 集成、3 项 review、10 项 transfer 全部通过。
- 10 项 game-art 构建和 10 项 Godot 安装事务全部通过，后者含 5 项真实 Godot 测试。
- `cargo fmt --all -- --check`、`git diff --check` 通过。
- `cargo clippy --locked -p core -p forge-cli --all-targets --features game-art-manifest -- -D warnings` 通过。
- 默认 release CLI 的 `test-asset-library-cli.py` 和 `test-asset-library-review-cli.py` 通过，包括新增 Schema 合同。

上述提交前 CLI 验收身份：工作树基于
`19a8f67936f56f0cf7c0aab381c3305fcdcb2655`，`dirty=true`（包含本轮修复），
target `aarch64-apple-darwin`，profile `release`，features `[]`；
二进制 SHA-256 `ed68a130edd1e1fc1e63bf79992d4f83b6ff9cd32f2f91b525af69e7a344f59a`。
Godot 为本机 4.6.3。测试全部使用隔离合成数据，未请求 Provider。

最终 clean commit 的 CLI 复验身份、SHA-256 和最新远程 CI 链接记入 PR 描述，
避免为回填自身提交号重复触发矩阵。旧审核报告中的失败证据保留为历史快照。
用户此前对正常图片、GIF、音频的人工播放验收继续有效；本轮异常证据回归
没有冒充新的浏览器人工验收。跨操作系统交换与 Windows 安装验收以本轮
最新提交的 CI 为准，旧绿色运行不替代新修复的验收。

## 分支安排

通过普通合并将上游修复传播到后续分支，保留 #34 → #35 → #36 → #37 →
#38 → #39 → #40 的祖先关系。本轮只更新 PR；实际逐个合并 main 尚未执行。
