# Godot setup and project acceptance

These commands require `godot_environment_setup`, `godot_project_toolchain_lock`,
`godot_project_acceptance` and `godot_desktop_export_verification` in `forge doctor`.
They are included from v0.5.0; the v0.4.0 release does not have them.
Godot 4.7 support and `setup godot --version` require `godot_version_selection`.
That capability is included from v0.6.0; the published v0.5.0 binary accepts only
Godot 4.6.x and downloads only 4.6.3. Always read the selected executable's own
guide before using these options.

## Set up once per machine

```sh
forge setup godot --path /absolute/path/to/Godot --json
forge doctor --json
```

Windows accepts the official versioned `_console.exe` filename; macOS accepts
`Godot.app` or its executable. No environment variable or shell restart is needed.
Omit `--path` to discover an existing compatible engine in PATH, standard application
directories or the top-level Downloads directory. Discovery does not scan whole disks.
An explicit invalid selection fails instead of silently selecting another engine.

Alternatively, explicitly download the pinned official standard engine:

```sh
forge setup godot --download --json
forge setup godot --download --version 4.7.2 --templates --json
forge setup godot --download --version 4.6.3 --json
```

Managed downloads currently support Windows x64 and macOS Apple Silicon and install
Godot **4.7.2** by default on builds with `godot_version_selection`. Use
`--version 4.6.3` to retain the previous supported download; only these two
checksum-pinned versions are accepted. `--version` requires `--download` and
cannot be used with `--path`. Each managed engine has a separate versioned
directory, so downloading one does not replace the other's files. Setup selects
the machine's engine but never changes a game's lock. Downloads require curl
(Windows uses curl.exe) and native ZIP extraction. SHA-512 is checked before extraction or execution.
The optional templates archive is large and is downloaded only with `--templates`.
It goes in Godot's standard user export-template directory for the selected version.
With `--path PATH --templates`, Forge selects the matching pinned template archive for
a standard 4.6.3 or 4.7.2 engine. Other builds/editions need their own templates.
Existing unrecognized installations are preserved. macOS signing/notarization of Godot remains upstream's.

Forge saves engine location, version and executable hashes in its local `godot.json`.
Use `FORGE_CONFIG_DIR` to isolate this configuration and managed engine directory.
Selection order: command-specific `--godot`/setup `--path`, `FORGE_GODOT_PATH`, saved
configuration, compatible discovered engine. A broken or modified saved engine is
an error; rerun setup to select a replacement explicitly. An old environment override
still takes priority over saved configuration and should be removed when no longer wanted.
The Windows console launcher and its sibling engine are both fingerprinted.

## Lock project requirements

```sh
forge godot lock --project /absolute/game --json
forge godot check --project /absolute/game --json
```

Commit `.forge/toolchain.lock.json` with the game. It pins Forge's package version
and Godot's full version string, including the engine build identifier and edition;
it contains no machine paths. Different platforms resolve their own executables.
This v1 lock does not pin the Forge Git commit or enable optional Cargo features;
acceptance reports separately record actual Forge build identity. Use clean releases
for team pins. Existing consumer locks and receipts are not migrated automatically.
Changing an existing lock requires `--update` after verification with the new tools.
A 4.6.3 project remains locked to its complete engine version when 4.7.2 is selected;
`check`, installation, verification and export fail on the mismatch without rewriting
the lock. Select the original engine explicitly until you choose to migrate.
Asset installation also checks this lock, and reviewed install plans fingerprint it.
Projects without the new lock retain their existing behavior.

## Verify without writing to the source project

Create the output's parent first; the output directory must not already exist and
must be outside the game. Snapshotting excludes `.git` and `.godot`, rejects symlinks
and checks source/file hashes before and after copying. This is working-copy
isolation, **not a sandbox**: only run trusted game code and editor plugins.
Each command also creates `user-profile/` inside its evidence directory and redirects
the Godot child processes' profile/data/cache/temp environment there. Import, runtime,
interaction tests and exported startup share that run's fresh profile; another run
gets a new one. This isolates normal `user://` saves, including custom user-directory
settings. `report.json.userData.root` records the profile root. Original saves are not
loaded or updated. Absolute file access and external services are not sandboxed.
Project settings and exported game data retain their original user-directory settings;
the acceptance profile applies only to processes launched by Forge.

```sh
forge godot verify --project /absolute/game --output /absolute/qa/run-1 --json
forge godot verify --project /absolute/game --output /absolute/qa/view-1 --screenshot --frames 60 --json
```

The first runs a headless import and loads the main scene for a bounded number of
frames. The second uses actual rendering and captures `screenshot.png`; a GPU/display
is required. The project must have a configured main scene. A test that exits early
without the completion marker fails. `--timeout` bounds each engine phase (default
120 seconds). `--cancel-file PATH` aborts if that file appears; timeout/cancellation
terminate the process tree/group. Child output is retained in phase-specific logs,
with a 16 MiB per-stream limit while running. No Provider requests are made.

For game-specific interaction assertions, supply `--acceptance-script tests/game.gd`.
It must be a project-relative `SceneTree` script, exit successfully and print exactly
`FORGE_ACCEPTANCE_OK` on its own line after its assertions. It runs headlessly in the
same snapshot. Missing interaction tests are reported as `not_run`, never a pass.

Check both exit status and the JSON envelope. Read `report.json` and phase logs on
failure; logs are also available while running. The report separates import, runtime,
interaction checks and screenshot capture. `visualReview` remains `not_assessed`;
agents/users must inspect screenshots, play and listen before recording artistic or
gameplay acceptance. A smoke test is not proof of complete game correctness.

## Export and test the result

```sh
forge godot export --project /absolute/game --preset "Desktop" --output /absolute/qa/build-1 --run --json
```

The preset must already exist in `export_presets.cfg` and target the host desktop
platform. Forge resolves the release template before isolating the child environment
and binds its absolute path only in the copied preset. Relative paths (including `..`
and `res://`) are resolved against the original project. Standard templates are selected
from the engine's versioned user or self-contained template directory. Missing templates
fail with a report; engine failures also retain logs. `check` reports the managed template
location separately from actual export success. `releaseTemplate` records the actual
path, source (`standard` or `custom`) and hash, rechecked after export. The existing
`customTemplate` field remains populated only for custom presets. `--timeout` defaults
to 300 seconds per phase.

Export runs in a fresh snapshot. `--run` launches the exported native program for a
bounded 30-frame headless startup check; without it, exported runtime is `not_run`.
The program/archive hash is recorded. On macOS the ZIP is extracted before running
the app. This checks native startup, not exported graphics, distribution signing,
notarization, notarization credentials, or cross-platform/device behavior. Configure
those in the game's own delivery workflow. Use Godot directly for cross-target export.

Codex, Claude and other terminal-capable agents can use the same CLI and matching
embedded guide. Keep engine execution in the official Godot CLI; no agent-specific
MCP server or duplicated asset implementation is required for this workflow.
