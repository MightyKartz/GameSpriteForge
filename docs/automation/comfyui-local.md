# Local ComfyUI image generation and delivery (development)

This development checkout can import and diagnose a **single, explicit
API-format workflow**, generate one transparent PNG, prepare a static Pack,
and resume after visual review for Godot installation. Video and image editing
are still pending. The staged contract is in
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
`mediaKind: "video"` and the actual result field
(`gifs` or `videos`) returned by their output node. Save a separate profile for
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
installed; a revision uses a new request. This feature has been verified with
one local Qwen image on Windows and an isolated Godot 4.6.3 project; it is not
yet a released CLI capability.

## 中文说明

此开发分支支持导入和诊断**明确指定的 API 格式工作流**，生成单张透明 PNG、
制作静态 Pack，并在明确审核后恢复执行 Godot 安装。视频与图片编辑尚未接入。
请在 ComfyUI 中使用 **Save (API Format)**
导出；带有 `nodes` 数组和 `links` 的普通界面 JSON 不能直接提交到 `/prompt`。
从导出的 JSON 中确认正向提示词所在的节点 ID、输入键及唯一输出节点，填入
上面的 profile；示例 ID 来自一次 Qwen 导出，其他版本可能不同。运行
`configure` 和 `doctor` 检查服务及节点，再用同一份版本化请求执行
`forge asset create --input ... --wait --json`。返回的 `previewPath` 与源哈希
需要人工或 Agent 明确检查；审核 JSON 必须绑定该哈希。若首次请求已写入
Godot 项目和安装目标，通过 `--resume JOB_ID --review ... --wait` 继续原有
安装事务，并检查安装 Job 的原生验证。拒绝的图不得安装，修改使用新请求。
