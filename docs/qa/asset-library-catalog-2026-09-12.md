# Versioned project catalog verification

Date: 2026-09-12. Base: `b56dd185a8e44fd92f2751268a05db0024ca5f6e`.
Scope: PR 1 of the [asset library plan](../architecture/forge-project-asset-library-plan.md).

The prerequisite [PR #34](https://github.com/MightyKartz/GameSpriteForge/pull/34)
passed the macOS quality matrix, Windows CLI contracts and Windows portable
package checks. Windows CLI included the identical portable digest vectors and
catalog publication tests. Those checks establish the foundation; they do not
by themselves validate this dependent catalog PR.

This phase adds provider-neutral initialization and explicit V1/V2 migration.
The V3 head references immutable, integrity-checked asset/revision objects;
local root mappings and byte-exact migration backups stay separate. Migration
previews bind both old JSON and observed Pack contents, reject input drift,
and preserve unknown/missing history as assertions. Interrupted migration can
resume with the original identity and inputs. Foreign legacy head overwrites,
object tampering and storage links are rejected.

All production catalog writers use the V3 adapter when selected. Successful
builds retain exact dependency publications in build-state before starting the
child; new revisions do not implicitly select a delivery version. Plans validate
referenced objects and local bindings. Godot catalog attachment no longer
requires Provider configuration, rejects changed publication bytes and keeps
the existing installation transaction. Restoring the old head restores its
complete prior metadata graph; orphaned immutable objects are not deleted.

Local validation:

| Check | Result |
| --- | --- |
| Core unit suite | 137 passed |
| `asset_library_tests` | 10 passed: initialization, migration, drift, recovery, history, concurrency, rollback head, metadata tampering, storage links and installation identity |
| `game_art_build_tests` | 10 passed, including V3 reuse without new Provider requests and exact dependency revisions in build-state |
| `godot_install_transaction_tests` normal cases | 5 passed; 4 native cases are separately ignored by default |
| `real_godot_install_with_v3_catalog --ignored` | Passed with actual Godot 4.6.3; installed-resource verification passed |
| CLI suite with `game-art-manifest` | 30 passed |
| `test-asset-library-cli.py` | Passed using isolated metadata and the real legacy foundation binary; old binary rejects V3 |
| Strict Clippy for Core/CLI all targets with `game-art-manifest` | Passed |

The historical executable used by the CLI test was built at
`5811f1d9454b69cfee0d070840873d663fd1a52e` with default features, SHA-256
`d221c70b8d49be232dce69ab3c1be51d8a40642bb46048a8f78be9312bae6865`.
It is an actual older binary, not a repackaging of the new one. Current-build
identity is recorded after the implementation commit below.

The CLI smoke checks mutation-free initialization failure, project/asset reads
and migration preview, byte-identical backups, stale-preview rejection and
absence of newly created Job/Plan stores before the separate doctor check.
Fixtures are synthetic; Sword assets, toolchain pins and Provider credentials
were not used or modified.

Search/registration, producer intake bindings, historical-version consumption,
preview/review and cross-machine transfer remain the following PRs. This phase's
legacy catalog view uses the latest successful build and its native Pack hash;
full historical build-fingerprint selection and foreign-platform delivery
adapters are not claimed here. Remote checks for this head are tracked on its PR.

Clean default-feature verification: source `c2dc3acf7046fe41ef9c1812b4d214c16e901ee5`,
`dirty=false`, `features=[]`, `aarch64-apple-darwin` debug build. Binary SHA-256:
`0ac46a86f549bdf6eae648f8fd75799676ecdc4f0d36ec91cd3bf20b3dc06599`.
The CLI smoke passed against this executable and the preserved PR 0 executable,
including actual legacy-binary refusal of V3 catalogs.
