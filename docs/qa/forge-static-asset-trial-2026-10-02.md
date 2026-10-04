# Static asset trial: generated props and background — 2026-10-02

A local trial that generated three images with Codex built-in imagegen (a chest,
a stone lantern, a forest-clearing background), prepared them with the installed
Forge release and rendered them in Godot, then replaced one prop to test update
stability. It changed no Forge source; the resulting guidance lives in
`forge-use` (`local-static.md`) and `examples/static-forest-scene/`.

## Toolchain

| Item | Identity |
| --- | --- |
| Forge launcher | `/Users/kartz/.local/bin/forge`, SHA-256 `618cdad7ef1981cf046852baad1a34c5e435847ab2ab2356a6eeb51cf5a5e342` |
| Forge build | 0.6.3, commit `546bcefaf145b32f6d5f27b1e509a93b071037f8`, clean, release, no optional features |
| Godot | 4.7.2.stable.official.ed1daf0bf, macOS Apple Silicon |
| Image generation | Codex built-in imagegen; Forge Provider requests: zero in every Job |
| Trial workspace | `~/.codex/visualizations/2026/09/22/01a0c7f3-e1fd-7cd2-91ca-5f09011bff6f/forge-static-trial-20261002/` (sources, requests, evidence, review scene) |

## Findings

1. **Generated transparent PNGs carry faint stray pixels.** Threshold-1 subject
   bounds were far larger than threshold-8 bounds (chest 1320×1057 vs 994×876;
   lantern 935×1284 vs 619×1219; chest v2 1350×1125 vs 1140×978). After
   reviewing the light/dark previews, `foregroundAlphaThreshold: 8` matched the
   real subjects for all three props. The threshold selects the crop; originals
   stay unchanged and residue inside the crop is not removed.
2. **A full-frame background round-trips unchanged.** The 1536×1024 opaque PNG
   used `canvasPolicy: "preserve_source"`; the installed texture is
   byte-identical to the source. Background and props were separate Packs.
3. **Canvas size is not world size.** Both props share a 256 canvas; the review
   scene draws the chest at scale 0.48 and the lantern at 0.65. Instance scale
   and ground position belong to the game scene, not the Pack.
4. **Same-ID replacement updates in place.** A regenerated chest (teal gem
   added) was prepared with the same set/item IDs and installed to the same
   asset key and target. `texturePaths` and `scenePath` in
   `forge_usage.json` stayed identical, `verify-install` passed, and the
   rendered scene kept its layout with the new art. The pre-update receipt then
   failed verification with `receipt_invalid: installation baseline differs
   from receipt` — the intended signal; a fresh receipt was exported and
   verified. Old receipts must be kept, not rewritten.

## Checks run

| Check | Result |
| --- | --- |
| `source inspect` with previews, 4 sources | Passed; alpha counts and multi-threshold bounds recorded |
| `plan prepare-static` + `plan execute`, 3 requests (props v1, background, props v2) | Succeeded; provider estimate and actual requests both zero |
| `pack validate` / `asset inspect`, 3 Packs | Passed |
| `godot plan-install` + execute, 3 installs | Passed after one expected refusal (see below) |
| `godot verify-install` with matching Packs, 3 installs | Passed |
| `receipt export` + `receipt verify`, 3 receipts | Passed; v1 props receipt recheck after the v2 install correctly failed with `installation baseline differs from receipt` |
| Godot 4.7.2 headless editor import | Passed |
| Native rendered captures (scene, light/dark size check, 1152×768) | Rendered and reviewed; both views captured successfully |

## Recovery evidence

The first props install was refused because the review project's entry script
did not exist yet (`Godot reported an error ... 'res://main.gd' ... File not
found`). The retained Pack was reused after adding the script; no source art
was regenerated. The first `source inspect` preview export failed because the
preview parent directory did not exist; creating it and rerunning succeeded.
Both failures are retained in the workspace evidence.

## Limits

Visual quality is an agent review of a trial: the chest reads clearly down to a
96 px canvas; the lantern's ornament is dense at that size. No user art
approval, Windows run, performance test or license review is claimed. The
background is a single scene image, not tiles; no collision, navigation or
gameplay behavior was built or verified.
