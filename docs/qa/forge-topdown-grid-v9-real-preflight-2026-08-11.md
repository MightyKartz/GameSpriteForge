# Forge Topdown Grid V9 Real Preflight

> 2026-08-11 update: the first real Direction Grid exposed undeclared equipment and hand-state drift. The offline remediation, zero-cost real-pixel recheck, and typed whole-Grid retry contract are documented in [forge-topdown-grid-v9-equipment-hand-remediation-2026-08-11.md](forge-topdown-grid-v9-equipment-hand-remediation-2026-08-11.md). The original Grid remains unapproved.

Date: 2026-08-11

Status: authorized bootstrap executed and stopped fail-closed during
SubjectLock quality. Direction Grid was not authorized or generated.

## Recovery decision

The prior Ayla project directory is no longer present under `generated-assets`.
QA metadata retained the old revisions, but the original StyleLock closure is
incomplete because its exact style board is unavailable. Forge therefore does
not recreate or claim the old Style/Subject revisions.

An exact byte-identical copy of the prior Ayla canonical image was recovered
from the durable JobStore and materialized as a new acceptance input. Its
SHA-256 is:

```text
e5f0b134254ddf0d44a8f49e9ecf2eca4de184e31e452c12afa054b444049745
```

The new acceptance creates fresh immutable revisions and preserves the old
evidence unchanged.

## Prepared local inputs

- project: `generated-assets/forge-topdown-grid-v9-real-20260811/project`
- style spec: `generated-assets/forge-topdown-grid-v9-real-20260811/specs/style.json`
- subject spec: `generated-assets/forge-topdown-grid-v9-real-20260811/specs/subject.json`
- V9 character spec: `generated-assets/forge-topdown-grid-v9-real-20260811/specs/character-v2-grid.json`
- copied canonical: `generated-assets/forge-topdown-grid-v9-real-20260811/inputs/ayla-canonical.png`

The Character spec intentionally retains a subject-revision placeholder until
SubjectLock creation succeeds. It must never be patched to an invented
revision.

## Planned paid sequence

1. Create a new StyleLock: 1 expected / 1 maximum image request.
2. Create `ayla-ranger-v9` from the recovered canonical: 1 / 1 image request.
3. Generate the V9 Direction Grid: 1 expected / 2 authorized image requests.
4. Stop at `awaiting_review`. Do not authorize or generate action grids.

The bootstrap plus first review stage is therefore 3 expected / 4 maximum
image requests. Allowed targets are only `style_board`,
`subject:ayla-ranger-v9`, and `direction_grid`; the only allowed model is
`grok-imagine-image-quality`. No video model, video request, Pack, or Godot
install is part of this authorization.

## Implementation correction found during preflight

The initial `grid-generation` CLI feature omitted the `forge subject` command
even though V9 requires a SubjectLock. The feature now includes
`consistency-v2`, and the grid product script asserts that the feature build
exposes `forge subject create`. V9 also resolves no video model.

`scripts/test-grid-generation.sh` passes after this correction: feature gates,
CLI builds, three Provider contracts, Pack/Godot fixture delivery, targeted
cell retry, and approval tamper protection are green.

## Authorized execution outcome

Authorization `ayla-v9-bootstrap-real-20260811` allowed only `style_board` and
`subject:ayla-ranger-v9`, with one request per target and 2,000,000,000 total
cost ticks.

- Style Job `279cd6fa-b950-4f53-a2d0-86fbe6978410`: succeeded; one request,
  500,000,000 observed cost ticks.
- Subject Job `0c8dea44-3db6-431e-a11a-3e9c99785f60`: failed fail-closed after
  one request, 700,000,000 observed cost ticks.
- Total: two settled requests, 1,200,000,000 observed cost ticks.
- No Direction Grid authorization exists; no Direction Grid, action Grid,
  video, Pack, or Godot asset was created.

The Subject output failed `character-identity@1.1.0` with
`character_face_detail_missing`: `enclosedFeatureCount` was 1 while the gate
requires at least 2. Its enclosed feature ratio (`0.01857585`) was diagnostic
and already above this profile's `0.003` ratio floor; it was not the failing
threshold. Native review also found an invented quiver and a complex opaque
background. The gate must not be relaxed to advance this output.

The structural remediation is to add a deterministic Subject import path for
an already approved canonical. Import must matte/normalize locally, calculate
the mask and baseline, create a fresh immutable SubjectLock with import
provenance, and make zero Provider requests. The exact recovered canonical is
already SHA-bound above. No further Provider request should be made until that
path is implemented and its fixture/security contract passes.

## Remediation outcome

The zero-Provider import path is now implemented and passed its offline and
security contracts. Ayla was imported byte-identically by Job
`4821faef-9e45-4504-9cec-e9a20d1ed580` as Subject revision
`e807965b18707c76`; its SubjectLock binds the current Style board SHA-256
`8418ff8a44a35bb051072b94e62580aabf56f54b3dcc3710d133a3aac5e11d9c`.
The next V9 Direction Grid plan validates at 1 expected / 2 maximum image
requests but remains unexecuted pending a separate narrow authorization.

Full evidence: [Subject import and Style closure QA](forge-subject-import-style-closure-2026-08-11.md).

## Direction Grid real execution

After the zero-Provider Subject import passed, the first V9 stage was executed
under a new authorization restricted to `direction_grid`, two maximum requests
and 2,800,000,000 cost ticks. Job
`833cbbf2-c8c8-4191-bcc6-4a0215d43b64` used one settled
`grok-imagine-image-quality` request costing 700,000,000 ticks, then stopped at
`awaiting_review` as required.

- DirectionGridLock SHA-256:
  `32facf8b31f87c3c3678e5f9e25e079a34422ba48b7346a447ff9b16ac53b403`
- Contact sheet SHA-256:
  `ca6c062d939441db5d299aa93fc5e90cc3545d5581b4b935bb6a3ae183ac7001`
- Structural slicing/background checks: passed for all four 512x512 cells;
  no boundary contact and no background foreground removal.
- Credential, temporary xAI URL and legacy JobStore-path scans: zero matches.

Native review does **not** approve this Direction Grid. Although the hood,
rear cape and four directions are readable, every direction invents a quiver,
arrows and a held staff despite `equipment.kind=none`, and the cape adds leaf
ornamentation absent from the approved canonical. This is an appearance/
equipment lineage failure that the current structural report does not detect.
The Job remains unapproved; no Action Grid, Pack or Godot resource was
generated. The authorization retains one request, but it must not be consumed
without an explicit targeted-retry decision.
