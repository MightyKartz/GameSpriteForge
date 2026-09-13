use serde::Serialize;

mod compiled {
    include!(concat!(env!("OUT_DIR"), "/build_metadata.rs"));
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildInfo {
    // None is serialized as null: an archive or failed Git query is unknown,
    // never an assertion that the source was clean.
    pub git_commit: Option<&'static str>,
    pub dirty: Option<bool>,
    pub target: &'static str,
    pub profile: &'static str,
    pub features: &'static [&'static str],
}

pub fn current() -> BuildInfo {
    BuildInfo {
        git_commit: compiled::GIT_COMMIT,
        dirty: compiled::DIRTY,
        target: compiled::TARGET,
        profile: compiled::PROFILE,
        features: compiled::FEATURES,
    }
}

// Stable IDs describe compiled CLI entry points; runtime tool availability is
// reported separately by doctor. Add new IDs only alongside their implementation.
pub const CAPABILITIES: &[&str] = &[
    "project_asset_catalog_v3",
    "project_asset_intake_search",
    "project_asset_output_registration",
    "project_asset_retention_delivery",
    "project_asset_preview_review",
    "project_asset_portability",
    "project_asset_integrity_index",
    "project_asset_metadata_merge",
    "project_asset_catalog_migration",
    "local_audio_import",
    "audio_pack_validation",
    "audio_godot_delivery",
    "optional_audio_tool_discovery",
    "local_static_import",
    "static_preserve_source_canvas",
    "single_png_chroma_matting",
    "layered_pack_v1",
    "layered_transform_opacity_tracks",
    "godot_unified_playback",
    "godot_native_preview",
    "effect_blend_modes",
    "local_animation_import",
    "preserve_source_coordinates",
    "local_animation_timing",
    "whole_sheet_source_transform",
    "pack_validation",
    "godot_install",
    "bundled_forge_use_skill",
    "embedded_usage_guide",
    "reviewed_source_hashes",
    "local_request_relative_paths",
    "source_png_inspection",
    "project_image_contract_verification",
    "effect_quality_profile",
    "preview_timing_diagnostics",
    "delivery_receipts",
    "godot_install_verification",
    "transactional_godot_install",
    "godot_import_cache_integrity",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_git_identity_is_explicit_and_other_build_fields_survive() {
        let value = serde_json::to_value(BuildInfo {
            git_commit: None,
            dirty: None,
            target: "test-target",
            profile: "release",
            features: &["map-compiler"],
        })
        .unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "gitCommit": null,
                "dirty": null,
                "target": "test-target",
                "profile": "release",
                "features": ["map-compiler"],
            })
        );
    }
}
