# Inspect sources and retain delivery evidence

These additions require the selected binary's `reviewed_source_hashes`,
`source_png_inspection`, `delivery_receipts`, `godot_install_verification` and
`transactional_godot_install` capabilities as applicable; cache baseline checks
also require `godot_import_cache_integrity`. Older release pins do
not acquire them by reading this guide. Keep the consumer's tested executable.

## Measure before processing

```bash
"$FORGE_BIN" source inspect --path /absolute/sheet.png \
  --frame-width 128 --frame-height 128 \
  --preview-dir /absolute/new-source-preview --json
```

The report measures PNG dimensions, color type/bit depth, decoded alpha,
multi-threshold bounds, grid divisibility and cell edge contact. Alpha counts,
bounds and previews use 8-bit RGBA, reducing 16-bit sources for the diagnostic.
Optional light
and dark composites go into a new directory; the source stays unchanged. Omit
`--preview-dir` for a read-only operation. Grid contact is a review cue, not proof
that the body was cut. RGB beneath zero alpha can be legitimate. Baked
checkerboards, style and recognizable motion still require visual inspection.

## Bind reviewed input bytes

Local static, single-action and multi-action preparation requests accept optional
`sourceLocks`. When present and nonempty, it must cover every distinct source
file exactly once, with no unrelated files or duplicate canonical paths:

```json
"sourceLocks": [
  {"path": "/absolute/approved.png", "sha256": "64_HEX_DIGITS_FROM_THE_REVIEWED_FILE"}
]
```

Use actual SHA-256 values from the reviewed bytes. PNG sequences include each
frame; sheets/videos include their source file; Pack inputs include every file
in the Pack directory. A mismatch or incomplete closure prevents plan creation.
The plan also rechecks inputs before execution. Omission retains the existing
unlocked contract. A matching hash establishes source identity, not who approved
it or whether the image is suitable.

Local source paths and their locks resolve relative to the request file (stdin
uses cwd) with `local_request_relative_paths`; Plans retain absolute paths for later execution. Older animation CLIs
resolved against cwd, so use absolute paths with those pins. Keep source hashes distinct from
normalized frame/Pack hashes.

## Export and verify a receipt

After a succeeded preparation Job, save a receipt before cleaning its Job store:

```bash
"$FORGE_BIN" receipt export --job PREPARE_JOB --out /absolute/prepared-receipt.json --json
"$FORGE_BIN" receipt verify --path /absolute/prepared-receipt.json \
  --pack /absolute/retained/Pack.gsfpack --expected-sha256 RECEIPT_SHA256 --json
```

The new output file contains the Job, resolved Plan/request, full Pack file
inventory, exact JSON report text and available source-transform/quality/usage
evidence. New CLI Jobs capture their executing binary/build and source hashes.
For old Jobs, `prepare.execution:null` explicitly means the producer evidence is
unavailable. The current `exporter` never replaces historical production identity.
Each embedded report retains its original artifact path and any recorded
producer hash. `recordedProducerSha256:null` means only its export-time text hash
is known; it does not claim those bytes were fingerprinted by the old producer.
Source hashes are retained evidence; verification does not reread the original
source paths. Retain the Pack itself alongside the receipt; the receipt contains
hashes and reports, not source media or texture bytes.

Retain the returned receipt SHA separately with the consumer's reviewed lock.
`--expected-sha256` checks those exact receipt bytes. Without an external expected
hash this is a consistency check, not proof against rewriting all the evidence.
Verification does not need the Job store, create Plans, start Godot or use a
Provider. `--pack` overrides the original Pack location after moving it.

Add `--install-job INSTALL_JOB` when exporting a subsequent delivery receipt.
The install must still be the current registered installation of those exact
Pack bytes. That receipt also preserves the install Job, native verification
report, installation snapshot and relevant registry entry. It can be verified
after copying the Godot project with `--project /absolute/moved-game`; project
and Pack overrides relocate reads without rewriting the retained evidence.

An optional `--review /absolute/review.json` attaches a separate review:

```json
{
  "schemaVersion": "1",
  "packSha256": "EXACT_PACK_SHA256",
  "status": "pending",
  "reviewer": "actual reviewer",
  "notes": "Review at intended game scale and on both backgrounds."
}
```

Allowed statuses are `pending`, `accepted` and `rejected`. Use `pending` while
review is outstanding; `accepted` or `rejected` must record an actual review.
The hash binds it to one output; generating a new Pack
does not transfer acceptance. Export never invents approval or overwrites an
existing receipt. `receipt verify` can return `verified:true` for a pending or
rejected review: read `visualReview` separately. Structural success, native
loading, visual review and device testing remain separate evidence.

## Audit an installation

```bash
"$FORGE_BIN" godot verify-install --project /absolute/game \
  --asset-key stable_asset_key --pack /absolute/retained/Pack.gsfpack --json
```

This read-only check compares ownership, usage, the registered snapshot hash,
installed file bytes, original Pack bytes and copied textures. With
`godot_import_cache_integrity`, snapshot v2 separately records `.import` routing
and the target textures' imported cache files. Existing caches must match:
restoring a PNG alone can leave Godot rendering stale cached pixels.
It does not perform a
fresh native load or rendering review. Older installations without the snapshot
need their separately retained historical receipt; do not manufacture a baseline
from potentially changed files and call it historical verification. When a copied
project lacks any recorded cache files or `.import` sidecars, stable asset
verification can pass but `cacheCheck.status` is `not_materialized` and
`materialized:false`; run a normal Godot import before native loading. If the
snapshot recorded no cache files, the status is `not_recorded`, also with
`materialized:false`; it makes no cache integrity claim. Receipt verification
exposes this as `installationCacheCheck`, and `installationVerified:true` alone
does not establish that caches are ready for native loading.
The audit does not recreate caches. Changed existing cache bytes or routing fail
verification instead of being treated as an absent cache.

## Check the consumer's image lock

With `project_image_contract_verification`, verify an existing project image
contract after installing or transferring assets:

```bash
"$FORGE_BIN" asset verify-images --root /absolute/project \
  --lock tools/asset-lock.json --scan game --json
```

The lock uses schema version `1` (number or string) and an `images` array with
`id`, root-relative `path`, `sha256`, `width`, `height` and `requiresAlpha` per
entry. Optional `expectedCounts.images` checks the declared count. `--lock` may
also be absolute. At least one root-relative `--scan` directory is required;
repeat it for separate directories. The combined PNG set must match the lock
exactly, excluding `.godot` and `.git`; symlinks and escaping paths are rejected.
Hashes, dimensions and declared pixel constraints are checked. Alpha-required
images need PNG type 6 RGBA, transparent and visible pixels, and a clear outer
border unless `requiresClearBorder:false`; `requiresOpaque:true` requires every
pixel to be opaque.

The report identifies its scope as `image_contract_only` and lists additional
unverified fields in `notCheckedFields`. Mismatches retain diagnostic `data` and
issues with `ok:false` and exit status 1. This reads files without changing the
lock, using a Job store or starting Godot. Legacy installed PNGs can be checked,
but success does not establish installation receipts, rig/audio contracts,
Godot caches, exported PCK/EXE identity or visual approval. Keep those separate
project and delivery checks; never accept unknown PNGs just to make the lock pass.

## Plan and execute an installation

An explicit `--target` wins when planning installation. Otherwise, an explicit
validated `--asset-key` supplies `addons/forge_assets/KEY`; with neither, the Pack
ID supplies that directory. Display filenames, including Chinese names, do not
select a shared sanitized fallback. Ownership and collision checks still apply.

New installers preflight dependencies, serialize Forge installs per project and
restore the old target, registry and affected texture caches on errors or
cancellation. Native success needs
successful exit codes, explicit completion results and validated resources;
ordinary warnings alone do not fail a Job. Serialize other Godot imports/exports
that share the project's cache as well. An unrelated editor or export process
does not acquire Forge's installation lock.
