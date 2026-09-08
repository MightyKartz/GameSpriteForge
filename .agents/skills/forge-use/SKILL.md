---
name: forge-use
description: Use Forge CLI to prepare local game art, generate icon or prop sets with a Forge Provider, inspect asset jobs, and deliver Packs to Godot. Includes Codex image generation followed by local PNG processing.
---

# Use Forge for game assets

Start from the game's asset specs, toolchain lock and import receipts. This skill
is self-contained: its required references and examples travel with the bundle.
Installing it does not install or upgrade Forge, enable Codex image tools, or
replace a project's pinned executable. Forge source development is a separate task.

## Select the actual toolchain

The capability baseline is the default Forge v0.3.0 CLI. Use the game's verified
absolute executable path, including its installed launcher when applicable; do
not substitute a convenient `forge` on PATH. Record that path and binary SHA-256:

```bash
export FORGE_BIN="/absolute/path/to/forge"
"$FORGE_BIN" --version
"$FORGE_BIN" doctor --json
"$FORGE_BIN" plan --help
shasum -a 256 "$FORGE_BIN"
```

Check `doctor.data.build` (`gitCommit`, `dirty`, `target`, `profile`, `features`)
and `doctor.data.capabilities`. A development executable can share a release
version while accepting different fields. Unknown Git identity is `null`; it is
not a clean-build claim. Check command help and successfully plan the actual
request instead of dropping unsupported fields. Runtime Godot/FFmpeg availability
is separate from compiled capabilities. Godot 4.6.x is installed separately.

The stable local PNG → static Pack → Godot route requires `local_static_import`,
`pack_validation` and `godot_install`; v0.2.1 predates local static intake. Verify
consumer contracts before updating any existing lock for future imports. Preserve
old receipts and source history; actual re-imports create new receipts.

## Choose the workflow

- **Codex image generation or existing static PNGs:** read
  [local static preparation and Godot delivery](references/local-static.md).
  Use one PNG per icon or prop. Codex's image model is an external source tool,
  not a Forge Provider; local preparation needs no Provider login or Style Lock.
- **Forge Provider generation:** read the [Provider workflow](references/provider.md)
  for Style → icon/prop generation, usage evidence and targeted retry. This route
  can make Provider requests. A local PNG alone does not require it.
- **Actual animation frames:** read [experimental animation](references/animation.md)
  for preserved coordinates, timing and sheet preprocessing. Unrelated still
  items are not animation frames. Character animation remains experimental.

## Execute and assess results

Use dedicated `FORGE_JOB_STORE` and `FORGE_PLAN_STORE` locations and retain them
until the required evidence is saved. A plan fingerprints inputs without changing
a Godot project. Its token expires after 15 minutes, is consumed once, and rejects
changed inputs. Inspect its bounds and Provider estimates before execution.

With `--json`, check process exit status before decoding stdout, then require
`ok:true`. Parser errors can write only stderr. For an execution, also require
`data.lifecycle_state == "succeeded"`; command acceptance or an existing Pack
does not prove Job success. Follow a detached Job with `job get --id JOB --json`.
Read `job report`, inspect artifacts and validate the Pack before delivery.

Keep structural validation separate from visual approval. Local static
`game_ready` does not evaluate style consistency or approve artwork.
`prototype_usable` remains a prototype. Do not disable a quality gate to claim
success or record human acceptance for a review that was not performed. Record an
explicitly requested prototype and its remaining review status in the receipt.
