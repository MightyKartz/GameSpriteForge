# Forge Provider Selection ADR

Status: accepted as an architecture recommendation only
Date: 2026-08-11

## Decision

Keep xAI Grok Imagine as the current stable baseline for existing V1–V8
workflows, but treat a PixelLab-style grid-specialized Provider as the
preferred candidate for `topdown-grid@9.0.0` real-model acceptance.

Do not migrate the default Provider surface now. The next paid validation
should compare xAI grid output against a PixelLab-style single-pass grid using
the same frozen Character spec, quality gates, Pack validation, and Godot
load test.

## Context

Forge's Provider boundary is model-independent, but the current production
Provider is xAI/Aurora. That model is strong at general image and video
generation, yet it has no native transparent output, no pixel-grid-specific
Controls, and no ControlNet/LoRA-style pose conditioning. This pushed Forge
into chained direction edits and image-to-video extraction, which proved
expensive and unstable in real runs.

`topdown-grid@9.0.0` changes the generation contract to one direction grid
plus one action grid per walk. The Provider should therefore be evaluated on
multi-view grid fidelity, native transparency, reference count, pixel-grid
preservation, price, and license—not merely generic image quality.

## Candidate comparison

| Candidate | Native alpha | Max references | Pixel-grid fit | Multi-view grid | Cost profile | License / commercial fit | Recommendation |
| --- | --- | ---: | --- | --- | --- | --- | --- |
| xAI Grok Imagine | No | 3 image / 7 video | General-purpose; needs post-cleanup | Possible but not specialized | Per image + input refs; video is expensive | Current working baseline | Keep as baseline and control group |
| PixelLab API | Yes | 3+ (workflow dependent) | High; pixel/grid-specialized | Native 2x2/3x3 direction grids | One grid request instead of chained edits | Requires commercial/API review | Preferred for grid acceptance |
| Retro Diffusion | Varies by deployment | Varies | High for pixel art | Not the primary multi-view grid fit | Self-host/API dependent | Requires license/self-host audit | Research alternative, not first production adapter |
| Gemini 3.x Flash Image | No/unclear for this contract | Multi-reference capable | Strong general edit model | Used by SpriteCook-like workflows | Attractive request cost | Vendor lock and terms require review | Secondary benchmark |
| FLUX.2 multi-reference | No native alpha in base form | 2–10 | Strong identity/edit potential | Useful for high-reference variants | Higher compute cost | Terms and hosting require review | Useful high-reference fallback |
| Qwen-Image-Edit | No native alpha in base form | Multi-reference capable | Strong edit drift correction | Useful for targeted repair | Potentially low-cost/self-host | License/weight redistribution audit | Good targeted-repair candidate |

## LayerDiffuse evaluation

LayerDiffuse-style native transparency is the correct long-term direction for
Forge. Current `keyframe-background-cleanup@1.3.0` and V8 direction cleanup
are compensating for the absence of native alpha by removing generated
backgrounds after the fact. That cleanup is useful but cannot perfectly
separate background-colored clothing or preserve every semi-transparent edge.

For grid-specialized production, native alpha should be treated as a strong
Provider requirement. If PixelLab or another candidate provides reliable
native transparency, Forge can reduce its dependence on post-hoc matting for
direction and action grids.

## Adapter alignment evidence

`packages/providers/src/pixellab.rs` adds a loopback-only PixelLab-style
adapter implementing the existing `MediaGenerationProvider` trait:

- ID: `pixellab-loopback`
- Capabilities: image generation, image editing, private file input, usage
- Constraints: max 3 image references, native alpha, no video
- No socket, credential, OAuth flow, temporary URL, or real request

`packages/providers/tests/pixellab_adapter_contract.rs` passes and shows that
the existing core Provider interface can consume this adapter without core
changes. A real PixelLab adapter would replace only the network/materialization
implementation behind the same trait.

## Recommendation

1. Keep xAI as the control group and existing workflow baseline.
2. For real `topdown-grid@9.0.0` acceptance, first request a small PixelLab
   or PixelLab-compatible budget and compare against the same xAI spec.
3. Use Qwen-Image-Edit or FLUX.2 only if the grid-specialized Provider cannot
   meet commercial or technical requirements.
4. Do not introduce model weights, LoRA, or ControlNet into the default CLI
   until license, distribution, and calibration review are complete.

## Consequences

- Core remains Provider-neutral.
- xAI-specific video extraction is no longer required for the grid path.
- Provider migration is constrained to the adapter crate and QA evidence.
- Real acceptance remains separately authorized and is not claimed by this ADR.
