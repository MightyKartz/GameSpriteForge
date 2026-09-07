use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const AUTHORIZATION_MANIFEST_SCHEMA_VERSION: &str = "1";
pub const REQUEST_LEDGER_SCHEMA_VERSION: &str = "1";

const MANIFEST_FILE: &str = "authorization-manifest.json";
const LEDGER_FILE: &str = "request-ledger.json";
const LEDGER_LOCK_FILE: &str = ".request-ledger.lock";

/// An immutable, non-secret allowance for one logical generation target.
///
/// `cost_reservation_ticks_per_request` is reserved before a network request.
/// A successful response replaces that reservation with the observed cost;
/// an ambiguous response keeps the whole reservation consumed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthorizedTargetV1 {
    pub target_id: String,
    pub max_requests: u32,
    pub cost_reservation_ticks_per_request: u64,
}

/// Immutable authorization reviewed by a caller before real-provider work.
///
/// This document deliberately contains no credentials, prompts, headers,
/// device codes, provider response URLs, or media bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthorizationManifestV1 {
    pub schema_version: String,
    pub authorization_id: String,
    pub provider_id: String,
    pub profile_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_models: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub max_total_requests: u32,
    /// Total HTTP operations, including model generation and private transport.
    /// Older grants omit this and receive a conservative four-operations-per-
    /// model-attempt allowance without changing their model attempt cap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_total_provider_operations: Option<u32>,
    pub max_total_cost_ticks: u64,
    pub allowed_targets: Vec<AuthorizedTargetV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_lineage_root_job_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_fingerprint: Option<String>,
}

impl AuthorizationManifestV1 {
    pub fn validate(&self) -> Result<(), AuthorizationError> {
        if self.schema_version != AUTHORIZATION_MANIFEST_SCHEMA_VERSION {
            return Err(AuthorizationError::InvalidManifest(format!(
                "unsupported authorization manifest schema version: {}",
                self.schema_version
            )));
        }
        validate_authorization_id(&self.authorization_id)?;
        validate_label("providerId", &self.provider_id)?;
        validate_label("profileId", &self.profile_id)?;
        if self.expires_at <= self.created_at {
            return Err(AuthorizationError::InvalidManifest(
                "expiresAt must be later than createdAt".into(),
            ));
        }
        if self.max_total_requests == 0 {
            return Err(AuthorizationError::InvalidManifest(
                "maxTotalRequests must be greater than zero".into(),
            ));
        }
        if self
            .max_total_provider_operations
            .is_some_and(|operations| operations < self.max_total_requests)
        {
            return Err(AuthorizationError::InvalidManifest(
                "maxTotalProviderOperations must be at least maxTotalRequests".into(),
            ));
        }
        if self.max_total_cost_ticks == 0 {
            return Err(AuthorizationError::InvalidManifest(
                "maxTotalCostTicks must be greater than zero".into(),
            ));
        }
        if self.allowed_targets.is_empty() {
            return Err(AuthorizationError::InvalidManifest(
                "allowedTargets must not be empty".into(),
            ));
        }
        let mut targets = HashSet::new();
        for target in &self.allowed_targets {
            validate_label("targetId", &target.target_id)?;
            if !targets.insert(target.target_id.as_str()) {
                return Err(AuthorizationError::InvalidManifest(format!(
                    "duplicate targetId: {}",
                    target.target_id
                )));
            }
            if target.max_requests == 0 {
                return Err(AuthorizationError::InvalidManifest(format!(
                    "target {} maxRequests must be greater than zero",
                    target.target_id
                )));
            }
            if target.cost_reservation_ticks_per_request == 0 {
                return Err(AuthorizationError::InvalidManifest(format!(
                    "target {} costReservationTicksPerRequest must be greater than zero",
                    target.target_id
                )));
            }
            if target.cost_reservation_ticks_per_request > self.max_total_cost_ticks {
                return Err(AuthorizationError::InvalidManifest(format!(
                    "target {} reserves more than maxTotalCostTicks per request",
                    target.target_id
                )));
            }
        }
        let target_request_limit = self
            .allowed_targets
            .iter()
            .map(|target| target.max_requests)
            .fold(0u32, u32::saturating_add);
        if self.max_total_requests > target_request_limit {
            return Err(AuthorizationError::InvalidManifest(
                "maxTotalRequests exceeds the sum of target request limits".into(),
            ));
        }
        if self.allowed_models.is_empty() {
            return Err(AuthorizationError::InvalidManifest(
                "allowedModels must not be empty".into(),
            ));
        }
        let mut models = HashSet::new();
        for model in &self.allowed_models {
            validate_label("allowedModels", model)?;
            if !models.insert(model.as_str()) {
                return Err(AuthorizationError::InvalidManifest(format!(
                    "duplicate allowed model: {model}"
                )));
            }
        }
        if let Some(job_id) = &self.source_lineage_root_job_id {
            validate_label("sourceLineageRootJobId", job_id)?;
        }
        if let Some(recipe_hash) = &self.recipe_hash {
            validate_label("recipeHash", recipe_hash)?;
        }
        if let Some(input_fingerprint) = &self.input_fingerprint {
            validate_label("inputFingerprint", input_fingerprint)?;
        }
        Ok(())
    }

    fn target(&self, target_id: &str) -> Result<&AuthorizedTargetV1, AuthorizationError> {
        self.allowed_targets
            .iter()
            .find(|target| target.target_id == target_id)
            .ok_or_else(|| AuthorizationError::TargetNotAllowed(target_id.into()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestLedgerStateV1 {
    Reserved,
    Submitted,
    Settled,
    Ambiguous,
    Released,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderOperationClassV1 {
    #[default]
    ModelGeneration,
    Transport,
    Poll,
    Cleanup,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequestLedgerEntryV1 {
    pub request_id: String,
    /// Non-secret Provider request identifier for asynchronous work. It is
    /// persisted before polling so a restarted process can settle the
    /// original reservation instead of submitting a duplicate request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_request_id: Option<String>,
    pub target_id: String,
    pub operation: String,
    #[serde(default)]
    pub operation_class: ProviderOperationClassV1,
    pub model: String,
    pub state: RequestLedgerStateV1,
    pub reserved_cost_ticks: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_cost_ticks: Option<u64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineage_root_job_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequestLedgerV1 {
    pub schema_version: String,
    pub authorization_id: String,
    pub next_sequence: u64,
    pub updated_at: DateTime<Utc>,
    pub requests: Vec<RequestLedgerEntryV1>,
}

impl RequestLedgerV1 {
    fn new(authorization_id: String, now: DateTime<Utc>) -> Self {
        Self {
            schema_version: REQUEST_LEDGER_SCHEMA_VERSION.into(),
            authorization_id,
            next_sequence: 1,
            updated_at: now,
            requests: Vec::new(),
        }
    }

    pub fn consumed_request_count(&self) -> u32 {
        self.requests
            .iter()
            .filter(|request| {
                request.operation_class == ProviderOperationClassV1::ModelGeneration
                    && request.state != RequestLedgerStateV1::Released
            })
            .count()
            .min(u32::MAX as usize) as u32
    }

    pub fn consumed_operation_count(&self) -> u32 {
        // Provider-operation slots are reservation attempts, not merely HTTP
        // calls. A Released entry remains durable evidence that its slot was
        // consumed. This makes an exact one-operation authorization truly
        // single-use even across reserve/release races, while model-request
        // and cost budgets can still exclude an unsubmitted Released entry.
        self.requests.len().min(u32::MAX as usize) as u32
    }

    pub fn consumed_cost_ticks(&self) -> u64 {
        self.requests
            .iter()
            .map(|request| match request.state {
                RequestLedgerStateV1::Settled => request
                    .observed_cost_ticks
                    .unwrap_or(request.reserved_cost_ticks),
                RequestLedgerStateV1::Reserved
                | RequestLedgerStateV1::Submitted
                | RequestLedgerStateV1::Ambiguous => request.reserved_cost_ticks,
                RequestLedgerStateV1::Released => 0,
            })
            .fold(0u64, u64::saturating_add)
    }
}

#[derive(Debug, Clone)]
pub struct ProviderAuthorizationConfig {
    pub store_root: PathBuf,
    pub authorization_id: String,
    /// Exact authorization-manifest bytes reviewed when this grant was
    /// attached to the Job. Legacy callers may omit it; V9.3 persists it.
    pub expected_manifest_sha256: Option<String>,
    pub job_id: Option<String>,
    pub lineage_root_job_id: Option<String>,
    /// Immutable canonical recipe identity for the Job that consumes this
    /// authorization. Kept out of the ledger because it belongs to the
    /// reviewed grant/session binding rather than an individual request.
    pub recipe_hash: Option<String>,
    /// Immutable canonical input identity for the Job that consumes this
    /// authorization.
    pub input_fingerprint: Option<String>,
}

impl ProviderAuthorizationConfig {
    pub fn new(store_root: impl Into<PathBuf>, authorization_id: impl Into<String>) -> Self {
        Self {
            store_root: store_root.into(),
            authorization_id: authorization_id.into(),
            expected_manifest_sha256: None,
            job_id: None,
            lineage_root_job_id: None,
            recipe_hash: None,
            input_fingerprint: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthorizationStore {
    authorization_dir: PathBuf,
}

impl AuthorizationStore {
    pub fn create(
        root: impl AsRef<Path>,
        manifest: &AuthorizationManifestV1,
    ) -> Result<Self, AuthorizationError> {
        manifest.validate()?;
        let root = root.as_ref();
        fs::create_dir_all(root).map_err(|source| AuthorizationError::Io {
            path: root.to_path_buf(),
            source,
        })?;
        let authorization_dir = root.join(&manifest.authorization_id);
        match fs::create_dir(&authorization_dir) {
            Ok(()) => {}
            Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(AuthorizationError::AlreadyExists(
                    manifest.authorization_id.clone(),
                ));
            }
            Err(source) => {
                return Err(AuthorizationError::Io {
                    path: authorization_dir,
                    source,
                });
            }
        }
        let store = Self { authorization_dir };
        write_json_atomic(&store.manifest_path(), manifest)?;
        write_json_atomic(
            &store.ledger_path(),
            &RequestLedgerV1::new(manifest.authorization_id.clone(), manifest.created_at),
        )?;
        Ok(store)
    }

    pub fn open(
        root: impl AsRef<Path>,
        authorization_id: &str,
    ) -> Result<Self, AuthorizationError> {
        validate_authorization_id(authorization_id)?;
        let authorization_dir = root.as_ref().join(authorization_id);
        if !authorization_dir.is_dir() {
            return Err(AuthorizationError::NotFound(authorization_id.into()));
        }
        let store = Self { authorization_dir };
        let manifest = store.read_manifest()?;
        manifest.validate()?;
        if manifest.authorization_id != authorization_id {
            return Err(AuthorizationError::InvalidManifest(
                "authorization directory does not match manifest id".into(),
            ));
        }
        let ledger = store.read_ledger()?;
        store.validate_ledger(&manifest, &ledger)?;
        Ok(store)
    }

    pub fn manifest(&self) -> Result<AuthorizationManifestV1, AuthorizationError> {
        self.read_manifest()
    }

    pub fn manifest_snapshot(
        &self,
    ) -> Result<(AuthorizationManifestV1, String), AuthorizationError> {
        self.read_manifest_snapshot()
    }

    pub fn ledger(&self) -> Result<RequestLedgerV1, AuthorizationError> {
        let _lock = self.lock_ledger()?;
        let manifest = self.read_manifest()?;
        let ledger = self.read_ledger()?;
        self.validate_ledger(&manifest, &ledger)?;
        Ok(ledger)
    }

    /// Fail closed when a caller needs an unexpired grant whose durable
    /// ledger has never contained any reservation, including released ones.
    pub fn ensure_active_and_pristine(&self) -> Result<(), AuthorizationError> {
        let _lock = self.lock_ledger()?;
        let manifest = self.read_manifest()?;
        self.ensure_active(&manifest)?;
        let ledger = self.read_ledger()?;
        self.validate_ledger(&manifest, &ledger)?;
        if !ledger.requests.is_empty() {
            return Err(AuthorizationError::InvalidLedger(
                "authorization ledger must be physically empty".into(),
            ));
        }
        Ok(())
    }

    pub fn bind(
        config: ProviderAuthorizationConfig,
        provider_id: &str,
        profile_id: &str,
    ) -> Result<ProviderAuthorizationSession, AuthorizationError> {
        if let Some(job_id) = &config.job_id {
            validate_label("jobId", job_id)?;
        }
        if let Some(job_id) = &config.lineage_root_job_id {
            validate_label("lineageRootJobId", job_id)?;
        }
        if let Some(recipe_hash) = &config.recipe_hash {
            validate_label("recipeHash", recipe_hash)?;
        }
        if let Some(input_fingerprint) = &config.input_fingerprint {
            validate_label("inputFingerprint", input_fingerprint)?;
        }
        let store = Self::open(&config.store_root, &config.authorization_id)?;
        let (manifest, manifest_sha256) = store.read_manifest_snapshot()?;
        manifest.validate()?;
        if config
            .expected_manifest_sha256
            .as_deref()
            .is_some_and(|expected| expected != manifest_sha256)
        {
            return Err(AuthorizationError::ManifestChanged);
        }
        store.ensure_active(&manifest)?;
        if manifest.provider_id != provider_id {
            return Err(AuthorizationError::ProviderMismatch {
                expected: manifest.provider_id,
                actual: provider_id.into(),
            });
        }
        if manifest.profile_id != profile_id {
            return Err(AuthorizationError::ProfileMismatch {
                expected: manifest.profile_id,
                actual: profile_id.into(),
            });
        }
        Ok(ProviderAuthorizationSession {
            store,
            manifest: Arc::new(manifest),
            manifest_sha256: Arc::from(manifest_sha256),
            context: Arc::new(Mutex::new(AuthorizationExecutionContext {
                job_id: config.job_id,
                lineage_root_job_id: config.lineage_root_job_id,
                recipe_hash: config.recipe_hash,
                input_fingerprint: config.input_fingerprint,
            })),
            provider_id: provider_id.into(),
            profile_id: profile_id.into(),
        })
    }

    fn reserve(
        &self,
        session: &ProviderAuthorizationSession,
        target_id: &str,
        operation: &str,
        model: &str,
    ) -> Result<RequestReservation, AuthorizationError> {
        self.reserve_compound(
            session,
            target_id,
            &[(operation, ProviderOperationClassV1::ModelGeneration)],
            model,
        )?
        .pop()
        .ok_or_else(|| AuthorizationError::InvalidLedger("empty reservation set".into()))
    }

    fn reserve_compound(
        &self,
        session: &ProviderAuthorizationSession,
        target_id: &str,
        operations: &[(&str, ProviderOperationClassV1)],
        model: &str,
    ) -> Result<Vec<RequestReservation>, AuthorizationError> {
        validate_label("targetId", target_id)?;
        if operations.is_empty() {
            return Err(AuthorizationError::InvalidManifest(
                "compound reservation must include at least one operation".into(),
            ));
        }
        for (operation, _) in operations {
            validate_label("operation", operation)?;
        }
        validate_label("model", model)?;
        let _lock = self.lock_ledger()?;
        self.ensure_manifest_unchanged(&session.manifest_sha256)?;
        let manifest = session.manifest.as_ref();
        self.ensure_active(manifest)?;
        if manifest.provider_id != session.provider_id {
            return Err(AuthorizationError::ProviderMismatch {
                expected: manifest.provider_id.clone(),
                actual: session.provider_id.clone(),
            });
        }
        if manifest.profile_id != session.profile_id {
            return Err(AuthorizationError::ProfileMismatch {
                expected: manifest.profile_id.clone(),
                actual: session.profile_id.clone(),
            });
        }
        // This check deliberately precedes loading or mutating the ledger.
        // A V9.3 reviewed grant is bound to the canonical recipe/input and
        // source lineage that produced the request; a stale Provider session
        // must therefore fail before it can consume any allowance.
        let context = session.context.lock().map_err(|_| {
            AuthorizationError::InvalidLedger("authorization Job context lock was poisoned".into())
        })?;
        validate_execution_context(manifest, &context)?;
        if !manifest.allowed_models.is_empty()
            && !manifest
                .allowed_models
                .iter()
                .any(|allowed| allowed == model)
        {
            return Err(AuthorizationError::ModelNotAllowed(model.into()));
        }
        let target = manifest.target(target_id)?;
        let mut ledger = self.read_ledger()?;
        self.validate_ledger(manifest, &ledger)?;
        let requested_model_attempts = operations
            .iter()
            .filter(|(_, class)| *class == ProviderOperationClassV1::ModelGeneration)
            .count()
            .min(u32::MAX as usize) as u32;
        let requested_operations = operations.len().min(u32::MAX as usize) as u32;
        let total_requests = ledger.consumed_request_count();
        if total_requests.saturating_add(requested_model_attempts) > manifest.max_total_requests {
            return Err(AuthorizationError::RequestBudgetExceeded(format!(
                "authorization {} has reached its total request limit {}",
                manifest.authorization_id, manifest.max_total_requests
            )));
        }
        let max_total_operations = manifest
            .max_total_provider_operations
            .unwrap_or_else(|| manifest.max_total_requests.saturating_mul(4));
        if ledger
            .consumed_operation_count()
            .saturating_add(requested_operations)
            > max_total_operations
        {
            return Err(AuthorizationError::RequestBudgetExceeded(format!(
                "authorization {} has reached its total Provider operation limit {}",
                manifest.authorization_id, max_total_operations
            )));
        }
        let target_requests = ledger
            .requests
            .iter()
            .filter(|request| {
                request.target_id == target_id
                    && request.operation_class == ProviderOperationClassV1::ModelGeneration
                    && request.state != RequestLedgerStateV1::Released
            })
            .count()
            .min(u32::MAX as usize) as u32;
        if target_requests.saturating_add(requested_model_attempts) > target.max_requests {
            return Err(AuthorizationError::RequestBudgetExceeded(format!(
                "target {} has reached its request limit {}",
                target_id, target.max_requests
            )));
        }
        let target_transport_operations = ledger
            .requests
            .iter()
            .filter(|request| {
                request.target_id == target_id
                    && request.operation_class != ProviderOperationClassV1::ModelGeneration
                    && request.state != RequestLedgerStateV1::Released
            })
            .count()
            .min(u32::MAX as usize) as u32;
        let requested_transport_operations = requested_operations - requested_model_attempts;
        let max_target_transport_operations = target.max_requests.saturating_mul(4);
        if target_transport_operations.saturating_add(requested_transport_operations)
            > max_target_transport_operations
        {
            return Err(AuthorizationError::RequestBudgetExceeded(format!(
                "target {} has reached its transport operation limit {}",
                target_id, max_target_transport_operations
            )));
        }
        let reserved_model_cost = target
            .cost_reservation_ticks_per_request
            .checked_mul(u64::from(requested_model_attempts))
            .ok_or_else(|| {
                AuthorizationError::CostBudgetExceeded(
                    "authorization compound cost reservation overflowed".into(),
                )
            })?;
        let new_cost = ledger
            .consumed_cost_ticks()
            .checked_add(reserved_model_cost)
            .ok_or_else(|| {
                AuthorizationError::CostBudgetExceeded(
                    "authorization cost reservation overflowed".into(),
                )
            })?;
        if new_cost > manifest.max_total_cost_ticks {
            return Err(AuthorizationError::CostBudgetExceeded(format!(
                "authorization {} cannot reserve {} ticks within remaining total cost budget",
                manifest.authorization_id, reserved_model_cost
            )));
        }
        let now = Utc::now();
        let mut request_ids = Vec::with_capacity(operations.len());
        for (operation, operation_class) in operations {
            let request_id = format!("request-{:016x}", ledger.next_sequence);
            ledger.next_sequence = ledger.next_sequence.saturating_add(1);
            request_ids.push(request_id.clone());
            ledger.requests.push(RequestLedgerEntryV1 {
                request_id,
                provider_request_id: None,
                target_id: target_id.into(),
                operation: (*operation).into(),
                operation_class: *operation_class,
                model: model.into(),
                state: RequestLedgerStateV1::Reserved,
                reserved_cost_ticks: if *operation_class
                    == ProviderOperationClassV1::ModelGeneration
                {
                    target.cost_reservation_ticks_per_request
                } else {
                    0
                },
                observed_cost_ticks: None,
                created_at: now,
                updated_at: now,
                job_id: context.job_id.clone(),
                lineage_root_job_id: context.lineage_root_job_id.clone(),
            });
        }
        ledger.updated_at = now;
        // Check again immediately before committing the reservation so a
        // manifest replacement during validation cannot authorize a write.
        self.ensure_manifest_unchanged(&session.manifest_sha256)?;
        write_json_atomic(&self.ledger_path(), &ledger)?;
        Ok(request_ids
            .into_iter()
            .map(|request_id| RequestReservation {
                store: self.clone(),
                manifest: session.manifest.clone(),
                manifest_sha256: session.manifest_sha256.clone(),
                request_id,
                finalized: false,
            })
            .collect())
    }

    fn transition(
        &self,
        manifest: &AuthorizationManifestV1,
        manifest_sha256: &str,
        request_id: &str,
        state: RequestLedgerStateV1,
        observed_cost_ticks: Option<u64>,
    ) -> Result<(), AuthorizationError> {
        let _lock = self.lock_ledger()?;
        self.ensure_manifest_unchanged(manifest_sha256)?;
        let mut ledger = self.read_ledger()?;
        self.validate_ledger(manifest, &ledger)?;
        let request = ledger
            .requests
            .iter_mut()
            .find(|request| request.request_id == request_id)
            .ok_or_else(|| AuthorizationError::ReservationNotFound(request_id.into()))?;
        let transition_allowed = matches!(
            (request.state, state),
            (
                RequestLedgerStateV1::Reserved,
                RequestLedgerStateV1::Submitted
            ) | (
                RequestLedgerStateV1::Reserved,
                RequestLedgerStateV1::Ambiguous
            ) | (
                RequestLedgerStateV1::Reserved,
                RequestLedgerStateV1::Released
            ) | (
                RequestLedgerStateV1::Submitted,
                RequestLedgerStateV1::Settled
            ) | (
                RequestLedgerStateV1::Submitted,
                RequestLedgerStateV1::Ambiguous
            )
        );
        if !transition_allowed {
            return Err(AuthorizationError::InvalidTransition {
                request_id: request_id.into(),
                from: request.state,
                to: state,
            });
        }
        request.state = state;
        request.observed_cost_ticks = observed_cost_ticks;
        let now = Utc::now();
        request.updated_at = now;
        ledger.updated_at = now;
        let consumed_total = ledger.consumed_cost_ticks();
        write_json_atomic(&self.ledger_path(), &ledger)?;
        if consumed_total > manifest.max_total_cost_ticks {
            return Err(AuthorizationError::CostBudgetExceeded(format!(
                "Provider cost and outstanding reservations {consumed_total} exceed authorization limit {}",
                manifest.max_total_cost_ticks
            )));
        }
        Ok(())
    }

    fn bind_provider_request_id(
        &self,
        manifest: &AuthorizationManifestV1,
        manifest_sha256: &str,
        request_id: &str,
        provider_request_id: &str,
    ) -> Result<(), AuthorizationError> {
        validate_label("providerRequestId", provider_request_id)?;
        let _lock = self.lock_ledger()?;
        self.ensure_manifest_unchanged(manifest_sha256)?;
        let mut ledger = self.read_ledger()?;
        self.validate_ledger(manifest, &ledger)?;
        if ledger.requests.iter().any(|request| {
            request.provider_request_id.as_deref() == Some(provider_request_id)
                && request.request_id != request_id
        }) {
            return Err(AuthorizationError::InvalidLedger(format!(
                "duplicate provider request id: {provider_request_id}"
            )));
        }
        let request = ledger
            .requests
            .iter_mut()
            .find(|request| request.request_id == request_id)
            .ok_or_else(|| AuthorizationError::ReservationNotFound(request_id.into()))?;
        if request.state != RequestLedgerStateV1::Submitted {
            return Err(AuthorizationError::InvalidTransition {
                request_id: request_id.into(),
                from: request.state,
                to: RequestLedgerStateV1::Submitted,
            });
        }
        request.provider_request_id = Some(provider_request_id.into());
        let now = Utc::now();
        request.updated_at = now;
        ledger.updated_at = now;
        write_json_atomic(&self.ledger_path(), &ledger)
    }

    fn finalize_provider_request(
        &self,
        manifest: &AuthorizationManifestV1,
        manifest_sha256: &str,
        provider_request_id: &str,
        observed_cost_ticks: Option<u64>,
    ) -> Result<bool, AuthorizationError> {
        validate_label("providerRequestId", provider_request_id)?;
        let _lock = self.lock_ledger()?;
        self.ensure_manifest_unchanged(manifest_sha256)?;
        let mut ledger = self.read_ledger()?;
        self.validate_ledger(manifest, &ledger)?;
        let request = ledger
            .requests
            .iter_mut()
            .find(|request| request.provider_request_id.as_deref() == Some(provider_request_id))
            .ok_or_else(|| {
                AuthorizationError::ProviderRequestNotFound(provider_request_id.into())
            })?;
        if matches!(
            request.state,
            RequestLedgerStateV1::Settled | RequestLedgerStateV1::Ambiguous
        ) {
            return Ok(false);
        }
        let terminal_state = if observed_cost_ticks.is_some() {
            RequestLedgerStateV1::Settled
        } else {
            RequestLedgerStateV1::Ambiguous
        };
        if request.state != RequestLedgerStateV1::Submitted {
            return Err(AuthorizationError::InvalidTransition {
                request_id: request.request_id.clone(),
                from: request.state,
                to: terminal_state,
            });
        }
        request.state = terminal_state;
        request.observed_cost_ticks = observed_cost_ticks;
        let now = Utc::now();
        request.updated_at = now;
        ledger.updated_at = now;
        let consumed_total = ledger.consumed_cost_ticks();
        write_json_atomic(&self.ledger_path(), &ledger)?;
        if consumed_total > manifest.max_total_cost_ticks {
            return Err(AuthorizationError::CostBudgetExceeded(format!(
                "Provider cost and outstanding reservations {consumed_total} exceed authorization limit {}",
                manifest.max_total_cost_ticks
            )));
        }
        Ok(true)
    }

    fn ensure_active(&self, manifest: &AuthorizationManifestV1) -> Result<(), AuthorizationError> {
        if Utc::now() > manifest.expires_at {
            return Err(AuthorizationError::Expired(manifest.expires_at));
        }
        Ok(())
    }

    fn ensure_manifest_unchanged(&self, expected_sha256: &str) -> Result<(), AuthorizationError> {
        let (_, current_sha256) = self.read_manifest_snapshot()?;
        if current_sha256 != expected_sha256 {
            return Err(AuthorizationError::ManifestChanged);
        }
        Ok(())
    }

    fn validate_ledger(
        &self,
        manifest: &AuthorizationManifestV1,
        ledger: &RequestLedgerV1,
    ) -> Result<(), AuthorizationError> {
        if ledger.schema_version != REQUEST_LEDGER_SCHEMA_VERSION {
            return Err(AuthorizationError::InvalidLedger(format!(
                "unsupported request ledger schema version: {}",
                ledger.schema_version
            )));
        }
        if ledger.authorization_id != manifest.authorization_id {
            return Err(AuthorizationError::InvalidLedger(
                "ledger authorization id does not match manifest".into(),
            ));
        }
        let mut request_ids = HashSet::new();
        let mut provider_request_ids = HashSet::new();
        for request in &ledger.requests {
            if !request_ids.insert(request.request_id.as_str()) {
                return Err(AuthorizationError::InvalidLedger(format!(
                    "duplicate request id: {}",
                    request.request_id
                )));
            }
            if let Some(provider_request_id) = &request.provider_request_id {
                validate_label("providerRequestId", provider_request_id)?;
                if !provider_request_ids.insert(provider_request_id.as_str()) {
                    return Err(AuthorizationError::InvalidLedger(format!(
                        "duplicate provider request id: {provider_request_id}"
                    )));
                }
                if request.state == RequestLedgerStateV1::Reserved {
                    return Err(AuthorizationError::InvalidLedger(format!(
                        "reserved request {} cannot have a provider request id",
                        request.request_id
                    )));
                }
            }
            manifest.target(&request.target_id)?;
        }
        Ok(())
    }

    fn read_manifest(&self) -> Result<AuthorizationManifestV1, AuthorizationError> {
        read_json(&self.manifest_path())
    }

    fn read_manifest_snapshot(
        &self,
    ) -> Result<(AuthorizationManifestV1, String), AuthorizationError> {
        let path = self.manifest_path();
        let bytes = fs::read(&path).map_err(|source| AuthorizationError::Io { path, source })?;
        let sha256 = format!("{:x}", Sha256::digest(&bytes));
        Ok((serde_json::from_slice(&bytes)?, sha256))
    }

    fn read_ledger(&self) -> Result<RequestLedgerV1, AuthorizationError> {
        read_json(&self.ledger_path())
    }

    fn lock_ledger(&self) -> Result<File, AuthorizationError> {
        let path = self.authorization_dir.join(LEDGER_LOCK_FILE);
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .map_err(|source| AuthorizationError::Io {
                path: path.clone(),
                source,
            })?;
        file.lock()
            .map_err(|source| AuthorizationError::Io { path, source })?;
        Ok(file)
    }

    fn manifest_path(&self) -> PathBuf {
        self.authorization_dir.join(MANIFEST_FILE)
    }

    fn ledger_path(&self) -> PathBuf {
        self.authorization_dir.join(LEDGER_FILE)
    }
}

#[derive(Debug, Clone)]
pub struct ProviderAuthorizationSession {
    store: AuthorizationStore,
    /// Manifest value validated when this session was bound. All subsequent
    /// authorization decisions derive from this immutable snapshot, while
    /// boundary checks reject any replacement of the durable document.
    manifest: Arc<AuthorizationManifestV1>,
    manifest_sha256: Arc<str>,
    context: Arc<Mutex<AuthorizationExecutionContext>>,
    provider_id: String,
    profile_id: String,
}

#[derive(Debug, Clone)]
struct AuthorizationExecutionContext {
    job_id: Option<String>,
    lineage_root_job_id: Option<String>,
    recipe_hash: Option<String>,
    input_fingerprint: Option<String>,
}

impl ProviderAuthorizationSession {
    #[allow(clippy::too_many_arguments)]
    pub fn validate_exact_scope(
        &self,
        provider_id: &str,
        profile_id: &str,
        model: &str,
        target_id: &str,
        max_requests: u32,
        max_operations: u32,
        max_cost_ticks: u64,
        lineage_root_job_id: &str,
        recipe_hash: &str,
        input_fingerprint: &str,
    ) -> Result<(), AuthorizationError> {
        let _lock = self.store.lock_ledger()?;
        self.store
            .ensure_manifest_unchanged(&self.manifest_sha256)?;
        let manifest = self.manifest.as_ref();
        self.store.ensure_active(manifest)?;
        let ledger = self.store.read_ledger()?;
        self.store.validate_ledger(manifest, &ledger)?;
        if !ledger.requests.is_empty() {
            return Err(AuthorizationError::InvalidLedger(
                "authorization ledger must be physically empty".into(),
            ));
        }
        let context = self.context.lock().map_err(|_| {
            AuthorizationError::InvalidLedger(
                "authorization execution context lock was poisoned".into(),
            )
        })?;
        validate_execution_context(manifest, &context)?;
        let exact_target = manifest.allowed_targets.as_slice()
            == [AuthorizedTargetV1 {
                target_id: target_id.into(),
                max_requests,
                cost_reservation_ticks_per_request: max_cost_ticks,
            }];
        if manifest.provider_id != provider_id
            || manifest.profile_id != profile_id
            || manifest.allowed_models.as_slice() != [model]
            || manifest.max_total_requests != max_requests
            || manifest.max_total_provider_operations != Some(max_operations)
            || manifest.max_total_cost_ticks != max_cost_ticks
            || !exact_target
            || manifest.source_lineage_root_job_id.as_deref() != Some(lineage_root_job_id)
            || manifest.recipe_hash.as_deref() != Some(recipe_hash)
            || manifest.input_fingerprint.as_deref() != Some(input_fingerprint)
        {
            return Err(AuthorizationError::InvalidManifest(
                "authorization does not exactly match the requested execution scope".into(),
            ));
        }
        Ok(())
    }

    pub fn set_job_context(
        &self,
        job_id: &str,
        lineage_root_job_id: &str,
    ) -> Result<(), AuthorizationError> {
        validate_label("jobId", job_id)?;
        validate_label("lineageRootJobId", lineage_root_job_id)?;
        let mut context = self.context.lock().map_err(|_| {
            AuthorizationError::InvalidLedger("authorization Job context lock was poisoned".into())
        })?;
        context.job_id = Some(job_id.into());
        context.lineage_root_job_id = Some(lineage_root_job_id.into());
        Ok(())
    }

    /// Bind every non-secret identity used to scope a V9.3 authorization.
    /// The job id remains attribution only; the lineage, recipe, and input
    /// values are enforced exactly when the corresponding manifest fields are
    /// present. Legacy manifests may omit any of those constraints.
    pub fn set_execution_context(
        &self,
        job_id: &str,
        lineage_root_job_id: &str,
        recipe_hash: &str,
        input_fingerprint: &str,
    ) -> Result<(), AuthorizationError> {
        validate_label("jobId", job_id)?;
        validate_label("lineageRootJobId", lineage_root_job_id)?;
        validate_label("recipeHash", recipe_hash)?;
        validate_label("inputFingerprint", input_fingerprint)?;
        let mut context = self.context.lock().map_err(|_| {
            AuthorizationError::InvalidLedger("authorization Job context lock was poisoned".into())
        })?;
        context.job_id = Some(job_id.into());
        context.lineage_root_job_id = Some(lineage_root_job_id.into());
        context.recipe_hash = Some(recipe_hash.into());
        context.input_fingerprint = Some(input_fingerprint.into());
        Ok(())
    }

    pub fn reserve(
        &self,
        target_id: &str,
        operation: &str,
        model: &str,
    ) -> Result<RequestReservation, AuthorizationError> {
        self.store.reserve(self, target_id, operation, model)
    }

    /// Atomically reserves all operations needed by one logical Provider
    /// attempt. Transport entries are durable audit records but do not consume
    /// the target's model-attempt cap or reserve model cost.
    pub fn reserve_compound(
        &self,
        target_id: &str,
        operations: &[(&str, ProviderOperationClassV1)],
        model: &str,
    ) -> Result<Vec<RequestReservation>, AuthorizationError> {
        self.store
            .reserve_compound(self, target_id, operations, model)
    }

    pub fn ensure_active(&self) -> Result<(), AuthorizationError> {
        self.ensure_manifest_unchanged()?;
        self.store.ensure_active(&self.manifest)
    }

    /// Verify that the durable authorization document still has the exact
    /// bytes that were validated when this session was bound.
    pub fn ensure_manifest_unchanged(&self) -> Result<(), AuthorizationError> {
        self.store.ensure_manifest_unchanged(&self.manifest_sha256)
    }

    pub fn authorization_id(&self) -> Result<String, AuthorizationError> {
        self.ensure_manifest_unchanged()?;
        Ok(self.manifest.authorization_id.clone())
    }

    pub fn manifest_sha256(&self) -> Result<String, AuthorizationError> {
        self.ensure_manifest_unchanged()?;
        Ok(self.manifest_sha256.to_string())
    }

    /// Whether this grant permits exactly one physical Provider operation.
    /// Callers use this to avoid transparent HTTP retries or follow-up
    /// transport requests that would exceed the reviewed allowance.
    pub fn is_exactly_one_operation(&self) -> Result<bool, AuthorizationError> {
        self.ensure_manifest_unchanged()?;
        Ok(self.manifest.max_total_provider_operations == Some(1))
    }

    /// Finalize an asynchronous request by its non-secret Provider id.
    /// Returns true only for the first terminal observation so repeated polls
    /// cannot double-count usage.
    pub fn finalize_provider_request(
        &self,
        provider_request_id: &str,
        observed_cost_ticks: Option<u64>,
    ) -> Result<bool, AuthorizationError> {
        self.store.finalize_provider_request(
            &self.manifest,
            &self.manifest_sha256,
            provider_request_id,
            observed_cost_ticks,
        )
    }
}

#[derive(Debug)]
pub struct RequestReservation {
    store: AuthorizationStore,
    manifest: Arc<AuthorizationManifestV1>,
    manifest_sha256: Arc<str>,
    request_id: String,
    finalized: bool,
}

impl RequestReservation {
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    pub fn mark_submitted(&mut self) -> Result<(), AuthorizationError> {
        self.store.transition(
            &self.manifest,
            &self.manifest_sha256,
            &self.request_id,
            RequestLedgerStateV1::Submitted,
            None,
        )
    }

    pub fn settle(mut self, observed_cost_ticks: u64) -> Result<(), AuthorizationError> {
        let result = self.store.transition(
            &self.manifest,
            &self.manifest_sha256,
            &self.request_id,
            RequestLedgerStateV1::Settled,
            Some(observed_cost_ticks),
        );
        self.finalized = true;
        result
    }

    pub fn mark_ambiguous(mut self) -> Result<(), AuthorizationError> {
        let result = self.store.transition(
            &self.manifest,
            &self.manifest_sha256,
            &self.request_id,
            RequestLedgerStateV1::Ambiguous,
            None,
        );
        self.finalized = true;
        result
    }

    pub fn release(mut self) -> Result<(), AuthorizationError> {
        let result = self.store.transition(
            &self.manifest,
            &self.manifest_sha256,
            &self.request_id,
            RequestLedgerStateV1::Released,
            Some(0),
        );
        self.finalized = true;
        result
    }

    /// Transfer this submitted reservation to a durable asynchronous
    /// Provider request id. After a successful bind, dropping this guard does
    /// not mark the request ambiguous; a later poll finalizes it.
    pub fn bind_provider_request_id(
        mut self,
        provider_request_id: &str,
    ) -> Result<(), AuthorizationError> {
        let result = self.store.bind_provider_request_id(
            &self.manifest,
            &self.manifest_sha256,
            &self.request_id,
            provider_request_id,
        );
        if result.is_ok() {
            self.finalized = true;
        }
        result
    }
}

impl Drop for RequestReservation {
    fn drop(&mut self) {
        if !self.finalized {
            let _ = self.store.transition(
                &self.manifest,
                &self.manifest_sha256,
                &self.request_id,
                RequestLedgerStateV1::Ambiguous,
                None,
            );
            self.finalized = true;
        }
    }
}

#[derive(Debug, Error)]
pub enum AuthorizationError {
    #[error("authorization id contains unsafe characters: {0}")]
    UnsafeAuthorizationId(String),
    #[error("invalid authorization manifest: {0}")]
    InvalidManifest(String),
    #[error("invalid request ledger: {0}")]
    InvalidLedger(String),
    #[error("authorization already exists: {0}")]
    AlreadyExists(String),
    #[error("authorization does not exist: {0}")]
    NotFound(String),
    #[error("authorization expired at {0}")]
    Expired(DateTime<Utc>),
    #[error("authorization does not permit target: {0}")]
    TargetNotAllowed(String),
    #[error("authorization provider mismatch: expected {expected}, got {actual}")]
    ProviderMismatch { expected: String, actual: String },
    #[error("authorization profile mismatch: expected {expected}, got {actual}")]
    ProfileMismatch { expected: String, actual: String },
    #[error("authorization does not permit model: {0}")]
    ModelNotAllowed(String),
    #[error("authorization manifest changed after the Provider session was bound")]
    ManifestChanged,
    #[error("authorization execution context mismatch for {field}: expected {expected:?}, got {actual:?}")]
    ExecutionContextMismatch {
        field: &'static str,
        expected: Option<String>,
        actual: Option<String>,
    },
    #[error("provider request budget exceeded: {0}")]
    RequestBudgetExceeded(String),
    #[error("provider cost budget exceeded: {0}")]
    CostBudgetExceeded(String),
    #[error("request reservation does not exist: {0}")]
    ReservationNotFound(String),
    #[error("Provider request does not exist in the authorization ledger: {0}")]
    ProviderRequestNotFound(String),
    #[error("invalid request ledger transition for {request_id}: {from:?} -> {to:?}")]
    InvalidTransition {
        request_id: String,
        from: RequestLedgerStateV1,
        to: RequestLedgerStateV1,
    },
    #[error("authorization filesystem error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("authorization JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

fn validate_authorization_id(value: &str) -> Result<(), AuthorizationError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(AuthorizationError::UnsafeAuthorizationId(value.into()));
    }
    Ok(())
}

fn validate_label(name: &str, value: &str) -> Result<(), AuthorizationError> {
    if value.trim().is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
        return Err(AuthorizationError::InvalidManifest(format!(
            "{name} must be a non-empty, bounded string without control characters"
        )));
    }
    Ok(())
}

fn validate_execution_context(
    manifest: &AuthorizationManifestV1,
    context: &AuthorizationExecutionContext,
) -> Result<(), AuthorizationError> {
    for (field, expected, actual) in [
        (
            "sourceLineageRootJobId",
            &manifest.source_lineage_root_job_id,
            &context.lineage_root_job_id,
        ),
        ("recipeHash", &manifest.recipe_hash, &context.recipe_hash),
        (
            "inputFingerprint",
            &manifest.input_fingerprint,
            &context.input_fingerprint,
        ),
    ] {
        if expected.is_some() && expected != actual {
            return Err(AuthorizationError::ExecutionContextMismatch {
                field,
                expected: expected.clone(),
                actual: actual.clone(),
            });
        }
    }
    Ok(())
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, AuthorizationError> {
    let bytes = fs::read(path).map_err(|source| AuthorizationError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), AuthorizationError> {
    let parent = path.parent().ok_or_else(|| {
        AuthorizationError::InvalidManifest("authorization output path has no parent".into())
    })?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|source| AuthorizationError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    serde_json::to_writer_pretty(&mut temporary, value)?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|source| AuthorizationError::Io {
            path: temporary.path().to_path_buf(),
            source,
        })?;
    temporary
        .persist(path)
        .map_err(|error| AuthorizationError::Io {
            path: path.to_path_buf(),
            source: error.error,
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use std::sync::{Arc, Barrier};
    use std::thread;

    fn manifest(id: &str, max_requests: u32, max_cost: u64) -> AuthorizationManifestV1 {
        let now = Utc::now();
        AuthorizationManifestV1 {
            schema_version: AUTHORIZATION_MANIFEST_SCHEMA_VERSION.into(),
            authorization_id: id.into(),
            provider_id: "xai".into(),
            profile_id: "default".into(),
            allowed_models: vec!["test-image".into()],
            created_at: now,
            expires_at: now + Duration::minutes(10),
            max_total_requests: max_requests,
            max_total_provider_operations: None,
            max_total_cost_ticks: max_cost,
            allowed_targets: vec![AuthorizedTargetV1 {
                target_id: "happy".into(),
                max_requests,
                cost_reservation_ticks_per_request: 100,
            }],
            source_lineage_root_job_id: None,
            recipe_hash: None,
            input_fingerprint: None,
        }
    }

    #[test]
    fn rejects_unsafe_ids_and_unknown_targets() {
        let temp = tempfile::tempdir().unwrap();
        let mut unsafe_manifest = manifest("../escape", 1, 100);
        assert!(matches!(
            AuthorizationStore::create(temp.path(), &unsafe_manifest),
            Err(AuthorizationError::UnsafeAuthorizationId(_))
        ));
        unsafe_manifest.authorization_id = "auth-safe".into();
        AuthorizationStore::create(temp.path(), &unsafe_manifest).unwrap();
        let config = ProviderAuthorizationConfig::new(temp.path(), "auth-safe");
        let session = AuthorizationStore::bind(config, "xai", "default").unwrap();
        assert!(matches!(
            session.reserve("angry", "edit_image", "test-image"),
            Err(AuthorizationError::TargetNotAllowed(_))
        ));
    }

    #[test]
    fn reservations_are_atomic_across_independent_sessions() {
        let temp = tempfile::tempdir().unwrap();
        AuthorizationStore::create(temp.path(), &manifest("auth-shared", 2, 200)).unwrap();
        let barrier = Arc::new(Barrier::new(8));
        let mut threads = Vec::new();
        for _ in 0..8 {
            let root = temp.path().to_path_buf();
            let barrier = barrier.clone();
            threads.push(thread::spawn(move || {
                let session = AuthorizationStore::bind(
                    ProviderAuthorizationConfig::new(root, "auth-shared"),
                    "xai",
                    "default",
                )
                .unwrap();
                barrier.wait();
                session.reserve("happy", "edit_image", "test-image")
            }));
        }
        let successes = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .filter(Result::is_ok)
            .count();
        assert_eq!(successes, 2);
        let ledger = AuthorizationStore::open(temp.path(), "auth-shared")
            .unwrap()
            .ledger()
            .unwrap();
        assert_eq!(ledger.consumed_request_count(), 2);
        assert_eq!(ledger.consumed_cost_ticks(), 200);
        assert!(ledger
            .requests
            .iter()
            .all(|request| request.state == RequestLedgerStateV1::Ambiguous));
    }

    #[test]
    fn settlement_replaces_reservation_with_observed_cost() {
        let temp = tempfile::tempdir().unwrap();
        AuthorizationStore::create(temp.path(), &manifest("auth-cost", 2, 150)).unwrap();
        let session = AuthorizationStore::bind(
            ProviderAuthorizationConfig::new(temp.path(), "auth-cost"),
            "xai",
            "default",
        )
        .unwrap();
        let mut first = session
            .reserve("happy", "edit_image", "test-image")
            .unwrap();
        first.mark_submitted().unwrap();
        first.settle(40).unwrap();
        let second = session
            .reserve("happy", "edit_image", "test-image")
            .unwrap();
        drop(second);
        let ledger = AuthorizationStore::open(temp.path(), "auth-cost")
            .unwrap()
            .ledger()
            .unwrap();
        assert_eq!(ledger.consumed_cost_ticks(), 140);
        assert_eq!(ledger.requests[0].state, RequestLedgerStateV1::Settled);
        assert_eq!(ledger.requests[1].state, RequestLedgerStateV1::Ambiguous);
    }

    #[test]
    fn asynchronous_request_id_survives_reopen_and_settles_once() {
        let temp = tempfile::tempdir().unwrap();
        AuthorizationStore::create(temp.path(), &manifest("auth-async", 1, 100)).unwrap();
        let session = AuthorizationStore::bind(
            ProviderAuthorizationConfig::new(temp.path(), "auth-async"),
            "xai",
            "default",
        )
        .unwrap();
        let mut request = session
            .reserve("happy", "generate_video", "test-image")
            .unwrap();
        request.mark_submitted().unwrap();
        request.bind_provider_request_id("video-remote-1").unwrap();

        let reopened = AuthorizationStore::bind(
            ProviderAuthorizationConfig::new(temp.path(), "auth-async"),
            "xai",
            "default",
        )
        .unwrap();
        assert!(reopened
            .finalize_provider_request("video-remote-1", Some(40))
            .unwrap());
        assert!(!reopened
            .finalize_provider_request("video-remote-1", Some(40))
            .unwrap());
        let ledger = AuthorizationStore::open(temp.path(), "auth-async")
            .unwrap()
            .ledger()
            .unwrap();
        assert_eq!(ledger.consumed_request_count(), 1);
        assert_eq!(ledger.consumed_cost_ticks(), 40);
        assert_eq!(ledger.requests[0].state, RequestLedgerStateV1::Settled);
        assert_eq!(
            ledger.requests[0].provider_request_id.as_deref(),
            Some("video-remote-1")
        );
    }

    #[test]
    fn asynchronous_terminal_response_without_cost_stays_conservatively_consumed() {
        let temp = tempfile::tempdir().unwrap();
        AuthorizationStore::create(temp.path(), &manifest("auth-async-unknown", 1, 100)).unwrap();
        let session = AuthorizationStore::bind(
            ProviderAuthorizationConfig::new(temp.path(), "auth-async-unknown"),
            "xai",
            "default",
        )
        .unwrap();
        let mut request = session
            .reserve("happy", "generate_video", "test-image")
            .unwrap();
        request.mark_submitted().unwrap();
        request
            .bind_provider_request_id("video-remote-unknown")
            .unwrap();
        assert!(session
            .finalize_provider_request("video-remote-unknown", None)
            .unwrap());
        let ledger = AuthorizationStore::open(temp.path(), "auth-async-unknown")
            .unwrap()
            .ledger()
            .unwrap();
        assert_eq!(ledger.requests[0].state, RequestLedgerStateV1::Ambiguous);
        assert_eq!(ledger.consumed_cost_ticks(), 100);
        assert_eq!(
            ledger.requests[0].provider_request_id.as_deref(),
            Some("video-remote-unknown")
        );
    }

    #[test]
    fn parent_child_and_replay_share_one_target_cap() {
        let temp = tempfile::tempdir().unwrap();
        AuthorizationStore::create(temp.path(), &manifest("auth-lineage", 2, 200)).unwrap();
        let session_for = |job_id: &str| {
            let mut config = ProviderAuthorizationConfig::new(temp.path(), "auth-lineage");
            config.job_id = Some(job_id.into());
            config.lineage_root_job_id = Some("root-job".into());
            AuthorizationStore::bind(config, "xai", "default").unwrap()
        };
        let mut parent = session_for("root-job")
            .reserve("happy", "edit_image", "test-image")
            .unwrap();
        parent.mark_submitted().unwrap();
        parent.settle(80).unwrap();
        let mut child = session_for("child-job")
            .reserve("happy", "edit_image", "test-image")
            .unwrap();
        child.mark_submitted().unwrap();
        child.settle(70).unwrap();
        assert!(matches!(
            session_for("replay-job").reserve("happy", "edit_image", "test-image"),
            Err(AuthorizationError::RequestBudgetExceeded(_))
        ));
        let ledger = AuthorizationStore::open(temp.path(), "auth-lineage")
            .unwrap()
            .ledger()
            .unwrap();
        assert_eq!(ledger.consumed_request_count(), 2);
        assert_eq!(ledger.consumed_cost_ticks(), 150);
        assert_eq!(ledger.requests[0].job_id.as_deref(), Some("root-job"));
        assert_eq!(ledger.requests[1].job_id.as_deref(), Some("child-job"));
        assert!(ledger
            .requests
            .iter()
            .all(|request| request.lineage_root_job_id.as_deref() == Some("root-job")));
    }

    #[test]
    fn constrained_context_mismatch_fails_before_ledger_reservation() {
        let temp = tempfile::tempdir().unwrap();
        let mut grant = manifest("auth-context-mismatch", 1, 100);
        grant.source_lineage_root_job_id = Some("root-job".into());
        grant.recipe_hash = Some("recipe-hash".into());
        grant.input_fingerprint = Some("input-hash".into());
        AuthorizationStore::create(temp.path(), &grant).unwrap();
        let session = AuthorizationStore::bind(
            ProviderAuthorizationConfig::new(temp.path(), "auth-context-mismatch"),
            "xai",
            "default",
        )
        .unwrap();
        session
            .set_execution_context("child-job", "root-job", "wrong-recipe", "input-hash")
            .unwrap();

        assert!(matches!(
            session.reserve("happy", "edit_image", "test-image"),
            Err(AuthorizationError::ExecutionContextMismatch {
                field: "recipeHash",
                ..
            })
        ));
        assert!(
            AuthorizationStore::open(temp.path(), "auth-context-mismatch")
                .unwrap()
                .ledger()
                .unwrap()
                .requests
                .is_empty()
        );
    }

    #[test]
    fn constrained_context_exactly_matches_before_reservation() {
        let temp = tempfile::tempdir().unwrap();
        let mut grant = manifest("auth-context-exact", 1, 100);
        grant.source_lineage_root_job_id = Some("root-job".into());
        grant.recipe_hash = Some("recipe-hash".into());
        grant.input_fingerprint = Some("input-hash".into());
        AuthorizationStore::create(temp.path(), &grant).unwrap();
        let session = AuthorizationStore::bind(
            ProviderAuthorizationConfig::new(temp.path(), "auth-context-exact"),
            "xai",
            "default",
        )
        .unwrap();
        session
            .set_execution_context("child-job", "root-job", "recipe-hash", "input-hash")
            .unwrap();

        let reservation = session
            .reserve("happy", "edit_image", "test-image")
            .unwrap();
        assert_eq!(
            AuthorizationStore::open(temp.path(), "auth-context-exact")
                .unwrap()
                .ledger()
                .unwrap()
                .requests
                .len(),
            1
        );
        reservation.release().unwrap();
    }

    #[test]
    fn exact_scope_requires_bound_identity_and_a_pristine_single_operation_grant() {
        let temp = tempfile::tempdir().unwrap();
        let mut grant = manifest("auth-exact-scope", 1, 1_400_000_000);
        grant.allowed_models = vec!["test-image".into()];
        grant.max_total_provider_operations = Some(1);
        grant.allowed_targets = vec![AuthorizedTargetV1 {
            target_id: "walk_down:frame:2".into(),
            max_requests: 1,
            cost_reservation_ticks_per_request: 1_400_000_000,
        }];
        grant.source_lineage_root_job_id = Some("root-job".into());
        grant.recipe_hash = Some("recipe-hash".into());
        grant.input_fingerprint = Some("input-hash".into());
        AuthorizationStore::create(temp.path(), &grant).unwrap();
        let session = AuthorizationStore::bind(
            ProviderAuthorizationConfig::new(temp.path(), "auth-exact-scope"),
            "xai",
            "default",
        )
        .unwrap();
        session
            .set_execution_context("child-job", "root-job", "recipe-hash", "input-hash")
            .unwrap();

        session
            .validate_exact_scope(
                "xai",
                "default",
                "test-image",
                "walk_down:frame:2",
                1,
                1,
                1_400_000_000,
                "root-job",
                "recipe-hash",
                "input-hash",
            )
            .unwrap();
        assert!(session
            .validate_exact_scope(
                "xai",
                "default",
                "test-image",
                "walk_down:frame:2",
                1,
                2,
                1_400_000_000,
                "root-job",
                "recipe-hash",
                "input-hash",
            )
            .is_err());

        let reservation = session
            .reserve("walk_down:frame:2", "edit_image", "test-image")
            .unwrap();
        reservation.release().unwrap();
        assert!(session
            .validate_exact_scope(
                "xai",
                "default",
                "test-image",
                "walk_down:frame:2",
                1,
                1,
                1_400_000_000,
                "root-job",
                "recipe-hash",
                "input-hash",
            )
            .is_err());
    }

    #[test]
    fn manifest_drift_after_scope_validation_fails_before_reservation() {
        let temp = tempfile::tempdir().unwrap();
        let mut grant = manifest("auth-scope-drift", 1, 100);
        grant.max_total_provider_operations = Some(1);
        grant.source_lineage_root_job_id = Some("root-job".into());
        grant.recipe_hash = Some("recipe-hash".into());
        grant.input_fingerprint = Some("input-hash".into());
        let store = AuthorizationStore::create(temp.path(), &grant).unwrap();
        let session = AuthorizationStore::bind(
            ProviderAuthorizationConfig::new(temp.path(), "auth-scope-drift"),
            "xai",
            "default",
        )
        .unwrap();
        session
            .set_execution_context("child-job", "root-job", "recipe-hash", "input-hash")
            .unwrap();
        session
            .validate_exact_scope(
                "xai",
                "default",
                "test-image",
                "happy",
                1,
                1,
                100,
                "root-job",
                "recipe-hash",
                "input-hash",
            )
            .unwrap();

        grant.max_total_provider_operations = Some(2);
        write_json_atomic(&store.manifest_path(), &grant).unwrap();

        assert!(matches!(
            session.reserve("happy", "edit_image", "test-image"),
            Err(AuthorizationError::ManifestChanged)
        ));
        assert!(store.ledger().unwrap().requests.is_empty());
    }

    #[test]
    fn expected_manifest_digest_rejects_replacement_during_provider_bind() {
        let temp = tempfile::tempdir().unwrap();
        let mut grant = manifest("auth-attach-drift", 1, 100);
        let store = AuthorizationStore::create(temp.path(), &grant).unwrap();
        let (_, reviewed_sha256) = store.manifest_snapshot().unwrap();
        grant.max_total_provider_operations = Some(2);
        write_json_atomic(&store.manifest_path(), &grant).unwrap();

        let mut config = ProviderAuthorizationConfig::new(temp.path(), "auth-attach-drift");
        config.expected_manifest_sha256 = Some(reviewed_sha256);
        assert!(matches!(
            AuthorizationStore::bind(config, "xai", "default"),
            Err(AuthorizationError::ManifestChanged)
        ));
        assert!(store.ledger().unwrap().requests.is_empty());
    }

    #[test]
    fn submitted_transport_failure_remains_ambiguously_consumed() {
        let temp = tempfile::tempdir().unwrap();
        AuthorizationStore::create(temp.path(), &manifest("auth-ambiguous", 1, 100)).unwrap();
        let session = AuthorizationStore::bind(
            ProviderAuthorizationConfig::new(temp.path(), "auth-ambiguous"),
            "xai",
            "default",
        )
        .unwrap();
        let mut request = session
            .reserve("happy", "edit_image", "test-image")
            .unwrap();
        request.mark_submitted().unwrap();
        drop(request);
        let ledger = AuthorizationStore::open(temp.path(), "auth-ambiguous")
            .unwrap()
            .ledger()
            .unwrap();
        assert_eq!(ledger.requests[0].state, RequestLedgerStateV1::Ambiguous);
        assert_eq!(ledger.consumed_cost_ticks(), 100);
        assert!(matches!(
            session.reserve("happy", "edit_image", "test-image"),
            Err(AuthorizationError::RequestBudgetExceeded(_))
        ));
    }

    #[test]
    fn compound_transport_and_model_attempt_use_separate_caps() {
        let temp = tempfile::tempdir().unwrap();
        let mut grant = manifest("auth-compound", 1, 100);
        grant.max_total_provider_operations = Some(3);
        AuthorizationStore::create(temp.path(), &grant).unwrap();
        let session = AuthorizationStore::bind(
            ProviderAuthorizationConfig::new(temp.path(), "auth-compound"),
            "xai",
            "default",
        )
        .unwrap();
        let mut reservations = session
            .reserve_compound(
                "happy",
                &[
                    ("private_file_upload", ProviderOperationClassV1::Transport),
                    ("edit_video", ProviderOperationClassV1::ModelGeneration),
                ],
                "test-image",
            )
            .unwrap();
        let mut upload = reservations.remove(0);
        upload.mark_submitted().unwrap();
        upload.settle(0).unwrap();
        let mut edit = reservations.remove(0);
        edit.mark_submitted().unwrap();
        edit.settle(80).unwrap();

        let ledger = AuthorizationStore::open(temp.path(), "auth-compound")
            .unwrap()
            .ledger()
            .unwrap();
        assert_eq!(ledger.consumed_request_count(), 1);
        assert_eq!(ledger.consumed_operation_count(), 2);
        assert_eq!(ledger.consumed_cost_ticks(), 80);
        assert_eq!(
            ledger.requests[0].operation_class,
            ProviderOperationClassV1::Transport
        );
        assert_eq!(ledger.requests[0].reserved_cost_ticks, 0);
        assert!(matches!(
            session.reserve("happy", "edit_video", "test-image"),
            Err(AuthorizationError::RequestBudgetExceeded(_))
        ));
    }

    #[test]
    fn unsubmitted_compound_model_reservation_can_be_released() {
        let temp = tempfile::tempdir().unwrap();
        let mut grant = manifest("auth-release", 1, 100);
        grant.max_total_provider_operations = Some(4);
        AuthorizationStore::create(temp.path(), &grant).unwrap();
        let session = AuthorizationStore::bind(
            ProviderAuthorizationConfig::new(temp.path(), "auth-release"),
            "xai",
            "default",
        )
        .unwrap();
        let mut reservations = session
            .reserve_compound(
                "happy",
                &[
                    ("private_file_upload", ProviderOperationClassV1::Transport),
                    ("edit_video", ProviderOperationClassV1::ModelGeneration),
                ],
                "test-image",
            )
            .unwrap();
        let mut upload = reservations.remove(0);
        upload.mark_submitted().unwrap();
        upload.mark_ambiguous().unwrap();
        reservations.remove(0).release().unwrap();

        let ledger = AuthorizationStore::open(temp.path(), "auth-release")
            .unwrap()
            .ledger()
            .unwrap();
        assert_eq!(ledger.consumed_request_count(), 0);
        assert_eq!(ledger.consumed_operation_count(), 2);
        assert_eq!(ledger.consumed_cost_ticks(), 0);
        assert_eq!(ledger.requests[1].state, RequestLedgerStateV1::Released);

        let retry = session.reserve("happy", "edit_video", "test-image");
        assert!(retry.is_ok());
    }

    #[test]
    fn exact_one_operation_grant_cannot_reuse_a_released_slot() {
        let temp = tempfile::tempdir().unwrap();
        let mut grant = manifest("auth-one-operation", 1, 100);
        grant.max_total_provider_operations = Some(1);
        AuthorizationStore::create(temp.path(), &grant).unwrap();
        let session = AuthorizationStore::bind(
            ProviderAuthorizationConfig::new(temp.path(), "auth-one-operation"),
            "xai",
            "default",
        )
        .unwrap();

        session
            .reserve("happy", "edit_image", "test-image")
            .unwrap()
            .release()
            .unwrap();

        assert!(matches!(
            session.reserve("happy", "edit_image", "test-image"),
            Err(AuthorizationError::RequestBudgetExceeded(_))
        ));
    }

    #[test]
    fn persisted_documents_have_no_credential_fields() {
        let temp = tempfile::tempdir().unwrap();
        AuthorizationStore::create(temp.path(), &manifest("auth-redaction", 1, 100)).unwrap();
        let store = AuthorizationStore::open(temp.path(), "auth-redaction").unwrap();
        let combined = format!(
            "{}\n{}",
            fs::read_to_string(store.manifest_path()).unwrap(),
            fs::read_to_string(store.ledger_path()).unwrap()
        )
        .to_ascii_lowercase();
        for forbidden in [
            "authorizationheader",
            "bearer",
            "access_token",
            "refresh_token",
            "device_code",
            "api_key",
            "temporaryurl",
            "prompt",
        ] {
            assert!(!combined.contains(forbidden), "found {forbidden}");
        }
    }
}
