# Forge Codex imagegen independent directions — 2026-08-16

Result: four authorized independent generations completed; Forge import blocked
at the Alpha edge gate; no retry, approval, action generation, Pack, or Godot.

## Authorized boundary

The user authorized exactly four direction generations. Codex built-in
`image_gen` was used as an external preview producer, not as a hidden Forge
Provider. Each request received only its approved same-direction frame:

| Request | Sole authority SHA-256 | Output SHA-256 |
| --- | --- | --- |
| front | `08e34f0e…93f15d0` | `2356516c…9de0042` |
| back | `2b04be2c…9893475` | `55ebedf2…7e3736` |
| right | `e2ca62f9…c06e86` | `e8180fb4…13d656` |
| left | `db459f7f…f1b5ca` | `9eec334f…04064` |

No candidate was chained into another request. Four requests completed, zero
retry requests were made, and the built-in interface exposed no stable Forge
model ID, durable Provider ledger, or API cost record. The original generated
files were preserved under the Codex generated-image directory and copied to
`generated-assets/experiments/codex-imagegen-four-directions-20260816/`.

## Raw output result

Every output is a `1254×1254` RGB PNG without Alpha. Despite the prompt requiring
genuine transparency and prohibiting a depicted checkerboard, all four outputs
baked a white/light-gray checkerboard into the pixels.

The views are front, rear, screen-right, and screen-left as requested. They are
neutral, planted, and empty-handed. Native review nevertheless finds clear
identity-bearing redraws: front adds a large diagonal torso strap and extra
clasps; back changes the cape hem motif count and construction; both side views
redraw facial, torso, pouch, and cape-trim details.

## Local Forge import

The four named files were imported without Provider credentials through the
new mutually exclusive four-file route.

- Setup-only Job `3c501dd9-8b8b-45dc-8147-5e931974a055` failed before image
  processing because the first local CLI binary lacked `grid-generation`; its
  usage remained zero. The CLI was rebuilt from the same workspace with that
  feature. No image generation was repeated.
- Processing Job: `8cff3609-1117-424b-8e70-8b499929480d`
- Parent approved Job: `9ea9cae9-dd94-4b4a-a277-4b5d0345b2ad`
- Lifecycle: `failed`
- Stable boundary: `direction_grid_import_alpha_edge_failed` /
  `alpha_edge_dark_outline_discontinuity`
- Forge Provider usage: zero requests, images, videos, edits, and uploads

Deterministic checkerboard matting itself was structurally successful:

- canvas: `2508×2508` assembled from four equal named inputs;
- background pixels removed: 4,976,862;
- fringe pixels removed: 5,153;
- reconstructed/decontaminated soft-edge pixels: 11,329;
- significant subject components: 4;
- neutral residual pixels: 0;
- border opacity: 0.

The post-matting Alpha edge report correctly blocked 406 isolated dark outline
discontinuity pixels against a strict pre-extraction budget of 8. It also found
140 opaque edge pixels. Because this hard gate precedes extraction, alignment,
relative identity assessment, and native review packaging, none of those later
reports or a DirectionGrid Lock was written.

## Safety and decision

The approved source tree SHA-256 was identical before and after:
`785da20140de9aff17be792f870dd010ec564f289e8baa60eef9eb7fe5caddbc`.
The JobStore grew from 23 to 25 Job directories solely because the setup failure
and the immutable processing failure were both preserved. Neither Job contains
an approval, `.gsfpack`, `.tres`, or `.tscn` artifact.

Do not approve or send this candidate downstream. The authorization did not
include a fifth request or a targeted replacement, so execution stops here.
Because the defect affects all four outputs and identity drift is already
visible, one targeted replacement would not close the set; a new generation
strategy or named production Provider route should be reviewed separately.
