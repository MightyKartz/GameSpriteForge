use std::fs;

use forge_core::job::{JobState, JobStore, SourceKind};
use tempfile::tempdir;

#[test]
fn new_job_creates_all_directories() {
    let temp = tempdir().unwrap();
    let store = JobStore::new(temp.path()).unwrap();

    let record = store.create_job(SourceKind::ImportVideo).unwrap();

    assert!(record.job_dir.is_dir());
    assert!(record.job_dir.join("job.json").is_file());
    for subdir in [
        "source",
        "raw",
        "processed",
        "thumbs",
        "previews",
        "exports",
    ] {
        assert!(record.job_dir.join(subdir).is_dir(), "missing {subdir}");
    }
}

#[test]
fn job_json_contains_source_kind() {
    let temp = tempdir().unwrap();
    let store = JobStore::new(temp.path()).unwrap();

    let record = store.create_job(SourceKind::ImportSpriteSheet).unwrap();
    let json = fs::read_to_string(record.job_dir.join("job.json")).unwrap();

    assert!(json.contains(r#""source_kind": "import_sprite_sheet""#));
}

#[test]
fn failed_job_stores_error_summary() {
    let temp = tempdir().unwrap();
    let store = JobStore::new(temp.path()).unwrap();
    let record = store.create_job(SourceKind::ImportFrames).unwrap();

    let failed = store
        .mark_failed(&record.job_id, "ffmpeg could not read source")
        .unwrap();
    let json = fs::read_to_string(record.job_dir.join("job.json")).unwrap();

    assert_eq!(failed.state, JobState::Failed);
    assert_eq!(
        failed.error_summary.as_deref(),
        Some("ffmpeg could not read source")
    );
    assert!(json.contains(r#""state": "failed""#));
    assert!(json.contains(r#""error_summary": "ffmpeg could not read source""#));
}

#[test]
fn set_state_persists_job_progress() {
    let temp = tempdir().unwrap();
    let store = JobStore::new(temp.path()).unwrap();
    let record = store.create_job(SourceKind::ImportVideo).unwrap();

    let updated = store
        .set_state(&record.job_id, JobState::FramesExtracted)
        .unwrap();
    let json = fs::read_to_string(record.job_dir.join("job.json")).unwrap();

    assert_eq!(updated.state, JobState::FramesExtracted);
    assert!(json.contains(r#""state": "frames_extracted""#));
}

#[test]
fn job_ids_are_filesystem_safe() {
    let temp = tempdir().unwrap();
    let store = JobStore::new(temp.path()).unwrap();

    let record = store.create_job(SourceKind::ImportGsfpack).unwrap();

    assert!(!record.job_id.is_empty());
    assert!(record
        .job_id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'));
    assert_eq!(record.job_dir, temp.path().join(&record.job_id));
}
