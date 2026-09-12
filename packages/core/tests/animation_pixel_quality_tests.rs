use std::fs;
use std::path::{Path, PathBuf};

use forge_core::automation::{
    run_operation, stage_plan_job, AutomationOperation, PlanStore, PrepareAssetRequest,
};
use forge_core::frames::{bbox_from_image, FrameSize};
use forge_core::job::{JobLifecycleState, JobStore};
use forge_core::quality::{
    compute_quality_report_with_pixels, measure_pixel_sequence, QualityProfile,
    QualityRecommendationId, QualityVerdict,
};
use image::{Rgba, RgbaImage};
use tempfile::tempdir;

fn frame(left: u32, top: u32, width: u32, height: u32) -> RgbaImage {
    let mut image = RgbaImage::new(64, 64);
    for y in top..top + height {
        for x in left..left + width {
            image.put_pixel(x, y, Rgba([255, 80, 20, 255]));
        }
    }
    image
}

fn save_frames(directory: &Path, frames: &[RgbaImage]) -> Vec<PathBuf> {
    frames
        .iter()
        .enumerate()
        .map(|(index, image)| {
            let path = directory.join(format!("frame-{index}.png"));
            image.save(&path).unwrap();
            path
        })
        .collect()
}

fn quality(
    frames: &[RgbaImage],
    profile: QualityProfile,
    looping: bool,
    tail: bool,
) -> forge_core::quality::QualityReport {
    let directory = tempdir().unwrap();
    let paths = save_frames(directory.path(), frames);
    let bboxes = frames
        .iter()
        .map(|image| bbox_from_image(image, 1))
        .collect::<Vec<_>>();
    let sizes = frames
        .iter()
        .map(|image| FrameSize::new(image.width(), image.height()))
        .collect::<Vec<_>>();
    compute_quality_report_with_pixels(&bboxes, &sizes, &paths, &paths, looping, profile, tail)
        .unwrap()
}

#[test]
fn effect_deformation_does_not_change_character_quality_thresholds() {
    let frames = [frame(3, 5, 10, 10), frame(42, 30, 16, 28)];
    let character = quality(&frames, QualityProfile::Character, false, false);
    assert_eq!(character.verdict, QualityVerdict::PrototypeUsable);
    assert!(character
        .recommendations
        .contains(&QualityRecommendationId::AdjustAnchor));
    let effect = quality(&frames, QualityProfile::Effect, false, false);
    assert_eq!(effect.verdict, QualityVerdict::GameReady);
    assert!(!effect
        .recommendations
        .contains(&QualityRecommendationId::AdjustAnchor));
    let diagnostics = effect.pixel_diagnostics.unwrap();
    assert_eq!(diagnostics.visual_approval, "not_assessed");
    assert!(!diagnostics.loop_check_applicable);
    assert!(
        diagnostics.normalized.adjacent_differences[0]
            .as_ref()
            .unwrap()
            .alpha_mean_absolute_difference
            > 0.05
    );
}

#[test]
fn only_explicit_non_looping_effect_tail_can_be_empty() {
    let visible = frame(20, 20, 20, 20);
    let empty = RgbaImage::new(64, 64);
    let ending = [visible.clone(), empty.clone(), empty.clone()];
    assert_eq!(
        quality(&ending, QualityProfile::Effect, false, false).verdict,
        QualityVerdict::Blocked
    );
    assert_eq!(
        quality(&ending, QualityProfile::Character, false, true).verdict,
        QualityVerdict::Blocked
    );
    assert_eq!(
        quality(&ending, QualityProfile::Effect, true, true).verdict,
        QualityVerdict::Blocked
    );
    let allowed = quality(&ending, QualityProfile::Effect, false, true);
    assert_eq!(allowed.verdict, QualityVerdict::GameReady);
    assert_eq!(
        allowed.pixel_diagnostics.unwrap().transparent_tail_start,
        Some(1)
    );
    assert_eq!(
        quality(
            &[empty.clone(), empty.clone()],
            QualityProfile::Effect,
            false,
            true
        )
        .verdict,
        QualityVerdict::Blocked
    );
    assert_eq!(
        quality(
            &[visible.clone(), empty, visible],
            QualityProfile::Effect,
            false,
            true
        )
        .verdict,
        QualityVerdict::Blocked
    );
}

#[test]
fn evidence_compares_visible_color_and_ignores_rgb_under_zero_alpha() {
    let hidden_red = RgbaImage::from_pixel(4, 4, Rgba([255, 0, 0, 0]));
    let hidden_blue = RgbaImage::from_pixel(4, 4, Rgba([0, 0, 255, 0]));
    let hidden = measure_pixel_sequence(&[hidden_red, hidden_blue]);
    assert_eq!(hidden.frames[0].nonzero_rgb_under_zero_alpha_pixels, 16);
    assert_eq!(
        hidden
            .first_last_difference
            .unwrap()
            .premultiplied_rgb_mean_absolute_difference,
        0.0
    );
    let opaque_red = RgbaImage::from_pixel(4, 4, Rgba([255, 0, 0, 255]));
    let opaque_blue = RgbaImage::from_pixel(4, 4, Rgba([0, 0, 255, 255]));
    let visible = measure_pixel_sequence(&[opaque_red, opaque_blue]);
    let delta = visible.first_last_difference.unwrap();
    assert_eq!(delta.alpha_mean_absolute_difference, 0.0);
    assert!((delta.premultiplied_rgb_mean_absolute_difference - 2.0 / 3.0).abs() < 0.00001);
    assert!(delta.brightness_mean_absolute_difference > 0.1);
}

#[test]
fn effect_still_requires_consistent_dimensions_and_exposes_boundary_contact() {
    let mismatch = quality(
        &[frame(8, 8, 8, 8), RgbaImage::new(32, 32)],
        QualityProfile::Effect,
        false,
        true,
    );
    assert_eq!(mismatch.verdict, QualityVerdict::Blocked);
    assert!(mismatch
        .pixel_diagnostics
        .unwrap()
        .normalized
        .adjacent_differences[0]
        .is_none());
    let touching = quality(
        &[frame(0, 8, 8, 8), frame(0, 8, 8, 8)],
        QualityProfile::Effect,
        false,
        false,
    );
    assert_eq!(touching.verdict, QualityVerdict::NeedsCleanup);
    assert!(touching.pixel_diagnostics.unwrap().normalized.frames[0].alpha_touches_edge);
}

#[test]
fn single_action_non_looping_job_exports_without_loop_repair_and_records_gif_rounding() {
    let directory = tempdir().unwrap();
    let paths = save_frames(
        directory.path(),
        &[frame(24, 32, 16, 16), frame(16, 16, 32, 32)],
    );
    let request: PrepareAssetRequest = serde_json::from_value(serde_json::json!({
        "schemaVersion": "1", "input": {"kind": "png_sequence", "paths": paths},
        "metadata": {"name":"Burst", "animation":"burst", "fps":8.0, "loop":false, "creator":"fixture", "license":"private"},
        "matting": {"mode":"preserve_alpha"},
        "normalize": {"mode":"preserve_source", "margin":0, "marginBottom":0,
            "alphaThreshold":0, "manualAnchor":{"x":32.0,"y":48.0,"lockedByUser":true}},
        "quality": {"requireGameReady":true}
    })).unwrap();
    let plans = PlanStore::new(directory.path().join("plans")).unwrap();
    let prepared = plans
        .prepare(AutomationOperation::PrepareAsset(request))
        .unwrap();
    let plan = plans.claim(&prepared.token).unwrap();
    let jobs = JobStore::new(directory.path().join("jobs")).unwrap();
    let queued = stage_plan_job(&jobs, &plan).unwrap();
    let completed = run_operation(&jobs, &queued.job_id, &plan.operation).unwrap();
    assert_eq!(completed.lifecycle_state, JobLifecycleState::Succeeded);
    let pack = completed
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .unwrap();
    forge_pack::validate_pack_layout(&pack.path).unwrap();
    let quality: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.path.join("quality-report.json")).unwrap()).unwrap();
    assert_eq!(quality["verdict"], "game_ready");
    assert!(!quality["recommendations"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("trim_loop_range")));
    assert_eq!(quality["pixelDiagnostics"]["profile"], "character");
    let timing: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.path.join("previews/preview.timing.json")).unwrap())
            .unwrap();
    assert_eq!(timing["nominalFrameDurationMs"], 125.0);
    assert_eq!(timing["encodedFrameDurationMs"], 130);
    assert_eq!(timing["encodedMinusNativeDurationMs"], 10.0);
    let decoder = gif::DecodeOptions::new()
        .read_info(fs::File::open(pack.path.join("previews/preview.gif")).unwrap())
        .unwrap();
    let mut decoder = decoder;
    let mut delays = Vec::new();
    while let Some(frame) = decoder.read_next_frame().unwrap() {
        delays.push(frame.delay);
    }
    assert_eq!(delays, vec![13, 13]);
}

#[test]
fn gif_sidecar_retains_native_nonuniform_timing() {
    let directory = tempdir().unwrap();
    let paths = save_frames(
        directory.path(),
        &[frame(8, 8, 10, 10), frame(10, 8, 10, 10)],
    );
    let output = forge_core::export::build_preview_gif_with_timing(
        &paths,
        &directory.path().join("preview.gif"),
        forge_core::export::PreviewGifParameters {
            fps: 8.0,
            loop_animation: false,
            background: forge_core::export::GifBackground::Transparent,
        },
        Some(&[80, 240]),
    )
    .unwrap();
    let timing: serde_json::Value =
        serde_json::from_slice(&fs::read(output.timing_report_path).unwrap()).unwrap();
    assert_eq!(
        timing["nativeFrameDurationsMs"],
        serde_json::json!([80, 240])
    );
    assert_eq!(timing["nativeTotalDurationMs"], 320.0);
    assert_eq!(timing["encodedMinusNativeDurationMs"], -60.0);
}

#[test]
fn normalization_cannot_turn_visible_source_into_an_approved_empty_tail() {
    let directory = tempdir().unwrap();
    let source = save_frames(directory.path(), &[frame(8, 8, 8, 8), frame(10, 10, 8, 8)]);
    let normalized_dir = directory.path().join("normalized");
    fs::create_dir(&normalized_dir).unwrap();
    let normalized_images = [frame(8, 8, 8, 8), RgbaImage::new(64, 64)];
    let normalized = save_frames(&normalized_dir, &normalized_images);
    let report = compute_quality_report_with_pixels(
        &normalized_images
            .iter()
            .map(|image| bbox_from_image(image, 0))
            .collect::<Vec<_>>(),
        &[FrameSize::new(64, 64); 2],
        &source,
        &normalized,
        false,
        QualityProfile::Effect,
        true,
    )
    .unwrap();
    assert_eq!(report.verdict, QualityVerdict::Blocked);
    assert_eq!(
        report.pixel_diagnostics.unwrap().transparent_tail_start,
        None
    );
}

#[test]
fn multi_action_effect_pipeline_retains_tail_evidence_and_each_preview_timing() {
    let directory = tempdir().unwrap();
    let source = save_frames(
        directory.path(),
        &[
            frame(8, 8, 8, 8),
            frame(40, 40, 16, 16),
            RgbaImage::new(64, 64),
        ],
    );
    let request: forge_core::automation::PrepareCharacterPackRequest = serde_json::from_value(serde_json::json!({
        "schemaVersion":"2", "metadata":{"name":"Effect Fixture", "defaultAnimation":"burst"},
        "animations":[
            {"name":"burst", "input":{"kind":"png_sequence","paths":source}, "fps":8.0,"loop":false,"frameDurationsMs":[80,120,200]},
            {"name":"dissolve", "input":{"kind":"png_sequence","paths":source}, "fps":8.0,"loop":false}
        ],
        "normalize":{"mode":"preserve_source","margin":0,"marginBottom":0,"alphaThreshold":0},
        "quality":{"profile":"effect","allowTransparentTail":true,"requireGameReady":true}
    })).unwrap();
    let plans = PlanStore::new(directory.path().join("plans")).unwrap();
    let prepared = plans
        .prepare(AutomationOperation::PrepareCharacterPack(request))
        .unwrap();
    let plan = plans.claim(&prepared.token).unwrap();
    let jobs = JobStore::new(directory.path().join("jobs")).unwrap();
    let queued = stage_plan_job(&jobs, &plan).unwrap();
    let completed = run_operation(&jobs, &queued.job_id, &plan.operation).unwrap();
    assert_eq!(completed.lifecycle_state, JobLifecycleState::Succeeded);
    let pack = completed
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .unwrap();
    forge_pack::validate_pack_layout(&pack.path).unwrap();
    let quality: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.path.join("quality/animations.json")).unwrap())
            .unwrap();
    for animation in quality["animations"].as_array().unwrap() {
        assert_eq!(animation["report"]["pixelDiagnostics"]["profile"], "effect");
        assert_eq!(
            animation["report"]["pixelDiagnostics"]["transparentTailStart"],
            2
        );
        assert_eq!(
            animation["report"]["pixelDiagnostics"]["visualApproval"],
            "not_assessed"
        );
    }
    let default_timing: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.path.join("previews/preview.timing.json")).unwrap())
            .unwrap();
    assert_eq!(
        default_timing["nativeFrameDurationsMs"],
        serde_json::json!([80, 120, 200])
    );
    assert_eq!(default_timing["encodedMinusNativeDurationMs"], -10.0);
    assert!(pack.path.join("previews/burst.timing.json").is_file());
    assert!(pack.path.join("previews/dissolve.timing.json").is_file());
}

#[test]
fn invalid_transparent_tail_policy_is_rejected_before_a_plan_is_written() {
    let directory = tempdir().unwrap();
    let source = save_frames(directory.path(), &[frame(8, 8, 8, 8), frame(8, 8, 8, 8)]);
    let plans_root = directory.path().join("plans");
    let plans = PlanStore::new(plans_root.clone()).unwrap();
    for (profile, looping) in [("character", false), ("effect", true)] {
        let request: PrepareAssetRequest = serde_json::from_value(serde_json::json!({
            "input":{"kind":"png_sequence","paths":source},"metadata":{"name":"bad policy","loop":looping},
            "quality":{"profile":profile,"allowTransparentTail":true}
        })).unwrap();
        let error = plans
            .prepare(AutomationOperation::PrepareAsset(request))
            .unwrap_err();
        assert!(error.to_string().contains("allowTransparentTail"));
        assert_eq!(fs::read_dir(&plans_root).unwrap().count(), 0);
    }
    let request: forge_core::automation::PrepareCharacterPackRequest =
        serde_json::from_value(serde_json::json!({
            "schemaVersion":"2","metadata":{"name":"Mixed loops","defaultAnimation":"burst"},
            "animations":[
                {"name":"burst","input":{"kind":"png_sequence","paths":source},"loop":false},
                {"name":"smoke","input":{"kind":"png_sequence","paths":source},"loop":true}
            ],"quality":{"profile":"effect","allowTransparentTail":true}
        }))
        .unwrap();
    assert!(plans
        .prepare(AutomationOperation::PrepareCharacterPack(request))
        .unwrap_err()
        .to_string()
        .contains("allowTransparentTail"));
    assert_eq!(fs::read_dir(&plans_root).unwrap().count(), 0);
}
