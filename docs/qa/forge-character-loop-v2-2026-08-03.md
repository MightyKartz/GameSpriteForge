# Forge Character Loop V2 release-blocking acceptance — 2026-08-03

## Decision

The Character release blocker is resolved. `loop@2.0.0`, stage-level retry, xAI
private video editing, additive Pack evidence, and Godot provenance passed offline
contracts and three consecutive clean real-xAI Style → Character → Pack → Godot
runs. No manual review was used for the three counted runs.

This evidence clears the Character stability gate for `v0.2.0-cli.1`. It does not
create or publish the tag: Developer ID packaging, notarization, FFmpeg/SBOM audit,
and the GitHub Release job remain separate release operations.

## Release-blocking implementation

- Candidate extraction covers the full source video at at most 12 FPS and 96 frames.
- Loop selection mattes and aligns a deterministic 256px analysis canvas, searches
  start/boundary pairs, and exports only evenly sampled `[start, end)` frames.
- The closure boundary is evidence only and is never duplicated in the animation.
- Mask IoU, palette, edge, anchor, transition, and motion thresholds were not lowered.
- Transition continuity compares motion per source-frame interval, avoiding false
  discontinuities when an eight-frame export samples a non-divisible source period.
- Candidate features and motion deltas are precomputed. Full-resolution matting runs
  only for the selected output frames. A historical 720p four-action local replay
  takes 11 seconds in the optimized Release binary.
- `outputFrameSha256` maps every reported source index to its actual selected PNG.
- Multiple subjects, missing foreground, and cropped candidates remain hard failures.
- Direction still and video prompts are separate. A still is explicitly one keyframe,
  not a loop, sprite sheet, turnaround, collage, or contact sheet.
- Character palette and edge consistency use the canonical character reference under
  `consistency@1.2.0`; the mixed style board still constrains generation but is not
  treated as the character's literal color palette.
- `still`, `video`, `loop`, and `matting` retries create source-linked Jobs. Explicit
  `loop`/`matting` retry is guaranteed to make zero Provider requests.
- Every Character Job writes `workflow-stage-manifest.json` with implementation
  versions, input/output hashes, invalidated descendants, and Provider-request flags.
- `.gsfpack` V2 adds `quality/loops.json`; Godot `forge_usage.json` records the loop
  profile, selected range, output indices, and Provider retry method.

## xAI video repair

- Forge uses `/v1/videos/edits` with the documented `grok-imagine-video` edit model;
  generation continues to use the separate generation default.
- Existing same-Provider `file_id` is preferred. Otherwise Forge uploads a private
  `/v1/files` input with a one-hour TTL, submits the edit, and deletes the file after
  success, failure, or cancellation.
- When Files API is unavailable, only videos at or below 32 MiB may use a data URL;
  larger inputs fall back to image-to-video from the same locked still.
- Structured xAI error code/message text is retained only after length and credential/
  data-URL checks; raw response bodies are never persisted.
- A real run exercised private upload, video edit, download, and cleanup. The Job
  recorded one private upload, one edited video, zero generated images, and
  `retryMethod: video_edit`.

## Three consecutive clean real-model runs

| Run | Character Job | Provider requests | Cost ticks | Retry | Result |
| --- | --- | ---: | ---: | --- | --- |
| 3 | `e3dae37c-fc5d-4b50-b68e-6808a6a6dd4e` | 13 | 26,200,000,000 | none | Pack + Godot passed |
| 4 | `6ab4e241-de4b-4ff6-a595-7657570a1a81` | 17 | 29,400,000,000 | `walk_up` video edit once | Pack + Godot passed |
| 5 | `2f7c8d19-e903-4234-96ac-709b0e37cd63` | 13 | 26,200,000,000 | none | Pack + Godot passed |

All twelve action results were `game_ready`. Every direction had exactly one subject.
Run 4 remained within the maximum two Provider video attempts and did not regenerate
the canonical character reference.

### Loop scores

| Run | idle | walk_up | walk_right | walk_down |
| --- | ---: | ---: | ---: | ---: |
| 3 | 0.978 | 0.925 | 0.967 | 0.935 |
| 4 | 0.879 | 0.896 | 0.922 | 0.874 |
| 5 | 0.957 | 0.930 | 0.952 | 0.923 |

Visual and machine-readable evidence:

- [Run 3 direction contact sheet](artifacts/forge-loop-v2-real-20260803/run-3/jobs/e3dae37c-fc5d-4b50-b68e-6808a6a6dd4e/contact-sheet.png)
- [Run 3 animation preview](artifacts/forge-loop-v2-real-20260803/run-3/jobs/e3dae37c-fc5d-4b50-b68e-6808a6a6dd4e/exports/5b9f6808-411e-4f37-ae48-e2a07ef193cb/preview.gif)
- [Run 4 direction contact sheet](artifacts/forge-loop-v2-real-20260803/run-4/jobs/6ab4e241-de4b-4ff6-a595-7657570a1a81/contact-sheet.png)
- [Run 4 animation preview](artifacts/forge-loop-v2-real-20260803/run-4/jobs/6ab4e241-de4b-4ff6-a595-7657570a1a81/exports/5ddccca0-0820-4a60-888e-aecc9c408f21/preview.gif)
- [Run 5 direction contact sheet](artifacts/forge-loop-v2-real-20260803/run-5/jobs/2f7c8d19-e903-4234-96ac-709b0e37cd63/contact-sheet.png)
- [Run 5 animation preview](artifacts/forge-loop-v2-real-20260803/run-5/jobs/2f7c8d19-e903-4234-96ac-709b0e37cd63/exports/173333df-d1aa-42af-82ff-4d13167f2513/preview.gif)

## Godot and security audit

- Godot `4.6.3.stable` installed and loaded all three Packs.
- Generated `.tres`/`.tscn` files are 5.4–7.7 KiB, far below 1 MiB.
- No Godot resource contains `PackedByteArray` or an embedded Image subresource.
- JobStore, asset projects, Packs, and Godot projects contain no Authorization header,
  Bearer credential, API/OAuth token, device code, API key, or video data URL.
- No temporary xAI media URL was persisted.
- Each clean Job has one hashed workflow stage manifest and reconstructible Provider
  usage/retry provenance.

## Regression gates

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- Full Rust workspace: 117 tests passed.
- `scripts/test-cli-product.sh`: passed.
- `scripts/test-cli-installer.sh`: passed.
- `scripts/test-cli-signing-contract.sh`: passed.
- Fixture coverage includes inner-period selection, repeated-boundary exclusion,
  non-uniform sampling gaps, static walk, subtle idle, missing foreground, malformed
  media, bad-loop video editing, unsupported-edit same-still fallback, zero-request
  local retry, output-index/hash identity, Pack validation, and Godot loading.

## Follow-up after the first CLI release

The 20-character/five-style benchmark is still required before making broad commercial
consistency claims. SubjectLock, collection medoid/outlier analysis, optional DINO/
LPIPS/SAM components, and deterministic `rig2d` research remain post-release work and
were not bundled into this release slice.
