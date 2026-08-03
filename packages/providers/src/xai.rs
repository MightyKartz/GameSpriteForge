use std::collections::HashMap;
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
    ProviderError, ProviderHealth, ProviderMedia, ProviderPoll, ProviderTicket, ProviderUsage,
    VideoGenerationMode,
};
use reqwest::blocking::multipart::{Form, Part};
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::StatusCode;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use url::Url;

const DEFAULT_BASE_URL: &str = "https://api.x.ai/v1";
const DEFAULT_IMAGE_MODEL: &str = "grok-imagine-image-quality";
const DEFAULT_VIDEO_MODEL: &str = "grok-imagine-video-1.5";
const DEFAULT_VIDEO_EDIT_MODEL: &str = "grok-imagine-video";
const JSON_RESPONSE_LIMIT: usize = 64 * 1024 * 1024;
const MEDIA_RESPONSE_LIMIT: u64 = 512 * 1024 * 1024;
const VIDEO_DATA_URL_INPUT_LIMIT: u64 = 32 * 1024 * 1024;

pub struct XaiProvider {
    credentials: Arc<dyn CredentialProvider>,
    client: Client,
    base_url: String,
    usage: Mutex<ProviderUsage>,
    temporary_files: Mutex<HashMap<String, String>>,
}

impl XaiProvider {
    pub fn new(credentials: Arc<dyn CredentialProvider>) -> Self {
        Self::with_base_url(credentials, DEFAULT_BASE_URL)
            .expect("the bundled xAI base URL must be valid")
    }

    pub fn with_base_url(
        credentials: Arc<dyn CredentialProvider>,
        base_url: impl Into<String>,
    ) -> Result<Self, ProviderError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        validate_base_url(&base_url, credentials.kind())?;
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(20))
            .timeout(Duration::from_secs(300))
            .http1_only()
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("Game-Sprite-Forge/0.1")
            .build()
            .map_err(|error| ProviderError::Request(error.to_string()))?;
        Ok(Self {
            credentials,
            client,
            base_url,
            usage: Mutex::new(ProviderUsage::default()),
            temporary_files: Mutex::new(HashMap::new()),
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
        output_path: &Path,
    ) -> Result<ProviderMedia, ProviderError> {
        let url = format!("{}/{}", self.base_url, endpoint.trim_start_matches('/'));
        let response = self.send_authenticated(|bearer| {
            self.client
                .post(&url)
                .bearer_auth(bearer)
                .header("Content-Type", "application/json")
                .json(&payload)
        })?;
        let value = parse_json_response(response, "xAI image request")?;
        self.record_usage(&value, true, false);
        let first = value
            .get("data")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .ok_or_else(|| ProviderError::InvalidOutput("xAI returned no image".into()))?;
        let bytes = if let Some(encoded) = first.get("b64_json").and_then(Value::as_str) {
            base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .map_err(|error| ProviderError::InvalidOutput(error.to_string()))?
        } else if let Some(url) = first.get("url").and_then(Value::as_str) {
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
                "xAI image response omitted both b64_json and url".into(),
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
        let mut bearer = self.credentials.bearer()?;
        let mut refreshed = false;
        let mut rate_attempt = 0u32;
        loop {
            let response = build(&bearer)
                .send()
                .map_err(|error| ProviderError::Request(error.to_string()))?;
            if response.status() == StatusCode::UNAUTHORIZED && !refreshed {
                bearer = self.credentials.refresh()?;
                refreshed = true;
                continue;
            }
            if response.status() == StatusCode::TOO_MANY_REQUESTS && rate_attempt < 3 {
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
            return classify_response(response);
        }
    }

    fn record_usage(&self, value: &Value, image: bool, video: bool) {
        let mut usage = self.usage.lock().unwrap();
        usage.requests += 1;
        if image {
            usage.generated_images += 1;
        }
        if video {
            usage.generated_videos += 1;
        }
        if let Some(ticks) = value
            .pointer("/usage/cost_in_usd_ticks")
            .and_then(Value::as_u64)
        {
            usage.cost_in_usd_ticks = Some(usage.cost_in_usd_ticks.unwrap_or(0) + ticks);
        }
    }

    fn upload_private_file(&self, bytes: &[u8], file_name: &str) -> Result<String, ProviderError> {
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
        Ok(file_id.into())
    }

    fn delete_private_file(&self, file_id: &str) -> Result<(), ProviderError> {
        if !valid_file_id(file_id) {
            return Err(ProviderError::InvalidOutput(
                "refusing to delete an invalid xAI file id".into(),
            ));
        }
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
                image_data_url(&reference.path).map(|url| {
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
        self.image_request("images/edits", payload, output_path)
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
        let url = format!("{}/videos/generations", self.base_url);
        let response = self.send_authenticated(|bearer| {
            self.client
                .post(&url)
                .bearer_auth(bearer)
                .header("Content-Type", "application/json")
                .json(&payload)
        })?;
        let value = parse_json_response(response, "xAI video generation")?;
        self.record_usage(&value, false, false);
        let request_id = value
            .get("request_id")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ProviderError::InvalidOutput("xAI omitted video request_id".into()))?;
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
        let existing_file_id = request
            .video
            .provider_asset_id
            .as_deref()
            .filter(|id| valid_file_id(id));
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
            match self.upload_private_file(&bytes, file_name) {
                Ok(file_id) => {
                    temporary_file_id = Some(file_id.clone());
                    json!({"file_id": file_id})
                }
                Err(error) if files_api_unavailable(&error) => {
                    if bytes.len() as u64 > VIDEO_DATA_URL_INPUT_LIMIT {
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
                Err(error) => return Err(error),
            }
        };
        let payload = json!({
            "model": request.model.as_deref().unwrap_or(DEFAULT_VIDEO_EDIT_MODEL),
            "prompt": request.prompt,
            "video": video,
        });
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
        self.record_usage(&value, false, false);
        self.usage.lock().unwrap().edited_videos += 1;
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
                self.record_usage(&value, false, true);
                Ok(ProviderPoll::Succeeded(ProviderMedia {
                    path: output_path.to_path_buf(),
                    mime_type: "video/mp4".into(),
                    provider_asset_id: Some(ticket.request_id.clone()),
                    revised_prompt: None,
                }))
            }
            "failed" | "expired" => {
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
                },
                &temp.path().join("image.png"),
            )
            .unwrap();
        server.join().unwrap();
    }
}
