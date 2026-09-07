use chrono::{Duration, Utc};
use forge_providers::authorization::{
    AuthorizationManifestV1, AuthorizationStore, AuthorizedTargetV1, ProviderAuthorizationConfig,
    ProviderOperationClassV1,
};

fn manifest() -> AuthorizationManifestV1 {
    let now = Utc::now();
    AuthorizationManifestV1 {
        schema_version: "1".into(),
        authorization_id: "v93-pristine".into(),
        provider_id: "xai".into(),
        profile_id: "default".into(),
        allowed_models: vec!["test-image".into()],
        created_at: now,
        expires_at: now + Duration::minutes(10),
        max_total_requests: 1,
        max_total_provider_operations: Some(1),
        max_total_cost_ticks: 100,
        allowed_targets: vec![AuthorizedTargetV1 {
            target_id: "walk_down:frame:2".into(),
            max_requests: 1,
            cost_reservation_ticks_per_request: 100,
        }],
        source_lineage_root_job_id: Some("source-job".into()),
        recipe_hash: Some("source-recipe".into()),
        input_fingerprint: Some("source-input".into()),
    }
}

#[test]
fn pristine_authorization_rejects_a_released_ledger_entry() {
    let temp = tempfile::tempdir().unwrap();
    let manifest = manifest();
    AuthorizationStore::create(temp.path(), &manifest).unwrap();
    let store = AuthorizationStore::open(temp.path(), &manifest.authorization_id).unwrap();
    store.ensure_active_and_pristine().unwrap();
    let mut config = ProviderAuthorizationConfig::new(temp.path(), &manifest.authorization_id);
    config.lineage_root_job_id = Some("source-job".into());
    config.recipe_hash = Some("source-recipe".into());
    config.input_fingerprint = Some("source-input".into());
    let session = AuthorizationStore::bind(config, "xai", "default").unwrap();
    session
        .reserve_compound(
            "walk_down:frame:2",
            &[("edit_image", ProviderOperationClassV1::ModelGeneration)],
            "test-image",
        )
        .unwrap()
        .remove(0)
        .release()
        .unwrap();

    let error = store.ensure_active_and_pristine().unwrap_err();

    assert!(error.to_string().contains("physically empty"));
}
