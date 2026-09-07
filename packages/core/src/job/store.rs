use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

use chrono::Utc;
use thiserror::Error;
use uuid::Uuid;

use super::types::{JobLifecycleState, JobOperationKind, JobRecord, JobState, SourceKind};

const APP_SUPPORT_DIR: &str = "Game Sprite Forge";
const JOBS_DIR: &str = "jobs";
const JOB_JSON: &str = "job.json";
const JOB_LOCK: &str = ".job.lock";
const CHILD_SCOPE_CLAIMS_DIR: &str = ".forge-child-scope-claims";
pub const JOB_WORKSPACE_JSON: &str = "workspace.json";
const JOB_SUBDIRS: [&str; 9] = [
    "source",
    "raw",
    "processed",
    "thumbs",
    "previews",
    "exports",
    "logs",
    "tools",
    "backups",
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
    #[error("child scope {scope} was already claimed by job {child_job_id}")]
    ChildScopeAlreadyClaimed { scope: String, child_job_id: String },
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
        self.create_job_with_id(source_kind, Uuid::new_v4().to_string())
    }

    pub fn mark_failed(
        &self,
        job_id: impl AsRef<str>,
        summary: impl Into<String>,
    ) -> Result<JobRecord, JobStoreError> {
        let summary = summary.into();
        self.update_record(job_id.as_ref(), |record| {
            record.state = JobState::Failed;
            record.lifecycle_state = JobLifecycleState::Failed;
            record.error_summary = Some(summary);
        })
    }

    pub fn set_state(
        &self,
        job_id: impl AsRef<str>,
        state: JobState,
    ) -> Result<JobRecord, JobStoreError> {
        self.update_record(job_id.as_ref(), |record| record.state = state)
    }

    pub fn read_record(&self, job_id: &str) -> Result<JobRecord, JobStoreError> {
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

    pub fn write_record(&self, record: &JobRecord) -> Result<(), JobStoreError> {
        let _lock = self.lock_record(&record.job_dir)?;
        self.write_record_unlocked(record)
    }

    fn write_record_unlocked(&self, record: &JobRecord) -> Result<(), JobStoreError> {
        let path = record.job_dir.join(JOB_JSON);
        let contents =
            serde_json::to_string_pretty(record).map_err(|source| JobStoreError::Serialize {
                path: path.clone(),
                source,
            })?;
        let temporary = record
            .job_dir
            .join(format!(".{JOB_JSON}.{}.tmp", Uuid::new_v4()));
        fs::write(&temporary, contents).map_err(|source| JobStoreError::Io {
            path: temporary.clone(),
            source,
        })?;
        fs::rename(&temporary, &path).map_err(|source| JobStoreError::Io { path, source })
    }

    pub fn update_record<F>(&self, job_id: &str, update: F) -> Result<JobRecord, JobStoreError>
    where
        F: FnOnce(&mut JobRecord),
    {
        let job_dir = self.job_dir(job_id)?;
        if !job_dir.is_dir() {
            return Err(JobStoreError::JobNotFound(job_id.to_owned()));
        }
        let _lock = self.lock_record(&job_dir)?;
        let mut record = self.read_record(job_id)?;
        update(&mut record);
        record.updated_at = Utc::now();
        self.write_record_unlocked(&record)?;
        Ok(record)
    }

    /// Atomically reserve and create the sole child allowed to consume a named
    /// source scope. The claim lives in a registry beside (never inside) the
    /// canonical source Job, so two different destination JobStore roots still
    /// serialize on the same source without mutating the source Job or Pack.
    pub fn create_claimed_child_job(
        &self,
        source_job_dir: &Path,
        expected_source_job_id: &str,
        scope: &str,
        source_kind: SourceKind,
    ) -> Result<JobRecord, JobStoreError> {
        if !is_filesystem_safe_job_id(expected_source_job_id) {
            return Err(JobStoreError::UnsafeJobId(
                expected_source_job_id.to_string(),
            ));
        }
        let canonical_source =
            fs::canonicalize(source_job_dir).map_err(|source| JobStoreError::Io {
                path: source_job_dir.to_path_buf(),
                source,
            })?;
        if canonical_source.file_name().and_then(|name| name.to_str())
            != Some(expected_source_job_id)
        {
            return Err(JobStoreError::JobNotFound(
                expected_source_job_id.to_string(),
            ));
        }
        let source_store_root = canonical_source.parent().ok_or_else(|| JobStoreError::Io {
            path: canonical_source.clone(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "source Job directory has no parent registry location",
            ),
        })?;
        let claims_dir = source_store_root.join(CHILD_SCOPE_CLAIMS_DIR);
        fs::create_dir_all(&claims_dir).map_err(|source| JobStoreError::Io {
            path: claims_dir.clone(),
            source,
        })?;
        let lock_path = claims_dir.join(format!("{expected_source_job_id}.lock"));
        let lock = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|source| JobStoreError::Io {
                path: lock_path.clone(),
                source,
            })?;
        lock.lock().map_err(|source| JobStoreError::Io {
            path: lock_path,
            source,
        })?;
        // Re-read the source identity while holding the source-scoped claim
        // lock. This closes the validation-to-staging window for the parent id;
        // the full artifact closure is independently revalidated by the runner
        // immediately before any Provider request.
        let source_record_path = canonical_source.join(JOB_JSON);
        let source_record: JobRecord =
            serde_json::from_slice(&fs::read(&source_record_path).map_err(|source| {
                JobStoreError::Io {
                    path: source_record_path.clone(),
                    source,
                }
            })?)
            .map_err(|source| JobStoreError::Deserialize {
                path: source_record_path.clone(),
                source,
            })?;
        if source_record.job_id != expected_source_job_id
            || fs::canonicalize(&source_record.job_dir).map_err(|source| JobStoreError::Io {
                path: source_record.job_dir.clone(),
                source,
            })? != canonical_source
        {
            return Err(JobStoreError::JobNotFound(
                expected_source_job_id.to_string(),
            ));
        }
        let claims_path = claims_dir.join(format!("{expected_source_job_id}.json"));
        let mut claims = if claims_path.is_file() {
            serde_json::from_slice::<BTreeMap<String, String>>(&fs::read(&claims_path).map_err(
                |source| JobStoreError::Io {
                    path: claims_path.clone(),
                    source,
                },
            )?)
            .map_err(|source| JobStoreError::Deserialize {
                path: claims_path.clone(),
                source,
            })?
        } else {
            BTreeMap::new()
        };
        let claim_key = scope.to_string();
        if let Some(child_job_id) = claims.get(&claim_key) {
            return Err(JobStoreError::ChildScopeAlreadyClaimed {
                scope: scope.to_string(),
                child_job_id: child_job_id.clone(),
            });
        }
        let child_job_id = Uuid::new_v4().to_string();
        claims.insert(claim_key, child_job_id.clone());
        let bytes =
            serde_json::to_vec_pretty(&claims).map_err(|source| JobStoreError::Serialize {
                path: claims_path.clone(),
                source,
            })?;
        let temporary =
            claims_dir.join(format!(".{expected_source_job_id}.{}.tmp", Uuid::new_v4()));
        fs::write(&temporary, bytes).map_err(|source| JobStoreError::Io {
            path: temporary.clone(),
            source,
        })?;
        fs::rename(&temporary, &claims_path).map_err(|source| JobStoreError::Io {
            path: claims_path,
            source,
        })?;
        // If child creation fails, the durable source claim remains
        // fail-closed. A later audit can reconcile the reserved child id
        // without ever creating a sibling.
        self.create_job_with_id(source_kind, child_job_id)
    }

    fn create_job_with_id(
        &self,
        source_kind: SourceKind,
        job_id: String,
    ) -> Result<JobRecord, JobStoreError> {
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
            asset_id: Some(Uuid::new_v4().to_string()),
            parent_job_id: None,
            authorization_id: None,
            authorization_manifest_sha256: None,
            lineage_root_job_id: None,
            operation_kind: JobOperationKind::LegacyPipeline,
            lifecycle_state: JobLifecycleState::Idle,
            progress: 0.0,
            input_hash: None,
            recipe_hash: None,
            recipe: None,
            repair: None,
            steps: Vec::new(),
            artifacts: Vec::new(),
            error_code: None,
            recoverable: false,
            cancellation_requested: false,
            worker_pid: None,
            next_actions: Vec::new(),
        };
        self.write_record(&record)?;
        Ok(record)
    }

    fn lock_record(&self, job_dir: &Path) -> Result<File, JobStoreError> {
        let path = job_dir.join(JOB_LOCK);
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .map_err(|source| JobStoreError::Io {
                path: path.clone(),
                source,
            })?;
        file.lock()
            .map_err(|source| JobStoreError::Io { path, source })?;
        Ok(file)
    }

    pub fn list_records(&self) -> Result<Vec<JobRecord>, JobStoreError> {
        let mut records = Vec::new();
        for entry in fs::read_dir(&self.root).map_err(|source| JobStoreError::Io {
            path: self.root.clone(),
            source,
        })? {
            let Ok(entry) = entry else {
                continue;
            };
            let Some(job_id) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if !entry.path().is_dir() || !is_filesystem_safe_job_id(&job_id) {
                continue;
            }
            if let Ok(record) = self.read_record(&job_id) {
                records.push(record);
            }
        }
        records.sort_by_key(|record| std::cmp::Reverse(record.updated_at));
        Ok(records)
    }

    pub fn request_cancellation(&self, job_id: &str) -> Result<JobRecord, JobStoreError> {
        self.update_record(job_id, |record| {
            record.cancellation_requested = true;
            if !is_terminal_lifecycle(record.lifecycle_state) {
                record.next_actions = vec!["wait_for_cancellation".to_string()];
            }
        })
    }

    /// Every job whose `parent_job_id` points at `parent_job_id`, most
    /// recently updated first (inherited from [`Self::list_records`]).
    pub fn list_children(&self, parent_job_id: &str) -> Result<Vec<JobRecord>, JobStoreError> {
        Ok(self
            .list_records()?
            .into_iter()
            .filter(|record| record.parent_job_id.as_deref() == Some(parent_job_id))
            .collect())
    }

    /// Set the cancellation flag on `job_id` and recursively on every
    /// non-terminal descendant (child jobs discovered via `parent_job_id`).
    /// Terminal children (Succeeded/Failed/Cancelled) are left untouched and
    /// are not traversed. Returns the number of jobs flagged, including the
    /// root job itself.
    pub fn request_cancellation_cascade(&self, job_id: &str) -> Result<usize, JobStoreError> {
        self.request_cancellation(job_id)?;
        let mut flagged = 1;
        let mut visited = BTreeSet::new();
        let mut frontier = vec![job_id.to_string()];
        while let Some(parent_id) = frontier.pop() {
            if !visited.insert(parent_id.clone()) {
                continue;
            }
            for child in self.list_children(&parent_id)? {
                if is_terminal_lifecycle(child.lifecycle_state) {
                    continue;
                }
                self.request_cancellation(&child.job_id)?;
                flagged += 1;
                frontier.push(child.job_id);
            }
        }
        Ok(flagged)
    }

    fn job_dir(&self, job_id: &str) -> Result<PathBuf, JobStoreError> {
        if !is_filesystem_safe_job_id(job_id) {
            return Err(JobStoreError::UnsafeJobId(job_id.to_owned()));
        }
        Ok(self.root.join(job_id))
    }
}

fn is_terminal_lifecycle(state: JobLifecycleState) -> bool {
    matches!(
        state,
        JobLifecycleState::Succeeded | JobLifecycleState::Failed | JobLifecycleState::Cancelled
    )
}

fn is_filesystem_safe_job_id(job_id: &str) -> bool {
    !job_id.is_empty()
        && job_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}
