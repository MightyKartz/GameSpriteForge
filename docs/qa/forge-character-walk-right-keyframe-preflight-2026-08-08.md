# Character `walk_right` keyframe real-acceptance preflight — 2026-08-08

Forge now has a bounded `topdown-keyframes@2.1.0` acceptance mode for a
single Character direction. This closes the scope gap that previously forced a
32-frame Character generation even when only `walk_right` hands, staff contact,
and cadence needed validation.

## Contract

- CLI: `forge generate character ... --validation-animation walk_right`
- Provider plan: 8 expected image edits, 16 maximum.
- Authorization targets: `walk_right:frame:0` through
  `walk_right:frame:7`, at most two attempts per target.
- Cadence: eight frames at 8 FPS, a 1,000 ms walk cycle inside the
  `animation-timing@1.0.0` 800–1,250 ms window.
- Local gates: consistency, normalization, hand/equipment contact, silhouette,
  quality, workflow provenance, and checkerboard/chroma-green/dark previews.
- Safety: the Job cannot export, catalog, or install an incomplete Character
  Pack. Godot installation remains reserved for the later complete four-action
  Pack; the validation Job supplies exact playback GIFs and contact sheets.

## Offline result

The provider keyframe contract passed with exactly eight `frame_image` nodes,
only `walk_right` item nodes, eight fixture image requests, no `.gsfpack`, and
no catalog entry. The CLI authorization-target unit contract also passed with
exactly the eight frame targets. A real xAI plan-only run resolved
`grok-imagine-image-quality`, reported 8/16 requests, and made zero Provider
requests.

Machine-readable evidence is in
`artifacts/forge-character-walk-right-keyframe-preflight-20260808/`.

## Remaining real-provider gate

Execution requires a new least-privilege authorization. The recommended cap is
16,000,000,000 cost ticks (approximately USD 1.60 using the current one-billion
tick reservation), with no Subject, Style, video, other direction, icon, prop,
portrait, terrain, or building targets.
