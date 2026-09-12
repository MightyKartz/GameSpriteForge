# Retention and exact-version delivery verification

PR 4 implements retained media, explicit selection, separate consumer resource
locks and exact-version inputs to the existing Godot installation Plan.

Fifteen library tests passed, including source removal, retained-byte corruption,
idempotent retention, Plan rejection after lock drift and consumer locks remaining unchanged after a new revision.
Five existing installation transaction tests passed. A new real Godot 4.6.3 fixture
retains two independently produced revisions, relocates both production Job stores,
and installs A → B → A using a logical asset ID different from the Pack ID. All
three native verifications passed, with three exact historical associations. A
failed next installation restored both native target bytes and the catalog head.

Three static CLI cases exercised bound production, immutable publication recovery,
retention, consumer locks, complete two-member Pack impact descriptions and native
delivery. Local tests use synthetic media only; consumer projects and toolchain pins
are untouched. Publication journal concurrency was fixed in parent PR #37.

Exact native installation associations retain their real Job ID and original installation snapshot text/SHA-256, so later installs do not erase historical receipt bytes. Legacy declarations remain explicitly unverified.

Clean default-feature build identity: `27dd818163031c2bbec366c6bd3a6e36a612b1ae`,
`dirty=false`, `features=[]`, debug `aarch64-apple-darwin`, binary SHA-256
`dd060b0c75f28552ee9b0d7df08e2be1eddebd46c865c6737091d7d2c699c60f`.
Library smoke and three static CLI/lock/retention/native Godot cases passed on
this executable. All five real Godot transaction cases, 30 CLI tests, strict
all-target Clippy and ten build reuse tests also passed during development.
