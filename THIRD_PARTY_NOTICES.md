# Third-party notices

The macOS CLI release includes separate `ffmpeg` and `ffprobe` executables from
FFmpeg 8.1.2, built without `--enable-gpl` and without `--enable-nonfree`.
FFmpeg is licensed under LGPL 2.1 or later when GPL components are not enabled.

- Project: https://ffmpeg.org/
- License guidance: https://ffmpeg.org/legal.html
- Exact source: published beside each Forge binary release
- Build configuration: `third_party/ffmpeg/BUILD.md`

Rust dependency licenses are recorded in the release SBOM.
