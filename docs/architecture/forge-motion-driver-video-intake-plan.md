# Forge motion-driver video intake plan

Date: 2026-08-19

## Decision

Introduce a Provider-independent boundary before any browser/API video enters
cycle discovery. A motion contract describes the intended action; a candidate
lock materializes and hashes one MP4 plus its prompt, identity anchor, model,
task provenance, and quota evidence. Native PTS decode and all downstream
replay perform zero Provider requests.

```text
MotionDriverLock
→ VideoCandidateLock
→ locked-video-native-source@1.0.0
→ gait-cycle / source-cycle-sampling
→ FrameRonin-style matting and translation-only normalization
→ QA / Godot / optional Pack after human approval
```

## Contracts

- `motion-driver-lock@1.0.0` records action, direction, source kind, immutable
  identity anchor, optional driving/pose media, duration, cycle count, fixed
  camera/root, ordered gait phases, and foot-clearance envelope.
- `video-candidate-lock@1.0.0` records acquisition route, Provider/model,
  optional task id, exact request count, prompt/driver/video hashes, ffprobe
  facts, and optional browser quota before/after evidence.
- Browser session secrets, cookies, authorization headers, and temporary media
  URLs are forbidden.
- Import refuses non-empty destinations and never mutates the source file.

## Rollout

1. Use retained V10 `walk_down` as a zero-request fixture.
2. Decode every original source frame with native PTS.
3. Reuse the existing gait-cycle and adaptive source-cycle selector.
4. Add standard matting/normalization orchestration and isolated Godot proof.
5. Only then consume Vidu free/rewarded calls and one Kimi member-quota call.
6. Keep official paid Seedance and production Pack outside the authorized
   scope until browser candidates are evaluated.

## Implemented result

The rollout is implemented. Browser candidates now use the following locked
order:

1. materialize and hash the downloaded MP4;
2. decode every source frame allowed by the locked probe count and preserve
   its original presentation timestamp;
3. bound only the analysis copy to 256px, remove border-connected background
   and detached non-subject components, and select one native gait cycle;
4. run adaptive 8/10/12-frame reconstruction without inventing intermediate
   poses;
5. reopen those exact high-resolution source frames, repeat deterministic
   cleanup, then apply integer translation-only anchor normalization;
6. run gait, Motion, same-direction Identity, and loop reconstruction gates;
7. write an isolated Godot project with source-PTS-derived frame durations.

Important corrections discovered during the browser run:

- native extraction uses the candidate probe count rather than a fixed
  120-frame ceiling;
- failed gait and sampling decisions still write machine-readable evidence;
- normalization is deferred until after native cycle selection;
- detached watermark/text islands are removed by subject-component cleanup;
- magenta despill handles both red and blue dominant key channels;
- the 12px translation budget is scaled from its 256px calibration canvas to
  the source resolution;
- a blocked gait can produce a diagnostic-only Godot preview, but never a
  production-eligible Pack.
