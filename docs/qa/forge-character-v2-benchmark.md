# Forge Character V2 benchmark

Profile: `character-v2-benchmark@1.0.0`

The frozen normal-quality corpus is `benchmarks/character-v2/frozen-20x5.json`. It contains 20 original characters across five visual styles. It intentionally contains no fabricated hard-defect labels; the hard-defect interception gate therefore remains `not_evaluated` until a separately labeled or deterministically injected defect corpus is supplied.

Run the offline contract:

```bash
bash scripts/test-character-v2-benchmark.sh
```

Inspect the real Provider request envelope before spending:

```bash
cargo run -p forge-cli --features consistency-v2 -- \
  benchmark plan \
  --manifest benchmarks/character-v2/frozen-20x5.json \
  --provider xai --profile default --json
```

For the frozen 20×5 comparison, Forge currently estimates:

- Shared Style and Subject Locks: 25 requests.
- Video baseline: 180 normal, 260 maximum requests.
- Keyframe workflow: 640 normal, 1,280 maximum requests.
- Both workflows, sharing setup: 845 normal, 1,565 maximum requests.

The benchmark summary recommends making keyframes the default only when all gates pass: 20 characters and five styles, keyframe Pack success at least 90%, hard-defect interception 100%, zero erroneous exports, at least ten percentage points of identity improvement over video, and median keyframe requests no more than 40 per Pack.

The contract test uses deterministic fixture results to verify gate mathematics and unsafe-reference rejection. It also executes `fixture-hard-defect.json`, which deterministically produces multiple major subjects and proves that Forge detects the hard defect without exporting a Pack. It does not claim real-model quality. Real xAI results must be stored as `CharacterBenchmarkRunV1` evidence and summarized with `forge benchmark summarize`.

`forge benchmark run-character` executes the real Style Lock → Subject Lock → Character Pack pipeline with one Provider instance, writes progress after every case, validates each exported Pack, and optionally installs each result into an isolated Godot target. Use `--limit 1 --workflow keyframes --skip-godot` for a low-cost calibration; omit `--limit` and `--skip-godot` only for the formal frozen run.

The frozen fixture baseline is recorded in `docs/qa/forge-character-v2-fixture-baseline-2026-08-03.json`. It produced 40/40 valid, game-ready Packs and the expected cold request counts: nine for every video Pack and 32 for every keyframe Pack. Fixture identity improvement is deliberately not a release-quality signal and remains a failing gate; a real xAI 20×5 run is required to evaluate that claim.
