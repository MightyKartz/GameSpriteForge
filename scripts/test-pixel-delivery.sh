#!/bin/sh
set -eu

unset FORGE_REAL_PROVIDER_ACCEPT || true
unset FORGE_REAL_PROVIDER_MAX_REQUESTS || true
unset FORGE_REAL_PROVIDER_MAX_COST_TICKS || true

cargo test -p core pixel_grid
cargo test -p core --features pixel-delivery-v2 pixel_delivery
cargo check -p core
cargo check -p core --features pixel-delivery-v2

# Feature-off compatibility: V6, V7, and V8 all exercise their existing Pack
# generation paths through the default build, where pixel-delivery-v2 is not
# linked and the legacy Lanczos3 branch remains the only active branch.
cargo test -p providers --test video_cycle_generation_contract fixture_v6_full_generation_uses_one_direction_lock_and_four_master_videos
cargo test -p providers --test video_cycle_generation_contract fixture_v7_full_generation_uses_direction_and_pose_edits_before_three_videos
cargo test -p providers --test direction_motion_generation_contract fixture_v8_complete_stage_preserves_source_cycles_and_exports_eight_animations
