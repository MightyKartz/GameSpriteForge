use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

const GSFPACK_SCHEMA: &str = include_str!("../../../schemas/gsfpack.schema.json");
const MANIFEST_SCHEMA: &str = include_str!("../../../schemas/manifest.schema.json");
const ATLAS_SCHEMA: &str = include_str!("../../../schemas/atlas.schema.json");
const QUALITY_REPORT_SCHEMA: &str = include_str!("../../../schemas/quality-report.schema.json");

const REQUIRED_FILES: &[&str] = &[
    "forgepack.json",
    "previews/preview.gif",
    "assets/frames",
    "assets/sprite_sheet.png",
    "assets/atlas.json",
    "assets/manifest.json",
    "quality-report.json",
];

const LEGACY_SPRITE_SHEET_IMAGE: &str = "sprite_sheet.png";

#[derive(Debug, thiserror::Error)]
pub enum PackError {
    #[error("pack path does not exist: {0}")]
    MissingPack(PathBuf),
    #[error("pack path is not a directory: {0}")]
    NotDirectory(PathBuf),
    #[error("required pack file is missing: {0}")]
    MissingFile(String),
    #[error("invalid pack asset path: {0}")]
    InvalidAssetPath(String),
    #[error("pack contains no frame pngs")]
    NoFrames,
    #[error("forgepack asset path mismatch for {field}: expected {expected}, got {actual}")]
    AssetPathMismatch {
        field: String,
        expected: String,
        actual: String,
    },
    #[error("schema compile failed for {schema}: {message}")]
    SchemaCompile { schema: String, message: String },
    #[error("schema validation failed for {document}: {message}")]
    SchemaValidation { document: String, message: String },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub frame_count: usize,
    pub preview_gif: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackInspectSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub frame_count: usize,
    pub preview_gif: String,
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub atlas_path: PathBuf,
    pub quality_report_path: PathBuf,
    pub default_animation: String,
    pub animations: Vec<PackAnimationSummary>,
    pub asset_type: String,
    pub items: Vec<PackItemSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackItemSummary {
    pub id: String,
    pub name: String,
    pub frame: usize,
    pub texture: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackAnimationSummary {
    pub name: String,
    pub frame_count: usize,
    pub fps: f32,
    #[serde(rename = "loop")]
    pub loop_animation: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForgePackJson {
    schema_version: String,
    #[serde(default)]
    asset_type: Option<String>,
    id: String,
    name: String,
    version: String,
    previews: PackPreviews,
    assets: PackAssets,
    #[serde(default)]
    items: Vec<PackItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackItem {
    id: String,
    texture: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackPreviews {
    #[serde(default)]
    gif: Option<String>,
    #[serde(default)]
    image: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackAssets {
    frames: Option<String>,
    sprite_sheet: Option<String>,
    atlas: Option<String>,
    manifest: String,
    godot_helper: Option<String>,
    quality_report: String,
    consistency_report: Option<String>,
    terrain_manifest: Option<String>,
    building_manifest: Option<String>,
    map_manifest: Option<String>,
    map_layout: Option<String>,
    validation_report: Option<String>,
    atlas_image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportedPack {
    pub summary: PackSummary,
    pub root: PathBuf,
    pub frame_paths: Vec<PathBuf>,
    pub forgepack: serde_json::Value,
    pub manifest: serde_json::Value,
    pub atlas: serde_json::Value,
    pub quality_report: serde_json::Value,
}

pub fn validate_pack_layout(pack_path: &Path) -> Result<(), PackError> {
    require_pack_root_directory(pack_path)?;

    require_regular_pack_file(pack_path, "forgepack.json")?;
    let metadata: ForgePackJson =
        serde_json::from_slice(&fs::read(pack_path.join("forgepack.json"))?)?;
    if metadata.schema_version == "3.0.0" {
        return validate_world_pack_layout(pack_path, &metadata);
    }

    for relative in REQUIRED_FILES {
        if *relative == "assets/frames" {
            require_regular_pack_directory(pack_path, relative)?;
        } else {
            require_regular_pack_file(pack_path, relative)?;
        }
    }

    expect_asset_path(
        "previews.gif",
        "previews/preview.gif",
        metadata.previews.gif.as_deref().unwrap_or_default(),
    )?;
    expect_asset_path(
        "assets.frames",
        "assets/frames",
        metadata.assets.frames.as_deref().unwrap_or_default(),
    )?;
    expect_asset_path(
        "assets.spriteSheet",
        "assets/sprite_sheet.png",
        metadata.assets.sprite_sheet.as_deref().unwrap_or_default(),
    )?;
    expect_asset_path(
        "assets.atlas",
        "assets/atlas.json",
        metadata.assets.atlas.as_deref().unwrap_or_default(),
    )?;
    expect_asset_path(
        "assets.manifest",
        "assets/manifest.json",
        &metadata.assets.manifest,
    )?;
    if let Some(godot_helper) = metadata.assets.godot_helper.as_deref() {
        expect_asset_path(
            "assets.godotHelper",
            "assets/godot_import.json",
            godot_helper,
        )?;
        require_regular_pack_file(pack_path, godot_helper)?;
    }
    expect_asset_path(
        "assets.qualityReport",
        "quality-report.json",
        &metadata.assets.quality_report,
    )?;
    if metadata.schema_version == "2.0.0" {
        let consistency = metadata
            .assets
            .consistency_report
            .as_deref()
            .ok_or_else(|| PackError::MissingFile("assets.consistencyReport".into()))?;
        expect_asset_path(
            "assets.consistencyReport",
            "consistency-report.json",
            consistency,
        )?;
        require_regular_pack_file(pack_path, consistency)?;
        if matches!(
            metadata.asset_type.as_deref(),
            Some("icon_set" | "prop_set")
        ) {
            if metadata.items.is_empty() {
                return Err(PackError::MissingFile("items".into()));
            }
            for item in &metadata.items {
                let expected = format!("assets/items/{}.png", item.id);
                expect_asset_path("items.texture", &expected, &item.texture)?;
                require_regular_pack_file(pack_path, &item.texture)?;
            }
        }
    }

    if frame_pngs(pack_path)?.is_empty() {
        return Err(PackError::NoFrames);
    }

    validate_json_file(
        pack_path.join("forgepack.json"),
        "gsfpack.schema.json",
        GSFPACK_SCHEMA,
    )?;
    validate_json_file(
        pack_path.join("assets/manifest.json"),
        "manifest.schema.json",
        MANIFEST_SCHEMA,
    )?;
    validate_json_file(
        pack_path.join("assets/atlas.json"),
        "atlas.schema.json",
        ATLAS_SCHEMA,
    )?;
    validate_atlas_images(pack_path)?;
    validate_json_file(
        pack_path.join("quality-report.json"),
        "quality-report.schema.json",
        QUALITY_REPORT_SCHEMA,
    )?;

    Ok(())
}

pub fn import_pack(pack_path: &Path) -> Result<ImportedPack, PackError> {
    validate_pack_layout(pack_path)?;
    let summary = read_pack_summary(pack_path)?;
    let metadata: ForgePackJson =
        serde_json::from_slice(&fs::read(pack_path.join("forgepack.json"))?)?;

    Ok(ImportedPack {
        summary,
        root: pack_path.to_path_buf(),
        frame_paths: if metadata.schema_version == "3.0.0" {
            vec![]
        } else {
            frame_pngs(pack_path)?
        },
        forgepack: read_json(pack_path.join("forgepack.json"))?,
        manifest: read_json(pack_path.join("assets/manifest.json"))?,
        atlas: if pack_path.join("assets/atlas.json").is_file() {
            read_json(pack_path.join("assets/atlas.json"))?
        } else {
            serde_json::Value::Null
        },
        quality_report: read_json(pack_path.join("quality-report.json"))?,
    })
}

pub fn read_pack_summary(pack_path: &Path) -> Result<PackSummary, PackError> {
    validate_pack_layout(pack_path)?;

    let metadata: ForgePackJson =
        serde_json::from_slice(&fs::read(pack_path.join("forgepack.json"))?)?;
    let frame_count = if metadata.schema_version == "3.0.0" {
        0
    } else {
        frame_pngs(pack_path)?.len()
    };
    let preview = metadata
        .previews
        .gif
        .or(metadata.previews.image)
        .unwrap_or_default();

    Ok(PackSummary {
        id: metadata.id,
        name: metadata.name,
        version: metadata.version,
        frame_count,
        preview_gif: preview,
    })
}

pub fn inspect_pack(pack_path: &Path) -> Result<PackInspectSummary, PackError> {
    let summary = read_pack_summary(pack_path)?;
    let metadata: ForgePackJson =
        serde_json::from_slice(&fs::read(pack_path.join("forgepack.json"))?)?;
    let manifest = read_json(pack_path.join("assets/manifest.json"))?;
    let animations = manifest
        .get("animations")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|animation| {
            Some(PackAnimationSummary {
                name: animation.get("name")?.as_str()?.to_string(),
                frame_count: animation.get("frames")?.as_array()?.len(),
                fps: animation
                    .get("fps")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(12.0) as f32,
                loop_animation: animation
                    .get("loop")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(true),
            })
        })
        .collect::<Vec<_>>();
    let asset_type = manifest
        .get("assetType")
        .and_then(|value| value.as_str())
        .or(metadata.asset_type.as_deref())
        .or_else(|| summary.name.is_empty().then_some("animation"))
        .unwrap_or(if animations.len() > 1 {
            "character"
        } else {
            "animation"
        })
        .to_string();
    let items = manifest
        .get("items")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|item| {
            Some(PackItemSummary {
                id: item.get("id")?.as_str()?.to_string(),
                name: item.get("name")?.as_str()?.to_string(),
                frame: item.get("frame")?.as_u64()? as usize,
                texture: item.get("texture")?.as_str()?.to_string(),
            })
        })
        .collect::<Vec<_>>();
    let default_animation = animations
        .first()
        .map(|animation| animation.name.clone())
        .unwrap_or_else(|| "idle".to_string());

    Ok(PackInspectSummary {
        id: summary.id,
        name: summary.name,
        version: summary.version,
        frame_count: summary.frame_count,
        preview_gif: summary.preview_gif,
        root: pack_path.to_path_buf(),
        manifest_path: pack_path.join("assets/manifest.json"),
        atlas_path: metadata
            .assets
            .atlas
            .or(metadata.assets.atlas_image)
            .map(|path| pack_path.join(path))
            .unwrap_or_else(|| pack_path.join("assets/manifest.json")),
        quality_report_path: pack_path.join("quality-report.json"),
        default_animation,
        animations,
        asset_type,
        items,
    })
}

fn validate_world_pack_layout(pack_path: &Path, metadata: &ForgePackJson) -> Result<(), PackError> {
    let asset_type = metadata
        .asset_type
        .as_deref()
        .ok_or_else(|| PackError::MissingFile("assetType".into()))?;
    if !matches!(asset_type, "terrain_set" | "building_kit" | "map") {
        return Err(PackError::SchemaValidation {
            document: "forgepack.json".into(),
            message: format!("unsupported V3 assetType {asset_type}"),
        });
    }
    expect_asset_path(
        "assets.manifest",
        "assets/manifest.json",
        &metadata.assets.manifest,
    )?;
    require_regular_pack_file(pack_path, &metadata.assets.manifest)?;
    expect_asset_path(
        "assets.qualityReport",
        "quality-report.json",
        &metadata.assets.quality_report,
    )?;
    require_regular_pack_file(pack_path, &metadata.assets.quality_report)?;
    let helper = metadata
        .assets
        .godot_helper
        .as_deref()
        .ok_or_else(|| PackError::MissingFile("assets.godotHelper".into()))?;
    expect_asset_path("assets.godotHelper", "assets/godot_import.json", helper)?;
    require_regular_pack_file(pack_path, helper)?;
    let preview = metadata
        .previews
        .image
        .as_deref()
        .ok_or_else(|| PackError::MissingFile("previews.image".into()))?;
    expect_asset_path("previews.image", "preview.png", preview)?;
    require_regular_pack_file(pack_path, preview)?;

    match asset_type {
        "terrain_set" => {
            let manifest = metadata
                .assets
                .terrain_manifest
                .as_deref()
                .ok_or_else(|| PackError::MissingFile("assets.terrainManifest".into()))?;
            expect_asset_path(
                "assets.terrainManifest",
                "assets/terrain-manifest.json",
                manifest,
            )?;
            require_regular_pack_file(pack_path, manifest)?;
            let atlas = metadata
                .assets
                .atlas_image
                .as_deref()
                .ok_or_else(|| PackError::MissingFile("assets.atlasImage".into()))?;
            expect_asset_path("assets.atlasImage", "assets/terrain-atlas.png", atlas)?;
            require_regular_pack_file(pack_path, atlas)?;
        }
        "building_kit" => {
            let manifest = metadata
                .assets
                .building_manifest
                .as_deref()
                .ok_or_else(|| PackError::MissingFile("assets.buildingManifest".into()))?;
            expect_asset_path(
                "assets.buildingManifest",
                "assets/building-manifest.json",
                manifest,
            )?;
            require_regular_pack_file(pack_path, manifest)?;
            let atlas = metadata
                .assets
                .atlas_image
                .as_deref()
                .ok_or_else(|| PackError::MissingFile("assets.atlasImage".into()))?;
            expect_asset_path("assets.atlasImage", "assets/building-atlas.png", atlas)?;
            require_regular_pack_file(pack_path, atlas)?;
        }
        "map" => {
            for (field, expected, actual) in [
                (
                    "assets.mapManifest",
                    "assets/map-manifest.json",
                    metadata.assets.map_manifest.as_deref(),
                ),
                (
                    "assets.mapLayout",
                    "assets/map-layout.json",
                    metadata.assets.map_layout.as_deref(),
                ),
                (
                    "assets.validationReport",
                    "validation-report.json",
                    metadata.assets.validation_report.as_deref(),
                ),
            ] {
                let actual = actual.ok_or_else(|| PackError::MissingFile(field.into()))?;
                expect_asset_path(field, expected, actual)?;
                require_regular_pack_file(pack_path, actual)?;
            }
            require_regular_pack_directory(pack_path, "assets/dependencies")?;
            require_regular_pack_directory(pack_path, "assets/runtime")?;
        }
        _ => unreachable!(),
    }

    validate_json_file(
        pack_path.join("forgepack.json"),
        "gsfpack.schema.json",
        GSFPACK_SCHEMA,
    )?;
    Ok(())
}

fn expect_asset_path(field: &str, expected: &str, actual: &str) -> Result<(), PackError> {
    if actual == expected {
        Ok(())
    } else {
        Err(PackError::AssetPathMismatch {
            field: field.to_string(),
            expected: expected.to_string(),
            actual: actual.to_string(),
        })
    }
}

fn require_pack_root_directory(pack_path: &Path) -> Result<(), PackError> {
    let metadata = fs::symlink_metadata(pack_path).map_err(|error| {
        if error.kind() == ErrorKind::NotFound {
            PackError::MissingPack(pack_path.to_path_buf())
        } else {
            PackError::Io(error)
        }
    })?;

    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(PackError::NotDirectory(pack_path.to_path_buf()));
    }

    Ok(())
}

fn require_regular_pack_file(pack_path: &Path, relative: &str) -> Result<(), PackError> {
    require_regular_pack_entry(pack_path, relative, PackEntryKind::File)
}

fn require_regular_pack_directory(pack_path: &Path, relative: &str) -> Result<(), PackError> {
    require_regular_pack_entry(pack_path, relative, PackEntryKind::Directory)
}

fn require_regular_pack_entry(
    pack_path: &Path,
    relative: &str,
    kind: PackEntryKind,
) -> Result<(), PackError> {
    let root = fs::canonicalize(pack_path)?;
    let path = pack_path.join(relative);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Err(PackError::MissingFile(relative.to_string()));
        }
        Err(error) => return Err(PackError::Io(error)),
    };

    if metadata.file_type().is_symlink() {
        return Err(PackError::InvalidAssetPath(relative.to_string()));
    }

    if !kind.matches(&metadata) {
        return Err(PackError::MissingFile(relative.to_string()));
    }

    let canonical = fs::canonicalize(&path)?;
    if !canonical.starts_with(&root) {
        return Err(PackError::InvalidAssetPath(relative.to_string()));
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum PackEntryKind {
    File,
    Directory,
}

impl PackEntryKind {
    fn matches(self, metadata: &fs::Metadata) -> bool {
        match self {
            PackEntryKind::File => metadata.is_file(),
            PackEntryKind::Directory => metadata.is_dir(),
        }
    }
}

fn frame_pngs(pack_path: &Path) -> Result<Vec<PathBuf>, PackError> {
    require_regular_pack_directory(pack_path, "assets/frames")?;
    let mut paths = fs::read_dir(pack_path.join("assets/frames"))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            if !entry.file_type().ok()?.is_file() {
                return None;
            }
            let path = entry.path();
            path.extension()
                .and_then(|value| value.to_str())
                .map(|value| value.eq_ignore_ascii_case("png"))
                .unwrap_or(false)
                .then_some(path)
        })
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

fn read_json(path: PathBuf) -> Result<serde_json::Value, PackError> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn validate_atlas_images(pack_path: &Path) -> Result<(), PackError> {
    let atlas = read_json(pack_path.join("assets/atlas.json"))?;
    let mut page_images = None;

    if let Some(image) = atlas.get("image").and_then(|value| value.as_str()) {
        validate_page_image_name(image)?;
        if image != LEGACY_SPRITE_SHEET_IMAGE {
            return Err(PackError::InvalidAssetPath(image.to_string()));
        }
        require_page_image_file(pack_path, image)?;
    }

    if let Some(images) = atlas.get("images").and_then(|value| value.as_array()) {
        let mut validated_images = HashSet::new();
        for image in images {
            let Some(image) = image.as_str() else {
                continue;
            };
            validate_page_image_name(image)?;
            require_page_image_file(pack_path, image)?;
            validated_images.insert(image.to_string());
        }
        page_images = Some(validated_images);
    }

    if let Some(frames) = atlas.get("frames").and_then(|value| value.as_array()) {
        for frame in frames {
            let Some(image) = frame.get("image").and_then(|value| value.as_str()) else {
                continue;
            };
            validate_page_image_name(image)?;
            if let Some(images) = page_images.as_ref() {
                if !images.contains(image) {
                    return Err(PackError::InvalidAssetPath(image.to_string()));
                }
            } else {
                require_page_image_file(pack_path, image)?;
            }
        }
    }

    Ok(())
}

fn validate_page_image_name(image: &str) -> Result<(), PackError> {
    let path = Path::new(image);
    let has_png_extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("png"))
        .unwrap_or(false);

    if image.is_empty()
        || image.contains('/')
        || image.contains('\\')
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || path.file_name().and_then(|value| value.to_str()) != Some(image)
        || !has_png_extension
    {
        return Err(PackError::InvalidAssetPath(image.to_string()));
    }

    Ok(())
}

fn require_page_image_file(pack_path: &Path, image: &str) -> Result<(), PackError> {
    let relative = format!("assets/{image}");
    require_regular_pack_file(pack_path, &relative)
}

fn validate_json_file(
    document_path: PathBuf,
    schema_name: &str,
    schema_source: &str,
) -> Result<(), PackError> {
    let schema = serde_json::from_str::<serde_json::Value>(schema_source)?;
    let document = read_json(document_path.clone())?;
    let validator =
        jsonschema::validator_for(&schema).map_err(|error| PackError::SchemaCompile {
            schema: schema_name.to_string(),
            message: error.to_string(),
        })?;

    if let Some(error) = validator.iter_errors(&document).next() {
        return Err(PackError::SchemaValidation {
            document: document_path.display().to_string(),
            message: error.to_string(),
        });
    }

    Ok(())
}
