pub mod gif;
pub mod godot;
pub mod manifest;
pub mod sheet;
pub mod sheet_layout;

use std::fs;
use std::path::{Path, PathBuf};

use crate::quality::QualityVerdict;

pub use gif::{build_preview_gif, GifBackground, PreviewGifOutput, PreviewGifParameters};
pub use godot::{export_godot_project, GodotProjectExportOutput, GodotProjectExportParams};
pub use manifest::{export_metadata, EngineManifest, ExportMetadata, PackMetadataParams};
pub use sheet::{build_sprite_sheet, Atlas, AtlasFrame, SpriteSheetOutput, SpriteSheetParameters};
pub use sheet_layout::{
    plan_sprite_sheet_layout, SpriteSheetFramePlacement, SpriteSheetLayout, SpriteSheetLayoutError,
    SpriteSheetLayoutParams, SpriteSheetPageLayout,
};

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("no frames were provided")]
    NoFrames,
    #[error("invalid export id: {0}")]
    InvalidExportId(String),
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),
    #[error("quality report is blocked; resolve quality issues before export")]
    QualityBlocked,
    #[error("frame dimensions must match")]
    FrameSizeMismatch,
    #[error("sprite sheet {width}x{height} exceeds max texture size {max_texture_size}")]
    TextureTooLarge {
        width: u32,
        height: u32,
        max_texture_size: u32,
    },
    #[error("gif dimensions {width}x{height} exceed GIF limits")]
    GifTooLarge { width: u32, height: u32 },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("gif encoding error: {0}")]
    Gif(#[from] ::gif::EncodingError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameSequenceOutput {
    pub export_dir: PathBuf,
    pub frames_dir: PathBuf,
    pub frame_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPackParams {
    pub exports_dir: PathBuf,
    pub export_id: String,
    pub frame_paths: Vec<PathBuf>,
    pub sheet: SpriteSheetParameters,
    pub gif: PreviewGifParameters,
    pub metadata: PackMetadataParams,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPackOutput {
    pub export_dir: PathBuf,
    pub frames_dir: PathBuf,
    pub frame_paths: Vec<PathBuf>,
    pub sprite_sheet_path: PathBuf,
    pub sprite_sheet_paths: Vec<PathBuf>,
    pub atlas_path: PathBuf,
    pub manifest_path: PathBuf,
    pub godot_helper_path: PathBuf,
    pub quality_report_path: PathBuf,
    pub preview_gif_path: PathBuf,
    pub pack_dir: PathBuf,
}

pub fn export_frame_sequence(
    frame_paths: &[PathBuf],
    exports_dir: &Path,
    export_id: &str,
) -> Result<FrameSequenceOutput, ExportError> {
    if frame_paths.is_empty() {
        return Err(ExportError::NoFrames);
    }
    if !is_safe_id(export_id) {
        return Err(ExportError::InvalidExportId(export_id.to_string()));
    }

    let export_dir = exports_dir.join(export_id);
    let frames_dir = export_dir.join("frames");
    fs::create_dir_all(&frames_dir)?;

    let mut exported = Vec::with_capacity(frame_paths.len());
    for (index, source) in frame_paths.iter().enumerate() {
        let target = frames_dir.join(format!("frame_{:03}.png", index + 1));
        fs::copy(source, &target)?;
        exported.push(target);
    }

    Ok(FrameSequenceOutput {
        export_dir,
        frames_dir,
        frame_paths: exported,
    })
}

pub fn export_pack(params: ExportPackParams) -> Result<ExportPackOutput, ExportError> {
    if params.metadata.quality_report.verdict == QualityVerdict::Blocked {
        return Err(ExportError::QualityBlocked);
    }

    let sequence =
        export_frame_sequence(&params.frame_paths, &params.exports_dir, &params.export_id)?;

    let sheet_output =
        build_sprite_sheet(&sequence.frame_paths, &sequence.export_dir, params.sheet)?;
    let preview_output = build_preview_gif(
        &sequence.frame_paths,
        &sequence.export_dir.join("preview.gif"),
        params.gif,
    )?;
    let metadata = export_metadata(params.metadata, &sheet_output.atlas);

    let manifest_path = sequence.export_dir.join("manifest.json");
    let godot_helper_path = sequence.export_dir.join("godot_import.json");
    let quality_report_path = sequence.export_dir.join("quality-report.json");
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&metadata.manifest)?,
    )?;
    fs::write(
        &godot_helper_path,
        serde_json::to_vec_pretty(&godot_import_helper(&metadata.manifest))?,
    )?;
    fs::write(
        &quality_report_path,
        serde_json::to_vec_pretty(&metadata.quality_report)?,
    )?;

    let pack_dir = sequence.export_dir.join(format!(
        "{}.gsfpack",
        sanitize_pack_name(&metadata.forgepack.name)
    ));
    write_pack_directory(
        &pack_dir,
        &metadata.forgepack,
        &sequence.frame_paths,
        &sheet_output.sprite_sheet_paths,
        &sheet_output.atlas_path,
        &manifest_path,
        &godot_helper_path,
        &quality_report_path,
        &preview_output.preview_gif_path,
    )?;

    Ok(ExportPackOutput {
        export_dir: sequence.export_dir,
        frames_dir: sequence.frames_dir,
        frame_paths: sequence.frame_paths,
        sprite_sheet_path: sheet_output.sprite_sheet_path,
        sprite_sheet_paths: sheet_output.sprite_sheet_paths,
        atlas_path: sheet_output.atlas_path,
        manifest_path,
        godot_helper_path,
        quality_report_path,
        preview_gif_path: preview_output.preview_gif_path,
        pack_dir,
    })
}

#[allow(clippy::too_many_arguments)]
fn write_pack_directory(
    pack_dir: &Path,
    forgepack: &manifest::ForgePackMetadata,
    frame_paths: &[PathBuf],
    sprite_sheet_paths: &[PathBuf],
    atlas_path: &Path,
    manifest_path: &Path,
    godot_helper_path: &Path,
    quality_report_path: &Path,
    preview_gif_path: &Path,
) -> Result<(), ExportError> {
    fs::create_dir_all(pack_dir.join("previews"))?;
    fs::create_dir_all(pack_dir.join("assets/frames"))?;

    fs::write(
        pack_dir.join("forgepack.json"),
        serde_json::to_vec_pretty(forgepack)?,
    )?;
    fs::copy(preview_gif_path, pack_dir.join("previews/preview.gif"))?;
    fs::copy(atlas_path, pack_dir.join("assets/atlas.json"))?;
    fs::copy(manifest_path, pack_dir.join("assets/manifest.json"))?;
    fs::copy(godot_helper_path, pack_dir.join("assets/godot_import.json"))?;
    fs::copy(quality_report_path, pack_dir.join("quality-report.json"))?;

    for sprite_sheet_path in sprite_sheet_paths {
        let Some(file_name) = sprite_sheet_path.file_name() else {
            continue;
        };
        fs::copy(sprite_sheet_path, pack_dir.join("assets").join(file_name))?;
    }

    for frame_path in frame_paths {
        let Some(file_name) = frame_path.file_name() else {
            continue;
        };
        fs::copy(frame_path, pack_dir.join("assets/frames").join(file_name))?;
    }

    Ok(())
}

fn godot_import_helper(manifest: &manifest::EngineManifest) -> serde_json::Value {
    let textures = if manifest.sheet.images.is_empty() {
        vec![manifest.sheet.image.clone()]
    } else {
        manifest.sheet.images.clone()
    };

    serde_json::json!({
        "engine": "godot",
        "format": "AnimatedSprite2D SpriteFrames helper",
        "spriteFrames": {
            "texture": manifest.sheet.image,
            "textures": textures,
            "atlas": "assets/atlas.json",
            "manifest": "assets/manifest.json",
            "frameWidth": manifest.sheet.frame_width,
            "frameHeight": manifest.sheet.frame_height,
            "columns": manifest.sheet.columns,
            "rows": manifest.sheet.rows,
            "animations": manifest.animations,
            "anchor": manifest.anchor
        },
        "importSteps": [
            "Copy every texture listed in this helper's spriteFrames.textures into the Godot project.",
            "Create a SpriteFrames resource for AnimatedSprite2D.",
            "Use the listed animation frames and FPS from this helper."
        ]
    })
}

fn is_safe_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn sanitize_pack_name(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect();

    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "pack".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frames::default_foot_anchor;
    use crate::quality::{QualityMetrics, QualityReport, QualityVerdict};
    use image::{Rgba, RgbaImage};
    use std::path::Path;

    #[test]
    fn frame_sequence_uses_three_digit_export_names() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.png");
        write_frame(&source, [255, 0, 0, 255]);

        let output =
            export_frame_sequence(&[source], &temp.path().join("exports"), "export_1").unwrap();

        assert_eq!(
            output.frame_paths[0].strip_prefix(temp.path()).unwrap(),
            Path::new("exports/export_1/frames/frame_001.png")
        );
        assert!(output.frames_dir.join("frame_001.png").is_file());
    }

    #[test]
    fn export_pack_writes_sheet_gif_manifest_and_gsfpack_layout() {
        let temp = tempfile::tempdir().unwrap();
        let source_a = temp.path().join("a.png");
        let source_b = temp.path().join("b.png");
        let source_c = temp.path().join("c.png");
        write_frame(&source_a, [255, 0, 0, 255]);
        write_frame(&source_b, [0, 0, 255, 255]);
        write_frame(&source_c, [0, 255, 0, 255]);

        let output = export_pack(ExportPackParams {
            exports_dir: temp.path().join("exports"),
            export_id: "export_1".to_string(),
            frame_paths: vec![source_a, source_b, source_c],
            sheet: SpriteSheetParameters {
                columns: 2,
                padding_px: 1,
                margin_px: 2,
                max_texture_size: 256,
                allow_multi_sheet: false,
            },
            gif: PreviewGifParameters {
                fps: 12.0,
                loop_animation: true,
                background: GifBackground::Checkerboard,
            },
            metadata: PackMetadataParams {
                id: "hero".to_string(),
                name: "Hero".to_string(),
                version: "0.1.0".to_string(),
                creator_name: "Game Sprite Forge".to_string(),
                license_type: "private".to_string(),
                source_kind: "import_frames".to_string(),
                source_name: None,
                animation_name: "idle".to_string(),
                animation_frames: Some(vec![1, 2]),
                fps: 12.0,
                loop_animation: true,
                anchor: default_foot_anchor(4, 4, 0),
                quality_report: quality_report(2),
            },
        })
        .unwrap();

        for relative in [
            "frames/frame_001.png",
            "frames/frame_002.png",
            "frames/frame_003.png",
            "sprite_sheet.png",
            "atlas.json",
            "manifest.json",
            "godot_import.json",
            "quality-report.json",
            "preview.gif",
            "Hero.gsfpack/forgepack.json",
            "Hero.gsfpack/previews/preview.gif",
            "Hero.gsfpack/assets/frames/frame_001.png",
            "Hero.gsfpack/assets/frames/frame_002.png",
            "Hero.gsfpack/assets/frames/frame_003.png",
            "Hero.gsfpack/assets/sprite_sheet.png",
            "Hero.gsfpack/assets/atlas.json",
            "Hero.gsfpack/assets/manifest.json",
            "Hero.gsfpack/assets/godot_import.json",
            "Hero.gsfpack/quality-report.json",
        ] {
            assert!(
                output.export_dir.join(relative).exists(),
                "missing export artifact {relative}"
            );
        }
        assert!(output.godot_helper_path.is_file());

        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&output.manifest_path).unwrap()).unwrap();
        assert_eq!(
            manifest["animations"][0]["frames"],
            serde_json::json!([1, 2])
        );

        let godot_helper: serde_json::Value =
            serde_json::from_slice(&fs::read(&output.godot_helper_path).unwrap()).unwrap();
        assert_eq!(godot_helper["engine"], "godot");
        assert_eq!(
            godot_helper["spriteFrames"]["animations"][0]["name"],
            "idle"
        );
    }

    #[test]
    fn export_pack_splits_high_resolution_sprite_sheets() {
        let temp = tempfile::tempdir().unwrap();
        let mut sources = Vec::new();
        for index in 0..30 {
            let source = temp.path().join(format!("source_{:03}.png", index + 1));
            write_sized_frame(
                &source,
                1304,
                696,
                [
                    (index * 7) as u8,
                    (index * 11) as u8,
                    (index * 17) as u8,
                    255,
                ],
            );
            sources.push(source);
        }

        let output = export_pack(ExportPackParams {
            exports_dir: temp.path().join("exports"),
            export_id: "export_1".to_string(),
            frame_paths: sources,
            sheet: SpriteSheetParameters {
                columns: 4,
                padding_px: 2,
                margin_px: 2,
                max_texture_size: 2048,
                allow_multi_sheet: true,
            },
            gif: PreviewGifParameters {
                fps: 12.0,
                loop_animation: true,
                background: GifBackground::Transparent,
            },
            metadata: PackMetadataParams {
                id: "hero".to_string(),
                name: "Hero".to_string(),
                version: "0.1.0".to_string(),
                creator_name: "Game Sprite Forge".to_string(),
                license_type: "private".to_string(),
                source_kind: "import_frames".to_string(),
                source_name: None,
                animation_name: "idle".to_string(),
                animation_frames: None,
                fps: 12.0,
                loop_animation: true,
                anchor: default_foot_anchor(1304, 696, 0),
                quality_report: quality_report(30),
            },
        })
        .unwrap();

        assert_eq!(output.frame_paths.len(), 30);
        assert_eq!(output.sprite_sheet_paths.len(), 15);
        assert!(output.export_dir.join("sprite_sheet.png").is_file());
        assert!(output.export_dir.join("sprite_sheet_002.png").is_file());
        assert!(output.pack_dir.join("assets/sprite_sheet.png").is_file());
        assert!(output
            .pack_dir
            .join("assets/sprite_sheet_002.png")
            .is_file());

        let atlas: serde_json::Value =
            serde_json::from_slice(&fs::read(&output.atlas_path).unwrap()).unwrap();
        assert_eq!(atlas["images"].as_array().unwrap().len(), 15);
        assert_eq!(atlas["frames"].as_array().unwrap().len(), 30);
        assert_eq!(atlas["frames"][2]["page"], serde_json::json!(1));
        assert_eq!(
            atlas["frames"][2]["image"],
            serde_json::json!("sprite_sheet_002.png")
        );

        let godot_helper: serde_json::Value =
            serde_json::from_slice(&fs::read(&output.godot_helper_path).unwrap()).unwrap();
        assert_eq!(
            godot_helper["spriteFrames"]["textures"]
                .as_array()
                .unwrap()
                .len(),
            15
        );
    }

    #[test]
    fn godot_helper_lists_all_multipage_textures() {
        let manifest = manifest::EngineManifest {
            name: "Hero Knight Pack".to_string(),
            sheet: manifest::ManifestSheet {
                image: "assets/sprite_sheet.png".to_string(),
                images: vec![
                    "assets/sprite_sheet.png".to_string(),
                    "assets/sprite_sheet_002.png".to_string(),
                ],
                frame_width: 64,
                frame_height: 64,
                columns: 4,
                rows: 4,
            },
            animations: vec![manifest::ManifestAnimation {
                name: "walk".to_string(),
                frames: vec![0, 1],
                fps: 12.0,
                loop_animation: true,
            }],
            anchor: manifest::ManifestAnchor {
                anchor_type: "feet".to_string(),
                x: 32.0,
                y: 64.0,
            },
        };

        let helper = godot_import_helper(&manifest);
        let textures = helper["spriteFrames"]["textures"].as_array().unwrap();

        assert_eq!(textures.len(), 2);
        assert_eq!(textures[0].as_str().unwrap(), "assets/sprite_sheet.png");
        assert_eq!(textures[1].as_str().unwrap(), "assets/sprite_sheet_002.png");
        assert_eq!(helper["spriteFrames"]["frameWidth"].as_u64().unwrap(), 64);
        assert_eq!(helper["spriteFrames"]["frameHeight"].as_u64().unwrap(), 64);
    }

    #[test]
    fn export_pack_rejects_blocked_quality_report() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("a.png");
        write_frame(&source, [255, 0, 0, 255]);

        let error = export_pack(ExportPackParams {
            exports_dir: temp.path().join("exports"),
            export_id: "export_1".to_string(),
            frame_paths: vec![source],
            sheet: SpriteSheetParameters {
                columns: 1,
                padding_px: 1,
                margin_px: 2,
                max_texture_size: 256,
                allow_multi_sheet: false,
            },
            gif: PreviewGifParameters {
                fps: 12.0,
                loop_animation: true,
                background: GifBackground::Checkerboard,
            },
            metadata: PackMetadataParams {
                id: "hero".to_string(),
                name: "Hero".to_string(),
                version: "0.1.0".to_string(),
                creator_name: "Game Sprite Forge".to_string(),
                license_type: "private".to_string(),
                source_kind: "import_frames".to_string(),
                source_name: None,
                animation_name: "idle".to_string(),
                animation_frames: Some(vec![1]),
                fps: 12.0,
                loop_animation: true,
                anchor: default_foot_anchor(4, 4, 0),
                quality_report: quality_report_with_verdict(1, QualityVerdict::Blocked),
            },
        })
        .unwrap_err();

        assert!(matches!(error, ExportError::QualityBlocked));
        assert!(!temp.path().join("exports/export_1").exists());
    }

    fn write_frame(path: &Path, rgba: [u8; 4]) {
        write_sized_frame(path, 4, 4, rgba);
    }

    fn write_sized_frame(path: &Path, width: u32, height: u32, rgba: [u8; 4]) {
        let image = RgbaImage::from_pixel(width, height, Rgba(rgba));
        image.save(path).unwrap();
    }

    fn quality_report(frame_count: usize) -> QualityReport {
        quality_report_with_verdict(frame_count, QualityVerdict::GameReady)
    }

    fn quality_report_with_verdict(frame_count: usize, verdict: QualityVerdict) -> QualityReport {
        QualityReport {
            verdict,
            metrics: QualityMetrics {
                bbox_bottom_drift_px: 0.0,
                bbox_center_x_drift_px: 0.0,
                bbox_center_y_drift_px: 0.0,
                bbox_width_variation_px: 0.0,
                alpha_coverage_avg: 1.0,
                loop_match_score: 1.0,
                frame_count,
                frame_size_consistent: true,
                cell_boundary_safe: true,
            },
            recommendations: Vec::new(),
            notes: Vec::new(),
        }
    }
}
