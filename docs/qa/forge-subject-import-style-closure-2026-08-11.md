# Forge Subject Import and Style Closure QA

Date: 2026-08-11

Status: implemented and accepted offline. One real, previously approved Ayla
canonical was imported locally with zero Provider requests. The next paid V9
Direction Grid plan was prepared for inspection but was not executed.

## Problem closed

The authorized V9 bootstrap stopped after its generated Subject candidate
failed identity quality and native review found an invented quiver and opaque
background. Repeating that Provider request would spend more while changing an
already approved character.

The recovered Ayla canonical also could not safely reuse the historical
Subject revision: the current StyleLock has the same revision label retained
by earlier QA but a different style-board SHA-256. Revision-only validation was
therefore insufficient.

## Implementation

- Added feature-gated `forge subject import` for an explicitly approved,
  transparent single-subject PNG.
- Import planning fingerprints the canonical and approval note, reports 0
  expected / 0 maximum Provider requests, resolves no Provider, requires no
  authorization, and does not touch credentials.
- Delivery-sized inputs are copied byte-for-byte. Other valid inputs are
  normalized locally. Structural failures such as opaque media, multiple
  subjects, or boundary clipping remain fail-closed.
- The immutable SubjectLock records `subject-import@1.0.0`, source/canonical/
  mask hashes, transformation, approval note, diagnostic identity result, and
  import-report path/hash.
- Character planning and V9 runtime now require both the Style revision and
  exact Style board SHA-256 stored by the SubjectLock to match the current
  StyleLock. A matching revision with a different board fails with
  `subject_style_hash_mismatch`.
- Added `subject-import-report@1.0.0` to `forge schema list/show`, focused
  tests, product scripts, and CLI automation documentation.
- The all-feature matrix exposed a separate sampling-boundary defect: a newly
  generated front DirectionAnchor using pixel delivery was compared against a
  differently sampled Subject canonical for edge density. Frame zero now
  establishes that direction's edge baseline, while identity, geometry,
  direction and hard-defect checks still compare independently. The legacy
  keyframe contract also uses an isolated cache. No quality threshold changed.

## Ayla zero-cost acceptance

- Source canonical SHA-256:
  `e5f0b134254ddf0d44a8f49e9ecf2eca4de184e31e452c12afa054b444049745`
- Style revision: `397c05ac46ca8209`
- Style board SHA-256:
  `8418ff8a44a35bb051072b94e62580aabf56f54b3dcc3710d133a3aac5e11d9c`
- Import Job: `4821faef-9e45-4504-9cec-e9a20d1ed580`
- New Subject: `ayla-ranger-v9@e807965b18707c76`
- SubjectLock SHA-256:
  `cae5e94397a5e1bbb3fcaa5757443ea27d27d84e3f3836ac278ee4c0ee1d2a92`
- Import report SHA-256:
  `ceeadf4bff6d62b4119922e344ad1d2bc14e7bc76ed928aa3abfb94ab109b959`
- Transformation: `passthrough`, 256x256 to 256x256.
- Canonical output SHA equals the approved input SHA exactly.
- Job lifecycle: `succeeded`; authorization ID: absent; Provider usage
  artifact: absent.

The prior authorization ledger remains exactly two settled bootstrap requests,
`nextSequence=3`, and 1,200,000,000 total observed cost ticks. Subject import
added no ledger record.

## V9 next-stage preflight

The Character spec now references Subject revision `e807965b18707c76`. A new
Direction Grid plan passed Style/Subject closure and reports:

- workflow: `topdown-grid@9.0.0`
- provider/model: `xai/default`, `grok-imagine-image-quality`
- expected requests: 1
- maximum requests: 2
- input fingerprint:
  `c2c24cf276f2de1dca45bebac376ca0d68901be9acc3934bfc007bb5f8ba98a8`
- recipe hash:
  `4eb00d71de15c5b5821a7408b8a7280589715f17ce39eb60ddaf1c27b3dd311e`

The plan was subsequently executed under a new narrow authorization. Direction
Grid Job `833cbbf2-c8c8-4191-bcc6-4a0215d43b64` used one image request / 
700,000,000 ticks and stopped at review. Native inspection rejected the
candidate because it invented a quiver, arrows, held staff and cape ornaments
despite `equipment.kind=none`. No Action Grid, Pack or Godot resource was
created; see the real preflight document for hashes and security evidence.

## Verification

- `scripts/test-subject-import.sh`: pass.
- `scripts/test-grid-generation.sh`: pass.
- Subject import tests: 2 passed.
- V9 Provider contract: 4 passed, including Style board SHA mismatch rejection.
- `cargo fmt --all -- --check`: pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: pass.
- Workspace all-feature Rust, Provider, Pack, Godot and doc-test matrix: pass.
- Secret-like fields, temporary xAI URLs, legacy user JobStore paths and
  Provider artifacts in the imported Job/SubjectLock scope: 0 matches.

No real Provider request was made during this remediation.
