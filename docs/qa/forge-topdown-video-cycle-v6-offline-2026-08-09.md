# Forge `topdown-video-cycle@6.0.0` offline acceptance

Date: 2026-08-09

Verdict: offline implementation accepted; workflow remains experimental and no
new real-Provider generation was authorized or performed.

## What passed

- V6 creates a DirectionLock generation master of at least 512×512 while
  retaining the 256×256 delivery derivative. A paid video request fails closed
  if the master is missing, changed, or undersized.
- The Provider contract uses the generation master, 720p, and four independent
  direction videos. The fixture supplies three repeated 12-phase cycles with
  explicit opposing contacts and passing poses.
- `gait-cycle@1.0.0` accepts only a 700–1,200 ms ordinary complete gait before
  loop closure. Non-game-ready gait evidence now blocks Pack export directly.
- `playback-cadence@1.0.0` fixes all walks at 8×100 ms and idle at 8×200 ms.
  The generated debug manifest, decoded GIF, Pack metadata and an actual Godot
  `SpriteFrames` resource agree within 10 ms.
- V5 remains at its public idle 6 FPS / walk 8 FPS contract and does not gain
  V6 frame-duration metadata.
- Pack validation rejects mismatched duration counts, missing or duplicate gait
  directions, non-monotonic/out-of-range phase indices and non-game-ready gait
  evidence.
- A frozen real V5 Job replayed through V6 with plan 0/0, observed requests 0,
  no credential access and no Pack on the required blocked direction.

## Frozen real-video result

Source Job: `605d08af-c5a3-4a32-98b7-849332dcce7d`.

- `walk_right`: `game_ready`, 917 ms.
- `walk_down`: `game_ready`, 1,000 ms.
- `walk_up`: `blocked`, 750 ms, reason
  `gait_foot_lobe_count_exceeded`.
- Pack: not emitted, as one required direction was blocked.
- Two independent local replays produced byte-identical gait, motion,
  animation-quality and loop reports.

Machine-readable evidence:
[`summary.json`](artifacts/forge-topdown-video-cycle-v6-offline-20260809/summary.json)

## Commands

```text
cargo test -p core quality::gait_cycle
cargo test -p pack --test pack_tests
cargo test -p providers --test video_cycle_generation_contract fixture_v6_full_generation_uses_one_direction_lock_and_four_master_videos -- --exact --nocapture
cargo test -p providers --test locked_frames_generation_contract -- --nocapture
cargo test -p providers --test video_cycle_generation_contract frozen_real_v5_job_replays_through_v6_without_credentials_or_media_requests -- --ignored --exact --nocapture
```

The real-Provider gate remains separate: first authorize one V6 `walk_right`
video, inspect its contact/passing semantics and playback in Godot, and only
then consider a four-direction paid run.
