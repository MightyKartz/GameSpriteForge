use forge_core::automation::{
    local_source_files, validate_source_locks, AutomationOperation, PlanStore, PrepareAssetRequest,
    QualityPolicy,
};
use image::{Rgba, RgbaImage};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn source(root: &Path, name: &str) -> PathBuf {
    let path = root.join(name);
    RgbaImage::from_pixel(4, 4, Rgba([200, 100, 0, 255]))
        .save(&path)
        .unwrap();
    path
}
fn lock(path: &Path) -> Value {
    json!({"path":path,"sha256":forge_core::delivery::hash_file(path).unwrap()})
}
fn operation(paths: &[PathBuf], locks: Value) -> AutomationOperation {
    let request:PrepareAssetRequest=serde_json::from_value(json!({"input":{"kind":"png_sequence","paths":paths},"metadata":{"name":"Locked"},"sourceLocks":locks})).unwrap();
    AutomationOperation::PrepareAsset(request)
}

#[test]
fn source_locks_accept_exact_closure_case_insensitive_hashes_and_repeated_input_references() {
    let root = tempfile::tempdir().unwrap();
    let a = source(root.path(), "a.png");
    let b = source(root.path(), "b.png");
    let mut uppercase = lock(&a);
    uppercase["sha256"] = json!(uppercase["sha256"].as_str().unwrap().to_ascii_uppercase());
    let op = operation(
        &[a.clone(), b.clone(), a.clone()],
        json!([uppercase, lock(&b)]),
    );
    validate_source_locks(&op).unwrap();
    assert_eq!(local_source_files(&op).unwrap().len(), 2);
    validate_source_locks(&operation(&[a, b], json!([]))).unwrap();
}

#[test]
fn nonempty_source_locks_reject_partial_unrelated_duplicate_and_malformed_entries() {
    let root = tempfile::tempdir().unwrap();
    let a = source(root.path(), "a.png");
    let b = source(root.path(), "b.png");
    let other = source(root.path(), "other.png");
    for locks in [
        json!([lock(&a)]),
        json!([lock(&a), lock(&b), lock(&other)]),
        json!([lock(&a), lock(&a), lock(&b)]),
        json!([{"path":a,"sha256":"not-sha256"},lock(&b)]),
        json!([{"path":a,"sha256":"g".repeat(64)},lock(&b)]),
    ] {
        assert!(validate_source_locks(&operation(&[a.clone(), b.clone()], locks)).is_err());
    }
}

#[test]
fn reviewed_hash_mismatch_prevents_plan_creation_and_source_changes_prevent_claim() {
    let root = tempfile::tempdir().unwrap();
    let a = source(root.path(), "a.png");
    let b = source(root.path(), "b.png");
    let locked = operation(&[a.clone(), b.clone()], json!([lock(&a), lock(&b)]));
    let plans_root = root.path().join("plans");
    let plans = PlanStore::new(plans_root.clone()).unwrap();
    let prepared = plans.prepare(locked.clone()).unwrap();
    RgbaImage::from_pixel(4, 4, Rgba([0, 0, 255, 255]))
        .save(&a)
        .unwrap();
    let before = fs::read_dir(&plans_root).unwrap().count();
    assert!(plans
        .prepare(locked)
        .unwrap_err()
        .to_string()
        .contains("reviewed source SHA-256 mismatch"));
    assert_eq!(fs::read_dir(&plans_root).unwrap().count(), before);
    assert!(plans
        .claim(&prepared.token)
        .unwrap_err()
        .to_string()
        .contains("reviewed source SHA-256 mismatch"));
}

#[cfg(unix)]
#[test]
fn source_locks_detect_alias_duplicates_and_pack_symlinks() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let a = source(root.path(), "a.png");
    let alias = root.path().join("alias.png");
    symlink(&a, &alias).unwrap();
    let op = operation(&[a.clone(), alias.clone()], json!([lock(&a), lock(&alias)]));
    assert!(validate_source_locks(&op)
        .unwrap_err()
        .contains("duplicate paths"));
    let pack = root.path().join("pack.gsfpack");
    fs::create_dir(&pack).unwrap();
    symlink(&a, pack.join("source.png")).unwrap();
    let request:PrepareAssetRequest=serde_json::from_value(json!({"input":{"kind":"gsfpack","path":pack},"metadata":{"name":"Pack"},"sourceLocks":[lock(&a)]})).unwrap();
    assert!(
        validate_source_locks(&AutomationOperation::PrepareAsset(request))
            .unwrap_err()
            .contains("symbolic link")
    );
}

#[test]
fn source_lock_closure_expands_every_pack_file() {
    let root = tempfile::tempdir().unwrap();
    let pack = root.path().join("pack.gsfpack");
    fs::create_dir(&pack).unwrap();
    fs::create_dir(pack.join("nested")).unwrap();
    let first = pack.join("metadata.json");
    let second = pack.join("nested/source.png");
    fs::write(&first, "{}").unwrap();
    fs::write(&second, "bytes").unwrap();
    let request:PrepareAssetRequest=serde_json::from_value(json!({"input":{"kind":"gsfpack","path":pack},"metadata":{"name":"Pack"},"sourceLocks":[lock(&first),lock(&second)]})).unwrap();
    validate_source_locks(&AutomationOperation::PrepareAsset(request.clone())).unwrap();
    fs::write(pack.join("new-file.txt"), "changed closure").unwrap();
    assert!(
        validate_source_locks(&AutomationOperation::PrepareAsset(request))
            .unwrap_err()
            .contains("cover every local source")
    );
}

#[test]
fn default_quality_fields_keep_legacy_request_serialization_stable() {
    let legacy = json!({"requireGameReady":true});
    let policy: QualityPolicy = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(serde_json::to_value(policy).unwrap(), legacy);
    let effect = json!({"requireGameReady":true,"profile":"effect","allowTransparentTail":true});
    let policy: QualityPolicy = serde_json::from_value(effect.clone()).unwrap();
    assert_eq!(serde_json::to_value(policy).unwrap(), effect);
}
