# Forge CLI + Godot consistency V1 QA — 2026-08-02

## Scope

This pass verifies the CLI-first product boundary for `v0.2.0-cli.1`: immutable
Style Locks, Character/Icon/Prop generation, `consistency@1.0.0`, targeted retry,
gray-band review, `.gsfpack` V2, external-texture Godot 4.6.x delivery, the
command-line installer, and the macOS ARM64 Release workflow.

Desktop and MCP source directories remain in the repository but are not Cargo/npm
workspace members, default scripts, README product surfaces, or Release inputs.

## Automated evidence

### Rust workspace

Commands:

```text
cargo fmt --all -- --check
cargo check
cargo test
```

Result: passed for CLI, Core, Pack, and Providers. The test set includes 27 Core unit
tests, automation and Pack contracts, local FFmpeg sampling, fake OIDC and xAI REST
failure/retry/security cases, a style-locked Character Pack, and Icon/Prop Godot
installation against Godot `4.6.3.stable`.

### CLI product contract

Command: `scripts/test-cli-product.sh`

Result: passed in an isolated JobStore/PlanStore. It covers:

- project initialization and immutable fixture Style creation;
- Character, Icon Set, and Prop Set generation;
- static targeted retry with the accepted sibling copied and SHA-256 checked;
- Character direction retry with the other three provider media pairs reused;
- detached asynchronous generation and cooperative Character cancellation;
- a gray-band Icon result promoted only through explicit review;
- Pack validation and a complete two-item retry Pack;
- a separate single-use Godot install plan;
- external Godot textures, resource size below 1 MiB, and absence of embedded Image
  `PackedByteArray`;
- a credential-pattern scan of the complete fixture JobStore.

### Installer contract

Command: `scripts/test-cli-installer.sh`

Result: passed on macOS ARM64. It verifies first install, idempotent reinstall, one PATH
marker, version upgrade with the old version retained, SHA-256 rejection, Mach-O
signature rejection, simulated network failure, and preservation of the prior command
target after every failed install.

### Product diagnostics and release metadata

- `forge doctor --json`: passed; reported CLI `0.2.0-cli.1`, Godot 4.6.3, local
  FFmpeg/FFprobe, the fixture Provider, and a non-interactive xAI status.
- `jq empty schemas/*.json`: passed.
- `git diff --check`: passed.
- `.github/workflows/release-cli.yml`: parsed successfully as YAML.
- `scripts/test-forge-dev-skill-source.mjs`: passed with the CLI-first skill contract.

The Release workflow builds on `macos-15`, pins FFmpeg 8.1.2 with GPL/nonfree disabled,
ships the exact source and LGPL text, signs all three Mach-O executables, notarizes the
ZIP, emits build information and a pinned-generator CycloneDX SBOM, and binds that SBOM
to `actions/attest@v4` provenance.

## External release gates

The current xAI credential check waited on operating-system credential-store approval
in this non-interactive QA session and was cancelled without exposing output. The
existing model-neutral Provider QA records three successful real xAI Character→Godot
runs, but the new Icon/Prop Style-Lock path still needs one real run each before tag
publication.

Developer ID signing, Apple notarization, Artifact Attestation publication, and a clean
Mac install require repository/Apple secrets and can only be proven by the tag workflow.
No GitHub Release was created in this pass. OAuth remains Preview pending written xAI
commercial approval; API Key remains the stable release authentication path.

## Independent revalidation

A fresh verification pass was run after implementation without changing Provider or
engine state:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test
cargo build -p forge-cli --release --target aarch64-apple-darwin
scripts/test-cli-product.sh
scripts/test-cli-installer.sh
npm run test:scripts
jq empty schemas/*.json
git diff --check
```

All commands passed. The product contract again completed project/style creation,
Character/Icon/Prop generation, full-pack targeted retry, gray-band review, detached
worker cancellation, credential-pattern scanning, Pack validation, and Godot 4.6.3
headless installation. The installer again passed initial install, idempotent reinstall,
upgrade, bad checksum, bad signature, and unavailable-release preservation cases.

The optimized release binary was confirmed as a 64-bit ARM64 Mach-O. In a simulated
version directory, `forge doctor --json` resolved sibling `ffmpeg` and `ffprobe`, found
Godot 4.6.3, and reported version `0.2.0-cli.1`; `otool` showed no FFmpeg/libav dynamic
linkage. The simulation used locally installed helpers only to verify resolution order,
not as evidence for the pinned FFmpeg 8.1.2 GitHub build.

No real xAI generation, Developer ID signing, Apple notarization, Artifact Attestation
publication, tag, or GitHub Release was performed during this independent pass. Those
remain the external release gates described above.
