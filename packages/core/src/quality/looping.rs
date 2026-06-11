use crate::frames::FrameBbox;

pub fn loop_match_score(first: &FrameBbox, last: &FrameBbox) -> f32 {
    if !first.has_foreground() || !last.has_foreground() {
        return 0.0;
    }

    let bbox_distance = (first.left - last.left).abs()
        + (first.top - last.top).abs()
        + (first.right - last.right).abs()
        + (first.bottom - last.bottom).abs();
    let center_distance = ((first.center_x - last.center_x).powi(2)
        + (first.center_y - last.center_y).powi(2))
    .sqrt();
    let coverage_distance = (first.alpha_coverage - last.alpha_coverage).abs() * 100.0;
    let scale = first
        .width
        .max(first.height)
        .max(last.width)
        .max(last.height)
        .max(1.0);

    let normalized_distance =
        (bbox_distance / (scale * 4.0)) + (center_distance / scale) + coverage_distance;
    (1.0 - normalized_distance / 3.0).clamp(0.0, 1.0)
}
