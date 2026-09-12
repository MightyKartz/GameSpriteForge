# Asset library foundation verification

Date: 2026-09-12. Base: `1c1e7079f22ea0dba5958e4aec79cad69f7ade65`.
Scope: PR 0 of the [implementation plan](../architecture/forge-project-asset-library-plan.md).

The new content inventory has one platform-independent SHA-256 contract and
explicit POSIX/Windows historical v2 verification adapters. Existing v2 hash
implementation and Pack/receipt bytes remain unchanged. The shared vector is
independently calculated and covers nested paths, Chinese, spaces, binary data,
empty files and path-component versus string ordering.

Production catalog publication is execution/content-idempotent. Parent project
builds publish complete provenance once, after persisting successful child output.
Recovery leaves completed catalog bytes and subsequent installation/review
evidence unchanged. Pre-upgrade child-only provenance can be finalized once;
new content does not inherit previous evidence. Explicit legacy registration
upserts retain their existing API contract.

Local checks passed:

| Check | Result |
| --- | --- |
| `cargo test --locked -p core --test content_digest_tests --test game_art_build_tests --test delivery_audit_tests` | 3 content identity, 9 build/recovery, 6 delivery audit tests passed |
| `cargo test --locked -p core --lib` | 137 passed, including catalog, game-art diff/plans and install transaction units |
| `cargo test --locked -p pack` | 41 integration tests passed |
| `cargo test --locked -p forge-cli --features game-art-manifest` | 30 tests passed, including receipts, guides and build/launcher identity |
| `cargo clippy --locked -p core --all-targets -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |

The quality workflow now runs the exact content vector and catalog contracts on
both macOS and Windows, and accepts the planned stacked branch prefix. Remote
CI results are recorded by the pull request; local POSIX/Windows hash emulation
alone is not a claim that Windows execution passed.

All fixtures are synthetic, with isolated stores. No real Provider requests,
external audio applications or Sword asset imports were performed. This PR
provides foundation APIs; V3 migration and resource-library CLI commands remain
subsequent work.
