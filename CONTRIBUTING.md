# Contributing to Forge

The public product is the Rust `forge` CLI. The retained desktop and MCP code
are not part of the default build or release.

Read the [workflow and release boundaries](docs/architecture/forge-workflow-boundaries.md)
before choosing a feature surface. The CLI declares `default = []`; optional
implementations and dedicated examples are not automatically release capabilities.

## Development build

```bash
cargo build -p forge-cli
cargo test --workspace
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

For Rust/CLI changes, run the applicable checks before submitting:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/test-cli-product.sh
```

For documentation or agent-instruction changes only, validate content, file references,
links, and skill metadata as applicable; a full Rust rebuild or test run is not required.
Repository-wide agent guidance lives in [AGENTS.md](AGENTS.md).

For optional features, also build and verify the exact affected surface. Choose
the relevant contracts below; passing the default build does not exercise gated
CLI commands.

| Changed surface | Build and focused verification |
| --- | --- |
| Project build | `cargo check -p forge-cli --features game-art-manifest`; `scripts/test-game-art-manifest.sh` |
| Stage 3 static assets / audit | `cargo check -p forge-cli --features collection-assets`; `cargo test -p providers --test stage3_static_contract`; `scripts/test-stage3-static.sh` |
| Grid generation | `cargo check -p forge-cli --features grid-generation`; `scripts/test-grid-generation.sh` |
| Environment / Terrain / Building / Map | `cargo test -p forge-cli --features world-assets`; `scripts/test-world-assets.sh` |
| Signing or distribution | `scripts/test-cli-signing-contract.sh`; the applicable installer/release checks |

PR CI runs four independent groups: the existing release matrix, project assets,
subject/grid contracts, and pixel/identity processing. Run an additional group with
`FORGE_EXPERIMENTAL_SUITE=project-assets bash scripts/test-experimental-feature-matrix.sh`
(or `subject-grid` / `processing`). Each produces its own JSON report. The retained
human-labeled identity calibration corpus is not versioned; CI records that
calibration as unexecuted while running portable identity tests and compiling the
evaluator.

Keep dated verification under `docs/qa/`, separating fixture contracts, real-provider
acceptance, and manual visual review. Real-provider runs require explicit acceptance
and request/cost limits; the test commands above are offline. Do not carry historical
passing counts forward as evidence for a changed working tree.

Do not commit credentials, Provider authorization headers, generated OAuth
state, or private model outputs.
