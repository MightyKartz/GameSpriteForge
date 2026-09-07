#!/bin/sh
set -eu

unset FORGE_REAL_PROVIDER_ACCEPT || true
unset FORGE_REAL_PROVIDER_MAX_REQUESTS || true
unset FORGE_REAL_PROVIDER_MAX_COST_TICKS || true

cargo test -p core --test grid_feature_gate_tests
cargo test -p core --features grid-generation --test grid_feature_gate_tests
cargo test -p core --test automation_tests
cargo test -p core --test job_store_tests
cargo test -p core --features subject-import --test subject_import_tests
cargo check -p forge-cli
cargo check -p forge-cli --features grid-generation
cargo test -p forge-cli --features grid-generation
cargo test -p providers --features grid-generation --test grid_generation_contract

cargo build -q -p forge-cli --features grid-generation
FEATURE_HELP_OUTPUT="$(mktemp)"
./target/debug/forge --help > "$FEATURE_HELP_OUTPUT"
if ! grep -Eq '^  subject[[:space:]]' "$FEATURE_HELP_OUTPUT"; then
  echo "grid-generation build does not expose the required subject command" >&2
  exit 1
fi
./target/debug/forge subject create --help >/dev/null
./target/debug/forge subject import --help >/dev/null
./target/debug/forge job import-direction-grid --help \
  | grep -q -- '--generator'
./target/debug/forge job import-direction-grid --help \
  | grep -q -- '--cape-hem'
./target/debug/forge provider authorize --help \
  | grep -q -- '--max-provider-operations'
./target/debug/forge provider authorize --help \
  | grep -q -- '--recipe-hash'
./target/debug/forge provider authorize --help \
  | grep -q -- '--input-fingerprint'
./target/debug/forge schema show --id subject-import-report@1.0.0 --json \
  | grep -q 'subject-import@1.0.0'
./target/debug/forge schema show --id character-direction-grid-lock@1.1.0 --json \
  | grep -q 'direction-grid-lock@1.1.0'
./target/debug/forge schema show --id character-direction-grid-appearance-report@1.0.0 --json \
  | grep -q 'direction-grid-appearance@1.0.0'
./target/debug/forge schema show --id grid-action-report@1.1.0 --json \
  | grep -q 'grid-action-report@1.1.0'
./target/debug/forge schema show --id grid-keyframe-action-report@1.0.0 --json \
  | grep -q 'grid-keyframe-action-report@1.0.0'
./target/debug/forge schema show --id grid-keyframe-action-report@1.3.0 --json \
  | grep -q 'grid-keyframe-action-report@1.3.0'
./target/debug/forge schema show --id grid-keyframe-action-report@1.4.0 --json \
  | grep -q 'grid-keyframe-action-report@1.4.0'
./target/debug/forge schema show --id gait-laterality@1.0.0 --json \
  | grep -q 'gait-laterality@1.0.0'
./target/debug/forge schema show --id checkerboard-sheet-matting-report@1.0.0 --json \
  | grep -q 'checkerboard-sheet-matting@1.0.0'
./target/debug/forge schema show --id checkerboard-sheet-matting-report@1.1.0 --json \
  | grep -q 'checkerboard-sheet-matting@1.1.0'
./target/debug/forge schema show --id direction-grid-import-evidence@1.0.0 --json \
  | grep -q 'direction-grid-import@1.0.0'
./target/debug/forge schema show --id direction-grid-import-evidence@1.1.0 --json \
  | grep -q 'direction-grid-import@1.1.0'
./target/debug/forge schema show --id direction-grid-import-consistency@1.0.0 --json \
  | grep -q 'direction-grid-import-consistency@1.0.0'
./target/debug/forge schema show --id direction-grid-authority-alignment@1.0.0 --json \
  | grep -q 'direction-grid-authority-alignment@1.0.0'
./target/debug/forge schema show --id alpha-edge-halo@1.0.0 --json \
  | grep -q 'alpha-edge-halo@1.0.0'
./target/debug/forge schema show --id cape-hem-consistency@1.0.0 --json \
  | grep -q 'cape-hem-consistency@1.0.0'
./target/debug/forge schema show --id character-anchor-stabilization@1.0.0 --json \
  | grep -q 'character-anchor-stabilization@1.0.0'

cargo build -q -p forge-cli
HELP_OUTPUT="$(mktemp)"
./target/debug/forge --help > "$HELP_OUTPUT"
if grep -q "topdown-grid\|grid-generation" "$HELP_OUTPUT"; then
  echo "default --help exposes the grid-generation feature" >&2
  exit 1
fi

# Leave the experimental binary ready for the explicit V9 acceptance path.
cargo build -q -p forge-cli --features grid-generation
