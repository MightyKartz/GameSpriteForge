# Forge 项目资源库 PR 整组审核

> 以下为修复前审核快照；后续 R1–R5 实施与验收见
> [集中修复记录](asset-library-review-fixes-2026-09-13.md)。

日期：2026-09-13。范围：PR #34–#40 的最终组合变更，相对 main 基线
`1c1e7079f22ea0dba5958e4aec79cad69f7ade65`，审核 HEAD 为
`2c63972f02a0b7bdb2c89755dc33df0be5391e9b`。

结论：建议先集中处理下列 5 项确认问题，再按依赖合并；其中 R1、R2 为
高优先级合并阻塞。本轮没有修复产品代码、提交代码、修改 PR 或执行合并。
本记录及汇总证据仅保存于本地，供确定一次修复范围。

## 审核方式与证据

共 70 个变更文件，9315 行新增、102 行删除。审查覆盖规范化内容摘要、
V3 对象/入口与迁移、登记/搜索、统一生产发布及恢复、game-art 历史依赖、
资源保有/锁版、安装事务接入、预览/审核、移机清单/重绑定/索引/合并、
CLI 参数、Schema、guide 与 CI 接入。检查新增测试是否覆盖对应异常边界。

七个 PR 当前共 23 个远程检查均通过。分支依赖均经 `git merge-base
--is-ancestor` 验证为直接线性祖先关系；各本地分支与其远端跟踪分支一致。
本轮未重新运行完整发布矩阵，也未把已有绿色 CI 当作对新增问题的反证。

使用审核 HEAD 重新构建默认 release CLI：

- source：`2c63972f02a0b7bdb2c89755dc33df0be5391e9b`；构建时工作区 clean。
- target：`aarch64-apple-darwin`；profile：`release`；features：`[]`。
- binary SHA-256：`9546811ac8bb0cb182d4817f91d01212eae689054b7428e631522e099096ea69`。
- 独立合成测试根：`/tmp/forge-pr-stack-review-20260913`。
- 复现脚本：该目录下 `reproduce.py`；逐次命令输出：`commands.json`。
- 汇总证据：[results.json](artifacts/asset-library-pr-stack-review-2026-09-13/results.json)。

下面的运行复现均在本机默认 CLI 上完成，没有调用生成 Provider、启动模型、
使用 Sword 媒体或更改消费者锁。现有浏览器人工播放通过的结论仍成立；
R4 检查的是异常证据状态下的 CLI 行为，不是重新进行浏览器验收。

## 确认问题

### R1 / P1：合并排序会撤销同一分支上后续作出的审核拒绝

位置：`packages/core/src/library/merge.rs:82–91`，关联 PR #40。

`reconcile` 将并行新增的 reviews 按 JSON 字符串排序；对审核引用而言就是
按对象 SHA 排序。`intake::hit` 和 preview 则把数组中最后一条同域记录当作
当前结论。SHA 排序破坏了原有分支内的审核先后关系。

复现：同一基础版本，ours 先写 visual=approved、后写 visual=rejected；
theirs 只新增 auditory=unknown。两边没有同域并发冲突。合并返回
`conflicts:[]`、`applied:true`，但该版本 visual 从 rejected 变为 approved。
这是实际批准状态反转，不只是展示次序变化。

建议修复边界：合并必须保留每个输入分支已有的顺序约束，不能以哈希排序
决定审核先后；若无法合并顺序则显式报冲突。同步检查 revisions 和 installations
等具有顺序消费者的数组。不能简单依赖机器时间戳来挑选并发结论。

回归验收：上述两分支用例在两种 SHA 大小关系下均保留 rejected；真正的
同版本同域并发审核仍报告冲突；并行版本与安装历史不丢失。

### R2 / P1：写入允许超过读取上限的对象，成功登记后目录无法正常查询

位置：`packages/core/src/library/mod.rs:142–159`，读取限制在 `71–75`。
对象写入基础属于 PR #35；公开 intake 触发路径属于 #36，origin 字段属于 #40。

`read_bytes` 拒绝超过 16 MiB 的元数据，但 `write_object`、共享 JSON 写入和
head 提交没有对应的可读性上限校验。过大来源说明、累计安装快照/历史或
批量目录可以被写入，并由权威 head 引用。

复现：通过正常 intake JSON 登记一个带 17 MiB `origin.notes` 的新版本，
`asset register` 退出 0；随后 `asset history` 退出 1，报
`library metadata exceeds 16 MiB`。读取该版本的搜索/审核/完整性路径都会
受到影响，后续版本登记也需要先读取已有历史。

建议修复边界：统一写入和读取的大小合同；在发布 head 前拒绝无法再次读取
的对象、head 和本地配置，保留原 head 与已有资源可查询性。若大规模历史
确需分片，应另行明确存储合同，不能仅删除读保护或无界调高阈值。

回归验收：覆盖上限内、边界和越界对象，以及包含大元数据的批量登记；
拒绝后旧 head 字节和旧资源查询不变，不能返回成功再留下不可读引用。

### R3 / P2：重复登记无法恢复缺失的本机路径绑定

位置：`packages/core/src/library/intake.rs:429–434`，关联 PR #36。

`locate` 已把调用者提供的现存来源写入内存 local config；但只有 asset head
内容改变时才持久化 local config。Git checkout 不带被忽略的 local.json，
或该文件意外丢失后，同路径、同字节再次登记命中 existing，位置记录又无需
变化，因此新的根绑定被丢弃。

复现：登记库外文件后，在临时库中移除 local.json，再用原 scan 登记。
命令返回 existing、退出 0；文件仍存在，但查询依然 unavailable，local.json
也未恢复。`project bind-root` 是现有绕过方法。

建议修复边界：独立判断本地根映射是否变化，在保持共享 head/版本幂等的同时
保存必要绑定；不应制造新版本来触发本地配置写入。

回归验收：缺失 local.json 与仅缺一个 root 两种情形，重复登记后 available，
revision、共享 head 和其他根映射保持正确。

### R4 / P2：证据位置变成空目录时，预览直接 panic

位置：`packages/core/src/library/preview.rs:213–217`，关联 PR #39。
`transfer.rs` 的证据检查也有同类 `files[0]` 假设，应在一次修复中处理。

保有审核证据创建时是文件，但读取时不能假设其类型仍不变。
`content_at` 对空目录返回空 inventory，预览直接访问 `i.files[0]`。

复现：正常记录一次审核，再把临时库中的对应 evidence .bin 换成空目录。
`asset preview` 退出 101，stdout 无 JSON，stderr 为 index out of bounds。
这违反了异常媒体应报告问题以及 CLI 运行错误使用 JSON 协议的行为。

建议修复边界：统一复验证据为普通文件，并安全检查 inventory 长度和摘要；
预览降级为缺失/已变化说明，导出和移机校验返回受控错误，不应 panic 或
把含同字节文件的目录当作有效证据。

回归验收：文件丢失、空目录、单文件目录、内容篡改分别覆盖预览/导出/校验。

### R5 / P2：公开 intake Schema 与实际 CLI 接收合同不一致

位置：`schemas/asset-intake.schema.json:19–30`，整组合并后的问题归入 PR #40。
文档 `docs/automation/project-asset-library.md:91` 指定使用此 Schema。

旧 Schema 对 items 设置 additionalProperties=false，却未添加实际实现支持的
origin、purpose、variant、role、parentRevisions。新增的
asset-library-intake.schema.json 有这些字段，但没有替换文档指定的旧合同。

复现：给 scan item 添加 `purpose:"battle"`，CLI 登记成功；按文档指定的
Draft 2020-12 Schema 校验，同一 JSON 被拒绝：purpose was unexpected。
外部 manifest 转换器或 Agent 因而无法用官方 Schema 验证合法请求。

建议修复边界：明确 scan 输出和登记 batch 的关系，统一 item 定义并更新引用，
避免维护两套逐渐分歧的属性表。保留已发布的 schema 路径或提供兼容引用。

回归验收：实际 scan 输出、全部可选字段、单项和批量请求都验证通过；
未知字段仍被拒绝，CLI 和 Schema 对声明字段的判定一致。

## 修复与合并安排建议

将 R1–R5 作为本轮固定修复清单，一次实施、一轮集中验收，不在审核中边发现
边修改。按依赖更新相应 PR；必要时只传播上游修复，不另建平行功能方案。
修复后先跑上述针对性回归，再对最终组合提交跑现有 macOS/Windows CI。
只有明确的新回归或未解决的上述问题才扩大修复范围。

PR 映射：#34 本轮未发现新增阻塞项；#35 处理 R2 的写入基础；#36 处理 R3
及 R2 intake 回归；#37、#38 本轮未发现独立新增阻塞项，但 R2 回归应覆盖
生产登记和安装关联增长；#39 处理 R4；#40 处理 R1、R5 和跨模块验证。
“未发现”不是对未执行平台/故障模型作出的全面无缺陷保证。

合并顺序保持 #34 → #35 → #36 → #37 → #38 → #39 → #40。当前分支祖先链
完整，保留提交关系的 merge commit 方式可以避免后续 PR 重复呈现上游提交；
如果使用 squash/rebase，需要明确重建并核对后续 PR 基线。每次合并后核对
下一 PR 的目标分支和实际差异，最终同步 main。此安排尚未执行。
