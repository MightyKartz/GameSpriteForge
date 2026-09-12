# Layered Packs

`forge asset prepare-layered` packages already registered PNG layers without
trimming, resizing, matting, or calling a Provider. The output uses
`forgepack.json` schema `4.0.0`, asset type `layered`, and a dedicated V1 layered
manifest. Existing animation, character, static and world Pack formats retain
their versions and behavior. Godot installation registers the asset as `layered`
in `.forge/assets.json`; read-only delivery verification rejects a mismatched
registered kind. Reinstall a pre-fix development delivery to correct its registry.

```powershell
forge asset prepare-layered --request ./layered-request.json --output ./character.gsfpack --json
forge pack validate --path ./character.gsfpack --json
forge asset inspect --pack ./character.gsfpack --json
```

The output directory must not exist. Relative source paths resolve from the
request file's directory. Each supplied SHA256 must identify the exact reviewed
PNG bytes. Inputs must be still PNGs with an alpha channel; APNG, corrupt PNG
containers and truncated IEND chunks are rejected even if their declared hashes
match. This local operation creates no Plan or Job store and makes no paid
generation request. Hashes establish byte identity; they do not establish that
the artwork or animation was reviewed.

## Request contract

The following skeleton describes one layer with a short rotation/opacity clip.
Replace the example source path and SHA256 with real local inputs. Initial
`transform` and `blend` can be omitted from a request; their defaults are identity
and `normal`. Delivered manifests always contain the explicit values.

```json
{
  "schemaVersion": "1",
  "id": "character_cast",
  "name": "Character cast",
  "license": "LicenseRef-ProjectArtwork",
  "canvas": { "width": 1254, "height": 1254, "origin": [0, 0] },
  "sampling": "linear",
  "layers": [
    {
      "id": "head_hair",
      "name": "Head and hair",
      "path": "cast/head_hair.png",
      "sha256": "REPLACE_WITH_EXACT_LOWERCASE_SHA256",
      "pivot": [767, 406],
      "transform": {
        "position": [0, 0], "rotationDegrees": 0,
        "scale": [1, 1], "opacity": 1
      },
      "blend": "normal"
    }
  ],
  "defaultClip": "cast",
  "clips": [
    {
      "id": "cast", "durationMs": 1000, "loop": false,
      "tracks": [
        {
          "layerId": "head_hair",
          "keyframes": [
            {
              "timeMs": 0,
              "transform": { "position": [0, 0], "rotationDegrees": 0, "scale": [1, 1], "opacity": 1 }
            },
            {
              "timeMs": 500,
              "transform": { "position": [1, -1], "rotationDegrees": 1.7, "scale": [1, 1], "opacity": 0.8 }
            },
            {
              "timeMs": 1000,
              "transform": { "position": [0, 0], "rotationDegrees": 0, "scale": [1, 1], "opacity": 1 }
            }
          ]
        }
      ]
    }
  ]
}
```

All layers have the same rectangular canvas and origin `[0,0]`. Their array order
is the draw order, back to front. `pivot` and `position` use source pixels, with X
right and Y down. A layer's local transform rotates/scales around `pivot`, then
translates by `position`. Identity placement reproduces the original shared
registration. Fit the entire root scene to the destination; do not individually
fit its layer textures.

Each layer can use `normal`, `add`, or `multiply` blending. Multiply is explicitly
alpha aware: destination RGB is multiplied by `mix(white, sourceRGB, sourceAlpha
* opacity)` and destination alpha is preserved. Transparent black or white
padding therefore leaves previously drawn layers unchanged. It uses a frozen
trusted shader, because raw Godot `BLEND_MODE_MUL` would overwrite destination
alpha and erase content beneath transparent shared-canvas padding. Sampling is
`nearest` or `linear` for the whole Pack. V1 uses flat independent layers. It does
not support a parent/bone hierarchy, mesh deformation, masks, custom shader code,
Spine/Live2D imports, automatic part extraction, or generated hidden underpaint.

Clips contain one track per animated layer. Tracks use full absolute transforms,
not deltas from a previous frame. Untracked layers keep their initial transform.
Keyframes must include time 0 and `durationMs`, strictly increase, and interpolate
linearly; `rotationDegrees` also interpolates numerically, allowing intentional
multiple rotations. There are no implicit easing curves or shortest-angle rules.
Clips may be absent for static layered artwork. If `defaultClip` is present, it
must identify a declared clip. Motion limits and seam quality remain the asset
author's responsibility; a valid transform does not prove that hidden pixels exist.

Portable IDs start with an ASCII letter and contain only letters, digits, `_` or
`-`, up to 64 characters. IDs must be unique ignoring case; Windows device names
are rejected. V1 allows 1–64 layers, 1–4096 pixels per canvas dimension, at most
64 Mi total layer pixels, at most 32 MiB per PNG, 64 clips, 1000 keyframes per track,
and 65536 keyframes per Pack. Positions, pivots, rotations, scales and opacity must
be finite. Scales are positive; opacity is in `[0,1]`.

## Delivered files and Godot contract

```text
character.gsfpack/
  forgepack.json
  quality-report.json
  previews/layers.png
  assets/
    manifest.json
    godot_import.json
    layered.tscn
    forge_layered_player.gd
    forge_alpha_multiply.gdshader
    layers/<layer-id>.png
```

The Pack records each copied texture's SHA256 and the same hash in its source
provenance. `asset inspect` returns the complete `layered` manifest. Its ordinary
`frameCount` remains zero and `animations` remains empty because layered clips do
not consist of sprite-sheet frames.

The Godot scene contains a root `Node2D`, a `Layers` container, one pivot `Node2D`
at `Layers/<id>`, and its `Sprite2D` child named `Sprite`. Sprites are uncentered,
offset by negative pivot, and have explicit texture filters and blend materials.
The bundled controller reads the adjacent manifest and uses this stable node
mapping. Scene text is generated deterministically from the manifest. Pack
validation rejects changes to the generated scene/controller, mismatched source
hashes, dimensions, helper metadata, unknown fields, or unsafe layer paths.

Use the existing Godot installation workflow to install and register a layered
Pack. The installed root exposes the common playback methods `play(clip,
restart)`, `pause()`, `seek(seconds)`, `set_speed(multiplier)`, `advance(seconds)`,
`reset_pose()`, `duration_seconds()` and `state()`, plus `completed(clip)` and a
`finished` state. It supports both native layered clips and the common wrapper
for frame animations. `advance` receives unscaled elapsed seconds; the player
applies speed once. External-clock users should disable automatic node processing
before advancing manually.

`previews/layers.png` is an ordered contact sheet of the original layers. It is
not a rendered composite or a motion approval. `quality-report.json` intentionally
reports `review_required` and `layered_structure_only`. Native composition,
occlusion, blend appearance, motion extremes and seams require Godot screenshots
or captures and separate review. No structural check promotes artwork to accepted
visual quality.

The generated controller is part of the `godot-layered@1.0.0` contract. A future
controller change must explicitly account for already delivered Pack bytes; do
not silently reinterpret an older Pack with a different script. The trusted V1
source is frozen at `scripts/godot/runtime/layered-player-v1.gd`, and multiply at
`scripts/godot/runtime/alpha-multiply-v1.gdshader`; its scene
serializer is `forge_pack::layered::godot_scene_v1`. New runtime profiles must
retain these V1 implementations and add explicit validation dispatch. A hash
declared by an arbitrary Pack is not sufficient to trust executable script bytes.
Report validation checks structural fields; explanatory `notes` can evolve
without invalidating a previously delivered Pack.

## GPU and PCK regression

```powershell
python scripts/test-godot-layered-render.py --godot C:/Tools/Godot.exe --output target/qa/layered-render-new
python scripts/test-godot-layered-render.py --godot C:/Tools/Godot.exe --project ./isolated-preview/project --reference ./reviewed/original.png --output target/qa/layered-character-new
```

The first command uses only generated geometric fixtures. The second reads an
existing preview project's `addons/forge_assets/preview` layered asset and copies
its declared files into the new output. `--reference` is optional. The input
project is never used as Godot's working project; its source hashes are compared
again after the test. The output contains copied artwork, so keep private
references in ignored local QA directories.

The test requires a functioning display/compatibility renderer. Headless mode is
used only for resource import and packing, never for rendering evidence. Native
SubViewport captures compare initial registration and track midpoint against a
separate reference scene. Synthetic fixtures exercise normal/add/multiply together
on one shared canvas, plus 24 numeric multiply probes for source alpha, destination
alpha, inherited opacity and transparent black/white padding. Destination alpha
must remain unchanged. An optional original PNG checks the assembled initial
pose against reviewed artwork.

The same resources and imported textures are packed with Godot's `PCKPacker`.
The source working project is temporarily renamed inside the newly created QA
directory, and Godot runs with `--main-pack` to verify that the PCK supplies every
required resource. Native/PCK RGBA screenshots must match byte for byte. Separate
reference comparisons allow at most two channel levels for GPU arithmetic. Logs,
PNG captures and `result.json` identify the adapter and all comparison results.
A deadline terminates a stalled Godot process. This checks a self-contained PCK;
it does not claim an EXE export, signing, publication, or device performance audit.
