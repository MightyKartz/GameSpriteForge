# Forge `topdown-video-cycle@6.0.0` implementation plan

Status: implemented and under offline acceptance; real Provider generation remains separately authorized.

Date: 2026-08-09

## Decision

V6 keeps xAI image-to-video and native Godot `SpriteFrames`, but removes four
contracts that made V5 unreliable:

- a 256 px delivery sprite is no longer the Provider generation master;
- a geometrically closed interval is no longer sufficient to prove a walk;
- source-video cadence no longer silently becomes gameplay cadence;
- debug previews no longer use timing different from Pack and Godot output.

V5 remains immutable and readable. V6 is a new experimental workflow and does
not authorize a real Provider run.

## Workflow

```text
SubjectLock + StyleLock
→ DirectionLock generation master and 256 px delivery derivative
→ 720p image-to-video with repeated constant-cadence cycles
→ ≤12 FPS full-clip candidates
→ matting and provisional alignment
→ gait-cycle@1.0.0 semantic cycle selection
→ eight phase samples without a duplicate boundary
→ playback-cadence@1.0.0 shared Character timing
→ shared 256 px normalization
→ direction, motion, consistency and loop quality
→ timing-identical Preview, Pack and Godot SpriteFrames
```

## Public contracts

- Workflow: `topdown-video-cycle@6.0.0`.
- Direction Lock entries retain `path`/`sha256` as the delivery derivative and
  add optional `generationMasterPath`/`generationMasterSha256`. V6 requires the
  master before a paid video request; older workflows remain compatible.
- Video generation uses 720p, four seconds, and requests at least three repeated
  ordinary in-place cycles. The prompt permits necessary limb and cape motion
  while locking camera, direction, core identity, palette and equipment.
- `gait-cycle@1.0.0` reports contact/passing phase indices, selected source
  range, phase score, foot-lobe count, motion, closure and verdict.
- `playback-cadence@1.0.0` defaults to eight equal-duration frames:
  - walk: 800 ms total, 100 ms per frame, 10 FPS;
  - idle: 1,600 ms total, 200 ms per frame, 5 FPS.
- All walk directions in one Character use the same cadence. Source cycle
  duration remains provenance only.

## Selection order

Walk candidates must first prove a full alternating gait:

```text
contact A → passing A → contact B → passing B → contact A boundary
```

Only candidates inside the cadence window with sufficient lower-body motion,
at most two supported foot lobes, correct phase order and a closed boundary may
participate in final ranking. The selector must try the next semantic candidate
when a visually stronger interval is only a half-cycle, a turn, a run, or
upper-body flicker.

Idle keeps the V5 initial-anchor constraint but receives fixed gameplay timing.

## Offline release gates

1. Fixture covers a valid full cycle, a high-scoring half-cycle, a static
   flicker, duplicated feet, wrong phase order and an excessive-motion run.
2. Retained V5 Job `605d08af-c5a3-4a32-98b7-849332dcce7d` supplies both
   positive and negative frozen real-video evidence: ordinary `walk_right` and
   `walk_down` clips select a 700–1,200 ms full gait, while the malformed
   `walk_up` clip remains blocked. The legacy Job
   `851999e7-e7ab-46f1-9e16-3fd98bde3ca1` predates DirectionLock/SubjectLock and
   is intentionally excluded from the safe immutable-retry route rather than
   being silently upgraded.
3. The frozen replay is run twice; gait, motion, animation-quality and loop
   reports must be byte-identical, no Pack may be emitted while any required
   direction is blocked, and all Provider usage remains zero.
4. Preview manifest, encoded GIF, Pack helper and Godot resource agree on total
   playback time within GIF's 10 ms quantum.
5. V1–V5 Jobs, Packs and schemas remain readable.
6. Provider requests and authorization-ledger changes remain zero.

Only after these gates pass may a separate authorization test one 720p
`walk_right` video. The full four-direction real run is a later gate.
