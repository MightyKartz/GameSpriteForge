# Forge topdown-video-locked V5 real-remediation plan

Date: 2026-08-09

## Goal

Repair the deterministic stages exposed by the first real `topdown-video-locked@5.0.0` acceptance without lowering existing hard gates or spending additional Provider requests.

Frozen source Jobs:

- `e621d016-1303-457b-995d-6f139f3b41b6`
- `c5809f1f-94ab-4c1e-b6a3-0cd1878f88f1`

## Changes

### 1. Anchored loop selection

- Add `loop@2.1.0` as an opt-in anchored extension of `loop@2.0.0`.
- For direction-locked idle video, candidate selection must start inside the initial DirectionAnchor window and meet a minimum anchor similarity.
- Record anchor frame, similarity, allowed start window and reasons in the loop report.
- Legacy workflows continue calling the unanchored `loop@2.0.0` API.

### 2. Selected-interval direction persistence

- Upgrade Character direction diagnostics to `direction-quality@1.2.0`.
- Assess every exported frame with the expected front/rear/right direction anchor classifier.
- Record matching-frame count/ratio and maximum consecutive mismatch run.
- Modern explicit-camera workflows block intervals that drift direction even if an aggregate visible-face ratio happens to pass.

### 3. Ordinary-walk semantics

- Upgrade motion semantics to `motion-semantics@1.1.0`.
- Enable the existing gait/phase/flicker/foot-lobe gate for locked-video V5.
- Add a maximum lower-body dynamic-degree guard so a running or kicking sequence cannot satisfy only the minimum-motion checks.
- Motion replay remains local and produces no Provider request.

### 4. Foot-plane cleanup

- Upgrade background cleanup to `keyframe-background-cleanup@1.3.0`.
- After matting and baseline cleanup, remove only small, exposed, line-like or sparse chroma-green residual components in the lower foot plane.
- Preserve moss-green clothes and even saturated-green boots by keeping broad or vertically substantial character components.
- Report removed pixels and fail if a residual remains.

### 5. Retry-sensitive action consistency

- Add V5 action consistency as `consistency@1.7.0`; legacy Character and keyframe paths retain `consistency@1.6.0`.
- Keep the pre-video DirectionLock consistency check as a paid-request guard.
- After loop selection and shared normalization, overwrite the exported consistency report with metrics computed from the actual eight action frames and their direction anchor.
- A child replay with changed frames must produce a different report hash; unchanged animations remain byte-for-byte reusable.

## Release gates

- Synthetic tests cover closed-eye idle subsequence selection, interval direction drift, excessive walk motion, foot-plane chroma residue and retry report invalidation.
- Zero-cost replay of both frozen real Jobs performs no Provider request and blocks the known bad idle/walk/down candidates for explicit reasons.
- Fixture V5 still produces 32 frames, a valid Pack and loadable Godot 4.6 resources.
- Existing Character V1/V2, keyframes, spritesheet, static assets and world tests remain green.
- No credential, authorization header or temporary media URL enters reports or Packs.

No real xAI generation is authorized by this remediation plan.
