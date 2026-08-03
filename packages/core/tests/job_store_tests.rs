use std::fs;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

use forge_core::job::{JobLifecycleState, JobState, JobStore, SourceKind};
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
        "logs",
        "tools",
        "backups",
    ] {
        assert!(record.job_dir.join(subdir).is_dir(), "missing {subdir}");
    }
}

#[test]
fn legacy_job_json_defaults_automation_fields() {
    let temp = tempdir().unwrap();
    let store = JobStore::new(temp.path()).unwrap();
    let record = store.create_job(SourceKind::ImportFrames).unwrap();
    fs::write(
        record.job_dir.join("job.json"),
        format!(
            r#"{{
  "job_id": "{}",
  "source_kind": "import_frames",
  "state": "created",
  "created_at": "2026-01-01T00:00:00Z",
  "updated_at": "2026-01-01T00:00:00Z",
  "job_dir": "{}",
  "error_summary": null
}}"#,
            record.job_id,
            record.job_dir.display()
        ),
    )
    .unwrap();

    let loaded = store.read_record(&record.job_id).unwrap();

    assert_eq!(loaded.lifecycle_state, JobLifecycleState::Idle);
    assert_eq!(loaded.progress, 0.0);
    assert!(loaded.artifacts.is_empty());
}

#[test]
fn cancellation_and_recent_listing_are_durable() {
    let temp = tempdir().unwrap();
    let store = JobStore::new(temp.path()).unwrap();
    let first = store.create_job(SourceKind::ImportFrames).unwrap();
    let second = store.create_job(SourceKind::ImportSpriteSheet).unwrap();

    let cancelled = store.request_cancellation(&first.job_id).unwrap();
    let records = store.list_records().unwrap();

    assert!(cancelled.cancellation_requested);
    assert_eq!(records.len(), 2);
    assert!(records.iter().any(|record| record.job_id == second.job_id));
}

#[test]
fn concurrent_worker_update_cannot_overwrite_cancellation() {
    let temp = tempdir().unwrap();
    let store = JobStore::new(temp.path()).unwrap();
    let record = store.create_job(SourceKind::ImportFrames).unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let worker_store = store.clone();
    let worker_job_id = record.job_id.clone();
    let worker_barrier = barrier.clone();

    let worker = thread::spawn(move || {
        worker_store
            .update_record(&worker_job_id, |record| {
                worker_barrier.wait();
                thread::sleep(Duration::from_millis(50));
                record.progress = 0.5;
            })
            .unwrap();
    });

    barrier.wait();
    store.request_cancellation(&record.job_id).unwrap();
    worker.join().unwrap();

    let final_record = store.read_record(&record.job_id).unwrap();
    assert!(final_record.cancellation_requested);
    assert_eq!(final_record.progress, 0.5);
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
