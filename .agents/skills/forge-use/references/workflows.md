# 基础工作流

以下命令的 `forge` 均指本次选定的同一个可执行文件；有需要时用其绝对路径替换。
命令中的路径、ID 和令牌必须使用本次任务的真实值。JSON 示例写入任务自己的 spec 文件，
不要修改仓库的 fixture 模板或把示例素材当成用户最终需求。

## 1. 发现与检查

```bash
forge --version
forge --help
forge doctor --json
forge provider list --json
forge profile character-workflows --json
```

`doctor` 返回的 `godotSupported`、媒体工具路径等需要逐项查看；`ok:true` 并不代表
所有依赖齐全。`provider list`/普通 doctor 不主动检查 Keychain，返回的认证状态不能
单独证明用户没有凭证。确需认证诊断时使用 `provider doctor --provider PROVIDER --json`。
`provider models --provider PROVIDER --json` 是静态路由信息，不是账户权益检查或价格报价。

先看 `forge SUBCOMMAND --help` 再使用可选命令；`schema` 也可能被 feature gate 关闭。
工作流列表可能含当前构建不能执行的条目，最终以目标路线的计划预检为准。

已有项目先检查：

```bash
forge project inspect --project /absolute/game-art --json
forge asset list --project /absolute/game-art --json
forge style inspect --project /absolute/game-art --json
```

只对新资产项目执行初始化，显式选择用户要求的 Provider/profile。
`project init` 的 Provider 默认是 xAI，离线演示必须显式指定 fixture：

```bash
forge project init --path /absolute/game-art --name "My Game" --provider fixture --profile default --json
```

fixture 的图像是确定性测试素材，可能只是几何占位形状；即使 spec 名为药水/钥匙，
也不能据文件名或质量通过宣称视觉语义已经完成。离线演示记录流程结果，实际图像如实展示。

## 2. Style → 图标/道具

最小 Style spec 示例：

```json
{
  "schemaVersion": "1",
  "prompt": "compact jewel-tone pixel art with dark outlines",
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

按用户需求调整；真实参考文件必须存在。没有可复用 Style 时先规划，再按
[执行与交付](jobs-and-delivery.md) 完成该 Style Job：

```bash
forge style create --project /absolute/game-art --spec /absolute/specs/style.json --plan-only --json
forge plan execute --token STYLE_TOKEN --wait --json
forge style inspect --project /absolute/game-art --json
```

上述执行命令可直接用于 fixture；真实 Provider 的执行还要满足执行参考中的预算、
授权和认证条件。生成 Style 本身也可能消耗请求，不要只计算后续素材请求。

图标 spec 示例：

```json
{
  "schemaVersion": "1",
  "kind": "icon_set",
  "id": "inventory-icons",
  "name": "Inventory Icons",
  "items": [
    { "id": "potion", "name": "Potion", "prompt": "a red healing potion" },
    { "id": "key", "name": "Key", "prompt": "a small brass key" }
  ],
  "license": "private"
}
```

道具使用 `kind: "prop_set"` 和对应命令。许可和素材名称按任务填写，不推断版权授权。

```bash
forge generate icon-set --project /absolute/game-art --spec /absolute/specs/icons.json --plan-only --json
forge plan execute --token ASSET_TOKEN --wait --json
```

道具命令为 `forge generate prop-set --project PROJECT --spec SPEC --plan-only --json`。
需要 Collection、Portrait、Equipment 或 Decal 时进入[可选工作流](optional-workflows.md)。

## 3. 角色路线

- 基础二进制的 schemaVersion 1 角色 spec 走 legacy `topdown@1.0.0`。仅在任务选择
  该路线时使用，不能为绕过 feature 错误自动降级。
- Provider Character V2 需要 SubjectLock 和对应能力，见可选工作流。单方向验证
  不导出部分角色 Pack。
- 已有透明帧、sprite sheet 或视频时，使用本地 `plan prepare-*`，不必重新向
  Provider 获取相同内容。外部 V11 四关键帧协议是特定输入合同，不是通用视频入口。

明确使用基础角色路线时，其最小 spec 形状如下，命令为
`forge generate character --project PROJECT --spec SPEC --plan-only --json`：

```json
{
  "schemaVersion": "1",
  "kind": "character",
  "id": "ranger",
  "name": "Ranger",
  "prompt": "a compact forest ranger with a green hood",
  "license": "private"
}
```

## 4. 已有素材与 Pack

已有 Pack 优先 `forge pack validate --path PACK --json`、`forge asset inspect --pack PACK --json`，
之后按交付要求安装。不要为了安装而再生成或重新包装。

单动画的本地加工例，假定用户已提供真实透明 PNG：

```json
{
  "schemaVersion": "1",
  "input": {
    "kind": "png_sequence",
    "paths": ["/absolute/frames/0.png", "/absolute/frames/1.png"]
  },
  "metadata": { "name": "Authored Animation" },
  "normalize": {
    "mode": "preserve_canvas", "marginBottom": 0, "margin": 0, "alphaThreshold": 0
  }
}
```

```bash
forge plan prepare-asset --request /absolute/asset-request.json --json
forge plan execute --token LOCAL_TOKEN --wait --json
```

`prepare-asset` 支持 PNG sequence、sprite sheet、video clip 和 Pack 输入；PNG sequence
至少两帧，不能当成任意单张 PNG 导入器。多动画使用
`forge plan prepare-character --request /absolute/character-request.json --json`，至少两个
animation；普通 PNG 循环至少两帧、非循环至少一帧，且不接受 Pack 合并。

`--request` 与 `--stdin` 二选一。低层 request 的媒体路径全部绝对化，它不会自动以
request 所在目录重定位。共享作者坐标时用完整的 `preserve_canvas` 参数对象；
四个字段都要写，不能只写 `mode`。视频/切图配置必须来自实际输入信息，先计划校验。
这类加工不调用媒体 Provider，报告中的请求证据仍需核对。

## 5. 存储与重用

普通使用沿用现有 Job/Plan store，使后续查询和重试能找到原 Job。只有离线演练或
用户要求隔离时，才在自己的临时根目录设置 `FORGE_JOB_STORE`、`FORGE_PLAN_STORE`、
`FORGE_AUTHORIZATION_STORE`；所有后续命令沿用相同设置。
不要清理用户原 store，或在运行任务中途换 store 后重新生成素材。
