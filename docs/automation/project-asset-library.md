# Project asset library

The default v0.4.0 release includes a local, provider-neutral asset library
through `project_asset_catalog_v3`, with transfer and merge through
`project_asset_portability`. Check the selected executable's `doctor --json`
capabilities before use. The completed implementation follows the
[implementation plan](../architecture/forge-project-asset-library-plan.md).
The complete command workflow, including transfer, root rebinding, integrity
audit and explicit Git merge, lives in the embedded
[project resource guide](../../.agents/skills/forge-use/references/project-assets.md)
(`forge guide project-assets`). Remote and installed-package acceptance results
are tracked separately from compiled capabilities.

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
in the backup. Use explicit root rebinding or transfer from the project resource
guide when moving a library between machines.

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
--catalog-project` contract without requiring generation configuration. That
interface retains its current-build, whole-Pack semantics. Use the explicit
version interface below for historical-version installation and member impact
reporting.

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

The scan JSON is an editable batch using `schemas/asset-library-intake.schema.json`;
`schemas/asset-intake.schema.json` remains a compatible reference to that same
contract. Optional `issues` are scan observations and are ignored by registration.
Both scan output and a batch without `issues` use the same resource item definition.
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

```sh
forge asset tags --project ./assets --json
forge asset search --project ./assets --kind image --metadata-only --json
```

`asset tags` lists the kind/tag/purpose vocabulary already in use, with asset
and revision counts, so callers can compose exact filters instead of guessing
spellings. It reads catalog metadata only: no media bytes are read and no index
cache is created or repaired. `search --metadata-only` skips source-byte
verification for large libraries; hits report `status:"unknown"` instead of an
availability state, so combining it with `--status` is rejected. The response
shape is unchanged, and Pack member lists come from recorded source metadata
rather than a live directory read. Both commands are read-only; verified
searches remain the default everywhere else.

The legacy `asset list/inspect` response shape remains unchanged; raw local
revisions are available through the new search/history commands. Pack-member
search indexes the whole Pack revision and does not enable member-only delivery.

## Reconcile declared requirements

```sh
forge asset check-requirements --project ./assets --input needs.json --json
```

A requirements batch uses `schemas/asset-requirements.schema.json`: each entry
declares a stable `id` plus optional `kind`, `tags` (all must match), `purpose`,
`query` and a paired `reviewDomain`/`reviewVerdict`. IDs must be unique within a
batch. The command reconciles every entry against the library with the verified
search contract and reports one of four statuses:

- `covered`: an available revision matches and satisfies the review filter;
- `needs_review`: available revisions match but none satisfies the review filter;
- `incomplete`: revisions match but none is currently available;
- `missing`: nothing matches.

Each result carries the evaluated match count and up to five hits with revision,
selection, availability and review states. Matches are evaluated in search
order and capped at 1000 per requirement; beyond that, `total` reflects the
evaluated prefix. For a `missing` entry whose kind is
`icon_set` or `prop_set`, `suggestedRequest` is a pre-filled local static request
skeleton with TODO placeholders; other kinds never get an invented template.
The command is read-only, performs no generation or Provider call, and `covered`
describes inventory and recorded review state, not visual or license approval.
See `examples/asset-library/requirements.json` for a batch to adapt.

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

## Retain, lock and deliver a specific version

```sh
forge asset retain --project ./library --id hero --revision REVISION --json
forge asset select --project ./library --id hero --revision REVISION --json
forge asset lock --project ./library --id hero --revision REVISION --out ./game/.forge/resources.lock.json --json
forge godot plan-install --library ./library --asset-id hero --asset-lock ./game/.forge/resources.lock.json --project ./game --json
forge asset installations --project ./library --id hero --json
```

`retain` verifies source and staged copies, then makes the exact revision's
project-local media the preferred location. Original locations remain alternatives.
Retained bytes are under ignored `.forge/library/media/`; committing catalog JSON
alone does not transfer them. Existing corrupt retained destinations are rejected,
not silently overwritten. Source moves or Job-store cleanup do not affect intact
retained versions. Retention does not regenerate, resample or approve media.

`select` records the user's preferred revision independently of build reuse.
`lock` writes a consumer lock using `schemas/resource-lock.schema.json`; repeated
calls update only the explicitly named asset. The lock records library identity,
revision and canonical content hash. New publications and selection changes never
modify consumer locks. This file is separate from the Forge/Godot toolchain lock.

`godot plan-install` accepts either the original `--pack` path or an explicit
`--library`, `--asset-id` and `--revision`/`--asset-lock`. The resolved Plan includes
the complete Pack and all members in its impact description. Execution remains
single-use and rechecks media, resource-lock bytes, catalog state and target identity.
Only whole-Pack installation is supported, even if several logical resources share
that Pack. Related library revisions are disclosed; installation evidence links
the requested logical resource and exact revision. Installation does not establish
human approval or actual runtime references. Existing direct-Pack calls remain valid.

Installations use the existing native resource verification, ownership checks,
snapshots and transaction rollback. Library selection and consumer locks are not
changed by installation, including A → B → A delivery. Historical installation
associations remain inspectable through `asset installations`.

## Offline preview and revision review

```sh
forge asset preview --project ./library --kind audio --out ./audio-preview --json
forge asset preview --project ./library --id hero --revision REVISION_A --revision REVISION_B --out ./comparison --json
forge asset review --project ./library --id hero --revision REVISION_A --domain visual --verdict needs_review --reviewer YOUR_NAME --statement "Check the silhouette" --evidence ./review-notes.txt --json
forge asset reviews --project ./library --id hero --revision REVISION_A --json
forge asset search --project ./library --review-domain visual --review-verdict approved --json
forge asset annotate --project ./library --id hero --name "Main hero" --tag player --tag forest --json
```

Preview writes a new directory containing `index.html`, a small report and copies
of supported existing media. Open `index.html` in a local browser. No server,
external service, automatic browser launch or Provider is involved. A fixed,
CSP-hashed local script plays flat animation/character Packs directly from copied
PNGs, with native frame indices/durations, animation selection, pause, stepping
and background selection. Metadata is escaped and parsed only as data. Browser
scheduling may vary; engine blend modes require Godot review. Other PNG,
JPEG, GIF, WebP and browser-supported WAV/MP3/OGG/FLAC/M4A/MP4/WebM use native image,
audio or video elements; unsupported formats show an explicit message. Existing
GIF bytes/timing and audio bytes remain unchanged. Pack previews include available
member/animation/audio metadata and original technical reports. Layered transform
tracks remain visible as metadata; this gallery does not synthesize a new rendered
layered animation.

With `--id`, preview includes that asset's history; repeat IDs for several assets,
or repeat `--revision` with one ID for a comparison. Without IDs, query/kind/tag
filters use the search ordering and `--limit`/`--offset` pagination (default 20).
The JSON `selection` reports total matches and the selected page. One preview
contains at most 100 revisions. Missing sources are reported, with review/source
metadata still available. Existing output directories are never overwritten.

The report's `media` array (also in `preview.json`) gives machine callers one
entry per revision with copied files as preview-relative paths, their original
file or Pack-member labels and an `image`/`audio`/`video` type, so agents can
open exact revision media without walking the output directory. The manifest
only describes bytes this preview copied: it never derives new media, and
missing sources stay in `issues` without fabricated entries.

Review domains are `technical`, `visual`, `auditory` and `license`; verdicts are
`approved`, `rejected`, `needs_review` and `unknown`. These are explicit human
assertions, separate from the Pack's recorded technical checks. Evidence must be
an existing regular file and is retained with its exact SHA-256. New records append
to that content revision's review history; the latest record in each domain is
shown in search/preview. New revisions start without inherited conclusions, and
reviews never change selection, consumer locks or installation status. Historical
imported review/license assertions remain available as original source metadata.

The page escapes names, statements, paths and source metadata, restricts content
loading with CSP, and displays separate POSIX/PowerShell commands for recording
reviews. The page itself is read-only. Browser support for a media format is
independent of Forge's ability to register, process or install it.
