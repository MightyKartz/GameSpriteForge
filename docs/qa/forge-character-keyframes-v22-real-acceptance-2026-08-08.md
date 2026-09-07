# Character V2.2 four-direction real xAI acceptance — 2026-08-08

## Verdict

**Blocked correctly; not a usable Character Pack.** Forge generated all 32 authorized
frames but rejected 21 as hard failures and paused 3 for review. Only 8 were
`game_ready`, so no Pack was exported, no Catalog entry was written, and Godot
installation was correctly skipped.

## Provider execution

- Source Job: `f17b569e-5549-41ad-a38c-c2e08ddbe8a6`
- Workflow: `topdown-keyframes@2.2.0`
- Provider/model: `xai` / `grok-imagine-image-quality`
- Scope: `idle`, `walk_up`, `walk_right`, `walk_down`, eight frames each
- Estimate / maximum / actual: 32 / 64 / 61 image edits
- Cost cap / observed settled cost: 51.2B / 45.7B ticks
- Approximate observed cost: USD 4.57
- All 61 ledger entries settled; all 32 authorized frame targets were used
- No Subject, Style, video, upload, icon, prop, portrait, or world request occurred

The conservative sum of per-request reservations was 48.8B ticks; observed Provider
cost was 45.7B. Both stayed below the user-approved 51.2B cap.

## What V2.2 fixed

The repaired reference boundary behaved as designed:

- Provider manifest: `styleReferenceMode: prompt_descriptor`
- Pose: `topdown-poses@1.1.0`
- Equipment: explicit `none`
- No keyframe request contained a Style image role
- No frame contains the staff previously copied from the Style board

This confirms that removing the Style board as an image reference fixed that specific
staff/content-leakage path.

## Remaining visual failures

Native-size and contact-sheet review found:

- `walk_up` remains predominantly front-facing instead of showing the rear view;
- `walk_right` remains predominantly front-facing instead of a stable side view;
- many frames contain opaque blue-gray scene rectangles or irregular background
  regions despite the transparent-background instruction;
- identity, face, leg/lower-body geometry, and edge detail change across frames;
- several in-betweens inherit defects from their neighboring anchors.

Machine results match the visual review:

| Animation | game_ready | awaiting_review | blocked |
| --- | ---: | ---: | ---: |
| idle | 2 | 0 | 6 |
| walk_up | 1 | 0 | 7 |
| walk_right | 1 | 1 | 6 |
| walk_down | 4 | 2 | 2 |

Top failure counts were identity similarity (16), missing lower-body proxy (15), edge
density drift (10), opaque background residual (9), and low-Alpha noise (7).

## Zero-cost replay, delivery, and security

Child Job `c3299bd2-8e16-41a3-b6cc-065adea014a0` replayed consistency from the immutable
source frames with zero Provider requests and reproduced the blocked outcome. The
authorization ledger remained at 61 requests.

No `.gsfpack` exists in either Job and `ayla-ranger-keyframes-v22` is absent from the
project Catalog. Consequently, running the Godot installer would have bypassed the
quality contract and was not attempted.

JobStore, retained reports, and the non-secret authorization manifest/ledger passed
credential, Bearer-header, Device Code, API-key, and temporary-media-URL scans.

## Evidence

- Machine summary:
  [`artifacts/forge-character-keyframes-v22-real-20260808/summary.json`](artifacts/forge-character-keyframes-v22-real-20260808/summary.json)
- Four-direction contact sheet:
  [`artifacts/forge-character-keyframes-v22-real-20260808/previews/all-directions-contact.png`](artifacts/forge-character-keyframes-v22-real-20260808/previews/all-directions-contact.png)
- Per-direction GIFs and machine reports are retained beside the summary.

## Recommended next change

Do not spend the remaining three authorized requests. The next workflow revision should
make direction a first-class visual lock rather than relying on action names: generate
and approve one canonical front, rear, and side direction anchor before any gait phase,
then derive all eight frames from the approved direction anchor. Background removal
should occur immediately after each Provider response so background defects do not
propagate into in-betweens. Revalidate one direction before another 32-frame run.

