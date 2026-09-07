# Forge `topdown-cycle@10.0.0` real walk-down probe — 2026-08-14

Verdict: the exact one-request real probe completed, but the generated cycle
was correctly rejected. No retry, Pack, export, or Godot installation was
performed.

## Authorized scope and usage

- Parent: approved V9 DirectionGridLock Job
  `9ea9cae9-dd94-4b4a-a277-4b5d0345b2ad`.
- Job: `f80b6e47-9f58-4d8d-9dc3-4bdd239a9e6c`.
- Workflow/provider/profile: `topdown-cycle@10.0.0`, `xai/default`.
- Model and target: `grok-imagine-video-1.5`, `walk_down:video`.
- Request shape: one 480p, four-second image-to-video generation.
- Exact authorization: one target request, one model-generation operation,
  and 3,300,000,000 reserved cost ticks.
- Observed durable ledger: one settled `generate_video` request and
  3,300,000,000 observed cost ticks. Provider usage records one generated
  video, zero image generations, zero video edits, and zero private uploads.

The generated video SHA-256 is
`1d1e5d3741af8b78da55f152102aa0ee466c35056fd994d68b87ef59885c30b4`.
The approved source tree remained unchanged at
`785da20140de9aff17be792f870dd010ec564f289e8baa60eef9eb7fe5caddbc`.

## Automatic and native review

The native source-cycle selector found a coherent 1,166 ms gait interval and
sampled 12 frames from source frames 19 through 45. Gait order was
`game_ready`, with a 0.981 phase-order score and 0.946 closure score. Canonical
identity, front direction, empty hands/equipment, and background cleanup also
passed.

Delivery was blocked by `character-scale-lock@1.0.0`:

- body-center drift reached 9 px; the locked maximum is 4 px;
- foot-baseline drift reached 10 px; the locked maximum is 3 px;
- body width varied only 92–95 px and height 218–226 px, so the primary defect
  is translation/anchor instability rather than severe scale pumping;
- motion diagnostics also detected stable-upper-body flicker, especially
  around output frames 7–8;
- temporal silhouette diagnostics detected upper-body contour/edge flicker,
  a 3.5 px adjacent center step, and an 8 px adjacent foot-anchor step.

Native contact-sheet review agrees with the reports: identity and clothing are
well preserved, the two feet alternate, and the stride is more legible than
the earlier video routes, but the whole Character drifts sideways and
vertically through the cycle. The extreme forward-foot poses and cape/edge
changes would still look unstable in Godot.

## Delivery and safety

- Final lifecycle: `failed` with `character_scale_lock_failed`.
- Pack count: 0.
- Godot project/install count: 0.
- Additional real requests or retries: 0.
- Credential, bearer-token, API-key, refresh-token, and temporary-URL artifact
  scan: 0 matches.
- The exact authorization is consumed and must not be reused.

Machine-readable evidence:
[`summary.json`](artifacts/forge-topdown-cycle-v10-real-20260814/summary.json)

Local visual evidence:

- `previews/debug/walk_down-checkerboard-1x.gif`
- `previews/debug/walk_down-checkerboard-4x-contact.png`
- `contact-sheet.png`

## Recommendation

Do not regenerate blindly and do not export this result to Godot. Implement a
V10.1 deterministic anchor-stabilization pass before any new paid request:
derive a robust planted-foot baseline and torso center per source frame,
translate frames onto the approved anchor without per-frame resizing, preserve
the intentional leg motion, and rerun the current scale, silhouette, motion,
and native-review gates offline. Only after that change passes on this already
generated video should a separately authorized new real probe be considered.
