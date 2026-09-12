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

- Latest-head remote verification is tracked in PR #40. Earlier remote builds
  passed actual bidirectional exchange and Windows installed/historical upgrade
  checks; their exact source identities are recorded below rather than being
  attributed to a later commit.
- PR 5's actual offline browser display/playback remains a manual acceptance
  gate: the browser tool rejected `file://` access. Byte/timing and HTML tests
  do not claim browser playback. The user has been asked to check the generated
  local synthetic preview. At the time of this QA record no answer had been
  received; the current manual gate status is tracked in PR #40's description.

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

The completion audit re-ran all five real-Godot transaction tests against the
final runtime code: native update failure/rollback, provider-neutral V3 install,
animation failure protocol, texture/cache restoration and retained A/B/A delivery
all passed (`/tmp/forge-library-final-rollback.log`). It also identified a missing
Windows-specific test: Unix symlink coverage did not prove junction behavior.
`windows_junctions_are_rejected_by_inventory_scan_and_catalog_writes` now creates
real junctions with paths containing spaces and checks inventory, scan and catalog
write rejection without changing the target bytes. Its actual Windows result is
part of the latest PR check run; the local macOS tests do not claim that result.

A final compatibility check reproduced an error in the legacy catalog projection:
an imported available Pack could be blocked by an unbound historical installation
root. The fix prefers retained/alternate media and treats optional unavailable
spec/install projections as absent, preserving the complete native history. The
same previously exported bundle failed `asset list` on the recorded pre-fix binary
and passed list/hash checks plus real Godot installation after the fix. A dedicated
Rust regression test and both CI transfer directions now cover this case. This
change was verified again through the full installed-package sequence on clean
default release source `18f508e82fc477100ae21c32076a363373c6ce16`:

- Binary SHA-256: `51c66c8b91a939dcba975a4c0d7e83d6afb84b98e22f4a9cc2febe63ccb11e87`.
- Archive SHA-256: `d831fe80f12503dc05a8465161786a0387f1e8cabb732f6f4d7a9183350e30cd`.
- Local complete log: `/tmp/forge-library-package-18f508e.log`.
- Library regression suite now contains 16 tests; transfer/merge/index contains seven.

The same installed macOS launcher then consumed the actual Windows CI-produced
bundle, with `--require-foreign`, unchanged manifest digest
`dc03d426c9c89b1e8ec4a94e5adb371da06d1051790745e97ef4f7ac3dd29f3f`,
and preserved Windows v2 Pack hash
`f2c1e3ce9b18b53d23e419851fb2ec8fc8badda8fad445e39b9ea4363d7cb777`.
Legacy list, content/review/lock checks and real Godot installation passed with
zero Provider requests. This is actual Windows-to-macOS media transfer, not a
same-machine directory rename.

The first full [bidirectional CI run](https://github.com/MightyKartz/GameSpriteForge/actions/runs/34708988785)
passed on PR head `4cc91b1a4ad2557c04fcd6fc975ad4c6b6f2453f`, with clean default
debug executables built from GitHub's merge commit
`93b08301d9700baec49dc6fc7f5e85c30bc7d672` on both operating systems.
Windows-to-macOS used the manifest digest above; macOS-to-Windows used
`78ca456b45397153d6a5702541cda598f29beed8ec8b0f2451f31de9c8deed77`.
Both asserted different producer/consumer operating systems and preserved the
same revision IDs, media inventories, consumer lock and historical hash mapping.

The [Windows installed-package and historical-upgrade run](https://github.com/MightyKartz/GameSpriteForge/actions/runs/34709138612)
passed for clean default release source `ba597b4284e8dda5644b6b046b6a116008b152b9`:
current binary `d4bec7e707234e316c2a63230b0c9b5c5e53d6a91ada0426ec13abf093753c32`,
archive `fd6b6fdee68041057320438cafaff367378ad8565b0e87151e122b340b45c595`.
It used an actual separately compiled pre-library executable from
`1c1e7079f22ea0dba5958e4aec79cad69f7ade65`, binary
`c94f566aecc56b2b6c7169cbea60b98856dc08732fc1856f69bdeaa43629b425`;
the test explicitly required different binary hashes and preserved the original
payload and launcher backup. This is a historical source-build Windows upgrade,
not a claim that the historical binary was distributed as an official Windows
release. The independent same-executable packaging-revision test remains intact.

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
