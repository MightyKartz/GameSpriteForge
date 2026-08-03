# Forge xAI Icon/Prop Real-Model Acceptance — 2026-08-03

## Scope

- Workspace binary: `target/release/forge 0.2.0-cli.1`
- Provider/profile: `xai/default` through the existing OAuth profile
- Engine: Godot `4.6.3.stable`
- Style revision: `f17362d90094f516`
- Isolated JobStore, PlanStore, Forge project, and Godot project under
  `docs/qa/artifacts/forge-static-real-20260803/`

## Real provider usage

| Operation | Job | Requests | Generated images | Cost ticks |
|---|---|---:|---:|---:|
| Style Lock | `40c04317-617f-4d64-afb5-d31523d9b65c` | 1 | 1 | 500,000,000 |
| Icon Set attempt 1 | `f9a40ce2-931c-4bb3-bdf2-20e468f0307c` | 7 | 7 | 4,600,000,000 |
| Icon Set attempt 2 | `80805d6e-3e0f-4cd9-ad47-2937481e5d30` | 7 | 7 | 4,600,000,000 |
| Prop Set | `44093379-ccfb-41cc-9dd3-5c9869b601c7` | 6 | 6 | 4,100,000,000 |
| **Total** |  | **21** | **21** | **13,800,000,000** |

## Icon Set result: blocked correctly

Both complete real-model runs reached the same classification. All five images are valid
128×128 alpha assets with one subject, safe bounds, and stable scale/anchor. The second run
reported:

| Item | Attempt | Palette overlap | Edge ratio | Verdict |
|---|---:|---:|---:|---|
| Healing Potion | 2 | 0.451 | 1.066 | `regenerate` |
| Mana Crystal | 1 | 0.554 | 1.090 | `awaiting_review` |
| Brass Key | 1 | 0.607 | 1.139 | `awaiting_review` |
| Coin Pouch | 2 | 0.511 | 0.632 | `regenerate` |
| Ancient Scroll | 1 | 0.637 | 1.014 | `awaiting_review` |

Forge correctly refused to export a Pack because two items were below the 0.55 palette hard
limit. No manual review was used to override `regenerate`.

The failure is diagnostic: the immutable Style Lock baseline assigns 86.5% of its palette
weight to `#608060`, the generated style-board background. This makes foreground assets appear
less palette-consistent than they are. Static consistency needs a foreground-aware Style
baseline (and then fixture + real-data recalibration) before Icon Set can pass the release gate.

- [All five generated icons](artifacts/forge-static-real-20260803/icon-set-all-items.png)
- [Second-run consistency report](artifacts/forge-static-real-20260803/job-store/80805d6e-3e0f-4cd9-ad47-2937481e5d30/consistency-report.json)

## Prop Set result: passed with explicit gray-zone review

All five props passed the hard media gates and were visually reviewed as one coherent forest
pixel-art set. Their initial verdicts were `awaiting_review`; no `regenerate` or `blocked` item
was accepted. The review decision is stored in the Job and the resulting Pack validates.

| Item | Attempt | Palette overlap | Edge ratio | Final result |
|---|---:|---:|---:|---|
| Supply Crate | 1 | 0.678 | 1.264 | accepted gray zone |
| Travel Barrel | 1 | 0.680 | 1.289 | accepted gray zone |
| Campfire Ring | 1 | 0.662 | 1.058 | accepted gray zone |
| Trail Signpost | 1 | 0.700 | 0.905 | accepted gray zone |
| Bedroll Pack | 2 | 0.572 | 0.977 | accepted gray zone |

- [Prop contact sheet](artifacts/forge-static-real-20260803/job-store/44093379-ccfb-41cc-9dd3-5c9869b601c7/exports/forest-camp-props/contact-sheet.png)
- [Prop preview](artifacts/forge-static-real-20260803/job-store/44093379-ccfb-41cc-9dd3-5c9869b601c7/exports/forest-camp-props/preview.gif)
- [Validated `.gsfpack`](artifacts/forge-static-real-20260803/job-store/44093379-ccfb-41cc-9dd3-5c9869b601c7/exports/forest-camp-props/forest-camp-props.gsfpack/forgepack.json)

## Godot and security acceptance

- Pack validation passed.
- Install Job `800722a1-d576-405c-9b80-191a3ca6fc9a` succeeded.
- Godot imported the external PNGs and loaded all five `Sprite2D` scenes headlessly.
- Every generated `.tscn` is 150–151 bytes, below the 1 MiB limit.
- No generated `.tscn` contains `PackedByteArray`, `ImageTexture`, or an embedded Image.
- The complete acceptance root contains no Authorization header, Bearer credential, API/OAuth
  token, device code, API key, or video data URL.

## Release conclusion at initial acceptance

Prop Set proves the real xAI → consistency review → Pack → Godot path works. Icon Set generation
quality is visually usable, but the foreground/background baseline defect makes the automated
release gate unreliable. Therefore static assets are **not yet eligible for a no-review release
claim**; fix and recalibrate the Style baseline before publishing `v0.2.0-cli.1`.

This finding was subsequently repaired with `style-baseline@2.3.0` and
`consistency@1.3.0`. The same real pixels were replayed with zero Provider requests; both
Icon and Prop Packs then validated and loaded in Godot. See
[the repair and replay report](forge-static-consistency-fix-2026-08-03.md).
