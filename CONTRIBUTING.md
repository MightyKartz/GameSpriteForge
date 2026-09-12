# Contributing to Forge

Forge is a Rust CLI workspace. Its four packages are `cli`, `core`, `pack`, and
`providers`; source builds and releases use Cargo. Node.js, npm, and Tauri are
not required. The retired desktop application is available in Git history.

Use the repo's [forge-dev skill](.agents/skills/forge-dev/SKILL.md) for source work
and [forge-use skill](.agents/skills/forge-use/SKILL.md) for asset production.
The [Codex artwork workflow](docs/automation/codex-local-assets.md) records which
local development implementations are required by existing game consumers.

## Development build

```bash
cargo build -p forge-cli
cargo test
```

Windows source development requires Rust's MSVC toolchain and Visual Studio C++
build tools/Windows SDK. From a configured developer PowerShell:

```powershell
cargo build --locked -p forge-cli --no-default-features
cargo test --locked -p core --test image_contract_tests --test delivery_audit_tests --test static_delivery_tests
python scripts/test-image-contract-cli.py --forge target/debug/forge.exe
```

The Windows CI job checks source compilation and offline delivery contracts. It
does not create an official Windows release or change `doctor.platformSupported`.
Its embedded-guide check uses `test-cli-skill.py --guide-only`: Windows can read
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
export FORGE_DEV_CODESIGN_IDENTITY="Apple Development: Your Name (J6P96F432P)"
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

Godot integration tests use Godot 4.6.x when it is available at
`/Applications/Godot.app` or on `PATH`. Video tests require the pinned FFmpeg
toolchain described in `third_party/ffmpeg/BUILD.md` or compatible local
development binaries.

The local animation CLI/Godot smoke requires Pillow; install
`scripts/requirements-local-assets.txt` into a Python virtual environment. CI uses
Python 3.12. Its source-transform fixtures are synthetic and do not require
consumer artwork or Provider credentials.

## Embedded guide and optional Codex skill

The single maintained skill source is `.agents/skills/forge-use/`. Keep every
runtime reference and example inside that directory; the CLI embeds its explicit
file list at compile time. `forge guide` serves that same content directly, and
optional skill installation writes it into a Codex discovery directory. Maintain
both relative resource links for installed skills and `guide` commands for
reading embedded references/examples without installation. Do not add a second
guide content tree.

When adding a bundled file, update the list in `packages/cli/src/skill.rs`, expose
it in the guide resource list, and verify guide reads and the complete installed
bundle:

```bash
cargo build --locked -p forge-cli --no-default-features
python3 scripts/test-cli-skill.py --forge "$PWD/target/debug/forge"
```

This check exercises offline guide reads from a standalone binary and isolated
project/user installations, including exact resource content, update backups and
preservation of modified content. Release checks
also run it through the installer's public launcher. Skill-only changes must run
CI because they change the binary's embedded payload. Guide reads must remain
read-only with no credential or Job-store access. Updating a CLI changes the
guide that executable returns; installed skill copies retain explicit update and
user-modification protection. CLI installation does not register a Codex skill.

Before submitting changes:

```bash
cargo fmt --all -- --check
cargo test
# The post-v0.2 Environment/Terrain/Building/Map commands are source-gated.
cargo test -p forge-cli --features world-assets
bash scripts/test-world-assets.sh
bash scripts/test-cli-signing-contract.sh
```

Do not commit credentials, Provider authorization headers, generated OAuth
state, or private model outputs.
