use forge_core::frames::{FrameBbox, FrameSize};
use forge_core::quality::{
    compute_quality_report, compute_quality_report_with_loop_range, loop_match_score,
    QualityVerdict,
};

fn bbox(left: f32, top: f32, right: f32, bottom: f32) -> FrameBbox {
    FrameBbox::from_bounds(left, top, right, bottom, 0.25)
}

fn sizes(count: usize) -> Vec<FrameSize> {
    vec![FrameSize::new(64, 64); count]
}

#[test]
fn empty_alpha_frame_is_blocked() {
    let report = compute_quality_report(
        &[bbox(10.0, 10.0, 30.0, 40.0), FrameBbox::empty()],
        &sizes(2),
    );

    assert_eq!(report.verdict, QualityVerdict::Blocked);
    assert!(report
        .notes
        .iter()
        .any(|note| note == "frame_without_foreground"));
}

#[test]
fn verdict_thresholds_match_spec() {
    let game_ready = compute_quality_report(
        &[bbox(20.0, 10.0, 40.0, 50.0), bbox(20.0, 10.0, 40.0, 50.0)],
        &sizes(2),
    );
    assert_eq!(game_ready.verdict, QualityVerdict::GameReady);

    let needs_cleanup = compute_quality_report(
        &[bbox(10.0, 10.0, 30.0, 40.0), bbox(30.0, 18.0, 50.0, 48.0)],
        &sizes(2),
    );
    assert_eq!(needs_cleanup.verdict, QualityVerdict::NeedsCleanup);

    let prototype_usable = compute_quality_report(
        &[bbox(2.0, 10.0, 22.0, 30.0), bbox(42.0, 24.0, 62.0, 48.0)],
        &sizes(2),
    );
    assert_eq!(prototype_usable.verdict, QualityVerdict::PrototypeUsable);

    let blocked = compute_quality_report(&[bbox(10.0, 10.0, 30.0, 40.0)], &sizes(1));
    assert_eq!(blocked.verdict, QualityVerdict::Blocked);
}

#[test]
fn frame_size_mismatch_is_blocked() {
    let report = compute_quality_report(
        &[bbox(20.0, 10.0, 40.0, 50.0), bbox(20.0, 10.0, 40.0, 50.0)],
        &[FrameSize::new(64, 64), FrameSize::new(32, 64)],
    );

    assert_eq!(report.verdict, QualityVerdict::Blocked);
    assert!(!report.metrics.frame_size_consistent);
}

#[test]
fn loop_score_is_high_for_matching_frames_and_low_for_distant_frames() {
    let first = bbox(20.0, 10.0, 40.0, 50.0);
    let close = bbox(21.0, 10.0, 41.0, 50.0);
    let far = bbox(2.0, 2.0, 12.0, 14.0);

    assert!(loop_match_score(&first, &close) > 0.75);
    assert!(loop_match_score(&first, &far) < 0.75);
}

#[test]
fn loop_range_uses_selected_start_and_end_without_changing_frame_count() {
    let far_start = bbox(2.0, 2.0, 12.0, 14.0);
    let loop_start = bbox(20.0, 10.0, 40.0, 50.0);
    let loop_end = bbox(21.0, 10.0, 41.0, 50.0);
    let far_end = bbox(45.0, 24.0, 62.0, 48.0);

    let report = compute_quality_report_with_loop_range(
        &[far_start, loop_start, loop_end, far_end],
        &sizes(4),
        Some((1, 2)),
    );

    assert_eq!(report.metrics.frame_count, 4);
    assert!(report.metrics.loop_match_score > 0.75);
    assert!(!report
        .recommendations
        .iter()
        .any(|recommendation| format!("{recommendation:?}") == "TrimLoopRange"));
}
