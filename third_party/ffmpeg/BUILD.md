# Bundled FFmpeg build

Forge v0.2 pins FFmpeg 8.1.2 for the macOS Apple Silicon release.

The release workflow downloads the unmodified source archive from:

```text
https://ffmpeg.org/releases/ffmpeg-8.1.2.tar.xz
```

Pinned source SHA-256:

```text
464beb5e7bf0c311e68b45ae2f04e9cc2af88851abb4082231742a74d97b524c
```

Configuration:

```text
./configure \
  --prefix=<release-staging> \
  --arch=arm64 \
  --target-os=darwin \
  --disable-gpl \
  --disable-nonfree \
  --disable-doc \
  --disable-debug \
  --disable-ffplay \
  --enable-static \
  --disable-shared
```

The distributed `ffmpeg` and `ffprobe` are separate helper programs invoked as
subprocesses; Forge does not link their libraries. Every GitHub Release must
also publish the exact FFmpeg source archive, this build description, and the
applicable LGPL license text. Release is blocked if `ffmpeg -version` reports
`--enable-gpl` or `--enable-nonfree`.

This file records engineering policy, not legal advice. A distribution-license
and codec-patent review remains a commercial release gate.

## Windows helpers

The pinned Windows recipe is [build-windows-ffmpeg.ps1](../../scripts/build-windows-ffmpeg.ps1).
It disables library autodetection and explicitly enables zlib and Media Foundation
(`--enable-mediafoundation --enable-d3d11va`). The pinned Media Foundation wrapper
requires D3D11VA context support at compile time. It exposes Windows H.264 encoding through
`h264_mf`; it does not add x264. CI verifies native MP4 export against these exact
helpers, then repeats preview acceptance through the installed portable launcher.
See the [Windows portable guide](../../docs/releases/windows-portable.md) for
source hashes, licensing inventory and operating-system requirements.
