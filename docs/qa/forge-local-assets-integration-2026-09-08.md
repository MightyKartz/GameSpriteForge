# Local asset integration and release — 2026-09-08

Goal: deliver the verified local static and animation workflows in a distinct
Forge release, then migrate Sword after isolated replay. Source art and the old
consumer binaries are retained. Real Provider generation is outside this work.

| Batch | Scope | Status |
| --- | --- | --- |
| 1 | Local static intake, rendering/anchors, alpha bounds, skills/docs and CI | Merged through PR #20; GitHub quality matrix passed |
| 2 | Preserved animation coordinates/timing and whole-sheet preprocessing | Merged through PR #21; GitHub quality matrix passed |
| 3 | Build/capability identity, v0.3.0 RC and final package verification | Release candidate preparation |
| 4 | Isolated consumer replay, Sword lock migration and rollback evidence | Pending |

## Batch 1

Integrates the documentation commit `a7bfa0f` and static commits `fa58d64`,
`1f9d309`, `c1f4480`. The static CLI smoke now uses its `--godot` executable
for both installation Jobs and final resource checks. The PR quality matrix and
release workflow explicitly exercise local static delivery, including the ignored
real-engine test. Current usage docs distinguish source capabilities from the
older published v0.2.1.

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
- Sword's locks remain unchanged until the actual release executable reproduces
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
The first candidate is v0.3.0-rc.1; publication and installed replay remain pending.

Local build-identity validation passed: five CLI tests, formatting, warning-free
workspace Clippy, default CLI build, installer contract and shell syntax. The
development doctor correctly reported a dirty debug build and all seven capability
IDs; [the identity summary](artifacts/forge-local-assets-integration-20260908/build-identity.json)
distinguishes this evidence from a release payload check.

Independent release-check review removed a test-only payload PATH injection.
Installed doctor now has to discover its sibling FFmpeg/FFprobe through the same
public executable link used by an ordinary installation; the verifier still
requires both reported paths to point inside that exact versioned payload.
