# Forge v0.4.0 bilingual README and showcase refresh

## Scope and identities

The English and Chinese READMEs now introduce project resource libraries,
static/animation/layered/audio processing, native Godot delivery and version
consumption. Companion guides identify capabilities included in default v0.4.0
and retain the experimental Windows/character and optional source-feature bounds.
The embedded guide source is corrected for subsequent builds; published v0.4.0
archives are unchanged.

The showcase uses the clean published macOS v0.4.0 launcher at source commit
`62787fd244b7673b9c558ef34bf7113a180d6eae`, default features, release profile.
Its binary SHA-256 is
`697984ce84579528cbcaa924ff9608d460773d5c0d76afe97f60702afce416ad`.
Godot is 4.6.3. The separate local build used to verify the updated guide reports
that same base commit with `dirty: true`, default features and release profile;
its binary SHA-256 is
`49934d1f64a74646c5e0096ce978959c288cc050845765bdc707c202ad0c3448`.

## Documentation and embedded guide checks

- Both READMEs render successfully through GitHub's Markdown API.
- Both languages contain the same six image references and identical shell
  commands apart from translated comments. All 119 local links in the reviewed
  README/guide/showcase set resolve.
- The five-command library quickstart runs successfully with the published
  launcher and a public PNG in an isolated directory.
- `cargo build --locked --release -p forge-cli --no-default-features` succeeds.
- `scripts/test-cli-skill.py` passes all 33 cases / 143 command calls on the rebuilt
  CLI, including all 10 guide resources, standalone reads, safe installation,
  upgrade backups and zero-Provider local plans. No personal skill is installed.
- Python compilation, `git diff --check`, media/recipe hash comparisons and a
  separate read-only review of the full change pass.

The guide test's bundle content hash is
`e9ec3bc04ec3eacb86e29846e5b949d7d55798776d8453ca784efdad91e374aa`.
Local evidence remains under `target/qa/readme-v040-guide`,
`target/qa/readme-quickstart-oyhdda0c` and `target/qa/readme-v040-browser`.

## Native and browser media

The [reproducer](../media/showcase/v040/render.py) uses only public repository
PNGs, an unchanged public historical GIF and a deterministic synthetic chime.
Two fresh prop Packs (128 px and 256 px canvases) and one audio Pack validate.
The 256 px prop Pack and audio Pack install into an isolated Godot project;
14 static and 4 audio installed files verify. Both portable receipts verify
with the original Job store temporarily moved away. All three production Jobs
report zero Provider requests. Raw execution evidence is retained under
`target/qa/readme-v040-demo/run3`.

The native scene instantiates the delivered prop scenes and plays the installed
`AudioStreamWAV` three times. Godot Movie Maker records the video and actual
engine audio. The MP4 is 7.2 s, 1200 × 700, 216 H.264 frames at 30 FPS, with
stereo 48 kHz AAC. Each cue window contains signal (peak −15 dB). The silent GIF
is 900 × 525 at 15 FPS. The cover and distributed capture frames were visually
inspected. Chrome plays the MP4 to its 7.2 s end with no media error; that browser
check is muted and does not constitute human listening approval.

The real generated gallery and revision-comparison pages were copied unchanged
and served on loopback for Chrome/Playwright capture. Their screenshots show
actual unknown review states. No product HTML/CSS or media was altered for the
screenshots. The comparison capture is the upper viewport, as documented.
The only console error observed was an absent favicon from the temporary server;
all referenced media loaded. The five new media files total 3,714,221 bytes.

See [native provenance](../media/showcase/v040/provenance.json),
[screenshot provenance](../media/showcase/v040/screenshots.json) and
[reproduction notes](../media/showcase/v040/README.md). No private Sword files,
credentials or personal paths are published, and no human review assertions are
created. The older Sword GIFs remain historical prototype presentations.

## GitHub metadata

The intended bilingual repository description is:

> A CLI to organize, process, review and deliver 2D game art and audio to Godot. 面向 AI 协作开发的 2D 游戏资源工具链：整理、加工、审核并交付美术与音频到 Godot。

README and guide changes are submitted together in a PR. Repository description
updates are separate GitHub metadata and do not require a release rebuild.
