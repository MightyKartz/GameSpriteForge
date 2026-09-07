use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

use forge_core::asset_project::assess_character_silhouette_temporal_source;
use forge_core::frames::{bbox_from_image, FrameSize};
use forge_core::quality::compute_quality_report_for_animation;

fn main() {
    let directory = PathBuf::from(
        env::args()
            .nth(1)
            .expect("usage: inspect_loop_ranges <frame-directory>"),
    );
    let mut paths = fs::read_dir(&directory)
        .expect("read frame directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("png"))
        .collect::<Vec<_>>();
    paths.sort();
    let frames = paths
        .iter()
        .map(|path| image::open(path).expect("read frame").to_rgba8())
        .collect::<Vec<_>>();
    let mut candidates = Vec::new();
    for start in 0..frames.len() {
        for end in start..frames.len() {
            if end + 1 - start < 4 {
                continue;
            }
            let selected = frames[start..=end].to_vec();
            let bboxes = selected
                .iter()
                .map(|frame| bbox_from_image(frame, 0))
                .collect::<Vec<_>>();
            let sizes = selected
                .iter()
                .map(|frame| FrameSize::new(frame.width(), frame.height()))
                .collect::<Vec<_>>();
            let quality = compute_quality_report_for_animation(&bboxes, &sizes, true);
            let temporal = assess_character_silhouette_temporal_source(
                &[('a'.to_string(), selected)]
                    .into_iter()
                    .collect::<BTreeMap<_, _>>(),
            );
            let temporal_animation = temporal.animations.first().expect("one animation");
            candidates.push(serde_json::json!({
                "startFrame": start,
                "endFrame": end,
                "frameCount": end + 1 - start,
                "loopMatchScore": quality.metrics.loop_match_score,
                "qualityVerdict": quality.verdict,
                "temporalVerdict": temporal_animation.verdict,
                "temporalReasons": temporal_animation.reasons,
            }));
        }
    }
    candidates.sort_by(|left, right| {
        right["loopMatchScore"]
            .as_f64()
            .partial_cmp(&left["loopMatchScore"].as_f64())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&candidates).expect("serialize candidates")
    );
}
