# Layered registry correction (PR #32)

Layered Godot installs now register `kind: layered` instead of falling through to
`animation`. Read-only verification rejects a mismatched registered kind. Existing
pre-fix development installs can be corrected with a normal reinstall.

Validation uses synthetic PNGs only. Project registry, layered Pack and delivery
audit Rust tests: 17 passed. `scripts/test-layered-cli.py` passed native installation,
preview, playback, correct persisted kind and rejection of a registry changed to
`animation`. No consumer artwork, toolchain pins or installation was modified.

Test binary identity (source before this fix commit; working tree includes fix):

```json
{
  "build": {
    "gitCommit": "38722ae5b959bfd20ea385c2f23a91e755e6e0fa",
    "dirty": true,
    "target": "aarch64-apple-darwin",
    "profile": "debug",
    "features": []
  },
  "cliSha256": "c211164348d564ee439a7981d94448ebafe203a4e68998383676fc0b5c7877c8",
  "nativeLoad": "passed",
  "playback": "passed"
}
```

Workspace/all-target Clippy 1.98.0 also passed with warnings denied.
