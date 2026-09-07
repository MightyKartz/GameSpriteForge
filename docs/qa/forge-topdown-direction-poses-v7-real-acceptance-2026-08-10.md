# Forge Top-down Direction Poses V7 Real Acceptance — 2026-08-10

## Result

`topdown-direction-poses@7.0.0` completed real xAI generation but did not pass the game-ready release gate. Forge stopped before Pack export and Godot installation.

- Job: `cb00d5b8-a2e2-449c-b2e3-a145426d8358`
- Lifecycle: `failed`
- Error: `character_gait_cycle_failed`
- Provider: `xai/default`
- Image model: `grok-imagine-image-quality`
- Video model: `grok-imagine-video-1.5`
- Pack exported: no
- Godot installed: no

## Authorized scope and observed usage

Authorized targets were limited to:

```text
direction_rear
direction_right
walk_up:pose
walk_right:pose
walk_down:pose
walk_up:video
walk_right:video
walk_down:video
```

Authorization allowed 16 requests and 42,000,000,000 cost ticks. Observed usage was:

- 9 settled requests.
- 6 image edits: 3,900,000,000 ticks.
- 3 720p videos: 17,100,000,000 ticks.
- Total: 21,000,000,000 ticks, approximately US$2.10.
- Video edits, Subject generation, Style generation and private file uploads: zero.
- `direction_right` consumed its second permitted attempt; all other targets used one attempt.

## Gate results

Direction and framing passed for all delivered animations. The static SubjectLock idle passed.

| Animation | Direction | Gait | Motion/silhouette finding | Result |
|---|---|---|---|---|
| `idle` | down | not applicable | static SubjectLock frame passed | game ready |
| `walk_up` | rear | gait passed; phase score 0.941 | stable upper-body flicker, contour/core edge drift | blocked |
| `walk_right` | right | third foot lobe; maximum 3 | phase-order failure and upper-body flicker | blocked |
| `walk_down` | down | gait passed; phase score 0.983 | lower-body edge ghost and contour/core drift | blocked |

The generated directions, camera framing and body scale were correct. The failure is temporal animation quality rather than direction-lock quality.

## Visual evidence

![V7 real contact sheet](../../generated-assets/forge-topdown-direction-poses-v7-real-20260810/jobs/cb00d5b8-a2e2-449c-b2e3-a145426d8358/contact-sheet.png)

Local playback previews:

- `walk_up`: `generated-assets/forge-topdown-direction-poses-v7-real-20260810/jobs/cb00d5b8-a2e2-449c-b2e3-a145426d8358/previews/debug/walk_up-checkerboard-1x.gif`
- `walk_right`: `generated-assets/forge-topdown-direction-poses-v7-real-20260810/jobs/cb00d5b8-a2e2-449c-b2e3-a145426d8358/previews/debug/walk_right-checkerboard-1x.gif`
- `walk_down`: `generated-assets/forge-topdown-direction-poses-v7-real-20260810/jobs/cb00d5b8-a2e2-449c-b2e3-a145426d8358/previews/debug/walk_down-checkerboard-1x.gif`

## Evidence hashes

| Evidence | SHA-256 |
|---|---|
| `job.json` | `7af42bb0189821d5564f71fa64b7a6557ed1fcbfb1bf4099ca2760a56ccdc132` |
| `provider-usage.json` | `e1ba7baeb108371f9d2804b3f8ab278eb98608cdf654db8dbde3a4c82baf4ac1` |
| `character-gait-cycle-report.json` | `43f6850c0f33bd1a8a43985d1e2805d4edfb8e4f5bfb2bea14d761e79e2912c8` |
| `character-motion-semantics-report.json` | `a7f921769d1caa558ebf47dd2264ec056d6b13aafdffdeebe3de598c4a26a64d` |
| `character-semantic-quality-report.json` | `f95ca0b75160d4ce8f95374dc61128671d0930204227afd1bd6d31a15db883eb` |
| `consistency-report.json` | `c76555dc5d0baf2b4528adeb1a801abff5f57061079abe97f531df4e6134f49e` |
| `contact-sheet.png` | `4695289c197dc0456119324a7a75f0260526ca78fd6b839223911d889b66d998` |
| `direction-lock.json` | `a67d0d5311b069733e21ee7b7cd2d17d7d177b6b9241f7402cf55b5bd47517ad` |
| `direction-pose-lock.json` | `f06450ef985d8784dfb325c28b9ea8ee09cd21ca460f3982ede8294997b2e636` |

## Security audit

The run root was scanned for API keys, OAuth access/refresh tokens, Device Codes, Bearer values, Authorization headers and temporary media/download URLs. No matches were found. No Pack or Godot project was produced.

## Decision

Do not spend the remaining authorization automatically. The result indicates that DirectionLock and PoseLock solved direction and framing, but image-to-video still introduced temporal upper-body variation and lower-body artifacts. A retry could produce a different sample, but it would not establish that the workflow is reliable. The next change should make candidate selection evaluate motion, silhouette and foot-lobe gates before committing to one interval, then replay this already-paid video locally before requesting more media.

