//! Explicit, local ComfyUI workflow profiles and bounded HTTP transport.
use std::fs;
use std::io::Read;
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
    if !valid_id(id) {
        return Err(ComfyError::InvalidProfile(
            "profile ID must use 1-64 ASCII letters, digits, _ or -".into(),
        ));
    }
    let mut profile: WorkflowProfile = serde_json::from_slice(&fs::read(path)?)?;
    if profile.workflow.is_relative() {
        profile.workflow = path
            .parent()
            .unwrap_or(Path::new("."))
            .join(&profile.workflow);
    }
    profile.workflow = profile.workflow.canonicalize()?;
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
    use std::io::Write;
    file.write_all(&bytes)?;
    Ok(stored)
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
        let mut missing_inputs = Vec::new();
        for (node_id, node) in workflow.as_object().unwrap() {
            let Some(class) = node["class_type"].as_str() else {
                continue;
            };
            let Some(definition) = available.get(class) else {
                continue;
            };
            let declared = &definition["input"];
            if let Some(inputs) = node["inputs"].as_object() {
                for key in inputs.keys() {
                    // ComfyUI also permits hidden inputs managed by the server.
                    let found = ["required", "optional", "hidden"]
                        .iter()
                        .any(|group| declared[*group].get(key).is_some());
                    if !found {
                        missing_inputs.push(format!("{node_id}.{key}"));
                    }
                }
            }
        }
        missing_inputs.sort();
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
                "unknown node inputs: {}",
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

    pub fn history(&self, prompt_id: &str) -> Result<Value, ComfyError> {
        if !valid_id(prompt_id) {
            return Err(ComfyError::InvalidProfile("invalid prompt ID".into()));
        }
        self.get_json(&format!("/history/{prompt_id}"))
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
