# Animation preview delivery plan

## Problem and decision

GIF's indexed colors and binary transparency cannot represent soft game effects.
Forge also retained prior GIF frames and ignored native per-frame durations.
Browser review must use original PNGs; shareable video is a separate derivative.

1. Repair compatibility GIF disposal and per-frame delays. Explicitly threshold
   alpha for transparent GIFs and disclose that soft transparency is lost.
2. Add a shared, validated flat-Pack animation reader. Honor frame indices,
   repeats, per-frame durations, loop flags and the original shared canvas.
3. Use this reader in offline resource review: original PNG bytes, animation
   selection, play/pause, frame stepping and dark/light/checkerboard backgrounds.
   Keep a strict CSP and escape all metadata. No network or catalog mutations.
4. Add `forge pack preview --path PACK --out VIDEO.mp4`, with animation and
   background options, plus an optional content-addressed cache. Composite PNGs
   directly and encode H.264 MP4 through existing FFmpeg discovery. Report the
   encoder, source identity and native versus encoded timing. Use 60 fps for
   broad playback compatibility; video timing is quantized to that frame grid.
5. Verify synthetic alpha, moving silhouettes, nonuniform timing, reordered
   frames, input/output integrity, cache invalidation and malicious metadata.
   Exercise actual FFmpeg, offline browser playback and Godot 4.7.2 locally;
   use existing macOS/Windows CI for portable Rust contracts.
6. Update English/Chinese user guidance and record reproducible QA evidence.
   Push one reviewable PR; do not merge or release in this task.

## Compatibility and limits

Existing v1/v2 Pack contracts require `previews/preview.gif`; this PR keeps that
compatibility artifact and makes it secondary in the review experience. Removing
it from newly written Packs needs a separately versioned format migration.
No Pack schema, game resource, catalog revision or consumer pin is rewritten.
MP4 has a baked background and is not a transparent game asset. Browser timing
depends on display refresh/scheduling; frame stepping inspects exact PNGs.
Layered/world/audio Packs retain their existing viewers; native Godot remains
the reference for engine blending, tracks and runtime appearance.

GIF delay units are 10 ms and viewers may clamp short delays. Transparent GIF
uses a 128 alpha threshold; checkerboard GIF composites before quantization.
Neither can reproduce all PNG colors. Existing stored GIFs are immutable and
are not silently regenerated; the new PNG player works with legacy flat Packs.
MP4 export requires an available H.264 encoder (platform encoder or libx264 in
the user's FFmpeg); missing tools/encoders must fail clearly, never relabel GIF
bytes or install/download software automatically.
