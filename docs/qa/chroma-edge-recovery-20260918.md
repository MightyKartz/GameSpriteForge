# Chroma edge recovery QA — 2026-09-18

Scope: optional manual-key controls added after the real lightning effect showed
that a conservative matting path can shrink faint glow even though its visible
RGB stayed nearly unchanged. The source and prior Forge evidence remain in the
local QA archive; this file records the comparison without committing generated
media.

## Tested input

- Source: 1254×1254 RGB PNG with a 2×2 cyan lightning effect on a flat magenta
  background.
- Existing production-style parameters: manual `#FF00FF`, threshold 110,
  softness 90, despill 0.5, halo 1.
- New options: `backgroundScope:"border_connected"` and
  `edgeColorRecovery:true`.
- CLI: local debug build from this branch; no Provider requests and no Job store
  are used by `source matte`.

## Result

The existing parameters produced a `game_ready` result but retained 4.96% visible
pixels with alpha mean 12.29. The new strict border-connected/recovery path with the
same threshold and softness retained 5.66% visible pixels with alpha mean 14.07.
A gentler threshold/softness pair (96/48) retained 5.71% visible pixels with
alpha mean 14.36. Both new outputs cleared RGB beneath alpha zero and showed no
visible magenta fringe on the dark/light contact sheet. Small purple pixels in
the gentler output are part of the generated lightning art, not proof of
background residue.

The higher RGB difference in recovered output is intentional: partial-alpha edge
pixels are unblended from the magenta key toward their inferred foreground color.
This is a visual review aid, not art approval. The default `auto` scope and all
old request JSON remain unchanged.

## Limits

This was a macOS local synthetic/real-image check, not Windows CI evidence and
not a license review. Border-connected removal is suitable for flat color-key
backgrounds, not semantic segmentation or complex photographic backgrounds.
