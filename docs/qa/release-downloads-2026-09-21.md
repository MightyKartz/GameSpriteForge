# Release download audit and consolidation

Baseline: [Forge v0.6.3](https://github.com/MightyKartz/GameSpriteForge/releases/tag/v0.6.3),
published September 20, 2026. [Machine-readable evidence](artifacts/release-downloads-2026-09-21.json).

The previous release delivered PNG animation playback, H.264 MP4 export and GIF
transparency/timing fixes. Its 14 uploaded attachments mixed installation files
with dependency source, license/build documents, SBOM and verification data.
Windows exposed two similarly sized ZIPs, even though the recommended installer
already contains the portable archive. Release-note relative QA links were also
written for repository browsing rather than GitHub's Release page.

## Implemented download layout

- Two prominent, bilingual installation choices: the macOS online installer and
  the complete Windows installer ZIP. Commands identify the selected release.
- Eight uploaded assets instead of fourteen. Seven existing installer, portable
  archive and checksum filenames retain their exact bytes. This preserves
  historical installer/automation URLs; the duplicate Windows portable ZIP is
  explicitly an advanced download rather than a second required installer.
- One optional `forge-source-and-notices.zip` contains the previously separate
  source, notices, build recipes, SBOM and paired verification. It additionally
  retains the native license/build evidence from both packages and the pinned
  zlib source. Identical source tarballs are stored once. An internal manifest
  covers every support file.
- The release page has direct installer links, optional downloads inside a
  details section, per-asset hashes, and repository links pinned to the release
  commit. English and Chinese READMEs link directly to existing stable installers.

The assembler reuses the paired archive verifier before collecting files and
rejects missing or changed sources. It does not rebuild or rewrite native ZIPs.
Both platforms must still pass their existing native release jobs. Manual
workflow dispatch retains the assembled download set and generated notes for
inspection without creating a GitHub Release.

## Verification

All 12 synthetic release-gate tests passed, covering native identities,
checksums, installer drift, consolidation, missing support files, pinned-source
corruption, complete support inventory and release-note links. A three-platform
CI job runs the same tests, including Windows path handling. Workflow YAML,
Python syntax and changed Markdown links were checked locally.

A local rehearsal used the actual v0.6.3 macOS and Windows published archives.
All seven preserved files retained their published hashes. The source bundle
has 29 inventoried files, including both platforms' applicable license texts,
exact FFmpeg/zlib archives and paired verification. Every entry was rehashed.
No historical GitHub assets were modified.

The unchanged macOS installer installed the assembled payload into an isolated
temporary root. Its public launcher reported clean release commit `546bcef`,
default features, and binary SHA-256 recorded in the evidence. FFmpeg and FFprobe
8.1.2 were found and executed from that same installed payload with external
helper discovery disabled. This is historical-package assembly validation, not
acceptance of a newly compiled version. Windows archive/inventory checks ran
locally; Windows executables were not run on macOS.

The generated Rust SBOM remains macOS-target-specific and is labelled accordingly.
No signing, notarization, engine installation or consumer toolchain upgrade is
introduced by this packaging change. Future publication still uses the versioned
tag workflow and its native verification gates.
