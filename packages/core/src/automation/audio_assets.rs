//! Local audio preparation through the same immutable Plan and Job contract.
use std::fs;

use super::{fingerprint_operation_inputs, AutomationOperation, AutomationRunError};
use crate::audio::{prepare_audio_cancellable, PrepareAudioRequest};
use crate::delivery::{directory_sha256, hash_file};
use crate::job::{JobArtifactRecord, JobLifecycleState, JobRecord, JobState, JobStore};

pub(super) fn run_prepare_audio(
    store: &JobStore,
    job_id: &str,
    request: &PrepareAudioRequest,
) -> Result<JobRecord, AutomationRunError> {
    let record = store.read_record(job_id)?;
    let operation = AutomationOperation::PrepareAudio(request.clone());
    let fingerprint = fingerprint_operation_inputs(&operation)?;
    if record.input_hash.as_deref() != Some(&fingerprint) {
        return Err(AutomationRunError::Processing(
            "local audio inputs changed after planning".into(),
        ));
    }
    let exports = record.job_dir.join("exports");
    fs::create_dir_all(&exports)?;
    let pack_path = exports.join(format!("{}.gsfpack", request.id));
    let cancelled = || {
        store
            .read_record(job_id)
            .map_or(true, |r| r.cancellation_requested)
    };
    let prepared =
        prepare_audio_cancellable(request, &pack_path, cancelled).map_err(|message| {
            if cancelled() {
                AutomationRunError::Cancelled
            } else {
                AutomationRunError::Processing(message)
            }
        })?;
    let verification = (|| {
        if cancelled() {
            return Err(AutomationRunError::Cancelled);
        }
        if fingerprint_operation_inputs(&operation)? != fingerprint {
            return Err(AutomationRunError::Processing(
                "local audio inputs changed during import".into(),
            ));
        }
        let hashes = (
            directory_sha256(&prepared.pack_path)?,
            hash_file(&prepared.quality_report_path)?,
        );
        if cancelled() {
            return Err(AutomationRunError::Cancelled);
        }
        Ok(hashes)
    })();
    let (pack_hash, report_hash) = match verification {
        Ok(hashes) => hashes,
        Err(error) => {
            // The preparation call created this new job-owned directory.
            let _ = fs::remove_dir_all(&prepared.pack_path);
            return Err(error);
        }
    };
    let published = store.update_record(job_id, |record| {
        if record.cancellation_requested {
            return;
        }
        record.state = JobState::Exported;
        record.lifecycle_state = JobLifecycleState::Succeeded;
        record.progress = 1.0;
        record.worker_pid = None;
        for step in &mut record.steps {
            step.state = "succeeded".into();
        }
        record.artifacts.extend([
            JobArtifactRecord {
                kind: "gsfpack".into(),
                path: prepared.pack_path.clone(),
                sha256: Some(pack_hash.clone()),
            },
            JobArtifactRecord {
                kind: "quality_report".into(),
                path: prepared.quality_report_path.clone(),
                sha256: Some(report_hash.clone()),
            },
        ]);
        record.next_actions = vec![
            "inspect_asset".into(),
            "listen_audio".into(),
            "plan_install_godot".into(),
        ];
    });
    match published {
        Ok(record) if record.cancellation_requested => {
            let _ = fs::remove_dir_all(&prepared.pack_path);
            Err(AutomationRunError::Cancelled)
        }
        Ok(record) => Ok(record),
        Err(error) => {
            let _ = fs::remove_dir_all(&prepared.pack_path);
            Err(error.into())
        }
    }
}
