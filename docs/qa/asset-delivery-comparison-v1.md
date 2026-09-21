# Archived v1: compare Forge with reusable native tools

Archived on 2026-09-21 before M4 paired observations. This historical protocol
uses human time as its primary gate; new observations use [v2](asset-delivery-comparison.md).
The v1 template and earlier evidence remain unchanged. The frozen v1 decision
threshold was at least 20% median paired human task/setup savings in each project,
no more unresolved delivery failures, and total saved human time exceeding
incremental maintenance. Linked product pages may now describe v2; they do not
retroactively change this archived gate.

This protocol evaluates the [product focus](../architecture/product-focus.md).
It does not assume Forge wins. A working pipeline or fast subprocess is not a
measurement of user productivity, asset quality or willingness to adopt.

## Two different levels of evidence

The automated pilot below compares elapsed execution for already prepared PNGs.
It uses existing public showcase artwork in disposable Godot projects. It does
not modify a game, generate artwork or simulate human time. Keep all trials,
including failures. Do not extrapolate its timings to animation or recovery.

The product decision requires at least two independently maintained game projects
with three paired real iterations each. Candidates are Survival and Sword, but
no consumer pin, artwork or receipt is changed by setting up this experiment.
Run against isolated copies of the same initial state. First delivery, a revision
that changes/removes assets, and a recovery/relocation task should be represented.
Cross-platform claims require actually running on both native platforms.

## Fair task contract

Before each pair, write the user-visible acceptance independently of Forge:
expected files/resources, image dimensions/alpha, animation coordinates/timing,
stale-file removal, game loading, and recovery requirements where applicable.
Do not require Packs, a catalog or Forge receipts in the baseline. Use Git,
Godot, Aseprite or other appropriate existing tools. Both arms may preserve and
improve scripts and skills across iterations; record the initial setup and later
maintenance effort. Use the same source snapshot, machine, engine and AI model
settings. Alternate arm order and record deviations. Do not deliberately give
the baseline less context or debugging time.

Record each pair using [the observation template](artifacts/delivery-comparison-template.json).
Use a new file per real pair. Hash the sources, tools and project snapshot; record
the test contract and relevant evidence paths. Keep private media and raw agent
transcripts outside this repository. Missing measurements stay `null`.

| Measure | Collection |
| --- | --- |
| Active human seconds | Time actual setup, approvals, fixes and review separately; exclude unattended waits. |
| Agent/model cost | Record model identity, configuration, usage and actual billing where available; never infer from subprocess count. |
| Machine elapsed seconds | Include preparation through shared acceptance, with setup separately visible. |
| Correctness | Same independent native checks for both arms; failures remain failures, not zero-second successes. |
| Rework and unresolved failures | Retain failed attempts and human interventions, with cause and final state. |
| Maintenance | Count changes to baseline scripts as well as Forge-specific fixes, support and onboarding. |
| Adoption | Record whether the developer chooses to reuse the workflow in the next iteration. |

Only compare productivity after acceptance matches. Report each pair and each
project, not just a pooled average. A zero baseline human duration makes percentage
savings undefined. Apply the decision gate in the focus document; leave the
decision **insufficient evidence** until human effort, maintenance and the required
project iterations exist. Do not overwrite earlier observations after improving tools.

`activeHumanSeconds` includes task execution, review and fixes, but excludes
`setupHumanSeconds` and `maintenanceHumanSeconds`, which are separate. For each
pair calculate savings as `(baselineTask - forgeTask) / baselineTask`, where
`Task = activeHumanSeconds + setupHumanSeconds`; charge setup when it occurs,
not again on every reused run. Take the median paired saving per project. Across
the window, compare summed task-time savings with the difference between Forge
and baseline maintenance time. Allocate a shared maintenance task once and retain
its allocation rationale. Do not count maintenance twice as task fixes.

Recorded pilot: [2026-09-21 results](focus-delivery-pilot-2026-09-21.md).

## Reproduce the ready-PNG pilot

Use Python 3.10+, a verified default Forge release with `local-delivery-example`,
and a native supported Godot. No Python packages or model credentials are needed.

```sh
python3 scripts/experiments/compare-static-delivery.py \
  --forge /absolute/installed/bin/forge \
  --godot /absolute/path/to/godot \
  --output /absolute/new/experiment-directory --trials 3
```

The runner copies the same eight public PNGs into each arm's private input tree.
The native arm copies ready PNGs and invokes Godot's importer. The Forge arm uses
the selected executable's embedded example with `preserve_source`, retaining its
additional evidence. Each trial includes first delivery and an update that replaces
one image and removes one item. Both arms then run the same independent Godot
texture load/dimension/RGBA checks and installed-file inventory check.

Timings include command execution and acceptance; extraction of the runner,
installation of tools, human/agent authoring and art review are outside the timed
window and explicitly unmeasured. OS/tool caches are not cleared; arm order
alternates and both arms reuse code. This compares the complete documented Forge
example with minimal native import, not raw Rust/Python speed or equivalent safety
guarantees. Interrupted writes, rollback and a second machine are not tested here.

A negative control substitutes an incorrect reference image after the trials.
The shared native acceptance must reject it, proving that a loaded texture alone
cannot pass. Runner regression tests also reject missing completion markers,
script errors, stale files and changed PNG bytes.
All outputs are confined to a new experiment directory; sources are rehashed at
the end. The runner exits nonzero if a trial or acceptance self-check fails, while
retaining the report and logs.

## 中文执行约定

先跑可复现的 PNG 小实验，确认两组都能加载相同像素，并如实记录 Forge 的附加开销。
对照组可长期复用原生工具、脚本和 skill，不要求复制 Forge 的格式。该实验不评估人工
效率、审美、完整回滚或跨机器能力。随后在两个真实项目各记录三轮配对观察，包含首次
交付、更新和恢复/迁移。缺失的人力、模型费用或维护时间保留为 null；样本不足时不宣称
Forge 胜出。只有满足相同验收，并抵消自身维护成本后，才据此扩大投入。
