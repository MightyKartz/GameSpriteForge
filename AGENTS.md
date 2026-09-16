# Working on Forge

## Product and scope

Forge is a Rust CLI for AI-assisted game asset production and management.
Codex or another tool can generate source artwork; Forge prepares, validates,
catalogs and delivers it to Godot. Optional Providers can generate assets through
explicit online requests. Do not describe local processing as model generation.
See [PRODUCT.md](PRODUCT.md) for scope and [README.md](README.md) for current
release availability. The retired desktop application lives in Git history.

## Choose the relevant instructions

- CLI implementation, audits, packaging and developer docs:
  [.agents/skills/forge-dev/SKILL.md](.agents/skills/forge-dev/SKILL.md).
- Preparing assets for a game without changing Forge:
  [.agents/skills/forge-use/SKILL.md](.agents/skills/forge-use/SKILL.md).
  With an installed or pinned executable, read that executable's `forge guide`;
  the checkout's guide may describe newer capabilities.
- Video and promotional work: read the project's own `AGENTS.md` when present
  and use the relevant media skill. Its toolchain is separate from the Rust CLI.

Load only the references needed for the task. Keep shared repository rules here,
developer procedures in `forge-dev`, and executable user guidance in `forge-use`.
The latter is embedded in the binary; developer guidance is not.

## Repository map

| Path | Responsibility |
| --- | --- |
| `packages/cli` | Commands, JSON protocol, build identity, embedded guide |
| `packages/core` | Plans, Jobs, processing, catalog and Godot delivery |
| `packages/pack` | Pack formats and validation |
| `packages/providers` | Provider integrations |
| `schemas`, `profiles`, `examples` | Contracts, processing profiles, request examples |
| `scripts`, `.github/workflows` | Integration checks, packaging and CI |
| `docs/automation`, `docs/architecture` | Workflow contracts and design decisions |
| `docs/releases`, `docs/qa` | Release notes and reviewed verification evidence |

## Establish the checkout and executable

Inspect the current branch, worktree status and relevant history before editing.
Preserve existing tracked and untracked work, including media projects. Use a
`codex/` branch for a new change unless the task specifies another branch.

Distinguish the source checkout, locally installed CLI and a game's pinned CLI.
For source verification, build the selected checkout and use an absolute binary
path. Account for `CARGO_TARGET_DIR`; a pre-existing `target/debug/forge` may be stale.
Record commit, dirty state, features, `doctor --json` identity and binary SHA-256
when reporting runtime results. A version string alone is insufficient.

Default CLI features are empty. Verify optional character/world workflows only
when relevant; do not present source-only features as default release behavior.
Preserve consumer toolchain locks and existing asset receipts unless updating them
is part of the task. Importing with a new build creates new evidence.

## Select verification by impact

Use the [developer verification guide](.agents/skills/forge-dev/references/verification.md)
for concrete commands and native-runtime requirements.

- Documentation-only changes: check links, examples and the diff. Run the skill
  validator when changing a skill. Rust builds are unnecessary unless embedded
  `forge-use` content or another compiled input changes.
- Rust changes: run formatting and focused tests for the changed contracts.
  Include Pack tests when changing Pack formats or processing output.
- Catalog changes: cover content identity, revisions, reviews and transfer;
  portability claims require exchanging fixtures between macOS and Windows.
- Godot delivery changes: check the saved resources in real Godot as well as
  transactions and CLI behavior. Serialize operations sharing a Godot cache.
- Packaging changes: exercise installed public launchers and bundled helpers
  on the relevant native platform; source compilation is not release validation.

Use isolated Job/Plan stores and synthetic fixtures for tests. Keep structural
validation, native loading, visual/listening review and device testing distinct.
A `game_ready` result does not establish artistic quality or license approval.

## macOS, Windows and GitHub coordination

Fetch before comparing with GitHub or preparing a merge. Inspect divergence and
open PRs; preserve commits from either machine and resolve conflicts explicitly.
Do not overwrite another machine's work with a force push or destructive reset.
Stage the task's files explicitly so unrelated generated media cannot enter a PR.

For changes spanning both platforms, use the existing CI workflows and
[Windows portable guide](docs/releases/windows-portable.md). Report which checks
ran locally and which ran on Windows CI; a macOS pass is not a Windows pass.
Creating a PR, merging it and publishing a release are distinct actions. Follow
the user's requested scope and existing authorization for each.

## Documentation and delivery

Keep public English and Chinese descriptions aligned when changing product claims.
Link to release documentation instead of copying a version snapshot into every
agent instruction. Keep detailed test commands in the verification guide.

Commit reusable fixtures and intentionally selected public media only. Do not
commit credentials, private source media, generated Job stores or build/render
caches. Promotional edits should preserve authentic source assets and avoid
claiming that generated animations have passed reviews that did not occur.

Report the changed behavior, verification performed, remaining limitations and
commit/PR when applicable. State whether a change is local, pushed or merged.
