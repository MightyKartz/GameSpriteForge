# Chroma recovery and despill composition — 2026-10-08

## Defect and correction

`matte_pixel` previously selected edge color recovery **or** despill through an
`if/else if`. Enabling `edgeColorRecovery` suppressed despill even on opaque
pixels, where recovery immediately returns. Both explicit controls now compose:
recover foreground RGB first, then apply the existing despill function to the
result when alpha is nonzero. Alpha computation, parameter defaults, background
scope, halo cleanup and JSON/Pack contracts are unchanged. Setting despill to zero
retains recovery-only behavior; disabling recovery retains despill-only behavior.

The new regression fails against the old implementation and passes with the fix.
It checks partially keyed edges, opaque spill, non-key foreground/source alpha,
removed background, and exclusion of enclosed artwork in `border_connected` mode.
The macOS and Windows source quality jobs explicitly run `chroma_tests`.

## Retained real source and controlled comparison

The fox messenger is a retained external image-generation source from Forge PRO's
M3 experiment, not a newly generated image or an approved production animation.
No Provider requests occurred. Private source media and generated stores remain
outside Git.

- Source: 1536×1024, 3×2 cells; SHA-256
  `7f11a726d122cf191bb4d57604a5d0b3d8475aafa4e0067bb73ce9ece6c12830`.
- Orchestrator: Forge PRO `0.1.0b5`, commit `22f67b7`, separate from Forge.
- Old release: Forge `0.7.6`, commit
  `c38ddd0def615e9a1475031dce55b904728972d0`, clean/default features,
  `aarch64-apple-darwin`, release profile, binary SHA-256
  `cf77fcbeda335f5d6a5197b5a4f7e8987a4706fb3b6ac1f712a966cbe8283af3`.
- Fixed source build used for this comparison: version `0.7.6`, base commit
  `02164ff9e27032101033d91a42d9033d6feecade`, **dirty** with the correction and
  regression tests, default features `[]`, `aarch64-apple-darwin`, debug profile;
  binary SHA-256
  `792b79a80000af4b72d8ddae2ddd51f49e4aa5c3be2802e9efe0a9526e82e9b2`.
  This is local source evidence, not a published v0.7.7 package.
- Fixed recipe: manual `#02F902`, global scope, threshold 40, softness 16,
  despill strength 1.0, edge recovery true, halo 0. The A workaround changes only
  recovery to false on the old release. These values came from this fox run;
  they are not general art rules.

Ran `suggest-recipe`, filled the retained source hash, asserted the complete
recipe matched the prior baseline, then ran `prepare-images --preview-timing`
and `check-run` using a separate candidate lock for the locally built Forge.
Six frame alpha channels were byte-identical in all three variants. Canvas
499×547, pivot (222,484), selected source rectangles, offsets, loop and authored
durations 167/166/167/167/166/167 ms were preserved. All run file hashes and Pack
inventories verified. Candidates remain pending review and were not delivered.

## Separate warnings

**KEY_RESIDUE:** zero visible pixels within RGB distance 40 of the key in every
frame of all three variants. There is no bright key-residue warning.

**KEY_FRINGE:** key-like dark outline remains. The detector uses an inward 2px
band, alpha ≥16, hue distance ≤25°, saturation ≥0.35, value ≥0.02 and absolute
chroma ≥24/255; warning threshold is 3% and at least three pixels.

| Frame | Old recovery only | A: recovery off | B: composed recovery + despill |
| --- | ---: | ---: | ---: |
| 0 | 40.45% | 32.12% | 31.38% |
| 1 | 41.11% | 32.95% | 32.01% |
| 2 | 41.63% | 32.59% | 31.75% |
| 3 | 40.79% | 32.69% | 31.53% |
| 4 | 41.23% | 32.70% | 31.83% |
| 5 | 41.09% | 32.57% | 31.51% |

Total flagged pixels: 14,033 → 10,827 (22.85% reduction); the A workaround
has 11,146 (B improves another 2.86%). All six KEY_FRINGE warnings remain.
`check-run`: zero errors, six fringe warnings and one timing information entry.
Dark/light contact sheets show a modestly reduced outline tint, with the green
cast still visible; this is not a claim of complete cleanup or artistic approval.

The unchanged despill formula scales strength by `(1 - alpha_factor * 0.5)`.
Thus strength 1.0 removes only half the dominant-channel excess on opaque
pixels. This is a remaining algorithm/parameter limitation, separate from the
now-fixed control-flow bug; no new matting implementation was added to PRO.
Intentional key-hued art also needs palette review before enabling despill.

Local evidence is retained in Forge PRO's ignored
`.forge-pro/handoff/forge-combined-recovery/`: `comparison.json`,
`check-combined.json`, candidate lock/doctor and the preview's
`agent/contact.png`, `agent/onion.png`. These are machine-local evidence paths,
not a portable public fixture. Synthetic regression fixtures are committed.

## Verification boundary

Local macOS checks: formatting; 36 focused core tests (`chroma_tests`,
`source_matte_tests`, `animation_pixel_quality_tests`, `layered_pack_tests`);
41 Pack tests; actual `test-static-native-matte-cli.py`; the real fox pipeline.
Native `test-local-animation-delivery.py` passed five isolated macOS Godot 4.7.2
cases (single, character, nearest, legacy, source transform), checking saved
durations, loop, sampling, anchors, Pack validation and zero Provider requests.
Its `outputs/combined-recovery-animation/report.json` records the tested binary
hash; this native evidence is separate from fox visual review.
Windows execution and installed release packages require their own CI evidence.
Versioned consumer locks and the previously accepted Godot demonstration run
were preserved. Source checks do not establish publication or package acceptance.

## 中文说明

原实现用 `if/else if` 令颜色恢复与去色溢互斥；即使完全不透明像素的颜色恢复
直接返回，去色溢也被跳过。现在先恢复 RGB，再对仍可见的像素执行原有去色溢。
Alpha、默认参数、背景范围、去光晕与 JSON/Pack 合同不变；关闭任一处理仍可单独
使用另一项。合成回归覆盖软边、不透明暗绿像素、源 Alpha、纯背景与连通范围排除。

狐狸源图、配方、切片、偏移、画布、锚点和 1000ms 时长保持一致，六帧 Alpha
逐字节相同。亮色残留 **KEY_RESIDUE 全部为 0**；暗色描边 **KEY_FRINGE 仍然六帧
报警**，从约 40–42% 降到约 31–32%，仅略好于 A 的约 32–33%。深浅背景联络表仍
可见暗绿描边，不能宣称彻底修好。原去色溢公式在不透明像素上令 strength 1.0
仅消除一半主通道超出量，属于后续参数/算法限制，不等于本次组合开关缺陷未修复。

本次真实验证用带明确 SHA 与 dirty 身份的本地 Forge 开发构建及独立候选锁，
不代表已发布安装包。没有新生图、Provider 请求或美术验收；候选没有交付。
旧锁、旧 run 与已接受的 Godot 流程演示未改动。Windows 和版本安装包须单独验证。
