# Forge CLI v0.4.0 release preparation

The release promotes the resource-library, local audio and native delivery work
already merged into main. Workspace packages move from 0.3.2 to 0.4.0 without
dependency upgrades. The Windows portable distribution remains experimental and
unsigned; macOS Apple Silicon remains unsigned and not notarized.

The release workflow reuses `windows-portable.yml` on `windows-2025`. Windows
compilation, installation/reinstallation, actual historical-binary upgrade,
PowerShell 5.1 and native Godot checks execute on that Windows runner. A personal
Windows computer is not needed to produce the release package.

Publication now depends on both native jobs. A final job verifies archive and
payload checksums, the shared version/commit and expected platform, Windows clean
release identity, and exact installer bytes before publishing both archives in
one GitHub Release. Manual workflow dispatch exercises the same preparation and
verification without creating a release. macOS upgrade acceptance now uses the
latest prior stable release, v0.3.2.

Local preparation checks:

- Six release-gate regression tests passed, covering both ZIP layouts, mismatched
  identity, archive/payload corruption, unlisted files, dirty Windows builds and
  installer drift.
- The gate accepted the existing v0.3.2 macOS release archive and the Windows
  archive built from `af36e2258ec2828b0c5a40cf4b55f96c39f82c7e`, each checked
  against its own expected identity. This verifies real artifact layouts; it is
  not evidence that those historical packages share a commit.
- `actionlint` v1.7.12 passed for both changed workflows (shellcheck/pyflakes
  disabled). Its downloaded executable was checked against the release checksum.
- Locked Cargo metadata, Rust formatting and whitespace checks passed.

The preparation PR records exact preflight and PR CI runs. The published
`release-verification.json` records final archive/binary hashes and the tag's
common source commit, avoiding a documentation commit solely to record itself.
Source-media generation, consumer toolchain upgrades and private Sword media are
outside this release task.
