# Forge `topdown-grid@9.3.0` real-source preflight

Date: 2026-08-13  
Verdict: **passed — pending Plan and exact authorization only; no real execution**

## Purpose and source boundary

This is a read-only preflight against the actual V9.2 failure source, not a
model validation probe. It proves that the V9.3 implementation can prepare
the one-frame scope and bind it to an exact, empty-ledger authorization without
claiming a Job, starting a worker, or contacting a Provider.

- JobStore: `/Users/kartz/Development/Forge/generated-assets/forge-topdown-grid-v9-real-20260811/jobs`
- eligible source: `0048ce3b-a0ea-4a63-8ed9-9063d01e9043`
- approved DirectionGridLock source: `9ea9cae9-dd94-4b4a-a277-4b5d0345b2ad`
- excluded already-consumed ordinary-fresh child:
  `d080b43c-bb35-4fc0-8f46-ba33aa1a8062`

The source tree SHA-256 was unchanged before and after:
`79dc39bb62c90b6afd2c08ef04c9a40f55f522a0c1c89dc95f2dddf897ae2622`.
The aggregate of Job records was also unchanged before and after:
`1a6b5ccb08ae7ce068f6f17bc2d8dcd1161199289f04fb90254c72a47481a8bb`.
Both counts remained 10 Jobs; top-level entries stayed identical and the
source-scoped claim registry was absent before and after.

## Isolated preflight result

Plan and authorization state were written only to a local isolated preflight
store. Its transient filesystem location is intentionally not a durable
evidence reference. The checked-in machine-readable summary is
[`preflight-summary.json`](artifacts/forge-topdown-grid-v93-preflight-20260813/preflight-summary.json).

The request SHA-256 was
`9d6207fa16628bd91aae0c9b714ecf5dd256092a25a67ae91c84efbd97e51fcf`; the
pending Plan SHA-256 was
`b9d27d641e47181b8a74bc7ea132ebbd8f73e579e0f7c0cac6669e460647121d`.
Only the token digest is retained (never the token itself):
`180c8979f207812f5f2d3d32f7c03142729df08f433034a22453cb148ad292f9`.

The pending Plan was `topdown-grid@9.3.0`, validation-only `walk_down`, with
retry stage `frame`, retry frame `2`, and exactly 1 expected / 1 maximum image
request. Its exact bindings are:

- recipe hash: `404a299c0063bfff3dac80991d00ba670e8177bc4bef9585fb8a7447f9634521`
- input fingerprint: `522b4ef0e2512ac935419d237b844673c667afd928bd3048d9e27f235831797e`

The isolated grant `v93-preflight-only-20260813` was bound to provider `xai`,
profile `default`, model `grok-imagine-image-quality`, target
`walk_down:frame:2`, source lineage root
`833cbbf2-c8c8-4191-bcc6-4a0215d43b64`, and those exact Plan hashes. It allows
only 1 request, 1 Provider operation, and `1,400,000,000` total/reserved cost
ticks. Its manifest SHA-256 is
`13978f0850618853c59fd2f7a8a3e2fbc94a224d95e374440ca068b3ecf510a6`; its
ledger SHA-256 is
`72c480318aa567e194c7a3826796ce0224a4fe9407e887e0e1a2442db75eb102`, with
zero ledger requests.

## Explicit non-actions

No Job was created, no stage or source claim was made, no worker ran, no
Provider or network request was issued, and no real authorization store was
mutated. The isolated grant is short-lived preflight evidence only; it expires
and is not an authorization for a future probe. Any real `walk_down` frame-2
execution still requires a new, independent user authorization.

## Offline implementation evidence

At this preflight point, the latest focused offline results were: full Grid
contracts 26/26, JobStore 13, Automation 17, Providers 44, CLI 17, plus
focused schema/reference/authorization contracts. Workspace-wide and product
release gates remain separately in progress and are not asserted by this
preflight.
