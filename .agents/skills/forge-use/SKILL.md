---
name: forge-use
description: Use Forge CLI for repeatable local game asset preparation, verified Godot delivery and recovery, with optional libraries and Provider generation.
---

# Prepare and deliver assets to Godot

Start with the game's existing specs, toolchain lock and receipts. Use Forge when
repeatable processing, native delivery, updates or recovery help the task. A few
ready-to-import PNGs may only need Godot's native import. Do not create a library,
configure a Provider or migrate existing assets just to start a local delivery.

## Work from the user's task

1. Establish the source files, intended game use and target project from available
   context. Ask only for missing decisions that affect the result. Codex or another
   tool creates artwork; Forge's local processing does not generate it with a model.
2. Select the pinned executable and read only the matching workflow below. The agent
   handles requests, Plans, Jobs and records; the user should see the prepared
   result, relevant choices and any actionable failure.
3. Prepare and inspect the result. Preserve the source and reviewed hashes. Honor
   existing visual-review and installation authorization; do not ask again for
   an approval already supplied, or invent approval for an unreviewed result.
4. Install within the authorized scope, verify the final project and retain the
   necessary evidence in files. Report installed paths, review status and recovery
   steps. A failed installation should reuse the retained preparation after the
   cause is fixed, not automatically regenerate source art.

## Select the actual toolchain

Use the game's verified absolute executable path, preserving its public launcher.
If none is pinned, locate and verify an installation before selecting it. Never
silently upgrade a game or replace its pin with a convenient executable on PATH.

```bash
export FORGE_BIN="/absolute/path/to/forge"
"$FORGE_BIN" doctor --json
shasum -a 256 "$FORGE_BIN"
```

Record `doctor.data.cliPath`, the payload SHA-256 and `data.build` (`gitCommit`,
`dirty`, `target`, `profile`, `features`); check `data.capabilities`. On Windows a
public launcher can be a script: hash the payload reported by `cliPath` as well.
A version string alone is insufficient; unknown Git values are `null`. Tool
availability is separate from compiled capabilities. Check command help and plan
support rather than dropping unsupported request fields.

`guide` reads this executable's offline bundle without installing a skill or
calling a Provider. Installed skill links below stay within the bundle. For a
pinned build without `guide`, use its `skill show --json` if available or matching
file documentation; do not upgrade only to obtain newer instructions. Upgrading
Forge never updates game pins or separately installed skills automatically.

## Read only the needed workflow

| Task | Bundled guide | Installed reference |
| --- | --- | --- |
| Prepare local icons, props or backgrounds | `forge guide static` | [Static PNGs](references/local-static.md) |
| Preserve existing animation coordinates and timing | `forge guide animation` | [Animation](references/animation.md) |
| Prepare local WAV audio | `forge guide audio` | [Audio](references/audio.md) |
| Diagnose delivery, verify sources/receipts, recover | `forge guide delivery` | [Delivery](references/delivery.md) |
| Repeated revisions, reviews or cross-machine reuse | `forge guide project-assets` | [Optional library](references/project-assets.md) |
| Configure Godot or verify/export a game | `forge guide godot-workflow` | [Godot workflow](references/godot-workflow.md) |
| Explicitly requested online generation | `forge guide provider` | [Provider workflow](references/provider.md) |
| User-installed local ComfyUI image/video generation | `forge guide comfyui` | [Local ComfyUI](references/comfyui.md) |

Read request examples with `guide static-example`, `guide audio-example`,
`guide provider-example` or `guide comfyui-image-example`. Check exit status
before using redirected output.
Plain output preserves file bytes; `--json` includes the resource index and hashes.
Godot must be available for native delivery; inspect the selected build's supported
versions. Local images/audio need no Provider credentials or Style Lock. Online
Provider generation uses the user's account and can incur charges. Character
animation remains experimental; unrelated still images are not animation frames.

## Three shortest task paths

All three paths share Plan → Job → inspect/validate → review → install → verify
and receipt retention. Read one workflow; create a library only when versioned
reuse or its comparison page helps.

| Input/task | Prepare | Inspect output before installation |
| --- | --- | --- |
| Named animation frames | `guide animation`, then its `plan prepare-character` recipe | Per-action quality/PNG preview, shared anchor, frame order and durations |
| Music or SFX WAV | `audio inspect`, `guide audio`, `plan prepare-audio` | `asset inspect` audioItems, quality warnings, processed WAV listening, declared loop |
| Icons or props PNG | `source inspect`, `guide static`, `plan prepare-static` | Contact sheet/normalized PNGs, canvas, anchor and sampling |

For each: `plan execute --token TOKEN --wait --json`, retain `data.job_id`, then
`job report --id JOB --json`. Locate the Pack by its `gsfpack` artifact; use
`pack validate` and `asset inspect`. Follow the selected workflow for review and
`godot plan-install`; finish with `godot verify-install` and `receipt export` /
`receipt verify`. See [recovery by phase](references/delivery.md#recover-by-phase)
(`guide delivery`) when interrupted. Never infer approval from a succeeded Job.

## Reuse the complete delivery example when appropriate

For reviewed static/animation/audio inputs with an already authorized processing recipe
and installation, read `forge guide local-delivery-example` on a build exposing
it in the guide index. This Python example joins preparation, retained Packs,
installation and final receipt verification. It requires reviewed `sourceLocks`
and an expected binary hash; it does not itself pause for visual or listening approval.
Use the separate prepare/review/install steps from the selected guide when the
processed result still needs review. For WAV preparation choose `--operation prepare-audio` and explicitly set
`sampleRate` and `channels` after inspecting the source; the example refuses to
choose these conversions for you.

With `filesystem_write_probe`, explicitly run `storage check --path DIR --json`
on production/output directories and inspect `supported`. Unsupported storage
requires a supported location; copying an installation back bypasses its guarantees.
Keep existing library bindings when present; add a new library only for a reuse
or version-history need. Never recursively replace a game's `.forge` directory.

## Verify execution and report the result

Use dedicated `FORGE_JOB_STORE` / `FORGE_PLAN_STORE` directories. Plans fingerprint
inputs; tokens expire after 15 minutes and are single-use. Inspect operation bounds
and Provider estimates. With `--json`, check process status before parsing stdout,
then require `ok:true`; parser failures may only write stderr. Execution additionally
requires `data.lifecycle_state == "succeeded"`. Follow detached Jobs with `job get`
and inspect `job report`; an existing Pack does not prove success.

Validate the retained Pack and final installed resources. Preserve old receipts;
new imports create new evidence. Keep stores until required evidence is saved.
Technical results (`game_ready`, `technical_pass`, `prototype_usable`) do not grant
visual, listening, gameplay or license approval. Keep review status explicit and
never weaken a quality gate to make a delivery appear successful.
