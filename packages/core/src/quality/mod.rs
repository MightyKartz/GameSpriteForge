pub mod loop_selection;
pub mod looping;
pub mod metrics;

pub use loop_selection::{
    select_loop_frames, LoopSelectionError, LoopSelectionPolicy, LoopSelectionReport,
    LoopSelectionResult, LoopSelectionVerdict, LOOP_SELECTION_PROFILE,
};
pub use looping::loop_match_score;
pub use metrics::{
    compute_quality_metrics, compute_quality_metrics_with_loop_range, compute_quality_report,
    compute_quality_report_for_animation, compute_quality_report_with_loop_range,
    quality_recommendations, QualityMetrics, QualityRecommendationId, QualityReport,
    QualityVerdict,
};
