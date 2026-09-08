# Use Forge with Codex

Forge v0.3.2 exposes its complete usage guide and request examples through the
CLI. Ask Codex to read that guide and use Forge directly: no skill installation,
source checkout or separate download is needed. The same content can optionally
be installed as `forge-use` for Codex skill discovery.

## Use the embedded guide

Open your game project in Codex and ask, for example:

> Use the locally installed Forge CLI to prepare cultivation-themed inventory
> icons for this Godot game. Keep the game's pinned Forge executable if it has
> one, verify its version and capabilities, and read its embedded guide when
> supported. Generate the source artwork with the available image tool, retain
> the original PNGs, then prepare, review and install the assets with Forge.

Use the game's verified absolute CLI path throughout the workflow, including the
installed launcher when applicable. If the game has no pin, locate and verify the
local installation before choosing it:

```bash
export FORGE_BIN="/absolute/path/to/forge"
"$FORGE_BIN" --version
"$FORGE_BIN" doctor --json
"$FORGE_BIN" --help
shasum -a 256 "$FORGE_BIN"
```

v0.3.2 advertises `embedded_usage_guide` in `doctor.data.capabilities`. On that
verified executable, read only the resources relevant to your task:

```bash
"$FORGE_BIN" guide
"$FORGE_BIN" guide static
"$FORGE_BIN" guide static-example
```

`guide` defaults to `overview`. Plain output is the original bundled file, with
no added heading or wrapper. Each resource also accepts its exact bundle path:

| Resource | Bundle path | Use |
| --- | --- | --- |
| `overview` | `SKILL.md` | Toolchain selection and workflow routing |
| `static` | `references/local-static.md` | Local PNG preparation, Pack review and Godot delivery |
| `provider` | `references/provider.md` | Optional Forge Provider generation and usage evidence |
| `animation` | `references/animation.md` | Experimental local animation processing |
| `static-example` | `examples/local-static.json` | Local icon/prop request to adapt |
| `provider-example` | `examples/provider-icons.json` | Provider icon request to adapt |

For a new request file, `"$FORGE_BIN" guide static-example > request.json`
exports the example. Check the command succeeded before editing or consuming
that file; failed commands can leave an empty redirected file. Set source paths,
IDs and rights information for the actual artwork before planning.

`guide --json` uses the standard success envelope. Its `data` contains `name`,
`schemaVersion`, `cliVersion`, `build` and `contentHash` for bundle and executable
identity, plus the selected resource's `path`, `sha256` and `content`.
`resources` lists each `topic`, `path` and `mediaType`. Check process exit status
before parsing the envelope, then require `ok: true`.

Guide reads are offline and read-only: they do not access Provider credentials,
create Plans or Jobs, install files, or alter Codex configuration. Reading a
different CLI version returns that executable's bundled documentation. Keep the
game's toolchain lock intact. v0.3.1 has no `guide`, but can expose its complete
bundle with `"$FORGE_BIN" skill show --json`. v0.3.0 needs matching file
documentation. Do not change executables just to read the documentation.

## Optional: install a discoverable skill

Codex discovers skills from its supported directories or plugins; having a CLI
on PATH does not register an embedded skill. For discovery by name, install the
bundle from the selected executable into your existing game project:

```bash
"$FORGE_BIN" skill install --project .
"$FORGE_BIN" skill check --project . --json
```

This creates `.agents/skills/forge-use/` in the selected project. The project
directory must already exist; it does not need to contain a Godot project yet.
Open the project in Codex and ask it to use `forge-use`. Codex can select the
skill from its description. You can also mention
`$forge-use` explicitly in Codex CLI. If it does not appear, restart Codex.
These discovery locations and invocation options follow the
[Codex skill documentation](https://learn.chatgpt.com/docs/build-skills#where-to-save-skills).

For use across projects, choose personal installation instead:

```bash
"$FORGE_BIN" skill install --user
"$FORGE_BIN" skill check --user --json
```

Personal installation uses `~/.agents/skills/forge-use/`. Choose one scope to avoid
duplicate skills with the same name; Codex does not merge them. Both `install` and
`check` require exactly one of `--project PATH` or `--user`.

Skill installation is available from v0.3.1 and works offline. You still need
Codex with an available image-generation tool to create new artwork, and Godot 4.6.x for engine
delivery. Existing transparent PNGs can go directly to Forge without a Provider
account. Installing the skill does not install those tools, alter Codex settings,
change `AGENTS.md`, or upgrade a game's pinned Forge executable.

## Inspect and update an installed skill

```bash
"$FORGE_BIN" skill show
"$FORGE_BIN" skill show --json
"$FORGE_BIN" skill check --project . --json
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

The embedded guide follows the CLI version automatically. For a separately
installed skill, rerun `install` explicitly after upgrading the CLI. Repeating
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
