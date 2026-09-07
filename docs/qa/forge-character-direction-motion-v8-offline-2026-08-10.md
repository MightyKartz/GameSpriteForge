# Forge Character Direction Motion V8 offline acceptance — 2026-08-10

Result: **offline accepted; real xAI acceptance not run**.

Workflow: `topdown-direction-motion@8.0.0`

## What was verified

- The image-lock stage plans 8 expected / 16 maximum requests, performs one
  text-to-image `front_idle` and seven image edits, writes exactly eight typed
  nodes, exports no Pack, performs no video request, and stops for review.
- The approval record binds the source Job, Lock SHA-256, and all eight node
  SHA-256 values. The request/API default is `image_locks`.
- The complete child plans 4/8 requests, accepts only an approved image-lock
  Job, performs zero image requests, and sends four videos from
  `front_walk`, `back_walk`, `right_walk`, and `left_walk` respectively.
- A local `loop` replay plans 0/0, uses `LocalReplayProvider`, performs zero
  media requests, skips credentials, and reproduces all four loop reports
  byte-for-byte.
- V8 candidate extraction retains decoded source PTS. The 12.5 FPS fixture
  remains 80 ms per frame instead of being rewritten to 83/84 ms.
- The selected closed interval is exported contiguously without its duplicate
  end-boundary frame. Playback speed ratio remains 1.0.
- V8 normalization uses one anchor per animation, so natural source body motion
  is not erased by per-frame body-bottom alignment.
- The Pack contains eight named animations, portable Lock/approval evidence,
  and a sanitized Provider manifest with no absolute JobStore paths.
- Godot 4.6.3 imports ordinary external textures and `SpriteFrames`; all eight
  animations load, left uses `walk_left`/`idle_left` with `flipH: false`, text
  resources remain below 1 MiB, and no `PackedByteArray` is emitted.
- Legacy V6 and V7 keep their old fixed-FPS extraction path; their focused
  contracts remain green.
- A real `target/debug/forge` fixture run completed the public CLI sequence with
  no pre-existing SubjectLock: image locks → `job review --accept` → complete
  with `--image-lock-job` → four-request Pack.

## Gates

```text
cargo fmt --all -- --check                                  PASS
cargo clippy --workspace --all-targets -- -D warnings       PASS
cargo test -p core                                          PASS
cargo test -p forge-cli -p pack                             PASS
cargo test -p providers                                     PASS after V8 PTS isolation
V6/V7 focused compatibility contracts                       PASS
V8 direction_motion_generation_contract (2 tests)           PASS
scripts/test-cli-product.sh                                  PASS
scripts/test-consistency-v2.sh                               PASS
staged public Forge V8 CLI contract                          PASS
Godot 4.6.3 headless Pack import/load                        PASS
credential / temporary-path / embedded-image scan            PASS
```

The full Provider run contains one intentionally ignored test that requires a
retained real-xAI V5 Job and explicitly performs no Provider request.

## External gate

No real Provider call was authorized or made during this implementation. V8
must remain experimental. The next paid gate is one separately authorized
`walk_right` probe using an approved eight-image Lock. A four-direction run may
only follow visual acceptance of that probe.

Machine-readable summary:
[`artifacts/forge-character-direction-motion-v8-offline-20260810/summary.json`](artifacts/forge-character-direction-motion-v8-offline-20260810/summary.json)
