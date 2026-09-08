# Local asset integration and release — 2026-09-08

Goal: deliver the verified local static and animation workflows in a distinct
Forge release, then migrate Sword after isolated replay. Source art and the old
consumer binaries are retained. Real Provider generation is outside this work.

| Batch | Scope | Status |
| --- | --- | --- |
| 1 | Local static intake, rendering/anchors, alpha bounds, skills/docs and CI | Local checks passed; GitHub checks pending |
| 2 | Preserved animation coordinates/timing and whole-sheet preprocessing | Review in progress |
| 3 | Build/capability identity, v0.3.0 RC and final package verification | Pending |
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
