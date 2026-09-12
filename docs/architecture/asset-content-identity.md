# Asset library content identity

This foundation implements the first prerequisite of the
[project asset library plan](forge-project-asset-library-plan.md). It does not
migrate catalogs, change Pack versions, or add user-facing library commands.

`forge-content-inventory-v1` identifies a directory's regular file contents.
`content_digest::directory_inventory` returns its algorithm, digest and portable
file inventory. Paths are exact UTF-8 relative names joined with `/`, with no
case conversion or Unicode normalization. Nonportable Windows names, case
collisions (including directory prefixes), links/reparse points and duplicate
inventory entries are rejected. Empty directories are not content.

The SHA-256 input is the UTF-8 algorithm name followed by a zero byte. Files
are sorted by their exact UTF-8 path bytes. Each contributes `file\0`, an
unsigned 64-bit little-endian path byte length, the path bytes, an unsigned
64-bit little-endian content length, and the lowercase 64-byte hexadecimal
SHA-256 of its contents. Machine roots, mtimes, labels and provenance are absent.
Equal identities prove equal inventories subject to SHA-256; they do not merge
separate ownership, licensing or generation assertions.

The shared [test vector](../../examples/asset-library/content-digest-vector.json)
includes an empty file, nested paths, spaces, Chinese names, and a sibling path
whose ordering differs between string and path-component ordering. Expected
digests were calculated independently with Python `hashlib` and `struct`.
The same fixture runs on macOS and Windows CI.

Existing `forge-directory-hash-v2` remains unchanged. Its native path spelling
is part of historical Pack, plan and receipt hashes. The new
`legacy_directory_digest` can recompute v2 using an explicitly selected original
POSIX or Windows path style after validating portable contents. It preserves
the old path-component ordering and byte framing. This is a verification
primitive, not authority to rewrite a receipt or infer its producer. A future
library can record both identities after verifying the retained bytes and
original evidence. Receipts without those bytes cannot restore media.

Production now uses `publish_catalog_asset`, whose key is asset ID + nonempty
source Job ID + Pack digest. Replaying a completed publication does not rewrite
catalog bytes, timestamps, reviews or installation evidence. A changed Pack or
new execution replaces the legacy current entry without inheriting old evidence.
The explicit V1/V2 registration APIs retain their upsert contract and share the
same locked update implementation. A pre-upgrade child V1 entry may be enriched
once by its parent's full V2 provenance for the same execution and bytes.

Project-build children no longer publish preliminary entries: their parent
persists successful child output in build-state, then publishes complete
provenance. Recovery can repeat that finalization without another generation.
Ordinary asset retries continue to publish themselves. Catalog schema remains V2;
immutable version storage and explicit migration belong to the next PR.
