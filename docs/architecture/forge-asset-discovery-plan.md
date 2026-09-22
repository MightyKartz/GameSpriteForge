# Forge CLI 资源发现与需求对账实施计划（P0–P2）

日期：2026-09-22。状态：实施中。源码基线：`7c47397`（main，release v0.6.2 准备提交）。

背景：Codex 出图后经 Forge 整理交付 Godot，创造性工作消耗模型 token，整理与交付由 CLI 完成。当前缺口在于 Codex「找素材」和「确认缺什么」的成本：搜索每次全量核验媒体字节、没有标签词表可查、预览产物缺少机器可读媒体清单、库里「有什么」和游戏「要什么」之间没有对账入口。本计划覆盖此前分析中的 P0–P2；语义向量搜索、跨项目联合搜索、自动打标与 MCP 恢复均不在范围内。

## 0. 现状盘点

| 现有实现 | 与本计划的关系 |
| --- | --- |
| `packages/core/src/library/index.rs` | 已有可重建的元数据缓存（避免重复解码 asset/review 对象），但不跳过媒体字节核验 |
| `intake::search` / `hit()` | 每个命中都对已知位置做 `content_at` 全量内容检查，大媒体库下每次查询都是重 IO；状态过滤只认 `available/changed/unavailable` |
| 查询词 | `query` 已对 ID/名称/Pack 成员做子串匹配；kind/tag 为精确匹配，但没有任何命令能列出库里已用的 kind/tag/purpose 词表 |
| `preview.rs` / `preview.json` | 预览目录复制了媒体并写 HTML，报告只有数量；调用方（Codex）要自己遍历目录才能定位具体版本的媒体文件 |
| `game-art-manifest.schema.json` | 声明「要生成什么」，面向生产且强制 Provider；不回答「对照资源库，哪些已有合格版本可复用、哪些真正缺失」 |

计划文档已预留「查询索引可以重建」与「后续候选」，本计划不引入必须提交 Git 的数据库，不自动推断标签/用途/谱系，不改变既有审核与安装语义。

## 1. PR 拆分与契约

依赖顺序：PR A → PR B → PR C，依次叠放审核；每个 PR 独立可用。能力标识随实现加入 `doctor --json` capabilities。

### PR A（P1）：词表命令与免核验搜索

交付 `forge asset tags` 与 `forge asset search --metadata-only`。

```sh
forge asset tags --project ./library --json
forge asset search --project ./library --kind image --metadata-only --json
```

- `tags` 只读输出词表：资源数、版本数，以及按 kind / tag / purpose 分组的计数，按计数降序、名称字典序稳定排列。纯元数据读取，不触碰媒体字节，不创建或修复索引缓存。
- `search --metadata-only` 跳过 `hit()` 中的 `content_at` 字节核验：命中的 `status` 如实报为 `unknown`，且该模式与 `--status` 过滤互斥（拒绝而非静默忽略）；Pack 成员列表仅来自版本记录的来源元数据，不再从可用目录现查。响应结构与现有 `search` 完全一致。
- 能力：`project_asset_vocabulary`、`project_asset_metadata_search`。

验收：合成库（含缺失源与多 Pack）下词表计数准确；`--metadata-only` 不做媒体读取（缺失源不报 unavailable 而报 unknown）；与 `--status` 组合明确报错；现有 search 契约测试不变。

### PR B（P2）：预览媒体清单

交付预览报告中的机器可读媒体路径。

```sh
forge asset preview --project ./library --id hero --revision REV --out ./preview --json
```

- `preview.json` 与 CLI JSON 响应新增 `media` 数组：逐项列出 `assetId`、`revision` 与其复制后的媒体文件（预览目录内相对路径 + 原始标签/成员名），Codex 无需遍历目录即可直接打开文件抽查。缺失源仍只进 `issues`，不伪造条目。
- 清单只描述本次预览复制的既有字节，不生成新的派生媒体，不改变 HTML 页面合同。
- 能力：`project_asset_preview_media_manifest`。

验收：图片/音频/Pack 混合预览的清单与实际复制文件一一对应；缺失源条目不出现；现有预览测试与页面转义测试不变。

### PR C（P0）：需求对账命令

交付 `schemas/asset-requirements.schema.json` 与 `forge asset check-requirements`。

```sh
forge asset check-requirements --project ./library --input needs.json --json
```

- 需求批：`{schemaVersion:"1", kind:"asset_requirements", requirements:[{id, kind?, tags?, purpose?, query?, reviewDomain?, reviewVerdict?}]}`。`id` 必填且批内唯一；`tags` 为全部满足；`reviewDomain/reviewVerdict` 必须成对出现，取值与现有审核域一致。
- 对账逻辑只读执行：每条需求按现有 search 语义匹配（完整字节核验，不引入新的匹配规则），逐条给出：
  - `covered`：存在可用版本且满足审核要求；
  - `needs_review`：存在可用版本但审核要求未满足；
  - `incomplete`：有匹配但无可用版本（changed/unavailable）；
  - `missing`：无任何匹配。
- 每条命中附 revision、selected、status、reviewStates；每需求最多返回 5 条命中（按现有 search 排序），并报告总匹配数。
- 对 `missing` 且 kind 为 `icon_set`/`prop_set` 的需求，附带预填的本地静态生成请求骨架（id/name 已填、路径占位），供 Codex 出图后直接改用；其他 kind 不虚构模板。模板不参与指纹，不产生任何写操作。
- 该命令不复用 game-art manifest：无 Provider 字段、不做生成、不做复用指纹判断。「covered」只说明库存与审核状态，不等于风格或视觉批准。
- 能力：`project_asset_requirements_check`。

验收：四种状态各有合成 fixture 覆盖；review 过滤不成对、重复 id 有确定报错；模板仅对支持的 kind 出现；全程零 Provider 请求、零写操作；旧契约测试不变。

## 2. 文档与测试

- 更新 `docs/automation/project-asset-library.md` 与内嵌 `forge-use` 的 `references/project-assets.md`（同一来源，随 CLI 发布）；schemas 目录登记新 schema，examples 提供需求批示例。
- 扩展 `scripts/test-asset-library-cli.py` 覆盖三条命令，Rust 侧为词表、免核验搜索、需求对账补单元测试；工作流路径触发规则补充新增文件。
- 中英文产品描述同步；能力标识以 `doctor --json` 为准。

## 3. 明确不做

- 不恢复 MCP、不引入语义/向量搜索（词表稳定后再评估）。
- 不在登记时自动推断标签、用途、生成器或谱系。
- 不改变 search 默认行为、安装事务、审核与锁的既有语义；`--metadata-only` 为显式可选。
- 不做跨项目联合搜索、需求批的持久存储或自动清理。

