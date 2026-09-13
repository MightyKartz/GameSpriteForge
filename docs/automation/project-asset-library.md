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

## Scan, register, search and history

```sh
forge asset scan --project ./assets --root ./existing-media --out scan.json --json
forge asset register --project ./assets --input scan.json --json
forge asset search --project ./assets --kind audio --tag battle --limit 20 --offset 0 --json
forge asset history --project ./assets --id local-example --json
```

Scan is read-only except for the explicitly named, create-new output file. It
visits only the selected root, skips links, `.git`, `.godot`, `.forge`, `target`,
`build`, `dist`, `node_modules` and `__pycache__`, and stops recursion at Pack roots.
Invalid Packs and unreadable candidates appear in `issues`; duplicate names and
identical content are observations, not automatic merges. Raw file kinds are
extension classifications, not decode validation or quality approval.

The scan JSON is an editable batch using `schemas/asset-intake.schema.json`.
Choose logical `assetId`, `name` and `tags` before registration; the generated ID
is based on the path within the selected root, so separately scanned roots can
require explicit ID disambiguation. A single item uses the same one-element
`items` array. The caller may map an external manifest into this generic format.
No generator, Provider, ancestry, approval or license is inferred.

Registration rechecks each full content inventory under the catalog lock and
publishes one atomic head for the entire batch. Missing files, invalid Packs,
kind conflicts and changed bytes reject the batch; no partial records become
visible. A different content revision under an existing ID requires
`newRevision: true`. Identical content under that ID is idempotent and can add
an alternate source location. Existing names/tags remain unchanged on duplicate
registration. Registering the same bytes under different explicit IDs preserves
both logical resources. Immutable orphan objects after an interrupted/rejected
batch are harmless and are not automatically deleted.

Search returns `{items,total,offset,limit}` with one hit per revision, sorted by
`registeredAt` descending, then asset ID and revision digest. Migration uses the
original execution `createdAt`; it does not invent an observation timestamp.
History returns newest registered revisions first. Queries match ID/name/Pack
member ID; kind and tag filters are exact. Availability filters are `available`,
`changed` and `unavailable`, based on current content checks across known locations.
These are file states, independent of human review. Search verifies source bytes
and can therefore read large media; it does not create an index or write files.

The legacy `asset list/inspect` response shape remains unchanged; raw local
revisions are available through the new search/history commands. Pack-member
search indexes the whole Pack revision and does not enable member-only delivery.

## Register completed production outputs

Local static, audio, animation and character requests accept this optional field:

```json
{"assetProject":{"projectPath":"../library","assetId":"hero-idle"}}
```

The same field is available in layered requests. CLI paths resolve relative to
the request file. Initialize the V3 library first. The binding and project
identity participate in local Plan fingerprints; requests without a binding
retain their original serialized contract and behavior. Provider generation
continues to publish through its existing project binding, including parent
builds and retry/resume paths.

Only completed, validated Packs enter the library. A Pack requiring review is
not labeled approved. Publication requests are saved beside the Pack, outside
its content inventory, with a name ending in `.publication-<digest>.json`. For
local Jobs the `asset_publication_request` artifact identifies this file. If
catalog registration fails, the complete Pack remains available and the Job
reports `asset_registration_pending` with a recovery action:

```sh
forge asset recover --input /absolute/path/to/output.gsfpack.publication-digest.json --json
```

Recovery verifies the recorded bytes and only repeats registration. It does not
create a Job or call a Provider. The durable request remains available for
idempotent retries; it is not deleted after successful recovery. A failed Job
remains historical execution evidence; the recovery response and catalog show
successful registration separately. Local and direct-command publication records
use actual execution IDs, while direct layered outputs carry command/build
identity without a fabricated Job. User selection and human review are unchanged.

V3 game-art reuse searches available historical revisions in dependency order.
Spec, workflow, style/subject locks, Provider configuration, dependency revision
and Pack content must match. Reverting a spec may reuse an earlier coherent build
without modifying either the latest publication or the user's selected revision.
