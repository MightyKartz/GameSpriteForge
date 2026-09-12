//! Flat, shared-canvas layer contract. Coordinates are source pixels, transforms
//! are absolute local values around the declared pivot, and array order draws
//! back to front. V1 deliberately has no mesh, bones, parent hierarchy or scripts.
use std::{collections::HashSet, fs, io::Cursor, path::Path};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{PackError, PackInspectSummary};

pub const GODOT_LAYERED_CONTROLLER: &str =
    include_str!("../../../scripts/godot/runtime/layered-player-v1.gd");
/// Frozen source and generator dispatch for already delivered native resources.
/// Future profiles retain the V1 snapshot and scene generator for validation.
pub const LAYERED_RUNTIME_PROFILE_V1: &str = "godot-layered@1.0.0";
pub const GODOT_ALPHA_MULTIPLY_SHADER_V1: &str =
    include_str!("../../../scripts/godot/runtime/alpha-multiply-v1.gdshader");
const LAYERED_SCHEMA: &str = include_str!("../../../schemas/layered-manifest.schema.json");
pub const MAX_TOTAL_PIXELS: u64 = 64 * 1024 * 1024;
pub const MAX_PNG_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayeredCanvas {
    pub width: u32,
    pub height: u32,
    pub origin: [f64; 2],
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LayeredSampling {
    Nearest,
    Linear,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LayerBlend {
    #[default]
    Normal,
    Add,
    Multiply,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayerTransform {
    pub position: [f64; 2],
    pub rotation_degrees: f64,
    pub scale: [f64; 2],
    pub opacity: f64,
}

impl Default for LayerTransform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0],
            rotation_degrees: 0.0,
            scale: [1.0, 1.0],
            opacity: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayeredLayer {
    pub id: String,
    pub name: String,
    pub texture: String,
    pub sha256: String,
    pub pivot: [f64; 2],
    pub transform: LayerTransform,
    pub blend: LayerBlend,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayerKeyframe {
    pub time_ms: u32,
    pub transform: LayerTransform,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayerTrack {
    pub layer_id: String,
    pub keyframes: Vec<LayerKeyframe>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayeredClip {
    pub id: String,
    pub duration_ms: u32,
    #[serde(rename = "loop")]
    pub loop_animation: bool,
    pub tracks: Vec<LayerTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayeredManifest {
    pub schema_version: String,
    pub asset_type: String,
    pub id: String,
    pub name: String,
    pub canvas: LayeredCanvas,
    pub sampling: LayeredSampling,
    pub layers: Vec<LayeredLayer>,
    pub clips: Vec<LayeredClip>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_clip: Option<String>,
}

fn invalid(message: impl Into<String>) -> PackError {
    PackError::SchemaValidation {
        document: "assets/manifest.json".into(),
        message: message.into(),
    }
}

/// Restrict generated filenames and Godot node names to one portable component.
pub fn valid_id(id: &str) -> bool {
    let Some(first) = id.as_bytes().first() else {
        return false;
    };
    let upper = id.to_ascii_uppercase();
    first.is_ascii_alphabetic()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        && !matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        && !(upper.len() == 4
            && (upper.starts_with("COM") || upper.starts_with("LPT"))
            && matches!(upper.as_bytes()[3], b'1'..=b'9'))
}

pub fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn validate_transform(transform: &LayerTransform) -> Result<(), PackError> {
    if transform
        .position
        .iter()
        .any(|n| !n.is_finite() || n.abs() > 1_000_000.0)
        || !transform.rotation_degrees.is_finite()
        || transform.rotation_degrees.abs() > 360_000.0
        || transform
            .scale
            .iter()
            .any(|n| !n.is_finite() || !(0.0001..=1000.0).contains(n))
        || !transform.opacity.is_finite()
        || !(0.0..=1.0).contains(&transform.opacity)
    {
        return Err(invalid("layer transform must have finite bounded position/rotation, positive scale and opacity in 0..=1"));
    }
    Ok(())
}

/// Validate the portable semantic contract without reading files or running Godot.
pub fn validate_manifest(manifest: &LayeredManifest) -> Result<(), PackError> {
    let canvas = &manifest.canvas;
    if manifest.schema_version != "1"
        || manifest.asset_type != "layered"
        || !valid_id(&manifest.id)
        || manifest.name.trim().is_empty()
        || manifest.name.len() > 256
    {
        return Err(invalid(
            "expected layered manifest V1 with a portable id and name",
        ));
    }
    if !(1..=4096).contains(&canvas.width)
        || !(1..=4096).contains(&canvas.height)
        || canvas.origin != [0.0, 0.0]
        || manifest.layers.is_empty()
        || manifest.layers.len() > 64
        || u64::from(canvas.width) * u64::from(canvas.height) * manifest.layers.len() as u64
            > MAX_TOTAL_PIXELS
    {
        return Err(invalid("layered V1 requires 1..=64 layers on a shared 1..=4096 pixel canvas, origin [0,0], at most 64 Mi pixels total"));
    }
    let mut layer_ids = HashSet::new();
    for layer in &manifest.layers {
        if !valid_id(&layer.id)
            || !layer_ids.insert(layer.id.to_ascii_lowercase())
            || layer.name.trim().is_empty()
            || layer.name.len() > 256
            || layer.texture != format!("assets/layers/{}.png", layer.id)
            || !valid_sha256(&layer.sha256)
        {
            return Err(invalid(format!(
                "invalid/duplicate layer id, name, texture or SHA256: {}",
                layer.id
            )));
        }
        if layer.pivot.iter().any(|n| !n.is_finite())
            || !(0.0..=f64::from(canvas.width)).contains(&layer.pivot[0])
            || !(0.0..=f64::from(canvas.height)).contains(&layer.pivot[1])
        {
            return Err(invalid(format!(
                "pivot must be inside the shared canvas: {}",
                layer.id
            )));
        }
        validate_transform(&layer.transform)?;
    }
    if manifest.clips.len() > 64 {
        return Err(invalid("at most 64 layered clips are supported"));
    }
    let exact_ids = manifest
        .layers
        .iter()
        .map(|layer| layer.id.as_str())
        .collect::<HashSet<_>>();
    let mut clip_ids = HashSet::new();
    let mut total_keyframes = 0usize;
    for clip in &manifest.clips {
        if !valid_id(&clip.id)
            || !clip_ids.insert(clip.id.to_ascii_lowercase())
            || !(1..=3_600_000).contains(&clip.duration_ms)
            || clip.tracks.is_empty()
            || clip.tracks.len() > manifest.layers.len()
        {
            return Err(invalid(format!(
                "invalid clip id, duration or tracks: {}",
                clip.id
            )));
        }
        let mut tracks = HashSet::new();
        for track in &clip.tracks {
            total_keyframes += track.keyframes.len();
            if total_keyframes > 65_536 {
                return Err(invalid(
                    "at most 65536 keyframes are supported per layered Pack",
                ));
            }
            if !exact_ids.contains(track.layer_id.as_str())
                || !tracks.insert(&track.layer_id)
                || !(2..=1000).contains(&track.keyframes.len())
                || track.keyframes.first().map(|key| key.time_ms) != Some(0)
                || track.keyframes.last().map(|key| key.time_ms) != Some(clip.duration_ms)
            {
                return Err(invalid(format!("clip {} needs unique known layers and 2..=1000 keyframes including both endpoints", clip.id)));
            }
            for pair in track.keyframes.windows(2) {
                if pair[0].time_ms >= pair[1].time_ms {
                    return Err(invalid("keyframe times must increase strictly"));
                }
            }
            for keyframe in &track.keyframes {
                validate_transform(&keyframe.transform)?;
            }
        }
    }
    if let Some(default) = &manifest.default_clip {
        if !manifest.clips.iter().any(|clip| &clip.id == default) {
            return Err(invalid("defaultClip must identify an existing clip"));
        }
    }
    Ok(())
}

/// Decode a PNG only after bounding its header dimensions and encoded size.
/// Alpha-bearing full-canvas backgrounds and completely transparent layers are
/// legal: registration, not a detected foreground crop, defines a layer.
pub fn validate_texture_bytes(bytes: &[u8], canvas: &LayeredCanvas) -> Result<(), PackError> {
    if bytes.len() as u64 > MAX_PNG_BYTES
        || !(1..=4096).contains(&canvas.width)
        || !(1..=4096).contains(&canvas.height)
    {
        return Err(invalid("layer PNG exceeds encoded size or canvas bounds"));
    }
    let mut container = png::Decoder::new(Cursor::new(bytes))
        .read_info()
        .map_err(|e| invalid(format!("invalid layer PNG: {e}")))?;
    let info = container.info();
    let dimensions = (info.width, info.height);
    if dimensions != (canvas.width, canvas.height) {
        return Err(invalid(
            "layer PNG dimensions differ from the shared canvas",
        ));
    }
    if info.animation_control.is_some() {
        return Err(invalid(
            "layer textures must be still PNGs, not APNG animations",
        ));
    }
    // The image decoder can stop after decoded pixels. Consume through IEND as
    // well so truncated/corrupt terminal CRCs cannot pass with a matching hash.
    container
        .finish()
        .map_err(|e| invalid(format!("invalid layer PNG container: {e}")))?;
    let decoded = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .map_err(|e| invalid(format!("invalid layer PNG: {e}")))?;
    if !decoded.color().has_alpha() {
        return Err(invalid("layer PNG must have an alpha channel"));
    }
    Ok(())
}

/// Native resource graph is derived, never accepted as arbitrary user code.
pub fn godot_scene(manifest: &LayeredManifest) -> String {
    godot_scene_v1(manifest)
}

/// V1 scene serialization is a persisted contract, not a mutable UI template.
pub fn godot_scene_v1(manifest: &LayeredManifest) -> String {
    use std::fmt::Write;
    let mut scene = format!("[gd_scene load_steps={} format=3]\n\n[ext_resource type=\"Script\" path=\"forge_layered_player.gd\" id=\"1_player\"]\n[ext_resource type=\"Shader\" path=\"forge_alpha_multiply.gdshader\" id=\"shader_multiply\"]\n", manifest.layers.len() + 6);
    for (i, layer) in manifest.layers.iter().enumerate() {
        writeln!(
            scene,
            "[ext_resource type=\"Texture2D\" path=\"layers/{}.png\" id=\"tex_{}\"]",
            layer.id, i
        )
        .unwrap();
    }
    for (name, blend) in [("normal", 0), ("add", 1)] {
        writeln!(scene, "\n[sub_resource type=\"CanvasItemMaterial\" id=\"blend_{name}\"]\nblend_mode = {blend}").unwrap();
    }
    writeln!(scene, "\n[sub_resource type=\"ShaderMaterial\" id=\"blend_multiply\"]\nshader = ExtResource(\"shader_multiply\")").unwrap();
    writeln!(scene, "\n[node name=\"LayeredAsset\" type=\"Node2D\"]\ntexture_filter = {}\nscript = ExtResource(\"1_player\")\n\n[node name=\"Layers\" type=\"Node2D\" parent=\".\"]", match manifest.sampling { LayeredSampling::Nearest => 1, LayeredSampling::Linear => 2 }).unwrap();
    for (i, layer) in manifest.layers.iter().enumerate() {
        let texture_filter = match manifest.sampling {
            LayeredSampling::Nearest => 1,
            LayeredSampling::Linear => 2,
        };
        let t = &layer.transform;
        let blend = match layer.blend {
            LayerBlend::Normal => "normal",
            LayerBlend::Add => "add",
            LayerBlend::Multiply => "multiply",
        };
        writeln!(scene, "\n[node name=\"{}\" type=\"Node2D\" parent=\"Layers\"]\nposition = Vector2({}, {})\nrotation = {}\nscale = Vector2({}, {})\nmodulate = Color(1, 1, 1, {})\n\n[node name=\"Sprite\" type=\"Sprite2D\" parent=\"Layers/{}\"]\ntexture_filter = {texture_filter}\nposition = Vector2({}, {})\ntexture = ExtResource(\"tex_{}\")\ncentered = false\nmaterial = SubResource(\"blend_{}\")", layer.id, layer.pivot[0] + t.position[0], layer.pivot[1] + t.position[1], t.rotation_degrees.to_radians(), t.scale[0], t.scale[1], t.opacity, layer.id, -layer.pivot[0], -layer.pivot[1], i, blend).unwrap();
    }
    scene
}

pub fn godot_helper(manifest: &LayeredManifest) -> serde_json::Value {
    godot_helper_v1(manifest)
}

/// Frozen helper contract paired with the trusted V1 runtime profile.
pub fn godot_helper_v1(manifest: &LayeredManifest) -> serde_json::Value {
    serde_json::json!({
        "assetType":"layered", "profile":LAYERED_RUNTIME_PROFILE_V1, "manifest":"assets/manifest.json",
        "scene":"assets/layered.tscn", "controller":"assets/forge_layered_player.gd",
        "blendShader":"assets/forge_alpha_multiply.gdshader", "multiplySemantics":"alpha_aware_preserve_destination_alpha",
        "nodeType":"Node2D", "layerNodePrefix":"Layers/", "canvas":manifest.canvas,
        "sampling":manifest.sampling, "defaultClip":manifest.default_clip,
        "layerOrder":manifest.layers.iter().map(|layer| &layer.id).collect::<Vec<_>>(),
        "clips":manifest.clips,
    })
}

pub fn quality_report(manifest: &LayeredManifest) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion":"1", "profile":"layered-structure@1.0.0", "verdict":"review_required",
        "scope":"layered_structure_only", "layerCount":manifest.layers.len(), "clipCount":manifest.clips.len(),
        "canvas":manifest.canvas, "sourceCanvasPreserved":true, "verifiedTextureHashes":true,
        "notes":["Layer pixels are copied without trimming, resampling or matting.",
                 "Preview is an ordered layer contact sheet, not a composite or motion approval.",
                 "Native playback, seams, occlusion and artistic quality require separate review."]
    })
}

/// Every ancestor below the pack root must be a regular directory. This also
/// rejects Windows junctions even when their canonical target stays in the pack.
fn regular(pack: &Path, relative: &str, directory: bool) -> Result<(), PackError> {
    let root_metadata = fs::symlink_metadata(pack)?;
    if is_link(&root_metadata) || !root_metadata.is_dir() {
        return Err(PackError::InvalidAssetPath(pack.display().to_string()));
    }
    let mut current = pack.to_path_buf();
    let components = relative.split('/').collect::<Vec<_>>();
    for (index, part) in components.iter().enumerate() {
        if part.is_empty() || *part == "." || *part == ".." || part.contains(['\\', ':']) {
            return Err(PackError::InvalidAssetPath(relative.into()));
        }
        current.push(part);
        let metadata = fs::symlink_metadata(&current)?;
        let should_be_directory = index + 1 < components.len() || directory;
        if is_link(&metadata)
            || (should_be_directory && !metadata.is_dir())
            || (!should_be_directory && !metadata.is_file())
        {
            return Err(PackError::InvalidAssetPath(relative.into()));
        }
    }
    Ok(())
}

fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_type().is_symlink() || metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

pub(super) fn validate_pack(pack: &Path, metadata: &crate::ForgePackJson) -> Result<(), PackError> {
    if metadata.asset_type.as_deref() != Some("layered") {
        return Err(invalid("schemaVersion 4.0.0 is reserved for layered Packs"));
    }
    crate::validate_json_file(
        pack.join("forgepack.json"),
        "gsfpack.schema.json",
        crate::GSFPACK_SCHEMA,
    )?;
    let document = crate::read_json(pack.join("forgepack.json"))?;
    for (field, expected) in [
        ("manifest", "assets/manifest.json"),
        ("godotHelper", "assets/godot_import.json"),
        ("godotScene", "assets/layered.tscn"),
        ("godotController", "assets/forge_layered_player.gd"),
        ("godotBlendShader", "assets/forge_alpha_multiply.gdshader"),
        ("qualityReport", "quality-report.json"),
    ] {
        crate::expect_asset_path(
            &format!("assets.{field}"),
            expected,
            document["assets"][field].as_str().unwrap_or_default(),
        )?;
        regular(pack, expected, false)?;
    }
    crate::expect_asset_path(
        "previews.image",
        "previews/layers.png",
        document["previews"]["image"].as_str().unwrap_or_default(),
    )?;
    regular(pack, "previews/layers.png", false)?;
    crate::validate_json_file(
        pack.join("assets/manifest.json"),
        "layered-manifest.schema.json",
        LAYERED_SCHEMA,
    )?;
    let manifest: LayeredManifest =
        serde_json::from_slice(&fs::read(pack.join("assets/manifest.json"))?)?;
    validate_manifest(&manifest)?;
    if manifest.id != metadata.id || manifest.name != metadata.name {
        return Err(invalid("Pack id/name differs from layered manifest"));
    }
    let sources = serde_json::json!(manifest
        .layers
        .iter()
        .map(|layer| {
            serde_json::json!({"id":layer.id,"sha256":layer.sha256,"texture":layer.texture})
        })
        .collect::<Vec<_>>());
    if document["source"]["metadata"]["profile"] != "local-layered-import@1.0.0"
        || document["source"]["metadata"]["canvasPolicy"] != "preserve_source"
        || document["source"]["metadata"]["sources"] != sources
    {
        return Err(invalid(
            "layered source provenance differs from the delivered layer hashes",
        ));
    }
    let mut expected_pngs = HashSet::new();
    for layer in &manifest.layers {
        regular(pack, &layer.texture, false)?;
        let path = pack.join(&layer.texture);
        if fs::metadata(&path)?.len() > MAX_PNG_BYTES {
            return Err(invalid("layer PNG exceeds 32 MiB"));
        }
        let bytes = fs::read(path)?;
        if format!("{:x}", Sha256::digest(&bytes)) != layer.sha256 {
            return Err(invalid(format!(
                "layer texture SHA256 mismatch: {}",
                layer.id
            )));
        }
        validate_texture_bytes(&bytes, &manifest.canvas)?;
        expected_pngs.insert(format!("{}.png", layer.id));
    }
    regular(pack, "assets/layers", true)?;
    for entry in fs::read_dir(pack.join("assets/layers"))? {
        let name = entry?.file_name().to_string_lossy().to_string();
        // Godot's adjacent import receipts can exist after a native preview.
        let is_import_receipt = expected_pngs
            .iter()
            .any(|png| name == format!("{png}.import"));
        if !expected_pngs.contains(&name) && !is_import_receipt {
            return Err(invalid(format!("undeclared layer file: {name}")));
        }
        if is_import_receipt {
            regular(pack, &format!("assets/layers/{name}"), false)?;
        }
    }
    let helper = crate::read_json(pack.join("assets/godot_import.json"))?;
    match helper["profile"].as_str() {
        Some(LAYERED_RUNTIME_PROFILE_V1) => {
            if helper != godot_helper_v1(&manifest)
                || fs::read(pack.join("assets/layered.tscn"))?
                    != godot_scene_v1(&manifest).as_bytes()
                || fs::read(pack.join("assets/forge_layered_player.gd"))?
                    != GODOT_LAYERED_CONTROLLER.as_bytes()
                || fs::read(pack.join("assets/forge_alpha_multiply.gdshader"))?
                    != GODOT_ALPHA_MULTIPLY_SHADER_V1.as_bytes()
            {
                return Err(invalid("layered native scene, controller or helper differs from the frozen V1 runtime contract"));
            }
        }
        _ => return Err(invalid("unsupported layered runtime profile")),
    }
    let report = crate::read_json(pack.join("quality-report.json"))?;
    let expected_report = quality_report(&manifest);
    for field in [
        "schemaVersion",
        "profile",
        "verdict",
        "scope",
        "layerCount",
        "clipCount",
        "canvas",
        "sourceCanvasPreserved",
        "verifiedTextureHashes",
    ] {
        if report.get(field) != expected_report.get(field) {
            return Err(invalid(format!(
                "layered structural report has an invalid {field}"
            )));
        }
    }
    if let Some(notes) = report.get("notes") {
        if !notes
            .as_array()
            .is_some_and(|values| values.iter().all(serde_json::Value::is_string))
        {
            return Err(invalid(
                "structural report notes must be an array of strings",
            ));
        }
    }
    Ok(())
}

pub(super) fn add_inspection(
    pack: &Path,
    summary: &mut PackInspectSummary,
) -> Result<(), PackError> {
    let manifest: LayeredManifest =
        serde_json::from_slice(&fs::read(pack.join("assets/manifest.json"))?)?;
    summary.default_animation = manifest
        .default_clip
        .clone()
        .or_else(|| manifest.clips.first().map(|clip| clip.id.clone()))
        .unwrap_or_default();
    summary.layered = Some(manifest);
    Ok(())
}
