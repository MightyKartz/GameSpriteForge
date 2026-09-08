# README and Sword showcase verification — 2026-09-08

The English and Chinese READMEs now start Codex users with `forge guide`, omit
skill installation instructions, and show existing Sword spell and enemy assets.
The CLI and its embedded guide are unchanged; this does not require a release.

## Published media

| Preview | Contents | Dimensions | Length | Size |
| --- | --- | --- | --- | --- |
| [Spells](../media/showcase/sword-spells.gif) | Fire, frost, lightning | 1040 × 440 | 4.2 s | 475,501 bytes |
| [Enemies](../media/showcase/sword-enemies.gif) | Wisp, stone golem, vine spirit, guardian slam | 1040 × 440 | 4 s | 770,428 bytes |

These are separate Godot asset presentations, not Sword gameplay or device
recordings. The art came from prior Codex image generation and Forge local
processing. No new art, video-model generation, Provider requests or Forge
imports were performed. No Sword game source or full source sheets are published.

## Verification

- Rehashed all seven selected source images and installed textures. Recomputed
  the seven original Pack directory fingerprints using the historical Forge
  `forge-directory-hash-v2` algorithm; they match the original receipts and
  installed usage metadata. Original build and review states are preserved in
  [provenance](../media/showcase/sword-provenance.json).
- The render snapshots' 21 input files match the recorded texture, SpriteFrames
  and usage hashes. Those Sword files have identical hashes after capture.
- Imported and rendered the isolated presentation with Godot 4.6.3; no script or
  resource errors appeared. Captured 210 spell frames and 200 enemy frames.
- FFmpeg 8.0.1 encoded the GIFs at 50 FPS. Decoded every frame, checked dimensions,
  duration, repeat flags and multiple distinct frames. Captures retain source
  frame timing, sampled to 20 ms; presentation phases, one-shot pauses and fixed
  display transforms are documented in the [media guide](../media/showcase/README.md).
- Inspected source captures and decoded GIF stills for clipping, label legibility
  and palette artifacts. This is presentation review, not human approval of the
  game's art or a full animation-quality acceptance test.
- Checked 40 local links across the two main READMEs, matching media references,
  removal of skill installation language and `git diff --check`.

The [compact verification record](artifacts/forge-readme-sword-showcase-20260908.json)
contains output and renderer hashes. Detailed captures and logs remain ignored
under `target/qa/sword-showcase/`; the final run is `render-xxonmuiz`.

## Scope and limits

Static icon/prop and background-removal examples remain on the homepage; the
older composed forest scene is retained in the media directory. Animation is
still labelled experimental. The GIFs introduce no interpolated poses, per-frame
coordinate repairs, gameplay controllers or production-readiness claims.

Validation was limited to the changed documentation and presentation resources.
No Rust/CLI build, full game test suite or new release was needed.
