# Codex walk-right V15.1 semantic stabilization — offline validation

Date: 2026-08-19

Verdict: **candidate C passes `motion-semantics@1.5.0` with zero Provider
requests, zero Alpha changes, and zero lower-body RGB changes.** It is the
selected Godot review candidate, not a production Pack: the hash-locked human
side-laterality approval remains pending.

Experiment root:

`generated-assets/experiments/codex-walk-right-v151-semantic-stabilization-20260819/`

## V1.5 metric correction

The old stable-upper gate aligned by the whole-character bounding box. Contact
strides move that box center by almost 30 px even though Forge had already
aligned the upper body. V1.5 now uses an upper-body Alpha anchor and splits
upper geometry from interior color.

| Sample | Alpha drift | Color flicker | Other blocker |
| --- | ---: | ---: | --- |
| V13 high-knee negative | 0.2439 | 0.4692 | knee/shin excessive |
| V14 normal amplitude | 0.0469 | 0.0610 | none |
| V15 side cadence source | 0.0680 | 0.0915 | color only |

V14 is therefore no longer falsely blocked. V13 remains strongly blocked, so
the corrected alignment did not erase the negative control.

## Zero-request candidates

All candidates use the four exact V15 clean-frame hashes. The processor aligns
their upper bodies, intersects common Alpha and locally uniform interior-color
pixels, erodes the non-rectangular stable core, computes aligned median RGB,
and feathers it back into each frame.

| Candidate | Erosion | Feather | Blend | Core pixels | Color flicker | Verdict |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| A | 8 px | 4 px | 0.65 | 2,222 | 0.0902 | blocked |
| B | 4 px | 3 px | 1.00 | 4,441 | 0.0831 | blocked |
| C | 2 px | 2 px | 1.00 | 6,329 | 0.0748 | game-ready motion |

Candidate C was selected. Across its four frames:

- Alpha changed pixels: `0`;
- lower-body RGB changed pixels: `0`;
- maximum per-pixel RGB sum delta: `55`;
- cadence score: `0.9813`, unchanged;
- opposing contact change: `0.1700`, unchanged;
- knee/shin dynamic degree: `0.7514`, unchanged;
- stable upper Alpha drift: `0.0680`, unchanged;
- stable upper color flicker: `0.0748`, below the `0.08` limit.

No image generation, API, torso splice, rectangle copy, Alpha edit, limb edit,
or Pack mutation occurred.

## Godot 4.6.3 comparison

The isolated project compares source V15 on the left and selected V15.1 on the
right using external PNGs, native `AnimatedSprite2D`/`SpriteFrames`, 4 FPS,
linear filtering, and no pixel snap.

- MovieWriter: 68 frames at 30 FPS, 2.2667 seconds;
- foot baseline drift: 1 px on both sides;
- whole-bbox center range: 29.5 px on both sides;
- subject-height range: 5 px on both sides;
- geometry is byte-identical in Alpha, as expected.

Godot warned that the volume had 5.65 GiB available before recording. The
verified 68-frame AVI/MP4 completed successfully; no recording was truncated.

## Artifacts

- processor: `tools/stabilize_semantic_core.py`;
- source V15 frames: `source/frames/walk_right/`;
- immutable attempts: `candidate-a/`, `candidate-b/`, `candidate-c/`;
- selected frames: `selected/frames/walk_right/`;
- selected motion report: `selected/motion-semantics-1.5.json`;
- selected processing report: `selected/stabilization-report.json`;
- selection record: `selected/selection.json`;
- processing lineage: `processing-lineage.json`;
- transparent selected sheet: `selected/walk-right-v151-transparent.png`;
- selected GIF: `selected/walk-right-v151-animation.gif`;
- Godot project: `godot/walk-right-v151-compare/`;
- comparison MP4:
  `godot/walk-right-v151-compare/qa-output/walk_right_v151_compare.mp4`.

## Verification

- `cargo test -p core motion_semantics -- --nocapture`: 8 passed.
- Godot import, headless runtime, MovieWriter, and MP4 conversion: passed.
- `cargo test -p providers --test keyframe_generation_contract`: 1 passed.
- `cargo test -p pack`: 46 passed across unit/integration tests.
- `cargo clippy -p core --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `cargo build -p forge-cli`: passed.

## Remaining gate

The selected frame hashes differ from V15 because RGB was stabilized. The
pending side-laterality review must therefore be reissued against the V15.1
hashes and approved by the user before `.gsfpack` export or direction
expansion.
