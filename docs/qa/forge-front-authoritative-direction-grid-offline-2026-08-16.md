# Front-authoritative DirectionGrid offline acceptance — 2026-08-16

Verdict: accepted offline. No real Provider request was made.

## Problem reproduced

The previous four-file experiment generated each direction independently.
Although the overall character was recognizable, rear/right/left invented a
skirt-like cape lower hem and gold edging absent from the front. This is a
generation-topology error, not a Godot import issue.

## Implemented result

- Added `front_authoritative_no_skirt_hem` to the typed cape contract.
- Added a generated-grid request field and strict V9 ImageLocks-only scope.
- Added `--front-authoritative-grid`, accepted only with
  `--item direction_grid --stage still --plan-only` and an approved source.
- Bound Plan budget to 1 expected / 1 maximum request.
- Sent only source `front_idle` as `DirectionAnchor`; Style remains text-only.
- Generated one 2×2 sheet rather than four independent images.
- Added source/authority evidence, return-time reference re-hash, Provider
  producer/downstream binding, cape report path/SHA and per-node closure.
- Made cape failure block review, reuse, Pack, catalog and Godot delivery.
- Added exact xAI one-request/one-operation/1.4B-tick authorization validation.

## Offline evidence

The focused fixture contract proves:

- Plan estimate/max = 1/1;
- one `edit_image` request;
- reference roles exactly `[DirectionAnchor]`;
- reference SHA equals approved source `front_idle`;
- prompt states sole authority, rotations only and no skirt-like lower cape hem;
- Lock, cape report and Provider manifest bind the new method.

The two cape unit contracts prove that a plain front plus gold-trimmed rotations
is rejected, while four plain lower hems pass. The historical continuous-gold
contract remains covered and unchanged.

Commands executed:

```text
cargo check --workspace --all-targets --features providers/grid-generation
cargo test -p core front_authoritative_no_skirt_contract -- --nocapture
cargo test -p providers --features grid-generation --test grid_generation_contract fixture_grid_front_authority_uses_one_anchor_and_one_request -- --exact --nocapture
jq empty schemas/cape-hem-consistency-report.schema.json schemas/character-direction-grid-lock.schema.json schemas/direction-grid-import-evidence.schema.json
```

## Real execution boundary

No Plan was executed against a real Provider, no authorization ledger was
created or consumed, and no source Job, Pack, catalog or Godot project was
modified. A real probe remains a separate user-authorized step after Plan-only
review.
