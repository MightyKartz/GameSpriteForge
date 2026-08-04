use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCapability {
    GenerateImage,
    EditImage,
    GenerateVideo,
    ImageToVideo,
    ReferenceToVideo,
    EditVideo,
    PrivateFileInput,
    Cancel,
    Usage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialKind {
    ApiKey,
    OAuthDeviceCode,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderHealth {
    pub provider_id: String,
    pub available: bool,
    pub authenticated: bool,
    pub auth_kind: CredentialKind,
    pub capabilities: Vec<ProviderCapability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constraints: Option<ProviderConstraints>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConstraints {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_image_references: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_video_references: Option<u8>,
    #[serde(default)]
    pub native_alpha: bool,
    #[serde(default)]
    pub video_edit: bool,
    #[serde(default)]
    pub end_frame: bool,
    #[serde(default)]
    pub private_file_input: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsage {
    #[serde(default)]
    pub requests: u32,
    #[serde(default)]
    pub generated_images: u32,
    #[serde(default)]
    pub generated_videos: u32,
    #[serde(default)]
    pub edited_videos: u32,
    #[serde(default)]
    pub private_file_uploads: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_in_usd_ticks: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderMedia {
    pub path: PathBuf,
    pub mime_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_asset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revised_prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderTicket {
    pub provider_id: String,
    pub request_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderPoll {
    Pending { progress: Option<u8> },
    Succeeded(ProviderMedia),
    Failed { code: String, message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateImageRequest {
    pub prompt: String,
    pub model: Option<String>,
    pub aspect_ratio: String,
    pub resolution: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditImageRequest {
    pub prompt: String,
    pub model: Option<String>,
    pub references: Vec<ProviderImageReference>,
    pub aspect_ratio: String,
    pub resolution: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceRole {
    SubjectIdentity,
    Style,
    PoseStructure,
    EditTarget,
    StartKeyframe,
    EndKeyframe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderImageReference {
    pub role: ReferenceRole,
    pub path: PathBuf,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_asset_id: Option<String>,
}

impl ProviderImageReference {
    pub fn from_path(role: ReferenceRole, path: impl Into<PathBuf>) -> Result<Self, ProviderError> {
        let path = path.into();
        let sha256 = format!("{:x}", Sha256::digest(fs::read(&path)?));
        Ok(Self {
            role,
            path,
            sha256,
            provider_asset_id: None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoGenerationMode {
    Text,
    ImageToVideo { image: PathBuf },
    ReferenceToVideo { images: Vec<PathBuf> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateVideoRequest {
    pub prompt: String,
    pub model: Option<String>,
    pub mode: VideoGenerationMode,
    pub duration_seconds: u32,
    pub aspect_ratio: String,
    pub resolution: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderInputRef {
    pub path: PathBuf,
    pub sha256: String,
    pub provider_asset_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditVideoRequest {
    pub prompt: String,
    pub model: Option<String>,
    pub video: ProviderInputRef,
}

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("provider is unavailable: {0}")]
    Unavailable(String),
    #[error("provider authentication is required: {0}")]
    AuthenticationRequired(String),
    #[error("provider entitlement is unavailable: {0}")]
    Entitlement(String),
    #[error("provider rate limit exceeded: {0}")]
    RateLimited(String),
    #[error("provider request failed: {0}")]
    Request(String),
    #[error("provider returned invalid output: {0}")]
    InvalidOutput(String),
    #[error("provider operation was cancelled")]
    Cancelled,
    #[error("real_provider_not_accepted: {0}")]
    RealProviderNotAccepted(String),
    #[error("provider_request_budget_exceeded: {0}")]
    RequestBudgetExceeded(String),
    #[error("provider_cost_budget_exceeded: {0}")]
    CostBudgetExceeded(String),
    #[error("provider io error: {0}")]
    Io(#[from] std::io::Error),
}

impl ProviderError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unavailable(_) => "provider_unavailable",
            Self::AuthenticationRequired(_) => "provider_authentication_required",
            Self::Entitlement(_) => "provider_entitlement_unavailable",
            Self::RateLimited(_) => "provider_rate_limited",
            Self::Request(_) => "provider_request_failed",
            Self::InvalidOutput(_) => "provider_invalid_output",
            Self::Cancelled => "cancelled",
            Self::RealProviderNotAccepted(_) => "real_provider_not_accepted",
            Self::RequestBudgetExceeded(_) => "provider_request_budget_exceeded",
            Self::CostBudgetExceeded(_) => "provider_cost_budget_exceeded",
            Self::Io(_) => "provider_io_error",
        }
    }
}

pub trait CredentialProvider: Send + Sync {
    fn kind(&self) -> CredentialKind;
    fn bearer(&self) -> Result<String, ProviderError>;
    fn refresh(&self) -> Result<String, ProviderError>;
    fn logout(&self) -> Result<(), ProviderError>;
}

pub trait MediaGenerationProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn capabilities(&self) -> Vec<ProviderCapability>;
    fn health_check(&self) -> ProviderHealth;
    fn resolved_image_model(&self, requested: Option<&str>) -> Option<String> {
        requested.map(str::to_owned)
    }
    fn resolved_video_model(&self, requested: Option<&str>) -> Option<String> {
        requested.map(str::to_owned)
    }
    fn resolved_video_edit_model(&self, requested: Option<&str>) -> Option<String> {
        self.resolved_video_model(requested)
    }
    fn generate_image(
        &self,
        request: &GenerateImageRequest,
        output_path: &Path,
    ) -> Result<ProviderMedia, ProviderError>;
    fn edit_image(
        &self,
        request: &EditImageRequest,
        output_path: &Path,
    ) -> Result<ProviderMedia, ProviderError>;
    fn generate_video(
        &self,
        request: &GenerateVideoRequest,
    ) -> Result<ProviderTicket, ProviderError>;
    fn edit_video(&self, _request: &EditVideoRequest) -> Result<ProviderTicket, ProviderError> {
        Err(ProviderError::Unavailable(
            "provider does not support video editing".into(),
        ))
    }
    fn poll(
        &self,
        ticket: &ProviderTicket,
        output_path: &Path,
    ) -> Result<ProviderPoll, ProviderError>;
    fn cancel(&self, ticket: &ProviderTicket) -> Result<(), ProviderError>;
    fn usage(&self) -> ProviderUsage;
}
