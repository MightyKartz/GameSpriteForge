# Forge Top-down Direction Pose Chain V7

Status: implemented behind the experimental workflow `topdown-direction-poses@7.0.0`.

## Outcome contract

V7 starts from the immutable full-body front image in `SubjectLock`. It does not ask a model to invent an idle animation and it does not generate a multi-view collage.

```text
SubjectLock front
→ rear DirectionLock edit
→ right DirectionLock edit
→ front/rear/right mid-walk PoseLock edits
→ three repeated-walk image-to-video candidates
→ candidate extraction
→ eight semantic gait phases per direction
→ identity, direction, framing, equipment, silhouette, anchor, motion and loop gates
→ one static idle + three eight-frame walks
→ .gsfpack
→ Godot SpriteFrames
```

Godot receives exactly the data it needs:

- `idle`: one non-looping frame derived from the SubjectLock front image.
- `walk_up`: eight frames, 800 ms playback duration.
- `walk_right`: eight frames, 800 ms playback duration.
- `walk_down`: eight frames, 800 ms playback duration.
- left playback: `walk_right` with `flipH=true`; it creates no Provider request.

The static idle is resized to the actual decoded video canvas before shared normalization. This prevents a 256 px SubjectLock and a 720 px video from being misclassified as character-scale drift while preserving their relative body occupancy.

## Provider boundary and budget

The default full run has eight logical targets:

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

Expected requests: 8. Maximum requests: 16. Each target may be attempted at most twice. No Subject, Style, idle-video, video-edit, explicit-left, or unrelated asset request is part of this workflow.

Direction and pose edits use the same locked Provider, profile and image model. Walk candidates use the same locked video model and 720p image-to-video path. Provider changes, model changes and silent fallback remain forbidden.

## Immutable locks

`source/direction-lock.json` records the front/rear/right delivery anchors and high-resolution generation masters. The front entry is local and has attempt `0`; rear and right are paid image edits.

`source/direction-pose-lock.json` records the three paid mid-walk poses, their generation masters, DirectionLock input hashes, assessments, Provider/model identity and Subject/Style provenance.

Generation masters remain in JobStore only. Portable Packs remove generation-master paths, copy delivery images into the Pack and expose:

```text
character-direction-lock.json
character-direction-pose-lock.json
assets/direction-lock/*.png
assets/direction-pose-lock/*.png
```

## Local quality gates

Before a paid video request, each DirectionLock and PoseLock must pass:

- full-body foreground and both feet present;
- expected cardinal facing;
- fixed camera and safe framing;
- scale, top and foot-baseline comparison to its direction anchor;
- immutable Subject, palette, equipment and effect contract;
- minimum 512 px generation master.

After video generation, Forge performs only local work:

- full-clip extraction at up to 12 FPS and 96 candidates;
- chroma/alpha cleanup;
- provisional alignment;
- complete gait-cycle detection with opposing contacts and passing phases;
- exactly eight selected source indices;
- fixed 800 ms walk timing shared by preview, Pack and Godot;
- direction semantics and no-turn validation;
- identity/palette/foreground-scale consistency;
- upper-body silhouette and edge-flicker checks;
- body-center and foot-anchor drift checks;
- duplicate limb, ghost foot, afterimage and effect checks;
- loop closure and transition continuity.

The explicit gait-cycle report is authoritative for side-view contact ordering. The generic motion-semantic phase heuristic may not reject a side view solely for `walk_phase_order_invalid` when `gait-cycle@1.0.0` independently proves a game-ready cycle with phase-order score at least `0.80` and at most two foot lobes. All other motion failures remain hard failures.

## Retry and replay

- `video` or `auto`: reuse immutable DirectionLock/PoseLock and regenerate only the selected walk video.
- `loop`, `matting`, `consistency`: reuse all Provider media and run with zero requests and no credential health check.
- `still` and `frame`: rejected for V7; direction and PoseLock mutation needs a future explicitly budgeted contract.
- Every retry creates a child Job. Hash mismatches, missing masters or changed Subject/Style/Provider/model fail closed.

## Offline acceptance

The fixture contract verifies:

- exact 8/16 plan estimate;
- two direction edits, three PoseLock edits and three 720p videos;
- no Subject/Style/video-edit request;
- one static idle plus three eight-frame walk animations;
- gait, direction, consistency, silhouette, anchor and loop gates;
- portable DirectionLock and PoseLock provenance;
- `.gsfpack` validation;
- external-texture Godot installation and headless project load;
- local consistency replay with an unauthenticated Provider and zero requests.

Real xAI generation is deliberately outside this implementation Goal. A real gate should begin with one `walk_right` probe using a fresh V7 Direction/Pose chain. Only after visual review should all three walk videos be authorized.

