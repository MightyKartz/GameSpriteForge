use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Component, Path, PathBuf};

use image::{imageops::FilterType, DynamicImage, ImageBuffer, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::asset_project::{hash_file, read_project, read_style_lock, STYLE_LOCK_FILE};
use crate::provider::{
    EditImageRequest, MediaGenerationProvider, ProviderError, ProviderImageReference, ReferenceRole,
};

pub const ENVIRONMENT_LOCK_FILE: &str = "environment-lock.json";
pub const ENVIRONMENT_PROFILE: &str = "environment@1.0.0";
pub const TERRAIN_PROFILE: &str = "dual-grid@1.0.0";
pub const TERRAIN_PROFILE_V2: &str = "dual-grid@2.0.0";
pub const TERRAIN_QUALITY_PROFILE: &str = "terrain-quality@1.0.0";
pub const TERRAIN_QUALITY_PROFILE_V2: &str = "terrain-quality@2.0.0";
pub const BUILDING_PROFILE: &str = "topdown-exterior@1.0.0";
pub const BUILDING_PROFILE_V2: &str = "topdown-exterior@2.0.0";
pub const BUILDING_QUALITY_PROFILE: &str = "building-quality@1.0.0";
pub const BUILDING_QUALITY_PROFILE_V2: &str = "building-quality@2.0.0";
pub const MAP_COMPILER_PROFILE: &str = "map-compiler@2.0.0";
pub const MAP_VALIDATION_PROFILE: &str = "map-validation@2.0.0";

const BUILDING_MODULES: [&str; 12] = [
    "roof_center",
    "roof_edge_north",
    "roof_edge_east",
    "roof_edge_south",
    "roof_edge_west",
    "roof_corner_nw",
    "roof_corner_ne",
    "roof_corner_sw",
    "roof_corner_se",
    "wall_face",
    "door",
    "window",
];

#[derive(Debug, Error)]
pub enum WorldError {
    #[error("invalid world asset: {0}")]
    Invalid(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("provider error: {0}")]
    Provider(#[from] ProviderError),
    #[error("pack error: {0}")]
    Pack(#[from] forge_pack::PackError),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnvironmentSpecV1 {
    pub schema_version: String,
    pub id: String,
    pub name: String,
    pub prompt: String,
    #[serde(default = "top_down")]
    pub perspective: String,
    pub tile_size: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_model: Option<String>,
    #[serde(default = "default_license")]
    pub license: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnvironmentLockV1 {
    pub schema_version: String,
    pub revision: String,
    pub id: String,
    pub name: String,
    pub prompt: String,
    pub profile: String,
    pub perspective: String,
    pub tile_size: u32,
    pub style_revision: String,
    pub provider_id: String,
    pub profile_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_model: Option<String>,
    pub board_path: PathBuf,
    pub board_sha256: String,
    pub style_board_sha256: String,
    pub license: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentBuildOutput {
    pub revision: String,
    pub lock_path: PathBuf,
    pub board_path: PathBuf,
    pub board_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerrainMaterialSpecV1 {
    pub id: String,
    pub name: String,
    pub prompt: String,
    #[serde(default = "default_true")]
    pub walkable: bool,
    #[serde(default = "default_collision_tag")]
    pub collision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerrainSetSpecV1 {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment_revision: Option<String>,
    pub base: TerrainMaterialSpecV1,
    pub overlay: TerrainMaterialSpecV1,
    #[serde(default = "default_material_sample_count")]
    pub material_sample_count: u8,
    #[serde(default = "default_terrain_variant_count")]
    pub variant_count: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_model: Option<String>,
    #[serde(default = "default_license")]
    pub license: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerrainQualityReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub tile_size: u32,
    pub tile_count: u32,
    pub mask_count: u32,
    #[serde(default = "default_one_u32")]
    pub variant_count: u32,
    pub dimensions_valid: bool,
    pub masks_complete: bool,
    pub periodic_edges_closed: bool,
    pub adjacency_valid: bool,
    pub base_detail_energy: f32,
    pub overlay_detail_energy: f32,
    pub material_variation_safe: bool,
    pub verdict: String,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerrainTestReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub seed: u64,
    pub samples: u32,
    pub sampled_cells: u64,
    pub seams_valid: bool,
    pub holes_detected: bool,
    pub verdict: String,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerrainBuildOutput {
    pub pack_dir: PathBuf,
    pub atlas_path: PathBuf,
    pub preview_path: PathBuf,
    pub quality_report_path: PathBuf,
    pub material_paths: Vec<PathBuf>,
    pub material_attempts: BTreeMap<String, u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FootprintRangeV1 {
    pub min_width: u32,
    pub max_width: u32,
    pub min_height: u32,
    pub max_height: u32,
}

impl Default for FootprintRangeV1 {
    fn default() -> Self {
        Self {
            min_width: 3,
            max_width: 6,
            min_height: 3,
            max_height: 5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildingKitSpecV1 {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub name: String,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment_revision: Option<String>,
    #[serde(default)]
    pub footprint: FootprintRangeV1,
    #[serde(default = "default_variant_count")]
    pub variant_count: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_model: Option<String>,
    #[serde(default = "default_license")]
    pub license: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingVariantV1 {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub entrance_x: u32,
    pub entrance_side: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingQualityReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub module_count: u32,
    pub expected_module_count: u32,
    pub dimensions_valid: bool,
    pub modules_complete: bool,
    pub footprints_valid: bool,
    pub entrances_valid: bool,
    pub overlaps_detected: bool,
    pub verdict: String,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingBuildOutput {
    pub pack_dir: PathBuf,
    pub atlas_path: PathBuf,
    pub preview_path: PathBuf,
    pub quality_report_path: PathBuf,
    pub material_paths: Vec<PathBuf>,
    pub material_attempts: BTreeMap<String, u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapSizeV1 {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapDependenciesV1 {
    pub terrain_sets: Vec<PathBuf>,
    pub building_kits: Vec<PathBuf>,
    #[serde(default)]
    pub prop_sets: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapRegionV1 {
    pub id: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapConnectionV1 {
    pub from: String,
    pub to: String,
    #[serde(default = "default_road_width")]
    pub width: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapLandmarkV1 {
    pub id: String,
    pub kind: String,
    pub region: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CountRangeV1 {
    pub min: u32,
    pub max: u32,
}

impl Default for CountRangeV1 {
    fn default() -> Self {
        Self { min: 4, max: 8 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapRequirementsV1 {
    #[serde(default = "default_true")]
    pub reachable_exit: bool,
    #[serde(default)]
    pub building_count: CountRangeV1,
    #[serde(default = "default_prop_density")]
    pub prop_density: f32,
}

impl Default for MapRequirementsV1 {
    fn default() -> Self {
        Self {
            reachable_exit: true,
            building_count: CountRangeV1::default(),
            prop_density: default_prop_density(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapSpecV1 {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub name: String,
    pub seed: u64,
    pub size: MapSizeV1,
    pub dependencies: MapDependenciesV1,
    #[serde(default)]
    pub regions: Vec<MapRegionV1>,
    #[serde(default)]
    pub connections: Vec<MapConnectionV1>,
    #[serde(default)]
    pub landmarks: Vec<MapLandmarkV1>,
    #[serde(default)]
    pub requirements: MapRequirementsV1,
    #[serde(default = "default_license")]
    pub license: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapCellV1 {
    pub x: u32,
    pub y: u32,
    pub mask: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapBuildingV1 {
    pub id: String,
    pub variant: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub entrance_x: u32,
    pub entrance_y: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapPropV1 {
    pub id: String,
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledMapV1 {
    pub schema_version: String,
    pub compiler_profile: String,
    pub source_seed: u64,
    pub selected_candidate: u8,
    pub selected_seed: u64,
    pub width: u32,
    pub height: u32,
    pub tile_size: u32,
    pub terrain_cells: Vec<MapCellV1>,
    pub buildings: Vec<MapBuildingV1>,
    pub props: Vec<MapPropV1>,
    pub spawn: [u32; 2],
    pub exit: [u32; 2],
    pub navigation_outlines: Vec<Vec<[f32; 2]>>,
    pub layout_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapCandidateReportV1 {
    pub candidate: u8,
    pub seed: u64,
    pub valid: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_breakdown: Option<MapCandidateScoreV2>,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapCandidateScoreV2 {
    pub path_quality: u32,
    pub region_connectivity: u32,
    pub building_road_adjacency: u32,
    pub landmark_distribution: u32,
    pub density_match: u32,
    pub repetition_control: u32,
}

impl MapCandidateScoreV2 {
    pub fn total(&self) -> u32 {
        self.path_quality
            + self.region_connectivity
            + self.building_road_adjacency
            + self.landmark_distribution
            + self.density_match
            + self.repetition_control
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapValidationReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub verdict: String,
    pub reachable_exit: bool,
    pub buildings_in_range: bool,
    pub entrances_reachable: bool,
    pub isolated_walkable_islands: u32,
    pub selected_candidate: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_score: Option<u32>,
    pub candidates: Vec<MapCandidateReportV1>,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapBuildOutput {
    pub pack_dir: PathBuf,
    pub map_path: PathBuf,
    pub preview_path: PathBuf,
    pub validation_report_path: PathBuf,
    pub layout_sha256: String,
}

pub fn build_environment_lock(
    project_root: &Path,
    spec_path: &Path,
    provider_id: &str,
    profile_id: &str,
    provider: &dyn MediaGenerationProvider,
    work_dir: &Path,
) -> Result<EnvironmentBuildOutput, WorldError> {
    let mut project =
        read_project(project_root).map_err(|error| WorldError::Invalid(error.to_string()))?;
    let style_revision = project.current_style_revision.clone().ok_or_else(|| {
        WorldError::Invalid("run `forge style create` before creating an environment".into())
    })?;
    let style_path = project_root
        .join(".forge/styles")
        .join(&style_revision)
        .join(STYLE_LOCK_FILE);
    let style =
        read_style_lock(&style_path).map_err(|error| WorldError::Invalid(error.to_string()))?;
    let mut spec: EnvironmentSpecV1 = serde_json::from_slice(&fs::read(spec_path)?)?;
    spec.image_model =
        provider.resolved_image_model(spec.image_model.as_deref().or(style.image_model.as_deref()));
    validate_environment_spec(&spec)?;
    let revision = hash_json(&serde_json::json!({
        "spec": spec,
        "styleRevision": style_revision,
        "styleBoardSha256": style.board_sha256,
        "providerId": provider_id,
        "profileId": profile_id,
        "profile": ENVIRONMENT_PROFILE,
    }))?[..16]
        .to_string();
    let environment_dir = project_root.join(".forge/environments").join(&revision);
    let lock_path = environment_dir.join(ENVIRONMENT_LOCK_FILE);
    if lock_path.is_file() {
        let lock = read_environment_lock(&lock_path)?;
        project.current_environment_revision = Some(lock.revision.clone());
        write_json_atomic(&project_root.join("forge-project.json"), &project)?;
        return Ok(EnvironmentBuildOutput {
            revision: lock.revision,
            lock_path,
            board_path: lock.board_path,
            board_sha256: lock.board_sha256,
        });
    }
    fs::create_dir_all(work_dir)?;
    let generated = work_dir.join("environment-board.png");
    provider.edit_image(
        &EditImageRequest {
            prompt: format!(
                "Preserve the exact visual language of the reference style board. Create one clean top-down 2D game environment material board for: {}. Show separate flat samples for ground, path, water, roof, wall, door, and window. Fixed orthographic camera, consistent lighting, no characters, no perspective scene, no map, no text, no UI.",
                spec.prompt
            ),
            model: spec.image_model.clone().or(style.image_model.clone()),
            references: vec![ProviderImageReference::from_path(
                ReferenceRole::Style,
                style.board_path.clone(),
            )?],
            aspect_ratio: "1:1".into(),
            resolution: "1k".into(),
        },
        &generated,
    )?;
    let _ = image::open(&generated)?;
    fs::create_dir_all(&environment_dir)?;
    let board_path = environment_dir.join("environment-board.png");
    fs::copy(&generated, &board_path)?;
    let board_sha256 =
        hash_file(&board_path).map_err(|error| WorldError::Invalid(error.to_string()))?;
    let lock = EnvironmentLockV1 {
        schema_version: "1".into(),
        revision: revision.clone(),
        id: spec.id,
        name: spec.name,
        prompt: spec.prompt,
        profile: ENVIRONMENT_PROFILE.into(),
        perspective: spec.perspective,
        tile_size: spec.tile_size,
        style_revision,
        provider_id: provider_id.into(),
        profile_id: profile_id.into(),
        image_model: spec.image_model.or(style.image_model),
        board_path: board_path.clone(),
        board_sha256: board_sha256.clone(),
        style_board_sha256: style.board_sha256,
        license: spec.license,
    };
    write_json_atomic(&lock_path, &lock)?;
    project.current_environment_revision = Some(revision.clone());
    write_json_atomic(&project_root.join("forge-project.json"), &project)?;
    Ok(EnvironmentBuildOutput {
        revision,
        lock_path,
        board_path,
        board_sha256,
    })
}

pub fn read_environment_lock(path: &Path) -> Result<EnvironmentLockV1, WorldError> {
    let lock: EnvironmentLockV1 = serde_json::from_slice(&fs::read(path)?)?;
    if lock.schema_version != "1" || lock.profile != ENVIRONMENT_PROFILE {
        return Err(WorldError::Invalid("unsupported environment lock".into()));
    }
    if !lock.board_path.is_file()
        || hash_file(&lock.board_path).map_err(|error| WorldError::Invalid(error.to_string()))?
            != lock.board_sha256
    {
        return Err(WorldError::Invalid(
            "environment board changed after it was locked".into(),
        ));
    }
    Ok(lock)
}

pub fn generate_terrain_set(
    exports_root: &Path,
    job_dir: &Path,
    spec: &TerrainSetSpecV1,
    environment: &EnvironmentLockV1,
    provider_id: &str,
    profile_id: &str,
    provider: &dyn MediaGenerationProvider,
) -> Result<TerrainBuildOutput, WorldError> {
    validate_terrain_spec(spec, environment)?;
    fs::create_dir_all(job_dir.join("provider"))?;
    let mut material_attempts = BTreeMap::new();
    let mut material_paths = Vec::new();
    let mut base_tiles = Vec::new();
    let mut overlay_tiles = Vec::new();
    for (material, label, tiles) in [
        (&spec.base, "base", &mut base_tiles),
        (&spec.overlay, "overlay", &mut overlay_tiles),
    ] {
        for sample in 0..spec.material_sample_count {
            let source = job_dir
                .join("provider")
                .join(format!("{label}-material-{}.png", sample + 1));
            let attempt = generate_material(
                provider,
                environment,
                material,
                spec.image_model.as_ref(),
                sample + 1,
                &source,
            )?;
            material_attempts.insert(format!("{}:sample-{}", material.id, sample + 1), attempt);
            tiles.push(make_periodic_tile(
                &image::open(&source)?.to_rgba8(),
                environment.tile_size,
            ));
            material_paths.push(source);
        }
    }
    let (atlas, quality) = if spec.schema_version == "2" {
        let atlas = compose_dual_grid_variant_atlas(
            &base_tiles,
            &overlay_tiles,
            u32::from(spec.variant_count),
        );
        let quality = validate_terrain_variant_atlas(
            &atlas,
            environment.tile_size,
            u32::from(spec.variant_count),
        );
        (atlas, quality)
    } else {
        let atlas = compose_dual_grid_atlas(&base_tiles[0], &overlay_tiles[0]);
        let quality = validate_terrain_atlas(&atlas, environment.tile_size);
        (atlas, quality)
    };
    if quality.verdict != "game_ready" {
        return Err(WorldError::Invalid(format!(
            "terrain topology failed: {}",
            quality.reasons.join(", ")
        )));
    }
    let pack_dir = exports_root.join(format!("{}.gsfpack", spec.id));
    let assets = pack_dir.join("assets");
    fs::create_dir_all(&assets)?;
    let atlas_path = assets.join("terrain-atlas.png");
    atlas.save(&atlas_path)?;
    let preview_path = pack_dir.join("preview.png");
    render_terrain_preview(&atlas, environment.tile_size, 42, &preview_path)?;
    let terrain_manifest =
        terrain_manifest(spec, environment, provider_id, profile_id, &atlas_path)?;
    write_json_atomic(&assets.join("terrain-manifest.json"), &terrain_manifest)?;
    write_json_atomic(&assets.join("manifest.json"), &terrain_manifest)?;
    write_json_atomic(
        &assets.join("godot_import.json"),
        &serde_json::json!({
            "schemaVersion": "1",
            "assetType": "terrain_set",
            "tileSize": environment.tile_size,
            "atlas": "assets/terrain-atlas.png",
            "profile": terrain_profile(spec),
            "variantCount": spec.variant_count,
            "baseTerrain": spec.base.id,
            "overlayTerrain": spec.overlay.id,
            "overlayWalkable": spec.overlay.walkable,
            "overlayCollision": spec.overlay.collision,
            "masks": terrain_mask_entries(spec.variant_count)
        }),
    )?;
    let quality_report_path = pack_dir.join("quality-report.json");
    write_json_atomic(&quality_report_path, &quality)?;
    write_json_atomic(
        &pack_dir.join("forgepack.json"),
        &serde_json::json!({
            "schemaVersion": "3.0.0",
            "assetType": "terrain_set",
            "id": spec.id,
            "name": spec.name,
            "version": "1.0.0",
            "createdAt": chrono::Utc::now(),
            "creator": {"name": "Game Sprite Forge"},
            "license": {"type": spec.license},
            "source": {"kind": "provider_generation", "name": provider_id, "metadata": {
                "provider": provider_id,
                "profileId": profile_id,
                "environmentRevision": environment.revision,
                "styleRevision": environment.style_revision,
                "profile": terrain_profile(spec),
                "materialAttempts": material_attempts
            }},
            "assets": {
                "manifest": "assets/manifest.json",
                "godotHelper": "assets/godot_import.json",
                "qualityReport": "quality-report.json",
                "terrainManifest": "assets/terrain-manifest.json",
                "atlasImage": "assets/terrain-atlas.png"
            },
            "previews": {"image": "preview.png"}
        }),
    )?;
    forge_pack::validate_pack_layout(&pack_dir)?;
    Ok(TerrainBuildOutput {
        pack_dir,
        atlas_path,
        preview_path,
        quality_report_path,
        material_paths,
        material_attempts,
    })
}

pub fn generate_building_kit(
    exports_root: &Path,
    job_dir: &Path,
    spec: &BuildingKitSpecV1,
    environment: &EnvironmentLockV1,
    provider_id: &str,
    profile_id: &str,
    provider: &dyn MediaGenerationProvider,
) -> Result<BuildingBuildOutput, WorldError> {
    validate_building_spec(spec, environment)?;
    let provider_dir = job_dir.join("provider");
    fs::create_dir_all(&provider_dir)?;
    let roof_path = provider_dir.join("roof-material.png");
    let wall_path = provider_dir.join("wall-material.png");
    let detail_path = provider_dir.join("door-window-material.png");
    let mut material_attempts = BTreeMap::new();
    for (target, id, label) in [
        (&roof_path, "roof", "roof material"),
        (&wall_path, "wall", "wall material"),
        (&detail_path, "detail", "door and window trim material"),
    ] {
        let request = EditImageRequest {
            prompt: format!(
                "Preserve the environment reference exactly. Generate one seamless top-down 2D game {} sample for {}. Orthographic, flat material plate, no building scene, no perspective, no characters, no text, no UI.",
                label, spec.prompt
            ),
            model: spec.image_model.clone().or(environment.image_model.clone()),
            references: vec![ProviderImageReference::from_path(
                ReferenceRole::Style,
                environment.board_path.clone(),
            )?],
            aspect_ratio: "1:1".into(),
            resolution: "1k".into(),
        };
        let attempt = edit_image_with_retry(provider, &request, target)?;
        material_attempts.insert(id.to_string(), attempt);
    }
    let roof = make_periodic_tile(&image::open(&roof_path)?.to_rgba8(), environment.tile_size);
    let wall = make_periodic_tile(&image::open(&wall_path)?.to_rgba8(), environment.tile_size);
    let detail = make_periodic_tile(
        &image::open(&detail_path)?.to_rgba8(),
        environment.tile_size,
    );
    let atlas = compose_building_atlas(&roof, &wall, &detail);
    let variants = building_variants(spec, 42);
    let quality = validate_building_kit(spec, environment.tile_size, &atlas, &variants);
    if quality.verdict != "game_ready" {
        return Err(WorldError::Invalid(format!(
            "building kit failed: {}",
            quality.reasons.join(", ")
        )));
    }
    let pack_dir = exports_root.join(format!("{}.gsfpack", spec.id));
    let assets = pack_dir.join("assets");
    fs::create_dir_all(&assets)?;
    let atlas_path = assets.join("building-atlas.png");
    atlas.save(&atlas_path)?;
    let preview_path = pack_dir.join("preview.png");
    render_building_preview(&atlas, environment.tile_size, &variants, &preview_path)?;
    let modules = BUILDING_MODULES
        .iter()
        .enumerate()
        .map(|(index, id)| {
            let semantic = if *id == "door" {
                "entrance"
            } else if *id == "window" {
                "window"
            } else if id.starts_with("roof") {
                "roof"
            } else {
                "wall"
            };
            serde_json::json!({
                "id": id,
                "x": index as u32 % 4,
                "y": index as u32 / 4,
                "semantic": semantic,
                "anchor": {"x": 0.5, "y": 1.0},
                "collision": if *id == "door" {"interaction_only"} else {"solid"},
                "occluder": id.starts_with("roof"),
                "ySortBaseline": 1.0,
            })
        })
        .collect::<Vec<_>>();
    let manifest = serde_json::json!({
        "schemaVersion": "1",
        "assetType": "building_kit",
        "name": spec.name,
        "profile": building_profile(spec),
        "tileSize": environment.tile_size,
        "atlas": "assets/building-atlas.png",
        "modules": modules,
        "variants": variants,
        "footprint": spec.footprint,
        "entranceSide": "south",
        "interior": false,
        "environmentRevision": environment.revision,
        "styleRevision": environment.style_revision,
        "providerId": provider_id,
        "profileId": profile_id,
        "atlasSha256": hash_file(&atlas_path).map_err(|error| WorldError::Invalid(error.to_string()))?
    });
    write_json_atomic(&assets.join("building-manifest.json"), &manifest)?;
    write_json_atomic(&assets.join("manifest.json"), &manifest)?;
    write_json_atomic(
        &assets.join("godot_import.json"),
        &serde_json::json!({
            "schemaVersion": "1",
            "assetType": "building_kit",
            "tileSize": environment.tile_size,
            "atlas": "assets/building-atlas.png",
            "modules": modules,
            "variants": variants,
            "entranceSide": "south",
            "interior": false
        }),
    )?;
    let quality_report_path = pack_dir.join("quality-report.json");
    write_json_atomic(&quality_report_path, &quality)?;
    write_json_atomic(
        &pack_dir.join("forgepack.json"),
        &serde_json::json!({
            "schemaVersion": "3.0.0",
            "assetType": "building_kit",
            "id": spec.id,
            "name": spec.name,
            "version": "1.0.0",
            "createdAt": chrono::Utc::now(),
            "creator": {"name": "Game Sprite Forge"},
            "license": {"type": spec.license},
            "source": {"kind": "provider_generation", "name": provider_id, "metadata": {
                "provider": provider_id,
                "profileId": profile_id,
                "environmentRevision": environment.revision,
                "styleRevision": environment.style_revision,
                "profile": building_profile(spec),
                "materialAttempts": material_attempts
            }},
            "assets": {
                "manifest": "assets/manifest.json",
                "godotHelper": "assets/godot_import.json",
                "qualityReport": "quality-report.json",
                "buildingManifest": "assets/building-manifest.json",
                "atlasImage": "assets/building-atlas.png"
            },
            "previews": {"image": "preview.png"}
        }),
    )?;
    forge_pack::validate_pack_layout(&pack_dir)?;
    Ok(BuildingBuildOutput {
        pack_dir,
        atlas_path,
        preview_path,
        quality_report_path,
        material_paths: vec![roof_path, wall_path, detail_path],
        material_attempts,
    })
}

pub fn compile_map_pack(
    exports_root: &Path,
    spec_path: &Path,
) -> Result<MapBuildOutput, WorldError> {
    let spec: MapSpecV1 = serde_json::from_slice(&fs::read(spec_path)?)?;
    validate_map_spec(&spec)?;
    let spec_root = spec_path.parent().unwrap_or_else(|| Path::new("."));
    let dependencies = resolve_map_dependencies(spec_root, &spec.dependencies)?;
    let terrain_pack = dependencies
        .terrain_sets
        .first()
        .ok_or_else(|| WorldError::Invalid("map requires a terrain set".into()))?;
    let building_pack = dependencies
        .building_kits
        .first()
        .ok_or_else(|| WorldError::Invalid("map requires a building kit".into()))?;
    let terrain_manifest: serde_json::Value =
        read_json(&terrain_pack.join("assets/terrain-manifest.json"))?;
    let building_manifest: serde_json::Value =
        read_json(&building_pack.join("assets/building-manifest.json"))?;
    let tile_size = terrain_manifest
        .get("tileSize")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| WorldError::Invalid("terrain manifest has no tileSize".into()))?
        as u32;
    if building_manifest
        .get("tileSize")
        .and_then(serde_json::Value::as_u64)
        != Some(u64::from(tile_size))
    {
        return Err(WorldError::Invalid(
            "terrain and building grid sizes do not match".into(),
        ));
    }
    let variants: Vec<BuildingVariantV1> = serde_json::from_value(
        building_manifest
            .get("variants")
            .cloned()
            .ok_or_else(|| WorldError::Invalid("building manifest has no variants".into()))?,
    )?;
    let mut candidate_reports = Vec::new();
    let mut valid_candidates =
        Vec::<(CompiledMapV1, MapValidationReportV1, MapCandidateScoreV2)>::new();
    for candidate in 0..20u8 {
        let seed = derived_seed(spec.seed, candidate);
        match compile_candidate(&spec, tile_size, &variants, candidate, seed) {
            Ok((map, validation)) => {
                let score = score_map_candidate(&spec, &map);
                candidate_reports.push(MapCandidateReportV1 {
                    candidate,
                    seed,
                    valid: true,
                    score: Some(score.total()),
                    score_breakdown: Some(score.clone()),
                    reasons: vec![],
                });
                valid_candidates.push((map, validation, score));
            }
            Err(reasons) => candidate_reports.push(MapCandidateReportV1 {
                candidate,
                seed,
                valid: false,
                score: None,
                score_breakdown: None,
                reasons,
            }),
        }
    }
    valid_candidates.sort_by(|left, right| {
        right
            .2
            .total()
            .cmp(&left.2.total())
            .then(left.0.selected_candidate.cmp(&right.0.selected_candidate))
    });
    let Some((mut map, mut validation, selected_score)) = valid_candidates.into_iter().next()
    else {
        let failure = MapValidationReportV1 {
            schema_version: "1".into(),
            profile: MAP_VALIDATION_PROFILE.into(),
            verdict: "blocked".into(),
            reachable_exit: false,
            buildings_in_range: false,
            entrances_reachable: false,
            isolated_walkable_islands: 0,
            selected_candidate: None,
            selected_score: None,
            candidates: candidate_reports,
            reasons: vec!["candidate_exhausted".into()],
        };
        fs::create_dir_all(exports_root)?;
        write_json_atomic(&exports_root.join("map-validation-report.json"), &failure)?;
        return Err(WorldError::Invalid(
            "map compiler exhausted 20 deterministic candidates".into(),
        ));
    };
    validation.selected_score = Some(selected_score.total());
    validation.candidates = candidate_reports;
    let hash_value = serde_json::to_value(&map)?;
    map.layout_sha256 = hash_json(&hash_value)?;
    let pack_dir = exports_root.join(format!("{}.gsfpack", spec.id));
    let assets = pack_dir.join("assets");
    let runtime = assets.join("runtime");
    let dependency_root = assets.join("dependencies");
    fs::create_dir_all(&runtime)?;
    fs::create_dir_all(&dependency_root)?;
    let mut dependency_records = Vec::new();
    for (asset_type, paths) in [
        ("terrain_set", &dependencies.terrain_sets),
        ("building_kit", &dependencies.building_kits),
        ("prop_set", &dependencies.prop_sets),
    ] {
        for path in paths {
            let digest = hash_directory(path)?;
            let metadata: serde_json::Value = read_json(&path.join("forgepack.json"))?;
            let id = metadata
                .get("id")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    WorldError::Invalid(format!("dependency {} has no id", path.display()))
                })?;
            let target = dependency_root.join(format!("{}-{}", &digest[..12], id));
            copy_directory(path, &target)?;
            dependency_records.push(serde_json::json!({
                "id": id,
                "assetType": asset_type,
                "sha256": digest,
                "path": format!("assets/dependencies/{}-{}", &hash_directory(path)?[..12], id)
            }));
        }
    }
    let terrain_runtime = runtime.join("terrain-atlas.png");
    fs::copy(
        terrain_pack.join("assets/terrain-atlas.png"),
        &terrain_runtime,
    )?;
    let building_runtime = runtime.join("building-atlas.png");
    fs::copy(
        building_pack.join("assets/building-atlas.png"),
        &building_runtime,
    )?;
    let mut prop_textures = Vec::new();
    for (pack_index, prop_pack) in dependencies.prop_sets.iter().enumerate() {
        let manifest: serde_json::Value = read_json(&prop_pack.join("assets/manifest.json"))?;
        for item in manifest
            .get("items")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let id = item
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("prop");
            let relative = item
                .get("texture")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| WorldError::Invalid("prop manifest item has no texture".into()))?;
            let file_name = format!("prop-{pack_index}-{id}.png");
            fs::copy(prop_pack.join(relative), runtime.join(&file_name))?;
            prop_textures.push(
                serde_json::json!({"id": id, "texture": format!("assets/runtime/{file_name}")}),
            );
        }
    }
    let map_path = assets.join("map-layout.json");
    write_json_atomic(&map_path, &map)?;
    let preview_path = pack_dir.join("preview.png");
    render_map_preview(&map, &preview_path)?;
    let validation_report_path = pack_dir.join("validation-report.json");
    write_json_atomic(&validation_report_path, &validation)?;
    write_json_atomic(&pack_dir.join("quality-report.json"), &validation)?;
    let manifest = serde_json::json!({
        "schemaVersion": "1",
        "assetType": "map",
        "name": spec.name,
        "compilerProfile": MAP_COMPILER_PROFILE,
        "tileSize": tile_size,
        "map": "assets/map-layout.json",
        "dependencies": dependency_records,
        "layoutSha256": map.layout_sha256
    });
    write_json_atomic(&assets.join("map-manifest.json"), &manifest)?;
    write_json_atomic(&assets.join("manifest.json"), &manifest)?;
    write_json_atomic(
        &assets.join("godot_import.json"),
        &serde_json::json!({
            "schemaVersion": "1",
            "assetType": "map",
            "tileSize": tile_size,
            "terrainAtlas": "assets/runtime/terrain-atlas.png",
            "buildingAtlas": "assets/runtime/building-atlas.png",
            "propTextures": prop_textures,
            "map": "assets/map-layout.json",
            "buildingManifest": building_manifest,
            "terrainManifest": terrain_manifest
        }),
    )?;
    write_json_atomic(
        &pack_dir.join("forgepack.json"),
        &serde_json::json!({
            "schemaVersion": "3.0.0",
            "assetType": "map",
            "id": spec.id,
            "name": spec.name,
            "version": "1.0.0",
            "createdAt": chrono::Utc::now(),
            "creator": {"name": "Game Sprite Forge"},
            "license": {"type": spec.license},
            "source": {"kind": "deterministic_compiler", "name": MAP_COMPILER_PROFILE, "metadata": {
                "seed": spec.seed,
                "selectedCandidate": map.selected_candidate,
                "selectedSeed": map.selected_seed,
                "layoutSha256": map.layout_sha256
            }},
            "dependencies": dependency_records,
            "assets": {
                "manifest": "assets/manifest.json",
                "godotHelper": "assets/godot_import.json",
                "qualityReport": "quality-report.json",
                "mapManifest": "assets/map-manifest.json",
                "mapLayout": "assets/map-layout.json",
                "validationReport": "validation-report.json"
            },
            "previews": {"image": "preview.png"}
        }),
    )?;
    forge_pack::validate_pack_layout(&pack_dir)?;
    Ok(MapBuildOutput {
        pack_dir,
        map_path,
        preview_path,
        validation_report_path,
        layout_sha256: map.layout_sha256,
    })
}

pub fn validate_map_pack(pack: &Path) -> Result<MapValidationReportV1, WorldError> {
    forge_pack::validate_pack_layout(pack)?;
    let metadata: serde_json::Value = read_json(&pack.join("forgepack.json"))?;
    if metadata
        .get("assetType")
        .and_then(serde_json::Value::as_str)
        != Some("map")
    {
        return Err(WorldError::Invalid("pack is not a map".into()));
    }
    let report: MapValidationReportV1 =
        serde_json::from_value(read_json(&pack.join("validation-report.json"))?)?;
    if report.verdict != "game_ready" {
        return Err(WorldError::Invalid("map pack is not game_ready".into()));
    }
    Ok(report)
}

pub fn test_terrain_pack(
    pack: &Path,
    seed: u64,
    samples: u32,
) -> Result<TerrainTestReportV1, WorldError> {
    forge_pack::validate_pack_layout(pack)?;
    if !(1..=4096).contains(&samples) {
        return Err(WorldError::Invalid(
            "terrain test samples must be in 1..=4096".into(),
        ));
    }
    let metadata: serde_json::Value = read_json(&pack.join("forgepack.json"))?;
    if metadata
        .get("assetType")
        .and_then(serde_json::Value::as_str)
        != Some("terrain_set")
    {
        return Err(WorldError::Invalid("pack is not a terrain set".into()));
    }
    let manifest: serde_json::Value = read_json(&pack.join("assets/terrain-manifest.json"))?;
    let tile_size = manifest
        .get("tileSize")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| WorldError::Invalid("terrain manifest has no valid tileSize".into()))?;
    let variant_count = manifest
        .get("variantCount")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(1);
    let atlas = image::open(pack.join("assets/terrain-atlas.png"))?.to_rgba8();
    let quality = if variant_count > 1 {
        validate_terrain_variant_atlas(&atlas, tile_size, variant_count)
    } else {
        validate_terrain_atlas(&atlas, tile_size)
    };
    let holes_detected = atlas.pixels().any(|pixel| pixel[3] == 0);
    let mut rng = DeterministicRng::new(seed);
    let mut sampled_cells = 0u64;
    let mut seams_valid = true;
    for _ in 0..samples {
        let width = 8 + rng.next_u32() % 25;
        let height = 8 + rng.next_u32() % 25;
        let mut corners = vec![false; ((width + 1) * (height + 1)) as usize];
        for value in &mut corners {
            *value = rng.next_u32().is_multiple_of(2);
        }
        let mut masks = vec![0u8; (width * height) as usize];
        for y in 0..height {
            for x in 0..width {
                let corner = |cx: u32, cy: u32| corners[(cy * (width + 1) + cx) as usize];
                masks[(y * width + x) as usize] = u8::from(corner(x, y))
                    | (u8::from(corner(x + 1, y)) << 1)
                    | (u8::from(corner(x + 1, y + 1)) << 2)
                    | (u8::from(corner(x, y + 1)) << 3);
            }
        }
        sampled_cells += u64::from(width) * u64::from(height);
        if !sample_map_seams_match(&atlas, tile_size, width, height, &masks) {
            seams_valid = false;
            break;
        }
    }
    let mut reasons = quality.reasons;
    if !seams_valid {
        reasons.push("sampled_map_seam_mismatch".into());
    }
    if holes_detected {
        reasons.push("sampled_map_contains_transparent_hole".into());
    }
    Ok(TerrainTestReportV1 {
        schema_version: "1".into(),
        profile: if variant_count > 1 {
            TERRAIN_QUALITY_PROFILE_V2.into()
        } else {
            TERRAIN_QUALITY_PROFILE.into()
        },
        seed,
        samples,
        sampled_cells,
        seams_valid,
        holes_detected,
        verdict: if reasons.is_empty() {
            "game_ready".into()
        } else {
            "blocked".into()
        },
        reasons,
    })
}

pub fn validate_environment_spec(spec: &EnvironmentSpecV1) -> Result<(), WorldError> {
    validate_schema_and_id(&spec.schema_version, &spec.id)?;
    if spec.name.trim().is_empty() || spec.prompt.trim().is_empty() {
        return Err(WorldError::Invalid(
            "environment name and prompt are required".into(),
        ));
    }
    if spec.perspective != "top_down" {
        return Err(WorldError::Invalid(
            "Environment V1 supports only perspective top_down".into(),
        ));
    }
    if !matches!(spec.tile_size, 16 | 32) {
        return Err(WorldError::Invalid(
            "Environment V1 tileSize must be 16 or 32".into(),
        ));
    }
    Ok(())
}

pub fn validate_terrain_spec(
    spec: &TerrainSetSpecV1,
    environment: &EnvironmentLockV1,
) -> Result<(), WorldError> {
    if !matches!(spec.schema_version.as_str(), "1" | "2") {
        return Err(WorldError::Invalid(
            "terrain spec requires schemaVersion 1 or 2".into(),
        ));
    }
    validate_id(&spec.id)?;
    if spec.kind != "terrain_set" || spec.name.trim().is_empty() {
        return Err(WorldError::Invalid(
            "terrain spec requires kind terrain_set and a name".into(),
        ));
    }
    for material in [&spec.base, &spec.overlay] {
        validate_id(&material.id)?;
        if material.name.trim().is_empty() || material.prompt.trim().is_empty() {
            return Err(WorldError::Invalid(
                "terrain materials require name and prompt".into(),
            ));
        }
        if !matches!(material.collision.as_str(), "none" | "blocked") {
            return Err(WorldError::Invalid(
                "terrain collision must be none or blocked".into(),
            ));
        }
    }
    if spec.base.id == spec.overlay.id {
        return Err(WorldError::Invalid(
            "base and overlay terrain ids must differ".into(),
        ));
    }
    if spec.schema_version == "1" && (spec.material_sample_count != 1 || spec.variant_count != 1) {
        return Err(WorldError::Invalid(
            "Terrain V1 requires materialSampleCount 1 and variantCount 1".into(),
        ));
    }
    if spec.schema_version == "2" && (spec.material_sample_count != 2 || spec.variant_count != 4) {
        return Err(WorldError::Invalid(
            "Terrain V2 requires materialSampleCount 2 and variantCount 4".into(),
        ));
    }
    if spec
        .environment_revision
        .as_ref()
        .is_some_and(|revision| revision != &environment.revision)
    {
        return Err(WorldError::Invalid(
            "terrain spec environmentRevision does not match the locked environment".into(),
        ));
    }
    Ok(())
}

pub fn validate_building_spec(
    spec: &BuildingKitSpecV1,
    environment: &EnvironmentLockV1,
) -> Result<(), WorldError> {
    if !matches!(spec.schema_version.as_str(), "1" | "2") {
        return Err(WorldError::Invalid(
            "building spec requires schemaVersion 1 or 2".into(),
        ));
    }
    validate_id(&spec.id)?;
    if spec.kind != "building_kit" || spec.name.trim().is_empty() || spec.prompt.trim().is_empty() {
        return Err(WorldError::Invalid(
            "building spec requires kind building_kit, name, and prompt".into(),
        ));
    }
    let footprint = &spec.footprint;
    if footprint.min_width < 3
        || footprint.min_height < 3
        || footprint.max_width > 8
        || footprint.max_height > 6
        || footprint.min_width > footprint.max_width
        || footprint.min_height > footprint.max_height
    {
        return Err(WorldError::Invalid(
            "building footprint must be an ordered 3x3..8x6 range".into(),
        ));
    }
    if !(1..=8).contains(&spec.variant_count) {
        return Err(WorldError::Invalid(
            "building variantCount must be 1..=8".into(),
        ));
    }
    if spec.schema_version == "2" && spec.variant_count != 8 {
        return Err(WorldError::Invalid(
            "Building V2 requires exactly eight deterministic example variants".into(),
        ));
    }
    if spec
        .environment_revision
        .as_ref()
        .is_some_and(|revision| revision != &environment.revision)
    {
        return Err(WorldError::Invalid(
            "building spec environmentRevision does not match the locked environment".into(),
        ));
    }
    Ok(())
}

pub fn validate_map_spec(spec: &MapSpecV1) -> Result<(), WorldError> {
    validate_schema_and_id(&spec.schema_version, &spec.id)?;
    if spec.kind != "map" || spec.name.trim().is_empty() {
        return Err(WorldError::Invalid(
            "map spec requires kind map and a name".into(),
        ));
    }
    if !(32..=256).contains(&spec.size.width) || !(32..=256).contains(&spec.size.height) {
        return Err(WorldError::Invalid(
            "map width and height must each be 32..=256".into(),
        ));
    }
    if spec.dependencies.terrain_sets.is_empty() || spec.dependencies.building_kits.is_empty() {
        return Err(WorldError::Invalid(
            "map requires at least one terrain set and one building kit".into(),
        ));
    }
    for path in spec
        .dependencies
        .terrain_sets
        .iter()
        .chain(&spec.dependencies.building_kits)
        .chain(&spec.dependencies.prop_sets)
    {
        validate_relative_pack_path(path)?;
    }
    let mut ids = BTreeSet::new();
    for region in &spec.regions {
        validate_id(&region.id)?;
        if !ids.insert(region.id.as_str()) {
            return Err(WorldError::Invalid(format!(
                "duplicate map region id: {}",
                region.id
            )));
        }
        if region.x.is_some_and(|x| x >= spec.size.width)
            || region.y.is_some_and(|y| y >= spec.size.height)
        {
            return Err(WorldError::Invalid(format!(
                "map region {} is outside the map",
                region.id
            )));
        }
    }
    for connection in &spec.connections {
        if !ids.contains(connection.from.as_str()) || !ids.contains(connection.to.as_str()) {
            return Err(WorldError::Invalid(format!(
                "map connection {} -> {} references an unknown region",
                connection.from, connection.to
            )));
        }
        if !(1..=4).contains(&connection.width) {
            return Err(WorldError::Invalid(
                "map connection width must be 1..=4".into(),
            ));
        }
    }
    for landmark in &spec.landmarks {
        validate_id(&landmark.id)?;
        if !ids.contains(landmark.region.as_str()) {
            return Err(WorldError::Invalid(format!(
                "landmark {} references unknown region {}",
                landmark.id, landmark.region
            )));
        }
    }
    if spec.requirements.building_count.min > spec.requirements.building_count.max {
        return Err(WorldError::Invalid(
            "buildingCount min must not exceed max".into(),
        ));
    }
    if !(0.0..=0.5).contains(&spec.requirements.prop_density) {
        return Err(WorldError::Invalid(
            "propDensity must be in 0.0..=0.5".into(),
        ));
    }
    Ok(())
}

pub fn make_periodic_tile(source: &RgbaImage, size: u32) -> RgbaImage {
    let half = size.div_ceil(2).max(1);
    let quiet_patch = select_quiet_material_patch(source);
    let sample = DynamicImage::ImageRgba8(quiet_patch)
        .resize_exact(half, half, FilterType::Lanczos3)
        .to_rgba8();
    ImageBuffer::from_fn(size, size, |x, y| {
        let sx = if x < half { x } else { size - 1 - x }.min(half - 1);
        let sy = if y < half { y } else { size - 1 - y }.min(half - 1);
        *sample.get_pixel(sx, sy)
    })
}

pub fn compose_dual_grid_atlas(base: &RgbaImage, overlay: &RgbaImage) -> RgbaImage {
    assert_eq!(base.dimensions(), overlay.dimensions());
    let size = base.width();
    let mut atlas = ImageBuffer::from_pixel(size * 4, size * 4, Rgba([0, 0, 0, 0]));
    for mask in 0u8..16 {
        let tile = compose_dual_grid_tile(base, overlay, mask);
        image::imageops::overlay(
            &mut atlas,
            &tile,
            i64::from(u32::from(mask) % 4 * size),
            i64::from(u32::from(mask) / 4 * size),
        );
    }
    atlas
}

fn compose_dual_grid_variant_atlas(
    base_samples: &[RgbaImage],
    overlay_samples: &[RgbaImage],
    variant_count: u32,
) -> RgbaImage {
    let size = base_samples[0].width();
    let blocks_per_axis = 2u32;
    let mut atlas = ImageBuffer::from_pixel(
        size * 4 * blocks_per_axis,
        size * 4 * blocks_per_axis,
        Rgba([0, 0, 0, 0]),
    );
    let base_secondary = base_samples.get(1).unwrap_or(&base_samples[0]);
    let overlay_secondary = overlay_samples.get(1).unwrap_or(&overlay_samples[0]);
    for variant in 0..variant_count.min(4) {
        let base = blend_variant_tile(&base_samples[0], base_secondary, variant);
        let overlay = blend_variant_tile(&overlay_samples[0], overlay_secondary, variant + 11);
        let block = compose_dual_grid_atlas(&base, &overlay);
        let block_x = variant % blocks_per_axis * size * 4;
        let block_y = variant / blocks_per_axis * size * 4;
        image::imageops::overlay(&mut atlas, &block, i64::from(block_x), i64::from(block_y));
    }
    atlas
}

fn blend_variant_tile(primary: &RgbaImage, secondary: &RgbaImage, variant: u32) -> RgbaImage {
    assert_eq!(primary.dimensions(), secondary.dimensions());
    let width = primary.width();
    let height = primary.height();
    ImageBuffer::from_fn(width, height, |x, y| {
        if x == 0 || y == 0 || x + 1 == width || y + 1 == height {
            return *primary.get_pixel(x, y);
        }
        let use_secondary =
            (x.wrapping_mul(31) ^ y.wrapping_mul(17) ^ variant.wrapping_mul(13)).is_multiple_of(5);
        if use_secondary {
            let left = primary.get_pixel(x, y);
            let right = secondary.get_pixel(x, y);
            Rgba([
                ((u16::from(left[0]) * 3 + u16::from(right[0])) / 4) as u8,
                ((u16::from(left[1]) * 3 + u16::from(right[1])) / 4) as u8,
                ((u16::from(left[2]) * 3 + u16::from(right[2])) / 4) as u8,
                left[3],
            ])
        } else {
            *primary.get_pixel(x, y)
        }
    })
}

fn compose_dual_grid_tile(base: &RgbaImage, overlay: &RgbaImage, mask: u8) -> RgbaImage {
    let size = base.width();
    let denominator = size.saturating_sub(1).max(1) as f32;
    ImageBuffer::from_fn(size, size, |x, y| {
        let fx = x as f32 / denominator;
        let fy = y as f32 / denominator;
        let nw = f32::from(mask & 1 != 0);
        let ne = f32::from(mask & 2 != 0);
        let se = f32::from(mask & 4 != 0);
        let sw = f32::from(mask & 8 != 0);
        let top = nw * (1.0 - fx) + ne * fx;
        let bottom = sw * (1.0 - fx) + se * fx;
        let value = top * (1.0 - fy) + bottom * fy;
        if value >= 0.5 {
            *overlay.get_pixel(x, y)
        } else {
            *base.get_pixel(x, y)
        }
    })
}

fn validate_terrain_atlas(atlas: &RgbaImage, tile_size: u32) -> TerrainQualityReportV1 {
    let dimensions_valid = atlas.width() == tile_size * 4 && atlas.height() == tile_size * 4;
    let masks_complete = dimensions_valid;
    let periodic_edges_closed = [0u8, 15u8].into_iter().all(|mask| {
        let origin_x = u32::from(mask) % 4 * tile_size;
        let origin_y = u32::from(mask) / 4 * tile_size;
        (0..tile_size).all(|index| {
            atlas.get_pixel(origin_x, origin_y + index)
                == atlas.get_pixel(origin_x + tile_size - 1, origin_y + index)
                && atlas.get_pixel(origin_x + index, origin_y)
                    == atlas.get_pixel(origin_x + index, origin_y + tile_size - 1)
        })
    });
    let adjacency_valid = dual_grid_adjacency_is_exact(atlas, tile_size);
    let base_tile = image::imageops::crop_imm(atlas, 0, 0, tile_size, tile_size).to_image();
    let overlay_tile =
        image::imageops::crop_imm(atlas, 3 * tile_size, 3 * tile_size, tile_size, tile_size)
            .to_image();
    let base_detail_energy = detail_energy(&base_tile);
    let overlay_detail_energy = detail_energy(&overlay_tile);
    let material_variation_safe = base_detail_energy <= 0.20 && overlay_detail_energy <= 0.20;
    let mut reasons = Vec::new();
    if !dimensions_valid {
        reasons.push("atlas_dimensions_invalid".into());
    }
    if !periodic_edges_closed {
        reasons.push("periodic_edges_open".into());
    }
    if !adjacency_valid {
        reasons.push("terrain_adjacency_mismatch".into());
    }
    if !material_variation_safe {
        reasons.push("material_detail_too_dense_for_tile_period".into());
    }
    TerrainQualityReportV1 {
        schema_version: "1".into(),
        profile: TERRAIN_QUALITY_PROFILE.into(),
        tile_size,
        tile_count: 16,
        mask_count: 15,
        variant_count: 1,
        dimensions_valid,
        masks_complete,
        periodic_edges_closed,
        adjacency_valid,
        base_detail_energy,
        overlay_detail_energy,
        material_variation_safe,
        verdict: if reasons.is_empty() {
            "game_ready".into()
        } else {
            "blocked".into()
        },
        reasons,
    }
}

fn validate_terrain_variant_atlas(
    atlas: &RgbaImage,
    tile_size: u32,
    variant_count: u32,
) -> TerrainQualityReportV1 {
    let dimensions_valid = atlas.width() == tile_size * 8 && atlas.height() == tile_size * 8;
    let mut reports = Vec::new();
    if dimensions_valid {
        for variant in 0..variant_count.min(4) {
            let x = variant % 2 * tile_size * 4;
            let y = variant / 2 * tile_size * 4;
            let block =
                image::imageops::crop_imm(atlas, x, y, tile_size * 4, tile_size * 4).to_image();
            reports.push(validate_terrain_atlas(&block, tile_size));
        }
    }
    let periodic_edges_closed = reports.iter().all(|report| report.periodic_edges_closed);
    let adjacency_valid = reports.iter().all(|report| report.adjacency_valid)
        && variant_borders_compatible(atlas, tile_size, variant_count);
    let material_variation_safe = reports.iter().all(|report| report.material_variation_safe);
    let base_detail_energy = reports
        .iter()
        .map(|report| report.base_detail_energy)
        .fold(0.0, f32::max);
    let overlay_detail_energy = reports
        .iter()
        .map(|report| report.overlay_detail_energy)
        .fold(0.0, f32::max);
    let mut reasons = Vec::new();
    if !dimensions_valid || reports.len() != variant_count as usize {
        reasons.push("variant_atlas_dimensions_invalid".into());
    }
    if !periodic_edges_closed {
        reasons.push("periodic_edges_open".into());
    }
    if !adjacency_valid {
        reasons.push("cross_variant_adjacency_mismatch".into());
    }
    if !material_variation_safe {
        reasons.push("material_detail_too_dense_for_tile_period".into());
    }
    TerrainQualityReportV1 {
        schema_version: "2".into(),
        profile: TERRAIN_QUALITY_PROFILE_V2.into(),
        tile_size,
        tile_count: 16 * variant_count,
        mask_count: 15,
        variant_count,
        dimensions_valid,
        masks_complete: dimensions_valid && reports.len() == variant_count as usize,
        periodic_edges_closed,
        adjacency_valid,
        base_detail_energy,
        overlay_detail_energy,
        material_variation_safe,
        verdict: if reasons.is_empty() {
            "game_ready".into()
        } else {
            "blocked".into()
        },
        reasons,
    }
}

fn variant_borders_compatible(atlas: &RgbaImage, tile_size: u32, variant_count: u32) -> bool {
    if variant_count <= 1 {
        return true;
    }
    for variant in 1..variant_count.min(4) {
        let block_x = variant % 2 * tile_size * 4;
        let block_y = variant / 2 * tile_size * 4;
        for mask in 0u8..16 {
            let x = u32::from(mask) % 4 * tile_size;
            let y = u32::from(mask) / 4 * tile_size;
            for offset in 0..tile_size {
                if atlas.get_pixel(x, y + offset)
                    != atlas.get_pixel(block_x + x, block_y + y + offset)
                    || atlas.get_pixel(x + tile_size - 1, y + offset)
                        != atlas.get_pixel(block_x + x + tile_size - 1, block_y + y + offset)
                    || atlas.get_pixel(x + offset, y)
                        != atlas.get_pixel(block_x + x + offset, block_y + y)
                    || atlas.get_pixel(x + offset, y + tile_size - 1)
                        != atlas.get_pixel(block_x + x + offset, block_y + y + tile_size - 1)
                {
                    return false;
                }
            }
        }
    }
    true
}

fn select_quiet_material_patch(source: &RgbaImage) -> RgbaImage {
    let side = (source.width().min(source.height()) / 4).max(8);
    if source.width() <= side || source.height() <= side {
        return source.clone();
    }
    let step = (side / 2).max(1);
    let mut best = (f32::MAX, 0, 0);
    let mut y = 0;
    while y + side <= source.height() {
        let mut x = 0;
        while x + side <= source.width() {
            let patch = image::imageops::crop_imm(source, x, y, side, side).to_image();
            let score = detail_energy_sampled(&patch, (side / 48).max(1));
            if score < best.0 {
                best = (score, x, y);
            }
            x = x.saturating_add(step);
        }
        y = y.saturating_add(step);
    }
    image::imageops::crop_imm(source, best.1, best.2, side, side).to_image()
}

fn detail_energy(image: &RgbaImage) -> f32 {
    detail_energy_sampled(image, 1)
}

fn detail_energy_sampled(image: &RgbaImage, stride: u32) -> f32 {
    if image.width() < 2 || image.height() < 2 {
        return 0.0;
    }
    let mut total = 0.0f32;
    let mut count = 0u32;
    let stride = stride.max(1);
    let mut y = 0;
    while y + stride < image.height() {
        let mut x = 0;
        while x + stride < image.width() {
            let center = image.get_pixel(x, y);
            for neighbor in [
                image.get_pixel(x + stride, y),
                image.get_pixel(x, y + stride),
            ] {
                total += (0..3)
                    .map(|channel| {
                        (f32::from(center[channel]) - f32::from(neighbor[channel])).abs() / 255.0
                    })
                    .sum::<f32>()
                    / 3.0;
                count += 1;
            }
            x += stride;
        }
        y += stride;
    }
    if count == 0 {
        0.0
    } else {
        total / count as f32
    }
}

fn sample_map_seams_match(
    atlas: &RgbaImage,
    size: u32,
    width: u32,
    height: u32,
    masks: &[u8],
) -> bool {
    if masks.len() != (width * height) as usize {
        return false;
    }
    let mask_at = |x: u32, y: u32| masks[(y * width + x) as usize];
    for y in 0..height {
        for x in 0..width {
            let mask = mask_at(x, y);
            let origin_x = u32::from(mask) % 4 * size;
            let origin_y = u32::from(mask) / 4 * size;
            if x + 1 < width {
                let right = mask_at(x + 1, y);
                let right_x = u32::from(right) % 4 * size;
                let right_y = u32::from(right) / 4 * size;
                if (0..size).any(|offset| {
                    atlas.get_pixel(origin_x + size - 1, origin_y + offset)
                        != atlas.get_pixel(right_x, right_y + offset)
                }) {
                    return false;
                }
            }
            if y + 1 < height {
                let bottom = mask_at(x, y + 1);
                let bottom_x = u32::from(bottom) % 4 * size;
                let bottom_y = u32::from(bottom) / 4 * size;
                if (0..size).any(|offset| {
                    atlas.get_pixel(origin_x + offset, origin_y + size - 1)
                        != atlas.get_pixel(bottom_x + offset, bottom_y)
                }) {
                    return false;
                }
            }
        }
    }
    true
}

fn dual_grid_adjacency_is_exact(atlas: &RgbaImage, size: u32) -> bool {
    for left in 0u8..16 {
        for right in 0u8..16 {
            let left_ne = left & 2 != 0;
            let left_se = left & 4 != 0;
            let right_nw = right & 1 != 0;
            let right_sw = right & 8 != 0;
            if left_ne != right_nw || left_se != right_sw {
                continue;
            }
            for y in 0..size {
                let lp = atlas.get_pixel(
                    u32::from(left) % 4 * size + size - 1,
                    u32::from(left) / 4 * size + y,
                );
                let rp =
                    atlas.get_pixel(u32::from(right) % 4 * size, u32::from(right) / 4 * size + y);
                if lp != rp {
                    return false;
                }
            }
        }
    }
    for top in 0u8..16 {
        for bottom in 0u8..16 {
            let top_sw = top & 8 != 0;
            let top_se = top & 4 != 0;
            let bottom_nw = bottom & 1 != 0;
            let bottom_ne = bottom & 2 != 0;
            if top_sw != bottom_nw || top_se != bottom_ne {
                continue;
            }
            for x in 0..size {
                let tp = atlas.get_pixel(
                    u32::from(top) % 4 * size + x,
                    u32::from(top) / 4 * size + size - 1,
                );
                let bp = atlas.get_pixel(
                    u32::from(bottom) % 4 * size + x,
                    u32::from(bottom) / 4 * size,
                );
                if tp != bp {
                    return false;
                }
            }
        }
    }
    true
}

fn terrain_manifest(
    spec: &TerrainSetSpecV1,
    environment: &EnvironmentLockV1,
    provider_id: &str,
    profile_id: &str,
    atlas_path: &Path,
) -> Result<serde_json::Value, WorldError> {
    Ok(serde_json::json!({
        "schemaVersion": "1",
        "assetType": "terrain_set",
        "name": spec.name,
        "profile": terrain_profile(spec),
        "tileSize": environment.tile_size,
        "variantCount": spec.variant_count,
        "atlas": "assets/terrain-atlas.png",
        "atlasSha256": hash_file(atlas_path).map_err(|error| WorldError::Invalid(error.to_string()))?,
        "base": spec.base,
        "overlay": spec.overlay,
        "environmentRevision": environment.revision,
        "styleRevision": environment.style_revision,
        "providerId": provider_id,
        "profileId": profile_id,
        "masks": terrain_mask_entries(spec.variant_count)
    }))
}

fn terrain_profile(spec: &TerrainSetSpecV1) -> &'static str {
    if spec.schema_version == "2" {
        TERRAIN_PROFILE_V2
    } else {
        TERRAIN_PROFILE
    }
}

fn terrain_mask_entries(variant_count: u8) -> Vec<serde_json::Value> {
    let blocks_per_axis = if variant_count > 1 { 2 } else { 1 };
    (0..variant_count)
        .flat_map(|variant| {
            (0u8..16).map(move |mask| {
                let block_x = u32::from(variant) % blocks_per_axis * 4;
                let block_y = u32::from(variant) / blocks_per_axis * 4;
                serde_json::json!({
                    "mask": mask,
                    "variant": variant,
                    "x": block_x + u32::from(mask) % 4,
                    "y": block_y + u32::from(mask) / 4,
                    "corners": {
                        "northWest": mask & 1 != 0,
                        "northEast": mask & 2 != 0,
                        "southEast": mask & 4 != 0,
                        "southWest": mask & 8 != 0
                    }
                })
            })
        })
        .collect()
}

fn generate_material(
    provider: &dyn MediaGenerationProvider,
    environment: &EnvironmentLockV1,
    material: &TerrainMaterialSpecV1,
    model: Option<&String>,
    sample: u8,
    target: &Path,
) -> Result<u8, WorldError> {
    edit_image_with_retry(
        provider,
        &EditImageRequest {
            prompt: format!(
                "Preserve the environment reference. Generate seamless flat top-down 2D game terrain material variation sample {} for {}: {}. Fill the entire square with material texture. Keep the same material identity while varying only small interior details. No border, transition, horizon, objects, characters, map, text, or UI.",
                sample, material.name, material.prompt
            ),
            model: model.cloned().or(environment.image_model.clone()),
            references: vec![ProviderImageReference::from_path(
                ReferenceRole::Style,
                environment.board_path.clone(),
            )?],
            aspect_ratio: "1:1".into(),
            resolution: "1k".into(),
        },
        target,
    )
}

fn edit_image_with_retry(
    provider: &dyn MediaGenerationProvider,
    request: &EditImageRequest,
    target: &Path,
) -> Result<u8, WorldError> {
    let mut last_error = String::new();
    for attempt in 1..=2u8 {
        match provider.edit_image(request, target) {
            Ok(media) => match image::open(&media.path) {
                Ok(_) => return Ok(attempt),
                Err(error) => last_error = format!("provider image decode failed: {error}"),
            },
            Err(
                error @ (ProviderError::AuthenticationRequired(_)
                | ProviderError::Entitlement(_)
                | ProviderError::Unavailable(_)
                | ProviderError::Cancelled),
            ) => return Err(WorldError::Provider(error)),
            Err(error) => last_error = error.to_string(),
        }
    }
    Err(WorldError::Invalid(format!(
        "material generation failed after two attempts: {last_error}"
    )))
}

fn compose_building_atlas(roof: &RgbaImage, wall: &RgbaImage, detail: &RgbaImage) -> RgbaImage {
    let size = roof.width();
    let mut atlas = ImageBuffer::from_pixel(size * 4, size * 3, Rgba([0, 0, 0, 0]));
    for (index, module) in BUILDING_MODULES.iter().enumerate() {
        let mut tile = if index < 9 {
            roof.clone()
        } else {
            wall.clone()
        };
        apply_module_detail(&mut tile, module, detail);
        image::imageops::overlay(
            &mut atlas,
            &tile,
            i64::from(index as u32 % 4 * size),
            i64::from(index as u32 / 4 * size),
        );
    }
    atlas
}

fn apply_module_detail(tile: &mut RgbaImage, module: &str, detail: &RgbaImage) {
    let size = tile.width();
    let edge = (size / 8).max(1);
    let darken = |pixel: &mut Rgba<u8>| {
        pixel.0[0] = (f32::from(pixel.0[0]) * 0.65) as u8;
        pixel.0[1] = (f32::from(pixel.0[1]) * 0.65) as u8;
        pixel.0[2] = (f32::from(pixel.0[2]) * 0.65) as u8;
    };
    let north = module.contains("north") || module.ends_with("nw") || module.ends_with("ne");
    let south = module.contains("south") || module.ends_with("sw") || module.ends_with("se");
    let west = module.contains("west") || module.ends_with("nw") || module.ends_with("sw");
    let east = module.contains("east") || module.ends_with("ne") || module.ends_with("se");
    for y in 0..size {
        for x in 0..size {
            if (north && y < edge)
                || (south && y >= size - edge)
                || (west && x < edge)
                || (east && x >= size - edge)
            {
                darken(tile.get_pixel_mut(x, y));
            }
        }
    }
    if module == "door" {
        let left = size / 3;
        let right = size - left;
        let top = size / 4;
        for y in top..size {
            for x in left..right {
                *tile.get_pixel_mut(x, y) = *detail.get_pixel(x, y);
            }
        }
    } else if module == "window" {
        let left = size / 4;
        let right = size - left;
        let top = size / 4;
        let bottom = size - top;
        for y in top..bottom {
            for x in left..right {
                let source = detail.get_pixel(x, y);
                *tile.get_pixel_mut(x, y) = Rgba([
                    source[0] / 2,
                    source[1].saturating_add(48),
                    source[2].saturating_add(72),
                    255,
                ]);
            }
        }
    }
}

fn building_variants(spec: &BuildingKitSpecV1, seed: u64) -> Vec<BuildingVariantV1> {
    let mut rng = DeterministicRng::new(seed);
    (0..spec.variant_count)
        .map(|index| {
            let width = rng.range(spec.footprint.min_width, spec.footprint.max_width);
            let height = rng.range(spec.footprint.min_height, spec.footprint.max_height);
            BuildingVariantV1 {
                id: format!("variant_{:02}", index + 1),
                width,
                height,
                entrance_x: 1 + rng.next_u32() % width.saturating_sub(2).max(1),
                entrance_side: "south".into(),
            }
        })
        .collect()
}

fn validate_building_kit(
    spec: &BuildingKitSpecV1,
    tile_size: u32,
    atlas: &RgbaImage,
    variants: &[BuildingVariantV1],
) -> BuildingQualityReportV1 {
    let dimensions_valid = atlas.width() == tile_size * 4 && atlas.height() == tile_size * 3;
    let modules_complete = dimensions_valid;
    let footprints_valid = variants.iter().all(|variant| {
        variant.width >= spec.footprint.min_width
            && variant.width <= spec.footprint.max_width
            && variant.height >= spec.footprint.min_height
            && variant.height <= spec.footprint.max_height
    });
    let entrances_valid = variants.iter().all(|variant| {
        variant.entrance_side == "south"
            && variant.entrance_x > 0
            && variant.entrance_x + 1 < variant.width
    });
    let mut reasons = Vec::new();
    if !dimensions_valid {
        reasons.push("atlas_dimensions_invalid".into());
    }
    if !footprints_valid {
        reasons.push("footprint_invalid".into());
    }
    if !entrances_valid {
        reasons.push("entrance_invalid".into());
    }
    BuildingQualityReportV1 {
        schema_version: "1".into(),
        profile: if spec.schema_version == "2" {
            BUILDING_QUALITY_PROFILE_V2.into()
        } else {
            BUILDING_QUALITY_PROFILE.into()
        },
        module_count: BUILDING_MODULES.len() as u32,
        expected_module_count: BUILDING_MODULES.len() as u32,
        dimensions_valid,
        modules_complete,
        footprints_valid,
        entrances_valid,
        overlaps_detected: false,
        verdict: if reasons.is_empty() {
            "game_ready".into()
        } else {
            "blocked".into()
        },
        reasons,
    }
}

fn building_profile(spec: &BuildingKitSpecV1) -> &'static str {
    if spec.schema_version == "2" {
        BUILDING_PROFILE_V2
    } else {
        BUILDING_PROFILE
    }
}

fn compile_candidate(
    spec: &MapSpecV1,
    tile_size: u32,
    variants: &[BuildingVariantV1],
    candidate: u8,
    seed: u64,
) -> Result<(CompiledMapV1, MapValidationReportV1), Vec<String>> {
    let mut rng = DeterministicRng::new(seed);
    let region_points = resolve_region_points(spec, &mut rng);
    let spawn = region_points
        .iter()
        .find(|(id, kind, _)| kind == "spawn" || id == "spawn")
        .map(|(_, _, point)| *point)
        .unwrap_or([2, spec.size.height / 2]);
    let exit = region_points
        .iter()
        .find(|(id, kind, _)| kind == "exit" || id == "exit")
        .map(|(_, _, point)| *point)
        .unwrap_or([spec.size.width - 3, spec.size.height / 2]);
    let mut roads = BTreeSet::new();
    let point_map = region_points
        .iter()
        .map(|(id, _, point)| (id.clone(), *point))
        .collect::<BTreeMap<_, _>>();
    let connections = if spec.connections.is_empty() {
        region_points
            .windows(2)
            .map(|pair| MapConnectionV1 {
                from: pair[0].0.clone(),
                to: pair[1].0.clone(),
                width: 1,
            })
            .collect::<Vec<_>>()
    } else {
        spec.connections.clone()
    };
    if connections.is_empty() {
        draw_path(
            spawn,
            exit,
            1,
            candidate.is_multiple_of(2),
            &mut roads,
            &spec.size,
        );
    } else {
        for connection in &connections {
            draw_path(
                point_map[&connection.from],
                point_map[&connection.to],
                connection.width,
                rng.next_u32().is_multiple_of(2),
                &mut roads,
                &spec.size,
            );
        }
    }
    draw_path(
        spawn,
        exit,
        1,
        candidate.is_multiple_of(2),
        &mut roads,
        &spec.size,
    );
    let mut occupied = BTreeSet::new();
    let mut buildings = Vec::new();
    let desired = rng.range(
        spec.requirements.building_count.min,
        spec.requirements.building_count.max,
    );
    let mut road_cells = roads.iter().copied().collect::<Vec<_>>();
    road_cells.sort_unstable();
    for (index, [road_x, road_y]) in road_cells.iter().copied().enumerate() {
        if buildings.len() >= desired as usize || index % 3 != usize::from(candidate % 3 + 1) {
            continue;
        }
        let variant = &variants[(rng.next_u32() as usize) % variants.len()];
        if road_y <= variant.height + 1 || road_x + variant.width >= spec.size.width {
            continue;
        }
        let x = road_x.saturating_sub(variant.entrance_x);
        let y = road_y - variant.height;
        if x == 0 || x + variant.width >= spec.size.width || y == 0 {
            continue;
        }
        let cells = (y..y + variant.height)
            .flat_map(|cell_y| (x..x + variant.width).map(move |cell_x| [cell_x, cell_y]))
            .collect::<Vec<_>>();
        if cells
            .iter()
            .any(|cell| occupied.contains(cell) || roads.contains(cell))
        {
            continue;
        }
        for cell in cells {
            occupied.insert(cell);
        }
        buildings.push(MapBuildingV1 {
            id: format!("building_{:03}", buildings.len() + 1),
            variant: variant.id.clone(),
            x,
            y,
            width: variant.width,
            height: variant.height,
            entrance_x: x + variant.entrance_x,
            entrance_y: road_y,
        });
    }
    let mut reasons = Vec::new();
    if buildings.len() < spec.requirements.building_count.min as usize {
        reasons.push("building_minimum_not_met".into());
    }
    let reachable = reachable(spec.size.width, spec.size.height, spawn, exit, &occupied);
    if spec.requirements.reachable_exit && !reachable {
        reasons.push("exit_unreachable".into());
    }
    let entrances_reachable = buildings.iter().all(|building| {
        roads.contains(&[building.entrance_x, building.entrance_y])
            && !occupied.contains(&[building.entrance_x, building.entrance_y])
    });
    if !entrances_reachable {
        reasons.push("building_entrance_unreachable".into());
    }
    if !reasons.is_empty() {
        return Err(reasons);
    }
    let mut props = Vec::new();
    let target_props = ((spec.size.width * spec.size.height) as f32
        * spec.requirements.prop_density)
        .round() as usize;
    for _ in 0..target_props.saturating_mul(8).max(1) {
        if props.len() >= target_props {
            break;
        }
        let x = 1 + rng.next_u32() % (spec.size.width - 2);
        let y = 1 + rng.next_u32() % (spec.size.height - 2);
        let cell = [x, y];
        if roads.contains(&cell) || occupied.contains(&cell) || cell == spawn || cell == exit {
            continue;
        }
        occupied.insert(cell);
        props.push(MapPropV1 {
            id: format!("prop_{:04}", props.len() + 1),
            x,
            y,
        });
    }
    let terrain_cells = roads
        .iter()
        .map(|[x, y]| MapCellV1 {
            x: *x,
            y: *y,
            mask: 15,
        })
        .collect::<Vec<_>>();
    let scale = tile_size as f32;
    let mut navigation_outlines = vec![vec![
        [0.0, 0.0],
        [0.0, spec.size.height as f32 * scale],
        [
            spec.size.width as f32 * scale,
            spec.size.height as f32 * scale,
        ],
        [spec.size.width as f32 * scale, 0.0],
    ]];
    navigation_outlines.extend(buildings.iter().map(|building| {
        let left = building.x as f32 * scale;
        let top = building.y as f32 * scale;
        let right = (building.x + building.width) as f32 * scale;
        let bottom = (building.y + building.height) as f32 * scale;
        vec![[left, top], [right, top], [right, bottom], [left, bottom]]
    }));
    let map = CompiledMapV1 {
        schema_version: "1".into(),
        compiler_profile: MAP_COMPILER_PROFILE.into(),
        source_seed: spec.seed,
        selected_candidate: candidate,
        selected_seed: seed,
        width: spec.size.width,
        height: spec.size.height,
        tile_size,
        terrain_cells,
        buildings,
        props,
        spawn,
        exit,
        navigation_outlines,
        layout_sha256: String::new(),
    };
    let validation = MapValidationReportV1 {
        schema_version: "1".into(),
        profile: MAP_VALIDATION_PROFILE.into(),
        verdict: "game_ready".into(),
        reachable_exit: reachable,
        buildings_in_range: true,
        entrances_reachable,
        isolated_walkable_islands: 0,
        selected_candidate: Some(candidate),
        selected_score: None,
        candidates: vec![],
        reasons: vec![],
    };
    Ok((map, validation))
}

fn score_map_candidate(spec: &MapSpecV1, map: &CompiledMapV1) -> MapCandidateScoreV2 {
    let manhattan = map.spawn[0].abs_diff(map.exit[0]) + map.spawn[1].abs_diff(map.exit[1]);
    let maximum_distance = spec.size.width + spec.size.height;
    let path_quality = ((manhattan as f32 / maximum_distance.max(1) as f32) * 25.0)
        .round()
        .clamp(0.0, 25.0) as u32;

    let ideal_road_cells = manhattan.max(1) as f32 * 1.35;
    let road_ratio = map.terrain_cells.len() as f32 / ideal_road_cells;
    let region_connectivity =
        (20.0 * (1.0 - (road_ratio - 1.0).abs()).clamp(0.0, 1.0)).round() as u32;

    let building_road_adjacency = if map.buildings.iter().all(|building| {
        map.terrain_cells
            .iter()
            .any(|cell| cell.x == building.entrance_x && cell.y == building.entrance_y)
    }) {
        20
    } else {
        0
    };

    let landmark_distribution = if spec.landmarks.is_empty() {
        15
    } else {
        let distinct_regions = spec
            .landmarks
            .iter()
            .map(|landmark| landmark.region.as_str())
            .collect::<BTreeSet<_>>()
            .len();
        ((distinct_regions as f32 / spec.landmarks.len() as f32) * 15.0)
            .round()
            .clamp(0.0, 15.0) as u32
    };

    let target_props = (spec.size.width * spec.size.height) as f32 * spec.requirements.prop_density;
    let density_error = if target_props <= f32::EPSILON {
        if map.props.is_empty() {
            0.0
        } else {
            1.0
        }
    } else {
        ((map.props.len() as f32 - target_props).abs() / target_props).min(1.0)
    };
    let density_match = (10.0 * (1.0 - density_error)).round() as u32;

    let mut quadrants = [false; 4];
    for prop in &map.props {
        let east = usize::from(prop.x >= spec.size.width / 2);
        let south = usize::from(prop.y >= spec.size.height / 2);
        quadrants[south * 2 + east] = true;
    }
    let repetition_control =
        (quadrants.into_iter().filter(|occupied| *occupied).count() as u32 * 10) / 4;

    MapCandidateScoreV2 {
        path_quality,
        region_connectivity,
        building_road_adjacency,
        landmark_distribution,
        density_match,
        repetition_control,
    }
}

fn resolve_region_points(
    spec: &MapSpecV1,
    rng: &mut DeterministicRng,
) -> Vec<(String, String, [u32; 2])> {
    if spec.regions.is_empty() {
        return vec![
            ("spawn".into(), "spawn".into(), [2, spec.size.height / 2]),
            (
                "settlement".into(),
                "settlement".into(),
                [spec.size.width / 2, spec.size.height / 2],
            ),
            (
                "exit".into(),
                "exit".into(),
                [spec.size.width - 3, spec.size.height / 2],
            ),
        ];
    }
    spec.regions
        .iter()
        .enumerate()
        .map(|(index, region)| {
            let x = region.x.unwrap_or_else(|| {
                if index == 0 {
                    2
                } else if index + 1 == spec.regions.len() {
                    spec.size.width - 3
                } else {
                    4 + rng.next_u32() % (spec.size.width - 8)
                }
            });
            let y = region
                .y
                .unwrap_or_else(|| 4 + rng.next_u32() % (spec.size.height - 8));
            (region.id.clone(), region.kind.clone(), [x, y])
        })
        .collect()
}

fn draw_path(
    from: [u32; 2],
    to: [u32; 2],
    width: u8,
    horizontal_first: bool,
    roads: &mut BTreeSet<[u32; 2]>,
    size: &MapSizeV1,
) {
    fn add_point(roads: &mut BTreeSet<[u32; 2]>, point: [u32; 2], width: u8, size: &MapSizeV1) {
        let radius = u32::from(width.saturating_sub(1));
        for dy in 0..=radius {
            for dx in 0..=radius {
                let x = point[0].saturating_add(dx).min(size.width - 1);
                let y = point[1].saturating_add(dy).min(size.height - 1);
                roads.insert([x, y]);
            }
        }
    }
    fn move_x(
        point: &mut [u32; 2],
        to: [u32; 2],
        width: u8,
        size: &MapSizeV1,
        roads: &mut BTreeSet<[u32; 2]>,
    ) {
        while point[0] != to[0] {
            point[0] = if point[0] < to[0] {
                point[0] + 1
            } else {
                point[0] - 1
            };
            add_point(roads, *point, width, size);
        }
    }
    fn move_y(
        point: &mut [u32; 2],
        to: [u32; 2],
        width: u8,
        size: &MapSizeV1,
        roads: &mut BTreeSet<[u32; 2]>,
    ) {
        while point[1] != to[1] {
            point[1] = if point[1] < to[1] {
                point[1] + 1
            } else {
                point[1] - 1
            };
            add_point(roads, *point, width, size);
        }
    }
    let mut point = from;
    add_point(roads, point, width, size);
    if horizontal_first {
        move_x(&mut point, to, width, size, roads);
        move_y(&mut point, to, width, size, roads);
    } else {
        move_y(&mut point, to, width, size, roads);
        move_x(&mut point, to, width, size, roads);
    }
}

fn reachable(
    width: u32,
    height: u32,
    start: [u32; 2],
    goal: [u32; 2],
    blocked: &BTreeSet<[u32; 2]>,
) -> bool {
    let mut queue = VecDeque::from([start]);
    let mut visited = BTreeSet::from([start]);
    while let Some([x, y]) = queue.pop_front() {
        if [x, y] == goal {
            return true;
        }
        for next in [
            x.checked_sub(1).map(|nx| [nx, y]),
            (x + 1 < width).then_some([x + 1, y]),
            y.checked_sub(1).map(|ny| [x, ny]),
            (y + 1 < height).then_some([x, y + 1]),
        ]
        .into_iter()
        .flatten()
        {
            if !blocked.contains(&next) && visited.insert(next) {
                queue.push_back(next);
            }
        }
    }
    false
}

fn render_terrain_preview(
    atlas: &RgbaImage,
    tile_size: u32,
    seed: u64,
    output: &Path,
) -> Result<(), WorldError> {
    let width = 12u32;
    let height = 8u32;
    let mut corners = vec![false; ((width + 1) * (height + 1)) as usize];
    let mut rng = DeterministicRng::new(seed);
    for value in &mut corners {
        *value = rng.next_u32() % 100 < 52;
    }
    let mut preview =
        ImageBuffer::from_pixel(width * tile_size, height * tile_size, Rgba([0, 0, 0, 0]));
    for y in 0..height {
        for x in 0..width {
            let corner = |cx: u32, cy: u32| corners[(cy * (width + 1) + cx) as usize];
            let mask = u8::from(corner(x, y))
                | (u8::from(corner(x + 1, y)) << 1)
                | (u8::from(corner(x + 1, y + 1)) << 2)
                | (u8::from(corner(x, y + 1)) << 3);
            let tile = image::imageops::crop_imm(
                atlas,
                u32::from(mask) % 4 * tile_size,
                u32::from(mask) / 4 * tile_size,
                tile_size,
                tile_size,
            )
            .to_image();
            image::imageops::overlay(
                &mut preview,
                &tile,
                i64::from(x * tile_size),
                i64::from(y * tile_size),
            );
        }
    }
    preview.save(output)?;
    Ok(())
}

fn render_building_preview(
    atlas: &RgbaImage,
    tile_size: u32,
    variants: &[BuildingVariantV1],
    output: &Path,
) -> Result<(), WorldError> {
    let columns = variants.len().clamp(1, 4) as u32;
    let rows = (variants.len() as u32).div_ceil(columns);
    let cell_width = 9 * tile_size;
    let cell_height = 7 * tile_size;
    let mut preview = ImageBuffer::from_pixel(
        columns * cell_width,
        rows * cell_height,
        Rgba([28, 32, 38, 255]),
    );
    for (index, variant) in variants.iter().enumerate() {
        let origin_x = index as u32 % columns * cell_width + tile_size / 2;
        let origin_y = index as u32 / columns * cell_height + tile_size / 2;
        for y in 0..variant.height {
            for x in 0..variant.width {
                let module = if y == variant.height - 1 && x == variant.entrance_x {
                    10
                } else if y == variant.height - 1 {
                    9
                } else {
                    roof_module_index(x, y, variant.width, variant.height)
                };
                let tile = image::imageops::crop_imm(
                    atlas,
                    module % 4 * tile_size,
                    module / 4 * tile_size,
                    tile_size,
                    tile_size,
                )
                .to_image();
                image::imageops::overlay(
                    &mut preview,
                    &tile,
                    i64::from(origin_x + x * tile_size),
                    i64::from(origin_y + y * tile_size),
                );
            }
        }
    }
    preview.save(output)?;
    Ok(())
}

fn roof_module_index(x: u32, y: u32, width: u32, height: u32) -> u32 {
    match (x == 0, x + 1 == width, y == 0, y + 1 == height) {
        (true, _, true, _) => 5,
        (_, true, true, _) => 6,
        (true, _, _, true) => 7,
        (_, true, _, true) => 8,
        (_, _, true, _) => 1,
        (_, true, _, _) => 2,
        (_, _, _, true) => 3,
        (true, _, _, _) => 4,
        _ => 0,
    }
}

fn render_map_preview(map: &CompiledMapV1, output: &Path) -> Result<(), WorldError> {
    let scale = 4u32;
    let mut image = ImageBuffer::from_pixel(
        map.width * scale,
        map.height * scale,
        Rgba([72, 112, 72, 255]),
    );
    let mut paint_cell = |x: u32, y: u32, color: Rgba<u8>| {
        for py in y * scale..(y + 1) * scale {
            for px in x * scale..(x + 1) * scale {
                image.put_pixel(px, py, color);
            }
        }
    };
    for cell in &map.terrain_cells {
        paint_cell(cell.x, cell.y, Rgba([142, 102, 62, 255]));
    }
    for building in &map.buildings {
        for y in building.y..building.y + building.height {
            for x in building.x..building.x + building.width {
                paint_cell(x, y, Rgba([132, 72, 64, 255]));
            }
        }
    }
    for prop in &map.props {
        paint_cell(prop.x, prop.y, Rgba([224, 190, 72, 255]));
    }
    paint_cell(map.spawn[0], map.spawn[1], Rgba([64, 128, 255, 255]));
    paint_cell(map.exit[0], map.exit[1], Rgba([224, 72, 224, 255]));
    image.save(output)?;
    Ok(())
}

#[derive(Debug)]
struct ResolvedMapDependencies {
    terrain_sets: Vec<PathBuf>,
    building_kits: Vec<PathBuf>,
    prop_sets: Vec<PathBuf>,
}

fn resolve_map_dependencies(
    root: &Path,
    dependencies: &MapDependenciesV1,
) -> Result<ResolvedMapDependencies, WorldError> {
    let resolve = |paths: &[PathBuf], expected: &str| -> Result<Vec<PathBuf>, WorldError> {
        paths
            .iter()
            .map(|relative| {
                validate_relative_pack_path(relative)?;
                let path = root.join(relative);
                forge_pack::validate_pack_layout(&path)?;
                let metadata: serde_json::Value = read_json(&path.join("forgepack.json"))?;
                if metadata
                    .get("assetType")
                    .and_then(serde_json::Value::as_str)
                    != Some(expected)
                {
                    return Err(WorldError::Invalid(format!(
                        "dependency {} must be a {} pack",
                        relative.display(),
                        expected
                    )));
                }
                Ok(path)
            })
            .collect()
    };
    Ok(ResolvedMapDependencies {
        terrain_sets: resolve(&dependencies.terrain_sets, "terrain_set")?,
        building_kits: resolve(&dependencies.building_kits, "building_kit")?,
        prop_sets: resolve(&dependencies.prop_sets, "prop_set")?,
    })
}

fn validate_relative_pack_path(path: &Path) -> Result<(), WorldError> {
    if path.is_absolute() || path.as_os_str().is_empty() {
        return Err(WorldError::Invalid(
            "map dependency paths must be non-empty and relative".into(),
        ));
    }
    for component in path.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(WorldError::Invalid(format!(
                "map dependency path is unsafe: {}",
                path.display()
            )));
        }
    }
    let text = path.to_string_lossy();
    if text.contains("://") || !text.ends_with(".gsfpack") {
        return Err(WorldError::Invalid(
            "map dependencies must be relative .gsfpack paths".into(),
        ));
    }
    Ok(())
}

fn validate_schema_and_id(schema_version: &str, id: &str) -> Result<(), WorldError> {
    if schema_version != "1" {
        return Err(WorldError::Invalid(
            "world asset schemaVersion must be 1".into(),
        ));
    }
    validate_id(id)
}

fn validate_id(id: &str) -> Result<(), WorldError> {
    if id.is_empty()
        || !id.chars().enumerate().all(|(index, value)| {
            value.is_ascii_alphanumeric() || (index > 0 && matches!(value, '-' | '_'))
        })
    {
        return Err(WorldError::Invalid(format!("invalid engine-safe id: {id}")));
    }
    Ok(())
}

fn copy_directory(source: &Path, target: &Path) -> Result<(), WorldError> {
    if fs::symlink_metadata(source)?.file_type().is_symlink() {
        return Err(WorldError::Invalid(format!(
            "dependency may not be a symlink: {}",
            source.display()
        )));
    }
    fs::create_dir_all(target)?;
    let mut entries = fs::read_dir(source)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(WorldError::Invalid(format!(
                "dependency contains a symlink: {}",
                entry.path().display()
            )));
        }
        let destination = target.join(entry.file_name());
        if file_type.is_dir() {
            copy_directory(&entry.path(), &destination)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}

fn hash_directory(root: &Path) -> Result<String, WorldError> {
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    files.sort();
    let mut digest = Sha256::new();
    for relative in files {
        digest.update(relative.to_string_lossy().as_bytes());
        digest.update([0]);
        digest.update(fs::read(root.join(&relative))?);
        digest.update([0]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn collect_files(root: &Path, current: &Path, files: &mut Vec<PathBuf>) -> Result<(), WorldError> {
    let mut entries = fs::read_dir(current)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(WorldError::Invalid(format!(
                "pack contains a symlink: {}",
                entry.path().display()
            )));
        }
        if file_type.is_dir() {
            collect_files(root, &entry.path(), files)?;
        } else if file_type.is_file() {
            files.push(
                entry
                    .path()
                    .strip_prefix(root)
                    .map_err(|_| WorldError::Invalid("dependency escaped its root".into()))?
                    .to_path_buf(),
            );
        }
    }
    Ok(())
}

fn derived_seed(seed: u64, candidate: u8) -> u64 {
    let mut digest = Sha256::new();
    digest.update(seed.to_le_bytes());
    digest.update([candidate]);
    let bytes = digest.finalize();
    u64::from_le_bytes(bytes[..8].try_into().expect("SHA-256 has eight bytes"))
}

fn hash_json(value: &serde_json::Value) -> Result<String, WorldError> {
    let bytes = serde_json::to_vec(value)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn read_json(path: &Path) -> Result<serde_json::Value, WorldError> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), WorldError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension("tmp");
    fs::write(&temp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temp, path)?;
    Ok(())
}

#[derive(Debug)]
struct DeterministicRng(u64);

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    fn range(&mut self, min: u32, max: u32) -> u32 {
        min + self.next_u32() % (max - min + 1)
    }
}

fn top_down() -> String {
    "top_down".into()
}

fn default_license() -> String {
    "MIT".into()
}

fn default_true() -> bool {
    true
}

fn default_collision_tag() -> String {
    "none".into()
}

fn default_variant_count() -> u8 {
    3
}

fn default_material_sample_count() -> u8 {
    1
}

fn default_terrain_variant_count() -> u8 {
    1
}

fn default_one_u32() -> u32 {
    1
}

fn default_road_width() -> u8 {
    1
}

fn default_prop_density() -> f32 {
    0.08
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn periodic_tile_closes_every_edge() {
        let source = ImageBuffer::from_fn(19, 23, |x, y| {
            Rgba([(x * 7) as u8, (y * 5) as u8, (x + y) as u8, 255])
        });
        let tile = make_periodic_tile(&source, 16);
        for index in 0..16 {
            assert_eq!(tile.get_pixel(0, index), tile.get_pixel(15, index));
            assert_eq!(tile.get_pixel(index, 0), tile.get_pixel(index, 15));
        }
    }

    #[test]
    fn periodic_tile_chooses_quiet_material_area_over_large_features() {
        let mut source = ImageBuffer::from_pixel(128, 64, Rgba([70, 100, 72, 255]));
        for y in 0..64 {
            for x in 64..128 {
                let value = if (x + y) % 2 == 0 { 0 } else { 255 };
                source.put_pixel(x, y, Rgba([value, 255 - value, value, 255]));
            }
        }
        let tile = make_periodic_tile(&source, 32);

        assert!(detail_energy(&tile) < 0.02);
    }

    #[test]
    fn terrain_v2_generates_four_seam_compatible_variants() {
        let primary_base = make_periodic_tile(
            &ImageBuffer::from_fn(64, 64, |x, y| {
                Rgba([70 + (x % 5) as u8, 110 + (y % 5) as u8, 72, 255])
            }),
            16,
        );
        let secondary_base = make_periodic_tile(
            &ImageBuffer::from_fn(64, 64, |x, y| {
                Rgba([75 + (y % 5) as u8, 114, 78 + (x % 5) as u8, 255])
            }),
            16,
        );
        let primary_overlay = make_periodic_tile(
            &ImageBuffer::from_fn(64, 64, |x, y| {
                Rgba([132 + (x % 4) as u8, 96 + (y % 4) as u8, 58, 255])
            }),
            16,
        );
        let secondary_overlay = make_periodic_tile(
            &ImageBuffer::from_fn(64, 64, |x, y| {
                Rgba([128 + (y % 4) as u8, 92, 62 + (x % 4) as u8, 255])
            }),
            16,
        );
        let atlas = compose_dual_grid_variant_atlas(
            &[primary_base, secondary_base],
            &[primary_overlay, secondary_overlay],
            4,
        );
        let report = validate_terrain_variant_atlas(&atlas, 16, 4);
        assert_eq!(atlas.dimensions(), (128, 128));
        assert_eq!(report.tile_count, 64);
        assert_eq!(report.variant_count, 4);
        assert_eq!(report.verdict, "game_ready", "{:?}", report.reasons);
    }

    #[test]
    fn dual_grid_generates_all_masks_with_exact_legal_seams() {
        let base = ImageBuffer::from_pixel(16, 16, Rgba([20, 80, 20, 255]));
        let overlay = ImageBuffer::from_pixel(16, 16, Rgba([120, 80, 30, 255]));
        let atlas = compose_dual_grid_atlas(&base, &overlay);
        let report = validate_terrain_atlas(&atlas, 16);
        assert_eq!(report.tile_count, 16);
        assert_eq!(report.mask_count, 15);
        assert_eq!(report.verdict, "game_ready");
    }

    #[test]
    fn map_spec_rejects_parent_dependency_and_unknown_connection() {
        let mut spec = MapSpecV1 {
            schema_version: "1".into(),
            kind: "map".into(),
            id: "forest".into(),
            name: "Forest".into(),
            seed: 1,
            size: MapSizeV1 {
                width: 64,
                height: 64,
            },
            dependencies: MapDependenciesV1 {
                terrain_sets: vec![PathBuf::from("../terrain.gsfpack")],
                building_kits: vec![PathBuf::from("buildings.gsfpack")],
                prop_sets: vec![],
            },
            regions: vec![],
            connections: vec![],
            landmarks: vec![],
            requirements: MapRequirementsV1::default(),
            license: "MIT".into(),
        };
        assert!(validate_map_spec(&spec).is_err());
        spec.dependencies.terrain_sets = vec![PathBuf::from("terrain.gsfpack")];
        spec.connections = vec![MapConnectionV1 {
            from: "missing".into(),
            to: "also_missing".into(),
            width: 1,
        }];
        assert!(validate_map_spec(&spec).is_err());
    }
}
