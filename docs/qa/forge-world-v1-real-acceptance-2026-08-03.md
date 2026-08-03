# Forge Terrain, Building Kit, and JSON Map real-model acceptance — 2026-08-03

## Decision

The first complete real xAI → Terrain/Building V3 Packs → JSON-only Map Compiler →
Godot 4.6.3 path is operational. Pack integrity, deterministic structure, collision and
navigation delivery, external texture use, resource size, and credential isolation all pass.

This is an engineering acceptance, not a release-quality art acceptance. The forest terrain
still shows visible periodic microtexture, and the building roof/wall modules are too repetitive
for commercial production. The implementation therefore remains an unreleased post-v0.2 world
pipeline. It must not delay or expand the `v0.2.0-cli.1` Character release gate.

The public CLI commands are compile-gated behind the non-default `world-assets` feature. The
`v0.2.0-cli.1` release workflow builds without this feature and does not expose Environment,
Terrain, Building, or Map commands.

## Environment

- CLI: workspace `target/debug/forge 0.2.0-cli.1`, locally signed before OAuth access.
- Provider/profile: `xai/default`, existing Preview OAuth profile.
- Engine: Godot `4.6.3.stable.official.7d41c59c4`.
- Tile profile: top-down, 32 px, `dual-grid@1.0.0`.
- Isolated acceptance root: `docs/qa/artifacts/forge-world-real-20260803/`.
- Style revision: `2045da06febe2f6f` (imported reference, zero Provider requests).
- Environment revision: `bdd891af018c889d`.

## Real Provider usage

| Operation | Job | Result | Requests | Generated images | Cost ticks |
| --- | --- | --- | ---: | ---: | ---: |
| Environment Lock | `ae4c424e-f27a-4652-b51b-606bdae31f29` | passed | 1 | 1 | 600,000,000 |
| Terrain first visual attempt | `a824e2a3-9d5e-47a1-9630-02947c7825d0` | structurally passed, visually rejected | 2 | 2 | 1,200,000,000 |
| Terrain final attempt | `bb849334-73b3-4e52-a0e9-42540270415f` | passed | 2 | 2 | 1,200,000,000 |
| Building Kit | `3f135601-bfd5-4d39-9449-ab7d777bbc44` | structurally passed | 3 | 3 | 1,800,000,000 |
| JSON Map Compiler | `5d8850cc-da76-4df3-9e7a-71263e2b1372` | passed, Provider-free | 0 | 0 | 0 |
| **Total acceptance spend** |  |  | **8** | **8** | **4,800,000,000** |

The two initial Environment requests and one Building attempt failed before any Provider media
request was accounted; each persisted zero usage and no partial Pack. Enforcing HTTP/1.1 for the
xAI client removed the repeated transport failure. A later Building retry succeeded without
changing the spec or credentials.

## Terrain result

- Exactly one base tile and 15 non-empty corner-mask tiles were emitted.
- All legal horizontal and vertical dual-grid adjacency pairs passed exact seam tests.
- `forge terrain test --seed 42 --samples 64` rendered 64 deterministic random corner fields and
  checked every sampled seam and alpha hole instead of only replaying the saved quality verdict.
- Tile dimensions, mask order, periodic edge closure, and atlas layout passed.
- The generator now selects the quietest source-material patch before periodic mirroring, avoiding
  the large flower/rock checker pattern found in the first real attempt.
- Final detail energies: base `0.021178892`, overlay `0.012009557`.
- Final verdict: `game_ready` under `terrain-quality@1.0.0`.

Evidence:

- [Final terrain preview](artifacts/forge-world-real-20260803/jobs/bb849334-73b3-4e52-a0e9-42540270415f/exports/moonlit-forest-ground.gsfpack/preview.png)
- [Terrain quality report](artifacts/forge-world-real-20260803/jobs/bb849334-73b3-4e52-a0e9-42540270415f/exports/moonlit-forest-ground.gsfpack/quality-report.json)
- [Terrain atlas](artifacts/forge-world-real-20260803/jobs/bb849334-73b3-4e52-a0e9-42540270415f/exports/moonlit-forest-ground.gsfpack/assets/terrain-atlas.png)

The final atlas has no structural seams, but its repeated horizontal bands remain visually
detectable. A commercial quality gate needs multiple deterministic material variants and/or a
separate decal layer; lowering the current structural thresholds would not solve this.

## Building Kit result

- All 12 fixed `topdown-exterior@1.0.0` modules are present.
- Four deterministic 3×3 to 6×5 example buildings were assembled.
- Footprints, south-facing entrances, non-overlap, collision metadata, occlusion metadata, and
  entry interaction semantics passed `building-quality@1.0.0`.
- The Godot installer generated one external atlas and four loadable example scenes with
  `TileMapLayer`, `StaticBody2D`, `LightOccluder2D`, `Marker2D`, and `Area2D` nodes.

Evidence:

- [Building preview](artifacts/forge-world-real-20260803/jobs/3f135601-bfd5-4d39-9449-ab7d777bbc44/exports/moonlit-forest-houses.gsfpack/preview.png)
- [Building atlas](artifacts/forge-world-real-20260803/jobs/3f135601-bfd5-4d39-9449-ab7d777bbc44/exports/moonlit-forest-houses.gsfpack/assets/building-atlas.png)
- [Building quality report](artifacts/forge-world-real-20260803/jobs/3f135601-bfd5-4d39-9449-ab7d777bbc44/exports/moonlit-forest-houses.gsfpack/quality-report.json)

The structural gate is working, but the generated examples read as repeated texture rectangles,
not production-ready cottages. Building V2 should use semantic module references or masked edits
for roof edges, roof corners, walls, doors, and windows instead of deriving all modules from three
periodic material samples.

## JSON Map Compiler result

- Input was only `MapSpecV1` JSON; no natural-language or text-model path exists.
- All relative Pack dependencies were normalized, hash-locked, validated, and copied into the
  self-contained Map Pack.
- Candidate 0 passed, using derived seed `14072193666015375138`.
- Layout SHA-256: `6fe050745529fea0aec756e99e1d25bfacb2081d43877866325ec5c458c26aa3`.
- The 64×48 map contains 8 buildings and 92 prop/decor placements.
- Spawn→Exit is reachable, every south entrance is reachable, and no isolated walkable island was
  found.
- Recompiling the same spec, dependency hashes, and profile produces the same layout SHA-256 in
  the fixture contract test.

Evidence:

- [Map preview](artifacts/forge-world-real-20260803/jobs/5d8850cc-da76-4df3-9e7a-71263e2b1372/exports/moonlit-forest-village.gsfpack/preview.png)
- [Map validation report](artifacts/forge-world-real-20260803/jobs/5d8850cc-da76-4df3-9e7a-71263e2b1372/exports/moonlit-forest-village.gsfpack/validation-report.json)
- [Compiled layout](artifacts/forge-world-real-20260803/jobs/5d8850cc-da76-4df3-9e7a-71263e2b1372/exports/moonlit-forest-village.gsfpack/assets/map-layout.json)

## Pack, Godot, and security acceptance

- Terrain, Building, and Map `.gsfpack` V3 directories all passed `forge pack validate`.
- The Map Pack contains self-contained, hash-addressed Terrain and Building dependency copies.
- Independent one-use Godot install plans all succeeded:
  - Terrain: `5f3bf906-55c5-4f5d-8d4a-46cf3ba3901b`
  - Building: `c04ef9be-01e8-4f52-beb5-a1d6f2e9fbe5`
  - Map: `5ed916b2-fbd8-472d-badd-aacab725ca5e`
- Godot loaded the `ForgeWorld` hierarchy headlessly, including the baked
  `NavigationRegion2D`.
- Largest generated text resource: 73,773 bytes; all are below 1 MiB.
- No text resource embeds an Image, ImageTexture, or image-pixel `PackedByteArray`.
- JobStore, copied Packs, and installed Godot resources contain no Authorization header, Bearer
  credential, access/refresh token, device code, or API-key marker.

## Regression verification

- `cargo fmt --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo test --workspace --all-targets`: passed, including 43 Core unit tests, 20 Pack tests,
  14 xAI/auth tests, and the world-generation contract.
- `scripts/test-cli-product.sh`: passed.
- `scripts/test-world-assets.sh`: passed with fixture generation and Godot 4.6.3 headless load.
- `scripts/test-cli-installer.sh`: passed.
- `scripts/test-cli-signing-contract.sh`: passed.

## Release conclusion and next gate

Do not publish the world pipeline yet. The correct sequence remains:

1. Finish three clean, all-`game_ready` Character → Pack → Godot runs and release
   `v0.2.0-cli.1` without Terrain/Building/Map in its public scope.
2. For `v0.3`, add and calibrate a visual repetition gate, generate forest/desert/snow across
   16 px and 32 px, and require multiple low-repetition variants before advertising Terrain Set.
3. For `v0.4`, replace material-only Building synthesis with module-aware generation and verify
   eight visually distinct buildings per kit.
4. For `v0.5`, retain the current JSON-only compiler boundary and run 30 seeds per accepted
   environment after the asset quality gates pass.
