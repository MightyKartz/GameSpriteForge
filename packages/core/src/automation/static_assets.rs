//! Local PNG intake for static Packs. This module has no Provider dependency.
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::Path,
};

use image::{imageops::FilterType, Rgba, RgbaImage};
use serde_json::json;

use super::{
    fingerprint_operation_inputs, AutomationOperation, AutomationRunError, PrepareStaticRequest,
};
use crate::{
    asset_project::{
        export_static_pack_with_source, hash_file, SamplingMode, StaticAssetItemSpecV1,
        StaticAssetKind, StaticAssetSetSpecV1, StaticCanvasPolicy, StaticPackItem,
        StaticPackSource,
    },
    job::{JobArtifactRecord, JobLifecycleState, JobRecord, JobState, JobStore},
};

const PROFILE: &str = "local-static-import@1.0.0";

pub(super) fn validate_request(request: &PrepareStaticRequest) -> Result<(), String> {
    if request.schema_version != "1" {
        return Err("prepare-static requires schemaVersion 1".into());
    }
    let valid_id = |id: &str| {
        !id.is_empty()
            && id.len() <= 80
            && id.as_bytes()[0].is_ascii_alphanumeric()
            && id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    };
    if !valid_id(&request.id) || request.name.trim().is_empty() || request.license.trim().is_empty()
    {
        return Err("prepare-static requires an engine-safe id, name, and explicit license".into());
    }
    match request.canvas_policy {
        StaticCanvasPolicy::Normalize => {
            if !request
                .canvas_size
                .is_some_and(|size| (64..=512).contains(&size) && size.is_power_of_two())
            {
                return Err(
                    "normalize requires canvasSize to be a power of two from 64 through 512".into(),
                );
            }
        }
        StaticCanvasPolicy::PreserveSource => {
            if request.canvas_size.is_some() || request.edge_padding_px != 0 {
                return Err("preserve_source does not accept canvasSize or nonzero edgePaddingPx; source pixels and origin are retained".into());
            }
        }
    }
    if request.items.is_empty() || request.items.len() > 64 {
        return Err("prepare-static requires 1..=64 items".into());
    }
    if request.foreground_alpha_threshold == 0 || request.edge_padding_px > 64 {
        return Err(
            "foregroundAlphaThreshold must be 1..=255 and edgePaddingPx must be 0..=64".into(),
        );
    }
    let mut ids = HashSet::new();
    let mut dimensions = None;
    for item in &request.items {
        if !valid_id(&item.id)
            || item.name.trim().is_empty()
            || !ids.insert(item.id.to_ascii_lowercase())
        {
            return Err(format!(
                "item ID must be engine-safe and unique (including case): {}",
                item.id
            ));
        }
        let image = read_png(&item.path, request.canvas_policy)?;
        if request.canvas_policy == StaticCanvasPolicy::PreserveSource {
            if dimensions.is_some_and(|size| size != image.dimensions()) {
                return Err("preserve_source items within one Pack must have the same source dimensions; use separate Packs for different canvases".into());
            }
            dimensions = Some(image.dimensions());
        }
        if !image
            .pixels()
            .any(|pixel| pixel[3] >= request.foreground_alpha_threshold)
        {
            return Err(format!(
                "no foreground reaches foregroundAlphaThreshold: {}",
                item.id
            ));
        }
    }
    Ok(())
}

fn read_png(path: &Path, policy: StaticCanvasPolicy) -> Result<RgbaImage, String> {
    let metadata = fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !metadata.is_file() || metadata.len() > 32 * 1024 * 1024 {
        return Err(format!(
            "PNG must be a regular file no larger than 32 MiB: {}",
            path.display()
        ));
    }
    let reader = image::ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    if reader.format() != Some(image::ImageFormat::Png) {
        return Err(format!("expected PNG content: {}", path.display()));
    }
    let (width, height) = reader.into_dimensions().map_err(|e| e.to_string())?;
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return Err("source PNG dimensions must be 1..=4096 pixels".into());
    }
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut container = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .map_err(|e| format!("invalid PNG input: {e}"))?;
    if container.info().animation_control.is_some() {
        return Err("static preparation accepts still PNGs, not APNG animations".into());
    }
    if policy == StaticCanvasPolicy::PreserveSource
        && (container.info().bit_depth != png::BitDepth::Eight
            || !matches!(
                container.info().color_type,
                png::ColorType::Rgb | png::ColorType::Rgba
            ))
    {
        return Err("preserve_source currently requires 8-bit RGB or RGBA PNG content".into());
    }
    container
        .finish()
        .map_err(|e| format!("invalid PNG container: {e}"))?;
    let decoded = image::open(path).map_err(|e| e.to_string())?;
    if policy == StaticCanvasPolicy::PreserveSource {
        if !matches!(
            decoded.color(),
            image::ColorType::Rgb8 | image::ColorType::Rgba8
        ) {
            return Err("preserve_source currently requires 8-bit RGB or RGBA PNG content".into());
        }
        return Ok(decoded.to_rgba8());
    }
    if !decoded.color().has_alpha() {
        return Err(format!(
            "source PNG must have an alpha channel: {}",
            path.display()
        ));
    }
    let image = decoded.to_rgba8();
    if !image.pixels().any(|p| p[3] == 0) || !image.pixels().any(|p| p[3] > 16) {
        return Err(format!(
            "source PNG needs transparent background and visible foreground: {}",
            path.display()
        ));
    }
    Ok(image)
}

struct StaticNormalization {
    image: RgbaImage,
    foreground_bounds: [u32; 4],
    crop_bounds: [u32; 4],
}

fn normalize(
    image: &RgbaImage,
    request: &PrepareStaticRequest,
) -> Result<StaticNormalization, String> {
    // The threshold locates the subject; it never modifies alpha values inside
    // the padded crop. This keeps hand-painted edges without allowing distant
    // nearly invisible generator residue to shrink or displace the subject.
    let mut left = image.width();
    let mut top = image.height();
    let mut right = 0;
    let mut bottom = 0;
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] >= request.foreground_alpha_threshold {
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
        }
    }
    if left == image.width() {
        return Err("no foreground reaches foregroundAlphaThreshold".into());
    }
    let foreground_bounds = [left, top, right + 1, bottom + 1];
    if request.canvas_policy == StaticCanvasPolicy::PreserveSource {
        return Ok(StaticNormalization {
            image: image.clone(),
            foreground_bounds,
            crop_bounds: [0, 0, image.width(), image.height()],
        });
    }
    left = left.saturating_sub(request.edge_padding_px);
    top = top.saturating_sub(request.edge_padding_px);
    right = (right + request.edge_padding_px).min(image.width() - 1);
    bottom = (bottom + request.edge_padding_px).min(image.height() - 1);
    let crop_bounds = [left, top, right + 1, bottom + 1];
    let cropped =
        image::imageops::crop_imm(image, left, top, right - left + 1, bottom - top + 1).to_image();
    let canvas = request.canvas_size.ok_or("normalize requires canvasSize")?;
    let usable = (canvas as f32 * 0.82).round() as u32;
    let ratio =
        (usable as f32 / cropped.width() as f32).min(usable as f32 / cropped.height() as f32);
    let width = (cropped.width() as f32 * ratio).round().max(1.0) as u32;
    let height = (cropped.height() as f32 * ratio).round().max(1.0) as u32;
    let filter = match request.sampling {
        SamplingMode::Nearest => FilterType::Nearest,
        SamplingMode::Linear => FilterType::Lanczos3,
    };
    let resized = image::imageops::resize(&cropped, width, height, filter);
    let mut output = RgbaImage::from_pixel(canvas, canvas, Rgba([0, 0, 0, 0]));
    let x = (canvas - width) / 2;
    let y = match request.kind {
        StaticAssetKind::IconSet => (canvas - height) / 2,
        StaticAssetKind::PropSet => canvas - height - canvas / 16,
    };
    image::imageops::overlay(&mut output, &resized, x.into(), y.into());
    Ok(StaticNormalization {
        image: output,
        foreground_bounds,
        crop_bounds,
    })
}

pub(super) fn run_prepare_static(
    store: &JobStore,
    job_id: &str,
    request: &PrepareStaticRequest,
) -> Result<JobRecord, AutomationRunError> {
    validate_request(request).map_err(AutomationRunError::Processing)?;
    let record = store.read_record(job_id)?;
    let fingerprint =
        fingerprint_operation_inputs(&AutomationOperation::PrepareStatic(request.clone()))?;
    if record.input_hash.as_deref() != Some(&fingerprint) {
        return Err(AutomationRunError::Processing(
            "local PNG inputs changed after planning".into(),
        ));
    }
    let source_dir = record.job_dir.join("source/static");
    let normalized_dir = record.job_dir.join("normalized/static");
    fs::create_dir_all(&source_dir)?;
    fs::create_dir_all(&normalized_dir)?;
    let mut items = Vec::new();
    let mut provenance = BTreeMap::new();
    let mut source_items = Vec::new();
    for item in &request.items {
        if store.read_record(job_id)?.cancellation_requested {
            return Err(AutomationRunError::Cancelled);
        }
        let source_path = source_dir.join(format!("{}.png", item.id));
        fs::copy(&item.path, &source_path)?;
        let image = read_png(&source_path, request.canvas_policy)
            .map_err(AutomationRunError::Processing)?;
        let source_hash =
            hash_file(&source_path).map_err(|e| AutomationRunError::Processing(e.to_string()))?;
        let output = normalize(&image, request).map_err(AutomationRunError::Processing)?;
        let output_path = normalized_dir.join(format!("{}.png", item.id));
        if request.canvas_policy == StaticCanvasPolicy::PreserveSource {
            // Preserve encoding, RGB/RGBA color mode, alpha, and every source pixel exactly.
            fs::copy(&source_path, &output_path)?;
        } else {
            output.image.save(&output_path)?;
        }
        let normalized_hash =
            hash_file(&output_path).map_err(|e| AutomationRunError::Processing(e.to_string()))?;
        provenance.insert(item.id.clone(), json!({"sourceKind":"import_png", "sha256":source_hash, "normalizedSha256":normalized_hash}));
        source_items.push(json!({"id":item.id,"sha256":source_hash,"normalizedSha256":normalized_hash,
            "sourceWidth":image.width(),"sourceHeight":image.height(),"outputWidth":output.image.width(),"outputHeight":output.image.height(),
            "foregroundBounds":output.foreground_bounds,"cropBounds":output.crop_bounds,
            "canvasPolicy":request.canvas_policy,"sourceBytesPreserved":source_hash == normalized_hash,
            "hasTransparentPixels":image.pixels().any(|p| p[3] == 0)}));
        items.push(StaticPackItem {
            id: item.id.clone(),
            name: item.name.clone(),
            image_path: output_path,
        });
    }
    if store.read_record(job_id)?.cancellation_requested {
        return Err(AutomationRunError::Cancelled);
    }
    if fingerprint_operation_inputs(&AutomationOperation::PrepareStatic(request.clone()))?
        != fingerprint
    {
        return Err(AutomationRunError::Processing(
            "local PNG inputs changed during import".into(),
        ));
    }
    let report = json!({"schemaVersion":"1","profile":PROFILE,"assetType":request.kind,
        "providerRequestOccurred":false,"providerRequestCount":0,"styleConsistencyEvaluated":false,
        "visualReviewRequired":true,"verdict":"game_ready","items":source_items,
        "checks":{"validPng":true,"visibleForeground":true,"transparentBackground":source_items.iter().all(|item| item["hasTransparentPixels"] == true),
            "uniformCanvas":true,"alphaPreservedWithoutChromaKey":true,
            "sourceCanvasPreserved":request.canvas_policy == StaticCanvasPolicy::PreserveSource},
        "normalization":{"canvasPolicy":request.canvas_policy,"foregroundAlphaThreshold":request.foreground_alpha_threshold,"edgePaddingPx":request.edge_padding_px},
        "notes":["game_ready describes local structural checks; source artwork still requires visual review", "bounds use exclusive right/bottom; edge padding is in source pixels and retained inside the normalized canvas"]});
    let report_path = record.job_dir.join("local-import-report.json");
    fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;
    let asset = StaticAssetSetSpecV1 {
        schema_version: "1".into(),
        kind: request.kind,
        id: request.id.clone(),
        name: request.name.clone(),
        license: request.license.clone(),
        items: request
            .items
            .iter()
            .map(|item| StaticAssetItemSpecV1 {
                id: item.id.clone(),
                name: item.name.clone(),
                prompt: String::new(),
                reference_image: None,
            })
            .collect(),
    };
    let source = StaticPackSource {
        sampling: request.sampling.clone(),
        canvas_policy: request.canvas_policy,
        item_provenance: provenance,
        source: json!({"kind":"import_frames","name":"Local PNG files",
            "metadata":{"operation":PROFILE,"providerRequestOccurred":false,"providerRequestCount":0,
                "inputFingerprint":record.input_hash,"recipeHash":record.recipe_hash,"inputs":source_items,
                "normalization":{"canvasPolicy":request.canvas_policy,"foregroundAlphaThreshold":request.foreground_alpha_threshold,"edgePaddingPx":request.edge_padding_px}}}),
    };
    let pack = export_static_pack_with_source(
        &record.job_dir.join("exports"),
        &asset,
        source,
        &items,
        &report,
    )
    .map_err(|e| AutomationRunError::Processing(e.to_string()))?;
    let pack_hash = super::runner::hash_directory(&pack.pack_dir)?;
    store
        .update_record(job_id, |record| {
            record.state = JobState::Exported;
            record.lifecycle_state = JobLifecycleState::Succeeded;
            record.progress = 1.0;
            record.worker_pid = None;
            for step in &mut record.steps {
                step.state = "succeeded".into();
            }
            record.artifacts.extend([
                JobArtifactRecord {
                    kind: "gsfpack".into(),
                    path: pack.pack_dir.clone(),
                    sha256: Some(pack_hash.clone()),
                },
                JobArtifactRecord {
                    kind: "contact_sheet".into(),
                    path: pack.contact_sheet_path.clone(),
                    sha256: None,
                },
                JobArtifactRecord {
                    kind: "quality_report".into(),
                    path: report_path.clone(),
                    sha256: hash_file(&report_path).ok(),
                },
            ]);
            record.next_actions = vec!["inspect_asset".into(), "plan_install_godot".into()];
        })
        .map_err(Into::into)
}
