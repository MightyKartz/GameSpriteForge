#!/bin/sh
set -eu

unset FORGE_REAL_PROVIDER_ACCEPT || true
unset FORGE_REAL_PROVIDER_MAX_REQUESTS || true
unset FORGE_REAL_PROVIDER_MAX_COST_TICKS || true
unset XAI_API_KEY || true

cargo test -p core --features subject-import --test subject_import_tests
cargo test -p forge-cli --features subject-import \
  tests::subject_import_requires_neither_provider_resolution_nor_authorization_targets -- --exact
cargo check -p forge-cli --features subject-import
cargo check -p forge-cli --features grid-generation

cargo build -q -p forge-cli --features subject-import
./target/debug/forge subject import --help >/dev/null
./target/debug/forge schema show --id subject-import-report@1.0.0 --json \
  | grep -q 'subject-import@1.0.0'

cargo build -q -p forge-cli
if ./target/debug/forge subject import --help >/dev/null 2>&1; then
  echo "default build exposes the experimental subject import command" >&2
  exit 1
fi

# Leave the binary ready for the V9 two-stage acceptance workflow.
cargo build -q -p forge-cli --features grid-generation
