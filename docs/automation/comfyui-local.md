# ComfyUI local workflow profiles (foundation)

This checkout can import and diagnose a **single, explicit API-format workflow**.
It does not yet provide `forge asset create`, media processing or Godot installation
from ComfyUI. The planned end-to-end contract is in
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
  "promptInput": {"node": "6", "input": "text"},
  "outputNode": "9",
  "outputField": "images",
  "maxOutputBytes": 67108864,
  "timeoutSeconds": 300
}
```

The identifiers `6` and `9` are placeholders, not a claim about Qwen's bundled
workflow. Video profiles use `mediaKind: "video"` and the actual result field
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

## 中文说明

当前仅支持导入和诊断**明确指定的 API 格式工作流**，尚未提供 ComfyUI 到
Godot 的 `forge asset create` 完整交付。请在 ComfyUI 中使用 **Save (API Format)**
导出；带有 `nodes` 数组和 `links` 的普通界面 JSON 不能直接提交到 `/prompt`。
从导出的 JSON 中确认正向提示词所在的节点 ID、输入键及唯一输出节点，填入
上面的 profile；示例中的 `6` 和 `9` 只是占位值。图片和视频模式分别配置，
视频输出字段以实际工作流为准。运行 `configure` 后用 `doctor` 检查服务和
节点类型。诊断通过不代表模型权重存在、生成成功或素材已通过游戏审核。
