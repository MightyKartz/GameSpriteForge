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
