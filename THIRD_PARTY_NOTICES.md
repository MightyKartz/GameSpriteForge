# Third-party notices

The macOS CLI release includes separate `ffmpeg` and `ffprobe` executables from
FFmpeg 8.1.2, built without `--enable-gpl` and without `--enable-nonfree`.
FFmpeg is licensed under LGPL 2.1 or later when GPL components are not enabled.

- Project: https://ffmpeg.org/
- License guidance: https://ffmpeg.org/legal.html
- Exact source: published with each Forge binary release, in
  `forge-source-and-notices.zip` for consolidated releases (older releases
  provide separate attachments). This archive also contains license texts,
  build recipes, the Rust dependency SBOM and paired-package verification.
- Build configuration: `third_party/ffmpeg/BUILD.md`

Rust dependency licenses are recorded in the release SBOM. The current SBOM is
generated for the macOS target; it is not a Windows-target dependency inventory.
