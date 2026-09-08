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
    "local_static_import",
    "local_animation_import",
    "preserve_source_coordinates",
    "local_animation_timing",
    "whole_sheet_source_transform",
    "pack_validation",
    "godot_install",
    "bundled_forge_use_skill",
    "embedded_usage_guide",
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
