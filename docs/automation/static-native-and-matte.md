# Native static canvases and local PNG matting

Use a verified source build that exposes these commands; updating documentation
does not update a previously pinned Forge executable. Both routes operate locally
and make zero Forge Provider requests.

## Import a reviewed canvas unchanged

`plan prepare-static` keeps its existing `normalize` default. Existing requests
with `canvasSize` continue cropping, resampling, and placing the subject on their
selected square canvas. Opt in to `canvasPolicy: "preserve_source"` when the source
already has reviewed framing or needs to align with another image:

```json
{
  "schemaVersion": "1",
  "kind": "prop_set",
  "id": "reviewed-background",
  "name": "Reviewed background",
  "license": "private",
  "sampling": "linear",
  "canvasPolicy": "preserve_source",
  "items": [
    {"id": "background", "name": "Background", "path": "sources/background.png"}
  ]
}
```

Supply the actual rights statement; `private` grants no rights. Image paths are
relative to the request file. Omit `canvasSize` and keep `edgePaddingPx` at zero
for `preserve_source`; conflicting options are rejected. Each image must be an
8-bit RGB or RGBA PNG, 1–4096 pixels per dimension and at most 32 MiB. RGB and
fully opaque RGBA are supported. Each image must contain visible foreground.
All images within one Pack must have identical dimensions; use separate Packs
for different canvases. Rectangular and non-power-of-two canvases are supported.

```bash
forge plan prepare-static --request native-static.json --json
forge plan execute --token TOKEN_FROM_PLAN --wait --json
```

Check `ok` and `data.lifecycle_state` for execution success, then find the
`gsfpack` artifact in `data.artifacts`. Native item textures and retained frames
are byte-for-byte copies of the source PNGs. There is no crop, resize, centering,
alpha thresholding, or chroma operation. The source alpha threshold still checks
whether any foreground exists; it does not alter pixels. Atlas and preview
images are derived artifacts. The local report records `canvasPolicy`, original
and output dimensions, hashes, whole-canvas `cropBounds`, and
`sourceBytesPreserved`. An opaque image reports `transparentBackground: false`.

The manifest and Godot helper record the actual width and height plus a custom
anchor `(0, 0)`. Generated prop scenes use an uncentered `Sprite2D` at `(0, 0)`,
preserving source coordinates. Icon texture consumers must apply the declared
origin and sampling on their own nodes. Import with the existing
`godot plan-install` and `plan execute --wait` flow. This does not export a game
or convert static assets into a rigged/layered scene.

## Remove a flat background from one PNG

`source matte` runs the existing local chroma algorithm on one still PNG and
writes a new RGBA PNG. Save a request such as:

```json
{
  "schemaVersion": "1",
  "input": "sources/portrait-on-white.png",
  "output": "derived/portrait-matte.png",
  "parameters": {
    "keyMode": "manual",
    "manualKeyColor": "#FFFFFF",
    "threshold": 24,
    "softness": 32,
    "despillStrength": 0.0,
    "haloPixels": 0
  }
}
```

```bash
forge source matte --request matte.json --json
```

Input and output paths resolve relative to the request file. The output path is
explicit and must name a new `.png`; existing files, including the input or an
alias to it, are refused. Parent directories are created only after processing
succeeds. The source remains unchanged. The output retains the original width,
height, and pixel coordinates. Alpha-zero pixels get RGB `(0, 0, 0)` to remove
hidden background colors. Soft alpha values are preserved by the chroma
algorithm, and RGB may change according to despill parameters. A result with no
visible pixels is rejected without publishing a PNG.

The input must be 8-bit RGB/RGBA, at most 128 MiB and 33,554,432 pixels. Animated
PNGs are rejected. This is color-key removal for flat backgrounds, not semantic
segmentation. Inspect hair, translucent edges, and foreground colors that
resemble the key; a matching foreground color can also be removed.

Omitting `parameters` selects the existing defaults: `auto_corners`, threshold
48, softness 18, despill strength 0.5, and no halo erosion. `auto_corners` averages
the four corner RGB values and applies border-connected chroma cleanup. Manual
mode uses `#RRGGBB`. Threshold and softness are integers from 0–255, despill
strength must be finite from 0–2, and halo pixels from 0–4. Halo erosion can remove
thin strokes; use it only after reviewing the un-eroded result.

The JSON report includes input/output SHA-256, dimensions, exact parameters,
resolved key color, alpha statistics, foreground bounds, zero Provider count,
and `visualReviewRequired: true`. Retain this report with the input and derived
PNG. It does not create a Plan, Job, receipt, or game asset lock. After review,
the matte can be imported with either static canvas policy.
