# Local H3 video agent delivery — 2026-09-23

Scope: development PR stack #73–#76 on Windows, ComfyUI 0.37.0, native
Godot 4.6.3, and an installed Forge CLI from commit `5de6054f3fca19ae6092ea8b5a8eeaa10eb2f02c`
(`x86_64-pc-windows-msvc`, no optional features, debug, SHA-256
`7f6d4cdc36859098c30e582225cbc0f4e820b7eb0d8ca664519e431008ec9e70`).
The user-installed H3 I2V workflow had SHA-256
`2aa394bfd9712a8b62201272f5b5fdc84a9ff9f7abd354187d3e4f9ec6dac190`;
its configured model was `minimax_h3_fl2va_pruned_w4a8_mixed.safetensors`.
No model, workflow, generated media or agent transcript is committed. Private
evidence is under `target/qa/comfyui`. Both agents received a natural-language
game brief, a Forge/profile pointer and an isolated QA target, without an exact
`forge asset create` command. Neither target was a production game install.

| Agent | Generation and delivery | Native result |
| --- | --- | --- |
| Codex CLI | Selected the shared `forge-use` skill and called `forge asset create`. One ComfyUI prompt `015f594b-865b-4c3c-b199-23a6232af087` produced MP4 SHA-256 `581d53199ae60474f5066595f8f4504551aa6abb567fe1d390cac86f9b57f8de`. Parent Job `783359db-b24c-4325-8303-4d1900a241fc`; install Job `eb88cb5f-751e-4f24-9194-64b58baf6489`; receipt SHA-256 `9bc11f647a779f459a050c2c2d1af971a58fdbfcf7e9e41528ae9a6b15f4b79e`. | Independent Godot scene script loaded and played 12 `idle_breathe` frames; Forge verified the receipt. Its first `forge godot verify` failed because the isolated project lacked a main scene, so the independent script is the native playback evidence. |
| DeepSeek Harness headless | Selected the same managed skill and command. One prompt `af2f1aaa-cdf1-456b-ac09-279ff69203f3` produced MP4 SHA-256 `e3a340b87fb8c5b07cf4cf585bd2fc534c797834d710bc767672b5303485d8c6`. Parent Job `b94d7597-8f49-4afe-add1-e5b5711398d3`; install Job `ad04531b-1a69-4d54-9af1-0e377c86abd6`; receipt SHA-256 `030d4c82b9b5533ab43a38614d1885197668bbde13f3332faa73bd4179ddb8ff`. | Independent Godot scene script passed playback and receipt checks. This run read a neighboring Codex QA log, so it is **not** an independent comparison arm. Its agent review accepted isolated technical QA only. |

The Codex run overlapped `--wait` and `--resume` on one parent Job. The older
binary created two prepare children, revealing a concurrency defect. PR #76
now serializes one `asset create` state transition with a per-Job lock; a second
resume observes the active Job, while cancellation is serviced by its owner.
Regression tests cover both races. The defect and fix are part of this QA record;
the original receipt is not evidence that the old binary avoided duplicate work.

The DeepSeek run also showed that a video request with `canvasSize: 256`
previously produced 536×536 Pack frames. PR #76 now applies the requested
canvas size during character-frame normalization and retains integer anchors
for Godot pixel snap. A replay of the same MP4 through a source-tree build
(without a new ComfyUI prompt) produced fourteen 256×256 PNG frames under
parent Job `dc6d3af1-1398-425a-877a-4330fcb6a634` and install Job
`5ab52bc5-45ab-4ae0-953a-ef76bda316a4`; Pack SHA-256
`95d94f2ea7084a3c01947bb156af354ede4430432db8fd26ca0495560bdf7041`,
receipt SHA-256 `88effa9c8c82fc9142dd98a9480792b92978dc02929391af6a876049b14437e4`.
An independent Godot script played twelve `idle_breathe` frames, asserted each
texture was 256×256, and the receipt check returned `verified: true` and
`installationVerified: true`. A first replay attempt exposed fractional anchors
and failed native installation; rounding the scaled anchor fixed that failure.
The original square reference had been stretched to a wide H3 input, and the
generated subject scale did not match the support action. Canvas normalization
does not repair those visual composition defects.

The ComfyUI service was stopped after these runs. This evidence establishes
live Codex and DeepSeek conversation-to-CLI-to-Godot paths for H3 I2V, with
the noted revision and regression checks. It does **not** establish a shippable
character animation, human visual/rights approval, three-agent coverage,
H3 reference-video capability, or a fair time/token saving. Claude Code
2.1.251 still reports `loggedIn: false`; a live natural-language Claude run
needs sign-in. The prospective [paired comparison](asset-delivery-comparison.md)
also needs two independently maintained real game requirements, frozen and
independent Forge/baseline arms, revision tasks and complete time/usage evidence.
These QA runs used isolated projects; the DeepSeek context carryover disqualifies
it as a paired efficiency observation. The workflow therefore remains optional.
