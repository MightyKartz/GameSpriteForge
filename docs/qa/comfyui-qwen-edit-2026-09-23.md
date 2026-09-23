# Local Qwen Image 2.1 edit P2 validation — 2026-09-23

This source-tree test used Windows `x86_64-pc-windows-gnu`, ComfyUI 0.37.0,
Qwen Image 2.1 edit and Godot 4.6.3 in an isolated game. The development
binary had no optional features and was rebuilt during P2 work; its exact hash
at the first live request was not retained. The installed UI template was
exported as API JSON, then narrowed to one reference. API workflow SHA-256:
`210bf9ab9b7dd03b89dc27fa8a2ad55efc49926d2463a25dcc3428f567203b65`.
The explicit profile maps prompt `459:474`, reference image `470:image`, and
SaveImage output `461:images`; declared model ID:
`qwen_image_2.1_int8_convrot.safetensors`. `provider doctor` checked the live
node metadata before generation. The private workflow, model and media remain
outside Git.

The first `asset create` edited the P0 blue potion to red but produced an
opaque purple background (parent Job `13d67478-7696-4270-ac8e-f3949c656aaa`,
source SHA-256 `98bb995b16fc5a6dcc113731238f8fa5fb78baff7a1ec9272df169c3386fe255`).
Static preparation rejected it. A separate source-import request with explicit
`staticMatting: "auto_corners"` removed the flat background; its derived PNG
SHA-256 was `164babad575300a2d5c4c001290ea3cd134c9ff47e7e71f4c132eec98ee5fbaf`.
Agent QA reviewed the red bottle preview and installed it in an isolated Godot
project under parent Job `8ebf0f78-ce23-4c7c-8235-153cd0401988`.

A second **single generated request** exercised the final edit-and-matte path
without source re-import. Parent Job `73b2a467-c6cc-40d0-9be4-875282b2bbba`
submitted one prompt `970d0da7-2d6f-4c1d-ad0b-14060c9539dc`, retained its
unaltered source PNG (`caf325741a937e3e6f114bae2294751fddcd326ff494aba788f8b54d25a40422`)
and derived matte (`164babad575300a2d5c4c001290ea3cd134c9ff47e7e71f4c132eec98ee5fbaf`).
Preparation child Job `08f02033-b786-435f-85ef-9ad3f452482a` produced a
128×128 red potion contact sheet. A source-hash-bound agent QA review in this
isolated project resumed the same parent; install Job
`47134be8-ca46-46b3-a7d4-e6d22ec60a93` succeeded. Godot 4.6.3 headless
loaded `res://addons/forge_assets/red_potion_direct/items/red_potion_edit_direct.png`
as a 128×128 `Texture2D` and printed
`PASS Forge Qwen direct edit Godot resource: red potion 128x128`.

This is agent QA for test assets, not a human art or license approval. The matte
mode depends on a flat corner-colored background and should be reviewed per
image. Unit/integration tests cover explicit matting source preservation,
profile descriptor hash validation and read-only upgrade checks. The 178-command
offline guide/skill harness passed with the new edit example and Claude skill
installation. It does not prove natural-language discovery in Codex, Claude
Code or DeepSeek Harness, packaged CLI behavior or macOS compatibility.
