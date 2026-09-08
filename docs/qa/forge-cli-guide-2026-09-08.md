# Embedded CLI guide — 2026-09-08

Goal: let Codex read Forge's usage instructions directly from the executable,
without installing a skill in each game. The guide and optional installed skill
share one maintained six-file bundle. v0.3.2 is published and verified.

## Implementation

- `forge guide` returns the overview. Six topics and their exact bundled paths
  expose the overview, static/Provider/experimental animation references and two
  JSON request examples.
- Plain output is the selected resource's original content. JSON adds compiled
  identity, content hashes and a resource index without loading every reference.
- Guide reads use no filesystem resources, network, Provider credentials or
  installed skill. Unknown paths return `guide_resource_not_found`.
- Main help points to the guide. Optional project/user skill installation keeps
  the existing update, backup and user-modification protections.
- English and Chinese READMEs lead with direct Codex use and retain the three
  product images. Animation remains experimental; image tools and Godot remain
  separately supplied tools.

## Local verification

The reviewed local contract build identifies itself as a clean default-feature
v0.3.2 debug build at `930b0e5`; it is not the released executable.
See [local results](artifacts/forge-cli-guide-20260908/local.json).

- 17 CLI Rust tests passed, including three new guide tests and the existing
  build-identity and bundled-helper integration tests.
- Workspace Clippy passed without warnings; formatting and skill validation
  passed. The edited documents' 81 local Markdown targets exist.
- 33 offline CLI cases passed across 118 commands. They cover exact resource
  output, aliases, JSON examples, shadowing/traversal attempts, help/capability
  discovery, standalone binary use and existing optional installation contracts.
- A request exported through `guide static-example` planned successfully before
  any skill installation, with zero estimated Provider requests.
- Guide/help reads preserved the isolated project, home, local files and stores.
  The existing `doctor` command initializes empty stores; its separate capability
  check is intentionally outside this read-only assertion. The first harness run
  included that command incorrectly; the harness was corrected before passing.

Full logs and synthetic fixtures are retained under ignored
`target/qa/v032-guide/`. Private game assets, existing consumer locks and import
receipts are outside the implementation scope.

## Guide-only PNG-to-Godot use

An agent used only CLI help, embedded guide resources and returned artifacts to
prepare two synthetic transparent PNGs, validate their Pack and install it into
an isolated Godot project. No project or user skill was installed. Home/config
and stores were isolated; Job and Plan stores remained outside the game project.
Both Jobs succeeded with zero Provider requests. Original source hashes remained
unchanged and normalized textures matched their installed copies. Godot 4.6.3
loaded both textures and prop scenes and verified the ground anchor `(128, 240)`
and linear filtering. See [forward-use evidence](artifacts/forge-cli-guide-20260908/forward.json).

The first native check assumed a Sprite2D scene root, while delivery correctly
uses a Node2D root and Sprite2D child. The check was corrected against the actual
returned scene, and the guide was clarified in `930b0e5`. The forward-use evidence
retains its earlier pinned development binary and guide hash; the final guide
content was then covered by the 33 CLI contracts. The asset-processing code did
not change. This is structural validation, not human art approval.

A separate isolated check installed the actual v0.3.1 release's managed skill,
then upgraded it with v0.3.2. The new status was `current` and every old managed
file, including its manifest, matched the full backup. See
[upgrade evidence](artifacts/forge-cli-guide-20260908/skill-upgrade.json).

## GitHub quality and merge

The [quality run](https://github.com/MightyKartz/GameSpriteForge/actions/runs/34197085775)
passed all eight matrix gates and the 33 guide/skill cases. The earlier run was
cancelled when the prop-scene guide clarification superseded its head.
[PR #28](https://github.com/MightyKartz/GameSpriteForge/pull/28) merged at
`922ce83e33b1a9c085570c3e34051d739b4167f9`, the v0.3.2 annotated tag's target.
See [quality evidence](artifacts/forge-cli-guide-20260908/quality.json).

## Published release verification

The [formal tag workflow](https://github.com/MightyKartz/GameSpriteForge/actions/runs/34198372444)
passed source, compiled identity, product, packaged install/upgrade, SBOM and
attestation checks before publishing
[v0.3.2](https://github.com/MightyKartz/GameSpriteForge/releases/tag/v0.3.2).
It passed 277 workspace Rust tests; the normally ignored native Godot check was
also run explicitly in the release contracts.

All nine public assets were downloaded independently. The ZIP checksum and CRC,
payload manifest, annotated tag target and exact tagged installer bytes matched.
The attestation was verified for this repository, workflow, tag and source commit;
the downloaded SBOM matched its complete signed predicate.

| Identity | SHA-256 |
| --- | --- |
| Release archive | `fb72ba869b220f132e5674493c686f9a129af7ce35bbaf4cbe71a8b03a8a1002` |
| Forge binary | `d6af1cc17892574c010470fa00aeea82c8626fbb280fee37e5ecfbfed07dbca9` |
| Embedded guide/skill | `5f63b2adfb9e69e63e5b29d0692f8e3796c9fb3a13527a6e46e4494129e70165` |

The actual downloaded archive passed fresh installation, same-version reinstall
and upgrade from v0.3.1 on the local machine. Its public launcher passed all 33
guide/skill cases, the product contract, three local static and five local
animation cases with native Godot. External FFmpeg discovery was disabled.

The normal HTTPS installer, with no version override, then selected v0.3.2.
Its public `forge` launcher matched the audited binary, clean default release
identity, embedded guide hash and same-payload FFmpeg helpers. The old v0.3.0 and
v0.3.1 binaries, shell profile and both Sword locks retained their original hashes.
See [release evidence](artifacts/forge-cli-guide-20260908/release.json).

Full release logs remain in ignored `release-candidates/v0.3.2/evidence/`. This
post-release update adds only QA records; shipped code and embedded content remain
at the verified release tag. No skill installation in a real game was needed.
