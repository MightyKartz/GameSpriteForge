# Godot setup and agent acceptance

Base: `56d24672b7d517f5af058e2640d8eef0ab16e41e`. This is a source addition
after v0.4.0, not a change to the installed/public v0.4.0 binary.

## Behavior

- `setup godot` saves a validated engine selection without environment edits;
  explicit `--download` installs checksum-pinned official Godot 4.6.3 on Windows
  x64/macOS Apple Silicon, and `--templates` explicitly installs matching templates.
- `godot lock/check` separates machine paths from portable project versions;
  configuration drift and mismatched requirements fail. Asset delivery observes
  the project lock and prepared install plans detect lock addition/change.
- `godot verify` snapshots source files, runs real Godot import/main-scene loading,
  optional viewport capture and project-supplied interaction assertions, and retains
  phase logs/reports. Reused evidence directories, script errors, missing completion
  markers, cancellation and timeouts cannot become successful acceptance.
- `godot export --run` uses the project's host-desktop preset, records exported
  artifacts and runs the actual exported application. It does not infer visual,
  gameplay, signing or device acceptance.

## Native Windows evidence

The synthetic fixture has a 160x120 viewport, a green polygon on a dark background,
and a `hit()`/score assertion. No private game media or Provider generation is used.
The public game source remains unchanged and has no generated `.godot` cache.

- First and second native passes: 16 CLI cases each, including a real GPU screenshot
  and exported Windows executable startup. Godot version:
  `4.6.3.stable.official.7d41c59c4`.
- The failure-report regression pass has 17 CLI cases, including a main scene that
  exits with code zero before completion; its runtime phase must still fail.
- Official Windows managed engine download, SHA-512 verification, persistent setup
  and repeat installation passed. Windows PowerShell extraction uses a process-only
  execution-policy override and retains diagnostics on failure.
- Embedded-guide checks: nine cases, 86 CLI calls; the new `godot-workflow` topic
  matches its embedded source and works without installing a personal skill.
- Core Godot unit checks: nine passed. Static-delivery checks additionally cover
  project toolchain lock drift between planning and execution. All five static
  delivery tests passed, including the real Godot installation check.
- Local development builds report `dirty:true`, default features and their source
  commit. They are verification binaries, not clean release artifacts.

Detailed local outputs are under ignored `target/qa/godot-workflow/`. The new
`Godot agent workflow` CI matrix checks real managed downloads/templates and native
exported applications on both Windows and macOS. Its artifacts retain setup JSON,
acceptance reports and phase logs. CI results must be read from the actual PR run;
local Windows evidence alone does not establish macOS success.

## Limits

- Engine recipes are deliberately pinned to standard Godot 4.6.3; other compatible
  4.6.x installations can be selected explicitly. No automatic project upgrades.
- The v1 portable lock checks Forge package version and the full Godot version;
  Forge build provenance is recorded separately, not pinned by this lock.
- Snapshot isolation is not an execution sandbox. Project scripts/plugins are code.
- Headless startup is a smoke test. Screenshot capture and visual approval remain
  separate, and no interactive display/GPU claim is made for headless CI runs.
- No new MCP server, game scaffolding, cross-target export, code-signing workflow,
  or installed user/consumer skill migration is introduced.
