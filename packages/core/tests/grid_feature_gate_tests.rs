use std::collections::BTreeMap;

use forge_core::automation::{
    AutomationOperation, CharacterPackMetadata, CharacterWorkflowSelection,
    GenerateCharacterPackRequest, GeneratedCharacterSpec, GenerationPolicy, PlanStore,
};
use forge_core::character_camera::CharacterCameraProfileV1;

#[test]
fn topdown_grid_is_rejected_without_the_grid_generation_feature() {
    let temp = tempfile::tempdir().unwrap();
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let request = GenerateCharacterPackRequest {
        schema_version: "3".into(),
        provider_id: "fixture".into(),
        profile_id: "default".into(),
        project_path: None,
        asset_id: Some("grid-feature-gate".into()),
        character: GeneratedCharacterSpec {
            prompt: "feature gate".into(),
            reference_image_path: None,
        },
        camera_profile: Some(CharacterCameraProfileV1::TopdownThreeQuarter),
        equipment: Default::default(),
        equipment_explicit: true,
        style_lock_path: None,
        subject_lock_path: None,
        reuse_from_job_dir: None,
        retry_animations: vec![],
        retry_stages: BTreeMap::new(),
        retry_frames: BTreeMap::new(),
        direction_grid_cape_contract: None,
        validation_only: false,
        validation_animations: vec![],
        motion_profile: Default::default(),
        direction_motion_stage: Default::default(),
        metadata: CharacterPackMetadata {
            name: "Grid Feature Gate".into(),
            default_animation: "idle_down".into(),
            creator: "Forge".into(),
            license: "MIT".into(),
            rendering: Default::default(),
        },
        workflow: CharacterWorkflowSelection {
            id: "topdown-grid".into(),
            version: "9.0.0".into(),
        },
        generation: GenerationPolicy::default(),
        normalize: Default::default(),
        sheet: Default::default(),
        quality: Default::default(),
    };
    let error = plans.prepare(AutomationOperation::GenerateCharacterPack(request));
    #[cfg(not(feature = "grid-generation"))]
    {
        let error = error.unwrap_err();
        assert!(error.to_string().contains("grid-generation"));
    }
    #[cfg(feature = "grid-generation")]
    {
        // With the feature enabled, validation proceeds to the next missing
        // immutable input rather than rejecting the workflow itself.
        let error = error.unwrap_err();
        assert!(!error.to_string().contains("grid-generation enabled"));
    }
}
