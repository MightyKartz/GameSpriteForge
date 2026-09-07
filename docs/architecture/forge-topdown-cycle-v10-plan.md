# Forge Top-down Continuous Cycle V10 plan

Status: implemented and accepted offline on 2026-08-13. One independently
authorized real `walk_down` Video 1.5 probe executed on 2026-08-14 and was
correctly rejected for center/foot-anchor drift; no Pack or Godot export was
produced.

## Decision

`topdown-cycle@10.0.0` keeps the approved V9 `DirectionGridLock` as the
immutable identity and direction authority. It does not regenerate four
direction stills and does not author walk animation as independent image
frames. Each walk direction is one continuous in-place media request followed
by deterministic complete-cycle discovery and adaptive 8/10/12-frame sampling.

The release surface remains closed except for one narrow real validation:

- validation: `walk_down`, one request, maximum one request, no Pack;
- complete offline contract: four walk requests, maximum four, plus four
  byte-reused direction idles;
- no still-generation request, no hidden retry, no video edit fallback;
- real xAI routing accepts only `xai/default`, `grok-imagine-video-1.5`,
  validation-only `walk_down`, 480p × 4 seconds, exactly one generation
  request / one model operation / 3.3B reserved cost ticks;
- every full four-direction, retry, different-model, longer-duration or other
  Provider route is rejected by Plan and Runner.

## Immutable and derived evidence

The source V9 lock, approval, node hashes, Style/Subject identity, camera and
equipment are checked during Plan preparation and again immediately before
fixture execution. A source-node hash change is rejected before any continuous
media request.

V10 creates a separate `cycle-direction-lock-adapter@1.0.0`. Its 512-pixel
generation masters are local animation inputs only; they never replace or
pretend to be the bytes approved by V9. The Pack preserves the original V9
direction nodes and approval separately from the V10 adapter provenance.

## Timing and scale contract

Decoded source media keeps native PTS up to 24 FPS and 120 candidate frames.
`gait-cycle@1.0.0` must find opposed contacts and passing poses before
`source-cycle-sampling@1.0.0` selects 8, 10, or 12 original source frames.
Preview, Pack and Godot use the same frame order and timing metadata.

All selected idles and walks enter one shared normalization pass. Static V9
anchors are canvas-matched once before this pass; no animation frame is scaled
independently. `character-scale-lock@1.0.0` blocks excessive body/torso scale,
center, or foot-baseline drift. Natural leg-width changes remain allowed while
stable upper-body scale remains locked.

## Release gates

The enabled real validation follows these gates:

1. keep the current fixture validation, full Pack and Godot contracts green;
2. perform one independently authorized `walk_down` probe with a 1/1 request
   cap and no retry; asynchronous polls may only follow the persisted id of
   that single submitted generation;
3. inspect native playback for identity, direction, body scale, foot baseline,
   upper-body flicker, duplicate limbs and loop closure;
4. stop in `awaiting_review` even when automatic gates pass; enable other
   directions only after `walk_down` is accepted;
5. keep the previous workflow available as immutable evidence, not as an
   automatic fallback.
