# GAME-SPRITE-FORGE html.to.design import

Status note, 2026-06-05: this folder is a visual reference archive, not the current source of truth for production UI behavior.

The production MVP now prioritizes real local functionality over 1:1 screenshot fidelity. In particular, the app content should not recreate fake macOS traffic-light window buttons, marketplace/cloud/generation affordances, or inactive export targets that do not perform real local actions. Use the imported HTML/PNG to preserve the dense workbench mood, spacing, palette, and canvas/timeline/inspector structure; use `apps/mac/src` and the docs under `docs/architecture` for current behavior.

Local page:

```text
http://127.0.0.1:8765/game-sprite-forge-app.html
```

Recommended import flow:

1. Open the target Figma page.
2. Run the `html.to.design` plugin.
3. Import the local URL above.
4. Use the full page capture. The mockup canvas is fixed at `1568 x 1003`.

If the local server is not running:

```bash
cd /Users/kartz/Development/Forge/figma-import
python3 -m http.server 8765 --bind 127.0.0.1
```

Generated files:

```text
game-sprite-forge-app.html
game-sprite-forge-app-final.png
assets/hero-knight-crop.png
assets/sprite-sheet-crop.png
assets/reference-game-sprite-forge.png
```
