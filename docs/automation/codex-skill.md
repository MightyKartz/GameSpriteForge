# Use Forge with Codex

Forge v0.3.1 includes the `forge-use` skill inside the CLI. Install it into your
game project to give Codex the Forge workflow and request examples without a
source checkout or a separate download. Skill installation works offline.

## Set up a game project

Install Forge, then run these commands from your existing game project:

```bash
forge skill install --project .
forge skill check --project . --json
```

This creates `.agents/skills/forge-use/` in the selected project. The project
directory must already exist; it does not need to contain a Godot project yet.
Open the project in Codex and ask, for example:

> Use forge-use to prepare a set of cultivation-themed inventory icons for this
> Godot game. Generate the source artwork with the available image tool, retain
> the original PNGs, then prepare and install the assets with Forge.

Codex can select the skill from its description. You can also mention
`$forge-use` explicitly in Codex CLI. If it does not appear, restart Codex.
These discovery locations and invocation options follow the
[Codex skill documentation](https://learn.chatgpt.com/docs/build-skills#where-to-save-skills).

For use across projects, choose personal installation instead:

```bash
forge skill install --user
forge skill check --user --json
```

Personal installation uses `~/.agents/skills/forge-use/`. Choose one scope to avoid
duplicate skills with the same name; Codex does not merge them. Both `install` and
`check` require exactly one of `--project PATH` or `--user`.

The skill includes instructions and examples. You still need Codex with an
available image-generation tool to create new artwork, and Godot 4.6.x for engine
delivery. Existing transparent PNGs can go directly to Forge without a Provider
account. Installing the skill does not install those tools, alter Codex settings,
change `AGENTS.md`, or upgrade a game's pinned Forge executable.

## Inspect and update

```bash
forge skill show
forge skill show --json
forge skill check --project . --json
```

`show` displays the bundled entrypoint. Its JSON form also contains each bundled
file and its SHA-256, the overall `contentHash`, CLI version and compiled build
identity. These are the contents of the executable you invoked.

`check` is read-only. A successful inspection returns `ok: true` and one of these
`data.status` values; scripts must inspect the status, not just the exit code:

| Status | Meaning | Next step |
| --- | --- | --- |
| `missing` | No skill is installed at this target. | Run `install` for this scope. |
| `current` | Installed content matches the bundled content. | Use the skill. |
| `outdated` | An unmodified Forge-managed skill has different content. | Run `install` to update from this executable. |
| `modified` | Managed content was changed, removed or augmented. | Preserve and review the changes before replacing it. |
| `unmanaged` | The existing target is not a recognized Forge installation. | Preserve it and choose how to migrate it manually. |

`outdated` describes a content mismatch, not a semantic version ordering. Invoking
an older Forge binary can install that binary's skill. Follow the game's toolchain
lock when selecting an executable.

After upgrading the CLI, rerun `install` explicitly to update its skill. Repeating
an identical install leaves it unchanged. Updating unmodified managed content
preserves the previous directory and returns its location in `backupPath`.
Backups remain outside Codex's `.agents/skills` discovery tree.

Forge records managed files in `.forge-skill-manifest.json`. Keep this file with
the skill. Modified or unrecognized content is not overwritten; there is no
`--force` option. Installation also refuses symlinks within its managed paths.
Preserve a custom skill elsewhere before manually resolving a name conflict.

For existing source-linked installations from v0.3.0 or earlier, preserve the
symlink and its source until you choose to remove the link and install the bundle.
The CLI does not replace that link automatically.

## What the skill covers

- Codex or existing PNG artwork → local icon/prop preparation → validated Pack →
  Godot delivery, including source preservation, sampling and stable asset paths.
- Optional Provider generation using your own account, with usage checked before
  execution.
- Experimental local animation preparation, preserving shared coordinates and
  timing where required. Visual review remains separate from structural checks.

The maintained source is [forge-use](../../.agents/skills/forge-use/SKILL.md).
Read the [local PNG guide](codex-local-assets.md) for the asset workflow and the
[CLI protocol](forge-cli.md) for automation details.
