# Compare Agent + Forge with Agent + reusable tools

Protocol: **agent-delivery-v2**, adopted 2026-09-21 before real M4 pairs begin.
Agents are the direct CLI users; developers benefit from better game delivery.
The [implementation plan](../architecture/agent-first-m4-plan.md) records scope,
candidate tasks and prerequisites. This protocol does not assume Forge wins.

## Version and evidence boundary

[v1](asset-delivery-comparison-v1.md) used median human task/setup savings of 20%
as its primary gate. The user clarified that Forge serves agents; v2 changes the
primary measure prospectively to agent task/setup elapsed time, with autonomy and
correctness guards. Human effort remains visible but zero human time does not
invalidate an agent comparison. Preserve the [v1 template](artifacts/delivery-comparison-template.json)
and old observations. Do not backfill v2 measurements from subprocess timings.

The ready-PNG pilot below and M0–M3 fixtures are technical evidence, not real M4
pairs. No real v2 paired results exist at adoption. Preliminary asset inspection
is task discovery, not an arm or a baseline timing.

## Real tasks and fair comparison

Use at least two independently maintained games with three paired real iterations
each. Survival and Sword are candidates. Each iteration must have a real game
need and a frozen acceptance contract before either arm starts; three reruns or
invented revisions do not count as three tasks. Across the cohort cover delivery,
revision and recovery. Animation scope still requires a real animation delivery,
local correction and reintegration sequence; static/audio evidence cannot promote
animation support. If such demand is absent, leave that part of M4 unverified.

Use isolated copies of the same project and source snapshot; preserve consumer
pins, receipts and private media. Record tracked commit plus any included dirty
files, source hashes, native platform/engine and selected Forge binary SHA-256,
features and doctor build identity. A source build is not a released consumer pin.
Both arms use the same model/settings, permissions, acceptance and time budget.
Alternate baseline/Forge order; record cache state and deviations. Give each arm
an independent context with the same brief, not the other arm's solution. Record
unavoidable carryover as a limitation, not an independent comparison.

Define game-facing acceptance without requiring Packs, catalogs or Forge receipts
from the baseline. Native tools, existing skills and reusable scripts are allowed
in both arms, including improvements retained between iterations. Include setup
and maintenance costs; do not make the baseline start from scratch. Reuse existing
Forge guide, reports, review and installation mechanisms; build no orchestration
framework for the experiment.

Separate two streams: **fixed_source_delivery** freezes inputs and excludes their
historical generation costs; **generation_inclusive** starts from the same brief
and retains every generation attempt and its actual costs. Do not pool the two
streams or treat different generated art as a controlled processing comparison.
Record generation and Forge/local processing separately in either stream.

## Collecting an observation

Copy [the v2 template](artifacts/delivery-comparison-v2-template.json) per pair.
Freeze the protocol, task, acceptance, budget and input identity before execution.
Record status as registered before starting, in_progress during execution, and
completed or interrupted at closure; completed means observed, not accepted.
Each attempt records its ID, start/end, outcome, failure/diagnostic, correction
and evidence. Define the tool-call counting unit before execution and use it for
both arms; do not count an orchestration wrapper and its enclosed call twice.
Keep timestamped event logs and private evidence outside Git; publish only selected
sanitized summaries/hashes. Missing measurements stay `null`; an observed zero
requires an observation source. Empty arrays in the template are not evidence of
zero failures. No required field is inferred from an old narrative or call count.

| Measure | Definition and collection |
| --- | --- |
| Technical acceptance | Same structural and actual native-engine checks; original/reference pixels, coordinates, timing, audio/loop settings, resource references and recovery as required by the task. |
| Autonomous completion | Full task acceptance within the frozen budget without unplanned human rescue. Planned authorization/creative review is recorded separately. Do not remove required reviews to improve autonomy. |
| Agent task elapsed | Timestamped task window, including reasoning, documentation reads, all failed attempts, scripting, tool execution and tool/engine waits through shared technical acceptance. Exclude only separately timed external human/business waits and setup/maintenance windows. This is elapsed time, not model compute or human time. |
| Setup | Agent elapsed and human active setup time charged once when incurred, including tool/guide discovery and reusable script setup; reused runs do not pay it again. |
| Agent work | Tool calls, tool failures, recovery attempts, documentation lookups, newly authored/modified script count and retained script hashes. Keep action logs; calls and lines of code alone never establish cost or benefit. |
| Human involvement | Planned review/authorization, unplanned interventions, active task/setup/maintenance seconds and reasons. Missing timing is unknown, not zero. |
| Usage and costs | Actual model identity, tokens and billing if available, separately for generation and agent reasoning; local processing elapsed and actual metered service cost separately. Zero Forge Provider requests does not mean zero generation or reasoning cost. |
| Failure and recovery | Every failed/interrupted attempt, cause, diagnostic locator, correction, reused outputs and final state. Correct rejection of a missing source is safe behavior but not successful resource delivery. |
| Maintenance and reuse | Agent elapsed and human active time, change hashes, beneficiary tasks and allocation rationale; include Forge fixes and baseline scripts. Record the next real task's actual reuse choice and reason. |
| Creative and rights review | Structural, native loading, visual, auditory and license statuses separately, with reviewer/evidence; unperformed reviews remain not_assessed. |

Use non-overlapping setup/task/maintenance windows within an arm. Nested tool
spans annotate their parent; never sum them into elapsed twice. Log interruptions
and external waits with reasons. Keep both raw wall time and the excluded seconds;
`agentTaskElapsedSeconds = taskWallSeconds - externalWaitSeconds`. Setup and
maintenance are outside that task window. Missing timestamps make time savings
unmeasured. Agent visual/diagnostic work remains inside the task; external human
review waits are excluded but disclosed. A required creative rejection is a task
failure even if native validation passes. `technicalAcceptancePassed` records
structural/native requirements; `firstAttemptAcceptancePassed` and
`finalAcceptancePassed` cover the entire frozen acceptance contract, including
required creative/rights review. Autonomous completion requires that full final
acceptance; unperformed required review cannot be scored as a success.

## Prospective decision gate

Evaluate each project and stream separately using all registered pairs. Frozen
budgets give unsuccessful arms a failure/censored duration, never a zero-second
success. Show first-attempt and final technical acceptance rates plus autonomous
completion rates over **all started tasks**, including interruptions. Use a
conservative small-sample decision, not a statistical population claim.

For each pair where both arms complete the required acceptance within budget:
`Task = agentTaskElapsedSeconds + setupAgentElapsedSeconds` and
`saving = (baselineTask - forgeTask) / baselineTask`. A zero baseline Task makes
percentage savings undefined. Do not replace it with a different winning metric.

The initial v2 gate for further investment in the observed workflow is:

1. At least three real pairs per project, with both arms accepted in at least
   three pairs per project in the same stream for the timing median. All other
   started pairs remain in failure/autonomy counts; disclose success-only timing.
2. Forge final technical and autonomous completion rates are no lower, with no
   additional unresolved delivery failures, no additional unplanned human rescue
   events and no source/history/pin corruption. First-attempt regressions must be
   explained; do not hide extra rework behind final success.
3. Median paired Task saving is at least **20% in each project**. This is an
   initial decision threshold, not a result. No switching to tool-call counts,
   another stream or a selected subset after seeing outcomes.
4. Across all started tasks, report actual task/setup effort including failed and
   interrupted runs. Net observed agent elapsed savings must exceed incremental
   Forge maintenance agent elapsed; allocate shared maintenance once with a
   rationale. Unknown/censored effort prevents a net-effort claim. Human
   maintenance is reported separately and assessed explicitly, not converted
   into agent seconds or silently treated as free.

Missing monetary data prevents a cost-saving claim, not a measured time claim.
Missing primary timing, autonomy, acceptance or maintenance evidence leaves the
investment gate **insufficient_evidence**. Unmeasured human effort limits the
human/total-cost conclusion explicitly. Passing technical checks alone never
approves art, listening quality, license or general animation production.

- **Continue** only the workflow supported by the gate, with limitations and
  actual reuse evidence; expand only after repeated need.
- **Narrow** to beneficial tasks if native tools already cover simple delivery or
  other workflows lose on setup/maintenance. Keep compatibility.
- **Defer** if demand, inputs, authority or primary evidence is missing. A changed
  gate requires a new version and a new observation cohort, retaining old results.

Recorded historical pilot: [2026-09-21 results](focus-delivery-pilot-2026-09-21.md).

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

Agent 是直接使用者。v2 在真实配对观察前替代 v1 的人工主门槛，以 Agent 任务及设置
耗时的项目内配对节省中位数至少 20% 为初始效率门槛，并要求自主完成率、技术成功率、
未解决失败及非预期人工介入不退化；净节省须覆盖额外 Agent 维护投入。人力、费用、
返工、脚本和实际复用单独报告，不混算单位，不挑有利指标替代主指标。

两个独立项目各至少三轮真实需求，两组允许复用现有工具。固定素材交付与包含生成的
完整任务分开测量，保留所有失败。正确拒绝缺失原件不等于资源交付成功。素材修复、
安装、迁移都在隔离副本进行，源文件、历史审核与消费项目锁保留。
技术、视觉、试听及许可分别验收；数据缺失保留 null。历史 PNG 实验不能转写为 v2
实测。没有真实动画任务时保留动画验收缺口，不用图标或音频观察代替。
