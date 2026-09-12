# PR 6 portability acceptance — work in progress

Scope: selected resource directory bundles, machine-local root rebinding,
integrity/evidence/connected-installation audits, disposable metadata index and
explicit three-way catalog reconciliation. This PR also completes purpose,
candidate/selected/discarded state, user-declared origin and parent/variant fields,
and the embedded `guide project-assets` resource. No consumers are upgraded.

## Local evidence so far

- Six new Rust integration tests pass: source distinctions, selected media and
  evidence/consumer-lock transfer, tampered/duplicate inventory rejection, local
  rebinding, cache corruption/deletion, parallel Git history and selection conflicts.
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
- Installed macOS launcher, historical-release upgrade, and installed Windows
  launcher acceptance are still pending for the final build.
- Sword read-only inventory and isolated sandbox acceptance are pending.
- PR 5's actual offline browser display/playback remains a manual acceptance
  gate: the browser tool rejected `file://` access. Byte/timing and HTML tests
  do not claim browser playback. The user has been asked to check the generated
  local synthetic preview; no answer has been received yet.

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
