use std::fs;
use std::path::{Path, PathBuf};

use forge_core::export::{export_pack, GifBackground, PreviewGifParameters, SpriteSheetParameters};
use forge_core::frames::{normalize_frames, CanvasMode, NormalizeOptions};
use forge_core::matting::{process_chroma_batch, ChromaParameters};
use forge_core::quality::compute_quality_report;
use forge_core::video::{
    extract_frames, probe_video, slice_sprite_sheet_grid, ExtractFramesParams, ProbeVideoParams,
    SliceSpriteSheetParams,
};

#[test]
fn sample_video_exports_schema_valid_reimportable_pack() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir
        .join("../../../examples/inputs/green-box-character.mp4")
        .canonicalize()
        .unwrap();

    let Some(ffmpeg_path) = forge_core::video::ffmpeg::find_in_path("ffmpeg").map(PathBuf::from)
    else {
        eprintln!("skipping sample pipeline e2e because ffmpeg is unavailable");
        return;
    };
    let Some(ffprobe_path) = forge_core::video::ffmpeg::find_in_path("ffprobe").map(PathBuf::from)
    else {
        eprintln!("skipping sample pipeline e2e because ffprobe is unavailable");
        return;
    };

    let temp = tempfile::tempdir().unwrap();
    let job_dir = temp.path().join("job");
    let output_dir = job_dir.join("processed/normalized");
    let exports_dir = job_dir.join("exports");

    let probe = probe_video(&ProbeVideoParams {
        input_path: fixture.clone(),
        configured_ffprobe_path: Some(ffprobe_path),
        bundled_resource_path: None,
    })
    .unwrap();
    assert_eq!(probe.width, 256);
    assert_eq!(probe.height, 256);

    let extracted = extract_frames(&ExtractFramesParams {
        input_path: fixture,
        start_time_seconds: 0.0,
        end_time_seconds: 1.0,
        keep_every_n_frames: 1,
        output_directory: job_dir.clone(),
        configured_ffmpeg_path: Some(ffmpeg_path),
        bundled_resource_path: None,
    })
    .unwrap();
    assert!(!extracted.frames.is_empty());

    let processed = process_chroma_batch(
        &extracted.frames,
        job_dir.join("processed"),
        &ChromaParameters::default(),
    )
    .unwrap();
    let processed_paths = processed
        .frames
        .iter()
        .map(|frame| processed.processed_dir.join(&frame.frame))
        .collect::<Vec<_>>();
    let images = processed_paths
        .iter()
        .map(|path| image::open(path).unwrap().to_rgba8())
        .collect::<Vec<_>>();

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
    std::fs::create_dir_all(&output_dir).unwrap();
    let normalized_paths = normalized
        .iter()
        .enumerate()
        .map(|(index, frame)| {
            let path = output_dir.join(format!("frame_{:05}.png", index + 1));
            frame.image.save(&path).unwrap();
            path
        })
        .collect::<Vec<_>>();

    let bboxes = normalized
        .iter()
        .map(|frame| frame.bbox)
        .collect::<Vec<_>>();
    let sizes = normalized
        .iter()
        .map(|frame| frame.size)
        .collect::<Vec<_>>();
    let quality_report = compute_quality_report(&bboxes, &sizes);
    let anchor = normalized.first().unwrap().anchor;

    let output = export_pack(forge_core::export::ExportPackParams {
        exports_dir,
        export_id: "sample_export".to_string(),
        frame_paths: normalized_paths,
        sheet: SpriteSheetParameters {
            columns: 6,
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
            id: "green-box-character".to_string(),
            name: "Green Box Character".to_string(),
            version: "0.1.0".to_string(),
            creator_name: "Game Sprite Forge".to_string(),
            license_type: "private".to_string(),
            source_kind: "import_video".to_string(),
            source_name: Some("green-box-character.mp4".to_string()),
            animation_name: "idle".to_string(),
            animation_frames: None,
            fps: 12.0,
            loop_animation: true,
            anchor,
            quality_report: quality_report.clone(),
        },
    })
    .unwrap();

    let summary = forge_pack::read_pack_summary(&output.pack_dir).unwrap();
    let imported = forge_pack::import_pack(&output.pack_dir).unwrap();

    assert_eq!(summary.frame_count, output.frame_paths.len());
    assert_eq!(imported.frame_paths.len(), output.frame_paths.len());
    assert!(output.preview_gif_path.is_file());

    let reexport = export_pack(forge_core::export::ExportPackParams {
        exports_dir: job_dir.join("reexports"),
        export_id: "sample_reexport".to_string(),
        frame_paths: imported.frame_paths.clone(),
        sheet: SpriteSheetParameters {
            columns: 6,
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
            id: "green-box-character-reexport".to_string(),
            name: "Green Box Character Reexport".to_string(),
            version: "0.1.0".to_string(),
            creator_name: "Game Sprite Forge".to_string(),
            license_type: "private".to_string(),
            source_kind: "import_gsfpack".to_string(),
            source_name: Some("Green Box Character.gsfpack".to_string()),
            animation_name: "idle".to_string(),
            animation_frames: None,
            fps: 12.0,
            loop_animation: true,
            anchor,
            quality_report,
        },
    })
    .unwrap();
    let reexported = forge_pack::read_pack_summary(&reexport.pack_dir).unwrap();
    forge_pack::validate_pack_layout(&reexport.pack_dir).unwrap();

    assert_eq!(reexport.frame_paths.len(), output.frame_paths.len());
    assert_eq!(reexported.frame_count, output.frame_paths.len());
}

#[test]
fn manual_qa_png_sequence_exports_schema_valid_reimportable_pack() {
    let fixture_dir = manual_qa_fixture_root().join("png-sequence");
    let raw_frames = sorted_pngs(&fixture_dir);
    assert_eq!(raw_frames.len(), 6);

    assert_pack_roundtrip(
        raw_frames,
        "manual_qa_png_sequence",
        "manual-qa-png-sequence",
        "Manual QA PNG Sequence",
        "import_frames",
        "frame_001.png...frame_006.png",
        3,
    );
}

#[test]
fn manual_qa_sprite_sheet_exports_schema_valid_reimportable_pack() {
    let fixture = manual_qa_fixture_root()
        .join("sprite-sheet/forge-walk-sheet.png")
        .canonicalize()
        .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let sliced = slice_sprite_sheet_grid(&SliceSpriteSheetParams {
        sheet_path: fixture,
        output_directory: temp.path().join("job"),
        frame_width: 64,
        frame_height: 64,
        columns: 4,
        rows: 2,
    })
    .unwrap();
    let raw_frames = sliced.frames;
    assert_eq!(raw_frames.len(), 8);

    assert_pack_roundtrip(
        raw_frames,
        "manual_qa_sprite_sheet",
        "manual-qa-sprite-sheet",
        "Manual QA Sprite Sheet",
        "import_sprite_sheet",
        "forge-walk-sheet.png",
        4,
    );
}

fn assert_pack_roundtrip(
    raw_frames: Vec<PathBuf>,
    export_id: &str,
    pack_id: &str,
    pack_name: &str,
    source_kind: &str,
    source_name: &str,
    columns: u32,
) {
    let temp = tempfile::tempdir().unwrap();
    let job_dir = temp.path().join("job");
    let output_dir = job_dir.join("processed/normalized");
    let exports_dir = job_dir.join("exports");

    let processed = process_chroma_batch(
        &raw_frames,
        job_dir.join("processed"),
        &ChromaParameters::default(),
    )
    .unwrap();
    let processed_paths = processed
        .frames
        .iter()
        .map(|frame| processed.processed_dir.join(&frame.frame))
        .collect::<Vec<_>>();
    let images = processed_paths
        .iter()
        .map(|path| image::open(path).unwrap().to_rgba8())
        .collect::<Vec<_>>();

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
    fs::create_dir_all(&output_dir).unwrap();
    let normalized_paths = normalized
        .iter()
        .enumerate()
        .map(|(index, frame)| {
            let path = output_dir.join(format!("frame_{:05}.png", index + 1));
            frame.image.save(&path).unwrap();
            path
        })
        .collect::<Vec<_>>();

    let bboxes = normalized
        .iter()
        .map(|frame| frame.bbox)
        .collect::<Vec<_>>();
    let sizes = normalized
        .iter()
        .map(|frame| frame.size)
        .collect::<Vec<_>>();
    let quality_report = compute_quality_report(&bboxes, &sizes);
    let anchor = normalized.first().unwrap().anchor;

    let output = export_pack(forge_core::export::ExportPackParams {
        exports_dir,
        export_id: export_id.to_string(),
        frame_paths: normalized_paths,
        sheet: SpriteSheetParameters {
            columns,
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
            id: pack_id.to_string(),
            name: pack_name.to_string(),
            version: "0.1.0".to_string(),
            creator_name: "Game Sprite Forge".to_string(),
            license_type: "private".to_string(),
            source_kind: source_kind.to_string(),
            source_name: Some(source_name.to_string()),
            animation_name: "walk".to_string(),
            animation_frames: None,
            fps: 12.0,
            loop_animation: true,
            anchor,
            quality_report: quality_report.clone(),
        },
    })
    .unwrap();

    assert_export_layout(&output);
    let summary = forge_pack::read_pack_summary(&output.pack_dir).unwrap();
    let imported = forge_pack::import_pack(&output.pack_dir).unwrap();
    assert_eq!(summary.frame_count, output.frame_paths.len());
    assert_eq!(imported.frame_paths.len(), output.frame_paths.len());

    let reexport = export_pack(forge_core::export::ExportPackParams {
        exports_dir: job_dir.join("reexports"),
        export_id: format!("{export_id}_reexport"),
        frame_paths: imported.frame_paths.clone(),
        sheet: SpriteSheetParameters {
            columns,
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
            id: format!("{pack_id}-reexport"),
            name: format!("{pack_name} Reexport"),
            version: "0.1.0".to_string(),
            creator_name: "Game Sprite Forge".to_string(),
            license_type: "private".to_string(),
            source_kind: "import_gsfpack".to_string(),
            source_name: Some(format!("{pack_name}.gsfpack")),
            animation_name: "walk".to_string(),
            animation_frames: None,
            fps: 12.0,
            loop_animation: true,
            anchor,
            quality_report,
        },
    })
    .unwrap();
    assert_export_layout(&reexport);
    let reexported = forge_pack::read_pack_summary(&reexport.pack_dir).unwrap();
    assert_eq!(reexport.frame_paths.len(), output.frame_paths.len());
    assert_eq!(reexported.frame_count, output.frame_paths.len());
}

fn assert_export_layout(output: &forge_core::export::ExportPackOutput) {
    assert!(output.frames_dir.is_dir());
    assert!(output.sprite_sheet_path.is_file());
    assert!(output.atlas_path.is_file());
    assert!(output.manifest_path.is_file());
    assert!(output.godot_helper_path.is_file());
    assert!(output.quality_report_path.is_file());
    assert!(output.preview_gif_path.is_file());
    forge_pack::validate_pack_layout(&output.pack_dir).unwrap();
}

fn manual_qa_fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../examples/inputs/manual-qa")
        .canonicalize()
        .unwrap()
}

fn sorted_pngs(directory: &Path) -> Vec<PathBuf> {
    let mut paths = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| extension.eq_ignore_ascii_case("png"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
}
