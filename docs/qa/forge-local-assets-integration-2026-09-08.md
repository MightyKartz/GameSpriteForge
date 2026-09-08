# Local asset integration and release — 2026-09-08

Goal: deliver the verified local static and animation workflows in a distinct
Forge release, then migrate Sword after isolated replay. Source art and the old
consumer binaries are retained. Real Provider generation is outside this work.

| Batch | Scope | Status |
| --- | --- | --- |
| 1 | Local static intake, rendering/anchors, alpha bounds, skills/docs and CI | Merged through PR #20; GitHub quality matrix passed |
| 2 | Preserved animation coordinates/timing and whole-sheet preprocessing | Merged through PR #21; GitHub quality matrix passed |
| 3 | Build/capability identity, v0.3.0 RC and final package verification | v0.3.0 published; downloaded stable package and normal installation verified |
| 4 | Isolated consumer replay, Sword lock migration and rollback evidence | Stable replay and formal migration passed; local consumer commit is clean |

## Batch 1

Integrates the documentation commit `a7bfa0f` and static commits `fa58d64`,
`1f9d309`, `c1f4480`. The static CLI smoke now uses its `--godot` executable
for both installation Jobs and final resource checks. The PR quality matrix and
release workflow explicitly exercise local static delivery, including the ignored
real-engine test. At this batch's merge, usage docs distinguished source
capabilities from published v0.2.1; the stable release updated their availability.

Combined-source verification: 249 Rust tests passed, 0 failed; the
1 normally ignored Godot integration test was run explicitly and passed.
`cargo fmt`, warning-free workspace Clippy, default CLI build, three local static
CLI/Godot cases, and the signing contract passed. Skill validation, shell syntax
and whitespace checks passed. The initial Clippy attempt ran out of disk space;
only rebuildable Forge Rust caches were removed, both locked consumer executable
hashes remained unchanged, and the rerun passed. See
[the static summary](artifacts/forge-local-assets-integration-20260908/static.json).

## Integration decisions

- The new public local intake API warrants a minor version: v0.3.0, with an RC
  before stable publication. Optional world/character features stay source-gated.
- Animation code is reviewed and integrated separately. Its historical private
  source reports are not required as public CI fixtures.
- Sword's locks change only after the actual release executable reproduces
  its installed pixels, frame coordinates/timing and rendering contracts.
- Keep earlier audit reports immutable; update usage guides as availability changes.

## Batch 2

Integrates implementation commits `9efc15b` and `d4b18e2` without copying the
intermediate private source/terminal QA reports. Review found that automatic
repair could change `preserve_source` back to per-frame bottom alignment or add
invalid margins. The focused fix `556b33a` keeps those recommendations as manual
actions while allowing unrelated supported repairs. Single-action and character
regressions cover both plan analysis and preparation.

The CLI/Godot smoke now has a small synthetic whole-sheet padding/offset case,
exact source/derived/frame pixel checks, and alpha=1 loss rejection. It keeps the
legacy and explicit rendering/timing cases and needs no private Sword artwork.
CI pins Python/Pillow for these fixtures. Both the source release checks and the
actual packaged-install checks exercise static and animation delivery.

Combined-source verification passed: 260 workspace tests, zero failures,
with one separately exercised batch-1 static engine test ignored by default.
This includes the single/character repair regressions. Formatting, warning-free
workspace Clippy, default CLI build, five animation CLI/Godot cases, three static
CLI/Godot cases and shell syntax passed. See
[the animation summary](artifacts/forge-local-assets-integration-20260908/animation.json).

## Batch 3

Adds compiled build identity and stable capabilities to doctor, with null values
for unknown Git identity. Tests cover worktrees, source archives, dirty source,
feature changes and incremental rebuild behavior. Default-marker filtering keeps
the release's optional feature list empty. Release verification compares the
executable with its payload metadata and verifies bundled FFmpeg/FFprobe selection.
The first candidate was v0.3.0-rc.1; its package-gate result and replacement
candidate are recorded below.

Local build-identity validation passed: five CLI tests, formatting, warning-free
workspace Clippy, default CLI build, installer contract and shell syntax. The
development doctor correctly reported a dirty debug build and all seven capability
IDs; [the identity summary](artifacts/forge-local-assets-integration-20260908/build-identity.json)
distinguishes this evidence from a release payload check.

Independent release-check review removed a test-only payload PATH injection.
The RC.1 package gate also exposed premature symlink resolution in the verifier,
as detailed below. Current installed checks invoke the ordinary public executable
link and require both reported helper paths to belong to its versioned payload.

### RC.1 package gate and RC.2 correction

PR #22's eight source quality gates passed, including 265 Rust tests. The
`v0.3.0-rc.1` tag points to `c0f70c57fcdc6be58d6a0f133ba9fdb20062859d`.
Its [release run](https://github.com/MightyKartz/GameSpriteForge/actions/runs/34180596991)
stopped before publication at the actual installed product contract. No GitHub
Release or downloadable RC.1 package was published; the tag is retained unchanged.

On macOS, `current_exe()` can retain the installer's public symlink. Taking its
parent therefore missed the actual payload's FFmpeg/FFprobe. The identity verifier
had resolved that symlink before launch, concealing the same entry-point behavior.
The runtime now resolves the executable before locating its helpers, and installed
verification invokes the public link unchanged with external helper search disabled.

The failure was reproduced locally: a direct payload launch passed the product
contract; its public link reported `ffmpeg_missing`. After the fix, relative and
chained launcher tests passed, a missing helper was correctly rejected, and the
full fixture product/Godot contract passed through the public link with no external
FFmpeg search. Formatting, warning-free workspace Clippy and shell checks passed.
This local fixture is separate from release-package evidence. RC.2 will be built
and verified from a new tag; stable publication and consumer migration remain gated
on the actual RC.2 package and isolated replay.

### RC.2 actual release and consumer gate

PR #24's eight source gates passed, including 267 Rust tests. The new
[`v0.3.0-rc.2` release](https://github.com/MightyKartz/GameSpriteForge/releases/tag/v0.3.0-rc.2)
was built from `87d453ba7bbbfd004ad8fe5ae9e0c614532191ef` and its
[release workflow](https://github.com/MightyKartz/GameSpriteForge/actions/runs/34183302486)
passed the actual packaged-install gate before publication.

The published archive was downloaded independently. Verification matched local
and remote tag commits, archive SHA-256, compiled clean release identity, and
payload `BUILD_INFO.json`. The downloaded installer matched the tagged source;
its ordinary HTTPS installation succeeded. The downloaded CycloneDX SBOM matched
the predicate in the verified GitHub attestation for that archive, tag and commit.
Fresh installation, reinstall, v0.2.1 upgrade, fixture product, native Godot,
local static and local animation contracts all passed with external FFmpeg
discovery disabled for the complete artifact check.

An isolated consumer replay used the actual RC.2 executable and the consumer's
existing three-entry registry. All 11 Packs and final registry bindings passed;
the original registered entries advanced correctly and the remaining eight were
registered. The 10 static items, nine animation actions / 42 frames and 19 PNGs
matched the retained old-tool baseline in native runtime semantics, complete
decoded RGBA and metadata. All 137 consumer tests / 8,064 assertions passed,
with zero failures or skipped tests. The 21 source/provenance image hashes and
both old binary hashes remained unchanged; local imports used zero Provider
requests. This validates delivery compatibility, not a new human art review.

Consumer development advanced during the audit. The additional scenery inputs
were first checked against an old-tool replay; later game-only changes retained
the same Forge inputs and resources. The RC.2 game checks used the latest clean
consumer baseline, rather than reporting the earlier game's smaller test count.
Large source media, private project files and detailed local stores remain local.

### Stable release package

PR #23's final head passed all eight source gates and 267 Rust tests. Its merged
tree was checked before tagging
[`v0.3.0`](https://github.com/MightyKartz/GameSpriteForge/releases/tag/v0.3.0)
at `60904c02a52f6831626ebbc4475f9c25db162931`. The stable version was built
independently; its [release workflow](https://github.com/MightyKartz/GameSpriteForge/actions/runs/34184929974)
passed before publication.

The downloaded stable archive passed the same independent tag, checksum,
attestation/SBOM, HTTPS installer, build identity and complete installed-package
contracts as RC.2. Its executable SHA-256 is
`e3693d317bd61937f0a0e1aa66306a88a9065dd3b47b35c62e2b07064149b912`.
The regular installer selected v0.3.0 as GitHub's latest stable release and
installed the same bytes into the ordinary versioned payload. Its public launcher
verified successfully with external helper discovery disabled, and the existing
shell profile remained unchanged. See [release evidence](artifacts/forge-local-assets-integration-20260908/release.json).

## Batch 4 — completed consumer migration

During stable preparation the consumer added a six-frame lightning effect. Its
source and single-action old-tool replay were checked independently; the other
11 Packs retained their earlier file hashes and proof. The resulting current
baseline contains 12 Packs, 10 static items, 10 animation actions / 48 frames
and 20 PNGs. This additional scope was included in the stable replay.

The stable executable from its final normal installation path replayed all
12 Packs successfully: 24 Jobs succeeded, zero Provider requests, and the
three-entry registry updated to 12 correct entries and revisions. Native runtime
semantics, complete decoded RGBA and metadata all strictly matched the old-tool
baseline. All 145 game tests / 8,124 assertions passed with zero failures or
skips. The comparison capture SHA-256 was identical on both sides:
`fda1cd641e753579fd8c4f44d393f5d6dce199a70ee0e3421549c815ef336bc3`.

Only then were the formal consumer's future-import locks migrated to the same
versioned v0.3.0 payload and its verified SHA/build/capabilities. The static
wrapper now reads the lock's default path, preserving explicit overrides, and
the prototype importer names its existing target explicitly. Both changes and
the exact final locks were exercised in the stable replay. Original lock bytes
and old executable paths remain available for rollback.

The formal project then passed its own 145 tests / 8,124 assertions and default
wrapper identity check. Hash guards confirmed 225 protected existing files,
its original registry, all 22 source/provenance images and both old executables
were unchanged. Existing runtime resources and historical receipts/catalog
producer records were retained; no formal re-import was performed. A focused
local consumer commit contains the locks, two tools, rollback files and review/QA
documentation. The consumer working tree is clean and was not uploaded, following
its repository rules. See [aggregate consumer evidence](artifacts/forge-local-assets-integration-20260908/consumer.json).

The final development skill records the public-launcher and bundled-helper
verification requirements learned from RC.1. Usage docs and both README languages
describe the published static workflow; animation remains in development/testing.
