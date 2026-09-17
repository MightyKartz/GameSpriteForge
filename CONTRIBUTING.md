# Contributing to Forge

Forge is a Rust CLI workspace. Its four packages are `cli`, `core`, `pack`, and
`providers`; source builds and releases use Cargo. Node.js, npm, and Tauri are
not required for the CLI. Video projects have separate toolchains and local
instructions. The retired desktop application is available in Git history.

Start with [AGENTS.md](AGENTS.md) for repository boundaries and cross-machine
coordination. Use the repo's [forge-dev skill](.agents/skills/forge-dev/SKILL.md) for source work
and [forge-use skill](.agents/skills/forge-use/SKILL.md) for asset production.
The [Codex artwork workflow](docs/automation/codex-local-assets.md) records which
local development implementations are required by existing game consumers.

## Development build

```bash
cargo build --locked -p forge-cli --no-default-features
```

Windows source development requires Rust's MSVC toolchain and Visual Studio C++
build tools/Windows SDK. From a configured developer PowerShell:

```powershell
cargo build --locked -p forge-cli --no-default-features
cargo test --locked -p core --test image_contract_tests --test delivery_audit_tests --test static_delivery_tests
python scripts/test-image-contract-cli.py --forge target/debug/forge.exe
```

The Windows source CI job checks compilation and offline delivery contracts.
The separate native [portable-package workflow](.github/workflows/windows-portable.yml)
tests Windows packages and supplies the Windows artifact to the paired
[release workflow](.github/workflows/release-cli.yml). See the
[Windows portable guide](docs/releases/windows-portable.md) for release scope.
Source compilation alone does not establish package acceptance.
The embedded-guide check uses `test-cli-skill.py --guide-only`: Windows can read
`guide` and `skill show`, but the CLI's safe `skill install` implementation still
supports macOS/Linux only. Full skill installation/update tests remain in macOS CI.
Use `FORGE_GODOT_PATH` and `GODOT_BIN` for a separately verified native Godot
executable when running the ignored integration tests. Preserve a consumer's
pinned CLI and toolchain lock until its own import/export checks approve a new build.

For image-lock verification, see [image contracts](docs/automation/image-contracts.md).

### Stable macOS Keychain access

Rust's default linker signature is ad-hoc and changes after every rebuild. macOS
Keychain can therefore ask again before returning an xAI credential. After building,
sign the CLI with the team's persistent Apple identity and fixed identifier:

```bash
export FORGE_DEV_CODESIGN_IDENTITY="Apple Development: Your Name (TEAM_ID)"
scripts/sign-dev-cli.sh
```

Re-run the script after rebuilding. This is local development signing; it does not
replace Developer ID signing, secure timestamps, notarization, or the Release workflow.
An OAuth item created by an older ad-hoc binary may require one final macOS prompt;
choose **Always Allow** for the newly signed `dev.gamespriteforge.cli`, or log in once
from the signed CLI to recreate the item under its stable designated requirement.

Developers without the team identity may keep Preview OAuth in an owner-only file that
skips Keychain entirely:

```bash
forge provider login --provider xai --method oauth --credential-store file
```

The non-secret auth profile records only the selected method and storage backend.
Production releases default to Keychain.

Godot integration tests use Godot 4.6.x or 4.7.x when it is available at
`/Applications/Godot.app` or on `PATH`. Video tests require the pinned FFmpeg
toolchain described in `third_party/ffmpeg/BUILD.md` or compatible local
development binaries.

The local animation CLI/Godot smoke requires Pillow; install
`scripts/requirements-local-assets.txt` into a Python virtual environment. CI uses
Python 3.12. Its source-transform fixtures are synthetic and do not require
consumer artwork or Provider credentials.

## Embedded guide and optional Codex skill

The single maintained user-guide source is `.agents/skills/forge-use/`. Keep every
runtime reference and example inside that directory; the CLI embeds its explicit
file list at compile time. `forge guide` serves that same content directly, and
optional skill installation writes it into a Codex discovery directory. Maintain
both relative resource links for installed skills and `guide` commands for
reading embedded references/examples without installation. Do not add a second
guide content tree.

When adding a bundled file, update the list in `packages/cli/src/skill.rs`, expose
it in the guide resource list, and follow the
[embedded-guide checks](.agents/skills/forge-dev/references/verification.md#embedded-guide-versus-developer-documentation).
Edits to `forge-use` must run CI because they change the embedded payload;
`forge-dev` and root `AGENTS.md` are documentation only.
Guide reads remain offline and read-only. Updating a CLI does not register or
update separately installed Codex skills or change a game's toolchain lock.

## Before submitting changes

Select checks from the [developer verification guide](.agents/skills/forge-dev/references/verification.md)
according to the changed behavior. It covers resource libraries, static art,
animation/layers, audio, native Godot delivery and release installation.
Run Rust formatting and relevant tests for code changes; documentation-only edits
need link/example review and a clean diff. Embedded user-guide edits also require
the rebuilt CLI checks above. Optional feature suites apply when those features
change; release candidates follow the complete release workflow.

Describe what was verified, on which platform and with which binary. Separate
structural validation, native loading, visual/listening review and device testing.

Do not commit credentials, Provider authorization headers, generated OAuth
state, or private model outputs.
