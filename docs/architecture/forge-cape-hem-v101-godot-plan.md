# Cape topology and V10.1 Godot delivery plan

Status: implemented offline; corrected direction art and complete four-direction
motion remain separately authorized work.

## Decision

The current approved four-direction source is not a valid final Godot identity
lock. Its front cape has no continuous gold lower hem while rear/right/left do.
Godot import cannot repair that semantic mismatch, and animation generation
must not spread it into more frames.

The delivery order is therefore:

1. Freeze `continuous_gold_lower_hem` as a four-direction garment-topology
   contract. Correct `front_idle` first, then import all four named files and
   approve a new DirectionGrid only if all four pass.
2. Reuse existing V10 video locally through `topdown-cycle@10.1.0` to determine
   whether translation-only anchor stabilization can remove size/placement
   jitter without spending another Provider request.
3. Generate or accept each directional walk separately only after the corrected
   DirectionGrid is approved. A direction may advance only when identity,
   equipment, scale, foot baseline, gait, motion and silhouette gates pass.
4. Export one complete Pack and invoke Godot only after four idles and four
   walks share the same approved garment topology, canvas, scale and anchor.

No partial validation Job may export `.gsfpack`, `.tres`, `.tscn`, or mutate a
Godot project.

## Cape hem contract

`cape-hem-consistency@1.0.0` is opt-in on the explicit four-file import route.
It measures a lower body band and counts gold pixels spatially adjacent to the
cape-green region, which avoids mistaking scarf, belt or boot gold for a cape
hem. Each of `front_idle`, `back_idle`, `right_idle`, and `left_idle` must meet
the same minimum signal. The report, path, SHA-256 and contract are bound into
the import evidence and DirectionGrid Lock. Missing or partial evidence fails
closed during approval and downstream reuse.

The first correction request should use only the approved front image as the
pixel/identity authority and explicitly require the same narrow gold lower hem
already visible on rear/right/left. It must not add a skirt, lengthen the cape,
change torso straps, or redraw the face. This plan does not authorize that
model request.

## V10.1 contract

`topdown-cycle@10.1.0` is not a generation workflow. It accepts only the
immutable original V10 `walk_down` Job whose scale lock failed solely on body
center and foot baseline drift. Plan estimate is 0 expected / 0 maximum.

For each selected frame it computes the median measured body center and bottom,
then applies integer translation on the existing canvas. It never rescales,
recolors, interpolates, changes Alpha, reorders frames, or calls a media
Provider. Maximum translation is 12 px, clipping is limited to 0.1%, and
residual center/baseline drift is limited to 1 px. Input/output paths and hashes
plus the report are stored as Job artifacts and a WorkflowGraph
`anchor_stabilization` node.

Passing stabilization is not acceptance. The complete gait, motion, silhouette,
identity, equipment and scale gates run again. Any remaining upper-body flicker
or contour drift blocks Pack/Godot delivery.

## Godot acceptance

The production Godot asset requires:

- a newly approved DirectionGrid with the hem contract game-ready in all four
  directions;
- `idle_down/up/right/left`, one frame each, byte-bound to that grid;
- `walk_down/up/right/left`, each one complete 8/10/12-frame source-timed cycle;
- one shared 256 px canvas, scale and foot anchor;
- every automatic report game-ready plus native review;
- a successful headless Godot 4.6 import/smoke test from the exported Pack.

Until those conditions hold, the existing V9.5 Godot smoke result remains a
diagnostic prototype, not the final character asset.
