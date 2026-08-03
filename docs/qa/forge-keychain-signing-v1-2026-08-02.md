# Forge Keychain and stable CLI signing QA — 2026-08-02

## Diagnosis

The existing xAI OAuth Keychain item is present and the API Key item is absent. The
previous development CLI was linker/ad-hoc signed with no Team ID; each rebuild changed
its CDHash, so macOS treated it as a new requesting program. The OAuth grant itself was
not lost.

An additional error-path issue caused `provider doctor` to resolve credentials a second
time after the first failure. During regression, a separate JobStore race was also
found: a worker progress write could overwrite a concurrent cancellation request with
an older full-record snapshot.

## Implementation

- Added a non-secret auth profile containing only `method` and `storage`.
- Existing credentials remain compatible and create the profile after the first
  successful legacy read.
- Provider resolution now reads only the selected API Key or OAuth entry.
- `provider doctor` failure fallback is non-interactive and cannot trigger a second
  Keychain read.
- Added `--credential-store keychain|file`; owner-only file storage is explicit and
  supported only for Preview OAuth development.
- Release identifiers are fixed to `dev.gamespriteforge.cli`,
  `dev.gamespriteforge.ffmpeg`, and `dev.gamespriteforge.ffprobe` under team
  `J6P96F432P`.
- The Release workflow signs with those identifiers and verifies identifier, Team ID,
  Developer ID authority, hardened runtime, and notarization inputs.
- The installer verifies the expected identifiers and, outside contract-test mode,
  Developer ID Application authority and Team ID.
- Added `scripts/sign-dev-cli.sh`; it rejects ad-hoc signing and signs the development
  CLI with a persistent Apple identity and fixed designated requirement.
- JobStore mutations now hold a per-job cross-process file lock and reread current state
  inside that lock, preventing stale worker snapshots from erasing cancellation.

## Verification

- Local development signing passed with
  `Developer ID Application: Ka Yan (J6P96F432P)`.
- `codesign --verify --strict` passed; the binary reports
  `Identifier=dev.gamespriteforge.cli` and `TeamIdentifier=J6P96F432P`.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo test`: passed, including 101 Rust tests after the new auth and concurrent
  JobStore regression cases.
- `scripts/test-cli-product.sh`: passed, including deterministic asynchronous
  cancellation and Godot 4.6.x headless installation.
- `scripts/test-cli-installer.sh`: passed, including wrong-identifier rejection and
  preservation of the previous install.
- `npm run test:scripts`, Release YAML parsing, and `git diff --check`: passed.
- Signed `forge doctor --json` and `forge provider list --json` returned immediately
  without reading Keychain.

## One-time migration gate

The existing OAuth item was created by the old ad-hoc binary. The first explicit read
from the new stable signed identity still waits for one macOS migration approval. The
test was cancelled without reading or printing credentials. The user must choose
**Always Allow** once, or log in once from the signed CLI to recreate the item. Future
rebuilds signed through `scripts/sign-dev-cli.sh` and official releases with the same
identifier/team should satisfy the same designated requirement.

Apple notarization and the final GitHub-signed artifact still require the tag workflow;
local development signing is not presented as notarization evidence.
