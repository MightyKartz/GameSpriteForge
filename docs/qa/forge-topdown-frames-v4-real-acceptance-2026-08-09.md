# Forge `topdown-frames@4.0.0` real xAI acceptance

Date: 2026-08-09

Result: **FAILED SAFE — no Pack exported and no Godot installation performed.**

Machine summary: [summary.json](artifacts/forge-topdown-frames-v4-real-20260809/summary.json)

## Authorization and actual usage

- Character: Ayla, existing immutable SubjectLock and StyleLock.
- Equipment: `none`.
- Provider/model: `xai` / `grok-imagine-image-quality`.
- Authorized targets: `direction_lock`, `idle`, `walk_up`, `walk_right`, `walk_down`.
- Authorized budget: expected 5, maximum 10 image requests; maximum 8,000,000,000 ticks.
- Actual: 4 image requests and 2,400,000,000 ticks (approximately USD 0.24).
- No Subject, Style, video, or unrelated asset was generated.

## Result by stage

The four-view DirectionLock passed on its first request. Front, rear, right and left views have strong identity, palette, full-body framing and cardinal separation.

`idle` passed the current machine gates, but fails manual review. The second cell changes both arms and facial expression substantially, while the fourth cell contains an unexpected light cheek artifact. This is a false acceptance: the current consistency checks do not sufficiently protect facial expression, hands and small costume/skin details inside a four-cell action sheet.

`walk_up` attempt 1 kept the correct rear view and visibly alternated the feet, but the local motion gate rejected it for insufficient contact diversity and invalid phase order. The long cape occludes most of the legs, so this appears to be a detector false negative rather than a frozen animation.

`walk_up` attempt 2 changed to a three-quarter side view and exposed the face. The direction gate correctly blocked it.

Forge stopped after the exhausted two attempts for `walk_up`. It did not spend the remaining authorization on `walk_right` or `walk_down`, because a complete game-ready Pack was no longer possible.

## Evidence

- DirectionLock sheet: `generated-assets/forge-topdown-frames-v4-real-20260809/jobs/5de29c82-8141-40f5-bbaa-bf79b2c17753/source/provider-direction-lock/attempt-1/sheet.png`
- Idle sheet: `generated-assets/forge-topdown-frames-v4-real-20260809/jobs/5de29c82-8141-40f5-bbaa-bf79b2c17753/source/provider-animation-sheets/idle/attempt-1/sheet.png`
- Walk-up attempt 1: `generated-assets/forge-topdown-frames-v4-real-20260809/jobs/5de29c82-8141-40f5-bbaa-bf79b2c17753/source/provider-animation-sheets/walk_up/attempt-1/sheet.png`
- Walk-up attempt 2: `generated-assets/forge-topdown-frames-v4-real-20260809/jobs/5de29c82-8141-40f5-bbaa-bf79b2c17753/source/provider-animation-sheets/walk_up/attempt-2/sheet.png`
- Direction, action, onion-skin, Provider usage and authorization ledgers remain in the immutable Job and authorization directories.

## Security audit

- Credential, token, Device Code and Authorization-header scan: clean.
- Temporary media URL scan: clean.
- No Pack or Godot project exists for this failed Job.

## Required remediation before another paid run

1. Add an action-cell identity/detail gate against the selected DirectionAnchor, emphasizing face, hands, scarf and clothing palette.
2. Treat idle as a local edit contract: preserve face and limbs, allowing only small torso/shoulder displacement.
3. Make walk motion analysis cape-aware by measuring visible foot contact and foot-centroid alternation instead of requiring a large unobscured lower-body mask change.
4. Preserve the correct first `walk_up` attempt as a frozen QA sample and calibrate the detector against it before spending on another retry.

