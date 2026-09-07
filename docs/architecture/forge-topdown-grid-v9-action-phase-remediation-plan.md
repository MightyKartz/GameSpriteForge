# Forge topdown-grid V9 Action Grid phase remediation

Date: 2026-08-12

## Decision

Keep `topdown-grid@9.0.0` image-only. The real `walk_down` failure was not a
request-count or threshold problem: both generated 2x2 sheets contained only
two effective pose families. Forge now treats an Action Grid as a four-phase
walk contract and uses the second allowed request to edit the failed sheet
rather than asking for an unrelated reroll.

## Four-phase contract

Each walk sheet contains exactly four ordered cells:

1. left contact;
2. left passing;
3. right contact;
4. right passing.

The lower body must provide four distinct silhouettes and alternating support.
The head, hood, shoulders, cape attachment, torso and body center remain stable;
arm motion is secondary to the leg cycle. This is a generic motion contract,
not an Ayla-specific costume rule.

`grid-action-report@1.1.0` records `grid-action-phases@1.0.0` evidence, source
cell indices, generation method and diagnostic-retry provenance. Reports with
duplicate cells, weak phase order, insufficient opposing contact or incomplete
repair provenance cannot be reused.

## Diagnostic retry

Attempt 1 uses references in this order:

```text
SubjectIdentity -> DirectionAnchor -> PoseStructure
```

If deterministic motion or equipment analysis fails, attempt 2 uses:

```text
EditTarget(failed sheet) -> DirectionAnchor -> PoseStructure
```

The prompt receives only deterministic reason codes and affected cell indices.
Motion and equipment reasons are aggregated into the same correction so the
second request cannot fix the gait while silently retaining a glove, floating
item or undeclared equipment defect.

Forge verifies the EditTarget SHA-256 immediately before submission and requires
the Provider result to materialize at the exact per-attempt output path. Input
or output integrity failures use stable error codes and are persisted into the
attempt ledger and failed Provider manifest.

## Local gates and provenance

- every action attempt is recorded before the Provider call and updated through
  transport, media validation, extraction, normalization and semantic quality;
- completed actions are persisted immediately, so a later direction failure
  does not erase prior evidence;
- targeted cell retry reruns both motion and per-frame equipment checks;
- all reused 1.0 and 1.1 reports rerun the current local motion/equipment gates;
- Provider reference capacity is checked before any request: three references
  with pose guidance, two without it;
- failed jobs still emit `action-attempts.json` and
  `source/grid-provider-manifest.json` with observed usage.

## Acceptance gate

The offline fixture gate must prove:

- a collapsed first sheet is repaired by editing that exact sheet;
- malformed sheets and incorrect Provider output paths fail closed with complete
  provenance;
- a one-cell glove or equipment defect cannot enter Pack export;
- Pack, targeted retry and Godot installation continue to work;
- xAI multipart references preserve the declared semantic order.

This remediation makes no real Provider request. The next paid step, if
authorized, should be one `walk_down:action_grid` probe with at most two image
edits. The other directions remain blocked until that probe passes native review.
