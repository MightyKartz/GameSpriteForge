# Forge / Sword workflow audit — 2026-09-08

The checked workflows run successfully on this Mac. Sword consumes two pinned
development binaries; its local static import and enhanced local animation
contracts are not supplied by the published v0.2.1 or the audited main baseline.
This audit updates usage/development skills and documentation, not those feature
branches or the release version.

## Toolchain and an actual local issue

| Target | Source identity | Result |
| --- | --- | --- |
| Main | `b44e1d3a189c036e061355e519a1fbbbe97f4808`, default CLI features | Workspace tests and rebuilt CLI/Godot contract passed |
| Published release | v0.2.1, existing installed release artifact | Doctor passed; `plan --help` correctly lacks `prepare-static` |
| Sword static | `c1f448082c34f531738126e38e41f5a9d66ca1a7` | Binary hash matches lock; static pipeline passed |
| Sword animation | `d4b18e2c2792db9877e5d5517ef390bbc254e2d2` | Binary hash matches lock; animation/source-transform pipelines passed |

The main checkout's existing `target/debug/forge` was stale and reported
`0.2.0-cli.1`. Rebuilt with
`cargo build --locked -p forge-cli --no-default-features`; it now reports `0.2.1`
and passes the product contract. This did not rebuild or replace either Sword
binary. All three 0.2.1 variants are distinguishable by capabilities and hashes,
not their version string alone.

Godot was `4.6.3.stable.official.7d41c59c4`. Doctor found the compatible local
Godot and Homebrew FFmpeg/FFprobe. This audit did not repeat a fresh installer,
bundled-tool selection, signing or notarization test.

## Checks rerun during this audit

| Check | Observed result |
| --- | --- |
| `cargo test --workspace` on main code | 242 passed, 0 failed, 0 ignored |
| Default CLI build and doctor | Passed; stale main executable replaced |
| `scripts/test-cli-product.sh` against rebuilt main, `FORGE_VERIFY_GODOT=1` | Passed; fixture generation/retries/review/jobs/Pack/install contracts, 32 native Godot frames with timing |
| Static branch `scripts/test-local-static-cli.py` | 3 cases passed: linear props, nearest props, linear icons; 2 items each, zero Provider requests, hashes/anchors/filtering and Godot resources |
| Animation branch `scripts/test-local-animation-delivery.py` | 4 cases passed: single action, two-action character, nearest and legacy; decoded RGBA, common coordinates, exact timing and native resources; invalid duration/canvas rejected |
| Animation branch `scripts/test-fixed-grid-source-transform.py` | 3 actual Sword source sheets passed: transparent padding, unchanged originals, exact grid pixels, Pack/install/resource checks; lossy offset rejected |
| Read-only Sword inventory | 2 binary hashes; 5 static source copies and their 5 image-generation originals; 5 installed static PNGs; 8 animation source sheets and 8 installed atlases matched recorded hashes |
| Existing Sword Packs | 1 static + 8 animation Packs revalidated with their pinned CLIs; all valid |
| Independent `forge-use` trial | Existing showcase barrel/crate imported as static props; Pack and fresh Godot resources passed; source hashes unchanged; zero Provider requests |

All local smoke tests used separate temporary stores and Godot projects. Sword
was read only and its working tree remained clean. Main product tests used the
offline fixture Provider; no real remote Provider or new image-generation call
was made. Counts of fixture requests are not claims of paid generation.

The three real-source animation runs retain `prototype_usable`, with the test
requests explicitly permitting prototypes. This proves lossless processing and
resource delivery, not visual approval. Sword's import receipts and later visual
review records are separate; no new human approval was submitted by this audit.
No fresh full-game or iPhone performance/visual acceptance was performed.

The independent skill trial retained `visualReviewRequired:true`: its source art
has some visible magenta fringe pixels. It did not invent a human approval or
regenerate the images. Its temporary Godot project initially used an incorrect
custom-user-data setting, creating only that test's cache outside the temporary
root; the agent moved those files back and verified the original location was
removed. No consumer project or source files were changed. This trial establishes
the supported static usage path, not every workflow described by the skill.

The compact [evidence summary](artifacts/forge-sword-workflow-20260908/summary.json)
records binary identities and results. Source artwork, generated Packs, full
temporary Job stores and private game source are not included in this repository.

## Documentation and skill changes

- Add `forge-use` for product use: actual binary capability checks, external
  image-generation handoff, static/animation route selection, zero Provider
  evidence, stable Godot targets and truthful quality/visual-review status.
- Update `forge-dev` from a desktop-first skill to the current Rust CLI product;
  retain desktop QA only for desktop changes.
- Add the [Codex local asset guide](../automation/codex-local-assets.md), with
  explicit development-version requirements and source/processing receipts.
- Correct the CLI guide's release-signing statement, distinguish local jobs from
  Provider jobs, describe parser errors separately from JSON runtime errors, and
  label the old character release gate as historical acceptance criteria.
- Link the guide and usage skill from both READMEs. Keep the product feature
  presentation focused on assets rather than implementation details.
- Remove the old skill source-string test and its npm invocations. It enforced
  desktop-only wording and a developer's absolute bundle path; skill validation
  and an isolated usage trial replace that evidence without adding new wording
  assertions.

## Recommended follow-up integration

1. Integrate the three static commits together: `fa58d64` (static rendering and
   ground anchors), `1f9d309` (local static plans), `c1f4480` (alpha bounds).
   Add their CLI/Godot smoke test to the release workflow and include the explicit
   real-engine Rust test. The implementation audit found no confirmed blocker.
2. Integrate animation coordinate/timing and whole-sheet preprocessing separately,
   keeping animation labeled experimental. Review the final combined source and
   rerun its contract tests before replacing either consumer lock. Existing
   generation normalization uses Lanczos; the local importer's nearest-neighbor
   guarantee must not be generalized to every Provider path.
3. Give the integrated CLI a distinct release identity and package/test it through
   the existing release workflow. Consider exposing build commit/capabilities in
   doctor so consumers need not infer support from `0.2.1`. Update Sword's lock
   only after reproducing its imports with that exact binary.

Do not merge the archived Stage 3 work merely to recover a skill or local import
feature. The smaller static and animation branches make their behavior and
verification independently reviewable. This audit does not publish those features
or turn a prototype animation into a release-ready asset.
