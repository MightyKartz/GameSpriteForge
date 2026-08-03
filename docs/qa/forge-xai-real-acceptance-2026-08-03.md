# Forge xAI real-model acceptance — 2026-08-03

## Decision

The real-model gate is **pass for Style Lock, Icon Set, Prop Set, Pack validation,
and Godot 4.6.3 delivery**. The current Character Pack gate is **not passed**:
all four directional stills clear `consistency@1.1.0`, but `walk_right` remains
`needs_cleanup` because its eight-frame loop joins two visibly different gait poses.
Forge correctly withheld the Character `.gsfpack`; it was not manually forced through.

## Environment and authentication

- Workspace CLI: `target/debug/forge`, signed as `dev.gamespriteforge.cli` under
  Team ID `J6P96F432P` before each xAI credential read.
- Provider/profile: `xai/default`, Preview OAuth Device Code.
- Provider doctor: authenticated with image, edit, video, image-to-video,
  reference-to-video, cancellation, and usage capabilities.
- The existing OAuth grant was read once without a repeated Keychain prompt.
- Engine: Godot `4.6.3.stable.official.7d41c59c4`.

## Real generation results

| Asset | Job | Result | Evidence |
| --- | --- | --- | --- |
| Style Lock | `54d75b96-68f3-4202-96d0-cf0eaeac5571` | Passed | Revision `5e19c859eb566af5`; board SHA-256 `f4d33c626d34f3f8a94c9f39c5ee4c137c86c8145a0c99765216185591231f90` |
| Icon Set | `c3e3c91a-1764-4cb9-aa58-e2cc8d7d3e69` | Passed after explicit gray-band review | `leaf-potion` and `moon-key`; Pack SHA-256 `a3fadcd2af211bf5777b5b6578aacec1e4432cefc077919059470201eef49a32` |
| Prop Set | `50774f9d-2ca7-457d-8a4b-9609045d9671` | Passed after explicit gray-band review | `ranger-chest` and `forest-lantern`; Pack SHA-256 `d5a3d1d3a1357cf71a7e7c96081e95a4f01e47031aaf0d13c4e8411667a9f877` |
| Character | `f152fe7e-3f0c-4f1c-abeb-cc7c3e89dd6b` | Release blocker | Directional consistency passed; `walk_right` loop score `0.72718287`, overall animation verdict `needs_cleanup`; no Pack exported |

The first real Icon Set run, `53c4941d-148d-4e80-9277-089921f75e2f`,
correctly stopped before export. It exposed exact-bin palette matching and
area-based foreground scale as false negatives. The initial Character run,
`5130f64c-65e4-4386-af7d-a168a6322d16`, also correctly stopped and exposed a
four-subject `walk_right` collage that the old edge-density-only gate had treated
as game-ready.

## Visual evidence

- [Style board](artifacts/forge-xai-real-acceptance-20260803/project/.forge/styles/5e19c859eb566af5/style-board.png)
- [Icon contact sheet](artifacts/forge-xai-real-acceptance-20260803/jobs/c3e3c91a-1764-4cb9-aa58-e2cc8d7d3e69/contact-sheet.png)
- [Prop contact sheet](artifacts/forge-xai-real-acceptance-20260803/jobs/50774f9d-2ca7-457d-8a4b-9609045d9671/contact-sheet.png)
- [Final character directional contact sheet](artifacts/forge-xai-real-acceptance-20260803/jobs/f152fe7e-3f0c-4f1c-abeb-cc7c3e89dd6b/contact-sheet.png)
- [Blocked walk-right animation review GIF](artifacts/forge-xai-real-acceptance-20260803/walk-right-final-review.gif)
- [Final character consistency report](artifacts/forge-xai-real-acceptance-20260803/jobs/f152fe7e-3f0c-4f1c-abeb-cc7c3e89dd6b/consistency-report.json)
- [Final character animation quality report](artifacts/forge-xai-real-acceptance-20260803/jobs/f152fe7e-3f0c-4f1c-abeb-cc7c3e89dd6b/animation-quality-report.json)

## Acceptance-driven implementation changes

1. `consistency@1.0.1` replaced exact RGB-bin intersection with weighted soft
   palette transport and replaced bounding-box area with longest foreground extent.
   The public `0.70/0.55` thresholds were not lowered.
2. `consistency@1.1.0` added major-subject counting. More than one independent
   component occupying at least ten percent of foreground is a hard, non-reviewable
   failure. Character edge density is now relative to the normalized character
   reference instead of the textured Style Board scene.
3. Failed Character jobs now persist Provider manifests before returning. Targeted
   retry can also reconstruct a minimal hash-checked manifest from older Job artifacts
   when the manifest is missing.
4. Auto-corner chroma matting now flood-clears border-connected green-screen color.
   A regression fixture proves that a gradient green background is removed while
   enclosed teal and green character details remain opaque.
5. Godot installation derives provider-level provenance from Pack metadata when the
   plan does not supply explicit Provider asset IDs. `forge_usage.json` and
   `.forge/assets.json` now retain `provider: xai` for these real static Packs.
6. Every Provider operation now writes a hashed `provider-usage.json`, including
   successful, review, cancellation, and failure outcomes.

## Pack and Godot verification

- Both static `.gsfpack` directories passed `forge pack validate` and `asset inspect`.
- Installation jobs `46173bb1-4eaf-4193-9251-8cdfd89ae194` and
  `904a966e-3bed-48b8-81ea-6c1a9455037f` succeeded through independent one-use plans.
- Godot 4.6.3 imported all four external PNGs and loaded the clean project headlessly.
- Prop scenes are 150 and 152 bytes; no `.tres` or `.tscn` reaches 1 MiB.
- No installed resource contains `PackedByteArray` or embedded `Image` subresources.
- Icon usage maps to `Texture2D`; Prop usage maps to per-item `Sprite2D` scenes.
- Both usage files and project manifest contain `provider: xai`.

## Security and cost audit

- JobStore, project, Pack, and Godot text/log scans found no Authorization header,
  Bearer value, access token, refresh token, device code, or xAI API key pattern.
- The only persisted HTTP host in logs is the fixed Godot banner URL
  `https://godotengine.org`; no xAI temporary media URL was persisted.
- Two recorded Character targeted retries cost USD 1.28 each, so the auditable cost
  lower bound is **USD 2.56**.
- Exact total cost cannot be reconstructed for this run because Style/static and the
  first Character job predated the new generic usage artifact. The missing audit trail
  is fixed for future runs; this report intentionally does not estimate a total.

## Regression verification

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- Full Rust suite: 106 tests passed before the final deterministic fixture calibration;
  the complete Provider suite passed again afterward.
- `scripts/test-cli-product.sh`: passed after recalibrating the offline gray-band fixture.
- Fixture coverage includes Style/Character/Icon/Prop generation, targeted retry,
  missing-manifest recovery, review, cancellation, usage recording, Pack validation,
  Godot install, provenance, and credential-pattern scanning.

## Remaining release gate

Do not publish the Character consistency claim from this run. The next implementation
must make the xAI video stage loop-aware rather than lowering the quality threshold:

1. strengthen the video prompt and response contract to require one subject and a true
   seamless in-place loop;
2. add deterministic candidate-range selection that physically exports the selected
   frames, not merely changes the quality score;
3. rerun three clean Character generations with the new `provider-usage.json`, require
   all four animations to be `game_ready`, then install and load the Pack in Godot.
