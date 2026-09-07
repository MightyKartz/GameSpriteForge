use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use base64::Engine as _;
#[cfg(test)]
use forge_core::provider::ProviderInputRef;
use forge_core::provider::{
    CredentialKind, CredentialProvider, EditImageRequest, EditVideoRequest, GenerateImageRequest,
    GenerateVideoRequest, MediaGenerationProvider, ProviderCapability, ProviderConstraints,
    ProviderError, ProviderHealth, ProviderImageReference, ProviderMedia, ProviderPoll,
    ProviderTicket, ProviderUsage, VideoGenerationMode,
};
use reqwest::blocking::multipart::{Form, Part};
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::StatusCode;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use url::Url;

use crate::authorization::{
    AuthorizationError, AuthorizationStore, ProviderAuthorizationConfig,
    ProviderAuthorizationSession, ProviderOperationClassV1, RequestReservation,
};

const DEFAULT_BASE_URL: &str = "https://api.x.ai/v1";
pub const DEFAULT_IMAGE_MODEL: &str = "grok-imagine-image-quality";
pub const IMAGE_2_CANDIDATE_MODEL: &str = forge_core::automation::XAI_IMAGE_2_CANDIDATE_MODEL;
pub const DEFAULT_VIDEO_MODEL: &str = "grok-imagine-video-1.5";
pub const DEFAULT_VIDEO_EDIT_MODEL: &str = "grok-imagine-video";
const JSON_RESPONSE_LIMIT: usize = 64 * 1024 * 1024;
const MEDIA_RESPONSE_LIMIT: u64 = 512 * 1024 * 1024;
const VIDEO_DATA_URL_INPUT_LIMIT: u64 = 32 * 1024 * 1024;
const REAL_PROVIDER_ACCEPT_ENV: &str = "FORGE_REAL_PROVIDER_ACCEPT";
const REAL_PROVIDER_MAX_REQUESTS_ENV: &str = "FORGE_REAL_PROVIDER_MAX_REQUESTS";
const REAL_PROVIDER_MAX_COST_TICKS_ENV: &str = "FORGE_REAL_PROVIDER_MAX_COST_TICKS";

#[derive(Debug, Clone, Copy)]
struct RealProviderBudgetLimits {
    max_requests: u32,
    max_cost_ticks: u64,
}

#[derive(Debug, Default)]
struct RealProviderBudgetState {
    limits: Option<RealProviderBudgetLimits>,
    reserved_requests: u32,
    observed_cost_ticks: u64,
}

#[derive(Debug, Default)]
struct RealProviderBudgetGuard {
    state: Mutex<RealProviderBudgetState>,
}

impl RealProviderBudgetGuard {
    fn ensure_accepted(&self) -> Result<RealProviderBudgetLimits, ProviderError> {
        let mut state = self.state.lock().unwrap();
        if let Some(limits) = state.limits {
            return Ok(limits);
        }
        if env::var(REAL_PROVIDER_ACCEPT_ENV).as_deref() != Ok("1") {
            return Err(ProviderError::RealProviderNotAccepted(format!(
                "set {REAL_PROVIDER_ACCEPT_ENV}=1 only after reviewing the plan"
            )));
        }
        let max_requests = parse_positive_env::<u32>(REAL_PROVIDER_MAX_REQUESTS_ENV)?;
        let max_cost_ticks = parse_positive_env::<u64>(REAL_PROVIDER_MAX_COST_TICKS_ENV)?;
        let limits = RealProviderBudgetLimits {
            max_requests,
            max_cost_ticks,
        };
        state.limits = Some(limits);
        Ok(limits)
    }

    fn reserve_request(&self) -> Result<(), ProviderError> {
        let limits = self.ensure_accepted()?;
        let mut state = self.state.lock().unwrap();
        if state.reserved_requests >= limits.max_requests {
            return Err(ProviderError::RequestBudgetExceeded(format!(
                "real Provider request limit {} has been reached",
                limits.max_requests
            )));
        }
        if state.observed_cost_ticks >= limits.max_cost_ticks {
            return Err(ProviderError::CostBudgetExceeded(format!(
                "real Provider cost limit {} ticks has been reached",
                limits.max_cost_ticks
            )));
        }
        state.reserved_requests += 1;
        Ok(())
    }

    fn record_cost(&self, ticks: Option<u64>) -> Result<(), ProviderError> {
        let limits = self.ensure_accepted()?;
        let ticks = ticks.ok_or_else(|| {
            ProviderError::CostBudgetExceeded(
                "real Provider response omitted costInUsdTicks; refusing an unmetered result"
                    .into(),
            )
        })?;
        let mut state = self.state.lock().unwrap();
        state.observed_cost_ticks = state.observed_cost_ticks.saturating_add(ticks);
        if state.observed_cost_ticks > limits.max_cost_ticks {
            return Err(ProviderError::CostBudgetExceeded(format!(
                "real Provider cost {} exceeds the configured limit {} ticks",
                state.observed_cost_ticks, limits.max_cost_ticks
            )));
        }
        Ok(())
    }
}

fn parse_positive_env<T>(name: &str) -> Result<T, ProviderError>
where
    T: std::str::FromStr + PartialEq + Default,
{
    let raw = env::var(name).map_err(|_| {
        ProviderError::RealProviderNotAccepted(format!(
            "{name} is required for real Provider execution"
        ))
    })?;
    let value = raw.parse::<T>().map_err(|_| {
        ProviderError::RealProviderNotAccepted(format!("{name} must be a positive integer"))
    })?;
    if value == T::default() {
        return Err(ProviderError::RealProviderNotAccepted(format!(
            "{name} must be greater than zero"
        )));
    }
    Ok(value)
}

fn provider_authorization_error(error: AuthorizationError) -> ProviderError {
    match error {
        AuthorizationError::RequestBudgetExceeded(message) => {
            ProviderError::RequestBudgetExceeded(message)
        }
        AuthorizationError::CostBudgetExceeded(message) => {
            ProviderError::CostBudgetExceeded(message)
        }
        other => ProviderError::RealProviderNotAccepted(other.to_string()),
    }
}

pub struct XaiProvider {
    credentials: Arc<dyn CredentialProvider>,
    client: Client,
    base_url: String,
    usage: Mutex<ProviderUsage>,
    temporary_files: Mutex<HashMap<String, String>>,
    real_budget: Option<RealProviderBudgetGuard>,
    authorization: Option<ProviderAuthorizationSession>,
}

impl XaiProvider {
    pub fn new(credentials: Arc<dyn CredentialProvider>) -> Self {
        Self::with_base_url(credentials, DEFAULT_BASE_URL)
            .expect("the bundled xAI base URL must be valid")
    }

    pub fn new_with_authorization(
        credentials: Arc<dyn CredentialProvider>,
        profile_id: &str,
        authorization: ProviderAuthorizationConfig,
    ) -> Result<Self, ProviderError> {
        Self::with_base_url_and_authorization(
            credentials,
            DEFAULT_BASE_URL,
            profile_id,
            authorization,
        )
    }

    pub fn with_base_url(
        credentials: Arc<dyn CredentialProvider>,
        base_url: impl Into<String>,
    ) -> Result<Self, ProviderError> {
        Self::with_base_url_internal(credentials, base_url.into(), None)
    }

    pub fn with_base_url_and_authorization(
        credentials: Arc<dyn CredentialProvider>,
        base_url: impl Into<String>,
        profile_id: &str,
        authorization: ProviderAuthorizationConfig,
    ) -> Result<Self, ProviderError> {
        let authorization = AuthorizationStore::bind(authorization, "xai", profile_id)
            .map_err(provider_authorization_error)?;
        Self::with_base_url_internal(credentials, base_url.into(), Some(authorization))
    }

    fn with_base_url_internal(
        credentials: Arc<dyn CredentialProvider>,
        base_url: String,
        authorization: Option<ProviderAuthorizationSession>,
    ) -> Result<Self, ProviderError> {
        let base_url = base_url.trim_end_matches('/').to_string();
        validate_base_url(&base_url, credentials.kind())?;
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(20))
            .timeout(Duration::from_secs(300))
            .http1_only()
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("Game-Sprite-Forge/0.1")
            .build()
            .map_err(|error| ProviderError::Request(error.to_string()))?;
        let real_budget =
            (!base_url_is_loopback(&base_url) && authorization.is_none()).then(Default::default);
        Ok(Self {
            credentials,
            client,
            base_url,
            usage: Mutex::new(ProviderUsage::default()),
            temporary_files: Mutex::new(HashMap::new()),
            real_budget,
            authorization,
        })
    }

    pub fn capability_list() -> Vec<ProviderCapability> {
        vec![
            ProviderCapability::GenerateImage,
            ProviderCapability::EditImage,
            ProviderCapability::GenerateVideo,
            ProviderCapability::ImageToVideo,
            ProviderCapability::ReferenceToVideo,
            ProviderCapability::EditVideo,
            ProviderCapability::PrivateFileInput,
            ProviderCapability::Cancel,
            ProviderCapability::Usage,
        ]
    }

    pub fn default_image_model() -> &'static str {
        DEFAULT_IMAGE_MODEL
    }

    pub fn default_video_model() -> &'static str {
        DEFAULT_VIDEO_MODEL
    }

    pub fn default_video_edit_model() -> &'static str {
        DEFAULT_VIDEO_EDIT_MODEL
    }

    pub fn constraints() -> ProviderConstraints {
        ProviderConstraints {
            max_image_references: Some(3),
            max_video_references: Some(7),
            native_alpha: false,
            video_edit: true,
            end_frame: false,
            private_file_input: true,
        }
    }

    fn image_request(
        &self,
        endpoint: &str,
        payload: Value,
        authorization_target: Option<&str>,
        output_path: &Path,
    ) -> Result<ProviderMedia, ProviderError> {
        let model = payload
            .get("model")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProviderError::InvalidOutput("xAI image request omitted model".into())
            })?;
        let operation = if endpoint.ends_with("edits") {
            "edit_image"
        } else {
            "generate_image"
        };
        let mut reservation = self.reserve_real_request(authorization_target, operation, model)?;
        if let Some(reservation) = reservation.as_mut() {
            reservation
                .mark_submitted()
                .map_err(provider_authorization_error)?;
        }
        let url = format!("{}/{}", self.base_url, endpoint.trim_start_matches('/'));
        let exact_one_operation = self.exact_one_operation_authorization()?;
        let response = self.send_authenticated(|bearer| {
            self.client
                .post(&url)
                .bearer_auth(bearer)
                .header("Content-Type", "application/json")
                .json(&payload)
        })?;
        let value = parse_json_response(response, "xAI image request")?;
        let ticks = self.record_usage(&value, true, false)?;
        self.settle_real_request(reservation, ticks)?;
        let first = value
            .get("data")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .ok_or_else(|| ProviderError::InvalidOutput("xAI returned no image".into()))?;
        self.ensure_authorization_manifest_unchanged()?;
        let bytes = if let Some(encoded) = first.get("b64_json").and_then(Value::as_str) {
            base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .map_err(|error| ProviderError::InvalidOutput(error.to_string()))?
        } else if !exact_one_operation {
            let url = first.get("url").and_then(Value::as_str).ok_or_else(|| {
                ProviderError::InvalidOutput(
                    "xAI image response omitted both b64_json and url".into(),
                )
            })?;
            // response_format=b64_json is requested, but URL fallback keeps the
            // provider compatible with xAI deployments that currently return
            // only temporary media URLs. The URL is consumed immediately and
            // never persisted in the Forge job manifest.
            download_media(
                &self.client,
                url,
                output_path,
                base_url_is_loopback(&self.base_url),
            )?;
            fs::read(output_path)?
        } else {
            return Err(ProviderError::InvalidOutput(
                "exact-one-operation xAI image requests require b64_json output".into(),
            ));
        };
        if detect_image_mime(&bytes) == "application/octet-stream" {
            let _ = fs::remove_file(output_path);
            return Err(ProviderError::InvalidOutput(
                "xAI returned malformed image bytes".into(),
            ));
        }
        let decoded = image::load_from_memory(&bytes).map_err(|error| {
            ProviderError::InvalidOutput(format!("xAI returned undecodable image: {error}"))
        })?;
        let mut canonical_png = Vec::new();
        decoded
            .write_to(
                &mut std::io::Cursor::new(&mut canonical_png),
                image::ImageFormat::Png,
            )
            .map_err(|error| ProviderError::InvalidOutput(error.to_string()))?;
        write_atomic(output_path, &canonical_png)?;
        Ok(ProviderMedia {
            path: output_path.to_path_buf(),
            mime_type: "image/png".into(),
            provider_asset_id: first
                .pointer("/file_output/file_id")
                .and_then(Value::as_str)
                .map(str::to_string),
            revised_prompt: first
                .get("revised_prompt")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
        })
    }

    fn send_authenticated<F>(&self, build: F) -> Result<Response, ProviderError>
    where
        F: Fn(&str) -> RequestBuilder,
    {
        let exact_one_operation = self.exact_one_operation_authorization()?;
        let mut bearer = self.credentials.bearer()?;
        let mut refreshed = false;
        let mut rate_attempt = 0u32;
        loop {
            self.ensure_authorization_manifest_unchanged()?;
            let response = build(&bearer)
                .send()
                .map_err(|error| ProviderError::Request(error.to_string()))?;
            // A response may race with a durable-manifest replacement. Check
            // before any retry or caller-visible follow-up decision.
            self.ensure_authorization_manifest_unchanged()?;
            if response.status() == StatusCode::UNAUTHORIZED && !exact_one_operation && !refreshed {
                bearer = self.credentials.refresh()?;
                refreshed = true;
                continue;
            }
            if response.status() == StatusCode::TOO_MANY_REQUESTS
                && !exact_one_operation
                && rate_attempt < 3
            {
                rate_attempt += 1;
                let seconds = response
                    .headers()
                    .get("retry-after")
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(1 << (rate_attempt - 1))
                    .clamp(1, 8);
                thread::sleep(Duration::from_secs(seconds));
                continue;
            }
            if response.status() == StatusCode::TOO_MANY_REQUESTS && exact_one_operation {
                return Err(ProviderError::RateLimited(
                    "xAI rate limited the exact-one-operation request; no retry was attempted"
                        .into(),
                ));
            }
            return classify_response(response);
        }
    }

    fn ensure_real_accepted(&self) -> Result<(), ProviderError> {
        if let Some(authorization) = &self.authorization {
            authorization
                .ensure_active()
                .map_err(provider_authorization_error)?;
        }
        self.real_budget
            .as_ref()
            .map(RealProviderBudgetGuard::ensure_accepted)
            .transpose()
            .map(|_| ())
    }

    fn exact_one_operation_authorization(&self) -> Result<bool, ProviderError> {
        self.authorization
            .as_ref()
            .map(ProviderAuthorizationSession::is_exactly_one_operation)
            .transpose()
            .map(|value| value.unwrap_or(false))
            .map_err(provider_authorization_error)
    }

    fn ensure_authorization_manifest_unchanged(&self) -> Result<(), ProviderError> {
        self.authorization
            .as_ref()
            .map(ProviderAuthorizationSession::ensure_manifest_unchanged)
            .transpose()
            .map(|_| ())
            .map_err(provider_authorization_error)
    }

    fn reserve_real_request(
        &self,
        authorization_target: Option<&str>,
        operation: &str,
        model: &str,
    ) -> Result<Option<RequestReservation>, ProviderError> {
        self.reserve_real_request_class(
            authorization_target,
            operation,
            model,
            ProviderOperationClassV1::ModelGeneration,
        )
    }

    fn reserve_real_request_class(
        &self,
        authorization_target: Option<&str>,
        operation: &str,
        model: &str,
        operation_class: ProviderOperationClassV1,
    ) -> Result<Option<RequestReservation>, ProviderError> {
        if let Some(authorization) = &self.authorization {
            let target = authorization_target.ok_or_else(|| {
                ProviderError::RealProviderNotAccepted(
                    "durably authorized xAI calls require an explicit authorizationTarget".into(),
                )
            })?;
            return authorization
                .reserve_compound(target, &[(operation, operation_class)], model)
                .and_then(|mut reservations| {
                    reservations.pop().ok_or_else(|| {
                        AuthorizationError::InvalidLedger("empty reservation set".into())
                    })
                })
                .map(Some)
                .map_err(provider_authorization_error);
        }
        if operation_class == ProviderOperationClassV1::ModelGeneration {
            self.real_budget
                .as_ref()
                .map(RealProviderBudgetGuard::reserve_request)
                .transpose()?;
        }
        Ok(None)
    }

    fn reserve_real_compound(
        &self,
        authorization_target: Option<&str>,
        operations: &[(&str, ProviderOperationClassV1)],
        model: &str,
    ) -> Result<Vec<Option<RequestReservation>>, ProviderError> {
        if let Some(authorization) = &self.authorization {
            let target = authorization_target.ok_or_else(|| {
                ProviderError::RealProviderNotAccepted(
                    "durably authorized xAI calls require an explicit authorizationTarget".into(),
                )
            })?;
            return authorization
                .reserve_compound(target, operations, model)
                .map(|reservations| reservations.into_iter().map(Some).collect())
                .map_err(provider_authorization_error);
        }
        for (_, operation_class) in operations {
            if *operation_class == ProviderOperationClassV1::ModelGeneration {
                self.real_budget
                    .as_ref()
                    .map(RealProviderBudgetGuard::reserve_request)
                    .transpose()?;
            }
        }
        Ok((0..operations.len()).map(|_| None).collect())
    }

    fn record_usage(
        &self,
        value: &Value,
        image: bool,
        video: bool,
    ) -> Result<Option<u64>, ProviderError> {
        let ticks = value
            .pointer("/usage/cost_in_usd_ticks")
            .and_then(Value::as_u64);
        let mut usage = self.usage.lock().unwrap();
        usage.requests += 1;
        if image {
            usage.generated_images += 1;
        }
        if video {
            usage.generated_videos += 1;
        }
        if let Some(ticks) = ticks {
            usage.cost_in_usd_ticks = Some(usage.cost_in_usd_ticks.unwrap_or(0) + ticks);
        }
        drop(usage);
        if let Some(budget) = &self.real_budget {
            budget.record_cost(ticks)?;
        }
        Ok(ticks)
    }

    fn settle_real_request(
        &self,
        reservation: Option<RequestReservation>,
        observed_cost_ticks: Option<u64>,
    ) -> Result<(), ProviderError> {
        let Some(reservation) = reservation else {
            return Ok(());
        };
        let Some(observed_cost_ticks) = observed_cost_ticks else {
            reservation
                .mark_ambiguous()
                .map_err(provider_authorization_error)?;
            return Err(ProviderError::CostBudgetExceeded(
                "real Provider response omitted costInUsdTicks; the durable reservation remains consumed"
                    .into(),
            ));
        };
        reservation
            .settle(observed_cost_ticks)
            .map_err(provider_authorization_error)
    }

    fn bind_async_real_request(
        &self,
        reservation: Option<RequestReservation>,
        provider_request_id: &str,
        observed_cost_ticks: Option<u64>,
    ) -> Result<(), ProviderError> {
        let Some(reservation) = reservation else {
            return Ok(());
        };
        reservation
            .bind_provider_request_id(provider_request_id)
            .map_err(provider_authorization_error)?;
        if let Some(observed_cost_ticks) = observed_cost_ticks {
            self.authorization
                .as_ref()
                .expect("durable reservation requires an authorization session")
                .finalize_provider_request(provider_request_id, Some(observed_cost_ticks))
                .map_err(provider_authorization_error)?;
        }
        Ok(())
    }

    fn finalize_async_real_request(
        &self,
        provider_request_id: &str,
        observed_cost_ticks: Option<u64>,
    ) -> Result<bool, ProviderError> {
        let Some(authorization) = &self.authorization else {
            return Ok(false);
        };
        authorization
            .finalize_provider_request(provider_request_id, observed_cost_ticks)
            .map_err(provider_authorization_error)
    }

    fn upload_private_file(
        &self,
        bytes: &[u8],
        file_name: &str,
        model: &str,
        authorization_target: Option<&str>,
        reservation: Option<RequestReservation>,
    ) -> Result<String, ProviderError> {
        let mut reservation = match reservation {
            Some(reservation) => Some(reservation),
            None => self.reserve_real_request_class(
                authorization_target,
                "private_file_upload",
                model,
                ProviderOperationClassV1::Transport,
            )?,
        };
        if let Some(reservation) = reservation.as_mut() {
            reservation
                .mark_submitted()
                .map_err(provider_authorization_error)?;
        }
        let url = format!("{}/files", self.base_url);
        let bytes = bytes.to_vec();
        let file_name = file_name.to_string();
        self.usage.lock().unwrap().requests += 1;
        let response = self.send_authenticated(|bearer| {
            let part = Part::bytes(bytes.clone())
                .file_name(file_name.clone())
                .mime_str("video/mp4")
                .expect("static video MIME type is valid");
            let form = Form::new()
                .text("expires_after", "3600")
                .text("purpose", "assistants")
                .part("file", part);
            self.client.post(&url).bearer_auth(bearer).multipart(form)
        })?;
        let value = parse_json_response(response, "xAI private file upload")?;
        let file_id = value
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| valid_file_id(id))
            .ok_or_else(|| ProviderError::InvalidOutput("xAI file upload omitted id".into()))?;
        let mut usage = self.usage.lock().unwrap();
        usage.private_file_uploads += 1;
        drop(usage);
        self.settle_real_request(reservation, Some(0))?;
        Ok(file_id.into())
    }

    fn delete_private_file(&self, file_id: &str) -> Result<(), ProviderError> {
        if !valid_file_id(file_id) {
            return Err(ProviderError::InvalidOutput(
                "refusing to delete an invalid xAI file id".into(),
            ));
        }
        self.ensure_real_accepted()?;
        let url = format!("{}/files/{file_id}", self.base_url);
        self.usage.lock().unwrap().requests += 1;
        self.send_authenticated(|bearer| self.client.delete(&url).bearer_auth(bearer))?;
        Ok(())
    }

    fn cleanup_ticket_file(&self, request_id: &str) -> Result<(), ProviderError> {
        let file_id = self.temporary_files.lock().unwrap().remove(request_id);
        if let Some(file_id) = file_id {
            self.delete_private_file(&file_id)?;
        }
        Ok(())
    }
}

impl MediaGenerationProvider for XaiProvider {
    fn id(&self) -> &'static str {
        "xai"
    }

    fn capabilities(&self) -> Vec<ProviderCapability> {
        Self::capability_list()
    }

    fn health_check(&self) -> ProviderHealth {
        ProviderHealth {
            provider_id: self.id().into(),
            available: true,
            authenticated: self.credentials.bearer().is_ok(),
            auth_kind: self.credentials.kind(),
            capabilities: self.capabilities(),
            constraints: Some(Self::constraints()),
            message: if self.credentials.kind() == CredentialKind::OAuthDeviceCode {
                Some("Preview OAuth; commercial distribution requires xAI confirmation".into())
            } else {
                None
            },
        }
    }

    fn set_authorization_job_context(
        &self,
        job_id: &str,
        lineage_root_job_id: &str,
    ) -> Result<(), ProviderError> {
        if let Some(authorization) = &self.authorization {
            authorization
                .set_job_context(job_id, lineage_root_job_id)
                .map_err(provider_authorization_error)?;
        }
        Ok(())
    }

    fn set_authorization_execution_context(
        &self,
        job_id: &str,
        lineage_root_job_id: &str,
        recipe_hash: &str,
        input_fingerprint: &str,
    ) -> Result<(), ProviderError> {
        if let Some(authorization) = &self.authorization {
            authorization
                .set_execution_context(job_id, lineage_root_job_id, recipe_hash, input_fingerprint)
                .map_err(provider_authorization_error)?;
        }
        Ok(())
    }

    fn durable_authorization_id(&self) -> Result<Option<String>, ProviderError> {
        self.authorization
            .as_ref()
            .map(ProviderAuthorizationSession::authorization_id)
            .transpose()
            .map_err(provider_authorization_error)
    }

    fn durable_authorization_manifest_sha256(&self) -> Result<Option<String>, ProviderError> {
        self.authorization
            .as_ref()
            .map(ProviderAuthorizationSession::manifest_sha256)
            .transpose()
            .map_err(provider_authorization_error)
    }

    fn validate_durable_authorization_scope(
        &self,
        provider_id: &str,
        profile_id: &str,
        model: &str,
        target: &str,
        max_requests: u32,
        max_operations: u32,
        max_cost_ticks: u64,
        lineage_root_job_id: &str,
        recipe_hash: &str,
        input_fingerprint: &str,
    ) -> Result<(), ProviderError> {
        self.authorization
            .as_ref()
            .ok_or_else(|| {
                ProviderError::RealProviderNotAccepted(
                    "durable Provider authorization is required".into(),
                )
            })?
            .validate_exact_scope(
                provider_id,
                profile_id,
                model,
                target,
                max_requests,
                max_operations,
                max_cost_ticks,
                lineage_root_job_id,
                recipe_hash,
                input_fingerprint,
            )
            .map_err(provider_authorization_error)
    }

    fn resolved_image_model(&self, requested: Option<&str>) -> Option<String> {
        Some(requested.unwrap_or(DEFAULT_IMAGE_MODEL).to_string())
    }

    fn resolved_video_model(&self, requested: Option<&str>) -> Option<String> {
        Some(requested.unwrap_or(DEFAULT_VIDEO_MODEL).to_string())
    }

    fn resolved_video_edit_model(&self, requested: Option<&str>) -> Option<String> {
        Some(requested.unwrap_or(DEFAULT_VIDEO_EDIT_MODEL).to_string())
    }

    fn generate_image(
        &self,
        request: &GenerateImageRequest,
        output_path: &Path,
    ) -> Result<ProviderMedia, ProviderError> {
        self.image_request(
            "images/generations",
            json!({
                "model": request.model.as_deref().unwrap_or(DEFAULT_IMAGE_MODEL),
                "prompt": request.prompt,
                "n": 1,
                "aspect_ratio": request.aspect_ratio,
                "resolution": request.resolution,
                "response_format": "b64_json"
            }),
            request.authorization_target.as_deref(),
            output_path,
        )
    }

    fn edit_image(
        &self,
        request: &EditImageRequest,
        output_path: &Path,
    ) -> Result<ProviderMedia, ProviderError> {
        if request.references.is_empty() || request.references.len() > 3 {
            return Err(ProviderError::InvalidOutput(
                "xAI image edit requires one to three reference images".into(),
            ));
        }
        let images = request
            .references
            .iter()
            .map(|reference| {
                reference_image_data_url(reference).map(|url| {
                    json!({
                        "url": url,
                        "type": "image_url",
                        "detail": "high"
                    })
                })
            })
            .collect::<Result<Vec<_>, ProviderError>>()?;
        let mut payload = json!({
            "model": request.model.as_deref().unwrap_or(DEFAULT_IMAGE_MODEL),
            "prompt": request.prompt,
            "aspect_ratio": request.aspect_ratio,
            "resolution": request.resolution,
            "response_format": "b64_json"
        });
        if images.len() == 1 {
            payload["image"] = images[0].clone();
        } else {
            payload["images"] = Value::Array(images);
        }
        self.image_request(
            "images/edits",
            payload,
            request.authorization_target.as_deref(),
            output_path,
        )
    }

    fn generate_video(
        &self,
        request: &GenerateVideoRequest,
    ) -> Result<ProviderTicket, ProviderError> {
        if !(1..=15).contains(&request.duration_seconds) {
            return Err(ProviderError::InvalidOutput(
                "xAI video duration must be between 1 and 15 seconds".into(),
            ));
        }
        let mut payload = json!({
            "model": request.model.as_deref().unwrap_or(DEFAULT_VIDEO_MODEL),
            "prompt": request.prompt,
            "duration": request.duration_seconds,
            "aspect_ratio": request.aspect_ratio,
            "resolution": request.resolution
        });
        match &request.mode {
            VideoGenerationMode::Text => {}
            VideoGenerationMode::ImageToVideo { image } => {
                payload["image"] = json!({"url": image_data_url(image)?});
            }
            VideoGenerationMode::ReferenceToVideo { images } => {
                if images.is_empty() || images.len() > 7 {
                    return Err(ProviderError::InvalidOutput(
                        "xAI reference-to-video requires one to seven images".into(),
                    ));
                }
                payload["reference_images"] = Value::Array(
                    images
                        .iter()
                        .map(|path| image_data_url(path).map(|url| json!({"url": url})))
                        .collect::<Result<Vec<_>, ProviderError>>()?,
                );
            }
        }
        let model = request.model.as_deref().unwrap_or(DEFAULT_VIDEO_MODEL);
        let mut reservation = self.reserve_real_request(
            request.authorization_target.as_deref(),
            "generate_video",
            model,
        )?;
        if let Some(reservation) = reservation.as_mut() {
            reservation
                .mark_submitted()
                .map_err(provider_authorization_error)?;
        }
        let url = format!("{}/videos/generations", self.base_url);
        let response = self.send_authenticated(|bearer| {
            self.client
                .post(&url)
                .bearer_auth(bearer)
                .header("Content-Type", "application/json")
                .json(&payload)
        })?;
        let value = parse_json_response(response, "xAI video generation")?;
        let request_id = value
            .get("request_id")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ProviderError::InvalidOutput("xAI omitted video request_id".into()))?;
        let ticks = value
            .pointer("/usage/cost_in_usd_ticks")
            .and_then(Value::as_u64);
        self.bind_async_real_request(reservation, request_id, ticks)?;
        let _ = self.record_usage(&value, false, false)?;
        Ok(ProviderTicket {
            provider_id: self.id().into(),
            request_id: request_id.into(),
        })
    }

    fn edit_video(&self, request: &EditVideoRequest) -> Result<ProviderTicket, ProviderError> {
        let bytes = fs::read(&request.video.path)?;
        let actual_sha256 = format!("{:x}", Sha256::digest(&bytes));
        if actual_sha256 != request.video.sha256 {
            return Err(ProviderError::InvalidOutput(
                "video input SHA-256 changed before xAI edit".into(),
            ));
        }
        let model = request.model.as_deref().unwrap_or(DEFAULT_VIDEO_EDIT_MODEL);
        let existing_file_id = request
            .video
            .provider_asset_id
            .as_deref()
            .filter(|id| valid_file_id(id));
        let operations = if existing_file_id.is_some() {
            vec![("edit_video", ProviderOperationClassV1::ModelGeneration)]
        } else {
            vec![
                ("private_file_upload", ProviderOperationClassV1::Transport),
                ("edit_video", ProviderOperationClassV1::ModelGeneration),
            ]
        };
        let mut compound = self
            .reserve_real_compound(request.authorization_target.as_deref(), &operations, model)?
            .into_iter();
        let mut upload_reservation = existing_file_id
            .is_none()
            .then(|| compound.next().flatten())
            .flatten();
        let mut edit_reservation = compound.next().flatten();
        let mut temporary_file_id = None;
        let video = if let Some(file_id) = existing_file_id {
            json!({"file_id": file_id})
        } else {
            let file_name = request
                .video
                .path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("forge-animation.mp4");
            match self.upload_private_file(
                &bytes,
                file_name,
                model,
                request.authorization_target.as_deref(),
                upload_reservation.take(),
            ) {
                Ok(file_id) => {
                    temporary_file_id = Some(file_id.clone());
                    json!({"file_id": file_id})
                }
                Err(error) if files_api_unavailable(&error) => {
                    if bytes.len() as u64 > VIDEO_DATA_URL_INPUT_LIMIT {
                        if let Some(reservation) = edit_reservation.take() {
                            reservation
                                .release()
                                .map_err(provider_authorization_error)?;
                        }
                        return Err(ProviderError::Unavailable(format!(
                            "xAI Files API is unavailable and the video exceeds the {} MiB data URL fallback limit: {error}",
                            VIDEO_DATA_URL_INPUT_LIMIT / (1024 * 1024)
                        )));
                    }
                    json!({
                        "url": format!(
                            "data:video/mp4;base64,{}",
                            base64::engine::general_purpose::STANDARD.encode(&bytes)
                        )
                    })
                }
                Err(error) => {
                    if let Some(reservation) = edit_reservation.take() {
                        reservation
                            .release()
                            .map_err(provider_authorization_error)?;
                    }
                    return Err(error);
                }
            }
        };
        let payload = json!({
            "model": request.model.as_deref().unwrap_or(DEFAULT_VIDEO_EDIT_MODEL),
            "prompt": request.prompt,
            "video": video,
        });
        let mut reservation = edit_reservation;
        if let Some(reservation) = reservation.as_mut() {
            reservation
                .mark_submitted()
                .map_err(provider_authorization_error)?;
        }
        let url = format!("{}/videos/edits", self.base_url);
        let response = self.send_authenticated(|bearer| {
            self.client
                .post(&url)
                .bearer_auth(bearer)
                .header("Content-Type", "application/json")
                .json(&payload)
        });
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                if let Some(file_id) = temporary_file_id.as_deref() {
                    let _ = self.delete_private_file(file_id);
                }
                return Err(error);
            }
        };
        let value = match parse_json_response(response, "xAI video edit") {
            Ok(value) => value,
            Err(error) => {
                if let Some(file_id) = temporary_file_id.as_deref() {
                    let _ = self.delete_private_file(file_id);
                }
                return Err(error);
            }
        };
        let request_id = match value
            .get("request_id")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        {
            Some(request_id) => request_id,
            None => {
                if let Some(file_id) = temporary_file_id.as_deref() {
                    let _ = self.delete_private_file(file_id);
                }
                return Err(ProviderError::InvalidOutput(
                    "xAI omitted video request_id".into(),
                ));
            }
        };
        let ticks = value
            .pointer("/usage/cost_in_usd_ticks")
            .and_then(Value::as_u64);
        self.bind_async_real_request(reservation, request_id, ticks)?;
        let _ = self.record_usage(&value, false, false)?;
        self.usage.lock().unwrap().edited_videos += 1;
        if let Some(file_id) = temporary_file_id {
            self.temporary_files
                .lock()
                .unwrap()
                .insert(request_id.into(), file_id);
        }
        Ok(ProviderTicket {
            provider_id: self.id().into(),
            request_id: request_id.into(),
        })
    }

    fn poll(
        &self,
        ticket: &ProviderTicket,
        output_path: &Path,
    ) -> Result<ProviderPoll, ProviderError> {
        if ticket.provider_id != self.id() {
            return Err(ProviderError::InvalidOutput(
                "provider ticket does not belong to xAI".into(),
            ));
        }
        // Polling an already submitted durable request must remain possible
        // after the authorization's submission window expires. It consumes no
        // new request allowance; the persisted Provider request id identifies
        // the original reservation.
        if self.authorization.is_none() {
            self.ensure_real_accepted()?;
        }
        let url = format!("{}/videos/{}", self.base_url, ticket.request_id);
        let response =
            self.send_authenticated(|bearer| self.client.get(&url).bearer_auth(bearer))?;
        let value = parse_json_response(response, "xAI video poll")?;
        match value
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("pending")
        {
            "done" => {
                let media_url = value
                    .pointer("/video/url")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        ProviderError::InvalidOutput("xAI video result omitted url".into())
                    })?;
                let download_result = download_media(
                    &self.client,
                    media_url,
                    output_path,
                    base_url_is_loopback(&self.base_url),
                );
                let cleanup_result = self.cleanup_ticket_file(&ticket.request_id);
                download_result?;
                cleanup_result?;
                if self.authorization.is_some() {
                    let ticks = value
                        .pointer("/usage/cost_in_usd_ticks")
                        .and_then(Value::as_u64);
                    let finalized = self.finalize_async_real_request(&ticket.request_id, ticks)?;
                    let mut usage = self.usage.lock().unwrap();
                    usage.generated_videos += 1;
                    if finalized {
                        if let Some(ticks) = ticks {
                            usage.cost_in_usd_ticks =
                                Some(usage.cost_in_usd_ticks.unwrap_or(0) + ticks);
                        }
                    }
                } else {
                    let _ = self.record_usage(&value, false, true)?;
                }
                Ok(ProviderPoll::Succeeded(ProviderMedia {
                    path: output_path.to_path_buf(),
                    mime_type: "video/mp4".into(),
                    provider_asset_id: Some(ticket.request_id.clone()),
                    revised_prompt: None,
                }))
            }
            "failed" | "expired" => {
                if self.authorization.is_some() {
                    let ticks = value
                        .pointer("/usage/cost_in_usd_ticks")
                        .and_then(Value::as_u64);
                    let finalized = self.finalize_async_real_request(&ticket.request_id, ticks)?;
                    if finalized {
                        if let Some(ticks) = ticks {
                            let mut usage = self.usage.lock().unwrap();
                            usage.cost_in_usd_ticks =
                                Some(usage.cost_in_usd_ticks.unwrap_or(0) + ticks);
                        }
                    }
                }
                self.cleanup_ticket_file(&ticket.request_id)?;
                Ok(ProviderPoll::Failed {
                    code: value
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("failed")
                        .into(),
                    message: value
                        .get("error")
                        .and_then(Value::as_str)
                        .unwrap_or("xAI video generation failed")
                        .into(),
                })
            }
            _ => Ok(ProviderPoll::Pending {
                progress: value
                    .get("progress")
                    .and_then(Value::as_u64)
                    .map(|value| value.min(100) as u8),
            }),
        }
    }

    fn cancel(&self, ticket: &ProviderTicket) -> Result<(), ProviderError> {
        // xAI does not currently document a cancellation endpoint. Forge stops
        // polling immediately and records local cancellation without inventing one.
        // Private edit inputs are still deleted eagerly.
        if self.authorization.is_some() {
            let _ = self.finalize_async_real_request(&ticket.request_id, None)?;
        }
        self.cleanup_ticket_file(&ticket.request_id)
    }

    fn usage(&self) -> ProviderUsage {
        self.usage.lock().unwrap().clone()
    }
}

fn validate_base_url(base_url: &str, auth_kind: CredentialKind) -> Result<(), ProviderError> {
    let url = Url::parse(base_url)
        .map_err(|_| ProviderError::InvalidOutput("invalid xAI base URL".into()))?;
    if auth_kind == CredentialKind::OAuthDeviceCode {
        let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
        if url.scheme() != "https" || (host != "x.ai" && !host.ends_with(".x.ai")) {
            return Err(ProviderError::InvalidOutput(
                "OAuth credentials may only be sent to an HTTPS x.ai origin".into(),
            ));
        }
    }
    Ok(())
}

fn classify_response(response: Response) -> Result<Response, ProviderError> {
    match response.status() {
        StatusCode::UNAUTHORIZED => Err(ProviderError::AuthenticationRequired(
            "xAI rejected the credential after one refresh attempt".into(),
        )),
        StatusCode::FORBIDDEN => Err(ProviderError::Entitlement(
            "xAI rejected this subscription or API entitlement".into(),
        )),
        StatusCode::TOO_MANY_REQUESTS => Err(ProviderError::RateLimited(
            "xAI rate limit remained active after three bounded retries".into(),
        )),
        status if !status.is_success() => {
            let status = status.as_u16();
            let detail = safe_error_detail(response);
            Err(ProviderError::Request(match detail {
                Some(detail) => format!("xAI request failed with HTTP {status}: {detail}"),
                None => format!("xAI request failed with HTTP {status}"),
            }))
        }
        _ => Ok(response),
    }
}

fn safe_error_detail(mut response: Response) -> Option<String> {
    let mut bytes = Vec::new();
    response
        .by_ref()
        .take(8 * 1024)
        .read_to_end(&mut bytes)
        .ok()?;
    let value: Value = serde_json::from_slice(&bytes).ok()?;
    let code = value
        .pointer("/error/code")
        .or_else(|| value.get("code"))
        .and_then(Value::as_str)
        .filter(|value| safe_error_fragment(value));
    let message = value
        .pointer("/error/message")
        .or_else(|| value.get("message"))
        .and_then(Value::as_str)
        .filter(|value| safe_error_fragment(value));
    match (code, message) {
        (Some(code), Some(message)) => Some(format!("{code}: {message}")),
        (Some(code), None) => Some(code.into()),
        (None, Some(message)) => Some(message.into()),
        (None, None) => None,
    }
}

fn safe_error_fragment(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    value.len() <= 512
        && !lower.contains("authorization")
        && !lower.contains("bearer ")
        && !lower.contains("access_token")
        && !lower.contains("refresh_token")
        && !lower.contains("data:")
}

fn valid_file_id(value: &str) -> bool {
    value.starts_with("file_")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn files_api_unavailable(error: &ProviderError) -> bool {
    match error {
        ProviderError::Entitlement(_) | ProviderError::Unavailable(_) => true,
        ProviderError::Request(message) => {
            message.contains("HTTP 404")
                || message.contains("HTTP 405")
                || message.contains("HTTP 501")
        }
        _ => false,
    }
}

fn parse_json_response(response: Response, context: &str) -> Result<Value, ProviderError> {
    let content_length = response.content_length();
    if content_length.is_some_and(|length| length > JSON_RESPONSE_LIMIT as u64) {
        return Err(ProviderError::InvalidOutput(format!(
            "{context} exceeded the response limit"
        )));
    }
    let bytes = response
        .bytes()
        .map_err(|error| ProviderError::Request(error.to_string()))?;
    if bytes.len() > JSON_RESPONSE_LIMIT {
        return Err(ProviderError::InvalidOutput(format!(
            "{context} exceeded the response limit"
        )));
    }
    serde_json::from_slice(&bytes)
        .map_err(|error| ProviderError::InvalidOutput(format!("{context}: {error}")))
}

fn image_data_url(path: &Path) -> Result<String, ProviderError> {
    if !path.is_file() {
        return Err(ProviderError::InvalidOutput(format!(
            "reference image does not exist: {}",
            path.display()
        )));
    }
    let bytes = fs::read(path)?;
    if bytes.len() > 20 * 1024 * 1024 {
        return Err(ProviderError::InvalidOutput(
            "reference image exceeds 20 MiB".into(),
        ));
    }
    let mime = detect_image_mime(&bytes);
    if mime == "application/octet-stream" {
        return Err(ProviderError::InvalidOutput(
            "reference image must be PNG, JPEG, GIF, or WebP".into(),
        ));
    }
    Ok(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

/// Read a reference exactly once, verify the durable content fingerprint, and
/// encode those same bytes for the Provider request. This deliberately happens
/// before any reservation or HTTP connection is attempted by `edit_image`.
fn reference_image_data_url(reference: &ProviderImageReference) -> Result<String, ProviderError> {
    if !reference.path.is_file() {
        return Err(ProviderError::InvalidOutput(format!(
            "reference image does not exist: {}",
            reference.path.display()
        )));
    }
    let bytes = fs::read(&reference.path)?;
    let actual_sha256 = format!("{:x}", Sha256::digest(&bytes));
    if actual_sha256 != reference.sha256 {
        return Err(ProviderError::InvalidOutput(format!(
            "reference image SHA-256 changed before xAI edit: {}",
            reference.path.display()
        )));
    }
    if bytes.len() > 20 * 1024 * 1024 {
        return Err(ProviderError::InvalidOutput(
            "reference image exceeds 20 MiB".into(),
        ));
    }
    let mime = detect_image_mime(&bytes);
    if mime == "application/octet-stream" {
        return Err(ProviderError::InvalidOutput(
            "reference image must be PNG, JPEG, GIF, or WebP".into(),
        ));
    }
    Ok(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

fn detect_image_mime(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        "image/jpeg"
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        "image/gif"
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        "image/webp"
    } else {
        "application/octet-stream"
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), ProviderError> {
    let parent = path
        .parent()
        .ok_or_else(|| ProviderError::InvalidOutput("provider output path has no parent".into()))?;
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .map_err(|error| ProviderError::Io(error.error))?;
    Ok(())
}

fn download_media(
    _api_client: &Client,
    url: &str,
    output_path: &Path,
    allow_http_loopback: bool,
) -> Result<(), ProviderError> {
    let parsed = Url::parse(url)
        .map_err(|_| ProviderError::InvalidOutput("xAI returned an invalid media URL".into()))?;
    let loopback_http = allow_http_loopback
        && parsed.scheme() == "http"
        && parsed
            .host_str()
            .is_some_and(|host| host == "127.0.0.1" || host == "localhost" || host == "::1");
    if parsed.scheme() != "https" && !loopback_http {
        return Err(ProviderError::InvalidOutput(
            "xAI media URL must use HTTPS".into(),
        ));
    }
    let media_client = Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(300))
        .http1_only()
        .redirect(reqwest::redirect::Policy::custom(move |attempt| {
            if attempt.previous().len() >= 3 {
                return attempt.stop();
            }
            let url = attempt.url();
            let loopback_http = allow_http_loopback
                && url.scheme() == "http"
                && url.host_str().is_some_and(|host| {
                    host == "127.0.0.1" || host == "localhost" || host == "::1"
                });
            if url.scheme() == "https" || loopback_http {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .user_agent("Game-Sprite-Forge/0.1")
        .build()
        .map_err(|error| ProviderError::Request(error.to_string()))?;
    let mut response = media_client
        .get(url)
        .send()
        .map_err(|error| ProviderError::Request(error.to_string()))?;
    if !response.status().is_success() {
        return Err(ProviderError::Request(format!(
            "xAI media download failed with HTTP {}",
            response.status().as_u16()
        )));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MEDIA_RESPONSE_LIMIT)
    {
        return Err(ProviderError::InvalidOutput(
            "xAI media exceeds 512 MiB".into(),
        ));
    }
    let parent = output_path
        .parent()
        .ok_or_else(|| ProviderError::InvalidOutput("provider output path has no parent".into()))?;
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    let mut limited = response.by_ref().take(MEDIA_RESPONSE_LIMIT + 1);
    let copied = std::io::copy(&mut limited, &mut temporary)?;
    if copied > MEDIA_RESPONSE_LIMIT {
        return Err(ProviderError::InvalidOutput(
            "xAI media exceeds 512 MiB".into(),
        ));
    }
    temporary.as_file().sync_all()?;
    temporary
        .persist(output_path)
        .map_err(|error| ProviderError::Io(error.error))?;
    Ok(())
}

fn base_url_is_loopback(value: &str) -> bool {
    Url::parse(value).ok().is_some_and(|url| {
        url.host_str()
            .is_some_and(|host| host == "127.0.0.1" || host == "localhost" || host == "::1")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authorization::RequestLedgerStateV1;
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread::JoinHandle;

    struct TestCredential(&'static str, &'static str);

    impl CredentialProvider for TestCredential {
        fn kind(&self) -> CredentialKind {
            match self.0 {
                "oauth_device_code" => CredentialKind::OAuthDeviceCode,
                _ => CredentialKind::ApiKey,
            }
        }

        fn bearer(&self) -> Result<String, ProviderError> {
            Ok(self.1.into())
        }

        fn refresh(&self) -> Result<String, ProviderError> {
            Ok(self.1.into())
        }

        fn logout(&self) -> Result<(), ProviderError> {
            Ok(())
        }
    }

    #[test]
    fn oauth_bearers_cannot_be_redirected_to_other_hosts() {
        let credentials: Arc<dyn CredentialProvider> =
            Arc::new(TestCredential("oauth_device_code", "secret"));
        assert!(XaiProvider::with_base_url(credentials, "https://attacker.example/v1").is_err());
    }

    #[test]
    fn api_key_mode_allows_loopback_for_contract_tests() {
        let credentials: Arc<dyn CredentialProvider> =
            Arc::new(TestCredential("api_key", "secret"));
        assert!(XaiProvider::with_base_url(credentials, "http://127.0.0.1:1234/v1").is_ok());
    }

    struct CountingCredential {
        refreshes: AtomicUsize,
    }

    impl CredentialProvider for CountingCredential {
        fn kind(&self) -> CredentialKind {
            CredentialKind::ApiKey
        }

        fn bearer(&self) -> Result<String, ProviderError> {
            Ok("test-key".into())
        }

        fn refresh(&self) -> Result<String, ProviderError> {
            self.refreshes.fetch_add(1, Ordering::SeqCst);
            Ok("refreshed-key".into())
        }

        fn logout(&self) -> Result<(), ProviderError> {
            Ok(())
        }
    }

    struct FakeResponse {
        status: u16,
        content_type: &'static str,
        body: Vec<u8>,
    }

    fn fake_server<F>(expected_requests: usize, handler: F) -> (String, JoinHandle<()>)
    where
        F: Fn(usize, &str, &str) -> FakeResponse + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let thread_base = base.clone();
        let handle = std::thread::spawn(move || {
            for index in 0..expected_requests {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = Vec::new();
                let mut buffer = [0_u8; 16 * 1024];
                loop {
                    match stream.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(count) => {
                            request.extend_from_slice(&buffer[..count]);
                            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                let request = String::from_utf8_lossy(&request);
                let first_line = request.lines().next().unwrap_or_default();
                let response = handler(index, first_line, &thread_base);
                let reason = match response.status {
                    200 => "OK",
                    401 => "Unauthorized",
                    403 => "Forbidden",
                    429 => "Too Many Requests",
                    _ => "Error",
                };
                write!(
                    stream,
                    "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nRetry-After: 0\r\nConnection: close\r\n\r\n",
                    response.status,
                    reason,
                    response.content_type,
                    response.body.len()
                )
                .unwrap();
                stream.write_all(&response.body).unwrap();
            }
        });
        (base, handle)
    }

    fn fake_json_request_server<F>(handler: F) -> (String, JoinHandle<()>)
    where
        F: Fn(&str, Value) -> FakeResponse + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = Vec::new();
            let mut header_end = None;
            let mut expected_length = None;
            loop {
                let mut buffer = [0_u8; 16 * 1024];
                let count = stream.read(&mut buffer).unwrap_or_default();
                if count == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..count]);
                if header_end.is_none() {
                    header_end = request
                        .windows(4)
                        .position(|window| window == b"\r\n\r\n")
                        .map(|index| index + 4);
                    if let Some(end) = header_end {
                        let headers = String::from_utf8_lossy(&request[..end]);
                        expected_length = headers.lines().find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        });
                    }
                }
                if header_end
                    .zip(expected_length)
                    .is_some_and(|(end, length)| request.len() >= end + length)
                {
                    break;
                }
            }
            let end = header_end.expect("request headers");
            let length = expected_length.expect("content length");
            let headers = String::from_utf8_lossy(&request[..end]);
            let first_line = headers.lines().next().unwrap_or_default();
            let body: Value = serde_json::from_slice(&request[end..end + length]).unwrap();
            let response = handler(first_line, body);
            write!(
                stream,
                "HTTP/1.1 {} OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                response.status,
                response.content_type,
                response.body.len()
            )
            .unwrap();
            stream.write_all(&response.body).unwrap();
        });
        (base, handle)
    }

    #[test]
    fn image_edit_preserves_reference_order_and_first_edit_target_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let edit_target = temp.path().join("failed-sheet.png");
        let direction = temp.path().join("direction.png");
        let pose = temp.path().join("pose.png");
        for (path, color) in [
            (&edit_target, [255, 0, 0, 255]),
            (&direction, [0, 255, 0, 255]),
            (&pose, [0, 0, 255, 255]),
        ] {
            image::RgbaImage::from_pixel(2, 2, image::Rgba(color))
                .save(path)
                .unwrap();
        }
        let expected = [&edit_target, &direction, &pose]
            .into_iter()
            .map(|path| image_data_url(path).unwrap())
            .collect::<Vec<_>>();
        let response_png = one_pixel_png();
        let encoded = base64::engine::general_purpose::STANDARD.encode(response_png);
        let (base, server) = fake_json_request_server(move |first_line, body| {
            assert!(first_line.contains("POST /v1/images/edits"));
            let images = body["images"].as_array().expect("ordered image references");
            assert_eq!(images.len(), 3);
            for (index, expected_url) in expected.iter().enumerate() {
                assert_eq!(images[index]["url"], *expected_url);
            }
            FakeResponse {
                status: 200,
                content_type: "application/json",
                body: serde_json::to_vec(&json!({
                    "data": [{"b64_json": encoded, "mime_type": "image/png"}]
                }))
                .unwrap(),
            }
        });
        let credentials: Arc<dyn CredentialProvider> =
            Arc::new(TestCredential("api_key", "secret"));
        let provider = XaiProvider::with_base_url(credentials, format!("{base}/v1")).unwrap();
        let output = temp.path().join("output.png");
        provider
            .edit_image(
                &EditImageRequest {
                    prompt: "diagnostic action-grid edit".into(),
                    model: None,
                    references: vec![
                        forge_core::provider::ProviderImageReference::from_path(
                            forge_core::provider::ReferenceRole::EditTarget,
                            &edit_target,
                        )
                        .unwrap(),
                        forge_core::provider::ProviderImageReference::from_path(
                            forge_core::provider::ReferenceRole::DirectionAnchor,
                            &direction,
                        )
                        .unwrap(),
                        forge_core::provider::ProviderImageReference::from_path(
                            forge_core::provider::ReferenceRole::PoseStructure,
                            &pose,
                        )
                        .unwrap(),
                    ],
                    aspect_ratio: "1:1".into(),
                    resolution: "1k".into(),
                    authorization_target: None,
                },
                &output,
            )
            .unwrap();
        assert!(output.is_file());
        server.join().unwrap();
    }

    #[test]
    fn image_edit_preserves_two_reference_fresh_gait_order() {
        let temp = tempfile::tempdir().unwrap();
        let direction = temp.path().join("direction.png");
        let pose = temp.path().join("pose.png");
        for (path, color) in [
            (&direction, [0, 255, 0, 255]),
            (&pose, [232, 232, 232, 232]),
        ] {
            image::RgbaImage::from_pixel(2, 2, image::Rgba(color))
                .save(path)
                .unwrap();
        }
        let expected = [&direction, &pose]
            .into_iter()
            .map(|path| image_data_url(path).unwrap())
            .collect::<Vec<_>>();
        let encoded = base64::engine::general_purpose::STANDARD.encode(one_pixel_png());
        let (base, server) = fake_json_request_server(move |first_line, body| {
            assert!(first_line.contains("POST /v1/images/edits"));
            assert!(body.get("image").is_none());
            let images = body["images"].as_array().expect("ordered image references");
            assert_eq!(images.len(), 2);
            for (index, expected_url) in expected.iter().enumerate() {
                assert_eq!(images[index]["url"], *expected_url);
            }
            assert!(body["prompt"]
                .as_str()
                .unwrap()
                .contains("Reference 1 is the immutable DirectionAnchor"));
            assert!(body["prompt"]
                .as_str()
                .unwrap()
                .contains("Reference 2 is a transparent grayscale PoseStructure"));
            FakeResponse {
                status: 200,
                content_type: "application/json",
                body: serde_json::to_vec(&json!({
                    "data": [{"b64_json": encoded, "mime_type": "image/png"}]
                }))
                .unwrap(),
            }
        });
        let credentials: Arc<dyn CredentialProvider> =
            Arc::new(TestCredential("api_key", "secret"));
        let provider = XaiProvider::with_base_url(credentials, format!("{base}/v1")).unwrap();
        let output = temp.path().join("output.png");
        provider
            .edit_image(
                &EditImageRequest {
                    prompt: "Reference 1 is the immutable DirectionAnchor. Reference 2 is a transparent grayscale PoseStructure.".into(),
                    model: None,
                    references: vec![
                        forge_core::provider::ProviderImageReference::from_path(
                            forge_core::provider::ReferenceRole::DirectionAnchor,
                            &direction,
                        )
                        .unwrap(),
                        forge_core::provider::ProviderImageReference::from_path(
                            forge_core::provider::ReferenceRole::PoseStructure,
                            &pose,
                        )
                        .unwrap(),
                    ],
                    aspect_ratio: "1:1".into(),
                    resolution: "1k".into(),
                    authorization_target: Some("walk_down:frame:2".into()),
                },
                &output,
            )
            .unwrap();
        assert!(output.is_file());
        server.join().unwrap();
    }

    #[test]
    fn image2_candidate_is_explicit_in_the_edit_payload_without_replacing_the_default() {
        let temp = tempfile::tempdir().unwrap();
        let reference = temp.path().join("approved-direction-grid.png");
        image::RgbaImage::from_pixel(2, 2, image::Rgba([32, 96, 48, 255]))
            .save(&reference)
            .unwrap();
        let encoded = base64::engine::general_purpose::STANDARD.encode(one_pixel_png());
        let (base, server) = fake_json_request_server(move |first_line, body| {
            assert!(first_line.contains("POST /v1/images/edits"));
            assert_eq!(body["model"], IMAGE_2_CANDIDATE_MODEL);
            assert!(body.get("image").is_some());
            assert!(body.get("images").is_none());
            FakeResponse {
                status: 200,
                content_type: "application/json",
                body: serde_json::to_vec(&json!({
                    "data": [{"b64_json": encoded, "mime_type": "image/png"}]
                }))
                .unwrap(),
            }
        });
        let provider = XaiProvider::with_base_url(
            Arc::new(TestCredential("api_key", "secret")),
            format!("{base}/v1"),
        )
        .unwrap();
        assert_eq!(
            provider.resolved_image_model(None).as_deref(),
            Some(DEFAULT_IMAGE_MODEL)
        );
        assert_eq!(
            provider
                .resolved_image_model(Some(IMAGE_2_CANDIDATE_MODEL))
                .as_deref(),
            Some(IMAGE_2_CANDIDATE_MODEL)
        );
        let output = temp.path().join("candidate.png");
        provider
            .edit_image(
                &EditImageRequest {
                    prompt: "bounded DirectionGrid comparison".into(),
                    model: Some(IMAGE_2_CANDIDATE_MODEL.into()),
                    references: vec![ProviderImageReference::from_path(
                        forge_core::provider::ReferenceRole::EditTarget,
                        &reference,
                    )
                    .unwrap()],
                    aspect_ratio: "1:1".into(),
                    resolution: "1k".into(),
                    authorization_target: Some("direction_grid".into()),
                },
                &output,
            )
            .unwrap();
        assert!(output.is_file());
        server.join().unwrap();
    }

    #[test]
    fn image_edit_rejects_changed_reference_hash_before_reservation_or_connection() {
        let temp = tempfile::tempdir().unwrap();
        let reference_path = temp.path().join("reference.png");
        image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]))
            .save(&reference_path)
            .unwrap();
        let mut reference = ProviderImageReference::from_path(
            forge_core::provider::ReferenceRole::EditTarget,
            &reference_path,
        )
        .unwrap();
        reference.sha256 = "0".repeat(64);

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let provider = XaiProvider::with_base_url(
            Arc::new(TestCredential("api_key", "secret")),
            format!("http://{}/v1", listener.local_addr().unwrap()),
        )
        .unwrap();
        let error = provider
            .edit_image(
                &EditImageRequest {
                    prompt: "must not submit".into(),
                    model: None,
                    references: vec![reference],
                    aspect_ratio: "1:1".into(),
                    resolution: "1k".into(),
                    authorization_target: None,
                },
                &temp.path().join("output.png"),
            )
            .unwrap_err();

        assert!(matches!(error, ProviderError::InvalidOutput(_)));
        assert!(error.to_string().contains("SHA-256 changed"));
        assert!(
            matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
        );
    }

    fn one_pixel_png() -> Vec<u8> {
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            1,
            1,
            image::Rgba([255, 0, 0, 255]),
        ))
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
        bytes
    }

    fn one_pixel_jpeg() -> Vec<u8> {
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(1, 1, image::Rgb([255, 0, 0])))
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Jpeg,
            )
            .unwrap();
        bytes
    }

    #[test]
    fn image_request_refreshes_once_and_materializes_url_fallback() {
        let jpeg = one_pixel_jpeg();
        let (base, server) = fake_server(3, move |index, first_line, base| match index {
            0 => FakeResponse {
                status: 401,
                content_type: "application/json",
                body: b"{}".to_vec(),
            },
            1 => {
                assert!(first_line.contains("POST /v1/images/generations"));
                FakeResponse {
                    status: 200,
                    content_type: "application/json",
                    body: serde_json::to_vec(&json!({
                        "data": [{"url": format!("{base}/media/image.jpeg"), "mime_type": "image/jpeg"}]
                    }))
                    .unwrap(),
                }
            }
            _ => FakeResponse {
                status: 200,
                content_type: "image/jpeg",
                body: jpeg.clone(),
            },
        });
        let credential = Arc::new(CountingCredential {
            refreshes: AtomicUsize::new(0),
        });
        let provider =
            XaiProvider::with_base_url(credential.clone(), format!("{base}/v1")).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("image.png");
        let media = provider
            .generate_image(
                &GenerateImageRequest {
                    prompt: "test".into(),
                    model: None,
                    aspect_ratio: "1:1".into(),
                    resolution: "1k".into(),
                    authorization_target: None,
                },
                &output,
            )
            .unwrap();
        assert_eq!(credential.refreshes.load(Ordering::SeqCst), 1);
        assert_eq!(media.path, output);
        assert_eq!(
            detect_image_mime(&fs::read(media.path).unwrap()),
            "image/png"
        );
        server.join().unwrap();
    }

    fn exact_one_operation_provider(
        authorization_root: &Path,
        authorization_id: &str,
        base: impl Into<String>,
        credentials: Arc<dyn CredentialProvider>,
    ) -> XaiProvider {
        let mut manifest = durable_test_manifest(authorization_id, 1);
        manifest.max_total_provider_operations = Some(1);
        crate::authorization::AuthorizationStore::create(authorization_root, &manifest).unwrap();
        let provider = XaiProvider::with_base_url_and_authorization(
            credentials,
            base,
            "default",
            ProviderAuthorizationConfig::new(authorization_root, authorization_id),
        )
        .unwrap();
        provider
            .set_authorization_execution_context("test-job", "source-job", "recipe", "input")
            .unwrap();
        provider
    }

    #[test]
    fn exact_one_operation_authorization_does_not_retry_unauthorized_image_request() {
        let (base, server) = fake_server(1, |_, first_line, _| {
            assert!(first_line.contains("POST /v1/images/generations"));
            FakeResponse {
                status: 401,
                content_type: "application/json",
                body: b"{}".to_vec(),
            }
        });
        let authorization_root = tempfile::tempdir().unwrap();
        let credential = Arc::new(CountingCredential {
            refreshes: AtomicUsize::new(0),
        });
        let provider = exact_one_operation_provider(
            authorization_root.path(),
            "auth-xai-exact-401",
            format!("{base}/v1"),
            credential.clone(),
        );
        let error = provider
            .generate_image(
                &GenerateImageRequest {
                    prompt: "test".into(),
                    model: Some("test-image".into()),
                    aspect_ratio: "1:1".into(),
                    resolution: "1k".into(),
                    authorization_target: Some("happy".into()),
                },
                &authorization_root.path().join("output.png"),
            )
            .unwrap_err();
        assert!(matches!(error, ProviderError::AuthenticationRequired(_)));
        assert_eq!(credential.refreshes.load(Ordering::SeqCst), 0);
        server.join().unwrap();
    }

    #[test]
    fn manifest_drift_during_unauthorized_response_never_retries() {
        let authorization_root = tempfile::tempdir().unwrap();
        let authorization_id = "auth-xai-drift-401";
        let mut grant = durable_test_manifest(authorization_id, 1);
        grant.max_total_provider_operations = Some(1);
        let store =
            crate::authorization::AuthorizationStore::create(authorization_root.path(), &grant)
                .unwrap();
        let manifest_path = authorization_root
            .path()
            .join(authorization_id)
            .join("authorization-manifest.json");
        let (base, server) = fake_server(1, move |_, first_line, _| {
            assert!(first_line.contains("POST /v1/images/generations"));
            let mut replacement = grant.clone();
            replacement.max_total_provider_operations = Some(2);
            fs::write(
                &manifest_path,
                serde_json::to_vec_pretty(&replacement).unwrap(),
            )
            .unwrap();
            FakeResponse {
                status: 401,
                content_type: "application/json",
                body: b"{}".to_vec(),
            }
        });
        let credential = Arc::new(CountingCredential {
            refreshes: AtomicUsize::new(0),
        });
        let provider = XaiProvider::with_base_url_and_authorization(
            credential.clone(),
            format!("{base}/v1"),
            "default",
            ProviderAuthorizationConfig::new(authorization_root.path(), authorization_id),
        )
        .unwrap();
        provider
            .set_authorization_execution_context("test-job", "source-job", "recipe", "input")
            .unwrap();

        let error = provider
            .generate_image(
                &GenerateImageRequest {
                    prompt: "test".into(),
                    model: Some("test-image".into()),
                    aspect_ratio: "1:1".into(),
                    resolution: "1k".into(),
                    authorization_target: Some("happy".into()),
                },
                &authorization_root.path().join("output.png"),
            )
            .unwrap_err();

        assert!(error.to_string().contains("manifest changed"));
        assert_eq!(credential.refreshes.load(Ordering::SeqCst), 0);
        server.join().unwrap();
        let ledger = store.ledger().unwrap();
        assert_eq!(ledger.requests.len(), 1);
        assert_eq!(ledger.requests[0].state, RequestLedgerStateV1::Submitted);
        assert_eq!(ledger.consumed_request_count(), 1);
        assert_eq!(ledger.consumed_cost_ticks(), 100);
    }

    #[test]
    fn exact_one_operation_authorization_does_not_retry_rate_limited_image_request() {
        let (base, server) = fake_server(1, |_, first_line, _| {
            assert!(first_line.contains("POST /v1/images/generations"));
            FakeResponse {
                status: 429,
                content_type: "application/json",
                body: b"{}".to_vec(),
            }
        });
        let authorization_root = tempfile::tempdir().unwrap();
        let provider = exact_one_operation_provider(
            authorization_root.path(),
            "auth-xai-exact-429",
            format!("{base}/v1"),
            Arc::new(TestCredential("api_key", "secret")),
        );
        let error = provider
            .generate_image(
                &GenerateImageRequest {
                    prompt: "test".into(),
                    model: Some("test-image".into()),
                    aspect_ratio: "1:1".into(),
                    resolution: "1k".into(),
                    authorization_target: Some("happy".into()),
                },
                &authorization_root.path().join("output.png"),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ProviderError::RateLimited(ref message)
                if message == "xAI rate limited the exact-one-operation request; no retry was attempted"
        ));
        server.join().unwrap();
    }

    #[test]
    fn exact_one_operation_authorization_rejects_image_url_fallback_without_get() {
        let (base, server) = fake_server(1, |_, first_line, base| {
            assert!(first_line.contains("POST /v1/images/generations"));
            FakeResponse {
                status: 200,
                content_type: "application/json",
                body: serde_json::to_vec(&json!({
                    "data": [{"url": format!("{base}/media/image.png")}],
                    "usage": {"cost_in_usd_ticks": 40}
                }))
                .unwrap(),
            }
        });
        let authorization_root = tempfile::tempdir().unwrap();
        let provider = exact_one_operation_provider(
            authorization_root.path(),
            "auth-xai-exact-url",
            format!("{base}/v1"),
            Arc::new(TestCredential("api_key", "secret")),
        );
        let error = provider
            .generate_image(
                &GenerateImageRequest {
                    prompt: "test".into(),
                    model: Some("test-image".into()),
                    aspect_ratio: "1:1".into(),
                    resolution: "1k".into(),
                    authorization_target: Some("happy".into()),
                },
                &authorization_root.path().join("output.png"),
            )
            .unwrap_err();
        assert!(matches!(error, ProviderError::InvalidOutput(_)));
        assert!(error.to_string().contains("require b64_json"));
        server.join().unwrap();
    }

    #[test]
    fn video_submit_and_async_poll_follow_the_contract() {
        let media_bytes = b"fixture-video".to_vec();
        let (base, server) = fake_server(4, move |index, first_line, base| match index {
            0 => {
                assert!(first_line.contains("POST /v1/videos/generations"));
                FakeResponse {
                    status: 200,
                    content_type: "application/json",
                    body: br#"{"request_id":"video-1"}"#.to_vec(),
                }
            }
            1 => FakeResponse {
                status: 200,
                content_type: "application/json",
                body: br#"{"status":"pending","progress":25}"#.to_vec(),
            },
            2 => FakeResponse {
                status: 200,
                content_type: "application/json",
                body: serde_json::to_vec(&json!({
                    "status": "done",
                    "video": {"url": format!("{base}/media/video.mp4")}
                }))
                .unwrap(),
            },
            _ => FakeResponse {
                status: 200,
                content_type: "video/mp4",
                body: media_bytes.clone(),
            },
        });
        let credentials: Arc<dyn CredentialProvider> =
            Arc::new(TestCredential("api_key", "secret"));
        let provider = XaiProvider::with_base_url(credentials, format!("{base}/v1")).unwrap();
        let ticket = provider
            .generate_video(&GenerateVideoRequest {
                prompt: "walk".into(),
                model: None,
                mode: VideoGenerationMode::Text,
                duration_seconds: 4,
                aspect_ratio: "1:1".into(),
                resolution: "720p".into(),
                authorization_target: None,
            })
            .unwrap();
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("video.mp4");
        assert!(matches!(
            provider.poll(&ticket, &output).unwrap(),
            ProviderPoll::Pending { progress: Some(25) }
        ));
        assert!(matches!(
            provider.poll(&ticket, &output).unwrap(),
            ProviderPoll::Succeeded(_)
        ));
        assert_eq!(fs::read(output).unwrap(), b"fixture-video");
        server.join().unwrap();
    }

    #[test]
    fn video_edit_uses_the_edit_endpoint_and_hash_checked_input() {
        let (base, server) = fake_server(1, |_, first_line, _| {
            assert!(first_line.contains("POST /v1/videos/edits"));
            FakeResponse {
                status: 200,
                content_type: "application/json",
                body: br#"{"request_id":"video-edit-1"}"#.to_vec(),
            }
        });
        let credentials: Arc<dyn CredentialProvider> =
            Arc::new(TestCredential("api_key", "secret"));
        let provider = XaiProvider::with_base_url(credentials, format!("{base}/v1")).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("source.mp4");
        fs::write(&input, b"fixture-video").unwrap();
        let sha256 = format!("{:x}", Sha256::digest(b"fixture-video"));

        let ticket = provider
            .edit_video(&EditVideoRequest {
                prompt: "repair the seamless loop".into(),
                model: None,
                video: ProviderInputRef {
                    path: input,
                    sha256,
                    provider_asset_id: Some("file_source-video".into()),
                },
                authorization_target: None,
            })
            .unwrap();

        assert_eq!(ticket.request_id, "video-edit-1");
        assert_eq!(provider.usage().edited_videos, 1);
        server.join().unwrap();
    }

    #[test]
    fn video_edit_preserves_only_safe_structured_error_details() {
        let (base, server) = fake_server(1, |_, _, _| {
            FakeResponse {
            status: 400,
            content_type: "application/json",
            body: br#"{"error":{"code":"invalid_model","message":"video edits require grok-imagine-video"}}"#
                .to_vec(),
        }
        });
        let credentials: Arc<dyn CredentialProvider> =
            Arc::new(TestCredential("api_key", "secret"));
        let provider = XaiProvider::with_base_url(credentials, format!("{base}/v1")).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("source.mp4");
        fs::write(&input, b"fixture-video").unwrap();
        let error = provider
            .edit_video(&EditVideoRequest {
                prompt: "repair".into(),
                model: None,
                video: ProviderInputRef {
                    path: input,
                    sha256: format!("{:x}", Sha256::digest(b"fixture-video")),
                    provider_asset_id: Some("file_source-video".into()),
                },
                authorization_target: None,
            })
            .unwrap_err();
        assert!(error.to_string().contains("invalid_model"));
        assert!(!error.to_string().contains("Bearer"));
        server.join().unwrap();
    }

    #[test]
    fn video_edit_uploads_and_deletes_a_private_temporary_file() {
        let (base, server) = fake_server(5, |index, first_line, base| match index {
            0 => {
                assert!(first_line.contains("POST /v1/files"));
                FakeResponse {
                    status: 200,
                    content_type: "application/json",
                    body: br#"{"id":"file_temporary-video"}"#.to_vec(),
                }
            }
            1 => {
                assert!(first_line.contains("POST /v1/videos/edits"));
                FakeResponse {
                    status: 200,
                    content_type: "application/json",
                    body: br#"{"request_id":"video-edit-private"}"#.to_vec(),
                }
            }
            2 => FakeResponse {
                status: 200,
                content_type: "application/json",
                body: serde_json::to_vec(&json!({
                    "status": "done",
                    "video": {"url": format!("{base}/media/private.mp4")}
                }))
                .unwrap(),
            },
            3 => FakeResponse {
                status: 200,
                content_type: "video/mp4",
                body: b"fixture-private-video".to_vec(),
            },
            _ => {
                assert!(first_line.contains("DELETE /v1/files/file_temporary-video"));
                FakeResponse {
                    status: 200,
                    content_type: "application/json",
                    body: br#"{"deleted":true}"#.to_vec(),
                }
            }
        });
        let credentials: Arc<dyn CredentialProvider> =
            Arc::new(TestCredential("api_key", "secret"));
        let provider = XaiProvider::with_base_url(credentials, format!("{base}/v1")).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("source.mp4");
        fs::write(&input, b"fixture-video").unwrap();
        let ticket = provider
            .edit_video(&EditVideoRequest {
                prompt: "repair the loop".into(),
                model: None,
                video: ProviderInputRef {
                    path: input,
                    sha256: format!("{:x}", Sha256::digest(b"fixture-video")),
                    provider_asset_id: None,
                },
                authorization_target: None,
            })
            .unwrap();
        let output = temp.path().join("edited.mp4");
        assert!(matches!(
            provider.poll(&ticket, &output).unwrap(),
            ProviderPoll::Succeeded(_)
        ));
        assert_eq!(fs::read(output).unwrap(), b"fixture-private-video");
        assert_eq!(provider.usage().private_file_uploads, 1);
        assert!(provider.temporary_files.lock().unwrap().is_empty());
        server.join().unwrap();
    }

    #[test]
    fn classifies_forbidden_and_rejects_malformed_image_media() {
        let (base, server) = fake_server(1, |_, _, _| FakeResponse {
            status: 403,
            content_type: "application/json",
            body: b"{}".to_vec(),
        });
        let credentials: Arc<dyn CredentialProvider> =
            Arc::new(TestCredential("api_key", "secret"));
        let provider = XaiProvider::with_base_url(credentials, format!("{base}/v1")).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let error = provider
            .generate_image(
                &GenerateImageRequest {
                    prompt: "test".into(),
                    model: None,
                    aspect_ratio: "1:1".into(),
                    resolution: "1k".into(),
                    authorization_target: None,
                },
                &temp.path().join("image.png"),
            )
            .unwrap_err();
        assert!(matches!(error, ProviderError::Entitlement(_)));
        server.join().unwrap();

        let encoded = base64::engine::general_purpose::STANDARD.encode(b"not-an-image");
        let (base, server) = fake_server(1, move |_, _, _| FakeResponse {
            status: 200,
            content_type: "application/json",
            body: serde_json::to_vec(&json!({"data": [{"b64_json": encoded}]})).unwrap(),
        });
        let credentials: Arc<dyn CredentialProvider> =
            Arc::new(TestCredential("api_key", "secret"));
        let provider = XaiProvider::with_base_url(credentials, format!("{base}/v1")).unwrap();
        let error = provider
            .generate_image(
                &GenerateImageRequest {
                    prompt: "test".into(),
                    model: None,
                    aspect_ratio: "1:1".into(),
                    resolution: "1k".into(),
                    authorization_target: None,
                },
                &temp.path().join("invalid.png"),
            )
            .unwrap_err();
        assert!(matches!(error, ProviderError::InvalidOutput(_)));
        server.join().unwrap();
    }

    #[test]
    fn retries_rate_limit_with_a_bounded_policy() {
        let encoded = base64::engine::general_purpose::STANDARD.encode(one_pixel_png());
        let (base, server) = fake_server(2, move |index, _, _| {
            if index == 0 {
                FakeResponse {
                    status: 429,
                    content_type: "application/json",
                    body: b"{}".to_vec(),
                }
            } else {
                FakeResponse {
                    status: 200,
                    content_type: "application/json",
                    body: serde_json::to_vec(&json!({
                        "data": [{"b64_json": encoded, "mime_type": "image/png"}]
                    }))
                    .unwrap(),
                }
            }
        });
        let credentials: Arc<dyn CredentialProvider> =
            Arc::new(TestCredential("api_key", "secret"));
        let provider = XaiProvider::with_base_url(credentials, format!("{base}/v1")).unwrap();
        let temp = tempfile::tempdir().unwrap();
        provider
            .generate_image(
                &GenerateImageRequest {
                    prompt: "test".into(),
                    model: None,
                    aspect_ratio: "1:1".into(),
                    resolution: "1k".into(),
                    authorization_target: None,
                },
                &temp.path().join("image.png"),
            )
            .unwrap();
        server.join().unwrap();
    }

    #[test]
    fn real_provider_budget_requires_acceptance_and_enforces_request_and_cost_limits() {
        temp_env::with_vars(
            [
                (REAL_PROVIDER_ACCEPT_ENV, None::<&str>),
                (REAL_PROVIDER_MAX_REQUESTS_ENV, None::<&str>),
                (REAL_PROVIDER_MAX_COST_TICKS_ENV, None::<&str>),
            ],
            || {
                let guard = RealProviderBudgetGuard::default();
                assert!(matches!(
                    guard.reserve_request(),
                    Err(ProviderError::RealProviderNotAccepted(_))
                ));
            },
        );

        temp_env::with_vars(
            [
                (REAL_PROVIDER_ACCEPT_ENV, Some("1")),
                (REAL_PROVIDER_MAX_REQUESTS_ENV, Some("2")),
                (REAL_PROVIDER_MAX_COST_TICKS_ENV, Some("100")),
            ],
            || {
                let guard = RealProviderBudgetGuard::default();
                guard.reserve_request().unwrap();
                guard.record_cost(Some(40)).unwrap();
                guard.reserve_request().unwrap();
                assert!(matches!(
                    guard.reserve_request(),
                    Err(ProviderError::RequestBudgetExceeded(_))
                ));
                assert!(matches!(
                    guard.record_cost(Some(61)),
                    Err(ProviderError::CostBudgetExceeded(_))
                ));
            },
        );
    }

    fn durable_test_manifest(
        authorization_id: &str,
        max_requests: u32,
    ) -> crate::authorization::AuthorizationManifestV1 {
        let now = chrono::Utc::now();
        crate::authorization::AuthorizationManifestV1 {
            schema_version: crate::authorization::AUTHORIZATION_MANIFEST_SCHEMA_VERSION.into(),
            authorization_id: authorization_id.into(),
            provider_id: "xai".into(),
            profile_id: "default".into(),
            allowed_models: vec!["test-image".into()],
            created_at: now,
            expires_at: now + chrono::Duration::minutes(10),
            max_total_requests: max_requests,
            max_total_provider_operations: None,
            max_total_cost_ticks: u64::from(max_requests) * 100,
            allowed_targets: vec![crate::authorization::AuthorizedTargetV1 {
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
    fn durable_authorization_is_shared_across_xai_instances() {
        let encoded = base64::engine::general_purpose::STANDARD.encode(one_pixel_png());
        let (base, server) = fake_server(2, move |_, first_line, _| {
            assert!(first_line.contains("POST /v1/images/generations"));
            FakeResponse {
                status: 200,
                content_type: "application/json",
                body: serde_json::to_vec(&json!({
                    "data": [{"b64_json": encoded}],
                    "usage": {"cost_in_usd_ticks": 40}
                }))
                .unwrap(),
            }
        });
        let authorization_root = tempfile::tempdir().unwrap();
        crate::authorization::AuthorizationStore::create(
            authorization_root.path(),
            &durable_test_manifest("auth-xai-shared", 2),
        )
        .unwrap();
        let provider = |base: &str| {
            XaiProvider::with_base_url_and_authorization(
                Arc::new(TestCredential("api_key", "secret")),
                format!("{base}/v1"),
                "default",
                ProviderAuthorizationConfig::new(authorization_root.path(), "auth-xai-shared"),
            )
            .unwrap()
        };
        let first = provider(&base);
        let second = provider(&base);
        let third = provider(&base);
        first
            .set_authorization_job_context("parent-job", "source-job")
            .unwrap();
        second
            .set_authorization_job_context("child-job", "source-job")
            .unwrap();
        let output = authorization_root.path().join("first.png");
        first
            .generate_image(
                &GenerateImageRequest {
                    prompt: "happy portrait".into(),
                    model: Some("test-image".into()),
                    aspect_ratio: "1:1".into(),
                    resolution: "1k".into(),
                    authorization_target: Some("happy".into()),
                },
                &output,
            )
            .unwrap();
        second
            .generate_image(
                &GenerateImageRequest {
                    prompt: "another happy portrait".into(),
                    model: Some("test-image".into()),
                    aspect_ratio: "1:1".into(),
                    resolution: "1k".into(),
                    authorization_target: Some("happy".into()),
                },
                &authorization_root.path().join("second.png"),
            )
            .unwrap();
        let error = third
            .generate_image(
                &GenerateImageRequest {
                    prompt: "third happy portrait".into(),
                    model: Some("test-image".into()),
                    aspect_ratio: "1:1".into(),
                    resolution: "1k".into(),
                    authorization_target: Some("happy".into()),
                },
                &authorization_root.path().join("third.png"),
            )
            .unwrap_err();
        assert!(matches!(error, ProviderError::RequestBudgetExceeded(_)));
        let ledger = crate::authorization::AuthorizationStore::open(
            authorization_root.path(),
            "auth-xai-shared",
        )
        .unwrap()
        .ledger()
        .unwrap();
        assert_eq!(ledger.consumed_request_count(), 2);
        assert_eq!(ledger.consumed_cost_ticks(), 80);
        assert_eq!(ledger.requests[0].job_id.as_deref(), Some("parent-job"));
        assert_eq!(ledger.requests[1].job_id.as_deref(), Some("child-job"));
        server.join().unwrap();
    }

    #[test]
    fn durable_authorization_context_mismatch_prevents_ledger_and_http() {
        let calls = Arc::new(AtomicUsize::new(0));
        let server_calls = Arc::clone(&calls);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_millis(250);
            while std::time::Instant::now() < deadline {
                if listener.accept().is_ok() {
                    server_calls.fetch_add(1, Ordering::SeqCst);
                }
                std::thread::yield_now();
            }
        });
        let authorization_root = tempfile::tempdir().unwrap();
        let mut manifest = durable_test_manifest("auth-xai-context-mismatch", 1);
        manifest.source_lineage_root_job_id = Some("source-job".into());
        manifest.recipe_hash = Some("recipe".into());
        manifest.input_fingerprint = Some("input".into());
        crate::authorization::AuthorizationStore::create(authorization_root.path(), &manifest)
            .unwrap();
        let provider = XaiProvider::with_base_url_and_authorization(
            Arc::new(TestCredential("api_key", "secret")),
            format!("{base}/v1"),
            "default",
            ProviderAuthorizationConfig::new(
                authorization_root.path(),
                "auth-xai-context-mismatch",
            ),
        )
        .unwrap();
        provider
            .set_authorization_execution_context("child-job", "source-job", "wrong-recipe", "input")
            .unwrap();

        let error = provider
            .generate_image(
                &GenerateImageRequest {
                    prompt: "happy portrait".into(),
                    model: Some("test-image".into()),
                    aspect_ratio: "1:1".into(),
                    resolution: "1k".into(),
                    authorization_target: Some("happy".into()),
                },
                &authorization_root.path().join("mismatch.png"),
            )
            .unwrap_err();
        assert!(matches!(error, ProviderError::RealProviderNotAccepted(_)));
        assert!(crate::authorization::AuthorizationStore::open(
            authorization_root.path(),
            "auth-xai-context-mismatch",
        )
        .unwrap()
        .ledger()
        .unwrap()
        .requests
        .is_empty());
        server.join().unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn durable_authorization_exact_context_allows_request() {
        let encoded = base64::engine::general_purpose::STANDARD.encode(one_pixel_png());
        let (base, server) = fake_server(1, move |_, first_line, _| {
            assert!(first_line.contains("POST /v1/images/generations"));
            FakeResponse {
                status: 200,
                content_type: "application/json",
                body: serde_json::to_vec(&json!({
                    "data": [{"b64_json": encoded}],
                    "usage": {"cost_in_usd_ticks": 40}
                }))
                .unwrap(),
            }
        });
        let authorization_root = tempfile::tempdir().unwrap();
        let mut manifest = durable_test_manifest("auth-xai-context-exact", 1);
        manifest.source_lineage_root_job_id = Some("source-job".into());
        manifest.recipe_hash = Some("recipe".into());
        manifest.input_fingerprint = Some("input".into());
        crate::authorization::AuthorizationStore::create(authorization_root.path(), &manifest)
            .unwrap();
        let provider = XaiProvider::with_base_url_and_authorization(
            Arc::new(TestCredential("api_key", "secret")),
            format!("{base}/v1"),
            "default",
            ProviderAuthorizationConfig::new(authorization_root.path(), "auth-xai-context-exact"),
        )
        .unwrap();
        provider
            .set_authorization_execution_context("child-job", "source-job", "recipe", "input")
            .unwrap();
        provider
            .generate_image(
                &GenerateImageRequest {
                    prompt: "happy portrait".into(),
                    model: Some("test-image".into()),
                    aspect_ratio: "1:1".into(),
                    resolution: "1k".into(),
                    authorization_target: Some("happy".into()),
                },
                &authorization_root.path().join("exact.png"),
            )
            .unwrap();
        let ledger = crate::authorization::AuthorizationStore::open(
            authorization_root.path(),
            "auth-xai-context-exact",
        )
        .unwrap()
        .ledger()
        .unwrap();
        assert_eq!(ledger.requests.len(), 1);
        assert_eq!(ledger.requests[0].job_id.as_deref(), Some("child-job"));
        server.join().unwrap();
    }

    #[test]
    fn durable_video_edit_upload_does_not_consume_the_model_retry_slot() {
        let (base, server) = fake_server(2, move |index, first_line, _| match index {
            0 => {
                assert!(first_line.contains("POST /v1/files"));
                FakeResponse {
                    status: 200,
                    content_type: "application/json",
                    body: br#"{"id":"file_fixture_1"}"#.to_vec(),
                }
            }
            _ => {
                assert!(first_line.contains("POST /v1/videos/edits"));
                FakeResponse {
                    status: 200,
                    content_type: "application/json",
                    body:
                        br#"{"request_id":"video-edit-compound","usage":{"cost_in_usd_ticks":40}}"#
                            .to_vec(),
                }
            }
        });
        let authorization_root = tempfile::tempdir().unwrap();
        crate::authorization::AuthorizationStore::create(
            authorization_root.path(),
            &durable_test_manifest("auth-xai-compound", 1),
        )
        .unwrap();
        let provider = XaiProvider::with_base_url_and_authorization(
            Arc::new(TestCredential("api_key", "secret")),
            format!("{base}/v1"),
            "default",
            ProviderAuthorizationConfig::new(authorization_root.path(), "auth-xai-compound"),
        )
        .unwrap();
        let temp = tempfile::tempdir().unwrap();
        let video_path = temp.path().join("walk.mp4");
        fs::write(&video_path, b"fixture-video").unwrap();
        let video_sha256 = format!("{:x}", Sha256::digest(b"fixture-video"));
        let ticket = provider
            .edit_video(&EditVideoRequest {
                prompt: "repair one closed gait cycle".into(),
                model: Some("test-image".into()),
                video: ProviderInputRef {
                    path: video_path,
                    sha256: video_sha256,
                    provider_asset_id: None,
                },
                authorization_target: Some("happy".into()),
            })
            .unwrap();
        assert_eq!(ticket.request_id, "video-edit-compound");
        let ledger = crate::authorization::AuthorizationStore::open(
            authorization_root.path(),
            "auth-xai-compound",
        )
        .unwrap()
        .ledger()
        .unwrap();
        assert_eq!(ledger.consumed_request_count(), 1);
        assert_eq!(ledger.consumed_operation_count(), 2);
        assert_eq!(ledger.requests[0].operation, "private_file_upload");
        assert_eq!(ledger.requests[0].reserved_cost_ticks, 0);
        assert_eq!(ledger.requests[1].operation, "edit_video");
        server.join().unwrap();
    }

    #[test]
    fn durable_video_submission_persists_remote_id_and_settles_on_terminal_poll() {
        let media_bytes = b"durable-fixture-video".to_vec();
        let (base, server) = fake_server(3, move |index, first_line, base| match index {
            0 => {
                assert!(first_line.contains("POST /v1/videos/generations"));
                FakeResponse {
                    status: 200,
                    content_type: "application/json",
                    body: br#"{"request_id":"video-durable-1"}"#.to_vec(),
                }
            }
            1 => {
                assert!(first_line.contains("GET /v1/videos/video-durable-1"));
                FakeResponse {
                    status: 200,
                    content_type: "application/json",
                    body: serde_json::to_vec(&json!({
                        "status": "done",
                        "video": {"url": format!("{base}/media/video.mp4")},
                        "usage": {"cost_in_usd_ticks": 40}
                    }))
                    .unwrap(),
                }
            }
            _ => FakeResponse {
                status: 200,
                content_type: "video/mp4",
                body: media_bytes.clone(),
            },
        });
        let authorization_root = tempfile::tempdir().unwrap();
        crate::authorization::AuthorizationStore::create(
            authorization_root.path(),
            &durable_test_manifest("auth-video-async", 1),
        )
        .unwrap();
        let provider = |base: &str| {
            XaiProvider::with_base_url_and_authorization(
                Arc::new(TestCredential("api_key", "secret")),
                format!("{base}/v1"),
                "default",
                ProviderAuthorizationConfig::new(authorization_root.path(), "auth-video-async"),
            )
            .unwrap()
        };
        let submitter = provider(&base);
        let ticket = submitter
            .generate_video(&GenerateVideoRequest {
                prompt: "walk".into(),
                model: Some("test-image".into()),
                mode: VideoGenerationMode::Text,
                duration_seconds: 4,
                aspect_ratio: "1:1".into(),
                resolution: "720p".into(),
                authorization_target: Some("happy".into()),
            })
            .unwrap();
        let submitted = crate::authorization::AuthorizationStore::open(
            authorization_root.path(),
            "auth-video-async",
        )
        .unwrap()
        .ledger()
        .unwrap();
        assert_eq!(
            submitted.requests[0].state,
            crate::authorization::RequestLedgerStateV1::Submitted
        );
        assert_eq!(
            submitted.requests[0].provider_request_id.as_deref(),
            Some("video-durable-1")
        );

        // A new Provider instance simulates process restart. It can settle the
        // original reservation using only the persisted remote request id.
        let poller = provider(&base);
        let output = authorization_root.path().join("video.mp4");
        assert!(matches!(
            poller.poll(&ticket, &output).unwrap(),
            ProviderPoll::Succeeded(_)
        ));
        assert_eq!(fs::read(output).unwrap(), b"durable-fixture-video");
        let settled = crate::authorization::AuthorizationStore::open(
            authorization_root.path(),
            "auth-video-async",
        )
        .unwrap()
        .ledger()
        .unwrap();
        assert_eq!(
            settled.requests[0].state,
            crate::authorization::RequestLedgerStateV1::Settled
        );
        assert_eq!(settled.requests[0].observed_cost_ticks, Some(40));
        assert_eq!(settled.consumed_request_count(), 1);
        assert_eq!(settled.consumed_cost_ticks(), 40);
        assert_eq!(poller.usage().generated_videos, 1);
        assert_eq!(poller.usage().cost_in_usd_ticks, Some(40));
        server.join().unwrap();
    }

    #[test]
    fn durable_video_without_terminal_cost_returns_media_and_keeps_reservation() {
        let media_bytes = b"unmetered-fixture-video".to_vec();
        let (base, server) = fake_server(3, move |index, _, base| match index {
            0 => FakeResponse {
                status: 200,
                content_type: "application/json",
                body: br#"{"request_id":"video-unmetered-1"}"#.to_vec(),
            },
            1 => FakeResponse {
                status: 200,
                content_type: "application/json",
                body: serde_json::to_vec(&json!({
                    "status": "done",
                    "video": {"url": format!("{base}/media/video.mp4")}
                }))
                .unwrap(),
            },
            _ => FakeResponse {
                status: 200,
                content_type: "video/mp4",
                body: media_bytes.clone(),
            },
        });
        let authorization_root = tempfile::tempdir().unwrap();
        crate::authorization::AuthorizationStore::create(
            authorization_root.path(),
            &durable_test_manifest("auth-video-unmetered", 1),
        )
        .unwrap();
        let provider = XaiProvider::with_base_url_and_authorization(
            Arc::new(TestCredential("api_key", "secret")),
            format!("{base}/v1"),
            "default",
            ProviderAuthorizationConfig::new(authorization_root.path(), "auth-video-unmetered"),
        )
        .unwrap();
        let ticket = provider
            .generate_video(&GenerateVideoRequest {
                prompt: "idle".into(),
                model: Some("test-image".into()),
                mode: VideoGenerationMode::Text,
                duration_seconds: 4,
                aspect_ratio: "1:1".into(),
                resolution: "720p".into(),
                authorization_target: Some("happy".into()),
            })
            .unwrap();
        let output = authorization_root.path().join("video.mp4");
        assert!(matches!(
            provider.poll(&ticket, &output).unwrap(),
            ProviderPoll::Succeeded(_)
        ));
        assert_eq!(fs::read(output).unwrap(), b"unmetered-fixture-video");
        let ledger = crate::authorization::AuthorizationStore::open(
            authorization_root.path(),
            "auth-video-unmetered",
        )
        .unwrap()
        .ledger()
        .unwrap();
        assert_eq!(
            ledger.requests[0].state,
            crate::authorization::RequestLedgerStateV1::Ambiguous
        );
        assert_eq!(ledger.consumed_cost_ticks(), 100);
        server.join().unwrap();
    }

    #[test]
    fn durable_authorization_rejects_missing_target_before_network() {
        let authorization_root = tempfile::tempdir().unwrap();
        crate::authorization::AuthorizationStore::create(
            authorization_root.path(),
            &durable_test_manifest("auth-xai-target", 1),
        )
        .unwrap();
        let provider = XaiProvider::with_base_url_and_authorization(
            Arc::new(TestCredential("api_key", "secret")),
            "http://127.0.0.1:1/v1",
            "default",
            ProviderAuthorizationConfig::new(authorization_root.path(), "auth-xai-target"),
        )
        .unwrap();
        let error = provider
            .generate_image(
                &GenerateImageRequest {
                    prompt: "test".into(),
                    model: Some("test-image".into()),
                    aspect_ratio: "1:1".into(),
                    resolution: "1k".into(),
                    authorization_target: None,
                },
                &authorization_root.path().join("missing.png"),
            )
            .unwrap_err();
        assert!(matches!(error, ProviderError::RealProviderNotAccepted(_)));
        let ledger = crate::authorization::AuthorizationStore::open(
            authorization_root.path(),
            "auth-xai-target",
        )
        .unwrap()
        .ledger()
        .unwrap();
        assert_eq!(ledger.consumed_request_count(), 0);
    }

    #[test]
    fn xai_resolves_default_models_for_provenance() {
        let credentials: Arc<dyn CredentialProvider> =
            Arc::new(TestCredential("api_key", "secret"));
        let provider = XaiProvider::with_base_url(credentials, "http://127.0.0.1:1/v1").unwrap();
        assert_eq!(
            provider.resolved_image_model(None).as_deref(),
            Some(DEFAULT_IMAGE_MODEL)
        );
        assert_eq!(
            provider.resolved_video_model(None).as_deref(),
            Some(DEFAULT_VIDEO_MODEL)
        );
        assert_eq!(
            provider.resolved_video_edit_model(None).as_deref(),
            Some(DEFAULT_VIDEO_EDIT_MODEL)
        );
    }
}
