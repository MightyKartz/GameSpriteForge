# Forge motion-driver video intake — offline validation

Date: 2026-08-19

Status: Stage A intake, native-source decode, retained cycle replay,
FrameRonin-style matting/anchor normalization, Motion QA, and Godot preview
implemented under the active Goal.

## Fixture

- Source: retained V10 real `walk_down` video.
- Original Job: `f80b6e47-9f58-4d8d-9dc3-4bdd239a9e6c`.
- Source SHA-256:
  `1d1e5d3741af8b78da55f152102aa0ee466c35056fd994d68b87ef59885c30b4`.
- Acquisition in this run: `external_import`.
- Provider requests in this run: `0`.
- Cash/quota mutation: none.

## Results

`VideoCandidateLock` materialized an immutable sibling MP4 and verified:

- canvas: 480×480;
- codec/pixel format: H.264 / yuv420p;
- FPS: 24;
- duration: 4.041667 seconds;
- frame count: 97;
- copied-video SHA-256 exactly matches the retained source.

`locked-video-native-source@1.0.0` decoded all 97 original source frames with
presentation timestamps from 0 through 4000 ms. No uniform-FPS timestamps were
invented and no Provider call occurred.

The retained `source-cycle-sampling@1.0.0` evidence selected 12 original frames
from source indices 19–45 at PTS 792–1875 ms. Auto-corner chroma matting and
integer translation-only stabilization then achieved:

- maximum translation: 12 px;
- residual body-center drift: 1 px;
- residual foot-baseline drift: 0 px;
- stabilization verdict: `game_ready`;
- Motion verdict: `blocked` only for real stable-upper-body color flicker.

This matches the retained V10 finding and demonstrates fail-closed replay: the
new intake did not convert a known quality defect into a false pass.

An isolated Godot 4.6.3 project loaded all 12 external PNG frames, replayed the
source-PTS-derived relative durations with native `SpriteFrames`, and rendered
a 73-frame/2.433-second MovieWriter proof. No Pack or production catalog was
written.

## Artifacts

- experiment:
  `generated-assets/experiments/motion-video-pipeline-stage-a-20260819/`;
- motion lock: `motion-driver-lock.json`;
- candidate lock: `candidate/video-candidate-lock.json`;
- materialized source: `candidate/source-video.mp4`;
- native decode report: `native/native-source.json`;
- native decoded frames: `native/raw/`.
- replay reports and frames: `replay/`;
- Godot project: `godot/`;
- Godot MP4: `godot/qa-output/stage-a.mp4`.

## Verification

- focused motion-video unit test: passed;
- Core example compilation: passed;
- JSON schema syntax: passed;
- Godot import/headless/MovieWriter: passed;
- paid Provider requests: zero.

The browser follow-up is complete in
`docs/qa/forge-motion-video-browser-benchmark-real-2026-08-19.md`.
