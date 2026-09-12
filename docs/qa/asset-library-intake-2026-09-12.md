# Local resource intake verification

PR 2 adds explicitly scoped scanning, editable batch registration, Pack member
search, immutable history and alternate source locations. Synthetic PNG, PCM WAV,
external binary and generated static Pack fixtures are used; no Sword media or
real Provider was accessed.

Local validation: 12 library integration tests passed, including prior migration
and installation cases, all-or-nothing batch rejection, exact-byte drift,
explicit ID conflicts, idempotency, alternate-location resolution, unavailable
history, excluded caches and member lookup. Strict core/CLI all-target Clippy
with `forge-cli/game-art-manifest` passed. CLI smoke verifies create-new scan
output, read-only queries, duplicate registration, drift refusal and absence of
Job/Plan stores before doctor. Legacy list/inspect output remains compatible.

Availability checks read media bytes on demand; this initial implementation has
no persistent derived search index. Raw extension classification does not imply
decode validation. Review and retention are delivered in subsequent PRs.

Clean verification: `e93daf4d2b06a248a2d1b6c57ecfc3f9276a421a`, `dirty=false`,
`features=[]`, debug `aarch64-apple-darwin`; executable SHA-256
`46f2a8bd8a8917be246d55f44993790767b85d9e8d200118b98707f6a86edb25`.
The expanded CLI smoke passed with the actual preserved legacy executable.
All 30 CLI tests also passed with the game-art-manifest feature.
