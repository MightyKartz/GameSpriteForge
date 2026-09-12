# PR 6 portability acceptance — work in progress

Scope: selected resource directory bundles, machine-local root rebinding,
integrity/evidence/connected-installation audits, disposable metadata index and
explicit three-way catalog reconciliation. This PR also completes purpose,
candidate/selected/discarded state, user-declared origin and parent/variant fields,
and the embedded `guide project-assets` resource. No consumers are upgraded.

## Local evidence so far

- Seven new Rust integration tests pass: source distinctions, selected media and
  evidence/consumer-lock transfer, tampered/duplicate inventory rejection, local
  rebinding, cache corruption/deletion, parallel Git history, selection conflicts
  and conflicting concurrent review assertions.
- Existing library (15), review (2), game-art build (10) and CLI (30) tests pass.
- Strict default-plus-game-art Clippy passes. Embedded guide checks pass with
  10 resources. Clean source/build identity will be recorded after committing.
- `test-asset-library-portability-cli.py` completed on macOS using the actual
  default CLI and Godot 4.6.3: synthetic PNG/GIF/WAV and a migrated legacy Pack,
  retained media and evidence, exact consumer lock, original Job store removal,
  transfer into a different root, whole-Pack native installation and connected
  installation audit. Provider request count is zero. This is same-system evidence.
- A legacy v2 Pack hash is preserved in its original immutable revision; the
  bundle records and verifies its POSIX/Windows hash convention against actual
  media, independently of the canonical content identity.

## Required gates still pending

- CI producer/consumer jobs actually exchange the same bundle between macOS and
  Windows in both directions. Results are not yet available for this PR.
- Installed Windows launcher and historical-executable upgrade are still pending.
- PR 5's actual offline browser display/playback remains a manual acceptance
  gate: the browser tool rejected `file://` access. Byte/timing and HTML tests
  do not claim browser playback. The user has been asked to check the generated
  local synthetic preview; no answer has been received yet.

## Installed macOS package and Sword sandbox

The macOS package at source `ba597b4284e8dda5644b6b046b6a116008b152b9`
passed `test-cli-release-artifact.sh`: fresh install, same-package reinstall,
actual v0.3.1-to-development-v0.3.2 upgrade, embedded skill/guide, old CLI rejection
of V3, resource intake/review/transfer, real Godot installation, existing product,
static and animation contracts. Calls used the public installed symlink; external
FFmpeg search was disabled and helpers were resolved inside the payload.
The archive was assembled locally with the verified v0.3.1 release's unchanged
LGPL helper/license files, the current clean default release CLI and new manifest;
it was not published as a release.

- Current binary SHA-256: `fd8b74da04dc65eecc1adfe7cbbb03dbcbd239f11c280d7ad54d45db3bc67506`.
- Current archive SHA-256: `5fba028e34b4f16159e7e9c362f8783b43e14e39595cbada74696ee78e7b376f`.
- Downloaded historical v0.3.1 archive SHA-256, checked against its release sidecar:
  `9cd68880717db49c15a489f113329cc12eeec72f8ab04c44e0bd42ba2ad03845`.
- Build: clean, `features:[]`, `profile:release`, `aarch64-apple-darwin`.
- Local detailed log: `/tmp/forge-library-package-acceptance.log`.

Sword was inspected read-only: 82 human catalog entries, four installation
entries without independently registered snapshots, 18 audio cues and five
versioned music manifests. Six representative resources (PNG, WAV and four
animation Packs) were copied into an independent temporary library. Registration,
retention, index rebuild and one real Godot animation installation passed with
zero Provider requests. Eleven original metadata/lock files and all six selected
original resources retained their byte hashes; Forge/Godot consumer pins were not
changed. Historical installation statements were retained as original notes with
explicitly absent snapshot evidence, rather than fabricated installation receipts.

Sword sandbox CLI identity: clean default debug build at
`4cc91b1a4ad2557c04fcd6fc975ad4c6b6f2453f`, binary SHA-256
`b441e38941f98f8bd0374a61487238197d85b1fe27e67c01cf1f6f4451c2795a`.
Only aggregate results are included here; private media and per-resource notes
remain outside the public checkout.

A subsequent focused fix makes concurrent review assertions conflict explicitly
and rejects empty purposes/duplicate parent references. Its seven-test suite passes;
the complete installed macOS package sequence above was repeated successfully on
the clean default release at `13c5518265213fca6433b7ae58eafb998205dd3e`.
Final local binary SHA-256:
`ab53d4e8fb19057d17ab8731b530a158017d0226f6b077663cd21bb9e36c75ad`.
Final local archive SHA-256:
`b6d2bfad8da43a7f31e965de2edf6f7f62e564dcbcfc902856805e48401bd06d`.
Detailed final log: `/tmp/forge-library-package-final.log`.

All seven draft PRs are open in order:
[#34](https://github.com/MightyKartz/GameSpriteForge/pull/34),
[#35](https://github.com/MightyKartz/GameSpriteForge/pull/35),
[#36](https://github.com/MightyKartz/GameSpriteForge/pull/36),
[#37](https://github.com/MightyKartz/GameSpriteForge/pull/37),
[#38](https://github.com/MightyKartz/GameSpriteForge/pull/38),
[#39](https://github.com/MightyKartz/GameSpriteForge/pull/39),
[#40](https://github.com/MightyKartz/GameSpriteForge/pull/40).
PRs #34–#39 passed their remote matrices. PR #40's latest check runs and artifact
links are the source for remaining remote acceptance; it remains a draft until
those results and the manual browser gate are resolved.

## Verification boundaries

Exported shared records exclude local root mappings, caches and unrelated history.
Selected Pack bytes and immutable provenance remain unchanged; arbitrary user
notes and original Pack metadata are not automatically scrubbed. Review them
before sharing. Parent revisions outside the selection are reported as external
lineage, without claiming retained source media or reproducibility.

`verify-asset-bundle` without an externally supplied manifest digest establishes
self-consistency only. Import requires that digest and a new destination. Neither
resource retention nor an exported receipt alone proves source regeneration.
Installation audits cover explicitly connected registry entries and applicable
native file/cache baselines; they do not discover dynamic or global runtime use.
