# Forge v0.3 test matrix

The v0.3 release matrix keeps Character V2 as the new commercial-quality gate while preventing regressions in the already public Icon Set and Prop Set workflows. Terrain, Building Kit, and JSON Map remain compile-gated experiments, but their V3 Pack and Godot delivery paths must continue to pass.

Run the complete matrix from the repository root:

```bash
bash scripts/test-v03-release-matrix.sh
```

Set `FORGE_V03_REPORT_DIR` to retain logs and machine-readable evidence at a specific location. By default the script writes a timestamped report under `target/qa/`.

The matrix contains six gates:

1. Rust formatting, default-workspace Clippy/tests, and combined `consistency-v2,world-assets` CLI compilation.
2. Subject Lock, 32-keyframe generation, single-frame retry, zero-cost replay, and benchmark protocol contracts.
3. The full frozen 20-character / 5-style Character comparison plus a cold Godot calibration.
4. Public v0.2 CLI regression for Character, Icon Set, Prop Set, async jobs, review, Pack validation, and Godot.
5. Five Style revisions producing five Icon Packs and five Prop Packs: 50 items total, all consistency-gated, Pack-validated, and installed into Godot.
6. Experimental Environment, Terrain, Building, and JSON-only Map generation through V3 Packs and a headless Godot world load.

Release-blocking gates are separate from the experimental world gate in `v0.3-test-report.json`. A world failure blocks merging the current implementation because it signals a regression, but it does not make world assets part of the v0.3 public CLI or commercial claim.

Fixture results establish deterministic workflow correctness, not real-model visual quality. Existing xAI evidence remains in the static and world acceptance reports; new commercial claims require a separately approved real-provider run.

## Latest execution

The complete matrix ran on 2026-08-04 against commit `7be5fa4711596e62db017964bc190384907e4bf9` and passed all six gates in 449 seconds. The run produced 40/40 valid Character Packs, 5/5 Icon Packs, 5/5 Prop Packs, and successful Terrain, Building, JSON Map, and Godot world validation. The compact machine-readable evidence is `docs/qa/forge-v03-release-matrix-2026-08-04.json`.

The same matrix runs in `.github/workflows/v03-quality.yml` for v0.3 pull requests. CI downloads the pinned Godot 4.6.3 build, passes its executable through `FORGE_GODOT_PATH`, and uploads the JSON report and individual gate logs even when a gate fails.

The environment-provided Godot path was separately verified with a nonstandard executable name against Character, the five-style static matrix, and the complete world installation path. This prevents the CI test from accidentally succeeding only because `/Applications/Godot.app` exists on a developer machine.
