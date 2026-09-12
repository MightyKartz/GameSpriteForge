# Sword 反馈实施与验证

日期：2026-09-12。根据 [Sword 反馈分析](forge-sword-feedback-2026-09-12.md) 建立并实施 Goal。实施基线为 `7073ced1b37f0a618be4cc235249d222dd1e8fa9`；本报告覆盖 Forge 源码与本机个人动画技能，未升级 Sword 的工具链。

## 已实现

| 原问题 | 最终行为 |
| --- | --- |
| 安装早退丢失旧目录 | 依赖预检提前；统一事务恢复目标、项目登记、可选 catalog 和相关纹理缓存；取消子进程后再恢复 |
| Godot 错误后继续 PASS | 显式传播失败，校验退出状态、错误输出、结构化完成报告，再启动独立进程验证保存的原生资源；普通 warning 不作为失败 |
| 并发安装覆盖更新 | 项目安装锁；取锁后重算计划输入；验证 ownership、asset key 及目标目录/父子目录冲突 |
| 单动作非循环仍建议截循环 | 单/多动作路径使用实际 loop 语义；非循环不进行循环闭合判定 |
| 默认目录受显示文件名影响 | 显式 target 优先，其次验证过的 asset key，最后 Pack ID；禁止使用整个安装命名空间作为目标 |
| 来源审核哈希只存在于消费脚本 | `sourceLocks` 可选完整文件闭包，计划前及执行前校验；请求文件相对路径统一解析并存为绝对路径 |
| 回执依赖 Job store 和重复脚本 | `receipt export/verify` 保存 Job、Plan、原始 JSON 报告文本、执行者身份、来源哈希、完整 Pack 清单及可选安装/独立 review；可搬移 Pack/项目后验证 |
| 无事后安装审计 | `godot verify-install` 只读核对原 Pack、登记、ownership、usage、原生文件、纹理及 import/cache 证据 |
| 特效套用角色脚点规则 | 显式 effect profile；保留真实结构失败，提供处理前后 alpha/预乘 RGB/亮度/相邻与首尾差异；透明非循环尾帧必须显式声明 |
| 源图尺寸/alpha/格线未知 | `source inspect` 输出实际 PNG/alpha/多阈值 bounds/格线诊断，可在新目录生成双色底预览 |
| GIF 与原生时长容易混淆 | GIF timing sidecar 同时报告名义 FPS、编码量化时长与原生时序；8 fps 的 125 ms/130 ms 差异明确可见 |
| 旧个人动画 skill 引用已移除 examples | 普通本地流程使用选定 CLI；历史路线校验显式 checkout/commit 和全部所需 example，在写产物前失败 |

Godot 动画 scene 同时改为引用交付的独立 SpriteFrames 文件，使 scene 和 `.tres` 共享同一原生资源。新增通用 [外部时钟示例](../../examples/godot/forge-external-clock/README.md)，覆盖 pause、非循环结束、循环取模和非均匀帧时长；未搬入 Sword 的战斗或存档规则。

## 额外复现并修复的缓存错误

真实 Godot 4.6.3 首装红图 A，再更新蓝图 B，并仅让验证阶段失败。旧实现已恢复目标 PNG、`.import`、scene、ownership 和登记的全部字节，但新 runtime 进程仍加载蓝图：`.godot/imported` 的 `.ctex`/`.md5` 尚未恢复。只有再次显式 import 才读回红图。

最终事务按完整 `res://` 纹理路径识别缓存范围，保存旧缓存和权限，在新源图复制后、import 前登记新增范围；失败恢复旧缓存并清理本次新增缓存，不恢复其他素材的缓存。MD5 仅用于匹配 Godot 的路径命名约定，完整性仍使用 SHA-256。真实回归要求失败后不补跑 import，新进程直接加载到 A 的相同 RGBA 哈希。

安装 snapshot v2 将稳定目标文件、import 路由和缓存文件分开记录，并由项目登记保存其 SHA。缓存被改动会失败；搬移项目后缓存缺失可以通过稳定资产检查，但明确返回 `not_materialized`，需要正常导入后才能原生加载。审计不会写缓存，也不声称运行了新的原生或视觉验证。

完整矩阵还揭露既有 world 安装问题：terrain 的 atlas source 尚未加入 TileSet，就设置 TileData 的 terrain/custom data。Godot 报 `tile_set is null`，旧安装器仅检查退出状态，曾将其当成成功。修复将 source 绑定移到 tile 配置之前，并在原生验证中核对 terrain、自定义数据和碰撞；保留错误输出检查。

继续运行还发现 building 碰撞/交互 shape 在父节点加入场景前设置 owner，Godot 报 `Invalid owner`。同样按节点加入顺序修复，验证保存后的 shape，避免依赖退出码掩盖未交付的子资源。

## 回执与技能边界

`prepare.execution:null` 保留旧 Job 的未知生产者，当前 exporter 不填补历史身份。报告的 `recordedProducerSha256:null` 同样保留旧产物未记录生产者哈希这一事实；新的嵌入文本哈希只证明导出时字节。可变的项目/catalog 登记文件不会冒充旧 Job 报告，安装回执仅保留相关登记项。保存外部 receipt SHA 并使用 `--expected-sha256` 才能绑定整份证据；内部一致性检查不等于签名。

个人技能备份在 `/Users/kartz/Development/Forge-skill-backup-20260912/forge-character-animation`，6 个文件更新，10 项行为测试及 skill validator 通过。独立 forward test 使用合成三帧确认逐像素 RGBA、固定锚点、非循环和 `70/150/230 ms` 完整保留，Provider 请求为 0。首轮正确检测到并行源码构建替换执行文件；保留失败记录后，第二轮固定同哈希副本并完整通过。个人技能属于本机安装，不作为 Forge 的额外产品源码复制到 Git。

新的用法集中在 [唯一内置 delivery 指南](../../.agents/skills/forge-use/references/delivery.md) 和 [动画指南](../../.agents/skills/forge-use/references/animation.md)，通过 `forge guide delivery/animation` 读取。增加对应 build capabilities；现有 v0.3.2 发布归档不因此获得新能力。本次同步源码，不发布新版本，不改 Sword 两份 v0.3.0 锁和已有素材回执。

## 验证记录

发布矩阵共 9 组检查最终通过：完整运行首轮 7 组通过，静态路径断言与 world 初始化修复后分别定向复跑通过；这不是重新整轮运行的 9/9。两次失败现场、原始矩阵和定向复跑记录均保留。共享安装脚本在 world 修复后另跑 3 项真实 Godot 回归，全部通过。

| 检查 | 结果 |
| --- | --- |
| Rust workspace、格式与 Clippy | 318 项通过；4 项真实 Godot 测试在对应 gate 单独运行并通过；最终所有 CLI features 的 Clippy 通过 |
| 回执最终定向测试 | 6 项通过，包含最后新增的可选 `review.notes` 类型负例；schema 与导出/验证行为一致 |
| 新 CLI 交付合同 | 15 场景 / 49 次命令通过，真实 Godot 未跳过，Provider 请求 0 |
| 内置 guide/skill 合同 | 33 场景 / 124 次命令通过；最终指南 skill validator 通过 |
| 本机个人 skill | 10 项行为测试、独立 forward test 及再次逐字节复核通过 |
| 其他合同 | game-art manifest、installer、signing、7 项 directional reuse 与 3 份真实回执 schema 验证通过 |

结果与测试构建身份记录在 [validation.json](artifacts/forge-sword-feedback-implementation-20260912/validation.json)。主验收目录为 `target/qa/sword-feedback-implementation-20260912/`，完整 Job、合成媒体和日志不进入 Git。独立 forward test 的非媒体报告已复制到该目录的 `independent-skill-forward/`；原始临时目录和各报告 SHA 见其中 `SOURCE.json`。额外 world 失败现场保留在 `target/qa/forge-world-product.*/`。

安装事务只对返回的错误和协作取消执行恢复；不宣称断电或 SIGKILL 后自动恢复。回滚若因文件系统错误失败，会返回恢复错误并指出原始备份。Forge 的项目锁不锁住外部编辑器；共享缓存的其他 Godot 导入/导出仍须序列化。结构、原生加载、视觉接受和真机测试始终分开。
