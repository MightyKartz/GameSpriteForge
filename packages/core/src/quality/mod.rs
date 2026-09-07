pub mod gait_cycle;
pub mod identity;
pub mod loop_selection;
pub mod looping;
pub mod metrics;
pub mod source_cycle_sampling;

pub use gait_cycle::{
    select_gait_cycle_frames, GaitCycleError, GaitCyclePolicy, GaitCycleReport, GaitCycleResult,
    GaitCycleVerdict, GaitPhaseV1, GAIT_CYCLE_PROFILE,
};
pub use identity::{
    evaluate_calibration_manifest, evaluate_identity_metric, IdentityEvaluationReportV1,
    IdentityMetricError, IdentityMetricReportV1, IdentityMetricWeightsV1, IdentitySignalScoresV1,
    IDENTITY_METRIC_PROFILE,
};
pub use loop_selection::{
    assess_loop_window, select_loop_frames, select_loop_frames_with_anchor,
    AnimationCadenceProfile, AnimationTimingV1, LoopAnchorPolicyV1, LoopSelectionError,
    LoopSelectionPolicy, LoopSelectionReport, LoopSelectionResult, LoopSelectionVerdict,
    ANCHORED_LOOP_SELECTION_PROFILE, ANIMATION_TIMING_PROFILE, LOOP_SELECTION_PROFILE,
};
pub use looping::loop_match_score;
pub use metrics::{
    compute_quality_metrics, compute_quality_metrics_with_loop_range, compute_quality_report,
    compute_quality_report_for_animation, compute_quality_report_with_loop_range,
    quality_recommendations, QualityMetrics, QualityRecommendationId, QualityReport,
    QualityVerdict,
};
pub use source_cycle_sampling::{
    sample_source_cycle_frames, validate_source_cycle_sampling_report, SourceCycleSamplingError,
    SourceCycleSamplingPolicyV1, SourceCycleSamplingReportV1, SourceCycleSamplingResult,
    SOURCE_CYCLE_SAMPLING_PROFILE, SOURCE_CYCLE_SAMPLING_PROFILE_24,
};
