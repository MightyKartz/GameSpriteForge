# Project resource library

Use the selected executable's `doctor --json` and command help. These commands
require `project_asset_catalog_v3`; transfer and merge also require
`project_asset_portability`. They are included in the default development build.
An older release with the same version string may lack them. Do not change an
existing game's toolchain pin just to use this guide.

## Register existing resources

```bash
forge project init --path ./library --name "Game resources" --local-assets --json
forge asset scan --project ./library --root ./source-media --out scan.json --json
forge asset register --project ./library --input scan.json --json
forge asset search --project ./library --kind audio --tag battle --limit 20 --json
forge asset history --project ./library --id music.battle --json
```

Scan only the explicitly selected root. Review `issues` and edit the returned
items before registering: choose stable `assetId`, name, tags and optional
`purpose`. Keep `expectedContent` intact: registration rechecks the actual bytes.
Set `newRevision:true` for an intentional additional version of an existing ID.
The same bytes and source assertions register idempotently. Optional `origin`
contains user-declared `tool`, `model`, `license` and `notes`. Different origin
assertions stay separate even for identical media; they do not approve licensing.
Optional `variant`, `role` and `parentRevisions` record explicit declarations;
Forge does not infer lineage from filenames. Parent IDs must identify existing
revision objects. Purpose/name/tags are resource metadata; change them through
`asset annotate`, without rewriting content history.

PNG/JPEG/WebP, GIF/video, WAV/other audio, generic files and valid Packs can be
registered. Registration does not imply support for processing or native delivery:
the audio processing path accepts WAV, and Godot installation requires a Pack.
Pack searches expose member IDs; installation always affects the entire Pack.

For existing V1/V2 catalogs, first run `project migrate-assets --project PATH
--dry-run --json`, then apply using `--apply --expected-sha256 DIGEST`. The digest
comes from that preview. Stop old writers first; migrated V3 is intentionally
unsupported by old catalog readers. Exact legacy bytes are retained as a local
backup. Missing sources and old installation statements remain unverified.

Local preparation requests can include `assetProject` with `projectPath` and
`assetId`. CLI request-relative paths are resolved before planning. This opt-in
registers validated output with actual execution provenance. If output succeeds
but publication fails, use the reported `asset recover --input JOURNAL` path;
recovery revalidates existing bytes without another Provider request.

## Review, retain and deliver a specific version

```bash
forge asset annotate --project ./library --id music.battle --purpose battle --tag music --json
forge asset preview --project ./library --id music.battle --out ./review --json
forge asset review --project ./library --id music.battle --revision REVISION --domain auditory --verdict needs_review --reviewer "Reviewer" --statement "Original notes" --evidence ./notes.txt --json
forge asset select --project ./library --id music.battle --revision REVISION --json
forge asset retain --project ./library --id music.battle --revision REVISION --json
forge asset lock --project ./library --id music.battle --revision REVISION --out ./consumer-resources.lock.json --json
```

Open the generated `index.html` locally. The page is read-only and copies existing
media without altering GIF timing or audio bytes. Unsupported/missing previews
are reported. Choose `technical`, `visual`, `auditory` or `license` review domains;
user assertions and their retained evidence bind to exactly one content version.
New versions do not inherit approval. `asset status --state candidate|discarded`
changes disposition, without deleting media or changing a consumer lock. Search
can combine `--purpose`, `--tag`, `--kind`, `--disposition`, availability `--status`
and paired `--review-domain`/`--review-verdict` filters.

Retention copies and verifies selected media inside the library. It does not
reconstruct missing source requests or preserve an entire Job store. The resource
lock is independent of Forge/Godot toolchain locks and does not follow new versions.

For a Pack, use the existing `godot plan-install --library ./library --asset-id ID
--asset-lock ./consumer-resources.lock.json --project ./game --target
addons/forge_assets/ID --json` interface (or `--revision REVISION`). Inspect the
whole-Pack impact and execute the returned single-use plan. Installation uses
existing drift checks, snapshots and rollback; it is not visual/listening approval.
`asset installations` shows history. `project verify-assets` additionally checks
connected project registries and applicable installation baselines. It cannot
prove dynamic references or global absence of use.

## Move between Windows and macOS

```bash
forge project export-assets --project ./library --asset-lock ./consumer-resources.lock.json --out ./transfer --json
forge project verify-asset-bundle --input ./transfer --expected-sha256 DIGEST --json
forge project import-assets --input ./transfer --path ./restored-library --expected-sha256 DIGEST --json
forge project verify-assets --project ./restored-library --rebuild-index --json
```

Transfer the entire directory plus the exported manifest digest through a trusted
channel. Verification without that digest establishes self-consistency only.
Import requires a new destination. The bundle contains selected media, immutable
revision/review records, retained evidence and installation snapshots, plus the
exact consumer lock when supplied. It excludes machine-local root bindings,
caches and unrelated revisions. Existing immutable provenance and original media
are preserved; review user-provided notes and Pack metadata before sharing them.
Unselected parent revisions remain external lineage references and are reported
by the integrity audit. A bundle is not a recipe for regenerating missing sources.

Ordinary Git can transport `.forge/catalog.json`, `library/identity.json` and
`library/objects/`. Media, caches and `library/local.json` are ignored by default;
metadata alone cannot recover absent media. On another machine, explicitly map an
existing root with `project bind-root --project PATH --root-id ROOT --path LOCAL`.
Its audit reports mismatched bytes rather than silently updating content identity.
Logical paths use `/`; incompatible names, links and case collisions are rejected.
Canonical content digests travel unchanged; legacy Pack hashes retain their
original POSIX/Windows convention and are separately verified against actual bytes.

After Git brings both branches' immutable objects into one store, save the three
catalog versions to separate JSON files. Run `project merge-assets --project PATH
--base base.json --ours ours.json --theirs theirs.json --json`. A conflict-free
preview can be applied with `--apply --expected-sha256 DIGEST`. Parallel append-only
history is preserved. Conflicting selections, metadata or concurrent review
assertions for the same revision/domain are explicitly reported;
resolve those inputs and preview again. Forge does not use timestamps to choose
a winner. `verify-assets --rebuild-index` checks the merged authoritative records
and recreates only a disposable cache. Queries never migrate or repair storage.

All registration, lookup, review, transfer and inspection above are offline. They
do not launch models, install third-party tools or request Provider credentials.
