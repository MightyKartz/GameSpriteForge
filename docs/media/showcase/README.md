# Product showcase assets

These images support the English and Chinese product READMEs. They illustrate
icons and props, local background removal, and existing Sword animation assets
prepared with Forge and replayed in Godot. The animation previews are prototype
examples; character animation remains in development.

| Image | What it shows |
| --- | --- |
| [gallery.png](gallery.png) | Eight illustrative objects, displayed using actual Forge-matted PNGs. |
| [processing.png](processing.png) | The same potion before and after Forge background removal. |
| [godot.png](godot.png) | An earlier authored Godot scene using those PNGs, retained here but no longer embedded in the main READMEs. |
| [sword-spells.gif](sword-spells.gif) | Sword's existing fire, frost and lightning frames replayed in a dedicated Godot showcase. |
| [sword-enemies.gif](sword-enemies.gif) | Sword's existing wisp, stone golem, vine spirit and guardian slam animations in the same showcase. |

## Sword animation previews

The source artwork was generated with Codex's built-in image tool for the Sword
game prototype. Forge processed those images locally and delivered animation
Packs and Godot SpriteFrames. This showcase uses the installed resources, with
their original frame order, duration weights, source coordinates and anchors.
It makes no new image generation, video generation or Forge Provider requests.

The two GIFs are captures of a **separate asset presentation scene**, not gameplay
or an iPhone recording. Layout, labels, starting phase and repetition between
one-shot effects belong to the presentation. The spells begin at visible phases
so the still thumbnail shows all three effects. Each animation has one constant display transform;
there is no frame interpolation, per-frame alignment or generated motion.
Original non-uniform timing is sampled on a 50 FPS capture clock (20 ms steps),
so frame transitions can be rounded by less than 20 ms. Looping actors retain
their resource loop timing; spells restart after a short gap and the guardian
holds its last frame between demonstrations.

Both captures are 1040 × 440 pixels. Spells run for 4.2 seconds and enemies for
4 seconds before repeating. GIF palette quantization changes display colors;
the full-color installed PNGs remain untouched in Sword.

The [Sword provenance record](sword-provenance.json) preserves source and installed
resource hashes, original Forge build identities, historical Pack validation and
review states. These were earlier imports, not new imports with the current CLI.
Source PNGs and installed textures were rehashed for this showcase; the original
Pack directory fingerprints were recomputed and matched the receipts and installed
usage records. Historical validation and review states are retained; this is not
a new import or visual approval. Prototype reviews do not establish production
readiness or universal character generation quality.

Only the two display GIFs, their provenance and the new presentation scripts are
published here. Sword's source artwork, installed asset sheets, game code and
private Job stores are not included. See the [showcase QA note](../../qa/forge-readme-sword-showcase-2026-09-08.md)
for capture verification.

To reproduce with access to the same local Sword resources, Python 3, Godot 4.6.x,
FFmpeg and a graphical session, run from the Forge repository root:

```bash
python3 docs/media/showcase/render_sword.py \
  --sword /path/to/Sword --godot /path/to/godot --ffmpeg /path/to/ffmpeg
```

[render_sword.py](render_sword.py) checks installed texture and Pack evidence,
copies the selected resources into a temporary project under `target/qa/`, and
runs [render_sword.gd](render_sword.gd) in Godot. FFmpeg encodes the captured PNGs
as GIFs; the script replaces the two GIFs in this directory and writes a local
render receipt. It does not run Sword's importers or modify the Sword checkout.
The original local resources are required; this is not a standalone asset pack.
If source assets change, update their provenance and review the new captures.

## Forest artwork: sources and processing

The source artwork was created on 2026-09-07 with Codex's built-in image generation
tool: one new image and one background edit. The [prompts](prompts.md) record both
calls. This is separately generated demonstration art, not an xAI/Forge generation
result or a Style Lock consistency acceptance sample.

Forge CLI **0.2.1** processed the resulting 1536 × 1024 sheet locally:

- Split the sheet into a 4 × 2 grid of 384 × 512 cells.
- Apply manual magenta chroma key (`#FF00FF`, threshold 65, softness 0,
  despill strength 0, halo pixels 0).
- Save eight matted PNGs. The files in [sprites](sprites) are byte-for-byte copies
  of those outputs; [potion-before.png](source/potion-before.png) is the unmodified
  first ingested frame.

The local processing plan set both estimated and maximum Provider requests to
**zero**. The two image generation calls above were separate from Forge processing.
The compact [provenance record](provenance.json) includes hashes, dimensions,
processing settings, and the observed Job result. The full source sheet and local
Job/Plan stores remain outside Git; only the selected reusable sprites and images
needed to render this presentation are included here.

## Forest artwork: evidence limits

This run passed ingestion, matting, normalization, and quality analysis. Its Pack
export was **blocked**, with `prototype_usable` quality and `awaiting_review`
lifecycle. The eight different static objects were treated as an animation
sequence; bottom drift was 40 px and the loop match score was 0. The request kept
`requireGameReady: true`, and the quality gate was not relaxed or manually approved.

The showcase uses the intermediate matted static PNGs, not an exported Pack. It
does not demonstrate a completed `forge godot plan-install` run or certify these
objects as an animation. Small chroma fringes remain on some edges; the images are
not a claim of perfect matting. The Godot scene is a composed example, not a map
generated by Forge or a screenshot of a Forge desktop application.

## Re-render the forest images

From the repository root, with Godot 4.6.x on `PATH`:

```bash
godot --headless --path docs/media/showcase --editor --import --quit
godot --path docs/media/showcase -- gallery
godot --path docs/media/showcase -- processing
godot --path docs/media/showcase -- godot
```

The first command imports the assets. Each subsequent command briefly opens a
window, captures its actual viewport, and exits. Rendering needs a graphical
session; the headless dummy renderer does not produce these images. Each capture
prints a `SHOWCASE_CAPTURE` record and should be 1280 × 760 pixels. The committed
captures were rendered with **Godot 4.6.3** on macOS using the Avenir Next system
font; another platform may use the Arial fallback and have slightly different text.

[render.gd](render.gd), [main.tscn](main.tscn), and [project.godot](project.godot)
contain the complete presentation. No external media downloads, credentials, or
Provider calls are needed to render it. These showcase files are included under
the repository's [MIT license](../../../LICENSE).
