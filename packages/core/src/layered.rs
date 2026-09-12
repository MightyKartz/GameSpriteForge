//! Provider-free intake of already registered PNG layers into a formal Pack.
//! Inputs retain their encoded bytes, common canvas and pivots. This does not
//! infer a rig, cut artwork, matte backgrounds, or approve occlusion quality.
use std::{
    fs,
    path::{Path, PathBuf},
};

use image::{imageops::FilterType, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub use forge_pack::layered::{
    LayerBlend, LayerKeyframe, LayerTrack, LayerTransform, LayeredCanvas, LayeredClip,
    LayeredLayer, LayeredManifest, LayeredSampling,
};

#[derive(Debug, thiserror::Error)]
pub enum LayeredError {
    #[error("invalid layered request: {0}")]
    Invalid(String),
    #[error("layered output already exists: {0}")]
    OutputExists(PathBuf),
    #[error("layered source changed or does not match SHA256: {0}")]
    SourceHashMismatch(PathBuf),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("pack error: {0}")]
    Pack(#[from] forge_pack::PackError),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrepareLayeredRequest {
    pub schema_version: String,
    pub id: String,
    pub name: String,
    pub license: String,
    pub canvas: LayeredCanvas,
    pub sampling: LayeredSampling,
    pub layers: Vec<LayeredSourceLayer>,
    #[serde(default)]
    pub clips: Vec<LayeredClip>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_clip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayeredSourceLayer {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub sha256: String,
    pub pivot: [f64; 2],
    #[serde(default)]
    pub transform: LayerTransform,
    #[serde(default)]
    pub blend: LayerBlend,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LayeredInputSummary {
    pub id: String,
    pub canvas: LayeredCanvas,
    pub layer_count: usize,
    pub clip_count: usize,
    pub total_source_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LayeredPackOutput {
    pub pack_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub preview_path: PathBuf,
    pub scene_path: PathBuf,
    pub quality_report_path: PathBuf,
    pub layer_count: usize,
    pub clip_count: usize,
}

pub fn manifest_for_request(request: &PrepareLayeredRequest) -> LayeredManifest {
    LayeredManifest {
        schema_version: "1".into(),
        asset_type: "layered".into(),
        id: request.id.clone(),
        name: request.name.clone(),
        canvas: request.canvas.clone(),
        sampling: request.sampling,
        layers: request
            .layers
            .iter()
            .map(|source| LayeredLayer {
                id: source.id.clone(),
                name: source.name.clone(),
                texture: format!("assets/layers/{}.png", source.id),
                sha256: source.sha256.clone(),
                pivot: source.pivot,
                transform: source.transform.clone(),
                blend: source.blend,
            })
            .collect(),
        clips: request.clips.clone(),
        default_clip: request.default_clip.clone(),
    }
}

/// Relative source paths are resolved by the caller (the CLI uses request-file
/// directory), matching other Forge request workflows. No source is modified.
pub fn validate_request(
    request: &PrepareLayeredRequest,
) -> Result<LayeredInputSummary, LayeredError> {
    if request.schema_version != "1"
        || request.license.trim().is_empty()
        || request.license.len() > 256
    {
        return Err(LayeredError::Invalid(
            "schemaVersion 1 and an explicit license are required".into(),
        ));
    }
    let manifest = manifest_for_request(request);
    forge_pack::layered::validate_manifest(&manifest)?;
    let mut total_source_bytes = 0;
    for source in &request.layers {
        total_source_bytes += read_source(source, &request.canvas)?.len() as u64;
    }
    Ok(LayeredInputSummary {
        id: request.id.clone(),
        canvas: request.canvas.clone(),
        layer_count: request.layers.len(),
        clip_count: request.clips.len(),
        total_source_bytes,
    })
}

fn read_source(
    source: &LayeredSourceLayer,
    canvas: &LayeredCanvas,
) -> Result<Vec<u8>, LayeredError> {
    let metadata = fs::symlink_metadata(&source.path)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > forge_pack::layered::MAX_PNG_BYTES
    {
        return Err(LayeredError::Invalid(format!(
            "layer source must be a regular PNG no larger than 32 MiB: {}",
            source.path.display()
        )));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(LayeredError::Invalid(
                "layer source cannot be a Windows reparse point".into(),
            ));
        }
    }
    let bytes = fs::read(&source.path)?;
    if format!("{:x}", Sha256::digest(&bytes)) != source.sha256 {
        return Err(LayeredError::SourceHashMismatch(source.path.clone()));
    }
    forge_pack::layered::validate_texture_bytes(&bytes, canvas)?;
    Ok(bytes)
}

/// Assemble and validate in a temporary sibling before reserving a new output
/// directory. Existing output paths are never merged into or overwritten.
pub fn prepare_layered_pack(
    request: &PrepareLayeredRequest,
    pack_dir: &Path,
) -> Result<LayeredPackOutput, LayeredError> {
    if fs::symlink_metadata(pack_dir).is_ok() {
        return Err(LayeredError::OutputExists(pack_dir.to_path_buf()));
    }
    validate_request(request)?;
    let manifest = manifest_for_request(request);
    let parent = pack_dir
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let staging = tempfile::Builder::new()
        .prefix(".forge-layered-")
        .tempdir_in(parent)?;
    let staged_pack = staging.path();
    fs::create_dir_all(staged_pack.join("assets/layers"))?;
    fs::create_dir_all(staged_pack.join("previews"))?;
    // Decode one original at a time. Preview thumbnails cannot affect delivered
    // texture bytes or their registration and are labelled as a contact sheet.
    let count = request.layers.len() as u32;
    let columns = (count as f64).sqrt().ceil() as u32;
    let rows = count.div_ceil(columns);
    let mut preview = RgbaImage::from_pixel(columns * 192, rows * 192, Rgba([0, 0, 0, 0]));
    for (i, source) in request.layers.iter().enumerate() {
        let bytes = read_source(source, &request.canvas)?;
        fs::write(staged_pack.join(&manifest.layers[i].texture), &bytes)?;
        let decoded =
            image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)?.to_rgba8();
        let scale = (176.0 / f64::from(request.canvas.width))
            .min(176.0 / f64::from(request.canvas.height))
            .min(1.0);
        let width = (f64::from(request.canvas.width) * scale).round().max(1.0) as u32;
        let height = (f64::from(request.canvas.height) * scale).round().max(1.0) as u32;
        let thumb = image::imageops::resize(
            &decoded,
            width,
            height,
            match request.sampling {
                LayeredSampling::Nearest => FilterType::Nearest,
                LayeredSampling::Linear => FilterType::Lanczos3,
            },
        );
        let x = (i as u32 % columns) * 192 + (192 - width) / 2;
        let y = (i as u32 / columns) * 192 + (192 - height) / 2;
        image::imageops::overlay(&mut preview, &thumb, i64::from(x), i64::from(y));
    }
    preview.save(staged_pack.join("previews/layers.png"))?;
    write_json(&staged_pack.join("assets/manifest.json"), &manifest)?;
    write_json(
        &staged_pack.join("assets/godot_import.json"),
        &forge_pack::layered::godot_helper(&manifest),
    )?;
    write_json(
        &staged_pack.join("quality-report.json"),
        &forge_pack::layered::quality_report(&manifest),
    )?;
    fs::write(
        staged_pack.join("assets/layered.tscn"),
        forge_pack::layered::godot_scene(&manifest),
    )?;
    fs::write(
        staged_pack.join("assets/forge_layered_player.gd"),
        forge_pack::layered::GODOT_LAYERED_CONTROLLER,
    )?;
    fs::write(
        staged_pack.join("assets/forge_alpha_multiply.gdshader"),
        forge_pack::layered::GODOT_ALPHA_MULTIPLY_SHADER_V1,
    )?;
    write_json(
        &staged_pack.join("forgepack.json"),
        &serde_json::json!({
            "schemaVersion":"4.0.0", "assetType":"layered", "id":request.id, "name":request.name,
            "version":"0.1.0", "createdAt":chrono::Utc::now(),
            "creator":{"name":"Game Sprite Forge"}, "license":{"type":request.license},
            "source":{"kind":"import_layers", "metadata":{
                "profile":"local-layered-import@1.0.0", "canvasPolicy":"preserve_source",
                "sources":request.layers.iter().map(|layer| serde_json::json!({
                    "id":layer.id,"sha256":layer.sha256,"texture":format!("assets/layers/{}.png",layer.id)
                })).collect::<Vec<_>>()
            }},
            "assets":{"manifest":"assets/manifest.json", "godotHelper":"assets/godot_import.json",
                  "godotScene":"assets/layered.tscn", "godotController":"assets/forge_layered_player.gd",
                  "godotBlendShader":"assets/forge_alpha_multiply.gdshader",
                      "qualityReport":"quality-report.json"},
            "previews":{"image":"previews/layers.png"}
        }),
    )?;
    forge_pack::validate_pack_layout(staged_pack)?;
    // create_dir is the no-clobber reservation. Publishing only begins after a
    // complete successful validation; failures never alter an existing output.
    fs::create_dir(pack_dir).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            LayeredError::OutputExists(pack_dir.to_path_buf())
        } else {
            LayeredError::Io(error)
        }
    })?;
    for name in [
        "assets",
        "previews",
        "quality-report.json",
        "forgepack.json",
    ] {
        fs::rename(staged_pack.join(name), pack_dir.join(name))?;
    }
    Ok(LayeredPackOutput {
        pack_dir: pack_dir.to_path_buf(),
        manifest_path: pack_dir.join("assets/manifest.json"),
        preview_path: pack_dir.join("previews/layers.png"),
        scene_path: pack_dir.join("assets/layered.tscn"),
        quality_report_path: pack_dir.join("quality-report.json"),
        layer_count: request.layers.len(),
        clip_count: request.clips.len(),
    })
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), LayeredError> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}
