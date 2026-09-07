# Forge `topdown-video-locked@5.0.0` real xAI acceptance

Date: 2026-08-09

Result: **FAILED SAFE — all four directions were generated, but no Pack was exported and no Godot installation was performed.**

Machine summary: [summary.json](artifacts/forge-topdown-video-locked-v5-real-20260809/summary.json)

## Authorization and usage

- Reused Ayla's accepted V4 DirectionLock; no Subject, Style or direction sheet was regenerated.
- Allowed only `idle:video`, `walk_up:video`, `walk_right:video`, and `walk_down:video`.
- Maximum two video requests per direction, eight total, with a 48,000,000,000 tick hard cap.
- Actual usage: eight videos and 26,400,000,000 ticks (approximately USD 2.64).
- Provider/model: `xai` / `grok-imagine-video-1.5`.

## Result

The first Job generated all four videos and automatically retried `walk_right`. Native-size review then blocked the candidate: the first `walk_down` changed yaw and introduced an unapproved backpack; `walk_up` and `idle` had excessive temporal outline changes.

The remaining authorization was used for one targeted retry each of `idle`, `walk_up`, and `walk_down`. `walk_right` was reused byte-for-byte.

- `idle`: local quality marked the selected interval game-ready, but the loop selector chose a nearly static closed-eye segment. It does not preserve the open-eye DirectionAnchor expression and is manually blocked.
- `walk_up`: the rear walk is visually improved, but the semantic gate detects the face in three of eight frames and temporal outline checks remain blocked.
- `walk_right`: visually the strongest result, with an intelligible side gait and closed loop, but upper-body mask and contour drift remain hard failures.
- `walk_down`: the retry removed the backpack, but produced an exaggerated high-kick march instead of a normal walk. Small green remnants also remain at the foot plane.

Because these are semantic and temporal hard failures, Forge did not use manual review to force export.

## Evidence

- Initial Job: `generated-assets/forge-topdown-video-locked-v5-real-20260809/jobs/e621d016-1303-457b-995d-6f139f3b41b6`
- Targeted retry Job: `generated-assets/forge-topdown-video-locked-v5-real-20260809/jobs/c5809f1f-94ab-4c1e-b6a3-0cd1878f88f1`
- Final idle preview: `generated-assets/forge-topdown-video-locked-v5-real-20260809/jobs/c5809f1f-94ab-4c1e-b6a3-0cd1878f88f1/previews/debug/idle-dark-1x.gif`
- Final walk-up preview: `generated-assets/forge-topdown-video-locked-v5-real-20260809/jobs/c5809f1f-94ab-4c1e-b6a3-0cd1878f88f1/previews/debug/walk_up-dark-1x.gif`
- Final walk-right preview: `generated-assets/forge-topdown-video-locked-v5-real-20260809/jobs/c5809f1f-94ab-4c1e-b6a3-0cd1878f88f1/previews/debug/walk_right-dark-1x.gif`
- Final walk-down preview: `generated-assets/forge-topdown-video-locked-v5-real-20260809/jobs/c5809f1f-94ab-4c1e-b6a3-0cd1878f88f1/previews/debug/walk_down-dark-1x.gif`

## Findings for the next implementation pass

1. Loop selection must score semantic compatibility with the DirectionAnchor, including eye/expression state for idle; closure alone selected the wrong idle subsection.
2. Direction persistence must be evaluated across every selected candidate frame, not only through the input anchor or a coarse visible-face ratio.
3. Walk semantics need an explicit ordinary-walk constraint that rejects kicks, marching and running even when motion energy and loop closure pass.
4. Ground cleanup needs a post-matting foot-plane pass for small chroma remnants touching the boots.
5. The collection consistency report was byte-identical before and after three animation videos changed. Its current metrics are effectively anchor-based and do not validate regenerated action frames.

## Security

- Credential, Token, Device Code and Authorization-header scan: clean.
- Temporary media URL scan: clean.
- No Pack or Godot project asset was produced from the blocked result.
