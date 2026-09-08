# Embedded CLI guide — 2026-09-08

Goal: let Codex read Forge's usage instructions directly from the executable,
without installing a skill in each game. The guide and optional installed skill
share one maintained six-file bundle. This work targets v0.3.2.

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

The final local contract build identifies itself as a dirty default-feature
v0.3.2 debug build based on `1784c7a`; it is not the released executable.
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

## Delivery gates

The guide-only PNG-to-Godot workflow, GitHub quality checks, merge, published
archive verification and normal installer upgrade are pending at this checkpoint.
Their actual results will be added after completion. A passing development
contract does not establish release delivery or human art approval.
