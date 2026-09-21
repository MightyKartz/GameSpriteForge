# Product focus and ready-PNG delivery pilot

Date: 2026-09-21. [Machine-readable evidence](artifacts/focus-delivery-pilot-2026-09-21.json).
Protocol: [asset delivery comparison](asset-delivery-comparison.md).

## What changed

Product/README guidance now focuses investment on repeatable processing, owned
Godot updates and recovery. Existing commands/formats remain supported. General
generation/agent-platform expansion and promotion of optional character/world
workflows are paused pending demonstrated consumer benefits.

The embedded guide now starts from the user's task, loads only the needed reference,
keeps libraries and Providers optional, and has the agent manage technical records.
It preserves pinning, source review, execution checks and installation authority.
The complete delivery example is appropriate only when its recipe/delivery is
already authorized; it does not pause to approve a newly processed result.

## Observed result

Three trials alternated arm order. Each delivered the same eight existing public
384×512 PNGs, then updated one item and removed another. Both arms passed the same
installed PNG inventory/byte checks and real Godot texture load, dimensions,
alpha and visible RGB checks. A wrong-reference negative control was rejected.
Original source files retained their hashes. All 12 measured attempts passed.

| Task | Native copy + Godot import, median | Forge complete delivery example, median |
| --- | ---: | ---: |
| First delivery | 3.23 s | 9.19 s |
| Replace one item and remove another | 2.94 s | 9.08 s |

On this machine and narrow task, the complete Forge path took approximately
2.8–3.1 times the elapsed execution time. It also performs extra planning,
validation and evidence retention that the minimal native arm does not provide.
These timings are not an isolated algorithm comparison or equivalent transactional
safety test. The preliminary run (before strengthening stale-file/stdout-error rejection)
is also retained in full, including its 15.16 s first Forge attempt. Its first/update
medians were native 3.53/3.82 s and Forge 10.83/10.93 s. The table uses the final
runner; neither run removes warm-up observations. OS caches were not cleared.

The immediate product action is to recommend native import for small batches of
already usable PNGs. This experiment does not justify removing Forge's recovery,
animation or processing contracts, nor does it establish a productivity win.

## Identity and validation

- Pilot executable: installed **v0.6.4**, clean release `6fd33cf`, default features,
  macOS ARM64. SHA-256 and full commit are in the evidence. It used its own embedded
  delivery example, not the checkout's rewritten guide. Godot **4.7.2** ran natively.
- The rewritten guide was separately rebuilt from `6fd33cf` plus this working
  change, recorded as `dirty:true`, debug/default features. All **155 CLI calls**
  in the full guide/install/update/protection suite passed. Its binary and bundle
  hashes are separate from the released pilot executable.
- Six comparison regression tests reject absent completion, script errors, wrong
  counts, stale/changed files, and incomplete or failed comparison pairs. Both skill
  validators, Rust formatting, changed Markdown links, JSON and workflow YAML pass.
- No Provider requests, consumer writes, consumer pin changes, private-media
  publication or package/release updates. Disposable experiments live outside Git.

## Unmeasured work

Active human time, model cost, script-authoring effort and maintenance are **null**.
No interviews, real game iterations, rollback comparison or native Windows pilot
ran here. The new guide's clarity is not claimed to have passed a user study.
The two-project/three-iteration product gate remains **insufficient evidence**.
Use the observation template during subsequent real tasks; retain failures and
allow both arms to reuse scripts before deciding whether to expand investment.

## 中文结论

三轮配对实验的 12 次首次/更新操作均通过相同 Godot 原生验收。现成 PNG 的直接导入
中位耗时为 3.23 / 2.94 秒，Forge 完整证据流程为 9.19 / 9.08 秒。因此指南明确允许
简单任务使用原生导入。额外时间包含计划、校验和回执，不据此断言这些保证没有价值。
人工耗时、模型费用、维护成本及真实项目收益尚未测量，扩大投入的门槛仍未满足。
