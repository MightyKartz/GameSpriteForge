# Forge Provider generation

Use this route when the requested work calls for generation by a Forge Provider.
For Codex-generated or existing PNGs, use [local preparation](local-static.md)
(`"$FORGE_BIN" guide static`) without Provider login or a Style Lock. Apply the
shared [toolchain and Job checks](../SKILL.md) (`"$FORGE_BIN" guide overview`).
The `guide` commands require a verified CLI with `embedded_usage_guide`; installed
skill links remain usable with an older pinned executable.

## Select the project and Provider

Inspect an existing asset project with `project inspect --project PATH --json`.
It pins one Provider/Profile and an immutable Style revision. Preserve the user's
selection; missing capabilities are a gap to report, not permission to switch
Providers. The release supports xAI; `fixture` is an offline test Provider and
does not establish real-model art quality.

```bash
"$FORGE_BIN" provider list --json
"$FORGE_BIN" project init --path /absolute/assets --name "My Game" --provider xai --json
```

Initialize only a new asset project. For offline contract checks, choose
`--provider fixture` in a separate temporary project. `doctor` and `provider list`
avoid credential reads; `provider doctor --provider xai --json` explicitly checks
authentication. If login is required for the authorized Provider route, use:

```bash
"$FORGE_BIN" provider login --provider xai --method api-key
```

The CLI reads the secret through a hidden TTY and stores it in the OS credential
store. Do not put credentials in arguments, specs, receipts or diagnostic output.
Device Code OAuth is a Preview alternative selected with `--method oauth`.

## Lock a Style and generate a set

Save this v1 Style spec in the working asset-spec directory and adapt its visual
intent and canvas sizes. `referenceImages` accepts zero to three image paths,
resolved relative to the spec; keep their originals and hashes.

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

`style create` and `generate` prepare and immediately consume their single-use
plans. They can make Provider requests; run them only within the user's authorized
generation scope. Do not use these commands as a no-cost validation probe. In the
v0.3.0 baseline, `--plan-only` is not a reliable dry run for Style/static generation.

Copy [the Provider icon spec](../examples/provider-icons.json) from an installed
skill, or retrieve it from the verified CLI into a new working request:

```bash
"$FORGE_BIN" guide provider-example > /absolute/asset-specs/provider-icons.json
```

Confirm the command succeeded before editing or consuming that file; a failed
command can leave an empty redirected file. Adapt it before generating. Its
`prompt` fields ask the Provider
to create items; they are not local `path` inputs. Use actual rights information
for `license`. For a prop set, change `kind` to `prop_set`, choose appropriate items,
and call `generate prop-set`. The Style Lock records references, hashes,
Provider/model identity, palette and sampling evidence; generation locks that
Style revision rather than mutating it.

```bash
"$FORGE_BIN" style create --project /absolute/assets --spec /absolute/asset-specs/style.json --wait --json
"$FORGE_BIN" style inspect --project /absolute/assets --json
"$FORGE_BIN" generate icon-set --project /absolute/assets --spec /absolute/asset-specs/provider-icons.json --wait --json
```

Require each Job to succeed, inspect `job report --id JOB --json`, and review its
media, consistency evidence and `providerRequestOccurred`/`providerRequestCount`.
Use estimates and recorded attempts instead of inferring cost from a command name.
Without `--wait`, follow the returned Job until it reaches a terminal lifecycle.

## Retry, review and deliver

Provider-generated static Jobs support `job retry --id JOB --item ITEM --wait --json`.
A retry creates a new source-linked Job, calls the Provider for the requested item
and reuses siblings only after checking their source/hash evidence. Inspect the
new result before another attempt; do not repeat paid requests indefinitely.
After creating an appropriate new Style revision, a static
`job retry --id JOB --stage consistency --wait --json` rechecks normalized images
locally with zero Provider requests. Confirm that in the new report. This retry
contract does not apply to local `prepare-static` Jobs.

`job review --id JOB --accept --reason "ACTUAL_REVIEW_REASON" --json` can promote
only an `awaiting_review` gray-band result after the required visual review was
actually performed. It cannot override corrupt media, invalid alpha/canvas/frame
data or other hard gates. Do not use acceptance as an automatic success step.

Find the resulting Pack in Job artifacts, validate it, then follow
[Godot installation and receipts](local-static.md#install-into-godot)
(`"$FORGE_BIN" guide static`). Keep stable
asset keys/targets and preserve Provider/Style provenance in the receipt.
Character generation and its action-specific retry stages remain experimental;
read [animation guidance](animation.md) (`"$FORGE_BIN" guide animation`) before
selecting that route. Optional
Subject V2, world assets and game-art manifests are source-build capabilities;
they are outside this default-CLI workflow.
