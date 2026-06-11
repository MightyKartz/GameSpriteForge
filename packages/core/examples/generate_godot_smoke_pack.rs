use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use forge_core::export::{export_pack, GifBackground, PreviewGifParameters, SpriteSheetParameters};
use forge_core::frames::{normalize_frames, CanvasMode, NormalizeOptions};
use forge_core::matting::{process_chroma_batch, ChromaParameters};
use forge_core::quality::compute_quality_report;
use forge_core::video::{slice_sprite_sheet_grid, SliceSpriteSheetParams};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_root = env::args()
        .nth(1)
        .map(PathBuf::from)
        .expect("usage: generate_godot_smoke_pack <output-root>");
    prepare_output_root(&output_root)?;

    let repo = Path::new("/Users/kartz/Development/Forge");
    let fixture = repo
        .join("examples/inputs/manual-qa/sprite-sheet/forge-walk-sheet.png")
        .canonicalize()?;
    let job_dir = output_root.join("forge-job");
    let exports_dir = output_root.join("exports");
    let normalized_dir = job_dir.join("processed/normalized");

    let sliced = slice_sprite_sheet_grid(&SliceSpriteSheetParams {
        sheet_path: fixture,
        output_directory: job_dir.clone(),
        frame_width: 64,
        frame_height: 64,
        columns: 4,
        rows: 2,
    })?;

    let processed = process_chroma_batch(
        &sliced.frames,
        job_dir.join("processed"),
        &ChromaParameters::default(),
    )?;
    let processed_paths = processed
        .frames
        .iter()
        .map(|frame| processed.processed_dir.join(&frame.frame))
        .collect::<Vec<_>>();
    let images = processed_paths
        .iter()
        .map(|path| Ok(image::open(path)?.to_rgba8()))
        .collect::<Result<Vec<_>, image::ImageError>>()?;

    let normalized = normalize_frames(
        &images,
        NormalizeOptions {
            mode: CanvasMode::SquareBottom,
            margin_bottom: 16,
            margin: 12,
            alpha_threshold: 0,
            manual_anchor: None,
        },
    );
    fs::create_dir_all(&normalized_dir)?;
    let normalized_paths = normalized
        .iter()
        .enumerate()
        .map(|(index, frame)| {
            let path = normalized_dir.join(format!("frame_{:05}.png", index + 1));
            frame.image.save(&path)?;
            Ok(path)
        })
        .collect::<Result<Vec<_>, image::ImageError>>()?;

    let bboxes = normalized
        .iter()
        .map(|frame| frame.bbox)
        .collect::<Vec<_>>();
    let sizes = normalized
        .iter()
        .map(|frame| frame.size)
        .collect::<Vec<_>>();
    let quality_report = compute_quality_report(&bboxes, &sizes);
    let anchor = normalized.first().expect("normalized frames").anchor;

    let output = export_pack(forge_core::export::ExportPackParams {
        exports_dir,
        export_id: "godot_smoke_walk".to_string(),
        frame_paths: normalized_paths,
        sheet: SpriteSheetParameters {
            columns: 4,
            padding_px: 2,
            margin_px: 2,
            max_texture_size: 2048,
            allow_multi_sheet: false,
        },
        gif: PreviewGifParameters {
            fps: 12.0,
            loop_animation: true,
            background: GifBackground::Checkerboard,
        },
        metadata: forge_core::export::PackMetadataParams {
            id: "godot-smoke-walk".to_string(),
            name: "Godot Smoke Walk".to_string(),
            version: "0.1.0".to_string(),
            creator_name: "Game Sprite Forge".to_string(),
            license_type: "private".to_string(),
            source_kind: "import_sprite_sheet".to_string(),
            source_name: Some("forge-walk-sheet.png".to_string()),
            animation_name: "walk".to_string(),
            animation_frames: None,
            fps: 12.0,
            loop_animation: true,
            anchor,
            quality_report,
        },
    })?;

    forge_pack::validate_pack_layout(&output.pack_dir)?;
    let summary = forge_pack::read_pack_summary(&output.pack_dir)?;

    println!("PACK_DIR={}", output.pack_dir.display());
    println!("FRAME_COUNT={}", summary.frame_count);
    println!("SPRITE_SHEET={}", output.sprite_sheet_path.display());
    println!("GODOT_HELPER={}", output.godot_helper_path.display());

    let summary_json = serde_json::json!({
        "packDir": output.pack_dir,
        "frameCount": summary.frame_count,
        "spriteSheet": output.sprite_sheet_path,
        "godotHelper": output.godot_helper_path,
        "previewGif": output.preview_gif_path,
    });
    fs::write(
        output_root.join("generated-pack-summary.json"),
        serde_json::to_vec_pretty(&summary_json)?,
    )?;

    Ok(())
}

fn prepare_output_root(output_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let artifact_name = output_root
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("smoke output root must have a directory name")?;
    if !artifact_name.starts_with("godot-pack-smoke-") {
        return Err(format!(
            "refusing to clean non-smoke output root: {}",
            output_root.display()
        )
        .into());
    }

    fs::create_dir_all(output_root)?;
    for child in ["forge-job", "exports"] {
        let path = output_root.join(child);
        if path.exists() {
            fs::remove_dir_all(path)?;
        }
    }

    let summary = output_root.join("generated-pack-summary.json");
    if summary.exists() {
        fs::remove_file(summary)?;
    }
    Ok(())
}
