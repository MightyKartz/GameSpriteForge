use std::fs;
use std::path::Path;

use forge_core::project::{
    inspect_project, register_project_asset, ProjectAssetKind, ProjectPathKind,
    RegisterProjectAsset,
};
use forge_pack::{PackAnimationSummary, PackInspectSummary};
use tempfile::tempdir;

#[test]
fn project_manifest_registers_one_stable_asset_and_revisions_changed_packs() {
    let temp = tempdir().unwrap();
    let project = temp.path().join("game");
    let pack = temp.path().join("Knight.gsfpack");
    fs::create_dir(&project).unwrap();
    fs::create_dir(&pack).unwrap();
    fs::write(project.join("project.godot"), "[application]\n").unwrap();
    fs::write(pack.join("identity.txt"), "v1").unwrap();
    let summary = pack_summary(&pack, "asset-v1");

    register_project_asset(RegisterProjectAsset {
        project_path: &project,
        asset_key: "knight",
        pack_path: &pack,
        pack_sha256: "pack-sha-v1",
        godot_target: Path::new("addons/forge_assets/knight"),
        scene_path: Path::new("addons/forge_assets/knight/forge_animated_sprite.tscn"),
        sprite_frames_path: Path::new("addons/forge_assets/knight/forge_sprite_frames.tres"),
        usage_path: Path::new("addons/forge_assets/knight/forge_usage.json"),
        pack: &summary,
        provider_refs: &[],
        job_id: "job-1",
    })
    .unwrap();
    register_project_asset(RegisterProjectAsset {
        project_path: &project,
        asset_key: "knight",
        pack_path: &pack,
        pack_sha256: "pack-sha-v1",
        godot_target: Path::new("addons/forge_assets/knight"),
        scene_path: Path::new("addons/forge_assets/knight/forge_animated_sprite.tscn"),
        sprite_frames_path: Path::new("addons/forge_assets/knight/forge_sprite_frames.tres"),
        usage_path: Path::new("addons/forge_assets/knight/forge_usage.json"),
        pack: &summary,
        provider_refs: &[],
        job_id: "job-2",
    })
    .unwrap();

    let inspection = inspect_project(&project).unwrap();
    let first = inspection.manifest.assets.get("knight").unwrap();
    assert_eq!(inspection.manifest.assets.len(), 1);
    assert_eq!(first.revision, 1);
    assert_eq!(first.kind, ProjectAssetKind::Character);
    assert_eq!(first.pack.kind, ProjectPathKind::ExternalAbsolute);
    assert_eq!(first.last_job_id, "job-2");

    let changed = pack_summary(&pack, "asset-v1");
    register_project_asset(RegisterProjectAsset {
        project_path: &project,
        asset_key: "knight",
        pack_path: &pack,
        pack_sha256: "pack-sha-v2",
        godot_target: Path::new("addons/forge_assets/knight"),
        scene_path: Path::new("addons/forge_assets/knight/forge_animated_sprite.tscn"),
        sprite_frames_path: Path::new("addons/forge_assets/knight/forge_sprite_frames.tres"),
        usage_path: Path::new("addons/forge_assets/knight/forge_usage.json"),
        pack: &changed,
        provider_refs: &[],
        job_id: "job-3",
    })
    .unwrap();

    let revised = inspect_project(&project).unwrap();
    assert_eq!(revised.manifest.assets["knight"].revision, 2);
}

#[cfg(unix)]
#[test]
fn project_manifest_rejects_symlinked_forge_metadata_directory() {
    use std::os::unix::fs::symlink;

    let temp = tempdir().unwrap();
    let project = temp.path().join("game");
    let outside = temp.path().join("outside");
    fs::create_dir(&project).unwrap();
    fs::create_dir(&outside).unwrap();
    fs::write(project.join("project.godot"), "[application]\n").unwrap();
    symlink(&outside, project.join(".forge")).unwrap();

    let error = inspect_project(&project).unwrap_err();

    assert!(error.to_string().contains("symbolic link"));
    assert!(!outside.join("assets.json").exists());
}

fn pack_summary(root: &Path, id: &str) -> PackInspectSummary {
    PackInspectSummary {
        id: id.into(),
        name: "Knight".into(),
        version: "0.1.0".into(),
        frame_count: 6,
        preview_gif: "previews/preview.gif".into(),
        root: root.to_path_buf(),
        manifest_path: root.join("assets/manifest.json"),
        atlas_path: root.join("assets/atlas.json"),
        quality_report_path: root.join("quality-report.json"),
        default_animation: "idle".into(),
        animations: vec![
            PackAnimationSummary {
                name: "idle".into(),
                frame_count: 3,
                fps: 8.0,
                loop_animation: true,
            },
            PackAnimationSummary {
                name: "attack".into(),
                frame_count: 3,
                fps: 12.0,
                loop_animation: false,
            },
        ],
        asset_type: "character".into(),
        items: vec![],
    }
}
