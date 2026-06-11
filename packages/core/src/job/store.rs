use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use thiserror::Error;
use uuid::Uuid;

use super::types::{JobRecord, JobState, SourceKind};

const APP_SUPPORT_DIR: &str = "Game Sprite Forge";
const JOBS_DIR: &str = "jobs";
const JOB_JSON: &str = "job.json";
const JOB_SUBDIRS: [&str; 6] = [
    "source",
    "raw",
    "processed",
    "thumbs",
    "previews",
    "exports",
];

#[derive(Debug, Error)]
pub enum JobStoreError {
    #[error("could not locate the user application support directory")]
    AppSupportDirUnavailable,
    #[error("job id contains filesystem-unsafe characters: {0}")]
    UnsafeJobId(String),
    #[error("job does not exist: {0}")]
    JobNotFound(String),
    #[error("failed filesystem operation for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize job record at {path}: {source}")]
    Serialize {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to deserialize job record at {path}: {source}")]
    Deserialize {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Debug, Clone)]
pub struct JobStore {
    root: PathBuf,
}

impl JobStore {
    pub fn default_app_store() -> Result<Self, JobStoreError> {
        let root = dirs_next::config_dir()
            .ok_or(JobStoreError::AppSupportDirUnavailable)?
            .join(APP_SUPPORT_DIR)
            .join(JOBS_DIR);
        Self::new(root)
    }

    pub fn new(root: impl Into<PathBuf>) -> Result<Self, JobStoreError> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|source| JobStoreError::Io {
            path: root.clone(),
            source,
        })?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn create_job(&self, source_kind: SourceKind) -> Result<JobRecord, JobStoreError> {
        let job_id = Uuid::new_v4().to_string();
        let job_dir = self.job_dir(&job_id)?;
        fs::create_dir_all(&job_dir).map_err(|source| JobStoreError::Io {
            path: job_dir.clone(),
            source,
        })?;

        for subdir in JOB_SUBDIRS {
            let path = job_dir.join(subdir);
            fs::create_dir_all(&path).map_err(|source| JobStoreError::Io { path, source })?;
        }

        let now = Utc::now();
        let record = JobRecord {
            job_id,
            source_kind,
            state: JobState::Created,
            created_at: now,
            updated_at: now,
            job_dir,
            error_summary: None,
        };
        self.write_record(&record)?;
        Ok(record)
    }

    pub fn mark_failed(
        &self,
        job_id: impl AsRef<str>,
        summary: impl Into<String>,
    ) -> Result<JobRecord, JobStoreError> {
        let mut record = self.read_record(job_id.as_ref())?;
        record.state = JobState::Failed;
        record.updated_at = Utc::now();
        record.error_summary = Some(summary.into());
        self.write_record(&record)?;
        Ok(record)
    }

    pub fn set_state(
        &self,
        job_id: impl AsRef<str>,
        state: JobState,
    ) -> Result<JobRecord, JobStoreError> {
        let mut record = self.read_record(job_id.as_ref())?;
        record.state = state;
        record.updated_at = Utc::now();
        self.write_record(&record)?;
        Ok(record)
    }

    fn read_record(&self, job_id: &str) -> Result<JobRecord, JobStoreError> {
        let job_dir = self.job_dir(job_id)?;
        if !job_dir.exists() {
            return Err(JobStoreError::JobNotFound(job_id.to_owned()));
        }

        let path = job_dir.join(JOB_JSON);
        let contents = fs::read_to_string(&path).map_err(|source| JobStoreError::Io {
            path: path.clone(),
            source,
        })?;
        serde_json::from_str(&contents)
            .map_err(|source| JobStoreError::Deserialize { path, source })
    }

    fn write_record(&self, record: &JobRecord) -> Result<(), JobStoreError> {
        let path = record.job_dir.join(JOB_JSON);
        let contents =
            serde_json::to_string_pretty(record).map_err(|source| JobStoreError::Serialize {
                path: path.clone(),
                source,
            })?;
        fs::write(&path, contents).map_err(|source| JobStoreError::Io { path, source })
    }

    fn job_dir(&self, job_id: &str) -> Result<PathBuf, JobStoreError> {
        if !is_filesystem_safe_job_id(job_id) {
            return Err(JobStoreError::UnsafeJobId(job_id.to_owned()));
        }
        Ok(self.root.join(job_id))
    }
}

fn is_filesystem_safe_job_id(job_id: &str) -> bool {
    !job_id.is_empty()
        && job_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}
