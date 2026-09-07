# Forge `topdown-video-cycle@6.0.0` real xAI acceptance

Date: 2026-08-09

Verdict: real generation completed but the Character was correctly blocked; no
Pack or Godot asset was produced.

## Authorized scope and usage

- Provider/profile: `xai/default`.
- Models: `grok-imagine-image-quality` and `grok-imagine-video-1.5`.
- Targets: `direction_lock`, `idle:video`, `walk_up:video`,
  `walk_right:video`, `walk_down:video` only.
- Authorized maximum: 10 requests and 48,000,000,000 cost ticks.
- Observed: 5 requests: one image edit and four 720p image-to-video requests.
- Forge usage estimate: 23,400,000,000 cost ticks. The xAI terminal responses
  did not supply per-request settled cost, so the durable ledger retains its
  conservative reservation provenance.
- No Subject, Style, video edit, upload or unrelated asset request occurred.

Job: `fb700708-603c-4c63-94be-d242eea23dd8`.

## Results

DirectionLock passed and produced four 512×512 generation masters. All three
walk videos contained a selectable complete gait:

- `walk_up`: 917 ms, `game_ready` gait.
- `walk_right`: 833 ms, `game_ready` gait.
- `walk_down`: 1,083 ms, `game_ready` gait.

The final cross-animation gates then correctly blocked delivery:

- `idle`: three lower-edge/foot lobes instead of at most two. Visually, the
  cape opens dramatically rather than behaving like a subtle breathing idle.
- `walk_up`: stable-upper-body flicker ratio 0.386, above the 0.08 limit.
- `walk_right`: stable-upper-body flicker ratio 0.401 and invalid final phase
  order. The selected frames look more like a run, with large cape and body
  changes.
- `walk_down`: stable-upper-body flicker ratio 0.128, invalid phase order,
  missing front-face evidence in part of the interval and direction drift.

The gait selector therefore solved the earlier half-cycle/speed problem, but
the video model still changed too much of the supposedly stable upper body and
produced run-like motion. Retrying blindly under the remaining allowance was
not permitted because the user required immediate stop on anomaly.

## Delivery and safety

- Pack count: 0.
- Godot installation: skipped because there was no valid Pack.
- Credential/Authorization-header/temporary-URL scan: passed.
- Provider, model, target and request counts match the durable authorization.
- The first `plan execute --authorization` attempt exposed a CLI ordering bug:
  legacy real-provider preflight ran before the durable authorization was
  attached. It failed before plan claim or network access. Execution then used
  legacy environment caps identical to the reviewed 10-request/48B allowance,
  while every real request remained enforced by the durable ledger.

Machine-readable evidence:
[`summary.json`](artifacts/forge-topdown-video-cycle-v6-real-20260809/summary.json)

Visual evidence:

- `contact-sheet.png`
- `previews/debug/{idle,walk_up,walk_right,walk_down}-checkerboard-1x.gif`

## Recommendation

Do not spend the remaining five requests yet. The next code change should make
V6 final motion/semantic diagnostics drive a targeted video retry and revise
the prompt toward restrained ordinary walking: planted torso, minimal cape and
scarf motion, no run stride, no eye closure, and explicit front/rear/profile
hold throughout every repeated cycle. The durable-authorization preflight
ordering bug should also be fixed before another paid run.
