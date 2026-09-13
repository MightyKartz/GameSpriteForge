# v0.4.0 native delivery showcase

[native-delivery.mp4](native-delivery.mp4) is a 7.2-second native Godot capture
with sound. [native-delivery.gif](native-delivery.gif) is its compact silent
preview; [native-delivery.png](native-delivery.png) is a cover frame.

Six existing public [prop PNGs](../sprites) are freshly imported with the
packaged v0.4.0 CLI as a static prop Pack. Two revisions use the same source
bytes with 128 px and 256 px normalized canvases. The 256 px revision is
installed into a new Godot project, and the presentation instantiates its
actual generated prop scenes.

The original three-note chime is synthesized deterministically by Python's
`math` and `wave` modules. It is a synthetic demonstration sound, not AI music
or a recording. Forge prepares its WAV as an audio Pack, then installs a native
`AudioStreamWAV`. Godot's Movie Maker records that installed stream playing
three times. The displayed waveform describes the source WAV; its cursor
follows native playback. The MP4 contains AAC-encoded audio from this capture.

This is an authored asset presentation in Godot, not Forge UI or gameplay.
The static props do not contain animation. No image generation, audio models,
Forge Provider requests, credentials or private Sword resources are used.

## Reproduce

From a Forge checkout on macOS, with Python 3 and a graphical session:

```bash
python3 docs/media/showcase/v040/render.py \
  --forge /absolute/installed/bin/forge \
  --godot /Applications/Godot.app/Contents/MacOS/Godot \
  --ffmpeg /absolute/installed/share/versions/v0.4.0/bin/ffmpeg \
  --work-dir /absolute/new/showcase-work
```

Pass the public Forge launcher without resolving its symlink. The work directory
must not exist. `ffprobe` must be beside the selected FFmpeg; the macOS packaged
FFmpeg provides the `h264_videotoolbox` presentation encoder. Godot 4.6.x is
installed separately. The script stages its encodes in the isolated workspace
and replaces these three published media files and `provenance.json` only after
all preparation, delivery and media checks pass.

[render.py](render.py) writes isolated Job and Plan stores, source copies,
requests, logs, retained Packs and portable receipts under `--work-dir`. It
checks successful execution, zero Provider counts, Pack validation, installation
drift, and receipt verification with the Job store moved away. The offline
`gallery/index.html` includes fresh Pack revisions, public PNG/WAV sources and
an unchanged historical [spells GIF](../sword-spells.gif). The separate
`comparison/index.html` compares the two exact prop revisions. No review
assertions are written. The historical GIF is registered media, not a fresh
v0.4.0 animation import.

[render.gd](render.gd) is the complete authored presentation. Godot records
216 frames at 30 FPS as a temporary MJPEG/PCM AVI. After MP4/GIF/cover encoding
and stream checks succeed, only that task-owned intermediate AVI is removed.
The raw project, stores and logs remain available locally. System fonts may
vary across machines, so a repeated capture is not expected to have identical
media hashes or revision IDs.

## Offline library screenshots

These technical reference screenshots capture Forge's generated offline media
gallery and two exact prop revisions. They preserve the read-only HTML that
`asset preview` creates from the prepared library.

After the reproduction above, generate the archived three-source view:

```bash
forge asset preview --project /absolute/new/showcase-work/library \
  --id source-potion --id source-historical-spells --id source-synthetic-chime \
  --out /absolute/new/showcase-work/readme-gallery --json
```

Use the same verified executable as the reproduction. Open
`readme-gallery/index.html` and `comparison/index.html` in a browser. The committed
captures use Chrome through Playwright CLI 0.1.19 at a 1400 × 1100 CSS-pixel
viewport: a full-page capture for [resource-library.png](resource-library.png)
and a viewport capture for [resource-comparison.png](resource-comparison.png).
The latter intentionally shows the upper comparison area; additional metadata
and previews continue below the viewport.

For this run, byte-identical copies of the generated pages and their media were
served on loopback for capture. No HTML, CSS, page text, review status or source
media was changed for the screenshots. GIF playback can show a different frame
on a subsequent capture. [screenshots.json](screenshots.json) records the selected
resources, page and image hashes, dimensions and capture settings. No human review
assertions were added; the visible review states remain `unknown`.

The browser and capture utility are presentation tools, not Forge dependencies.
When regenerating native media and its library, recapture these screenshots and
update their evidence as well; each run creates new resource revisions.

## Evidence

[provenance.json](provenance.json) records the compiled Forge identity and
binary SHA-256, source and output hashes, new revision/Job IDs, native delivery
and portable receipt checks. Media verification confirms 216 H.264 video
frames at 1200 × 700 / 30 FPS, stereo 48 kHz AAC, and non-silent signal in
each of the three chime windows. These signal checks are not listening review.
The evidence excludes local absolute paths. Full command
outputs and receipts remain in the ignored workspace.

These are technical checks. They do not record human visual, listening or
license approval, or demonstrate a production game/device test. The source
PNGs retain their earlier artwork history and edges; see the
[original provenance](../README.md#forest-artwork-sources-and-processing).
