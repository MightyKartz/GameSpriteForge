# Forge V18 approved walk-right production delivery

Date: 2026-08-19

Status: production approved and verified.

The user explicitly confirmed `v18 六项通过`, covering motion, placement,
edge cleanup, foot alternation, pivot, and collision. The original pending
review remains immutable. A new approved review was created without changing
the source video, prompt, replay-summary, frame hashes, or native durations.

## Approval closure

- source video SHA-256:
  `123d82fe5088a86083ab2dd689c9c5b9388181700341bb9ee09f666341007d38`;
- prompt SHA-256:
  `cf110e6e8bd503200a8053421ca9f37132e2a5a2b0887600a4140a7b168a8861`;
- approved review SHA-256:
  `a1f6638b2ba4e98eb11729c3e6d5a9bb640af59606adbe6b5dab01234d438648`;
- review timestamp: `2026-08-19T11:14:59.993679Z`;
- frame hashes: 24, unchanged from the pending record;
- checks: six of six true.

Approved review:
`generated-assets/experiments/motion-video-browser-benchmark-20260819/vidu-q2-rewarded-v2-phase-lock/review-v18-approved/animation-human-review.json`.

## Production Pack

- ID/name/version: `v18-walk-right-approved` / `V18 Walk Right Approved` /
  `1.0.0`;
- animation: `walk_right`, 24 frames, 2000ms total;
- frame timing: exact 83/84ms sequence;
- anchor: custom `(720,1316)`;
- rendering: linear, no pixel snap;
- productionEligible: true;
- Pack validation: passed;
- forgepack SHA-256:
  `b2d0338988c3cb1399421b2ab05255cda5f5f462aefbcd2aac3d22bec8ad8ca0`;
- manifest SHA-256:
  `435560bb2c909d29e8611c34233781d34046124b604196b7cd6191cc06543278`.

Pack:
`generated-assets/experiments/motion-video-browser-benchmark-20260819/vidu-q2-rewarded-v2-phase-lock/delivery-v18-approved-production/v18-walk-right-approved/V18-Walk-Right-Approved.gsfpack`.

## Godot verification

The production Pack was installed into a fresh Godot 4.6.3 project. Runtime
assertions passed for frame count, per-frame durations, total duration, and
external textures. Generated `.tres/.tscn` resources are below 1 MiB and
contain no embedded Image, Base64, or PackedByteArray payload.

Project:
`generated-assets/experiments/motion-video-browser-benchmark-20260819/vidu-q2-rewarded-v2-phase-lock/godot-v19-approved-production/`.

Movie:
`qa-output/v19-approved-walk-right.mp4`, SHA-256
`ce919232af8a4461a56be03ffc7cbebd61f8ef7e6da545835c3bccc3269458e6`.

## Verification

- Core unit tests: 329 passed;
- Core lib/examples clippy: passed;
- formatting: passed;
- Pack validation: passed;
- Godot headless and MovieWriter: passed;
- Provider requests: 0.

