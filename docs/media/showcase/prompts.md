# Showcase artwork prompts

Tool: Codex built-in `imagegen`, 2026-09-07. One generation call followed by one
edit call, with the first image supplied as the edit reference. Source generation
is separate from the subsequent Forge local processing.

## Generation

```text
Use case: stylized-concept. Create ONE production-oriented demo sprite sheet for Forge, a 2D game asset preparation CLI. This is source artwork for a truthful product showcase, not a screenshot or UI. Canvas exactly 1536 x 1024 pixels, landscape. Eight separate fantasy game objects arranged in an EXACT regular 4-column by 2-row grid. Cell boundaries at x=0,384,768,1152,1536 and y=0,512,1024. Each object centered in its own cell, with generous empty padding, no overlap into neighboring cells. Top row left to right: a corked red healing potion bottle, a teal mana crystal, a brass dungeon key, a small leather coin pouch. Bottom row left to right: a wooden supply crate, an upright wooden travel barrel, a wooden trail signpost with blank sign face, a stone-ring campfire with warm orange flames. The eight objects must look like one coherent set: premium clean 2D pixel-art illustration, crisp deliberately stepped pixel edges, restrained shading, charming hand-crafted detail, warm wood and antique brass, forest teal accents, three-quarter top-down game view, upper-left lighting. Make each object highly readable at small game sizes. Each object's silhouette should fit inside 260 x 330 pixels in its cell. Opaque perfectly flat pure chroma magenta background #FF00FF everywhere outside the objects, so Forge can demonstrate real background removal. No checkerboard, no gradients, no ground plane, no cast shadows outside the sprite silhouettes. No magenta inside the objects. No text, no labels, no border lines, no grid lines, no logos, no watermarks, no characters or people.
```

## Background edit

```text
Use case: precise-object-edit / background-extraction. Edit this sprite sheet for chroma-key processing. Keep all eight objects, their exact positions, pixel-art designs, scale, sharp silhouettes, and the 1536x1024 4-by-2 layout unchanged. Change ONLY the entire background outside the object silhouettes to one uniform opaque pure magenta color #FF00FF (RGB 255,0,255). This is a technical sprite sheet: flat solid magenta is REQUIRED, it must NOT be replaced with attractive scenery, dark gradients, glow, blur, or shadows. Remove the existing gradient background and colored glow around objects. Preserve the objects' internal shading and colors. No labels, no text, no grid lines. Every non-object pixel must be solid #FF00FF.
```

The generated background was near magenta rather than an exact uniform RGB value.
Forge's chroma-key threshold handled most of it; the source and resulting sprite
are shown without retouching in the processing comparison.
