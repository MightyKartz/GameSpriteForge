# Codex external keyframes → Godot 4.6 implementation plan

Date: 2026-08-17

## Decision

Forge treats Codex subscription image generation as an external candidate
authoring surface, not as a callable Provider API. The production boundary is
therefore a zero-request local intake:

`independent PNG candidates → deterministic validation/normalization → atlas + Pack → Godot 4.6`

Provider-generated 2×2 direction/action sheets and V10 video cycles remain
historical or experimental routes. A sheet is permitted as a deterministic
export/review artifact, never as the content authority for this workflow.

## Workflow contract

`topdown-external-keyframes@11.0.0` is available only through
`PrepareCharacterPackRequest` (`forge plan prepare-character`). It requires:

- a printable character prompt and explicit top-down camera profile;
- one transparent, non-looping idle PNG for down/up/right;
- four transparent, independently authored walk PNGs for down/up/right;
- either explicit left idle/walk frames or a declared right-to-left mirror;
- identical square source canvases and distinct frame SHA-256 values;
- no sprite-sheet, video, opaque/checkerboard, or Provider input;
- zero estimated and maximum Provider requests.

Existing shared-canvas normalization performs translation only. It does not
rescale individual frames, and all frames share one feet anchor. Character
direction, motion, equipment and silhouette gates run before Pack export.

## Runtime contract

Character manifests and `assets/godot_import.json` carry
`godot-sprite-rendering@1.0.0`:

- `textureFilter`: `nearest` or `linear`;
- `pixelSnap`: boolean;
- `mirrorPolicy`: `auto`, `explicit_left`, or `mirror_right_to_left`.

New external-keyframe requests must not use `auto`. Pack validation requires
the manifest and Godot helper declarations to match and verifies the required
animation names. Pixel-snapped anchors must be integral.

The Godot 4.6 installer uses external atlas PNG resources and clipped
`AtlasTexture` regions. Nearest/pixel-snapped sprites use `centered=false` and
place the frame at `(-anchor.x, -anchor.y)`, making the scene origin the feet
without half-pixel centering. Smooth sprites retain centered positioning and
linear filtering. `forge_usage.json` records explicit-left or mirrored-left
direction playback; gameplay movement and collision remain outside Forge.

## Compatibility

- Historical manifests/helpers without `rendering` remain valid.
- Existing generated workflows default to their StyleLock sampling.
- `auto` mirror policy resolves to explicit left when left animations exist,
  otherwise to right-to-left mirroring when the right idle/walk pair exists.
- Existing Pack timing and external-resource size/security gates are unchanged.

## Acceptance

- plan validation tests for valid external input and missing mirror policy;
- Pack tests for matched/mismatched rendering declarations;
- direction playback tests for explicit and mirrored left;
- complete CLI product test with Godot 4.6 headless install;
- format, Clippy, workspace tests and credential-safe offline evidence.

