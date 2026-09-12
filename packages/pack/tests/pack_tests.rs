use forge_pack::{import_pack, read_pack_summary, validate_pack_layout, PackError};
use serde_json::json;
use std::fs;
use std::path::Path;

#[test]
fn validates_required_gsfpack_layout() {
    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    write_pack_fixture(&pack, 3);

    validate_pack_layout(&pack).unwrap();
    let summary = read_pack_summary(&pack).unwrap();

    assert_eq!(summary.id, "hero");
    assert_eq!(summary.frame_count, 3);
    assert_eq!(summary.preview_gif, "previews/preview.gif");
}

#[test]
fn missing_required_layout_file_fails_validation() {
    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    write_pack_fixture(&pack, 1);
    fs::remove_file(pack.join("assets/atlas.json")).unwrap();

    let error = validate_pack_layout(&pack).unwrap_err();

    assert!(matches!(error, PackError::MissingFile(path) if path == "assets/atlas.json"));
}

#[test]
fn can_reimport_pack_and_preserve_exported_frame_count() {
    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    let original_exported_frame_count = 5;
    write_pack_fixture(&pack, original_exported_frame_count);

    let imported = import_pack(&pack).unwrap();

    assert_eq!(imported.summary.frame_count, original_exported_frame_count);
    assert_eq!(imported.frame_paths.len(), original_exported_frame_count);
    assert_eq!(imported.manifest["name"], "Hero");
    assert_eq!(
        imported.atlas["frames"].as_array().unwrap().len(),
        original_exported_frame_count
    );
}

#[test]
fn inspect_pack_returns_paths_needed_by_local_library() {
    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("library.gsfpack");
    write_pack_fixture(&pack, 3);

    let summary = forge_pack::inspect_pack(&pack).unwrap();

    assert_eq!(summary.name, "Hero");
    assert_eq!(summary.frame_count, 3);
    assert_eq!(summary.root, pack);
    assert_eq!(
        summary.manifest_path,
        summary.root.join("assets/manifest.json")
    );
    assert_eq!(summary.atlas_path, summary.root.join("assets/atlas.json"));
    assert_eq!(
        summary.quality_report_path,
        summary.root.join("quality-report.json")
    );
    assert_eq!(summary.default_animation, "idle");
    assert_eq!(summary.animations.len(), 1);
    assert_eq!(summary.animations[0].frame_count, 3);
    assert_eq!(summary.animations[0].fps, 12.0);
    assert!(summary.animations[0].loop_animation);
}

#[test]
fn validates_multipage_atlas_assets() {
    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    write_pack_fixture(&pack, 4);
    fs::write(pack.join("assets/sprite_sheet_002.png"), b"png").unwrap();
    fs::write(
        pack.join("assets/atlas.json"),
        json!({
            "image": "sprite_sheet.png",
            "images": ["sprite_sheet.png", "sprite_sheet_002.png"],
            "frameWidth": 16,
            "frameHeight": 16,
            "columns": 1,
            "rows": 2,
            "frames": [
                { "index": 0, "name": "frame_001.png", "x": 0, "y": 0, "width": 16, "height": 16 },
                { "index": 1, "name": "frame_002.png", "x": 0, "y": 16, "width": 16, "height": 16 },
                { "index": 2, "name": "frame_003.png", "page": 1, "image": "sprite_sheet_002.png", "x": 0, "y": 0, "width": 16, "height": 16 },
                { "index": 3, "name": "frame_004.png", "page": 1, "image": "sprite_sheet_002.png", "x": 0, "y": 16, "width": 16, "height": 16 }
            ]
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        pack.join("assets/manifest.json"),
        json!({
            "name": "Hero",
            "sheet": {
                "image": "assets/sprite_sheet.png",
                "images": ["assets/sprite_sheet.png", "assets/sprite_sheet_002.png"],
                "frameWidth": 16,
                "frameHeight": 16,
                "columns": 1,
                "rows": 2
            },
            "animations": [{
                "name": "idle",
                "frames": [0, 1, 2, 3],
                "fps": 12.0,
                "loop": true
            }],
            "anchor": { "type": "feet", "x": 8.0, "y": 16.0 }
        })
        .to_string(),
    )
    .unwrap();

    validate_pack_layout(&pack).unwrap();
}

#[test]
fn missing_multipage_atlas_image_fails_validation() {
    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    write_pack_fixture(&pack, 2);
    fs::write(
        pack.join("assets/atlas.json"),
        json!({
            "image": "sprite_sheet.png",
            "images": ["sprite_sheet.png", "sprite_sheet_002.png"],
            "frameWidth": 16,
            "frameHeight": 16,
            "columns": 1,
            "rows": 2,
            "frames": [
                { "index": 0, "name": "frame_001.png", "x": 0, "y": 0, "width": 16, "height": 16 },
                { "index": 1, "name": "frame_002.png", "page": 1, "image": "sprite_sheet_002.png", "x": 0, "y": 0, "width": 16, "height": 16 }
            ]
        })
        .to_string(),
    )
    .unwrap();

    let error = validate_pack_layout(&pack).unwrap_err();

    assert!(matches!(error, PackError::MissingFile(path) if path == "assets/sprite_sheet_002.png"));
}

#[test]
fn top_level_atlas_image_path_traversal_fails_validation() {
    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    write_pack_fixture(&pack, 2);
    fs::write(
        pack.join("assets/atlas.json"),
        json!({
            "image": "sprite_sheet.png",
            "images": ["sprite_sheet.png", "../escape.png"],
            "frameWidth": 16,
            "frameHeight": 16,
            "columns": 1,
            "rows": 2,
            "frames": [
                { "index": 0, "name": "frame_001.png", "x": 0, "y": 0, "width": 16, "height": 16 },
                { "index": 1, "name": "frame_002.png", "x": 0, "y": 16, "width": 16, "height": 16 }
            ]
        })
        .to_string(),
    )
    .unwrap();

    let error = validate_pack_layout(&pack).unwrap_err();

    assert!(matches!(error, PackError::InvalidAssetPath(path) if path == "../escape.png"));
}

#[test]
fn legacy_atlas_image_path_traversal_fails_validation() {
    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    write_pack_fixture(&pack, 2);
    fs::write(
        pack.join("assets/atlas.json"),
        json!({
            "image": "../escape.png",
            "frameWidth": 16,
            "frameHeight": 16,
            "columns": 2,
            "rows": 1,
            "frames": [
                { "index": 0, "name": "frame_001.png", "x": 0, "y": 0, "width": 16, "height": 16 },
                { "index": 1, "name": "frame_002.png", "x": 16, "y": 0, "width": 16, "height": 16 }
            ]
        })
        .to_string(),
    )
    .unwrap();

    let error = validate_pack_layout(&pack).unwrap_err();

    assert!(matches!(error, PackError::InvalidAssetPath(path) if path == "../escape.png"));
}

#[test]
fn legacy_atlas_image_must_be_sprite_sheet_png() {
    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    write_pack_fixture(&pack, 2);
    fs::write(pack.join("assets/alternate.png"), b"png").unwrap();
    fs::write(
        pack.join("assets/atlas.json"),
        json!({
            "image": "alternate.png",
            "frameWidth": 16,
            "frameHeight": 16,
            "columns": 2,
            "rows": 1,
            "frames": [
                { "index": 0, "name": "frame_001.png", "x": 0, "y": 0, "width": 16, "height": 16 },
                { "index": 1, "name": "frame_002.png", "x": 16, "y": 0, "width": 16, "height": 16 }
            ]
        })
        .to_string(),
    )
    .unwrap();

    let error = validate_pack_layout(&pack).unwrap_err();

    assert!(matches!(error, PackError::InvalidAssetPath(path) if path == "alternate.png"));
}

#[test]
fn top_level_atlas_image_non_png_fails_validation() {
    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    write_pack_fixture(&pack, 2);
    fs::write(pack.join("assets/sprite_sheet_002.jpg"), b"jpg").unwrap();
    fs::write(
        pack.join("assets/atlas.json"),
        json!({
            "image": "sprite_sheet.png",
            "images": ["sprite_sheet.png", "sprite_sheet_002.jpg"],
            "frameWidth": 16,
            "frameHeight": 16,
            "columns": 1,
            "rows": 2,
            "frames": [
                { "index": 0, "name": "frame_001.png", "x": 0, "y": 0, "width": 16, "height": 16 },
                { "index": 1, "name": "frame_002.png", "x": 0, "y": 16, "width": 16, "height": 16 }
            ]
        })
        .to_string(),
    )
    .unwrap();

    let error = validate_pack_layout(&pack).unwrap_err();

    assert!(matches!(error, PackError::InvalidAssetPath(path) if path == "sprite_sheet_002.jpg"));
}

#[test]
fn top_level_atlas_image_directory_fails_validation() {
    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    write_pack_fixture(&pack, 2);
    fs::create_dir(pack.join("assets/sprite_sheet_002.png")).unwrap();
    fs::write(
        pack.join("assets/atlas.json"),
        json!({
            "image": "sprite_sheet.png",
            "images": ["sprite_sheet.png", "sprite_sheet_002.png"],
            "frameWidth": 16,
            "frameHeight": 16,
            "columns": 1,
            "rows": 2,
            "frames": [
                { "index": 0, "name": "frame_001.png", "x": 0, "y": 0, "width": 16, "height": 16 },
                { "index": 1, "name": "frame_002.png", "x": 0, "y": 16, "width": 16, "height": 16 }
            ]
        })
        .to_string(),
    )
    .unwrap();

    let error = validate_pack_layout(&pack).unwrap_err();

    assert!(matches!(error, PackError::MissingFile(path) if path == "assets/sprite_sheet_002.png"));
}

#[test]
fn frame_image_missing_from_top_level_images_fails_validation() {
    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    write_pack_fixture(&pack, 3);
    fs::write(pack.join("assets/sprite_sheet_002.png"), b"png").unwrap();
    fs::write(pack.join("assets/sprite_sheet_003.png"), b"png").unwrap();
    fs::write(
        pack.join("assets/atlas.json"),
        json!({
            "image": "sprite_sheet.png",
            "images": ["sprite_sheet.png", "sprite_sheet_002.png"],
            "frameWidth": 16,
            "frameHeight": 16,
            "columns": 1,
            "rows": 2,
            "frames": [
                { "index": 0, "name": "frame_001.png", "x": 0, "y": 0, "width": 16, "height": 16 },
                { "index": 1, "name": "frame_002.png", "page": 1, "image": "sprite_sheet_002.png", "x": 0, "y": 0, "width": 16, "height": 16 },
                { "index": 2, "name": "frame_003.png", "page": 2, "image": "sprite_sheet_003.png", "x": 0, "y": 0, "width": 16, "height": 16 }
            ]
        })
        .to_string(),
    )
    .unwrap();

    let error = validate_pack_layout(&pack).unwrap_err();

    assert!(matches!(error, PackError::InvalidAssetPath(path) if path == "sprite_sheet_003.png"));
}

#[test]
fn single_page_atlas_without_images_still_validates() {
    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    write_pack_fixture(&pack, 2);

    validate_pack_layout(&pack).unwrap();
}

#[test]
fn validates_v3_terrain_pack_without_legacy_frames() {
    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("terrain.gsfpack");
    write_world_pack_fixture(&pack, "terrain_set");

    validate_pack_layout(&pack).unwrap();
    let summary = read_pack_summary(&pack).unwrap();
    let inspection = forge_pack::inspect_pack(&pack).unwrap();

    assert_eq!(summary.frame_count, 0);
    assert_eq!(summary.preview_gif, "preview.png");
    assert_eq!(inspection.asset_type, "terrain_set");
    assert_eq!(inspection.atlas_path, pack.join("assets/terrain-atlas.png"));
}

#[test]
fn v3_map_pack_requires_self_contained_dependency_directories() {
    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("world.gsfpack");
    write_world_pack_fixture(&pack, "map");
    fs::remove_dir_all(pack.join("assets/dependencies")).unwrap();

    let error = validate_pack_layout(&pack).unwrap_err();

    assert!(matches!(error, PackError::MissingFile(path) if path == "assets/dependencies"));
}

#[cfg(unix)]
#[test]
fn symlinked_required_file_fails_validation() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    write_pack_fixture(&pack, 1);
    let external_manifest = temp.path().join("external-forgepack.json");
    fs::copy(pack.join("forgepack.json"), &external_manifest).unwrap();
    fs::remove_file(pack.join("forgepack.json")).unwrap();
    symlink(&external_manifest, pack.join("forgepack.json")).unwrap();

    assert!(validate_pack_layout(&pack).is_err());
}

#[cfg(unix)]
#[test]
fn symlinked_frames_directory_fails_validation() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    write_pack_fixture(&pack, 2);
    let external_frames = temp.path().join("external-frames");
    fs::rename(pack.join("assets/frames"), &external_frames).unwrap();
    symlink(&external_frames, pack.join("assets/frames")).unwrap();

    assert!(validate_pack_layout(&pack).is_err());
}

#[cfg(unix)]
#[test]
fn symlinked_parent_asset_directory_fails_validation() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    write_pack_fixture(&pack, 2);
    let external_assets = temp.path().join("external-assets");
    fs::rename(pack.join("assets"), &external_assets).unwrap();
    symlink(&external_assets, pack.join("assets")).unwrap();

    let error = validate_pack_layout(&pack).unwrap_err();

    assert!(matches!(error, PackError::InvalidAssetPath(path) if path == "assets/frames"));
}

#[cfg(unix)]
#[test]
fn symlinked_pack_root_fails_validation() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    let symlinked_pack = temp.path().join("linked-hero.gsfpack");
    write_pack_fixture(&pack, 1);
    symlink(&pack, &symlinked_pack).unwrap();

    assert!(validate_pack_layout(&symlinked_pack).is_err());
}

#[cfg(unix)]
#[test]
fn symlinked_frame_pngs_are_not_counted() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    write_pack_fixture(&pack, 1);
    let external_frame = temp.path().join("external-frame.png");
    fs::write(&external_frame, b"png").unwrap();
    symlink(&external_frame, pack.join("assets/frames/frame_999.png")).unwrap();

    let summary = read_pack_summary(&pack).unwrap();

    assert_eq!(summary.frame_count, 1);
}

fn write_pack_fixture(pack: &Path, frame_count: usize) {
    fs::create_dir_all(pack.join("previews")).unwrap();
    fs::create_dir_all(pack.join("assets/frames")).unwrap();

    fs::write(pack.join("previews/preview.gif"), b"GIF89a").unwrap();
    fs::write(pack.join("assets/sprite_sheet.png"), b"png").unwrap();
    fs::write(
        pack.join("assets/atlas.json"),
        atlas_json(frame_count).to_string(),
    )
    .unwrap();
    fs::write(
        pack.join("assets/manifest.json"),
        manifest_json(frame_count).to_string(),
    )
    .unwrap();
    fs::write(
        pack.join("quality-report.json"),
        quality_json(frame_count).to_string(),
    )
    .unwrap();
    fs::write(
        pack.join("forgepack.json"),
        forgepack_json(frame_count).to_string(),
    )
    .unwrap();

    for index in 1..=frame_count {
        fs::write(
            pack.join("assets/frames")
                .join(format!("frame_{index:03}.png")),
            b"png",
        )
        .unwrap();
    }
}

fn write_world_pack_fixture(pack: &Path, asset_type: &str) {
    fs::create_dir_all(pack.join("assets")).unwrap();
    fs::write(pack.join("preview.png"), b"png").unwrap();
    fs::write(pack.join("assets/manifest.json"), b"{}").unwrap();
    fs::write(pack.join("assets/godot_import.json"), b"{}").unwrap();
    fs::write(pack.join("quality-report.json"), b"{}").unwrap();
    let (type_assets, dependencies) = match asset_type {
        "terrain_set" => {
            fs::write(pack.join("assets/terrain-manifest.json"), b"{}").unwrap();
            fs::write(pack.join("assets/terrain-atlas.png"), b"png").unwrap();
            (
                json!({
                    "terrainManifest": "assets/terrain-manifest.json",
                    "atlasImage": "assets/terrain-atlas.png"
                }),
                None,
            )
        }
        "map" => {
            fs::write(pack.join("assets/map-manifest.json"), b"{}").unwrap();
            fs::write(pack.join("assets/map-layout.json"), b"{}").unwrap();
            fs::write(pack.join("validation-report.json"), b"{}").unwrap();
            fs::create_dir_all(pack.join("assets/dependencies")).unwrap();
            fs::create_dir_all(pack.join("assets/runtime")).unwrap();
            (
                json!({
                    "mapManifest": "assets/map-manifest.json",
                    "mapLayout": "assets/map-layout.json",
                    "validationReport": "validation-report.json"
                }),
                Some(json!([])),
            )
        }
        _ => unreachable!(),
    };
    let mut assets = json!({
        "manifest": "assets/manifest.json",
        "godotHelper": "assets/godot_import.json",
        "qualityReport": "quality-report.json"
    });
    for (key, value) in type_assets.as_object().unwrap() {
        assets[key] = value.clone();
    }
    let mut metadata = json!({
        "schemaVersion": "3.0.0",
        "assetType": asset_type,
        "id": "world-asset",
        "name": "World Asset",
        "version": "1.0.0",
        "createdAt": "2026-08-03T00:00:00Z",
        "creator": {"name": "Game Sprite Forge"},
        "license": {"type": "MIT"},
        "source": {
            "kind": if asset_type == "map" { "deterministic_compiler" } else { "provider_generation" }
        },
        "assets": assets,
        "previews": {"image": "preview.png"}
    });
    if let Some(dependencies) = dependencies {
        metadata["dependencies"] = dependencies;
    }
    fs::write(pack.join("forgepack.json"), metadata.to_string()).unwrap();
}

fn forgepack_json(frame_count: usize) -> serde_json::Value {
    json!({
        "schemaVersion": "1.0.0",
        "id": "hero",
        "name": "Hero",
        "version": "0.1.0",
        "createdAt": "2026-06-04T00:00:00Z",
        "creator": { "name": "Game Sprite Forge" },
        "license": { "type": "private" },
        "source": { "kind": "import_frames" },
        "animations": [{
            "name": "idle",
            "frames": (0..frame_count).collect::<Vec<_>>(),
            "fps": 12.0,
            "loop": true
        }],
        "assets": {
            "frames": "assets/frames",
            "spriteSheet": "assets/sprite_sheet.png",
            "atlas": "assets/atlas.json",
            "manifest": "assets/manifest.json",
            "qualityReport": "quality-report.json"
        },
        "previews": { "gif": "previews/preview.gif" }
    })
}

fn atlas_json(frame_count: usize) -> serde_json::Value {
    json!({
        "image": "sprite_sheet.png",
        "frameWidth": 16,
        "frameHeight": 16,
        "columns": frame_count.max(1),
        "rows": 1,
        "frames": (0..frame_count).map(|index| json!({
            "index": index,
            "name": format!("frame_{:03}.png", index + 1),
            "x": index * 16,
            "y": 0,
            "width": 16,
            "height": 16
        })).collect::<Vec<_>>()
    })
}

fn manifest_json(frame_count: usize) -> serde_json::Value {
    json!({
        "name": "Hero",
        "sheet": {
            "image": "assets/sprite_sheet.png",
            "frameWidth": 16,
            "frameHeight": 16,
            "columns": frame_count.max(1),
            "rows": 1
        },
        "animations": [{
            "name": "idle",
            "frames": (0..frame_count).collect::<Vec<_>>(),
            "fps": 12.0,
            "loop": true
        }],
        "anchor": { "type": "feet", "x": 8.0, "y": 16.0 }
    })
}

fn quality_json(frame_count: usize) -> serde_json::Value {
    json!({
        "verdict": "game_ready",
        "metrics": {
            "bboxBottomDriftPx": 0.0,
            "bboxCenterXDriftPx": 0.0,
            "bboxCenterYDriftPx": 0.0,
            "bboxWidthVariationPx": 0.0,
            "alphaCoverageAvg": 0.25,
            "loopMatchScore": 1.0,
            "frameCount": frame_count,
            "frameSizeConsistent": true,
            "cellBoundarySafe": true
        },
        "recommendations": [],
        "notes": []
    })
}

#[test]
fn frame_durations_must_match_animation_frames() {
    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    write_pack_fixture(&pack, 8);
    let mut forgepack: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.join("forgepack.json")).unwrap()).unwrap();
    forgepack["animations"][0]["frameDurationsMs"] = json!(vec![100_u64; 7]);
    fs::write(
        pack.join("forgepack.json"),
        serde_json::to_vec_pretty(&forgepack).unwrap(),
    )
    .unwrap();

    assert!(matches!(
        validate_pack_layout(&pack).unwrap_err(),
        PackError::SchemaValidation { document, message }
            if document == "forgepack.json"
                && message.contains("8 frames but 7 frameDurationsMs values")
    ));
}

#[test]
fn manifest_frame_durations_tampering_fails_timing_validation() {
    let (temp, pack) = write_timing_pack_fixture();
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.join("assets/manifest.json")).unwrap()).unwrap();
    manifest["animations"][0]["frameDurationsMs"] = json!(vec![125_u64; 8]);
    fs::write(
        pack.join("assets/manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();

    assert_timing_validation_fails(&pack, "assets/manifest.json", "frameDurationsMs differs");
    drop(temp);
}

#[test]
fn manifest_fps_tampering_fails_timing_validation() {
    let (temp, pack) = write_timing_pack_fixture();
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.join("assets/manifest.json")).unwrap()).unwrap();
    manifest["animations"][0]["fps"] = json!(12.0);
    fs::write(
        pack.join("assets/manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();

    assert_timing_validation_fails(&pack, "assets/manifest.json", "fps differs");
    drop(temp);
}

#[test]
fn godot_helper_frame_durations_tampering_fails_timing_validation() {
    let (temp, pack) = write_timing_pack_fixture();
    let mut helper: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.join("assets/godot_import.json")).unwrap()).unwrap();
    helper["spriteFrames"]["animations"][0]["frameDurationsMs"] = json!(vec![125_u64; 8]);
    fs::write(
        pack.join("assets/godot_import.json"),
        serde_json::to_vec_pretty(&helper).unwrap(),
    )
    .unwrap();

    assert_timing_validation_fails(
        &pack,
        "assets/godot_import.json",
        "frameDurationsMs differs",
    );
    drop(temp);
}

#[test]
fn validates_matching_godot_rendering_contract() {
    let (_temp, pack) = write_timing_pack_fixture();
    let rendering = json!({
        "profile": "godot-sprite-rendering@1.0.0",
        "textureFilter": "nearest",
        "pixelSnap": true,
        "mirrorPolicy": "auto"
    });
    let manifest_path = pack.join("assets/manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["rendering"] = rendering.clone();
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let helper_path = pack.join("assets/godot_import.json");
    let mut helper: serde_json::Value =
        serde_json::from_slice(&fs::read(&helper_path).unwrap()).unwrap();
    helper["spriteFrames"]["rendering"] = rendering;
    fs::write(&helper_path, serde_json::to_vec_pretty(&helper).unwrap()).unwrap();

    validate_pack_layout(&pack).unwrap();
}

#[test]
fn validates_right_only_rendering_contract() {
    let (_temp, pack) = write_right_only_timing_pack_fixture(false);

    validate_pack_layout(&pack).unwrap();
}

#[test]
fn animation_blend_modes_require_valid_matching_metadata() {
    for blend in [
        json!("normal"),
        json!("add"),
        json!("multiply"),
        json!("screen"),
        json!(null),
        json!(3),
    ] {
        let (_temp, pack) = write_right_only_timing_pack_fixture(false);
        for (relative, pointer) in [
            ("assets/manifest.json", "/rendering"),
            ("assets/godot_import.json", "/spriteFrames/rendering"),
        ] {
            let path = pack.join(relative);
            let mut value: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            value.pointer_mut(pointer).unwrap()["blendMode"] = blend.clone();
            fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        }
        let valid = matches!(blend.as_str(), Some("normal" | "add" | "multiply"));
        assert_eq!(validate_pack_layout(&pack).is_ok(), valid, "blend={blend}");
    }
    let (_temp, pack) = write_right_only_timing_pack_fixture(false);
    let path = pack.join("assets/godot_import.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["spriteFrames"]["rendering"]["blendMode"] = json!("add");
    fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    assert_timing_validation_fails(&pack, "assets/godot_import.json", "rendering differs");
}

#[test]
fn right_only_rendering_contract_rejects_left_animations() {
    let (_temp, pack) = write_right_only_timing_pack_fixture(true);

    assert_timing_validation_fails(
        &pack,
        "assets/manifest.json",
        "right_only forbids idle_left and walk_left",
    );
}

#[test]
fn right_only_rendering_contract_requires_right_pair() {
    let (_temp, pack) = write_right_only_timing_pack_fixture(false);
    for (relative, pointer) in [
        ("forgepack.json", "/animations"),
        ("assets/manifest.json", "/animations"),
        ("assets/godot_import.json", "/spriteFrames/animations"),
    ] {
        let path = pack.join(relative);
        let mut document: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        document
            .pointer_mut(pointer)
            .and_then(serde_json::Value::as_array_mut)
            .unwrap()
            .retain(|animation| animation["name"] != "walk_right");
        fs::write(path, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
    }

    assert_timing_validation_fails(
        &pack,
        "assets/manifest.json",
        "right_only requires animations idle_right, walk_right",
    );
}

#[test]
fn rejects_mismatched_godot_rendering_contract() {
    let (_temp, pack) = write_timing_pack_fixture();
    let manifest_path = pack.join("assets/manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["rendering"] = json!({
        "profile": "godot-sprite-rendering@1.0.0",
        "textureFilter": "nearest",
        "pixelSnap": true,
        "mirrorPolicy": "auto"
    });
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let helper_path = pack.join("assets/godot_import.json");
    let mut helper: serde_json::Value =
        serde_json::from_slice(&fs::read(&helper_path).unwrap()).unwrap();
    helper["spriteFrames"]["rendering"] = json!({
        "profile": "godot-sprite-rendering@1.0.0",
        "textureFilter": "linear",
        "pixelSnap": false,
        "mirrorPolicy": "auto"
    });
    fs::write(&helper_path, serde_json::to_vec_pretty(&helper).unwrap()).unwrap();

    assert_timing_validation_fails(&pack, "assets/godot_import.json", "rendering differs");
}

#[test]
fn godot_helper_fps_tampering_fails_timing_validation() {
    let (temp, pack) = write_timing_pack_fixture();
    let mut helper: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.join("assets/godot_import.json")).unwrap()).unwrap();
    helper["spriteFrames"]["animations"][0]["fps"] = json!(12.0);
    fs::write(
        pack.join("assets/godot_import.json"),
        serde_json::to_vec_pretty(&helper).unwrap(),
    )
    .unwrap();

    assert_timing_validation_fails(&pack, "assets/godot_import.json", "fps differs");
    drop(temp);
}

#[test]
fn equivalent_f32_fps_spellings_share_one_runtime_cadence() {
    let (temp, pack) = write_timing_pack_fixture();
    let short_f32 = 6.855_184_f32;
    for path in ["forgepack.json", "assets/manifest.json"] {
        let path = pack.join(path);
        let mut document: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        document["animations"][0]["fps"] = json!(short_f32);
        fs::write(path, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
    }
    let helper_path = pack.join("assets/godot_import.json");
    let mut helper: serde_json::Value =
        serde_json::from_slice(&fs::read(&helper_path).unwrap()).unwrap();
    helper["spriteFrames"]["animations"][0]["fps"] = json!(f64::from(short_f32));
    fs::write(helper_path, serde_json::to_vec_pretty(&helper).unwrap()).unwrap();

    validate_pack_layout(&pack).unwrap();
    drop(temp);
}

#[test]
fn helper_animation_removal_fails_modern_timing_validation() {
    let (temp, pack) = write_timing_pack_fixture();
    let mut helper: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.join("assets/godot_import.json")).unwrap()).unwrap();
    helper["spriteFrames"]["animations"] = json!([]);
    fs::write(
        pack.join("assets/godot_import.json"),
        serde_json::to_vec_pretty(&helper).unwrap(),
    )
    .unwrap();

    assert_timing_validation_fails(&pack, "assets/godot_import.json", "animation names differ");
    drop(temp);
}

#[test]
fn helper_fps_removal_fails_modern_timing_validation() {
    let (temp, pack) = write_timing_pack_fixture();
    let mut helper: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.join("assets/godot_import.json")).unwrap()).unwrap();
    helper["spriteFrames"]["animations"][0]
        .as_object_mut()
        .unwrap()
        .remove("fps");
    fs::write(
        pack.join("assets/godot_import.json"),
        serde_json::to_vec_pretty(&helper).unwrap(),
    )
    .unwrap();

    assert_timing_validation_fails(&pack, "assets/godot_import.json", "fps is missing");
    drop(temp);
}

#[test]
fn helper_frame_durations_removal_fails_modern_timing_validation() {
    let (temp, pack) = write_timing_pack_fixture();
    let mut helper: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.join("assets/godot_import.json")).unwrap()).unwrap();
    helper["spriteFrames"]["animations"][0]
        .as_object_mut()
        .unwrap()
        .remove("frameDurationsMs");
    fs::write(
        pack.join("assets/godot_import.json"),
        serde_json::to_vec_pretty(&helper).unwrap(),
    )
    .unwrap();

    assert_timing_validation_fails(
        &pack,
        "assets/godot_import.json",
        "frameDurationsMs is missing",
    );
    drop(temp);
}

#[test]
fn all_documents_may_omit_legacy_frame_durations() {
    let (temp, pack) = write_timing_pack_fixture();
    let mut forgepack: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.join("forgepack.json")).unwrap()).unwrap();
    forgepack["animations"][0]
        .as_object_mut()
        .unwrap()
        .remove("frameDurationsMs");
    fs::write(
        pack.join("forgepack.json"),
        serde_json::to_vec_pretty(&forgepack).unwrap(),
    )
    .unwrap();
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.join("assets/manifest.json")).unwrap()).unwrap();
    manifest["animations"][0]
        .as_object_mut()
        .unwrap()
        .remove("frameDurationsMs");
    fs::write(
        pack.join("assets/manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let mut helper: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.join("assets/godot_import.json")).unwrap()).unwrap();
    helper["spriteFrames"]["animations"][0]
        .as_object_mut()
        .unwrap()
        .remove("frameDurationsMs");
    fs::write(
        pack.join("assets/godot_import.json"),
        serde_json::to_vec_pretty(&helper).unwrap(),
    )
    .unwrap();

    validate_pack_layout(&pack).unwrap();
    drop(temp);
}

fn write_timing_pack_fixture() -> (tempfile::TempDir, std::path::PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let pack = temp.path().join("hero.gsfpack");
    write_pack_fixture(&pack, 8);
    let animation = json!({
        "name": "idle",
        "frames": [0, 1, 2, 3, 4, 5, 6, 7],
        "fps": 10.0,
        "loop": true,
        "frameDurationsMs": [100, 100, 100, 100, 100, 100, 100, 100]
    });
    let mut forgepack: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.join("forgepack.json")).unwrap()).unwrap();
    forgepack["animations"] = json!([animation.clone()]);
    forgepack["assets"]["godotHelper"] = json!("assets/godot_import.json");
    fs::write(
        pack.join("forgepack.json"),
        serde_json::to_vec_pretty(&forgepack).unwrap(),
    )
    .unwrap();

    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.join("assets/manifest.json")).unwrap()).unwrap();
    manifest["animations"] = json!([animation.clone()]);
    fs::write(
        pack.join("assets/manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    fs::write(
        pack.join("assets/godot_import.json"),
        json!({ "spriteFrames": { "animations": [animation] } }).to_string(),
    )
    .unwrap();
    (temp, pack)
}

fn write_right_only_timing_pack_fixture(
    include_left: bool,
) -> (tempfile::TempDir, std::path::PathBuf) {
    let (temp, pack) = write_timing_pack_fixture();
    let idle_right = json!({
        "name": "idle_right",
        "frames": [0, 1, 2, 3],
        "fps": 10.0,
        "loop": true,
        "frameDurationsMs": [100, 100, 100, 100]
    });
    let walk_right = json!({
        "name": "walk_right",
        "frames": [4, 5, 6, 7],
        "fps": 10.0,
        "loop": true,
        "frameDurationsMs": [100, 100, 100, 100]
    });
    let idle_left = json!({
        "name": "idle_left",
        "frames": [0, 1, 2, 3],
        "fps": 10.0,
        "loop": true,
        "frameDurationsMs": [100, 100, 100, 100]
    });
    let walk_left = json!({
        "name": "walk_left",
        "frames": [4, 5, 6, 7],
        "fps": 10.0,
        "loop": true,
        "frameDurationsMs": [100, 100, 100, 100]
    });
    let mut animations = vec![idle_right, walk_right];
    if include_left {
        animations.extend([idle_left, walk_left]);
    }
    let rendering = json!({
        "profile": "godot-sprite-rendering@1.0.0",
        "textureFilter": "nearest",
        "pixelSnap": true,
        "mirrorPolicy": "right_only"
    });

    let forgepack_path = pack.join("forgepack.json");
    let mut forgepack: serde_json::Value =
        serde_json::from_slice(&fs::read(&forgepack_path).unwrap()).unwrap();
    forgepack["animations"] = json!(animations.clone());
    fs::write(
        &forgepack_path,
        serde_json::to_vec_pretty(&forgepack).unwrap(),
    )
    .unwrap();

    let manifest_path = pack.join("assets/manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["animations"] = json!(animations.clone());
    manifest["rendering"] = rendering.clone();
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let helper_path = pack.join("assets/godot_import.json");
    let mut helper: serde_json::Value =
        serde_json::from_slice(&fs::read(&helper_path).unwrap()).unwrap();
    helper["spriteFrames"]["animations"] = json!(animations);
    helper["spriteFrames"]["rendering"] = rendering;
    fs::write(&helper_path, serde_json::to_vec_pretty(&helper).unwrap()).unwrap();
    (temp, pack)
}

fn assert_timing_validation_fails(pack: &Path, document: &str, message: &str) {
    assert!(matches!(
        validate_pack_layout(pack).unwrap_err(),
        PackError::SchemaValidation {
            document: error_document,
            message: error_message,
        } if error_document == document && error_message.contains(message)
    ));
}

#[test]
fn equal_total_duration_does_not_hide_changed_cadence() {
    let (_temp, pack) = write_timing_pack_fixture();
    let path = pack.join("assets/godot_import.json");
    let mut helper: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    helper["spriteFrames"]["animations"][0]["frameDurationsMs"] =
        json!([90, 110, 100, 100, 100, 100, 100, 100]);
    fs::write(path, serde_json::to_vec(&helper).unwrap()).unwrap();
    assert_timing_validation_fails(
        &pack,
        "assets/godot_import.json",
        "frameDurationsMs differs",
    );
}

#[test]
fn declared_human_review_must_exist_and_match_its_schema() {
    let (_temp, pack) = write_timing_pack_fixture();
    let path = pack.join("forgepack.json");
    let mut metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    metadata["assets"]["animationHumanReview"] = json!("quality/animation-human-review.json");
    fs::write(path, serde_json::to_vec(&metadata).unwrap()).unwrap();
    assert!(matches!(
        validate_pack_layout(&pack).unwrap_err(),
        PackError::MissingFile(_)
    ));
    fs::create_dir(pack.join("quality")).unwrap();
    fs::write(pack.join("quality/animation-human-review.json"), b"{}").unwrap();
    assert!(matches!(
        validate_pack_layout(&pack).unwrap_err(),
        PackError::SchemaValidation { .. }
    ));
}
