use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use forge_core::character_animation_review::{
    review_is_approved, validate_animation_review_closure, AnimationHumanReviewV1,
};
use forge_core::export::{
    export_pack, CharacterMirrorPolicyV1, ExportPackParams, GifBackground,
    GodotRenderingContractV1, GodotTextureFilterV1, PackMetadataParams, PreviewGifParameters,
    SpriteSheetParameters,
};
use forge_core::frames::{bbox_from_image, manual_anchor, FrameSize};
use forge_core::quality::compute_quality_report_for_animation;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplaySummary {
    candidate_video_sha256: String,
    action: String,
    frame_durations_ms: Vec<u64>,
    frames: Vec<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let replay_summary_path = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("usage: package_reviewed_animation <replay-summary> <review> <output-directory>")?;
    let review_path = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("usage: package_reviewed_animation <replay-summary> <review> <output-directory>")?;
    let output_directory = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("usage: package_reviewed_animation <replay-summary> <review> <output-directory>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: package_reviewed_animation <replay-summary> <review> <output-directory>".into(),
        );
    }
    if output_directory.exists() && output_directory.read_dir()?.next().is_some() {
        return Err(format!(
            "refusing to overwrite non-empty {}",
            output_directory.display()
        )
        .into());
    }
    let replay: ReplaySummary = serde_json::from_slice(&fs::read(&replay_summary_path)?)?;
    let review: AnimationHumanReviewV1 = serde_json::from_slice(&fs::read(&review_path)?)?;
    validate_animation_review_closure(
        &review,
        &replay_summary_path,
        &replay.frames,
        &replay.frame_durations_ms,
    )?;
    let production_eligible = review_is_approved(&review);
    if replay.frames.len() != 24 {
        return Err("reviewed 24-frame delivery requires exactly 24 frames".into());
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
    quality.notes.push(if production_eligible {
        "animation_human_review_approved".into()
    } else {
        "animation_human_review_pending".into()
    });
    quality.notes.push(if production_eligible {
        "production_eligible_true".into()
    } else {
        "production_eligible_false".into()
    });
    let export_id = if production_eligible {
        "v18-walk-right-approved"
    } else {
        "v17-walk-right-review"
    };
    let output = export_pack(ExportPackParams {
        exports_dir: output_directory,
        export_id: export_id.into(),
        frame_paths: replay.frames.clone(),
        sheet: SpriteSheetParameters {
            columns: 2,
            padding_px: 0,
            margin_px: 0,
            max_texture_size: 4096,
            allow_multi_sheet: true,
        },
        gif: PreviewGifParameters {
            fps: 12.0,
            loop_animation: true,
            background: GifBackground::Transparent,
            scale: 1,
        },
        metadata: PackMetadataParams {
            id: export_id.into(),
            name: if production_eligible {
                "V18 Walk Right Approved".into()
            } else {
                "V17 Walk Right Review".into()
            },
            version: if production_eligible {
                "1.0.0".into()
            } else {
                "0.1.0-review".into()
            },
            creator_name: "Game Sprite Forge".into(),
            license_type: "private".into(),
            source_kind: "deterministic_compiler".into(),
            source_name: Some("Vidu Q2 2026-08-19 12:23".into()),
            source_metadata: Some(serde_json::json!({
                "candidateVideoSha256": replay.candidate_video_sha256,
                "animationHumanReviewStatus": review.status,
                "productionEligible": production_eligible,
                "nativePlacementPassthrough": true,
                "frameCount": replay.frames.len(),
            })),
            animation_name: replay.action,
            animation_frames: None,
            fps: 12.0,
            frame_durations_ms: replay.frame_durations_ms,
            loop_animation: true,
            anchor: manual_anchor(720.0, 1316.0),
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
        &review_path,
        output.pack_dir.join("quality/animation-human-review.json"),
    )?;
    let forgepack_path = output.pack_dir.join("forgepack.json");
    let mut forgepack: serde_json::Value = serde_json::from_slice(&fs::read(&forgepack_path)?)?;
    forgepack["assets"]["animationHumanReview"] =
        serde_json::Value::String("quality/animation-human-review.json".into());
    fs::write(&forgepack_path, serde_json::to_vec_pretty(&forgepack)?)?;
    forge_pack::validate_pack_layout(&output.pack_dir)?;
    let summary = forge_pack::inspect_pack(&output.pack_dir)?;
    let result = serde_json::json!({
        "profile": if production_eligible { "reviewed-animation-production-pack@1.0.0" } else { "reviewed-animation-candidate-pack@1.0.0" },
        "packDir": output.pack_dir,
        "frameCount": summary.frame_count,
        "animations": summary.animations,
        "reviewStatus": review.status,
        "productionEligible": production_eligible,
        "productionPackWritten": production_eligible,
        "candidatePackWritten": !production_eligible,
    });
    fs::write(
        output.export_dir.join(if production_eligible {
            "production-pack-decision.json"
        } else {
            "candidate-pack-decision.json"
        }),
        serde_json::to_vec_pretty(&result)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
