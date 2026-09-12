# Example Inputs

`forge-walk-sheet.png` is a small synthetic sprite sheet used by the Rust
`generate_godot_smoke_pack` example. It has a 4 × 2 grid of 64 × 64 cells and a
green background for exercising grid slicing, chroma processing, Pack export,
and native Godot import.

Run the complete smoke check from any checkout with:

```bash
bash scripts/run-godot-pack-smoke.sh
```

Godot must be on `PATH`, available as the standard macOS app, or selected with
`GODOT_BIN`. Full Packs and the temporary Godot project are written beneath
`target/qa/`. The CLI asset-delivery scripts generate their own isolated fixtures;
see `scripts/test-local-static-cli.py` and `scripts/test-local-animation-delivery.py`.
