# Local agent discovery and receipt recovery — 2026-09-23

Scope: development PR stack #73–#76 on Windows, ComfyUI 0.37.0 with the
user-installed Qwen Image 2.1 workflow, and native Godot 4.6.3. All generated
media and agent event logs are private under `target/qa/comfyui`; no model or
workflow is committed. The request to each agent described a game icon in
natural language and did not supply the exact Forge command. The game targets
were isolated QA Godot projects, not production installs. The first agent runs
used an installed Windows CLI from PR D (`x86_64-pc-windows-msvc`, no optional
features, SHA-256
`52778199a3f33f08ddf5adeefce320f4ddf9dde5a479cd999fa634baba3a1e68`).

| Agent | Discovery and same-command evidence | Result and limit |
| --- | --- | --- |
| Codex CLI 0.155 | Read the project `forge-use` skill, selected a local Qwen profile, and called `forge asset create` from an unprompted shell plan. The JSONL trace has 21 command executions and 12 Forge calls. | Purple potion parent Job `d0f743b2-638d-4a7f-afd0-cdfd4063180b`, install Job `a2d0624a-5ff4-4a6f-924a-045eb9287217`; 128×128 Godot texture loaded. The agent's visual review is not a human approval. Its Job store was inside the game and the old CLI let Godot add `.import` files to the Pack, so the original run does **not** prove retained receipt integrity. |
| DeepSeek Harness headless | Its `skill` tool selected `forge-use`, then it invoked the same `forge asset create` command. The permitted run's 252 JSONL events include 66 tool calls. | Green potion parent Job `f9fe49e6-3eb4-47aa-be62-b4066eadee73`, install Job `e0e3d5ad-c169-4f27-978f-bdc5e5f03643`; native Godot loaded the 128×128 texture. The agent itself caught the receipt/export failure caused by Godot scanning the retained Pack. Its first denied-permission attempt was interrupted and is not counted as success. This tested the headless conversation client, not the Web UI. |
| Claude Code 2.1.251 | Managed skill installation and the offline guide harness are covered separately. | `claude auth status` reported `loggedIn: false`; no live natural-language or generation result is claimed. |

The DeepSeek-generated original PNG has SHA-256
`026b65b3179560961368f98675135731b4fb7bedaac48c03254c05d52bf76a24`.
After the defect was found, Forge began placing `.gdignore` at a Job/Plan store
root when that store is inside a Godot project, and `asset create` began retaining
a source-bound review and automatic delivery receipt. The original source was
reused without another ComfyUI prompt. A native regression test deliberately
puts both stores inside a Godot project, imports a texture, then verifies the
retained Pack and automatic receipt; it passes with Godot 4.6.3. In a separate
live recovery run, parent Job `bf5a5d2c-1715-4c84-a9e2-3ac97f93e740`
installed through Job `70e260a5-307f-4858-8915-d7e57698e860`, returned
`receiptPath` and receipt SHA-256
`2421a743c4340f1e6de1f8f3d2fbc34ff1f627ae5c17ef12f884873dc8541c2d`.
`forge receipt verify --expected-sha256` reported `verified: true`,
`installationVerified: true`, and `visualReview: accepted`. The recovered run
used a source-tree development executable (SHA-256
`e1817c3eafe1c8969f76a05c696414108c17cab93cb946b3e20f289b464c18e2`),
not the installed CLI used for initial agent discovery.

The corrected commit `46ab896c704dab8fc57581d1f3586adb33b1b366` was also
installed outside the source tree on Windows (`x86_64-pc-windows-msvc`, debug,
no optional features, SHA-256
`8b01ac57e888b6ea2a132eacd8dd4cb8b774930a894ab04ebde12f39314284b4`).
The offline installed-binary harness passed all 178 commands, including Claude
skill installation. In a fresh isolated Godot project, the installed executable
reused the same green-potion source under parent Job
`631506c9-302e-4c94-a338-d51ce3a112d2`, installed via Job
`1be4e63c-4103-4338-8fde-9fedb699800a`, and returned automatic receipt
SHA-256 `f63fe74b237acc8fdaafb79337a22542a0e5a3fce4838dd0baf7f7760a059164`.
Godot 4.6.3 independently loaded the installed 128×128 `Texture2D`; the
installed CLI then verified the retained receipt and installation with
`--expected-sha256` (`installationVerified: true`, `visualReview: accepted`).

This establishes two live skill-discovery paths, a shared CLI protocol, and
recovery of the Godot/receipt defect. It does not satisfy the planned three-agent
image/video conversation matrix or the prospective paired comparison. Claude
login is pending; H3 reference-video weights and a second independent real game
requirement are unavailable locally. No time/token/cost saving is inferred from
these unmatched runs, and this workflow remains optional pending that evidence.
