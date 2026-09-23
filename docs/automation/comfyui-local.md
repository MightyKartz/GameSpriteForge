# Local ComfyUI image and video delivery (development)

This development checkout can import and diagnose a **single, explicit
API-format workflow**, generate a transparent PNG or H3-style I2V MP4,
prepare a static or character Pack, and resume after visual review for Godot
installation. Image editing remains pending. The staged contract is in
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
native verification artifacts. Rejected artwork remains traceable but is not
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
motion and loop endpoints before binding a review to the source hash. The
technical quality verdict alone cannot approve a character action. To stop a
pending Forge-owned prompt, use `forge asset create --resume JOB_ID --cancel
--json`; Forge refuses to interrupt a running prompt on a shared ComfyUI.

This development branch has been verified with one local Qwen PNG and a local
H3 I2V MP4 on Windows, each installed only after agent QA in an isolated Godot
4.6.3 project. The H3 walk attempt was rejected for insufficient motion; a
subtle idle loop passed. This is not yet a released CLI capability.

## 中文说明

此开发分支支持导入和诊断**明确指定的 API 格式工作流**，生成单张透明 PNG、
制作静态 Pack，或将 H3 图生视频 MP4 提帧制作角色 Pack，并在明确审核后恢复执行
Godot 安装。图片编辑尚未接入。
请在 ComfyUI 中使用 **Save (API Format)**
导出；带有 `nodes` 数组和 `links` 的普通界面 JSON 不能直接提交到 `/prompt`。
从导出的 JSON 中确认正向提示词所在的节点 ID、输入键及唯一输出节点，填入
上面的 profile；示例 ID 来自一次 Qwen 导出，其他版本可能不同。运行
`configure` 和 `doctor` 检查服务及节点，再用同一份版本化请求执行
`forge asset create --input ... --wait --json`。返回的 `previewPath` 与源哈希
需要人工或 Agent 明确检查；审核 JSON 必须绑定该哈希。若首次请求已写入
Godot 项目和安装目标，通过 `--resume JOB_ID --review ... --wait` 继续原有
安装事务，并检查安装 Job 的原生验证。拒绝的图不得安装，修改使用新请求。

视频请求可由 `forge guide comfyui-video-example` 取得：显式指定 H3 profile、
首帧 PNG、提示词，以及至少一个已有的 `supportAnimations`，因为角色 Pack
要求两个动画。profile 的 `referenceInput` 指向 LoadImage 的文件名输入；
`outputField` 必须依据 `/history`，本机 H3 的 `SaveVideo` 实际把 MP4
放在 `images`。Forge 保留带音轨的原 MP4，并只把视频帧交付角色 Pack；
不会暗中把音轨安装为音频资源。审核预览时须观看身份稳定性、动作和循环接缝。
待运行的 Forge prompt 可通过 `--resume JOB_ID --cancel` 精确删除；运行中的
任务不会调用可能打断其他人的全局 interrupt。真实测试里一段行走候选因动作
不足被拒绝，轻微待机循环经 Agent QA 后才安装到隔离 Godot 项目。
