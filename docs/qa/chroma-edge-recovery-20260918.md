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

## Follow-up: transparent source path

Codex image generation was unavailable during this follow-up because the local
usage limit returned HTTP 429, so the QA source was derived locally without
pretending a new model generation occurred. Each original 627×627 transparent
frame was copied pixel-exactly into a 700×700 cell with 37 px of padding on all
sides. No resampling, matting, color changes or Provider requests occurred.

The expanded sheet (`sha256
9b280b743b9320dda6c735a1443758c30f37bedd25e33c3922c43c3aca025ebd`) passed
`source inspect` with no alpha-1 or alpha-32 cell-edge contact. A `preserve_alpha`
Forge animation Job succeeded with zero Provider requests, `game_ready`,
`cellBoundarySafe:true`, and all four exported Pack frames decoded pixel-exactly
to the expanded source. This confirms that preserving a clean transparent source
is the highest-fidelity path; chroma matting and edge recovery remain fallbacks
for flat-background inputs.
