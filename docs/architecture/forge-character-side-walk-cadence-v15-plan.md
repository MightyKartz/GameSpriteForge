# Forge character side-walk cadence V15 plan

Date: 2026-08-18

## Decision

V14 fixed excessive knee lift but exposed a false-negative in the shared
four-pose phase gate: a right-facing side walk may retain a similar Alpha
silhouette when foreground and background legs exchange roles. V15 separates
deterministic contact/passing cadence from human laterality approval.

## Motion-semantics V1.4

- Four-frame `walk_left` and `walk_right` use a side-specific cadence score.
- The score accepts either a clear wide-contact/compact-passing silhouette
  rhythm or the existing geometric phase evidence.
- The legacy Alpha contact-polarity score remains reported for calibration but
  does not block a four-frame side walk by itself.
- `sideLateralityReviewRequired` explicitly records that Alpha cannot prove
  which anatomical leg is in front.
- A separate `side-walk-laterality-approval@1.0.0` document locks exact frame
  hashes and requires human confirmation of foreground/background exchange,
  same-leg repetition, and normal amplitude.

## V15 real-generation scope

Generate one fresh `walk_right` 2x2 sheet only. References have separate roles:

1. accepted direction still: sole identity/style/camera authority;
2. V14 2x2 sheet: normal-amplitude motion and layout reference only;
3. low-amplitude joint guide: phase geometry and near/far limb identity only.

The prompt must preserve V14's low foot clearance while making the near/far
leg exchange readable through overlap, shading, and boot depth. Head, hood,
scarf, torso, belt, pouches, and cape construction stay visually consistent,
but no pixels or rectangular upper-body region are copied or spliced.

## Promotion rule

V15 remains blocked until deterministic QA passes and a human laterality
approval with the exact clean-frame hashes is written. Four frames are the
delivery target; eight-frame expansion and other directions remain out of
scope.
