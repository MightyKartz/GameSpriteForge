//! Explicit, local ComfyUI workflow profiles and bounded HTTP transport.
use std::fs;
use std::io::{Read, Write};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use url::Url;
use uuid::Uuid;

const JSON_LIMIT: u64 = 8 * 1024 * 1024;
const MEDIA_LIMIT: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowProfile {
    pub schema_version: u32,
    pub endpoint: String,
    #[serde(default)]
    pub allow_remote: bool,
    pub media_kind: MediaKind,
    pub model_id: String,
    pub workflow: PathBuf,
    pub prompt_input: NodeInput,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_input: Option<NodeInput>,
    pub output_node: String,
    pub output_field: OutputField,
    pub max_output_bytes: u64,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Image,
    Video,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NodeInput {
    pub node: String,
    pub input: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputField {
    Images,
    Gifs,
    Videos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredProfile {
    pub profile_id: String,
    pub profile: WorkflowProfile,
    pub workflow_sha256: String,
}

/// Portable configuration fingerprint. Workflow bytes, media, model weights and credentials
/// are deliberately excluded; import requires the recipient to select the workflow file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileDescriptor {
    pub schema_version: u32,
    pub endpoint: String,
    pub allow_remote: bool,
    pub media_kind: MediaKind,
    pub model_id: String,
    pub prompt_input: NodeInput,
    pub reference_input: Option<NodeInput>,
    pub output_node: String,
    pub output_field: OutputField,
    pub max_output_bytes: u64,
    pub timeout_seconds: u64,
    pub workflow_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeCheck {
    pub profile_id: String,
    pub configured_workflow_sha256: String,
    pub candidate_workflow_sha256: String,
    pub changed_fields: Vec<&'static str>,
    pub requires_new_profile_id: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComfyDoctor {
    pub provider_id: &'static str,
    pub profile_id: String,
    pub reachable: bool,
    pub workflow_valid: bool,
    pub model_id: String,
    pub media_kind: MediaKind,
    pub workflow_sha256: String,
    pub missing_node_types: Vec<String>,
    pub missing_inputs: Vec<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OutputRef {
    pub filename: String,
    pub subfolder: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryState {
    Pending,
    Failed(String),
    Succeeded(OutputRef),
}

/// Read only the named prompt and output node; never consume another queue entry.
pub fn parse_history(
    history: &Value,
    prompt_id: &str,
    profile: &WorkflowProfile,
) -> Result<HistoryState, ComfyError> {
    let Some(entry) = history.get(prompt_id) else {
        return Ok(HistoryState::Pending);
    };
    if entry["status"]["status_str"] == "error" {
        return Ok(HistoryState::Failed(
            entry["status"]["messages"].to_string(),
        ));
    }
    if entry["status"]["completed"] != true {
        return Ok(HistoryState::Pending);
    }
    let field = match profile.output_field {
        OutputField::Images => "images",
        OutputField::Gifs => "gifs",
        OutputField::Videos => "videos",
    };
    let outputs = entry["outputs"].get(&profile.output_node).ok_or_else(|| {
        ComfyError::InvalidOutput(format!("output node {} missing", profile.output_node))
    })?;
    let files = outputs
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| ComfyError::InvalidOutput(format!("output field {field} missing")))?;
    if files.len() != 1 {
        return Err(ComfyError::InvalidOutput(format!(
            "expected exactly one {field} output, got {}",
            files.len()
        )));
    }
    let file = &files[0];
    let filename = file["filename"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ComfyError::InvalidOutput("output filename missing".into()))?;
    let subfolder = file["subfolder"].as_str().unwrap_or("");
    let kind = file["type"].as_str().unwrap_or("output");
    if !matches!(kind, "output" | "temp") {
        return Err(ComfyError::InvalidOutput(
            "unsupported output storage type".into(),
        ));
    }
    Ok(HistoryState::Succeeded(OutputRef {
        filename: filename.into(),
        subfolder: subfolder.into(),
        kind: kind.into(),
    }))
}

#[derive(Debug, thiserror::Error)]
pub enum ComfyError {
    #[error("invalid ComfyUI profile: {0}")]
    InvalidProfile(String),
    #[error("ComfyUI profile already exists with different content: {0}")]
    ProfileExists(String),
    #[error("ComfyUI profile not found: {0}")]
    ProfileNotFound(String),
    #[error("ComfyUI workflow changed after configuration; configure a new profile")]
    WorkflowChanged,
    #[error("ComfyUI request failed: {0}")]
    Request(String),
    #[error("ComfyUI returned invalid output: {0}")]
    InvalidOutput(String),
    #[error("ComfyUI prompt is running; shared-server interrupt is unsafe")]
    PromptRunning,
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
}

impl ComfyError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidProfile(_) => "comfyui_invalid_profile",
            Self::ProfileExists(_) => "comfyui_profile_exists",
            Self::ProfileNotFound(_) => "comfyui_profile_not_found",
            Self::WorkflowChanged => "comfyui_workflow_changed",
            Self::Request(_) => "comfyui_request_failed",
            Self::InvalidOutput(_) => "comfyui_invalid_output",
            Self::PromptRunning => "comfyui_prompt_running",
            Self::Io(_) => "comfyui_io_error",
            Self::Json(_) => "comfyui_invalid_json",
        }
    }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

pub fn profile_root() -> Result<PathBuf, ComfyError> {
    if let Some(root) = std::env::var_os("FORGE_COMFYUI_PROFILE_DIR") {
        return Ok(PathBuf::from(root));
    }
    Ok(dirs_next::config_dir()
        .ok_or_else(|| ComfyError::InvalidProfile("no user config directory".into()))?
        .join("Game Sprite Forge")
        .join("comfyui")
        .join("profiles"))
}

pub fn configure(root: &Path, id: &str, path: &Path) -> Result<StoredProfile, ComfyError> {
    let profile = read_candidate(path)?;
    store_profile(root, id, profile)
}

fn read_candidate(path: &Path) -> Result<WorkflowProfile, ComfyError> {
    let bytes = fs::read(path)?;
    if bytes.len() as u64 > JSON_LIMIT {
        return Err(ComfyError::InvalidProfile("profile exceeds 8 MiB".into()));
    }
    let mut profile: WorkflowProfile = serde_json::from_slice(&bytes)?;
    if profile.workflow.is_relative() {
        profile.workflow = path
            .parent()
            .unwrap_or(Path::new("."))
            .join(&profile.workflow);
    }
    profile.workflow = profile.workflow.canonicalize()?;
    Ok(profile)
}

fn store_profile(
    root: &Path,
    id: &str,
    profile: WorkflowProfile,
) -> Result<StoredProfile, ComfyError> {
    if !valid_id(id) {
        return Err(ComfyError::InvalidProfile(
            "profile ID must use 1-64 ASCII letters, digits, _ or -".into(),
        ));
    }
    let (hash, _) = validate_profile(&profile)?;
    let stored = StoredProfile {
        profile_id: id.into(),
        profile,
        workflow_sha256: hash,
    };
    let target = root.join(format!("{id}.json"));
    fs::create_dir_all(root)?;
    if target.exists() {
        let old: StoredProfile = serde_json::from_slice(&fs::read(&target)?)?;
        if serde_json::to_value(&old)? == serde_json::to_value(&stored)? {
            return Ok(old);
        }
        return Err(ComfyError::ProfileExists(target.display().to_string()));
    }
    let bytes = serde_json::to_vec_pretty(&stored)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)?;
    file.write_all(&bytes)?;
    Ok(stored)
}

pub fn export_descriptor(
    root: &Path,
    id: &str,
    output: &Path,
) -> Result<ProfileDescriptor, ComfyError> {
    let stored = load(root, id)?;
    let p = stored.profile;
    let descriptor = ProfileDescriptor {
        schema_version: 1,
        endpoint: p.endpoint,
        allow_remote: p.allow_remote,
        media_kind: p.media_kind,
        model_id: p.model_id,
        prompt_input: p.prompt_input,
        reference_input: p.reference_input,
        output_node: p.output_node,
        output_field: p.output_field,
        max_output_bytes: p.max_output_bytes,
        timeout_seconds: p.timeout_seconds,
        workflow_sha256: stored.workflow_sha256,
    };
    let bytes = serde_json::to_vec_pretty(&descriptor)?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?
        .write_all(&bytes)?;
    Ok(descriptor)
}

pub fn import_descriptor(
    root: &Path,
    id: &str,
    descriptor: &Path,
    workflow: &Path,
) -> Result<StoredProfile, ComfyError> {
    let bytes = fs::read(descriptor)?;
    if bytes.len() as u64 > JSON_LIMIT {
        return Err(ComfyError::InvalidProfile(
            "descriptor exceeds 8 MiB".into(),
        ));
    }
    let descriptor: ProfileDescriptor = serde_json::from_slice(&bytes)?;
    if descriptor.schema_version != 1
        || descriptor.workflow_sha256.len() != 64
        || !descriptor
            .workflow_sha256
            .bytes()
            .all(|b| b.is_ascii_hexdigit())
    {
        return Err(ComfyError::InvalidProfile(
            "invalid descriptor schema or workflow hash".into(),
        ));
    }
    let profile = WorkflowProfile {
        schema_version: 1,
        endpoint: descriptor.endpoint,
        allow_remote: descriptor.allow_remote,
        media_kind: descriptor.media_kind,
        model_id: descriptor.model_id,
        workflow: workflow.canonicalize()?,
        prompt_input: descriptor.prompt_input,
        reference_input: descriptor.reference_input,
        output_node: descriptor.output_node,
        output_field: descriptor.output_field,
        max_output_bytes: descriptor.max_output_bytes,
        timeout_seconds: descriptor.timeout_seconds,
    };
    let (hash, _) = validate_profile(&profile)?;
    if hash != descriptor.workflow_sha256 {
        return Err(ComfyError::WorkflowChanged);
    }
    store_profile(root, id, profile)
}

pub fn check_upgrade(root: &Path, id: &str, candidate: &Path) -> Result<UpgradeCheck, ComfyError> {
    let current = load(root, id)?;
    let next = read_candidate(candidate)?;
    let (hash, _) = validate_profile(&next)?;
    let old = &current.profile;
    let mut changed = Vec::new();
    if old.endpoint != next.endpoint || old.allow_remote != next.allow_remote {
        changed.push("endpoint");
    }
    if old.media_kind != next.media_kind {
        changed.push("mediaKind");
    }
    if old.model_id != next.model_id {
        changed.push("modelId");
    }
    if old.prompt_input.node != next.prompt_input.node
        || old.prompt_input.input != next.prompt_input.input
    {
        changed.push("promptInput");
    }
    if serde_json::to_value(&old.reference_input)? != serde_json::to_value(&next.reference_input)? {
        changed.push("referenceInput");
    }
    if old.output_node != next.output_node {
        changed.push("outputNode");
    }
    if serde_json::to_value(old.output_field)? != serde_json::to_value(next.output_field)? {
        changed.push("outputField");
    }
    if old.max_output_bytes != next.max_output_bytes || old.timeout_seconds != next.timeout_seconds
    {
        changed.push("limits");
    }
    if current.workflow_sha256 != hash {
        changed.push("workflowSha256");
    }
    Ok(UpgradeCheck {
        profile_id: id.into(),
        configured_workflow_sha256: current.workflow_sha256,
        candidate_workflow_sha256: hash,
        requires_new_profile_id: !changed.is_empty(),
        changed_fields: changed,
    })
}

pub fn load(root: &Path, id: &str) -> Result<StoredProfile, ComfyError> {
    if !valid_id(id) {
        return Err(ComfyError::InvalidProfile("invalid profile ID".into()));
    }
    let path = root.join(format!("{id}.json"));
    if !path.is_file() {
        return Err(ComfyError::ProfileNotFound(id.into()));
    }
    let stored: StoredProfile = serde_json::from_slice(&fs::read(path)?)?;
    if stored.profile_id != id {
        return Err(ComfyError::InvalidProfile(
            "stored profile ID mismatch".into(),
        ));
    }
    let (hash, _) = validate_profile(&stored.profile)?;
    if hash != stored.workflow_sha256 {
        return Err(ComfyError::WorkflowChanged);
    }
    Ok(stored)
}

pub fn validate_profile(profile: &WorkflowProfile) -> Result<(String, Value), ComfyError> {
    if profile.schema_version != 1 || profile.model_id.trim().is_empty() {
        return Err(ComfyError::InvalidProfile(
            "schemaVersion must be 1 and modelId must be nonempty".into(),
        ));
    }
    let url =
        Url::parse(&profile.endpoint).map_err(|e| ComfyError::InvalidProfile(e.to_string()))?;
    if url.scheme() != "http"
        || url.username() != ""
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
    {
        return Err(ComfyError::InvalidProfile(
            "endpoint must be an HTTP origin without credentials, path or query".into(),
        ));
    }
    let host = url.host_str().unwrap_or_default();
    let loopback = host.eq_ignore_ascii_case("localhost")
        || host
            .trim_matches(['[', ']'])
            .parse::<IpAddr>()
            .is_ok_and(|ip| ip.is_loopback());
    if !loopback && !profile.allow_remote {
        return Err(ComfyError::InvalidProfile(
            "non-loopback endpoint requires allowRemote=true".into(),
        ));
    }
    if profile.timeout_seconds == 0
        || profile.timeout_seconds > 86400
        || profile.max_output_bytes == 0
        || profile.max_output_bytes > MEDIA_LIMIT
    {
        return Err(ComfyError::InvalidProfile(
            "timeoutSeconds must be 1..86400 and maxOutputBytes 1..536870912".into(),
        ));
    }
    if profile.media_kind == MediaKind::Image
        && !matches!(profile.output_field, OutputField::Images)
    {
        return Err(ComfyError::InvalidProfile(
            "image outputField must be images".into(),
        ));
    }
    let bytes = fs::read(&profile.workflow)?;
    if bytes.len() as u64 > JSON_LIMIT {
        return Err(ComfyError::InvalidProfile("workflow exceeds 8 MiB".into()));
    }
    let workflow: Value = serde_json::from_slice(&bytes)?;
    let nodes = workflow.as_object().ok_or_else(|| {
        ComfyError::InvalidProfile("workflow must be an API-format object keyed by node ID".into())
    })?;
    let prompt_node = nodes
        .get(&profile.prompt_input.node)
        .ok_or_else(|| ComfyError::InvalidProfile("promptInput node missing".into()))?;
    if !prompt_node
        .get("inputs")
        .and_then(Value::as_object)
        .is_some_and(|inputs| inputs.contains_key(&profile.prompt_input.input))
    {
        return Err(ComfyError::InvalidProfile(
            "promptInput input missing".into(),
        ));
    }
    if let Some(reference) = &profile.reference_input {
        if !nodes
            .get(&reference.node)
            .and_then(|node| node.get("inputs"))
            .and_then(Value::as_object)
            .is_some_and(|inputs| inputs.contains_key(&reference.input))
        {
            return Err(ComfyError::InvalidProfile("referenceInput missing".into()));
        }
    }
    if !nodes
        .get(&profile.output_node)
        .is_some_and(|n| n.get("class_type").and_then(Value::as_str).is_some())
    {
        return Err(ComfyError::InvalidProfile(
            "outputNode missing or not API-format".into(),
        ));
    }
    if nodes.values().any(|n| {
        n.get("class_type").and_then(Value::as_str).is_none()
            || n.get("inputs").and_then(Value::as_object).is_none()
    }) {
        return Err(ComfyError::InvalidProfile(
            "all workflow nodes require class_type and inputs".into(),
        ));
    }
    Ok((format!("{:x}", Sha256::digest(bytes)), workflow))
}

pub struct ComfyClient {
    client: Client,
    endpoint: String,
}

impl ComfyClient {
    pub fn new(profile: &WorkflowProfile) -> Result<Self, ComfyError> {
        validate_profile(profile)?;
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(profile.timeout_seconds))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| ComfyError::Request(e.to_string()))?;
        Ok(Self {
            client,
            endpoint: profile.endpoint.trim_end_matches('/').into(),
        })
    }

    fn get_json(&self, route: &str) -> Result<Value, ComfyError> {
        let response = self
            .client
            .get(format!("{}{route}", self.endpoint))
            .send()
            .map_err(|e| ComfyError::Request(e.to_string()))?;
        bounded_json(response)
    }

    pub fn doctor(&self, stored: &StoredProfile) -> Result<ComfyDoctor, ComfyError> {
        let (_, workflow) = validate_profile(&stored.profile)?;
        let info = self.get_json("/object_info")?;
        let available = info
            .as_object()
            .ok_or_else(|| ComfyError::InvalidOutput("object_info is not an object".into()))?;
        let mut missing: Vec<String> = workflow
            .as_object()
            .unwrap()
            .values()
            .filter_map(|node| node["class_type"].as_str())
            .filter(|class| !available.contains_key(*class))
            .map(str::to_owned)
            .collect();
        missing.sort();
        missing.dedup();
        // Only mapped inputs must appear in the class's top-level metadata.
        // Other API-format keys may be generated by dynamic combo/autogrow widgets.
        let mut missing_inputs = Vec::new();
        for mapped in std::iter::once(&stored.profile.prompt_input)
            .chain(stored.profile.reference_input.iter())
        {
            let class = workflow[&mapped.node]["class_type"].as_str().unwrap();
            let declared = &available.get(class).unwrap_or(&Value::Null)["input"];
            if !["required", "optional", "hidden"]
                .iter()
                .any(|group| declared[*group].get(&mapped.input).is_some())
            {
                missing_inputs.push(format!("{}.{}", mapped.node, mapped.input));
            }
        }
        let output_class = workflow[&stored.profile.output_node]["class_type"]
            .as_str()
            .unwrap();
        let output_declared = available
            .get(output_class)
            .and_then(|v| v["output_node"].as_bool());
        let output_valid = output_declared == Some(true);
        let mut issues = Vec::new();
        if !missing.is_empty() {
            issues.push(format!("missing node types: {}", missing.join(", ")));
        }
        if !missing_inputs.is_empty() {
            issues.push(format!(
                "mapped inputs absent from object_info: {}",
                missing_inputs.join(", ")
            ));
        }
        if !output_valid {
            issues.push(format!(
                "output node {} is not declared as an output node",
                stored.profile.output_node
            ));
        }
        Ok(ComfyDoctor {
            provider_id: "comfyui",
            profile_id: stored.profile_id.clone(),
            reachable: true,
            workflow_valid: missing.is_empty() && missing_inputs.is_empty() && output_valid,
            model_id: stored.profile.model_id.clone(),
            media_kind: stored.profile.media_kind,
            workflow_sha256: stored.workflow_sha256.clone(),
            message: (!issues.is_empty()).then(|| issues.join("; ")),
            missing_node_types: missing,
            missing_inputs,
        })
    }

    pub fn submit(
        &self,
        profile: &WorkflowProfile,
        prompt: &str,
        prompt_id: &str,
    ) -> Result<(), ComfyError> {
        self.submit_with_reference(profile, prompt, prompt_id, None)
    }

    pub fn submit_with_reference(
        &self,
        profile: &WorkflowProfile,
        prompt: &str,
        prompt_id: &str,
        reference_filename: Option<&str>,
    ) -> Result<(), ComfyError> {
        let uuid = Uuid::parse_str(prompt_id)
            .map_err(|_| ComfyError::InvalidProfile("prompt ID must be a UUID".into()))?;
        if uuid.to_string() != prompt_id {
            return Err(ComfyError::InvalidProfile(
                "prompt ID must be canonical lowercase UUID".into(),
            ));
        }
        let (_, mut workflow) = validate_profile(profile)?;
        workflow[&profile.prompt_input.node]["inputs"][&profile.prompt_input.input] =
            Value::String(prompt.into());
        match (&profile.reference_input, reference_filename) {
            (Some(mapped), Some(filename)) if !filename.is_empty() => {
                workflow[&mapped.node]["inputs"][&mapped.input] = Value::String(filename.into());
            }
            (None, None) => {}
            _ => {
                return Err(ComfyError::InvalidProfile(
                    "reference image and referenceInput must be specified together".into(),
                ))
            }
        }
        let response = self
            .client
            .post(format!("{}/prompt", self.endpoint))
            .json(&json!({"prompt": workflow, "client_id": "forge", "prompt_id": prompt_id}))
            .send()
            .map_err(|e| ComfyError::Request(e.to_string()))?;
        let value = bounded_json(response)?;
        if value["prompt_id"].as_str() != Some(prompt_id) {
            return Err(ComfyError::InvalidOutput(
                "prompt_id missing or different; query history before retrying".into(),
            ));
        }
        Ok(())
    }

    pub fn upload_image(&self, filename: &str, bytes: Vec<u8>) -> Result<String, ComfyError> {
        if filename.is_empty() || filename.contains(['/', '\\']) || bytes.len() > 32 * 1024 * 1024 {
            return Err(ComfyError::InvalidProfile(
                "invalid reference image filename or size".into(),
            ));
        }
        let part = reqwest::blocking::multipart::Part::bytes(bytes)
            .file_name(filename.to_owned())
            .mime_str("image/png")
            .map_err(|e| ComfyError::Request(e.to_string()))?;
        let form = reqwest::blocking::multipart::Form::new()
            .part("image", part)
            .text("type", "input")
            .text("overwrite", "false");
        let response = self
            .client
            .post(format!("{}/upload/image", self.endpoint))
            .multipart(form)
            .send()
            .map_err(|e| ComfyError::Request(e.to_string()))?;
        let value = bounded_json(response)?;
        let name = value["name"]
            .as_str()
            .ok_or_else(|| ComfyError::InvalidOutput("upload filename missing".into()))?;
        if value["type"] != "input" || name.is_empty() || name.contains(['/', '\\']) {
            return Err(ComfyError::InvalidOutput(
                "invalid uploaded image reference".into(),
            ));
        }
        Ok(name.into())
    }

    pub fn history(&self, prompt_id: &str) -> Result<Value, ComfyError> {
        if !valid_id(prompt_id) {
            return Err(ComfyError::InvalidProfile("invalid prompt ID".into()));
        }
        self.get_json(&format!("/history/{prompt_id}"))
    }

    /// Delete only this Forge prompt while it is still pending. Running prompts are never
    /// interrupted because ComfyUI's interrupt endpoint acts on the shared active queue.
    pub fn cancel_pending(&self, prompt_id: &str) -> Result<bool, ComfyError> {
        let queue = self.get_json("/queue")?;
        let matches = |field: &str| -> Vec<&Value> {
            queue[field]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|item| item[1].as_str() == Some(prompt_id))
                .collect()
        };
        if !matches("queue_running").is_empty() {
            return Err(ComfyError::PromptRunning);
        }
        let pending = matches("queue_pending");
        if pending.is_empty() {
            return Ok(false);
        }
        if pending.len() != 1 || pending[0][3]["client_id"] != "forge" {
            return Err(ComfyError::InvalidOutput(
                "prompt does not belong to Forge".into(),
            ));
        }
        bounded_json(
            self.client
                .post(format!("{}/queue", self.endpoint))
                .json(&json!({"delete":[prompt_id]}))
                .send()
                .map_err(|e| ComfyError::Request(e.to_string()))?,
        )?;
        let queue = self.get_json("/queue")?;
        if queue["queue_running"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|item| item[1].as_str() == Some(prompt_id))
        {
            return Err(ComfyError::PromptRunning);
        }
        if queue["queue_pending"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|item| item[1].as_str() == Some(prompt_id))
        {
            return Err(ComfyError::InvalidOutput(
                "prompt remained queued after delete".into(),
            ));
        }
        Ok(true)
    }

    pub fn download(
        &self,
        filename: &str,
        subfolder: &str,
        kind: &str,
        limit: u64,
    ) -> Result<Vec<u8>, ComfyError> {
        if limit == 0 || limit > MEDIA_LIMIT {
            return Err(ComfyError::InvalidProfile(
                "invalid output byte limit".into(),
            ));
        }
        let response = self
            .client
            .get(format!("{}/view", self.endpoint))
            .query(&[
                ("filename", filename),
                ("subfolder", subfolder),
                ("type", kind),
            ])
            .send()
            .map_err(|e| ComfyError::Request(e.to_string()))?;
        let response = response
            .error_for_status()
            .map_err(|e| ComfyError::Request(e.to_string()))?;
        if response.content_length().is_some_and(|size| size > limit) {
            return Err(ComfyError::InvalidOutput("media exceeds byte limit".into()));
        }
        let mut bytes = Vec::new();
        response.take(limit + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > limit {
            return Err(ComfyError::InvalidOutput("media exceeds byte limit".into()));
        }
        Ok(bytes)
    }
}

fn bounded_json(response: reqwest::blocking::Response) -> Result<Value, ComfyError> {
    let response = response
        .error_for_status()
        .map_err(|e| ComfyError::Request(e.to_string()))?;
    if response
        .content_length()
        .is_some_and(|size| size > JSON_LIMIT)
    {
        return Err(ComfyError::InvalidOutput(
            "JSON response exceeds 8 MiB".into(),
        ));
    }
    let mut bytes = Vec::new();
    response.take(JSON_LIMIT + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > JSON_LIMIT {
        return Err(ComfyError::InvalidOutput(
            "JSON response exceeds 8 MiB".into(),
        ));
    }
    Ok(serde_json::from_slice(&bytes)?)
}
