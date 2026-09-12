use std::{fs, path::Path};

use forge_core::{
    audio::PrepareAudioRequest,
    automation::{run_operation, stage_plan_job, AutomationOperation, PlanStore, SourceLock},
    delivery::hash_file,
    job::{JobLifecycleState, JobOperationKind, JobStore, SourceKind},
};
use serde_json::json;

fn request(root: &Path) -> PrepareAudioRequest {
    let path = root.join("tone.wav");
    let pcm = (0..800)
        .flat_map(|i| ((i % 100) as i16 * 100 - 5000).to_le_bytes())
        .collect::<Vec<_>>();
    let mut wav = Vec::new();
    wav.extend(b"RIFF");
    wav.extend((36 + pcm.len() as u32).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(8000u32.to_le_bytes());
    wav.extend(16000u32.to_le_bytes());
    wav.extend(2u16.to_le_bytes());
    wav.extend(16u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend((pcm.len() as u32).to_le_bytes());
    wav.extend(pcm);
    fs::write(&path, wav).unwrap();
    serde_json::from_value(
        json!({"id":"sounds", "name":"Sounds", "items":[{"id":"tone","role":"sfx","path":path}]}),
    )
    .unwrap()
}

#[test]
fn audio_plan_is_single_use_zero_provider_and_binds_source_bytes() {
    let root = tempfile::tempdir().unwrap();
    let request = request(root.path());
    let plans = PlanStore::new(root.path().join("plans")).unwrap();
    let operation = AutomationOperation::PrepareAudio(request.clone());
    let prepared = plans.prepare(operation.clone()).unwrap();
    assert_eq!(prepared.estimate.maximum_provider_requests, 0);
    assert_eq!(prepared.estimate.provider_request_estimate, 0);
    assert!(prepared.estimate.provider_id.is_none());
    let plan = plans.claim(&prepared.token).unwrap();
    assert!(plans.claim(&prepared.token).is_err());
    let jobs = JobStore::new(root.path().join("jobs")).unwrap();
    let queued = stage_plan_job(&jobs, &plan).unwrap();
    assert_eq!(queued.source_kind, SourceKind::ImportAudio);
    assert_eq!(queued.operation_kind, JobOperationKind::PrepareAudio);
    let pending = plans.prepare(operation).unwrap();
    let mut wav = fs::read(&request.items[0].path).unwrap();
    wav[44] ^= 1;
    fs::write(&request.items[0].path, wav).unwrap();
    assert!(plans
        .claim(&pending.token)
        .unwrap_err()
        .to_string()
        .contains("input changed"));
    assert!(run_operation(&jobs, &queued.job_id, &plan.operation).is_err());
    let failed = jobs.read_record(&queued.job_id).unwrap();
    assert_eq!(failed.lifecycle_state, JobLifecycleState::Failed);
    assert!(!failed
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack"));
}

#[test]
fn audio_reviewed_hashes_require_complete_matching_source_set() {
    let root = tempfile::tempdir().unwrap();
    let mut request = request(root.path());
    let plans = PlanStore::new(root.path().join("plans")).unwrap();
    request.source_locks = vec![SourceLock {
        path: request.items[0].path.clone(),
        sha256: hash_file(&request.items[0].path).unwrap().to_uppercase(),
    }];
    plans
        .prepare(AutomationOperation::PrepareAudio(request.clone()))
        .unwrap();
    request.source_locks[0].sha256 = "0".repeat(64);
    assert!(plans
        .prepare(AutomationOperation::PrepareAudio(request.clone()))
        .is_err());
    request.source_locks[0].sha256 = hash_file(&request.items[0].path).unwrap();
    request.source_locks.push(request.source_locks[0].clone());
    assert!(plans
        .prepare(AutomationOperation::PrepareAudio(request))
        .is_err());
}

#[test]
fn cancelled_audio_job_does_not_publish_pack() {
    let root = tempfile::tempdir().unwrap();
    let operation = AutomationOperation::PrepareAudio(request(root.path()));
    let plans = PlanStore::new(root.path().join("plans")).unwrap();
    let pending = plans.prepare(operation).unwrap();
    let plan = plans.claim(&pending.token).unwrap();
    let jobs = JobStore::new(root.path().join("jobs")).unwrap();
    let queued = stage_plan_job(&jobs, &plan).unwrap();
    jobs.request_cancellation(&queued.job_id).unwrap();
    let done = run_operation(&jobs, &queued.job_id, &plan.operation).unwrap();
    assert_eq!(done.lifecycle_state, JobLifecycleState::Cancelled);
    assert!(!done
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack"));
}
