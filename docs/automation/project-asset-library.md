# Project asset library

Development builds with `project_asset_catalog_v3` can initialize a local,
provider-neutral asset library. Check the selected executable's `doctor --json`
capabilities before use. This is PR 1 of the
[implementation plan](../architecture/forge-project-asset-library-plan.md);
scan/search, retention, preview and transfer commands are subsequent work.

```sh
forge project init --path /absolute/assets --name "Local assets" --local-assets --json
forge project inspect --project /absolute/assets --json
forge asset list --project /absolute/assets --json
```

Initialization creates `.forge/catalog.json` version 3. It does not create a
generation `forge-project.json` or select a Provider. When a generation project
already exists, the library reuses its project identity and leaves its generation
configuration unchanged. Existing catalogs require explicit migration.

```sh
forge project migrate-assets --project /absolute/assets --dry-run --json
forge project migrate-assets --project /absolute/assets --apply \
  --expected-sha256 PREVIEW_EXPECTED_SHA256 --json
```

The preview writes nothing. Its `expectedSha256` binds both the original catalog
and observed resource contents; `catalogSha256` separately identifies the exact
original JSON. Apply rejects a stale preview and retains the original bytes in
`.forge/library/backups/<catalogSha256>.json`. Missing or changed Packs remain
historical assertions with unknown verified content; migration never fabricates
missing Jobs, images, reviews or installation snapshots. A foreign-platform path
that cannot be located remains unconfigured, with its original spelling retained
in the backup. This phase does not yet provide root rebinding or transfer.

Stop old writers and pin the upgraded executable before migration. Tested old
CLI versions reject V3 reads; direct external writes cannot be prevented by a
schema number. An identity marker detects a downgraded/replaced head. A migration
interrupted before its head commit can be repeated with its original preview,
name and unchanged inputs. After completion, restore the saved V3 head if an
external writer overwrites it; do not treat a legacy V2 view as a writable V3
replacement. Ordinary reads never migrate automatically.

The head points to immutable, SHA-256-addressed JSON objects under
`.forge/library/objects`. Asset records hold history, local-location references,
selection and installation associations; revisions hold content inventory and
execution provenance. Every new record is fully written before the one head
replacement. Installation rollback restores that head, which again references
the previous complete records; unreferenced staging objects are harmless and no
garbage collection is performed. Machine roots, original backups, media and
preview caches are ignored by the library's internal `.gitignore`. Metadata alone
does not transport or back up media.

Production routes use the same publication boundary as legacy catalogs, with
execution/Pack identity preventing duplicate revisions. A latest successful build
reference supports existing generation diff/reuse separately from the user's
delivery selection. New builds do not choose themselves for a game. Project
builds capture exact dependency publications before starting each child and
retain them in `resolvedDependencies` in build-state; completed upstream results
are used instead of rereading an unrelated concurrent build's current entry.

V3 catalog roots also work with the existing `godot plan-install
--catalog-project` contract without requiring generation configuration. This phase
retains its current-build, whole-Pack semantics. Explicit historical-version
installation and member impact reporting arrive with the version-consumption PR.

Schemas: [catalog head](../../schemas/project-catalog-v3.schema.json),
[immutable objects](../../schemas/asset-library-object.schema.json).
Read-only queries validate object digests; neither structural validity nor an
installation association certifies visual, listening or licensing approval.
