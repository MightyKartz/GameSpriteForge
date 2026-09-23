# Local ComfyUI image and video delivery (development)

This development checkout can import and diagnose a **single, explicit
API-format workflow**, generate a transparent PNG or H3-style I2V MP4,
prepare a static or character Pack, and resume after visual review for Godot
installation. A single-reference Qwen image edit is also supported with an
explicit `referenceInput` mapping. The staged contract is in
[the implementation plan](../architecture/local-comfyui-agent-delivery-plan.md).
Forge does not install ComfyUI, custom nodes or model weights.

Export the workflow using ComfyUI's **Save (API Format)** action. The usual
canvas/workflow JSON has a `nodes` array and `links`; it cannot be sent to
`POST /prompt`. API JSON is an object keyed by node IDs; each node has
`class_type` and `inputs`. Inspect that export and identify the exact node and
input containing the positive prompt, plus the one output node. No node ID is
inferred from its position or title.

Create a profile file beside the API JSON. Replace all values below with those
from your own installed workflow:

```json
{
  "schemaVersion": 1,
  "endpoint": "http://127.0.0.1:8188",
  "mediaKind": "image",
  "modelId": "user-declared-qwen-image-2.1-model",
  "workflow": "./my-qwen-api.json",
  "promptInput": {"node": "459:452", "input": "prompt"},
  "outputNode": "461",
  "outputField": "images",
  "maxOutputBytes": 67108864,
  "timeoutSeconds": 300
}
```

The IDs above came from one Qwen Image 2.1 API export; they may differ in your
workflow and must be checked, not copied blindly. Video profiles use
`mediaKind: "video"`, a `referenceInput` node/input mapping for I2V, and the
actual result field from `/history`. In the locally tested H3 workflow,
`SaveVideo` returned an MP4 under `images`, despite its filename extension.
Set `outputNode` to that `SaveVideo` node and `outputField` to `images`. Save a separate profile for
each workflow and mode. ComfyUI normally serves on port 8188; use the port your
launch script actually selects.

```text
forge provider configure --provider comfyui --profile local-qwen --config /absolute/profile.json --json
forge provider doctor --provider comfyui --profile local-qwen --json
```

`configure` checks the workflow structure and stores its absolute path and hash.
It returns the same profile if identical; a changed profile requires a new ID,
so an existing Job cannot silently switch workflows. `doctor` checks the live
`/object_info` node classes. It does not execute a generation request or prove
that model weights are present, the output type is correct, or the artwork is
usable. The endpoint defaults to loopback only; `allowRemote: true` is required
to select a different host. Remote endpoints remain subject to the user's own
network and data handling decisions.

Copy the bundled request with `forge guide comfyui-image-example > image.json`
and edit the explicit prompt, profile, identity and processing fields. Optionally
add `godotProject` (absolute existing project), `installTarget` (for example
`addons/forge_assets/blue_potion`) and `assetKey`. To use an already generated
local image instead, replace `workflowProfile` and `prompt` with an absolute
`source` PNG path. Both routes use one request contract and preserve the source.

```text
forge asset create --input /absolute/image.json --wait --json
forge asset create --resume JOB_ID --wait --json
```

The first call returns a durable `jobId`, `sourceSha256`, `previewPath` and
`packPath` after processing. Without `--wait`, it submits the ComfyUI prompt and
returns promptly; resume polls only that prompt ID. Inspect the preview and
child preparation Job. A technical Pack pass is not visual approval. For an
approved result, write `review.json` with this exact source hash:

```json
{
  "schemaVersion": "1",
  "sourceSha256": "<64 lowercase hex characters from the Job result>",
  "approved": true,
  "reviewer": "<reviewer identity>"
}
```

Then run `forge asset create --resume JOB_ID --review /absolute/review.json
--wait --json`. Godot installation happens only when the original request
contained its target and the review is approved. Inspect `installJobId` and its
native verification artifacts. A successful installed result also returns
`receiptPath` and `receiptSha256`; run `forge receipt verify --path RECEIPT_PATH
--expected-sha256 RECEIPT_SHA256 --json` after Godot imports the asset. If the
Forge Job or Plan store sits inside the Godot project, Forge places `.gdignore`
at the store root so Godot cannot add `.import` files to retained Packs.
If an original `--wait` call is still active, a concurrent `--resume` returns
the current Job snapshot with `wait_for_active_call`; poll the same Job after
that call finishes. A concurrent cancellation of a pending generation is
recorded for the active call to handle; concurrent review waits until the
active call returns.
Rejected artwork remains traceable but is not
installed; a revision uses a new request.

For a local H3 I2V character, copy `forge guide comfyui-video-example` and
replace the reference PNG path, prompt, profile, identity and the existing
`supportAnimations` paths. The reference must be a PNG; Forge hashes and
uploads it to the named workflow input before submitting one durable prompt.
The character Pack contract requires at least two animations, so a single H3
clip needs one existing support animation. The original MP4, including any
audio track, is retained in the Job; only decoded video frames enter the
character Pack. The audio track is not separately installed. The response
reports the video probe, source hash, Pack and GIF preview. Inspect identity,
motion and loop endpoints before binding a review to the source hash. Video
`canvasSize` sets the final square frame size for every animation in the Pack;
it does not correct a ComfyUI workflow that stretches a square reference into
wide video, nor does it ensure the subject has equal scale across animations.
Inspect both generated and support actions before approving. The
technical quality verdict alone cannot approve a character action. To stop a
pending Forge-owned prompt, use `forge asset create --resume JOB_ID --cancel
--json`; Forge refuses to interrupt a running prompt on a shared ComfyUI.

This development branch has been verified with one local Qwen PNG and a local
H3 I2V MP4 on Windows, each installed only after agent QA in an isolated Godot
4.6.3 project. The H3 walk attempt was rejected for insufficient motion; a
subtle idle loop passed. A separate H3 text-to-video profile also generated
an MP4 without `referenceImage`: its API workflow used the optional-first-frame
H3 conditioning node without a first-frame link, and its profile omitted
`referenceInput`. That sample was rejected because the generated robot differed
from the support action, so no Godot install was attempted. This is not yet a
released CLI capability.

For an image edit, export that **edit workflow** separately as API JSON. Map
`referenceInput` to the node/input that receives the reference filename, then
use `forge guide comfyui-edit-example` as a request starting point. Supply an
absolute `referenceImage` PNG. The locally tested Qwen edit produced a usable
red potion but an opaque purple background; without matting, Forge rejected
static preparation. Explicit `staticMatting: "auto_corners"` makes a separate
derived PNG while retaining the original model output and both SHA-256 hashes.
Inspect the derived preview and alpha before approving; this mode is suitable
only for a flat corner-colored background. A generated image can also be
reprocessed with a new source-import request without another model run.

`forge asset batch --input /absolute/batch.json --wait --json` preflights up to
32 request files against explicit `maxRequests`, `maxTotalWaitSeconds` and
`maxTotalOutputBytes` before starting Jobs. Paths in `requests` may be relative
to the manifest. Each request remains an independent Job, and a partial failure
reports completed Job IDs; inspect those Jobs before retrying. The manifest
schema is `schemas/local-generation-batch.schema.json`. This is a reservation
against configured maxima, not a GPU power meter or actual runtime estimate.

Profiles are immutable under their ID. `forge provider export --provider
comfyui --profile ID --output /absolute/descriptor.json --json` writes a
portable descriptor with the endpoint, model ID, node mapping and workflow
SHA-256, without workflow bytes, model weights or media. `forge provider import
--provider comfyui --profile NEW_ID --descriptor /absolute/descriptor.json
--workflow /absolute/api-workflow.json --json` requires an exact hash match.
`forge provider upgrade-check --provider comfyui --profile ID --config
/absolute/candidate-profile.json --json` compares a candidate without mutation;
use a new ID to adopt a changed workflow. Check endpoint and model ID before
sharing even this descriptor, as they may reveal machine configuration.

Install the bundled skill in a game project with `forge skill install --project
/absolute/game --json` for Codex and a DeepSeek Harness configuration that
discovers `.agents/skills`; use `--agent claude-code` for Claude Code's
`.claude/skills` path. Run `forge skill check` with the matching agent flag.
Skill installation confirms files, while natural-language discovery and CLI
selection must still be tested inside each agent client. H3 reference-to-video
is not claimed: the tested machine lacks that profile's required weights.

## 中文说明

此开发分支支持导入和诊断**明确指定的 API 格式工作流**，生成单张透明 PNG、
制作静态 Pack，或将 H3 图生视频 MP4 提帧制作角色 Pack，并在明确审核后恢复执行
Godot 安装。另可用单参考图的 Qwen 编辑 profile；需要在 API 工作流里明确映射
`referenceInput`，请求填写绝对路径 `referenceImage`。本机编辑输出是不透明底色，
直接制作被拒；显式选择 `staticMatting: "auto_corners"` 后，Forge 保留原图与
哈希，并另存去底图供制作。必须检查去底预览，不能把该模式当成通用抠图。
请在 ComfyUI 中使用 **Save (API Format)**
导出；带有 `nodes` 数组和 `links` 的普通界面 JSON 不能直接提交到 `/prompt`。
从导出的 JSON 中确认正向提示词所在的节点 ID、输入键及唯一输出节点，填入
上面的 profile；示例 ID 来自一次 Qwen 导出，其他版本可能不同。运行
`configure` 和 `doctor` 检查服务及节点，再用同一份版本化请求执行
`forge asset create --input ... --wait --json`。返回的 `previewPath` 与源哈希
需要人工或 Agent 明确检查；审核 JSON 必须绑定该哈希。若首次请求已写入
Godot 项目和安装目标，通过 `--resume JOB_ID --review ... --wait` 继续原有
安装事务，并检查安装 Job 的原生验证。拒绝的图不得安装，修改使用新请求。
安装成功的返回值还包含 `receiptPath` 与 `receiptSha256`；Godot 导入后可用
`forge receipt verify --path 回执路径 --expected-sha256 回执哈希 --json`
复核。若 Job 或 Plan 存储目录位于 Godot 项目内，Forge 会在目录根部建立
`.gdignore`，防止 Godot 在保留的 Pack 中写入 `.import` 文件。
如果原来的 `--wait` 调用还在运行，并发 `--resume` 只返回当前 Job 快照及
`wait_for_active_call`，待原调用结束后再查询同一 Job。并发取消待生成任务会
交由正在运行的调用处理；并发审核须等该调用结束。

视频请求可由 `forge guide comfyui-video-example` 取得：显式指定 H3 profile、
首帧 PNG、提示词，以及至少一个已有的 `supportAnimations`，因为角色 Pack
要求两个动画。profile 的 `referenceInput` 指向 LoadImage 的文件名输入；
`outputField` 必须依据 `/history`，本机 H3 的 `SaveVideo` 实际把 MP4
放在 `images`。Forge 保留带音轨的原 MP4，并只把视频帧交付角色 Pack；
不会暗中把音轨安装为音频资源。审核预览时须观看身份稳定性、动作和循环接缝。
视频请求的 `canvasSize` 会把 Pack 中所有动画帧缩放到同一个方形画布；
它无法修复 ComfyUI 工作流把方形首帧拉伸成宽屏视频的问题，也不能保证各动作
主体的视觉大小相同，审核时要比较生成动作与辅助动作。
待运行的 Forge prompt 可通过 `--resume JOB_ID --cancel` 精确删除；运行中的
任务不会调用可能打断其他人的全局 interrupt。真实测试里一段行走候选因动作
不足被拒绝，轻微待机循环经 Agent QA 后才安装到隔离 Godot 项目。
另一个 H3 文生视频 profile 在 API 工作流里不连接首帧，且不声明
`referenceInput`；请求不填写 `referenceImage`。本机已生成 MP4 并制作 Pack，
但生成角色与辅助动作外观不一致，审核结果为拒绝，未安装到 Godot。

批量请求使用 `forge asset batch --input /absolute/batch.json --wait --json`，
清单显式限制请求数、等待秒数与最大输出字节数；超预算时不会启动 Job。
部分失败须按返回的 Job ID 逐个检查。profile 可用 `provider export` 导出
配置指纹，配合单独提供且哈希匹配的 API 工作流执行 `provider import`；
`provider upgrade-check` 只比较差异，不改旧 profile 或 Job。
Codex 与已启用 `.agents/skills` 发现的 DeepSeek Harness 可在项目里安装
`forge-use`；Claude Code 使用 `forge skill install --agent claude-code`。
三种客户端都需要实际对话验证能否发现 skill，并选择同一 `forge asset create`。
