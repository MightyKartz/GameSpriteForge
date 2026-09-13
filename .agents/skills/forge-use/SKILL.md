---
name: forge-use
description: Use Forge CLI to prepare local game art or WAV audio, generate icon or prop sets with a Forge Provider, inspect asset jobs, and deliver Packs to Godot. Audio requires a verified source build.
---

# Use Forge for game assets

Start from the game's asset specs, toolchain lock and import receipts. This skill
is self-contained: its required references and examples travel with the bundle.
Forge v0.3.2 exposes this same bundle through `guide`, so reading and using it
requires no skill installation or source checkout. An optional skill installation
lets Codex discover it by name. Neither route installs or upgrades Forge, enables
Codex image tools, or replaces a project's pinned executable. Forge source
development is a separate task.

## Select the actual toolchain

The asset-processing baseline is the default Forge v0.3.0 CLI. Use the game's
verified absolute executable path, including its installed launcher when
applicable. If no executable is pinned, locate the local installation and verify
it before selecting it. Do not substitute a convenient `forge` on PATH for an
existing lock. Record the selected path and binary SHA-256:

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

## Read the matching guide

On v0.3.2 or a verified build with `embedded_usage_guide`, `"$FORGE_BIN" guide`
reads this entrypoint and the commands below read its bundled resources offline.
They are read-only and use no Provider, Job store or Codex configuration. Keep
using that same executable for the workflow. A CLI upgrade carries its matching
guide; it does not update any separately installed skill files.

When this skill is installed, the relative links below work within its directory.
When reading it through the CLI, use the corresponding `guide` command instead
of looking for those files in the game project. Older pinned executables such as
v0.3.0 have no `guide` command: use matching file documentation and verify the
older CLI's capabilities without changing its pin just to access documentation.
v0.3.1 can expose the complete bundle through `"$FORGE_BIN" skill show --json`.
Installed skill files can also be read directly; check their instructions against
the pinned executable's capabilities.

## Choose the workflow

- **Existing resources, candidates, reviews and cross-machine reuse:** read
  [project resource library](references/project-assets.md), or run
  `"$FORGE_BIN" guide project-assets` on a build with `project_asset_catalog_v3`.
- **Codex image generation or existing static PNGs:** read
  [local static preparation and Godot delivery](references/local-static.md),
  or run `"$FORGE_BIN" guide static`.
  Use one PNG per icon or prop. Codex's image model is an external source tool,
  not a Forge Provider; local preparation needs no Provider login or Style Lock.
- **Forge Provider generation:** read the [Provider workflow](references/provider.md),
  or run `"$FORGE_BIN" guide provider`. It covers Style → icon/prop generation,
  usage evidence and targeted retry. This route
  can make Provider requests. A local PNG alone does not require it.
- **Actual animation frames:** read [experimental animation](references/animation.md),
  or run `"$FORGE_BIN" guide animation`. It covers preserved coordinates, timing
  and sheet preprocessing. Unrelated still
  items are not animation frames. Character animation remains experimental.
- **Local music, sound effects or ambience:** read [audio preparation](references/audio.md),
  or run `"$FORGE_BIN" guide audio` on a build with `local_audio_import`.
  Import WAV sources from the user's chosen tools. Optional external audio tools
  are separate installations; Forge's diagnostics never run or install them.
- **Source inspection, reviewed hashes, durable receipts and installed audits:**
  read [delivery evidence](references/delivery.md), or run
  `"$FORGE_BIN" guide delivery`. Check each new capability against the selected binary.

The references link to local request examples. Without an installed bundle, read
them with `"$FORGE_BIN" guide static-example`, `"$FORGE_BIN" guide provider-example`,
or `"$FORGE_BIN" guide audio-example` when the selected build supports audio.
Plain output is the resource's exact text; `--json` adds its path, SHA-256, bundle
and CLI identity, and the resource list. Check command exit status before using
the output, especially when redirecting an example into a new request file.

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

Keep structural validation separate from visual or listening approval. Local static
`game_ready` does not evaluate style consistency or approve artwork.
Audio `technical_pass` does not establish sound quality or a seamless loop.
`prototype_usable` remains a prototype. Do not disable a quality gate to claim
success or record human acceptance for a review that was not performed. Record an
explicitly requested prototype and its remaining review status in the receipt.
