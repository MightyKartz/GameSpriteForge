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

#[test]
fn named_child_scope_is_atomic_and_single_use() {
    let temp = tempdir().unwrap();
    let store = JobStore::new(temp.path()).unwrap();
    let source = store.create_job(SourceKind::FromCode).unwrap();
    let scope = "topdown-grid@9.3.0:walk_down:frame:2";
    let source_entries_before = fs::read_dir(&source.job_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<std::collections::BTreeSet<_>>();

    let first = store
        .create_claimed_child_job(&source.job_dir, &source.job_id, scope, SourceKind::FromCode)
        .unwrap();
    let error = store
        .create_claimed_child_job(&source.job_dir, &source.job_id, scope, SourceKind::FromCode)
        .unwrap_err();

    assert!(error.to_string().contains(&first.job_id));
    assert_eq!(store.list_records().unwrap().len(), 2);
    let source_entries_after = fs::read_dir(&source.job_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(source_entries_after, source_entries_before);
    assert!(source
        .job_dir
        .parent()
        .unwrap()
        .join(".forge-child-scope-claims")
        .join(format!("{}.json", source.job_id))
        .is_file());
}

#[test]
fn named_child_scope_rejects_unsafe_or_directory_mismatched_source_ids() {
    let temp = tempdir().unwrap();
    let source_store = JobStore::new(temp.path().join("source-store")).unwrap();
    let destination = JobStore::new(temp.path().join("destination")).unwrap();
    let source = source_store.create_job(SourceKind::FromCode).unwrap();

    let unsafe_error = destination
        .create_claimed_child_job(
            &source.job_dir,
            "../../escape",
            "topdown-grid@9.3.0:walk_down:frame:2",
            SourceKind::FromCode,
        )
        .unwrap_err();
    assert!(matches!(
        unsafe_error,
        forge_core::job::JobStoreError::UnsafeJobId(_)
    ));

    let mismatch_error = destination
        .create_claimed_child_job(
            &source.job_dir,
            "different-safe-id",
            "topdown-grid@9.3.0:walk_down:frame:2",
            SourceKind::FromCode,
        )
        .unwrap_err();
    assert!(matches!(
        mismatch_error,
        forge_core::job::JobStoreError::JobNotFound(_)
    ));
    assert!(destination.list_records().unwrap().is_empty());
}

#[test]
fn concurrent_named_child_scope_creates_exactly_one_child() {
    let temp = tempdir().unwrap();
    let store = JobStore::new(temp.path()).unwrap();
    let source = store.create_job(SourceKind::FromCode).unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let mut workers = Vec::new();
    for _ in 0..2 {
        let store = store.clone();
        let source_dir = source.job_dir.clone();
        let source_id = source.job_id.clone();
        let barrier = barrier.clone();
        workers.push(thread::spawn(move || {
            barrier.wait();
            store.create_claimed_child_job(
                &source_dir,
                &source_id,
                "topdown-grid@9.3.0:walk_down:frame:2",
                SourceKind::FromCode,
            )
        }));
    }
    barrier.wait();
    let results = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
    assert_eq!(store.list_records().unwrap().len(), 2);
}

#[test]
fn named_child_scope_is_single_use_across_distinct_job_stores() {
    let temp = tempdir().unwrap();
    let source_store = JobStore::new(temp.path().join("source-store")).unwrap();
    let destination_a = JobStore::new(temp.path().join("destination-a")).unwrap();
    let destination_b = JobStore::new(temp.path().join("destination-b")).unwrap();
    let source = source_store.create_job(SourceKind::FromCode).unwrap();
    let scope = "topdown-grid@9.3.0:walk_down:frame:2";

    let first = destination_a
        .create_claimed_child_job(&source.job_dir, &source.job_id, scope, SourceKind::FromCode)
        .unwrap();
    let error = destination_b
        .create_claimed_child_job(&source.job_dir, &source.job_id, scope, SourceKind::FromCode)
        .unwrap_err();

    assert!(error.to_string().contains(&first.job_id));
    assert_eq!(destination_a.list_records().unwrap().len(), 1);
    assert!(destination_b.list_records().unwrap().is_empty());
}

#[test]
fn concurrent_distinct_job_stores_create_only_one_child_for_source_scope() {
    let temp = tempdir().unwrap();
    let source_store = JobStore::new(temp.path().join("source-store")).unwrap();
    let destination_a = JobStore::new(temp.path().join("destination-a")).unwrap();
    let destination_b = JobStore::new(temp.path().join("destination-b")).unwrap();
    let source = source_store.create_job(SourceKind::FromCode).unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let mut workers = Vec::new();
    for store in [destination_a.clone(), destination_b.clone()] {
        let source_dir = source.job_dir.clone();
        let source_id = source.job_id.clone();
        let barrier = barrier.clone();
        workers.push(thread::spawn(move || {
            barrier.wait();
            store.create_claimed_child_job(
                &source_dir,
                &source_id,
                "topdown-grid@9.3.0:walk_down:frame:2",
                SourceKind::FromCode,
            )
        }));
    }
    barrier.wait();
    let results = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
    assert_eq!(
        destination_a.list_records().unwrap().len() + destination_b.list_records().unwrap().len(),
        1
    );
}
