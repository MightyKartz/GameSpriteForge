# Survival feedback implementation plan

Status: implemented in [PR #56](https://github.com/MightyKartz/GameSpriteForge/pull/56).
Local acceptance passed; native CI is tracked by the PR.
Baseline: `546bcef` (Forge 0.6.3).
Evidence: [consumer audit](../qa/survival-feedback-2026-09-21.md).
Results: [implementation verification](../qa/survival-delivery-implementation-2026-09-21.md).

## This PR

1. Align the public Godot toolchain-lock schema with runtime 4.6/4.7 support,
   including patchless versions. Test serialized runtime locks against the schema.
2. Add `forge storage check --path DIRECTORY --json`: an explicitly requested,
   isolated write probe for exclusive publication, existing-file protection,
   replacement, hard links and file locking. Clean up its own temporary directory.
   Report unsupported operations without changing caller files or selecting a
   weaker publication method. Keep ordinary `doctor` behavior unchanged.
3. Improve mixed preserved-canvas errors with measured item dimensions and
   explicit grouping guidance. Distinguish project-import, resource-install and
   native-verification failures in durable Job errors and identify retained logs.
4. Ship an embedded Python local-delivery example: require an explicit binary
   hash and reviewed source locks, check capabilities/storage, prepare locally,
   retain the validated Pack, install directly into the selected project, export
   standard preparation/delivery receipts and verify the final installation.
   Record recoverable progress; never recursively copy `.forge` between projects.
5. Improve embedded guidance for role-specific alpha/grid checks, game-size
   preview, immutable reruns, source locks, partial failures and progressive
   resource-library adoption. Extend the existing external-clock example's
   guidance without adding game-specific combat rules.

## Acceptance

- Formatting and focused Rust tests pass; schema accepts exactly the supported
  version grammar. Existing locks are never rewritten by reads.
- Storage tests preserve sentinel files and deny a second publication; native
  ExFAT probing reports unsupported operations and cleans up. It does not certify
  an entire filesystem or guarantee power-loss durability.
- A synthetic delivery exercises the shipped example against native Godot,
  verifies final installed files/caches, preserves zero Provider requests and
  verifies retained receipts after removal of the isolated JobStore.
- Wrong binary hash, missing reviewed source locks and existing output directory
  fail before production; an interrupted/failed delivery retains its prepared
  Pack, receipts and Job IDs. Existing owned resources remain protected by the
  existing installer transaction.
- Embedded guide integrity/install checks pass; CI runs the new contracts on
  macOS and Windows. Native ExFAT evidence is local and reported separately.

## Deferred design boundary

Full ExFAT transactional delivery is a subsequent change, not an implicit
fallback in this PR. It must specify target ownership, locking, crash recovery,
deletion inventory, registry/cache publication, concurrent writers and explicit
final verification. Adding copy-after-staging would bypass the current safety
contract. Multi-size preview UI, automatic batch grouping, animation generation
and weapon rigging are likewise outside this bounded reliability change.

## Delivery

Commit only implementation, guides, synthetic tests and compact evidence. Record
the tested source identity and binary hash, open a PR against the fetched main,
and attach it to the task. PR creation does not authorize merging or release.
