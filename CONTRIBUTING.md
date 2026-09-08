# Contributing to Forge

The public product is the Rust `forge` CLI. The retained desktop and MCP code
are not part of the default build or release.

Use the repo's [forge-dev skill](.agents/skills/forge-dev/SKILL.md) for source work
and [forge-use skill](.agents/skills/forge-use/SKILL.md) for asset production.
The [Codex artwork workflow](docs/automation/codex-local-assets.md) records which
local development implementations are required by existing game consumers.

## Development build

```bash
cargo build -p forge-cli
cargo test
```

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

## Bundled Codex skill

The single maintained skill source is `.agents/skills/forge-use/`. Keep every
runtime reference and example inside that directory; the CLI embeds its explicit
file list at compile time. When adding a bundled file, update the list in
`packages/cli/src/skill.rs` and verify the complete installed bundle:

```bash
cargo build --locked -p forge-cli --no-default-features
python3 scripts/test-cli-skill.py --forge "$PWD/target/debug/forge"
```

This check exercises a standalone binary and isolated project/user installations,
including update backups and preservation of modified content. Release checks
also run it through the installer's public launcher. Skill-only changes must run
CI because they change the binary's embedded payload.

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
