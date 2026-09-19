# Animation preview verification — 2026-09-19

Implementation plan: [animation preview delivery](../architecture/animation-preview-plan.md).

## PR #55 follow-up review

The initial CI run on `92963cb` failed all four native matrix jobs in Clippy,
before reaching video or Godot execution. CI pins Rust 1.98.1, whose
`chunks_exact_to_as_chunks` lint was absent from the earlier local toolchain.
Use fixed-array RGBA chunks rather than suppressing the lint. Local verification
now uses Rust 1.98.1, matching CI.

Review also found a Windows checkout hazard: CRLF in the embedded player changes
its raw-byte hash, while HTML parsing normalizes newlines before CSP validation.
The player is now pinned to LF in `.gitattributes`, and hash calculation follows
HTML normalization even for source archives with CRLF or lone CR. A regression
test covers all three forms and runs in the native matrix.

Rust 1.98.1 workspace/all-target Clippy, the new hash test and the four focused
animation/catalog test suites passed. The native test passed with the published
macOS FFmpeg after adding timing-only cache invalidation coverage. The initial
retiming fixture correctly failed Pack validation because its Godot helper still
had old durations; the fixture now updates all timing declarations consistently.
No Pack validator was weakened. Native Windows acceptance must come from the
latest PR CI run, not the earlier failed jobs.

## Verified build identity

The final CLI smoke check used commit
`2407a64d2f516d74daae7af30248d07881a92666`, a clean checkout, no default features,
debug profile, target `aarch64-apple-darwin`. `doctor --json` reported that exact
identity and `pack_mp4_preview` / `project_asset_png_animation_preview`.
Binary SHA-256:
`34fd4c6af668f110e34c3db70e1e7776879968d034f9388c0ada2672c6c7c9c8`.

Earlier native Godot/browser checks used the same implementation before the
final non-animation Pack guard, at base commit
`7c47397d06f3b8d022ea6b3996dc85e2a1c78ac1`, dirty checkout, default features empty,
debug profile; binary SHA-256
`7f0ea92eeb9fbce07da892e935d12b41429de51735896331b3762da9d1fad0f6`.
The final guard was covered by animation, audio and layered regression tests;
the clean build repeated MP4 export and resource review generation successfully.

## Automated checks

All listed local checks passed on macOS:

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked -p pack
cargo test --locked -p core --lib export::
cargo test --locked -p core --lib library::
cargo test --locked -p core --test animation_preview_tests --test animation_delivery_tests --test animation_pixel_quality_tests --test asset_library_review_tests
cargo test --locked -p core --test audio_processing_tests --test layered_pack_tests
cargo test --locked -p forge-cli --bin forge
cargo test --locked -p core --test animation_preview_tests native_mp4 -- --ignored
python scripts/test-cli-skill.py --forge /absolute/source-build/forge
python scripts/test-asset-library-review-cli.py --forge /absolute/source-build/forge
python scripts/test-local-animation-delivery.py --forge /absolute/source-build/forge --godot /absolute/Godot-4.7.2 --output /new/isolated/output
```

Synthetic preview tests cover reordered/repeated frames, nonuniform timing,
near-invisible RGB, independent GIF disposal, alpha policy, unchanged PNG bytes,
HTML escaping/CSP, output collision and Pack-write rejection. Native MP4 checks
decode frames to verify the correct silhouette without prior-frame accumulation,
probe H.264 dimensions/duration, test odd-canvas padding, cache reuse, background
invalidation and corrupt-cache rejection. The native test is explicitly ignored
in ordinary Rust runs and explicitly executed after media-helper setup in CI.

MP4 integration passed with both local libx264 and the v0.6.2 macOS package's
LGPL FFmpeg / `h264_videotoolbox`. No encoder was downloaded or added to the
release bundle. The helper SHA-256 was
`264ae41bfffc9aede23a8ec74aa05f063aae6a0c0bede34d7593cb44d33b289e`.
Hardware encoder outputs need not be byte-identical across fresh renders; each
cached output is verified against its own recorded digest.

The native Godot script passed its five cases: single animation, character
actions, nearest sampling, legacy Pack and whole-sheet source transform. It
checked saved durations, sampling, anchors and loop flags, with zero Provider
requests. Godot identified itself as `4.7.2.stable.official.ed1daf0bf`; executable
SHA-256 `c7cccbf8fb143e34e02fd6521e09be2c2b974f0d5db080b19071c9c570718ccf`.

## Actual media and browser review

An existing four-frame lightning Pack was reviewed without changing its bytes,
catalog selection, receipts or consumer toolchain. The original GIF was not used
to create either the player or MP4. Its PNG durations were 110/80/130/190 ms.
The new review copied four PNGs and reported no issues. The MP4 was 700 × 700,
31 frames at 60 fps: native 510 ms, encoded 516.667 ms. A second identical request
hit the verified cache; a light-background request used a different cache key.
Private source artwork and generated review/video directories remain outside Git.

- Safari: opened the generated `file://` page, observed playback, paused/stepped
  to frame 3/4 (130 ms), and switched to the light background. The inspected PNG
  showed soft edges without the old GIF's opaque black speckles/accumulation.
- Chrome via Playwright: reviewed the same page over loopback HTTP, exercised
  frame stepping and background selection, and captured a screenshot. The only
  console error was an unrelated missing favicon; no player/CSP errors appeared.

These are visual observations, not an artistic approval or proof of exact
wall-clock browser timing. PNG frame stepping preserves the actual source image;
live playback is subject to browser scheduling. MP4 is lossy, opaque and sampled
on a 60 fps grid. Additive/multiply blending and layered motion require Godot.

## Remaining acceptance

Windows was not run locally. The existing Godot workflow now executes preview
Rust tests and the native MP4 test on macOS and Windows, using published media
helpers. Consult the PR's checks for actual Windows results before merging.
No released package, existing Pack schema, installed consumer or public release
was changed by this task. GIF removal from the Pack format is intentionally a
separate format-version migration.
