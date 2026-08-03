use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use base64::Engine;
use dirs_next::data_dir;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const VISION_COMPONENT_PROTOCOL: &str = "vision-component@1.0.0";
pub const VISION_COMPONENT_ID: &str = "vision-consistency";
// This preview key only exercises fail-closed verification while the component is unpublished.
// Replace it with a CI-controlled release key before publishing the first real component manifest.
const FORGE_COMPONENT_SIGNING_KEY_ID: &str = "forge-component-preview-unpublished-1";
const FORGE_COMPONENT_SIGNING_PUBLIC_KEY_HEX: &str =
    "4e3e13bf4de88bdfedaeed0bf37fc01a747d9897695d369cf04578a32fc553dd";

#[derive(Debug, Error)]
pub enum ComponentError {
    #[error("invalid component: {0}")]
    Invalid(String),
    #[error("component is not installed: {0}")]
    NotInstalled(String),
    #[error("component release is not published: {0}")]
    NotPublished(String),
    #[error("component process failed: {0}")]
    Process(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisionOperation {
    Health,
    SegmentForeground,
    IdentityEmbedding,
    PerceptualDistance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisionInputV1 {
    pub path: PathBuf,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisionComponentRequestV1 {
    pub schema_version: String,
    pub request_id: String,
    pub operation: VisionOperation,
    #[serde(default)]
    pub inputs: Vec<VisionInputV1>,
    #[serde(default)]
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisionComponentResponseV1 {
    pub schema_version: String,
    pub request_id: String,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<VisionComponentErrorV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisionComponentErrorV1 {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentLicenseV1 {
    pub name: String,
    pub spdx: String,
    pub redistribution: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentFileV1 {
    pub path: PathBuf,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentManifestV1 {
    pub schema_version: String,
    pub id: String,
    pub version: String,
    pub protocol: String,
    pub executable: String,
    pub executable_sha256: String,
    #[serde(default)]
    pub model_sha256: Vec<String>,
    #[serde(default)]
    pub model_files: Vec<ComponentFileV1>,
    #[serde(default)]
    pub licenses: Vec<ComponentLicenseV1>,
    pub signing_key_id: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentStatusV1 {
    pub id: String,
    pub version: String,
    pub protocol: String,
    pub status: String,
    pub installed: bool,
    pub available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<PathBuf>,
    pub message: String,
}

pub trait VisionComponent: Send + Sync {
    fn invoke(
        &self,
        request: &VisionComponentRequestV1,
    ) -> Result<VisionComponentResponseV1, ComponentError>;
}

#[derive(Debug, Default)]
pub struct FixtureVisionComponent;

impl VisionComponent for FixtureVisionComponent {
    fn invoke(
        &self,
        request: &VisionComponentRequestV1,
    ) -> Result<VisionComponentResponseV1, ComponentError> {
        validate_request(request)?;
        let result = match request.operation {
            VisionOperation::Health => serde_json::json!({
                "componentId": "fixture-vision",
                "protocol": VISION_COMPONENT_PROTOCOL,
                "models": ["fixture-mask@1", "fixture-embedding@1", "fixture-distance@1"]
            }),
            VisionOperation::SegmentForeground => {
                let input = request.inputs.first().ok_or_else(|| {
                    ComponentError::Invalid("segment_foreground requires one input".into())
                })?;
                serde_json::json!({
                    "inputSha256": input.sha256,
                    "maskSha256": format!("{:x}", Sha256::digest(format!("mask:{}", input.sha256))),
                    "foregroundCoverage": 0.5
                })
            }
            VisionOperation::IdentityEmbedding => {
                let input = request.inputs.first().ok_or_else(|| {
                    ComponentError::Invalid("identity_embedding requires one input".into())
                })?;
                let digest = Sha256::digest(input.sha256.as_bytes());
                let embedding = digest
                    .chunks_exact(4)
                    .map(|chunk| {
                        u32::from_be_bytes(chunk.try_into().expect("four bytes")) as f64
                            / u32::MAX as f64
                    })
                    .collect::<Vec<_>>();
                serde_json::json!({"model": "fixture-embedding@1", "embedding": embedding})
            }
            VisionOperation::PerceptualDistance => {
                if request.inputs.len() != 2 {
                    return Err(ComponentError::Invalid(
                        "perceptual_distance requires exactly two inputs".into(),
                    ));
                }
                let left = &request.inputs[0].sha256;
                let right = &request.inputs[1].sha256;
                let different = left
                    .bytes()
                    .zip(right.bytes())
                    .filter(|(left, right)| left != right)
                    .count();
                serde_json::json!({
                    "model": "fixture-distance@1",
                    "distance": different as f64 / 64.0
                })
            }
        };
        Ok(VisionComponentResponseV1 {
            schema_version: "1".into(),
            request_id: request.request_id.clone(),
            ok: true,
            result: Some(result),
            error: None,
        })
    }
}

pub fn component_store_root() -> Result<PathBuf, ComponentError> {
    if let Ok(root) = std::env::var("FORGE_COMPONENT_STORE") {
        return Ok(PathBuf::from(root));
    }
    data_dir()
        .map(|root| root.join("Game Sprite Forge/components"))
        .ok_or_else(|| ComponentError::Invalid("component store is unavailable".into()))
}

pub fn list_components() -> Result<Vec<ComponentStatusV1>, ComponentError> {
    Ok(vec![
        ComponentStatusV1 {
            id: "fixture-vision".into(),
            version: "1.0.0".into(),
            protocol: VISION_COMPONENT_PROTOCOL.into(),
            status: "built_in_fixture".into(),
            installed: true,
            available: true,
            root: None,
            message: "deterministic in-process fixture for contract tests".into(),
        },
        inspect_component(VISION_COMPONENT_ID)?,
    ])
}

pub fn inspect_component(id: &str) -> Result<ComponentStatusV1, ComponentError> {
    if id == "fixture-vision" {
        return Ok(list_components()?
            .into_iter()
            .next()
            .expect("fixture component is listed"));
    }
    if id != VISION_COMPONENT_ID {
        return Err(ComponentError::Invalid(format!("unknown component: {id}")));
    }
    let root = component_store_root()?.join(id);
    if !root.is_dir() {
        return Ok(ComponentStatusV1 {
            id: id.into(),
            version: "unpublished".into(),
            protocol: VISION_COMPONENT_PROTOCOL.into(),
            status: "not_published".into(),
            installed: false,
            available: false,
            root: Some(root),
            message:
                "the signed SAM/DINO/LPIPS component is gated on license and calibration review"
                    .into(),
        });
    }
    let current = fs::read_to_string(root.join("current"))?.trim().to_string();
    let version_root = root.join(&current);
    let manifest: ComponentManifestV1 =
        serde_json::from_slice(&fs::read(version_root.join("manifest.json"))?)?;
    validate_installed_manifest(&manifest, &version_root)?;
    Ok(ComponentStatusV1 {
        id: manifest.id,
        version: manifest.version,
        protocol: manifest.protocol,
        status: "installed".into(),
        installed: true,
        available: true,
        root: Some(version_root),
        message: "component manifest and executable hashes are valid".into(),
    })
}

pub fn install_component(
    id: &str,
    accept_licenses: bool,
) -> Result<ComponentStatusV1, ComponentError> {
    if id != VISION_COMPONENT_ID {
        return Err(ComponentError::Invalid(format!("unknown component: {id}")));
    }
    if !accept_licenses {
        return Err(ComponentError::Invalid(
            "component installation requires --accept-licenses".into(),
        ));
    }
    Err(ComponentError::NotPublished(
        "vision-consistency has no signed commercial-redistribution manifest yet".into(),
    ))
}

pub fn invoke_external_component(
    executable: &Path,
    request: &VisionComponentRequestV1,
) -> Result<VisionComponentResponseV1, ComponentError> {
    validate_request(request)?;
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| ComponentError::Process("component stdin is unavailable".into()))?
        .write_all(&serde_json::to_vec(request)?)?;
    drop(child.stdin.take());
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(ComponentError::Process(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    let response: VisionComponentResponseV1 = serde_json::from_slice(&output.stdout)?;
    if response.schema_version != "1" || response.request_id != request.request_id {
        return Err(ComponentError::Invalid(
            "component response schema or requestId does not match".into(),
        ));
    }
    Ok(response)
}

fn validate_request(request: &VisionComponentRequestV1) -> Result<(), ComponentError> {
    if request.schema_version != "1" || request.request_id.trim().is_empty() {
        return Err(ComponentError::Invalid(
            "vision component requires schemaVersion 1 and requestId".into(),
        ));
    }
    for input in &request.inputs {
        if !input.path.is_file() {
            return Err(ComponentError::Invalid(format!(
                "component input is missing: {}",
                input.path.display()
            )));
        }
        validate_sha(&input.sha256, "input SHA-256")?;
        let actual = format!("{:x}", Sha256::digest(fs::read(&input.path)?));
        if actual != input.sha256 {
            return Err(ComponentError::Invalid(format!(
                "component input SHA-256 changed: {}",
                input.path.display()
            )));
        }
    }
    Ok(())
}

fn validate_installed_manifest(
    manifest: &ComponentManifestV1,
    root: &Path,
) -> Result<(), ComponentError> {
    if manifest.schema_version != "1"
        || manifest.id != VISION_COMPONENT_ID
        || manifest.protocol != VISION_COMPONENT_PROTOCOL
        || manifest.signing_key_id != FORGE_COMPONENT_SIGNING_KEY_ID
        || manifest.signature.trim().is_empty()
    {
        return Err(ComponentError::Invalid(
            "component manifest identity, protocol, or signature metadata is invalid".into(),
        ));
    }
    verify_manifest_signature(manifest)?;
    validate_sha(&manifest.executable_sha256, "executable SHA-256")?;
    let executable = component_file_path(root, Path::new(&manifest.executable))?;
    let actual = format!("{:x}", Sha256::digest(fs::read(&executable)?));
    if actual != manifest.executable_sha256 {
        return Err(ComponentError::Invalid(
            "component executable failed SHA-256 verification".into(),
        ));
    }
    for hash in &manifest.model_sha256 {
        validate_sha(hash, "model SHA-256")?;
    }
    for model in &manifest.model_files {
        validate_sha(&model.sha256, "model SHA-256")?;
        let path = component_file_path(root, &model.path)?;
        let actual = format!("{:x}", Sha256::digest(fs::read(path)?));
        if actual != model.sha256 {
            return Err(ComponentError::Invalid(format!(
                "component model failed SHA-256 verification: {}",
                model.path.display()
            )));
        }
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ComponentSignedManifestV1<'a> {
    schema_version: &'a str,
    id: &'a str,
    version: &'a str,
    protocol: &'a str,
    executable: &'a str,
    executable_sha256: &'a str,
    model_sha256: &'a [String],
    model_files: &'a [ComponentFileV1],
    licenses: &'a [ComponentLicenseV1],
    signing_key_id: &'a str,
}

fn manifest_signing_payload(manifest: &ComponentManifestV1) -> Result<Vec<u8>, ComponentError> {
    Ok(serde_json::to_vec(&ComponentSignedManifestV1 {
        schema_version: &manifest.schema_version,
        id: &manifest.id,
        version: &manifest.version,
        protocol: &manifest.protocol,
        executable: &manifest.executable,
        executable_sha256: &manifest.executable_sha256,
        model_sha256: &manifest.model_sha256,
        model_files: &manifest.model_files,
        licenses: &manifest.licenses,
        signing_key_id: &manifest.signing_key_id,
    })?)
}

fn verify_manifest_signature(manifest: &ComponentManifestV1) -> Result<(), ComponentError> {
    let public_key = decode_hex_32(FORGE_COMPONENT_SIGNING_PUBLIC_KEY_HEX)?;
    let verifying_key = VerifyingKey::from_bytes(&public_key).map_err(|error| {
        ComponentError::Invalid(format!("invalid trusted signing key: {error}"))
    })?;
    verify_manifest_signature_with_key(manifest, &verifying_key)
}

fn verify_manifest_signature_with_key(
    manifest: &ComponentManifestV1,
    verifying_key: &VerifyingKey,
) -> Result<(), ComponentError> {
    let signature_bytes = base64::engine::general_purpose::STANDARD
        .decode(&manifest.signature)
        .map_err(|_| ComponentError::Invalid("component signature is not valid base64".into()))?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|_| ComponentError::Invalid("component signature must contain 64 bytes".into()))?;
    verifying_key
        .verify(&manifest_signing_payload(manifest)?, &signature)
        .map_err(|_| {
            ComponentError::Invalid("component manifest Ed25519 signature is invalid".into())
        })
}

fn component_file_path(root: &Path, relative: &Path) -> Result<PathBuf, ComponentError> {
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(ComponentError::Invalid(format!(
            "component file path must be relative and traversal-free: {}",
            relative.display()
        )));
    }
    Ok(root.join(relative))
}

fn decode_hex_32(value: &str) -> Result<[u8; 32], ComponentError> {
    if value.len() != 64 {
        return Err(ComponentError::Invalid(
            "trusted component key must contain 32 bytes".into(),
        ));
    }
    let mut output = [0_u8; 32];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| ComponentError::Invalid("trusted component key is not hex".into()))?;
    }
    Ok(output)
}

fn validate_sha(value: &str, label: &str) -> Result<(), ComponentError> {
    if value.len() != 64 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err(ComponentError::Invalid(format!(
            "{label} must be a 64-character SHA-256"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn test_manifest(root: &Path) -> ComponentManifestV1 {
        let executable = root.join("vision-component");
        fs::write(&executable, b"fixture component").expect("fixture executable");
        ComponentManifestV1 {
            schema_version: "1".into(),
            id: VISION_COMPONENT_ID.into(),
            version: "0.0.1-test".into(),
            protocol: VISION_COMPONENT_PROTOCOL.into(),
            executable: "vision-component".into(),
            executable_sha256: format!("{:x}", Sha256::digest(b"fixture component")),
            model_sha256: Vec::new(),
            model_files: Vec::new(),
            licenses: vec![ComponentLicenseV1 {
                name: "fixture".into(),
                spdx: "MIT".into(),
                redistribution: "allowed".into(),
            }],
            signing_key_id: FORGE_COMPONENT_SIGNING_KEY_ID.into(),
            signature: String::new(),
        }
    }

    #[test]
    fn component_manifest_signature_covers_payload() {
        let root = tempfile::tempdir().expect("temp dir");
        let mut manifest = test_manifest(root.path());
        let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
        manifest.signature = base64::engine::general_purpose::STANDARD.encode(
            signing_key
                .sign(&manifest_signing_payload(&manifest).expect("payload"))
                .to_bytes(),
        );
        verify_manifest_signature_with_key(&manifest, &signing_key.verifying_key())
            .expect("valid signature");

        manifest.version = "tampered".into();
        assert!(
            verify_manifest_signature_with_key(&manifest, &signing_key.verifying_key()).is_err()
        );
    }

    #[test]
    fn component_files_reject_parent_traversal() {
        let root = tempfile::tempdir().expect("temp dir");
        assert!(component_file_path(root.path(), Path::new("../model.bin")).is_err());
    }
}
