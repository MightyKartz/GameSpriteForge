use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use forge_core::character_animation_review::{review_is_approved, AnimationHumanReviewV1};
use forge_core::export::{
    export_pack, CharacterMirrorPolicyV1, ExportPackParams, GifBackground,
    GodotRenderingContractV1, GodotTextureFilterV1, PackMetadataParams, PreviewGifParameters,
    SpriteSheetParameters,
};
use forge_core::frames::{bbox_from_image, manual_anchor, FrameSize};
use forge_core::quality::compute_quality_report_for_animation;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplaySummary {
    candidate_video_sha256: String,
    action: String,
    direction: String,
    frame_durations_ms: Vec<u64>,
    frames: Vec<PathBuf>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplayFrameLock {
    native_frame_lock_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Point {
    x: f64,
    y: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Collision {
    width: f64,
    height: f64,
    offset_x: f64,
    offset_y: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecoveryPackConfig {
    export_id: String,
    name: String,
    direction: String,
    source_name: String,
    original_approved_review_sha256: String,
    pivot: Point,
    collision: Collision,
    visual_scale: f64,
    movement_speed_pixels_per_second: f64,
    target_cycle_duration_ms: u64,
    playback_speed_scale: f64,
}

fn sha256(path: &Path) -> Result<String, Box<dyn Error>> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if arguments.len() != 4 {
        return Err("usage: package_recovered_approved_animation <replay-summary> <approved-review> <recovery-pack-config> <output-directory>".into());
    }
    let replay_path = &arguments[0];
    let review_path = &arguments[1];
    let config_path = &arguments[2];
    let output_directory = &arguments[3];
    if output_directory.exists() && output_directory.read_dir()?.next().is_some() {
        return Err(format!(
            "refusing to overwrite non-empty {}",
            output_directory.display()
        )
        .into());
    }

    let replay: ReplaySummary = serde_json::from_slice(&fs::read(replay_path)?)?;
    let review: AnimationHumanReviewV1 = serde_json::from_slice(&fs::read(review_path)?)?;
    let config: RecoveryPackConfig = serde_json::from_slice(&fs::read(config_path)?)?;
    let replay_frame_lock_path = replay_path
        .parent()
        .ok_or("replay summary must have a parent directory")?
        .join("replay-frame-lock.json");
    let replay_frame_lock: ReplayFrameLock =
        serde_json::from_slice(&fs::read(&replay_frame_lock_path)?)?;
    if !valid_sha256(&replay_frame_lock.native_frame_lock_sha256) {
        return Err("replay frame lock lacks a valid native frame lock SHA-256".into());
    }
    let replay_frame_lock_sha256 = sha256(&replay_frame_lock_path)?;
    let recipe_sha256 = sha256(config_path)?;
    let lineage = json!({
        "schemaVersion": "1",
        "profile": "approved-animation-recovery-lineage@1.0.0",
        "direction": config.direction,
        "animation": review.animation,
        "recoveryPolicy": "exact-approved-frame-bytes",
        "approvedReviewSha256": config.original_approved_review_sha256,
        "nativeFrameLockSha256": replay_frame_lock.native_frame_lock_sha256,
        "replayFrameLockSha256": replay_frame_lock_sha256,
        "recipeSha256": recipe_sha256,
        "providerRequestCountThisOperation": 0,
        "nativePlacementPassthrough": true
    });
    let mut lineage_bytes = serde_json::to_vec_pretty(&lineage)?;
    lineage_bytes.push(b'\n');
    let animation_review_lineage_sha256 = sha256_bytes(&lineage_bytes);
    if !review_is_approved(&review) {
        return Err("review is not an approved six-check review".into());
    }
    if replay.action != review.animation
        || replay.direction != config.direction
        || replay.action != format!("walk_{}", config.direction)
        || replay.frames.len() != review.frame_sha256.len()
        || replay.frame_durations_ms != review.frame_durations_ms
    {
        return Err("replay, direction, durations, and approval shape do not match".into());
    }
    if config.original_approved_review_sha256 != sha256(review_path)? {
        return Err("approved review bytes do not match the locked original hash".into());
    }
    for (index, (path, expected)) in replay.frames.iter().zip(&review.frame_sha256).enumerate() {
        let actual = sha256(path)?;
        if &actual != expected {
            return Err(format!(
                "approved frame {index} hash mismatch: expected {expected}, got {actual}"
            )
            .into());
        }
    }
    for number in [
        config.pivot.x,
        config.pivot.y,
        config.collision.width,
        config.collision.height,
        config.collision.offset_x,
        config.collision.offset_y,
        config.visual_scale,
        config.movement_speed_pixels_per_second,
        config.playback_speed_scale,
    ] {
        if !number.is_finite() {
            return Err("runtime geometry contains a non-finite number".into());
        }
    }
    if config.collision.width <= 0.0
        || config.collision.height <= 0.0
        || config.visual_scale <= 0.0
        || config.movement_speed_pixels_per_second <= 0.0
        || config.target_cycle_duration_ms == 0
        || config.playback_speed_scale <= 0.0
    {
        return Err("runtime geometry and timing must be positive".into());
    }

    let images = replay
        .frames
        .iter()
        .map(|path| Ok(image::open(path)?.to_rgba8()))
        .collect::<Result<Vec<_>, image::ImageError>>()?;
    let bboxes = images
        .iter()
        .map(|image| bbox_from_image(image, 48))
        .collect::<Vec<_>>();
    let sizes = images
        .iter()
        .map(|image| FrameSize::new(image.width(), image.height()))
        .collect::<Vec<_>>();
    let mut quality = compute_quality_report_for_animation(&bboxes, &sizes, true);
    quality.notes.push("animation_human_review_approved".into());
    quality.notes.push("production_eligible_true".into());
    quality
        .notes
        .push("recovered_frames_match_original_approval".into());
    quality.notes.push("no_frame_transform_applied".into());

    let source_cycle_duration_ms = replay.frame_durations_ms.iter().sum::<u64>();
    let fps = replay.frames.len() as f32 * 1000.0 / source_cycle_duration_ms as f32;
    let output = export_pack(ExportPackParams {
        exports_dir: output_directory.to_path_buf(),
        export_id: config.export_id.clone(),
        frame_paths: replay.frames.clone(),
        sheet: SpriteSheetParameters {
            columns: 2,
            padding_px: 0,
            margin_px: 0,
            max_texture_size: 4096,
            allow_multi_sheet: true,
        },
        gif: PreviewGifParameters {
            fps,
            loop_animation: true,
            background: GifBackground::Transparent,
            scale: 1,
        },
        metadata: PackMetadataParams {
            id: config.export_id.clone(),
            name: config.name.clone(),
            version: "1.0.0-recovery.1".into(),
            creator_name: "Forge Character Animation".into(),
            license_type: "User-owned provider-generated asset; human-approved recovery".into(),
            source_kind: "deterministic_compiler".into(),
            source_name: Some(config.source_name.clone()),
            source_metadata: Some(json!({
                "animationHumanReviewSha256": config.original_approved_review_sha256,
                "animationHumanReviewStatus": "approved",
                "animationReviewLineageSha256": animation_review_lineage_sha256,
                "candidateVideoSha256": review.source_video_sha256,
                "promptSha256": review.prompt_sha256,
                "replaySummarySha256": review.replay_summary_sha256,
                "approvedOriginalSourceVideoSha256": review.source_video_sha256,
                "approvedOriginalReplaySummarySha256": review.replay_summary_sha256,
                "recoveredReplaySummarySha256": sha256(replay_path)?,
                "recoveredCandidateVideoSha256": replay.candidate_video_sha256,
                "approvedFrameSha256": review.frame_sha256,
                "frameCount": replay.frames.len(),
                "direction": config.direction,
                "productionEligible": true,
                "providerRequestCount": 0,
                "providerRequestCountThisOperation": 0,
                "inheritedProviderRequestCount": 1,
                "totalProviderRequestCount": 1,
                "packInventoryExternal": true,
                "recoveryPolicy": "exact-approved-frame-bytes",
                "frameTransformsApplied": false,
                "nativePlacementPassthrough": true,
                "nativeFrameLockSha256": replay_frame_lock.native_frame_lock_sha256,
                "replayFrameLockSha256": replay_frame_lock_sha256,
                "recipeSha256": recipe_sha256,
                "runtimeContract": {
                    "pivot": {"x": config.pivot.x, "y": config.pivot.y},
                    "collision": {
                        "width": config.collision.width,
                        "height": config.collision.height,
                        "offsetX": config.collision.offset_x,
                        "offsetY": config.collision.offset_y
                    },
                    "visualScale": config.visual_scale,
                    "movementSpeedPixelsPerSecond": config.movement_speed_pixels_per_second,
                    "sourceCycleDurationMs": source_cycle_duration_ms,
                    "targetCycleDurationMs": config.target_cycle_duration_ms,
                    "playbackSpeedScale": config.playback_speed_scale
                }
            })),
            animation_name: replay.action.clone(),
            animation_frames: None,
            fps,
            frame_durations_ms: replay.frame_durations_ms.clone(),
            loop_animation: true,
            anchor: manual_anchor(config.pivot.x as f32, config.pivot.y as f32),
            rendering: GodotRenderingContractV1 {
                texture_filter: GodotTextureFilterV1::Linear,
                pixel_snap: false,
                mirror_policy: CharacterMirrorPolicyV1::Auto,
                ..Default::default()
            },
            quality_report: quality,
        },
    })?;

    fs::create_dir_all(output.pack_dir.join("quality"))?;
    fs::copy(
        review_path,
        output.pack_dir.join("quality/animation-human-review.json"),
    )?;
    fs::write(
        output
            .pack_dir
            .join("quality/animation-review-lineage.json"),
        &lineage_bytes,
    )?;
    let certification = json!({
        "schemaVersion": "1",
        "profile": "approved-animation-recovery-certification@1.0.0",
        "animation": replay.action,
        "direction": config.direction,
        "approvedReviewPath": review_path,
        "approvedReviewSha256": config.original_approved_review_sha256,
        "animationReviewLineageSha256": animation_review_lineage_sha256,
        "nativeFrameLockSha256": replay_frame_lock.native_frame_lock_sha256,
        "replayFrameLockSha256": replay_frame_lock_sha256,
        "recipeSha256": recipe_sha256,
        "recoveredReplaySummaryPath": replay_path,
        "recoveredReplaySummarySha256": sha256(replay_path)?,
        "frameSha256": review.frame_sha256,
        "frameDurationsMs": replay.frame_durations_ms,
        "frameBytesMatchOriginalApproval": true,
        "frameTransformsApplied": false,
        "providerRequestCountThisOperation": 0,
        "runtimeContract": {
            "pivot": {"x": config.pivot.x, "y": config.pivot.y},
            "collision": {
                "width": config.collision.width,
                "height": config.collision.height,
                "offsetX": config.collision.offset_x,
                "offsetY": config.collision.offset_y
            },
            "visualScale": config.visual_scale,
            "movementSpeedPixelsPerSecond": config.movement_speed_pixels_per_second,
            "sourceCycleDurationMs": source_cycle_duration_ms,
            "targetCycleDurationMs": config.target_cycle_duration_ms,
            "playbackSpeedScale": config.playback_speed_scale
        }
    });
    fs::write(
        output.pack_dir.join("quality/recovery-certification.json"),
        serde_json::to_vec_pretty(&certification)?,
    )?;
    let forgepack_path = output.pack_dir.join("forgepack.json");
    let mut forgepack: serde_json::Value = serde_json::from_slice(&fs::read(&forgepack_path)?)?;
    forgepack["assets"]["animationHumanReview"] =
        serde_json::Value::String("quality/animation-human-review.json".into());
    fs::write(&forgepack_path, serde_json::to_vec_pretty(&forgepack)?)?;
    forge_pack::validate_pack_layout(&output.pack_dir)?;
    let summary = forge_pack::inspect_pack(&output.pack_dir)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "profile": "recovered-approved-animation-pack@1.0.0",
            "packDir": output.pack_dir,
            "frameCount": summary.frame_count,
            "animations": summary.animations,
            "reviewStatus": "approved",
            "frameBytesMatchOriginalApproval": true,
            "productionEligible": true,
            "providerRequestCountThisOperation": 0
        }))?
    );
    Ok(())
}
