# Verify a project's image contract

Source builds with the `project_image_contract_verification` capability expose a
read-only check for an existing image lock and the PNG files it describes:

```bash
forge asset verify-images --root /absolute/project \
  --lock tools/asset-lock.json --scan game --json
```

Select and verify the executable before using this command. An older pinned
release does not acquire this capability from updated documentation. The check
can inspect images installed by an older Forge version without upgrading or
reinstalling those assets.

## Paths and exact file membership

`--root` names the project root. `--lock` is relative to that root or an absolute
lock-file path. At least one `--scan` directory is required; scan directories and
image paths are relative to the root. Repeat `--scan` for separate directories,
or use `--scan .` to scan the root:

```bash
forge asset verify-images --root /absolute/project \
  --lock tools/asset-lock.json --scan game --scan shared-textures --json
```

Every locked image must belong to the selected scan directories. Their combined
PNG file set must match the image lock exactly: missing files, unlocked PNGs and
duplicate locked paths fail verification. Overlapping scan directories do not
count the same file twice. Scanning excludes `.godot` and `.git` directories;
generated Godot import caches are outside this image contract. Symlinks and
paths escaping the selected project root are rejected. Choose the scan scope
explicitly instead of inferring it from a lock filename.

Paths use project-relative components, with no `.` or `..` components except
the standalone `--scan .`. Backslash separators are normalized to `/` in locks;
file spelling is case-sensitive so the contract remains portable. IDs and paths
that collide ignoring case are rejected, as are Windows junctions/reparse points.

## Input contract

The lock accepts `schemaVersion` as the number `1` or the string `"1"`. Each
entry in `images` requires `id`, `path`, `width`, `height`, `sha256` and
`requiresAlpha`. Dimensions are positive integers and SHA-256 is a 64-digit
hexadecimal value. For example:

```json
{
  "schemaVersion": 1,
  "images": [
    {
      "id": "player_portrait",
      "path": "game/addons/forge_assets/player/items/portrait.png",
      "width": 512,
      "height": 512,
      "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
      "requiresAlpha": true
    }
  ],
  "expectedCounts": {
    "images": 1
  }
}
```

Replace the example hash with the SHA-256 of the reviewed file. This command
does not generate a lock or accept current files as a new approved baseline.

The verifier checks the PNG file bytes against `sha256`, decodes the image and
checks its dimensions. Optional `expectedCounts.images`, when present, must
match the number of locked images. Omitting it leaves that declared-count check
unavailable; the exact scanned-file comparison still applies.
`expectedImageCount: null` and `countCheck: "not_declared"` identify an omitted
count; a declared count must be a nonnegative integer, not `null`.

Inspection limits are 16 MiB per lock, 128 MiB per PNG and 33,554,432 pixels per
image. PNG container integrity is checked through IEND. Pixel checks preserve
16-bit alpha, including faint nonzero border pixels.

The pixel constraints are:

- `requiresAlpha: true` requires PNG color type 6 (RGBA), both fully transparent
  and visible pixels, and a fully transparent outer border by default. Having an
  alpha channel alone is insufficient.
- `requiresClearBorder: false` permits visible pixels on the outer border, for
  example where a source deliberately crops an arm. The border requirement only
  applies when `requiresAlpha` is true; its default is `true`.
- `requiresOpaque: true` requires every pixel to be fully opaque, rejecting both
  fully transparent and partially transparent pixels. Its default is `false`.
  `requiresAlpha: false` does not imply opacity; it also permits RGB PNGs.

Existing locks may contain other project metadata. The report explicitly lists
unverified top-level fields, image-entry fields and other `expectedCounts`
members in `notCheckedFields`. These fields are retained in the source lock but
are not interpreted as verified evidence. For example, `sourceManifest`,
`nativeSourceSHA256`, `visibleRegion`, rig definitions and audio metadata need
their own checks.

## Results and verification limits

With `--json`, a completed check returns the normal CLI envelope and a report
whose scope is `image_contract_only`. Failed checks return `ok: false`, retain
the diagnostic `data` including issues, and exit with status 1. Automation should
inspect the exit status and report together; it must not discard mismatch details
merely because `ok` is false. Invalid command arguments or unreadable/invalid
inputs can instead use the standard CLI error path.

The command does not write project files or locks, use a Job store, create Plans
or Jobs, start Godot, or call a Provider. It also does not update asset counts or
silently accept extra PNGs. A successful check establishes the declared image
contract for the scanned files at inspection time.

This does not establish installation provenance or receipts, rig or audio
correctness, Godot import-cache integrity, exported PCK membership, EXE identity
or visual approval. Keep the project's existing checks for those concerns. Use
`godot verify-install` and retained delivery receipts for installations that
support those contracts; an image-lock check cannot create missing historical
installation evidence.
