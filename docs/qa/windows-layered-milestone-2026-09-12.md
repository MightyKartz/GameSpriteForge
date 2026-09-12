# Windows and layered milestone — 2026-09-12

Implementation is reviewed in PR #32 on `codex/mahjong-cli-integration`.
The source consumer, artwork, pinned Forge binary and locks in the local Mahjong
checkout are read-only inputs and are not included in this repository.

## Local acceptance

- Rust formatting and workspace/all-target Clippy pass with warnings denied.
  Pack validation has 33 passing tests; native static preparation, matting,
  layered pack tampering, image contracts and delivery audit tests pass.
- `test-static-native-matte-cli.py` covers nine CLI categories, including
  rectangular raw PNG preservation, no-clobber outputs and legacy defaults.
- Godot 4.6.3 loads saved native static and layered resources. The unified
  controller passes 39 assertions; actual preview UI signals pass 22 assertions
  for play, pause, reset, clip choice, seeking, speed and displayed state.
- Six Hiyu textures retain their source bytes and SHA-256 hashes on their common
  1254 × 1254 canvas. Ordered CPU RGBA composition equals the original source
  image byte for byte. The consumer's watched source files and pins are unchanged.
- Real Windows GPU acceptance uses RTX 4090 Laptop / OpenGL compatibility, not
  the headless Dummy renderer. Hiyu's Godot render equals its independent layered
  reference exactly; compared with the original PNG its maximum channel error is
  1 (tolerance 2, no out-of-tolerance pixels). Native and exported PCK RGBA
  screenshots are byte-identical.
- A separate animated synthetic fixture changes 714 pixels at its midpoint and
  matches the independent reference exactly. Normal, additive and alpha-aware
  multiply are exercised; 24 multiply math oracles run in native and exported
  PCK projects, including transparent padding, partial alpha and parent opacity.
- Windows packaging development fixtures pass real old-to-new binary upgrade,
  unchanged reinstall, archive/hash/path/launcher tamper rejection, paths with
  spaces, PowerShell 5.1 installation and real MP4 → PNG → GIF helper execution.
  Dirty/debug builds are rejected by the normal release gate.

Local evidence is generated below ignored `target/qa`: `hiyu-layered-01`,
`layered-render-hiyu-01`, `layered-render-self-03`, `preview-ui-02`,
`windows-package-reviewed-01`, and `windows-helper-reproducibility.json`.
Private images and generated packs remain there. Tests are reproducible using
the committed scripts; final clean-build checksums and CI runs are recorded in
the PR and in the downloadable artifact's `BUILD_INFO.json`/`MANIFEST.sha256`.

## CI and limits

The preceding image-contract/Windows baseline passed both jobs of
[quality run 34683325236](https://github.com/MightyKartz/GameSpriteForge/actions/runs/34683325236).
That run predates this complete milestone. The updated quality workflow exercises
macOS and Windows contracts, while `windows-portable.yml` builds the exact PR
head and uploads a checksum-verified package only after real installation,
native import, preview and controller checks pass. GPU/PCK visual evidence is
local because hosted Windows runners do not provide this GPU setup.

Windows distribution remains experimental and unsigned. Matting removes a flat
chroma background; it is not semantic segmentation. Layered v1 is an ordered
flat hierarchy with full transform/opacity keyframes, not a bone/mesh system.
The Hiyu rig has no animation clips, so its acceptance establishes composition
and export; animation is tested with the synthetic fixture. The PCK test checks
exported resources on Godot 4.6.3, not a signed standalone game executable.

See [Windows installation and upgrades](../releases/windows-portable.md),
[native static and matting](../automation/static-native-and-matte.md),
[layered packages](../automation/layered-packs.md), and
[playback and preview](../automation/godot-playback-preview.md).
