# 执行、审核、重试与交付

## JSON 与计划

带 `--json` 的正常命令输出一个 JSON envelope：成功为 `ok:true,data`，执行错误为
`ok:false,error:{code,message}`，并返回非零退出码。`--help` 和参数解析错误由 Clap
处理，不保证输出 JSON。读取 stdout 和退出码，保留 stderr 诊断。

注意字段命名不同：

| 输出 | 关键字段 |
| --- | --- |
| Plan | `data.token`、`expiresAt`、`inputFingerprint`、`recipeHash`、`estimate`、`effects` |
| 执行/`job get` | `data.job_id`、`lifecycle_state`、`artifacts`、`next_actions`、`error_code` |
| `job report` | `data.job.lifecycle_state`、`data.reports`、`providerRequestOccurred`、`providerRequestCount`、`providerCostInUsdTicks`、`authorizationAccounting` |

上表每行第一个路径后的同级字段位于相同对象内。不要把 Plan 的 camelCase 套在
JobRecord 的 snake_case 上。

`--plan-only` 保存本地 pending token；不会创建执行 Job、请求媒体或改动 Godot。
保存整个 Plan 响应，检查 `estimate.providerRequestEstimate` 与 `maximumProviderRequests`，
以及模型、工作流和 effects。以响应中的 `expiresAt` 为准，令牌仅能消费一次。
输入或规格改变后重新规划；不要手改令牌文件、指纹或哈希。

## 真实 Provider 与预算

已有针对本次 Provider、模型、目标和预算的明确授权可直接承接，无需每一步重复确认。
缺少范围或上限时先完成可执行计划，再提出具体预算。计划只有请求估计，不能当作实时
价格报价；使用可验证的报价/计费资料确定成本上限与每请求预留，不把未知费用写成零。

登录解决凭证身份，持久 authorization 约束消费。使用 CLI 的登录/凭证存储流程，
不让用户把密钥粘贴进聊天、spec、命令记录或输出文档。创建授权前读取
`forge provider authorize --help`；复用前检查
`forge provider authorization --id AUTH_ID --json` 中的清单、到期时间与账本。

基础路线的授权范围：

| 路线 | target | model 来源 |
| --- | --- | --- |
| Style 图像生成 | `style_board` | StyleSpec 的 `imageModel`；未声明时查当前 binary 的默认图像 route |
| Icon/Prop V1 生成 | 原始 `spec.items[].id`，不加资产前缀 | 本次 Plan 的 `estimate.model` |
| Icon/Prop V1 单项重试 | 所选 item ID | 本次重试 Plan 的 `estimate.model` |

Style Plan 当前保守估计 1/1，且不返回 model；参考图本地组板或缓存命中可能实际零请求。
必要时在规划前明确 StyleSpec 的 `imageModel`，不能把估计量写成必然收费。
静态素材模型不一定继承 StyleLock，以本次 Plan 为准。复杂 Character 的 targets
随版本和阶段变化，需匹配该版本流程或来源账本，不能仅从 animation 名推导。

每请求预留是调用者批准的成本边界，不是 CLI 自动报价。没有可靠费用依据或单位换算时，
保留已完成的计划，列出已知目标/模型/请求范围和缺失成本信息；不照抄历史金额或随便
填大数。账本中尚未结算或响应不确定的请求仍会占用预留，不能假定失败就没有消费。

下面是授权形状，所有变量须来自已经批准的具体范围，不是默认预算：

```bash
forge provider authorize --provider "$PROVIDER" --profile "$PROFILE" --id "$AUTH_ID" \
  --target "$TARGET" --model "$MODEL" \
  --max-requests-per-target "$PER_TARGET" --max-requests "$MAX_REQUESTS" \
  --max-cost-ticks "$MAX_COST_TICKS" \
  --cost-reservation-ticks-per-request "$RESERVATION_TICKS" --json
```

按目标/模型重复相应参数；目标必须匹配该工作流的实际媒体阶段，不能从素材显示名猜测。
有精确范围要求时还需 `--source-job`、`--max-provider-operations`，或成对的
`--recipe-hash`/`--input-fingerprint`。遵守该工作流返回的限制；错误不能通过放宽授权掩盖。

当前 CLI 的收费执行预检仍要求以下三个环境值，即使传入 `--authorization`：

```bash
FORGE_REAL_PROVIDER_ACCEPT=1 \
FORGE_REAL_PROVIDER_MAX_REQUESTS="$MAX_REQUESTS" \
FORGE_REAL_PROVIDER_MAX_COST_TICKS="$MAX_COST_TICKS" \
forge plan execute --token "$TOKEN" --authorization "$AUTH_ID" --json
```

它们只在已有批准范围内设置，不修改用户全局 shell 配置。请求/费用上限需为正整数；
环境变量不是跨任务总账，仍以持久授权账本约束累计消耗。重试不能重置预算，也不能为
绕过耗尽账本自行换授权 ID。fixture 和本地加工不需要真实 Provider 的这些环境值。

## Job 生命周期

```bash
forge job get --id JOB_ID --json
forge job report --id JOB_ID --json
forge job graph --id JOB_ID --json
```

并非所有 Job 都有图；`workflow_graph_missing` 表示该 Job 没有可用图，不等于素材失败。
此时继续通过 `job get/report` 和 artifacts 检查，不能构造不存在的 replay 节点。

- 默认异步返回 Job，短任务可执行时加 `--wait`。`queued`/`running` 时按适当间隔继续查，
  保持用户知情；不要用重复生成代替等待。
- `awaiting_review` 应进入审核。`--wait` 也可能返回这个状态，不能据 `ok:true` 判完成。
- `succeeded` 后检查实际 artifacts；锁定阶段成功可能没有 Pack。
  单个 step 的状态不能替代 Job 生命周期和产物验证；发现不一致时如实记录，不手改 Job。
- `failed`/`cancelled` 先检查 `error_code`、报告和 `next_actions`。
  按用户要求取消用 `forge job cancel --id JOB_ID --json`，取消是协作式，需确认最终状态。
- `job report` 不请求 Provider，可含当前实现的本地诊断；诊断不会改写历史判定。
  实际请求/费用结合该 Job、来源链和授权账本解读，字段缺失不等于无成本。

## 审核

先检查报告和实际候选，确认审核对象、动作/版本及需要人工判断的内容。
已有对同一候选的批准就记录该决定；工作流要求人工批准但尚无决定时先展示候选。
预算授权不代表视觉批准，不能编造审核人或理由。

```bash
forge job review --id JOB_ID --accept --reason "具体的已批准审核结论" --json
```

这会写入审核决定，可能导出 Pack、登记资产，也可能只批准底图/方向锁。
查看新的 `lifecycle_state`、`artifacts` 和 `next_actions` 后继续。
`blocked`、`regenerate`、损坏媒体等硬门禁不能靠 accept 绕过。

拒绝审核使用同一命令但省略 `--accept`，没有 `--reject` 参数。
拒绝已成功的静态素材可能将其 catalog 标为隔离，不能当作只读检查执行。

## 定向修复

先读报告，再为最小范围生成计划：

```bash
forge job retry --id JOB_ID --item ITEM_OR_ACTION --stage STAGE --plan-only --json
forge job retry --id JOB_ID --item ACTION --stage frame --frame INDEX --plan-only --json
```

| 路线 | 请求性质 |
| --- | --- |
| Character `still`/`video`/`frame` | 生成或编辑媒体，可能收费 |
| Character 明确 `loop`/`matting`/`consistency` | 使用留存媒体，本地重处理 |
| Static `auto --item ITEM` | 重做该 item，可能收费 |
| Static `consistency` | 本地复检 |
| `auto` | 按证据选阶段，不能默认免费 |
| `job replay --from NODE` | 取决于节点，生成节点仍会请求 Provider |

实际可用 stage/frame 取决于工作流。CLI frame 参数范围为 0–7，但不能据此假定每个
动作有八帧。child 保留来源链与未选中的已接受素材，完成后用报告和哈希核对。

`job graph` 提供真实节点 ID。`job replay --id JOB_ID --from NODE --wait --json`
没有 `--plan-only` 或 `--authorization` 参数；不清楚节点的费用边界时，先用对应
`job retry ... --plan-only` 规划，不盲目 replay。

## Pack 与 Godot

从 Job artifacts 中找到 `kind == "gsfpack"` 的路径；不要猜目录或以中间预览替代 Pack。

```bash
forge pack validate --path /absolute/asset.gsfpack --json
forge asset inspect --pack /absolute/asset.gsfpack --json
```

`data.valid:true` 证明结构/合同通过，视觉验收另记。角色需要动作诊断时可用
`forge pack audit-motion --path /absolute/asset.gsfpack --json`，这不请求 Provider。

Godot 项目必须存在 `project.godot`，doctor 中引擎须受支持（当前安装器要求 4.6.x）。
目标在 `addons/forge_assets` 下，已有目标必须为 Forge-owned。安装命令：

```bash
forge godot plan-install --pack /absolute/asset.gsfpack --project /absolute/godot-game \
  --target addons/forge_assets/inventory --asset-key inventory --json
forge plan execute --token INSTALL_TOKEN --wait --json
forge project inspect --project /absolute/godot-game --json
```

要将安装关联回已有 Forge catalog，可在计划时增加 `--catalog-project /absolute/game-art`；
先确认对应 catalog 中有该资产。独立 Pack 安装可以省略它。
只有用户要求新 Godot 项目时才创建新项目；不要改写现有游戏的 `project.godot`。

安装后检查安装 Job 成功、实际资源和 `forge_usage.json`、项目登记，以及外部纹理引用。
区分 Pack 内相对路径与 Godot 项目相对路径；不要把 usage 中每个 texture 字段直接
拼接到安装目录，需核对实际资源引用和文件存在性。
`.tres/.tscn` 小于 1 MiB，不能含内嵌 Image/PackedByteArray 或 `ImageTexture.create_from_image`。
需要视觉引擎验收时实际打开/渲染查看，单纯安装成功不代表玩法或渲染已经验收。
普通安装失败会尝试恢复资源和登记；若报告恢复失败，保留 Job/备份并报告位置，
不要靠删除目录后重试掩盖问题。

当前 binary 有 `project audit` 时可运行
`forge project audit --project /absolute/game-art --scope godot --json`。
该命令需要 Collection 能力，指向 Forge 资产项目，不能在无此命令的基础构建中强制要求。
