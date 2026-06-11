use base64::Engine;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DependencyStatus {
    ffmpeg_path: Option<String>,
    ffprobe_path: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportVideoParams {
    source_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportFramesParams {
    source_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportSpriteSheetParams {
    source_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListLocalPacksParams {
    exports_dir: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NormalizeFramesParams {
    frame_paths: Vec<String>,
    output_directory: String,
    options: forge_core::frames::NormalizeOptions,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QualityLoopRange {
    start_index: usize,
    end_index: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NormalizedFrameSummary {
    frame: String,
    path: PathBuf,
    bbox: forge_core::frames::FrameBbox,
    size: forge_core::frames::FrameSize,
    anchor: forge_core::frames::FootAnchor,
    warnings: Vec<forge_core::frames::NormalizeWarning>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NormalizeFramesResult {
    output_directory: PathBuf,
    frames: Vec<NormalizedFrameSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportGsfpackJobResult {
    job: forge_core::job::types::JobRecord,
    imported: forge_pack::ImportedPack,
    raw_frames: forge_core::video::ExtractFramesResult,
    preview_gif_path: PathBuf,
}

#[tauri::command]
fn check_ffmpeg(ffmpeg_path: Option<String>, ffprobe_path: Option<String>) -> DependencyStatus {
    let (ffmpeg, ffmpeg_error) = resolve_dependency_tool_path("ffmpeg", ffmpeg_path);
    let (ffprobe, ffprobe_error) = resolve_dependency_tool_path("ffprobe", ffprobe_path);

    let errors = [ffmpeg_error, ffprobe_error]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let message = if !errors.is_empty() {
        Some(errors.join("; "))
    } else if ffmpeg.is_none() || ffprobe.is_none() {
        Some("Install ffmpeg or choose an ffmpeg binary in Settings.".to_string())
    } else {
        None
    };

    DependencyStatus {
        ffmpeg_path: ffmpeg,
        ffprobe_path: ffprobe,
        message,
    }
}

fn resolve_dependency_tool_path(
    expected_name: &str,
    configured_path: Option<String>,
) -> (Option<String>, Option<String>) {
    let configured_path = configured_path
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty());

    if let Some(path) = configured_path {
        return match sanitize_configured_tool_path(expected_name, Some(PathBuf::from(path))) {
            Ok(path) => (path.map(|path| path.display().to_string()), None),
            Err(error) => (None, Some(format!("{expected_name}: {error}"))),
        };
    }

    (forge_core::video::ffmpeg::find_in_path(expected_name), None)
}

#[tauri::command]
fn sample_video_path(app: tauri::AppHandle) -> Result<String, String> {
    let resource_path = app
        .path()
        .resource_dir()
        .map_err(|error| error.to_string())?
        .join("examples/inputs/green-box-character.mp4");
    if resource_path.is_file() {
        return Ok(resource_path.to_string_lossy().into_owned());
    }

    let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../examples/inputs/green-box-character.mp4");
    fs::canonicalize(&dev_path)
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|error| format!("sample video is unavailable: {error}"))
}

#[tauri::command]
fn create_job(source_kind: String) -> Result<forge_core::job::types::JobRecord, String> {
    forge_core::job::store::JobStore::default_app_store()
        .map_err(|error| error.to_string())?
        .create_job(source_kind_from_code(&source_kind)?)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn import_video(params: ImportVideoParams) -> Result<forge_core::job::types::JobRecord, String> {
    let job = create_import_job(forge_core::job::types::SourceKind::ImportVideo)?;
    let source_path = Path::new(&params.source_path);
    let file_name = source_path
        .file_name()
        .ok_or_else(|| "source path has no file name".to_string())?;
    fs::copy(source_path, job.job_dir.join("source").join(file_name))
        .map_err(|error| error.to_string())?;
    mark_job_state(&job.job_dir, forge_core::job::types::JobState::SourceReady)
}

#[tauri::command]
fn import_frames(params: ImportFramesParams) -> Result<forge_core::job::types::JobRecord, String> {
    if params.source_paths.is_empty() {
        return Err("choose at least one PNG frame".to_string());
    }

    let job = create_import_job(forge_core::job::types::SourceKind::ImportFrames)?;
    for (index, source) in params.source_paths.iter().enumerate() {
        let source_path = Path::new(source);
        let extension = source_path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if !extension.eq_ignore_ascii_case("png") {
            return Err(format!("frame source must be a PNG: {source}"));
        }

        fs::copy(
            source_path,
            job.job_dir
                .join("source")
                .join(format!("frame_{:05}.png", index + 1)),
        )
        .map_err(|error| error.to_string())?;
    }

    mark_job_state(&job.job_dir, forge_core::job::types::JobState::SourceReady)
}

#[tauri::command]
fn import_sprite_sheet(
    params: ImportSpriteSheetParams,
) -> Result<forge_core::job::types::JobRecord, String> {
    let job = create_import_job(forge_core::job::types::SourceKind::ImportSpriteSheet)?;
    let source_path = Path::new(&params.source_path);
    let extension = source_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("png");
    fs::copy(
        source_path,
        job.job_dir
            .join("source")
            .join(format!("sprite_sheet.{extension}")),
    )
    .map_err(|error| error.to_string())?;
    mark_job_state(&job.job_dir, forge_core::job::types::JobState::SourceReady)
}

#[tauri::command]
fn probe_video(
    mut params: forge_core::video::ProbeVideoParams,
) -> Result<forge_core::video::VideoProbe, String> {
    ensure_job_scoped_existing_path(&params.input_path)?;
    params.configured_ffprobe_path =
        sanitize_configured_tool_path("ffprobe", params.configured_ffprobe_path)?;
    params.bundled_resource_path = None;
    forge_core::video::probe_video(&params).map_err(|error| error.to_string())
}

#[tauri::command]
fn extract_frames(
    mut params: forge_core::video::ExtractFramesParams,
) -> Result<forge_core::video::ExtractFramesResult, String> {
    let input_job = ensure_job_scoped_existing_path(&params.input_path)?;
    let output_job = ensure_job_scoped_output(&params.output_directory)?;
    ensure_same_job(&input_job, &output_job)?;
    params.configured_ffmpeg_path =
        sanitize_configured_tool_path("ffmpeg", params.configured_ffmpeg_path)?;
    params.bundled_resource_path = None;
    let result = forge_core::video::extract_frames(&params).map_err(|error| error.to_string())?;
    mark_job_state(
        &output_job,
        forge_core::job::types::JobState::FramesExtracted,
    )?;
    Ok(result)
}

#[tauri::command]
fn slice_sprite_sheet(
    params: forge_core::video::SliceSpriteSheetParams,
) -> Result<forge_core::video::ExtractFramesResult, String> {
    let input_job = ensure_job_scoped_existing_path(&params.sheet_path)?;
    let output_job = ensure_job_scoped_output(&params.output_directory)?;
    ensure_same_job(&input_job, &output_job)?;

    let result =
        forge_core::video::slice_sprite_sheet_grid(&params).map_err(|error| error.message)?;
    mark_job_state(
        &output_job,
        forge_core::job::types::JobState::FramesExtracted,
    )?;
    Ok(result)
}

#[tauri::command]
fn slice_sprite_sheet_transparent(
    params: forge_core::video::SliceSpriteSheetTransparentParams,
) -> Result<forge_core::video::ExtractFramesResult, String> {
    let input_job = ensure_job_scoped_existing_path(&params.sheet_path)?;
    let output_job = ensure_job_scoped_output(&params.output_directory)?;
    ensure_same_job(&input_job, &output_job)?;

    let result = forge_core::video::slice_sprite_sheet_transparent(&params)
        .map_err(|error| error.message)?;
    mark_job_state(
        &output_job,
        forge_core::job::types::JobState::FramesExtracted,
    )?;
    Ok(result)
}

#[tauri::command]
fn preview_chroma_frame(
    raw_frame_path: String,
    parameters: forge_core::matting::ChromaParameters,
    target_canvas_mode: forge_core::preview::TargetCanvasMode,
    previews_dir: String,
) -> Result<forge_core::preview::ChromaPreviewResult, String> {
    let input_job = ensure_job_scoped_existing_path(Path::new(&raw_frame_path))?;
    let output_job = ensure_job_scoped_output(Path::new(&previews_dir))?;
    ensure_same_job(&input_job, &output_job)?;
    let result = forge_core::preview::preview_chroma_frame(
        raw_frame_path,
        &parameters,
        target_canvas_mode,
        previews_dir,
    )
    .map_err(|error| error.to_string())?;
    mark_job_state(&output_job, forge_core::job::types::JobState::PreviewReady)?;
    Ok(result)
}

#[tauri::command]
fn process_chroma_batch(
    raw_frame_paths: Vec<String>,
    processed_dir: String,
    parameters: forge_core::matting::ChromaParameters,
) -> Result<forge_core::matting::BatchMattingResult, String> {
    let output_job = ensure_job_scoped_output(Path::new(&processed_dir))?;
    let paths = raw_frame_paths
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    for path in &paths {
        let input_job = ensure_job_scoped_existing_path(path)?;
        ensure_same_job(&input_job, &output_job)?;
    }
    let result = forge_core::matting::process_chroma_batch(&paths, processed_dir, &parameters)
        .map_err(|error| error.to_string())?;
    mark_job_state(&output_job, forge_core::job::types::JobState::Processed)?;
    Ok(result)
}

#[tauri::command]
fn normalize_frames(params: NormalizeFramesParams) -> Result<NormalizeFramesResult, String> {
    let output_directory = PathBuf::from(params.output_directory);
    let output_job = ensure_job_scoped_output(&output_directory)?;
    for path in &params.frame_paths {
        let input_job = ensure_job_scoped_existing_path(Path::new(path))?;
        ensure_same_job(&input_job, &output_job)?;
    }
    let images = params
        .frame_paths
        .iter()
        .map(|path| image::open(path).map(|image| image.to_rgba8()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let normalized = forge_core::frames::normalize_frames(&images, params.options);
    fs::create_dir_all(&output_directory).map_err(|error| error.to_string())?;

    let mut summaries = Vec::with_capacity(normalized.len());
    for (index, frame) in normalized.into_iter().enumerate() {
        let file_name = format!("frame_{:05}.png", index + 1);
        let path = output_directory.join(&file_name);
        frame.image.save(&path).map_err(|error| error.to_string())?;
        summaries.push(NormalizedFrameSummary {
            frame: file_name,
            path,
            bbox: frame.bbox,
            size: frame.size,
            anchor: frame.anchor,
            warnings: frame.warnings,
        });
    }

    mark_job_state(&output_job, forge_core::job::types::JobState::Processed)?;

    Ok(NormalizeFramesResult {
        output_directory,
        frames: summaries,
    })
}

#[tauri::command]
fn compute_quality_report(
    bboxes: Vec<forge_core::frames::FrameBbox>,
    sizes: Vec<forge_core::frames::FrameSize>,
    loop_range: Option<QualityLoopRange>,
) -> forge_core::quality::QualityReport {
    forge_core::quality::compute_quality_report_with_loop_range(
        &bboxes,
        &sizes,
        loop_range.map(|range| (range.start_index, range.end_index)),
    )
}

#[tauri::command]
fn export_pack(
    mut params: forge_core::export::ExportPackParams,
) -> Result<forge_core::export::ExportPackOutput, String> {
    let input_job = ensure_export_frame_inputs(&params.frame_paths)?;
    params.exports_dir = normalize_user_export_directory(&params.exports_dir)?;
    let result = forge_core::export::export_pack(params).map_err(|error| error.to_string())?;
    mark_job_state(&input_job, forge_core::job::types::JobState::Exported)?;
    Ok(result)
}

#[tauri::command]
fn export_godot_project(
    mut params: forge_core::export::GodotProjectExportParams,
) -> Result<forge_core::export::GodotProjectExportOutput, String> {
    params.output_dir = normalize_user_export_directory(&params.output_dir)?;
    forge_pack::read_pack_summary(&params.pack_dir).map_err(|error| error.to_string())?;
    forge_pack::validate_pack_layout(&params.pack_dir).map_err(|error| error.to_string())?;
    forge_core::export::export_godot_project(params).map_err(|error| error.to_string())
}

#[tauri::command]
fn import_gsfpack(path: String) -> Result<ImportGsfpackJobResult, String> {
    let imported = forge_pack::import_pack(Path::new(&path)).map_err(|error| error.to_string())?;
    let job = create_import_job(forge_core::job::types::SourceKind::ImportGsfpack)?;

    let raw_directory = job.job_dir.join("raw");
    fs::create_dir_all(&raw_directory).map_err(|error| error.to_string())?;
    remove_existing_png_frames(&raw_directory)?;
    let mut raw_frames = Vec::with_capacity(imported.frame_paths.len());
    for (index, frame_path) in imported.frame_paths.iter().enumerate() {
        let destination = raw_directory.join(format!("frame_{:05}.png", index + 1));
        fs::copy(frame_path, &destination).map_err(|error| error.to_string())?;
        raw_frames.push(destination);
    }

    let preview_source = imported.root.join(&imported.summary.preview_gif);
    let preview_gif_path = job.job_dir.join("previews/imported-preview.gif");
    fs::copy(&preview_source, &preview_gif_path).map_err(|error| error.to_string())?;
    let job = mark_job_state(
        &job.job_dir,
        forge_core::job::types::JobState::FramesExtracted,
    )?;

    Ok(ImportGsfpackJobResult {
        job,
        imported,
        raw_frames: forge_core::video::ExtractFramesResult {
            raw_directory,
            frames: raw_frames,
        },
        preview_gif_path,
    })
}

#[tauri::command]
fn validate_gsfpack(path: String) -> Result<forge_pack::PackSummary, String> {
    forge_pack::read_pack_summary(Path::new(&path)).map_err(|error| error.to_string())
}

#[tauri::command]
fn read_preview_image(path: String) -> Result<String, String> {
    let target = fs::canonicalize(Path::new(&path)).map_err(|error| error.to_string())?;
    let mime_type = preview_image_mime_type(&target)?;
    ensure_preview_image_scope(&target)?;

    let bytes = fs::read(&target).map_err(|error| error.to_string())?;
    const MAX_PREVIEW_IMAGE_BYTES: usize = 24 * 1024 * 1024;
    if bytes.len() > MAX_PREVIEW_IMAGE_BYTES {
        return Err("preview image is too large to embed".to_string());
    }

    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:{mime_type};base64,{encoded}"))
}

#[tauri::command]
fn list_local_packs(
    params: ListLocalPacksParams,
) -> Result<Vec<forge_pack::PackInspectSummary>, String> {
    let exports_dir = normalize_user_export_directory(Path::new(&params.exports_dir))?;
    let mut packs = Vec::new();

    if !exports_dir.exists() {
        return Ok(packs);
    }

    let canonical_exports_dir =
        fs::canonicalize(&exports_dir).map_err(|error| error.to_string())?;

    for entry in fs::read_dir(&canonical_exports_dir).map_err(|error| error.to_string())? {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        if is_gsfpack_path(&path) {
            if let Some(candidate) = canonical_pack_candidate(&path, &canonical_exports_dir) {
                if let Ok(summary) = forge_pack::inspect_pack(&candidate) {
                    packs.push(summary);
                }
            }
            continue;
        }

        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            let Ok(nested_entries) = fs::read_dir(&path) else {
                continue;
            };
            for nested in nested_entries {
                let Ok(nested) = nested else {
                    continue;
                };
                let nested_path = nested.path();
                if let Some(candidate) =
                    canonical_pack_candidate(&nested_path, &canonical_exports_dir)
                {
                    if let Ok(summary) = forge_pack::inspect_pack(&candidate) {
                        packs.push(summary);
                    }
                }
            }
        }
    }

    packs.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.root.cmp(&right.root))
    });
    Ok(packs)
}

fn canonical_pack_candidate(path: &Path, canonical_exports_dir: &Path) -> Option<PathBuf> {
    if !is_gsfpack_path(path) {
        return None;
    }
    let metadata = fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return None;
    }
    let candidate = fs::canonicalize(path).ok()?;
    if candidate.starts_with(canonical_exports_dir) {
        Some(candidate)
    } else {
        None
    }
}

#[tauri::command]
fn inspect_local_pack(path: String) -> Result<forge_pack::PackInspectSummary, String> {
    forge_pack::inspect_pack(Path::new(&path)).map_err(|error| error.to_string())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            check_ffmpeg,
            sample_video_path,
            create_job,
            import_video,
            import_frames,
            import_sprite_sheet,
            probe_video,
            preview_chroma_frame,
            extract_frames,
            slice_sprite_sheet,
            slice_sprite_sheet_transparent,
            process_chroma_batch,
            normalize_frames,
            compute_quality_report,
            export_pack,
            export_godot_project,
            import_gsfpack,
            validate_gsfpack,
            read_preview_image,
            list_local_packs,
            inspect_local_pack
        ])
        .run(tauri::generate_context!())
        .expect("error while running Game Sprite Forge");
}

fn source_kind_from_code(value: &str) -> Result<forge_core::job::types::SourceKind, String> {
    match value {
        "import_video" => Ok(forge_core::job::types::SourceKind::ImportVideo),
        "import_frames" => Ok(forge_core::job::types::SourceKind::ImportFrames),
        "import_sprite_sheet" => Ok(forge_core::job::types::SourceKind::ImportSpriteSheet),
        "import_gsfpack" => Ok(forge_core::job::types::SourceKind::ImportGsfpack),
        _ => Err(format!("unsupported MVP source kind: {value}")),
    }
}

fn create_import_job(
    source_kind: forge_core::job::types::SourceKind,
) -> Result<forge_core::job::types::JobRecord, String> {
    forge_core::job::store::JobStore::default_app_store()
        .map_err(|error| error.to_string())?
        .create_job(source_kind)
        .map_err(|error| error.to_string())
}

fn mark_job_state(
    job_dir: &Path,
    state: forge_core::job::types::JobState,
) -> Result<forge_core::job::types::JobRecord, String> {
    let job_id = job_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "job directory has no job id".to_string())?;
    forge_core::job::store::JobStore::default_app_store()
        .map_err(|error| error.to_string())?
        .set_state(job_id, state)
        .map_err(|error| error.to_string())
}

fn ensure_job_scoped_output(path: &Path) -> Result<PathBuf, String> {
    let root = canonical_job_root()?;
    let target = absolute_path(path).map_err(|error| error.to_string())?;
    let existing_ancestor = nearest_existing_ancestor(&target);
    let existing_ancestor =
        fs::canonicalize(existing_ancestor).map_err(|error| error.to_string())?;

    if target.starts_with(&root) && existing_ancestor.starts_with(&root) {
        job_dir_for_path(&target)
    } else {
        Err("output paths must stay under Game Sprite Forge jobs".to_string())
    }
}

fn ensure_job_scoped_existing_path(path: &Path) -> Result<PathBuf, String> {
    let root = canonical_job_root()?;
    let target = fs::canonicalize(path).map_err(|error| error.to_string())?;

    if target.starts_with(&root) {
        job_dir_for_path(&target)
    } else {
        Err("input paths must stay under Game Sprite Forge jobs".to_string())
    }
}

fn ensure_export_frame_inputs(frame_paths: &[PathBuf]) -> Result<PathBuf, String> {
    let Some(first_path) = frame_paths.first() else {
        return Err("export at least one processed frame".to_string());
    };
    let first_job = ensure_job_scoped_existing_path(first_path)?;
    for path in &frame_paths[1..] {
        let job = ensure_job_scoped_existing_path(path)?;
        ensure_same_job(&first_job, &job)?;
    }
    Ok(first_job)
}

fn normalize_user_export_directory(path: &Path) -> Result<PathBuf, String> {
    if path.as_os_str().is_empty() {
        return Err("choose an export folder".to_string());
    }
    let expanded = expand_home_path(path)?;
    let absolute = absolute_path(&expanded).map_err(|error| error.to_string())?;
    if absolute.parent().is_none() {
        return Err("export folder cannot be the filesystem root".to_string());
    }
    Ok(absolute)
}

fn expand_home_path(path: &Path) -> Result<PathBuf, String> {
    let Some(text) = path.to_str() else {
        return Ok(path.to_path_buf());
    };
    if text == "~" {
        return home_dir();
    }
    if let Some(rest) = text.strip_prefix("~/") {
        return Ok(home_dir()?.join(rest));
    }
    Ok(path.to_path_buf())
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "could not locate the home directory".to_string())
}

fn ensure_same_job(left: &Path, right: &Path) -> Result<(), String> {
    if left == right {
        Ok(())
    } else {
        Err("stage inputs and outputs must belong to the same job".to_string())
    }
}

fn job_dir_for_path(path: &Path) -> Result<PathBuf, String> {
    let root = canonical_job_root()?;
    let mut current = path;

    loop {
        if current.join("job.json").is_file() {
            return fs::canonicalize(current).map_err(|error| error.to_string());
        }
        if current == root {
            break;
        }
        let Some(parent) = current.parent() else {
            break;
        };
        current = parent;
    }

    Err("path must be inside a Game Sprite Forge job directory".to_string())
}

fn canonical_job_root() -> Result<PathBuf, String> {
    let store =
        forge_core::job::store::JobStore::default_app_store().map_err(|error| error.to_string())?;
    fs::canonicalize(store.root()).map_err(|error| error.to_string())
}

fn nearest_existing_ancestor(path: &Path) -> &Path {
    let mut current = path;
    while !current.exists() {
        let Some(parent) = current.parent() else {
            return current;
        };
        current = parent;
    }
    current
}

fn sanitize_configured_tool_path(
    expected_name: &str,
    value: Option<PathBuf>,
) -> Result<Option<PathBuf>, String> {
    let Some(path) = value else {
        return Ok(None);
    };
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("{expected_name} path has no file name"))?;
    if file_name != expected_name {
        return Err(format!(
            "configured tool path must point to {expected_name}"
        ));
    }
    let metadata = fs::metadata(&path).map_err(|error| error.to_string())?;
    if !metadata.is_file() {
        return Err(format!("configured {expected_name} path must be a file"));
    }
    if !is_executable_file(&metadata) {
        return Err(format!(
            "configured {expected_name} path must be executable"
        ));
    }
    fs::canonicalize(path)
        .map(Some)
        .map_err(|error| error.to_string())
}

#[cfg(unix)]
fn is_executable_file(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable_file(_metadata: &fs::Metadata) -> bool {
    true
}

fn remove_existing_png_frames(raw_directory: &Path) -> Result<(), String> {
    for entry in fs::read_dir(raw_directory).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        let is_png = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case("png"))
            .unwrap_or(false);
        if is_png {
            fs::remove_file(path).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn ensure_preview_image_scope(path: &Path) -> Result<(), String> {
    if ensure_job_scoped_existing_path(path).is_ok() {
        return Ok(());
    }

    let default_exports = absolute_path(&home_dir()?.join("Game Sprite Forge/Exports"))
        .map_err(|error| error.to_string())?;
    if path.starts_with(&default_exports) {
        return Ok(());
    }

    Err(
        "preview images must stay inside Game Sprite Forge jobs or the default Exports folder"
            .to_string(),
    )
}

fn preview_image_mime_type(path: &Path) -> Result<&'static str, String> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("");
    if extension.eq_ignore_ascii_case("png") {
        return Ok("image/png");
    }
    if extension.eq_ignore_ascii_case("gif") {
        return Ok("image/gif");
    }
    if extension.eq_ignore_ascii_case("jpg") || extension.eq_ignore_ascii_case("jpeg") {
        return Ok("image/jpeg");
    }
    Err("preview image must be PNG, GIF, or JPEG".to_string())
}

fn is_gsfpack_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("gsfpack"))
        .unwrap_or(false)
}

fn absolute_path(path: &Path) -> std::io::Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    Ok(normalize_path_components(&absolute))
}

fn normalize_path_components(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn configured_tool_path_must_match_expected_binary_name() {
        let temp = tempfile::tempdir().unwrap();
        let wrong = temp.path().join("not-ffmpeg");
        std::fs::write(&wrong, b"").unwrap();

        let error = sanitize_configured_tool_path("ffmpeg", Some(wrong)).unwrap_err();

        assert_eq!(error, "configured tool path must point to ffmpeg");
    }

    #[test]
    fn configured_tool_path_canonicalizes_matching_file() {
        let temp = tempfile::tempdir().unwrap();
        let binary = temp.path().join("ffmpeg");
        std::fs::write(&binary, b"").unwrap();
        make_executable(&binary);

        let resolved = sanitize_configured_tool_path("ffmpeg", Some(binary.clone()))
            .unwrap()
            .unwrap();

        assert_eq!(resolved, binary.canonicalize().unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn configured_tool_path_requires_executable_file() {
        let temp = tempfile::tempdir().unwrap();
        let binary = temp.path().join("ffmpeg");
        std::fs::write(&binary, b"").unwrap();

        let error = sanitize_configured_tool_path("ffmpeg", Some(binary)).unwrap_err();

        assert_eq!(error, "configured ffmpeg path must be executable");
    }

    #[test]
    fn check_ffmpeg_reports_invalid_configured_path_without_path_fallback() {
        let temp = tempfile::tempdir().unwrap();
        let wrong = temp.path().join("not-ffmpeg");
        std::fs::write(&wrong, b"").unwrap();

        let status = check_ffmpeg(Some(wrong.display().to_string()), None);

        assert!(status.ffmpeg_path.is_none());
        assert!(status
            .message
            .unwrap()
            .contains("ffmpeg: configured tool path must point to ffmpeg"));
    }

    #[test]
    fn list_local_packs_lists_direct_and_one_level_nested_valid_packs() {
        let temp = tempfile::tempdir().unwrap();
        let exports = temp.path().join("exports");
        let direct = exports.join("direct.gsfpack");
        let nested = exports.join("collection/nested.gsfpack");
        write_pack_fixture(&direct, "direct", "Direct Pack", 2);
        write_pack_fixture(&nested, "nested", "Nested Pack", 3);

        let packs = list_local_packs(ListLocalPacksParams {
            exports_dir: exports.display().to_string(),
        })
        .unwrap();

        let names = packs
            .iter()
            .map(|pack| pack.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["Direct Pack", "Nested Pack"]);
    }

    #[test]
    fn list_local_packs_ignores_broken_pack_folders() {
        let temp = tempfile::tempdir().unwrap();
        let exports = temp.path().join("exports");
        let valid = exports.join("valid.gsfpack");
        let broken = exports.join("broken.gsfpack");
        write_pack_fixture(&valid, "valid", "Valid Pack", 1);
        std::fs::create_dir_all(&broken).unwrap();
        std::fs::write(broken.join("forgepack.json"), "{}").unwrap();

        let packs = list_local_packs(ListLocalPacksParams {
            exports_dir: exports.display().to_string(),
        })
        .unwrap();

        let names = packs
            .iter()
            .map(|pack| pack.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["Valid Pack"]);
    }

    #[cfg(unix)]
    #[test]
    fn list_local_packs_skips_symlinked_pack_and_nested_directory() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let exports = temp.path().join("exports");
        let outside = temp.path().join("outside");
        let valid = exports.join("valid.gsfpack");
        let outside_pack = outside.join("escaped.gsfpack");
        let outside_nested_pack = outside.join("nested/nested-escaped.gsfpack");
        write_pack_fixture(&valid, "valid", "Valid Pack", 1);
        write_pack_fixture(&outside_pack, "escaped", "Escaped Pack", 1);
        write_pack_fixture(
            &outside_nested_pack,
            "nested-escaped",
            "Nested Escaped Pack",
            1,
        );
        symlink(&outside_pack, exports.join("symlinked.gsfpack")).unwrap();
        symlink(outside.join("nested"), exports.join("symlinked-nested")).unwrap();

        let packs = list_local_packs(ListLocalPacksParams {
            exports_dir: exports.display().to_string(),
        })
        .unwrap();

        let names = packs
            .iter()
            .map(|pack| pack.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["Valid Pack"]);
    }

    fn make_executable(path: &Path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(path).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(path, permissions).unwrap();
        }
        #[cfg(not(unix))]
        {
            let _ = path;
        }
    }

    fn write_pack_fixture(pack: &Path, id: &str, name: &str, frame_count: usize) {
        std::fs::create_dir_all(pack.join("previews")).unwrap();
        std::fs::create_dir_all(pack.join("assets/frames")).unwrap();

        std::fs::write(pack.join("previews/preview.gif"), b"GIF89a").unwrap();
        std::fs::write(pack.join("assets/sprite_sheet.png"), b"png").unwrap();
        std::fs::write(
            pack.join("assets/atlas.json"),
            atlas_json(frame_count).to_string(),
        )
        .unwrap();
        std::fs::write(
            pack.join("assets/manifest.json"),
            manifest_json(name, frame_count).to_string(),
        )
        .unwrap();
        std::fs::write(
            pack.join("quality-report.json"),
            quality_json(frame_count).to_string(),
        )
        .unwrap();
        std::fs::write(
            pack.join("forgepack.json"),
            forgepack_json(id, name, frame_count).to_string(),
        )
        .unwrap();

        for index in 1..=frame_count {
            std::fs::write(
                pack.join("assets/frames")
                    .join(format!("frame_{index:03}.png")),
                b"png",
            )
            .unwrap();
        }
    }

    fn forgepack_json(id: &str, name: &str, frame_count: usize) -> serde_json::Value {
        json!({
            "schemaVersion": "1.0.0",
            "id": id,
            "name": name,
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

    fn manifest_json(name: &str, frame_count: usize) -> serde_json::Value {
        json!({
            "name": name,
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
}
