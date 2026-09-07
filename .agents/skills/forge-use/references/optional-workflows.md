# 可选工作流

先检查所选 binary 的 `--help` 并执行目标操作的计划预检。下表 feature 名用于源码
构建定位，不代表当前发行版一定提供；缺失时说明能力缺口，保留用户选定路线。
不要自动 `--all-features` 构建或改用其他工作流来绕过错误。

| 需求 | 源码 feature / 先决条件 |
| --- | --- |
| Subject、Character V2 | `consistency-v2`，已有 Style |
| 已批准 canonical 导入 | `subject-import`，含 Subject 能力 |
| Collection、静态 V2、Portrait/Equipment/Decal、导出替换与审计 | `collection-assets` |
| Grid 角色 | `grid-generation`，实验工作流 |
| Environment/Terrain → Building → Map | `terrain-assets` → `building-assets` → `map-compiler` |
| 项目 manifest 批量构建 | `game-art-manifest` |

有仓库源码时，可从 `examples/cli` 复制对应版本的 spec 到任务目录，再填写真实参考
路径和锁 revision。示例内可能有 `REPLACE_WITH_*_REVISION` 或不存在的 anchor 文件；
未替换不能执行。`schema list/show` 也受 feature gate 控制，仅在帮助中存在时使用。

## Subject 与 Character V2

已有 Subject 先 list/inspect；需要新主体才生成：

```bash
forge subject list --project PROJECT --json
forge subject create --project PROJECT --spec SUBJECT_SPEC --plan-only --json
forge subject inspect --project PROJECT --id SUBJECT_ID --revision REVISION --json
```

create 计划需按执行参考执行成功后才有新 revision。不要虚构 revision 或手改锁。
已批准的透明单主体 PNG 可用本地导入，spec 不同时带 referenceImages 内容：

```bash
forge subject import --project PROJECT --spec SUBJECT_SPEC --canonical /absolute/canonical.png \
  --approval-note "该 canonical 的已有批准依据" --plan-only --json
```

Provider Character V2 的稳定目录路线为 `topdown-video@2.0.0`，使用 schemaVersion 2
spec、真实 Subject revision 和显式 camera/equipment。可以先选一个方向验证：

```bash
forge generate character --project PROJECT --spec CHARACTER_V2_SPEC \
  --validation-animation walk_right --plan-only --json
```

验证通过后按任务范围计划完整角色。validation Job 不导出部分 Pack；不要承诺
后续完整生成会自动复用所有验证素材或免除费用，以新计划为准。
keyframe/keypose/spritesheet/Grid 等版本具有各自输入、审核与预算要求，不因版本号高就
默认替代 stable 路线。基础 schemaVersion 1 角色走 legacy，与上述路线不同。

## 外部四关键帧 V11

`topdown-external-keyframes@11.0.0` 经
`forge plan prepare-character --request REQUEST --json` 进入本地加工，不请求 Provider。
它只接受独立、真实透明、相同正方形画布的 PNG；每个声明 idle 一帧且不循环，walk
四帧且不可字节相同。不要用 sprite sheet 或视频冒充这条输入协议。

request 必须给出该版本要求的 characterPrompt、cameraProfile、equipment 和 rendering。
镜像需显式选择 `right_only`、`explicit_left` 或 `mirror_right_to_left`，并提供匹配动作，
不能默认为用户镜像左方向。保留作者共享坐标时使用基础参考中的完整 preserve_canvas 对象。
源码示例为 `examples/cli/character-external-keyframes-v11.json`；它包含待填的绝对路径，
不是任何外部动画素材都可直接执行的通用模板。

## Collection 与 Portrait

先固定 Style 和 Collection。Collection 指向真实 anchor 时可走本地路线；需要生成
anchor 则可能收费，以计划为准：

```bash
forge collection create --project PROJECT --spec COLLECTION_SPEC --plan-only --json
```

Portrait 还需要 Subject。V2 spec 要求 `schemaVersion: "2"`、明确的 framingProfile，
并把 neutral 放在表达列表首位。bust 与 full-body 不能混用；full-body 需匹配脚部落地
约束的 Collection。已有 `stage3/portraits.json` 是 V1，不能直接当成 V2 两阶段 spec。

```bash
forge generate portrait-set --project PROJECT --spec PORTRAIT_V2_SPEC \
  --phase base --neutral-reference-policy subject-style --plan-only --json
```

执行 base 计划后查看并审核 neutral；批准该具体候选后记录 `job review`，再计划：

```bash
forge generate portrait-set --project PROJECT --spec PORTRAIT_V2_SPEC \
  --phase expressions --base-job APPROVED_BASE_JOB --plan-only --json
```

表达阶段只使用已批准 base。neutral 不用普通 item retry 改写；需要变更身份底图时
创建新的 base 流程。Equipment/Decal 使用各自 `generate equipment-set`/`generate decal-set`
和匹配的 spec，不能以改写 kind 的方式绕过输入规则。

可编辑交付和单项替换使用：

```bash
forge asset export-editable --project PROJECT --id ASSET_ID --output /absolute/editable --json
forge asset replace-item --id SOURCE_JOB --item ITEM --path /absolute/replacement.png --wait --json
```

replace-item 的 `--id` 是来源 Job ID，本地替换创建 child Job，核对零请求及新 Pack。

## Grid、World 与项目批量构建

Grid 按匹配的版本先生成 image locks、审核后再 complete，保留相同 Subject、camera、
model 与已批准引用。`--direction-motion-stage` 和 `--image-lock-job` 的使用以该版本
合同和计划为准；不要将单方向探针成功表述为完整多方向角色验收。

World 依次固定 Environment、生成 Terrain/Building，再编译 Map。使用帮助中可用的
`environment create`、`generate terrain-set`、`generate building-kit`、`map compile`
及其匹配 spec；不要把工程生成/安装成功解释为地形衔接和建筑视觉质量已获批准。

同一资产项目的批量增量构建可用：

```bash
forge project diff --project PROJECT --manifest /absolute/game-art.json --json
forge project plan-build --project PROJECT --manifest /absolute/game-art.json --json
```

核对 build/reuse/skip、依赖、失效原因和预算，再执行返回令牌。
输入或 catalog/Pack 哈希漂移时重新规划，不修改计划以强行复用。
这些流程的批准与真实请求范围仍遵守[执行与交付](jobs-and-delivery.md)。

## 已批准方向动画的零生成整合

已有 right/down/up 恢复 Pack，且任务要求复用现有帧时，可使用仓库的
[三方向复用辅助流程](../../../../docs/architecture/forge-existing-animation-reuse-plan.md)。
它校验原始批准与帧时长，通过 CLI 安装到独立 Godot 审核项目，再测试整段固定倍率。
这是受限定输入约束的实验工具，不是默认 CLI 的通用多方向合成命令。
保留源帧、源 Pack 和原生时序；来源批准不自动覆盖新整合效果，数值尺寸匹配也不代表
视觉通过。此流程不会调用 Provider、生成左向或补出 idle。
