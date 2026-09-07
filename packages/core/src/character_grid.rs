use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

use crate::asset_project::{character_body_bbox, CharacterEquipmentKindV1, ConsistencyVerdict};
use crate::character_camera::CharacterCameraProfileV1;
use crate::character_direction::DirectionViewV1;
use crate::equipment_contact::{
    assess_hand_equipment_contact_with_kind, HandEquipmentContactReportV1,
};
use crate::quality::{evaluate_identity_metric, IdentitySignalScoresV1};

pub const DIRECTION_GRID_LOCK_PROFILE: &str = "direction-grid-lock@1.1.0";
pub const DIRECTION_GRID_IMPORT_LOCK_PROFILE: &str = "direction-grid-lock@1.2.0";
pub const DIRECTION_GRID_APPEARANCE_PROFILE: &str = "direction-grid-appearance@1.0.0";
pub const DIRECTION_GRID_APPEARANCE_FILE: &str = "direction-grid-appearance-report.json";
pub const DIRECTION_GRID_IMPORT_CONSISTENCY_PROFILE: &str =
    "direction-grid-import-consistency@1.0.0";
pub const DIRECTION_GRID_IMPORT_CONSISTENCY_FILE: &str =
    "direction-grid-import-consistency-report.json";
pub const CAPE_HEM_CONSISTENCY_PROFILE: &str = "cape-hem-consistency@1.0.0";
pub const CAPE_HEM_CONSISTENCY_FILE: &str = "cape-hem-consistency-report.json";
pub const GRID_APPROVAL_PROFILE: &str = "direction-grid-approval@1.0.0";
pub const GRID_APPROVAL_FILE: &str = "direction-grid-approval.json";
pub const GRID_ACTION_REPORT_PROFILE: &str = "grid-action-report@1.1.0";
pub const GRID_ACTION_PHASE_PROFILE: &str = "grid-action-phases@1.0.0";
pub const GRID_ACTION_PHASE_EVIDENCE_PROFILE: &str = "grid-action-phase-evidence@1.0.0";
pub const GRID_WORKFLOW: &str = "topdown-grid@9.0.0";
pub const GRID_KEYFRAME_WORKFLOW: &str = "topdown-grid@9.1.0";
pub const GRID_STRUCTURED_GAIT_WORKFLOW: &str = "topdown-grid@9.2.0";
pub const GRID_ASYMMETRIC_GAIT_WORKFLOW: &str = "topdown-grid@9.3.0";
pub const GRID_PLATFORM_SAFE_GAIT_WORKFLOW: &str = "topdown-grid@9.4.0";
pub const GRID_FOOTWEAR_CLEANUP_WORKFLOW: &str = "topdown-grid@9.5.0";
pub const GRID_KEYFRAME_ACTION_REPORT_PROFILE: &str = "grid-keyframe-action-report@1.0.0";
pub const GRID_STRUCTURED_GAIT_ACTION_REPORT_PROFILE_LEGACY: &str =
    "grid-keyframe-action-report@1.1.0";
pub const GRID_STRUCTURED_GAIT_ACTION_REPORT_PROFILE: &str = "grid-keyframe-action-report@1.2.0";
pub const GRID_ASYMMETRIC_GAIT_ACTION_REPORT_PROFILE: &str = "grid-keyframe-action-report@1.3.0";
pub const GRID_PLATFORM_SAFE_GAIT_ACTION_REPORT_PROFILE: &str = "grid-keyframe-action-report@1.4.0";
pub const GRID_FOOTWEAR_CLEANUP_ACTION_REPORT_PROFILE: &str = "grid-keyframe-action-report@1.5.0";

pub fn is_direction_grid_lock_profile(profile: &str) -> bool {
    matches!(
        profile,
        "direction-grid-lock@1.0.0"
            | DIRECTION_GRID_LOCK_PROFILE
            | DIRECTION_GRID_IMPORT_LOCK_PROFILE
    )
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GridPoseGuidanceV1 {
    #[default]
    Enabled,
    Disabled,
    Grayscale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapeHemContractV1 {
    ContinuousGoldLowerHem,
    FrontAuthoritativeNoSkirtHem,
}

impl CapeHemContractV1 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ContinuousGoldLowerHem => "continuous_gold_lower_hem",
            Self::FrontAuthoritativeNoSkirtHem => "front_authoritative_no_skirt_hem",
        }
    }
}

impl GridPoseGuidanceV1 {
    pub const fn enabled(self) -> bool {
        matches!(self, Self::Enabled | Self::Grayscale)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectionGridNodeV1 {
    pub node_id: String,
    pub direction: DirectionViewV1,
    pub cell_index: u8,
    pub path: PathBuf,
    pub sha256: String,
    pub generation_master_path: PathBuf,
    pub generation_master_sha256: String,
    pub provider_request_occurred: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectionGridProducerKindV1 {
    ProviderGeneration,
    ExternalImport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectionGridProducerV1 {
    pub kind: DirectionGridProducerKindV1,
    pub generator: String,
    pub provider_request_occurred: bool,
    pub input_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matted_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectionGridDownstreamBindingV1 {
    pub provider_id: String,
    pub profile_id: String,
    pub image_model: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectionGridLockV1 {
    pub schema_version: String,
    pub profile: String,
    pub workflow: String,
    pub provider_id: String,
    pub profile_id: String,
    pub image_model: String,
    pub style_revision: String,
    #[serde(default)]
    pub subject_id: String,
    #[serde(default)]
    pub subject_revision: String,
    #[serde(default)]
    pub subject_canonical_sha256: String,
    #[serde(default)]
    pub equipment_kind: CharacterEquipmentKindV1,
    #[serde(default)]
    pub camera_profile: CharacterCameraProfileV1,
    pub sheet_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appearance_report_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appearance_report_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_consistency_report_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_consistency_report_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alpha_edge_report_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alpha_edge_report_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cape_hem_contract: Option<CapeHemContractV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cape_hem_report_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cape_hem_report_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub producer: Option<DirectionGridProducerV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downstream_binding: Option<DirectionGridDownstreamBindingV1>,
    pub nodes: Vec<DirectionGridNodeV1>,
}

impl DirectionGridLockV1 {
    pub fn node(&self, node_id: &str) -> Option<&DirectionGridNodeV1> {
        self.nodes.iter().find(|node| node.node_id == node_id)
    }

    pub fn node_for_animation(&self, animation: &str) -> Option<&DirectionGridNodeV1> {
        let node_id = match animation {
            "idle_down" | "walk_down" => "front_idle",
            "idle_up" | "walk_up" => "back_idle",
            "idle_right" | "walk_right" => "right_idle",
            "idle_left" | "walk_left" => "left_idle",
            _ => return None,
        };
        self.node(node_id)
    }

    pub fn node_hashes(&self) -> BTreeMap<String, String> {
        self.nodes
            .iter()
            .map(|node| (node.node_id.clone(), node.sha256.clone()))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectionGridHandStateBaselineV1 {
    pub available: bool,
    pub source: String,
    pub left_exposed_pixels: u32,
    pub right_exposed_pixels: u32,
    pub exterior_elongation_score: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectionGridHandStateNodeV1 {
    pub node_id: String,
    pub left_exposed_pixels: u32,
    pub right_exposed_pixels: u32,
    pub exposed_coverage_ratio: f32,
    pub exterior_elongation_score: f32,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectionGridAppearanceReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub equipment_kind: String,
    pub equipment: HandEquipmentContactReportV1,
    pub hand_state_baseline: DirectionGridHandStateBaselineV1,
    pub hand_state_nodes: Vec<DirectionGridHandStateNodeV1>,
    pub mirrored_side_coverage_ratio: f32,
    pub unexpected_elongated_node_count: u32,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectionGridImportConsistencyNodeV1 {
    pub node_id: String,
    pub source_path: PathBuf,
    pub source_sha256: String,
    pub candidate_path: PathBuf,
    pub candidate_sha256: String,
    pub identity_scores: IdentitySignalScoresV1,
    pub body_width_ratio: f32,
    pub body_height_ratio: f32,
    pub center_drift_px: f32,
    pub foot_baseline_drift_px: f32,
    pub added_foreground_ratio: f32,
    pub removed_foreground_ratio: f32,
    pub face_hair_change_ratio: f32,
    pub upper_covering_change_ratio: f32,
    pub torso_detail_change_ratio: f32,
    pub belt_pouch_change_ratio: f32,
    pub sleeve_hand_change_ratio: f32,
    pub cape_exterior_change_ratio: f32,
    pub source_exterior_elongation_score: f32,
    pub candidate_exterior_elongation_score: f32,
    pub exterior_elongation_delta: f32,
    pub new_torso_linear_component_count: u32,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectionGridImportConsistencyReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub absolute_appearance_profile: String,
    pub absolute_appearance_sha256: String,
    pub nodes: Vec<DirectionGridImportConsistencyNodeV1>,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapeHemConsistencyNodeV1 {
    pub node_id: String,
    pub path: PathBuf,
    pub sha256: String,
    pub body_width: f32,
    pub body_height: f32,
    pub lower_hem_start_ratio: f32,
    pub lower_hem_end_ratio: f32,
    pub gold_adjacent_to_cape_pixel_count: u32,
    pub gold_adjacent_to_cape_ratio: f32,
    pub minimum_gold_adjacent_to_cape_ratio: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum_gold_adjacent_to_cape_ratio: Option<f32>,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapeHemConsistencyReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub contract: CapeHemContractV1,
    pub nodes: Vec<CapeHemConsistencyNodeV1>,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
}

const CAPE_HEM_START_RATIO: f32 = 0.68;
const CAPE_HEM_END_RATIO: f32 = 0.90;
const MIN_GOLD_CAPE_HEM_RATIO: f32 = 0.004;
const MAX_FRONT_AUTHORITY_GOLD_CAPE_HEM_RATIO: f32 = 0.004;

/// Enforces an explicitly selected garment topology. This is opt-in because
/// gold trim is not a universal character property. The detector looks only
/// in the lower body band and counts gold pixels that are spatially adjacent
/// to cape-green pixels, avoiding scarf, belt and boot colors elsewhere.
pub fn assess_cape_hem_consistency(
    contract: CapeHemContractV1,
    images: &BTreeMap<String, RgbaImage>,
    bindings: &BTreeMap<String, (PathBuf, String)>,
) -> CapeHemConsistencyReportV1 {
    let mut nodes = Vec::with_capacity(4);
    let mut report_reasons = Vec::new();
    for node_id in ["front_idle", "back_idle", "right_idle", "left_idle"] {
        let Some(image) = images.get(node_id) else {
            report_reasons.push(format!("cape_hem_node_missing:{node_id}"));
            continue;
        };
        let Some((path, sha256)) = bindings.get(node_id) else {
            report_reasons.push(format!("cape_hem_binding_missing:{node_id}"));
            continue;
        };
        let Some(body) = character_body_bbox(image) else {
            report_reasons.push(format!("cape_hem_body_missing:{node_id}"));
            continue;
        };
        let start_y = (body.top + body.height * CAPE_HEM_START_RATIO)
            .floor()
            .max(0.0) as u32;
        let end_y = (body.top + body.height * CAPE_HEM_END_RATIO)
            .ceil()
            .min(image.height() as f32) as u32;
        let radius = ((body.width * 0.018).round() as i32).max(3);
        let mut adjacent_gold = 0u32;
        for y in start_y..end_y {
            for x in 0..image.width() {
                if !is_cape_gold(*image.get_pixel(x, y)) {
                    continue;
                }
                let mut near_cape = false;
                for offset_y in -radius..=radius {
                    for offset_x in -radius..=radius {
                        let sample_x = x as i32 + offset_x;
                        let sample_y = y as i32 + offset_y;
                        if sample_x >= 0
                            && sample_y >= 0
                            && sample_x < image.width() as i32
                            && sample_y < image.height() as i32
                            && is_cape_green(*image.get_pixel(sample_x as u32, sample_y as u32))
                        {
                            near_cape = true;
                            break;
                        }
                    }
                    if near_cape {
                        break;
                    }
                }
                adjacent_gold += u32::from(near_cape);
            }
        }
        let ratio = adjacent_gold as f32 / (body.width * body.height).max(1.0);
        let mut reasons = Vec::new();
        let (minimum_ratio, maximum_ratio, verdict) = match contract {
            CapeHemContractV1::ContinuousGoldLowerHem => {
                let verdict = if ratio < MIN_GOLD_CAPE_HEM_RATIO {
                    reasons.push("cape_hem_gold_trim_missing".into());
                    ConsistencyVerdict::Regenerate
                } else {
                    ConsistencyVerdict::GameReady
                };
                (MIN_GOLD_CAPE_HEM_RATIO, None, verdict)
            }
            CapeHemContractV1::FrontAuthoritativeNoSkirtHem => {
                let verdict = if ratio > MAX_FRONT_AUTHORITY_GOLD_CAPE_HEM_RATIO {
                    reasons.push("cape_skirt_like_gold_lower_hem_present".into());
                    ConsistencyVerdict::Regenerate
                } else {
                    ConsistencyVerdict::GameReady
                };
                (0.0, Some(MAX_FRONT_AUTHORITY_GOLD_CAPE_HEM_RATIO), verdict)
            }
        };
        nodes.push(CapeHemConsistencyNodeV1 {
            node_id: node_id.into(),
            path: path.clone(),
            sha256: sha256.clone(),
            body_width: body.width,
            body_height: body.height,
            lower_hem_start_ratio: CAPE_HEM_START_RATIO,
            lower_hem_end_ratio: CAPE_HEM_END_RATIO,
            gold_adjacent_to_cape_pixel_count: adjacent_gold,
            gold_adjacent_to_cape_ratio: ratio,
            minimum_gold_adjacent_to_cape_ratio: minimum_ratio,
            maximum_gold_adjacent_to_cape_ratio: maximum_ratio,
            verdict,
            reasons,
        });
    }
    if nodes.len() != 4 {
        report_reasons.push("cape_hem_consistency_incomplete".into());
    }
    report_reasons.extend(nodes.iter().flat_map(|node| {
        node.reasons
            .iter()
            .map(|reason| format!("{}:{reason}", node.node_id))
    }));
    let verdict = if nodes.len() == 4
        && nodes
            .iter()
            .all(|node| node.verdict == ConsistencyVerdict::GameReady)
    {
        ConsistencyVerdict::GameReady
    } else {
        ConsistencyVerdict::Regenerate
    };
    CapeHemConsistencyReportV1 {
        schema_version: "1".into(),
        profile: CAPE_HEM_CONSISTENCY_PROFILE.into(),
        contract,
        nodes,
        verdict,
        reasons: report_reasons,
    }
}

fn is_cape_green(pixel: Rgba<u8>) -> bool {
    let [red, green, blue, alpha] = pixel.0;
    alpha > 48
        && red < 180
        && green as f32 >= red as f32 * 0.72
        && green as f32 > blue as f32 * 1.25
        && green.saturating_sub(blue) > 18
}

fn is_cape_gold(pixel: Rgba<u8>) -> bool {
    let [red, green, blue, alpha] = pixel.0;
    alpha > 48
        && red >= 110
        && green >= 60
        && blue <= 125
        && red as f32 > green as f32 * 1.05
        && green as f32 > blue as f32 * 1.20
        && u16::from(red) + u16::from(green) > 220
}

/// Evaluates only evidence that can be derived from immutable input pixels.
/// It deliberately does not encode a ranger, human, gender, garment, or
/// species ontology. The exposed-hand proxy activates only when the canonical
/// image provides a reliable two-sided warm-surface baseline; otherwise the
/// report leaves hand state to the mandatory review stage.
pub fn assess_direction_grid_appearance(
    canonical: &RgbaImage,
    nodes: &BTreeMap<String, RgbaImage>,
    equipment_kind: CharacterEquipmentKindV1,
) -> DirectionGridAppearanceReportV1 {
    let ordered = ["front_idle", "back_idle", "right_idle", "left_idle"];
    let frames = ordered
        .iter()
        .filter_map(|node_id| nodes.get(*node_id).cloned())
        .collect::<Vec<_>>();
    let equipment = assess_hand_equipment_contact_with_kind(
        &BTreeMap::from([("direction_grid".into(), frames)]),
        equipment_kind,
    );

    let seed = exposed_surface_seed(canonical);
    let canonical_counts = seed
        .as_ref()
        .and_then(|seed| distal_exposed_counts(canonical, seed))
        .unwrap_or((0, 0));
    let minimum_baseline = minimum_exposed_pixels(canonical);
    let baseline_available = seed.is_some()
        && canonical_counts.0 >= minimum_baseline
        && canonical_counts.1 >= minimum_baseline;
    let baseline_total = canonical_counts.0.saturating_add(canonical_counts.1).max(1);
    let hand_state_baseline = DirectionGridHandStateBaselineV1 {
        available: baseline_available,
        source: "subject_canonical".into(),
        left_exposed_pixels: canonical_counts.0,
        right_exposed_pixels: canonical_counts.1,
        exterior_elongation_score: exterior_elongation_score(canonical),
    };

    let mut hand_state_nodes = Vec::with_capacity(ordered.len());
    for node_id in ordered {
        let counts = seed
            .as_ref()
            .and_then(|seed| {
                nodes
                    .get(node_id)
                    .and_then(|image| distal_exposed_counts(image, seed))
            })
            .unwrap_or((0, 0));
        let coverage = counts.0.saturating_add(counts.1) as f32 / baseline_total as f32;
        let mut reasons = Vec::new();
        let mut verdict = ConsistencyVerdict::GameReady;
        if !baseline_available {
            reasons.push("exposed_hand_baseline_unavailable".into());
            verdict = ConsistencyVerdict::AwaitingReview;
        } else if node_id == "front_idle" {
            let minimum_left = (canonical_counts.0 / 4).max(3);
            let minimum_right = (canonical_counts.1 / 4).max(3);
            if counts.0 < minimum_left || counts.1 < minimum_right {
                reasons.push("front_hand_state_drift".into());
                verdict = ConsistencyVerdict::AwaitingReview;
            } else if coverage < 0.65 {
                reasons.push("front_hand_coverage_drift".into());
                verdict = ConsistencyVerdict::AwaitingReview;
            }
        }
        hand_state_nodes.push(DirectionGridHandStateNodeV1 {
            node_id: node_id.into(),
            left_exposed_pixels: counts.0,
            right_exposed_pixels: counts.1,
            exposed_coverage_ratio: coverage,
            exterior_elongation_score: nodes
                .get(node_id)
                .map(exterior_elongation_score)
                .unwrap_or_default(),
            verdict,
            reasons,
        });
    }

    let side_total = |node_id: &str| {
        hand_state_nodes
            .iter()
            .find(|node| node.node_id == node_id)
            .map(|node| {
                node.left_exposed_pixels
                    .saturating_add(node.right_exposed_pixels)
            })
            .unwrap_or_default()
    };
    let right_node = hand_state_nodes
        .iter()
        .find(|node| node.node_id == "right_idle")
        .expect("typed direction node");
    let left_node = hand_state_nodes
        .iter()
        .find(|node| node.node_id == "left_idle")
        .expect("typed direction node");
    let right_total = side_total("right_idle");
    let left_total = side_total("left_idle");
    let pair_ratio = |first: u32, second: u32| {
        if first.max(second) == 0 {
            1.0
        } else {
            first.min(second) as f32 / first.max(second) as f32
        }
    };
    let mirrored_ratios = [
        pair_ratio(right_total, left_total),
        pair_ratio(
            right_node.left_exposed_pixels,
            left_node.right_exposed_pixels,
        ),
        pair_ratio(
            right_node.right_exposed_pixels,
            left_node.left_exposed_pixels,
        ),
    ];
    let mirrored_side_coverage_ratio = mirrored_ratios.into_iter().fold(1.0_f32, f32::min);
    let severe_mirrored_pairs = mirrored_ratios
        .into_iter()
        .skip(1)
        .filter(|ratio| *ratio < 0.45)
        .count();
    if baseline_available && severe_mirrored_pairs >= 2 {
        for node in hand_state_nodes
            .iter_mut()
            .filter(|node| matches!(node.node_id.as_str(), "right_idle" | "left_idle"))
        {
            node.reasons.push("mirrored_hand_state_drift".into());
            node.verdict = ConsistencyVerdict::Regenerate;
        }
    } else if baseline_available && mirrored_side_coverage_ratio < 0.65 {
        for node in hand_state_nodes
            .iter_mut()
            .filter(|node| matches!(node.node_id.as_str(), "right_idle" | "left_idle"))
        {
            node.reasons.push("mirrored_hand_state_review".into());
            node.verdict = ConsistencyVerdict::AwaitingReview;
        }
    }
    let hand_state_hard_failure = hand_state_nodes.iter().any(|node| {
        matches!(
            node.verdict,
            ConsistencyVerdict::Blocked | ConsistencyVerdict::Regenerate
        )
    });

    let unexpected_floor = (hand_state_baseline.exterior_elongation_score + 1.0).max(2.0);
    let unexpected_elongated_node_count = if equipment_kind == CharacterEquipmentKindV1::None {
        hand_state_nodes
            .iter()
            .filter(|node| node.exterior_elongation_score >= unexpected_floor)
            .count() as u32
    } else {
        0
    };
    if unexpected_elongated_node_count >= 2 {
        for node in hand_state_nodes
            .iter_mut()
            .filter(|node| node.exterior_elongation_score >= unexpected_floor)
        {
            node.reasons
                .push("unexpected_elongated_exterior_foreground".into());
            if node.verdict == ConsistencyVerdict::GameReady {
                // This is useful evidence for the mandatory human review, but
                // not a universal equipment classifier: capes, wings, tails,
                // sleeves and non-human anatomy can have the same geometry.
                node.verdict = ConsistencyVerdict::AwaitingReview;
            }
        }
    }

    let mut reasons = Vec::new();
    if equipment.verdict != ConsistencyVerdict::GameReady {
        reasons.push("direction_grid_equipment_contract_failed".into());
    }
    if hand_state_hard_failure {
        reasons.push("direction_grid_hand_state_drift".into());
    }
    if unexpected_elongated_node_count >= 2 {
        reasons.push("direction_grid_undeclared_object_evidence".into());
    }
    let verdict = if equipment.verdict != ConsistencyVerdict::GameReady
        || hand_state_nodes.iter().any(|node| {
            matches!(
                node.verdict,
                ConsistencyVerdict::Blocked | ConsistencyVerdict::Regenerate
            )
        }) {
        ConsistencyVerdict::Regenerate
    } else if hand_state_nodes
        .iter()
        .any(|node| node.verdict == ConsistencyVerdict::AwaitingReview)
    {
        ConsistencyVerdict::AwaitingReview
    } else {
        ConsistencyVerdict::GameReady
    };
    DirectionGridAppearanceReportV1 {
        schema_version: "1".into(),
        profile: DIRECTION_GRID_APPEARANCE_PROFILE.into(),
        equipment_kind: equipment_kind_label(equipment_kind).into(),
        equipment,
        hand_state_baseline,
        hand_state_nodes,
        mirrored_side_coverage_ratio,
        unexpected_elongated_node_count,
        verdict,
        reasons,
    }
}

pub fn assess_direction_grid_import_consistency(
    source_images: &BTreeMap<String, RgbaImage>,
    candidate_images: &BTreeMap<String, RgbaImage>,
    source_bindings: &BTreeMap<String, (PathBuf, String)>,
    candidate_bindings: &BTreeMap<String, (PathBuf, String)>,
    absolute_appearance_sha256: &str,
) -> DirectionGridImportConsistencyReportV1 {
    let mut nodes = Vec::with_capacity(4);
    let mut report_reasons = Vec::new();
    for node_id in ["front_idle", "back_idle", "right_idle", "left_idle"] {
        let Some(source) = source_images.get(node_id) else {
            report_reasons.push(format!("direction_grid_source_node_missing:{node_id}"));
            continue;
        };
        let Some(candidate) = candidate_images.get(node_id) else {
            report_reasons.push(format!("direction_grid_candidate_node_missing:{node_id}"));
            continue;
        };
        let Some((source_path, source_sha256)) = source_bindings.get(node_id) else {
            report_reasons.push(format!("direction_grid_source_binding_missing:{node_id}"));
            continue;
        };
        let Some((candidate_path, candidate_sha256)) = candidate_bindings.get(node_id) else {
            report_reasons.push(format!(
                "direction_grid_candidate_binding_missing:{node_id}"
            ));
            continue;
        };
        let identity = evaluate_identity_metric(candidate, source);
        let same_dimensions = source.dimensions() == candidate.dimensions();
        let (body_width_ratio, body_height_ratio, center_drift_px, foot_baseline_drift_px) =
            relative_body_geometry(source, candidate);
        let (added_foreground_ratio, removed_foreground_ratio) =
            silhouette_change_ratios(source, candidate);
        let face_hair_change_ratio = region_change_ratio(source, candidate, 0.18, 0.82, 0.00, 0.38);
        let upper_covering_change_ratio =
            region_change_ratio(source, candidate, 0.05, 0.95, 0.00, 0.33);
        let torso_detail_change_ratio =
            region_change_ratio(source, candidate, 0.18, 0.82, 0.28, 0.66);
        let belt_pouch_change_ratio =
            region_change_ratio(source, candidate, 0.12, 0.88, 0.57, 0.80);
        let sleeve_hand_change_ratio = outer_region_change_ratio(source, candidate, 0.30, 0.70);
        let cape_exterior_change_ratio = outer_region_change_ratio(source, candidate, 0.22, 0.78);
        let source_exterior_elongation_score = exterior_elongation_score(source);
        let candidate_exterior_elongation_score = exterior_elongation_score(candidate);
        let exterior_elongation_delta =
            (candidate_exterior_elongation_score - source_exterior_elongation_score).max(0.0);
        let new_torso_linear_component_count = changed_torso_linear_components(source, candidate);

        let mut reasons = Vec::new();
        let mut verdict = ConsistencyVerdict::GameReady;
        if !same_dimensions {
            reasons.push("direction_grid_pair_dimensions_changed".into());
            verdict = ConsistencyVerdict::Regenerate;
        }
        if identity.scores.composite < 0.85 {
            reasons.push("direction_grid_same_direction_identity_mismatch".into());
            verdict = ConsistencyVerdict::Regenerate;
        }
        if !(0.90..=1.10).contains(&body_width_ratio)
            || !(0.90..=1.10).contains(&body_height_ratio)
            || center_drift_px > 2.0
            || foot_baseline_drift_px > 2.0
        {
            reasons.push("direction_grid_source_relative_geometry_changed".into());
            verdict = ConsistencyVerdict::Regenerate;
        }
        if added_foreground_ratio > 0.08
            || removed_foreground_ratio > 0.30
            || identity.scores.silhouette_iou < 0.65
        {
            reasons.push("direction_grid_permanent_silhouette_change".into());
            verdict = ConsistencyVerdict::Regenerate;
        }
        if new_torso_linear_component_count > 0 && identity.scores.composite >= 0.94 {
            reasons.push("direction_grid_new_torso_linear_detail".into());
            verdict = ConsistencyVerdict::Regenerate;
        }
        if exterior_elongation_delta > 1.0
            && added_foreground_ratio > 0.01
            && removed_foreground_ratio < 0.10
            && identity.scores.composite >= 0.94
        {
            reasons.push("direction_grid_new_exterior_object".into());
            verdict = ConsistencyVerdict::Regenerate;
        }
        if verdict == ConsistencyVerdict::GameReady
            && (identity.scores.composite < 0.95
                || face_hair_change_ratio > 0.10
                || upper_covering_change_ratio > 0.12
                || torso_detail_change_ratio > 0.10
                || belt_pouch_change_ratio > 0.14
                || sleeve_hand_change_ratio > 0.15
                || cape_exterior_change_ratio > 0.16)
        {
            if identity.scores.composite < 0.95 {
                reasons.push("direction_grid_identity_detail_review".into());
            }
            if face_hair_change_ratio > 0.10 {
                reasons.push("direction_grid_face_hair_detail_review".into());
            }
            if upper_covering_change_ratio > 0.12 {
                reasons.push("direction_grid_upper_covering_detail_review".into());
            }
            if torso_detail_change_ratio > 0.10 {
                reasons.push("direction_grid_torso_detail_review".into());
            }
            if belt_pouch_change_ratio > 0.14 {
                reasons.push("direction_grid_belt_pouch_detail_review".into());
            }
            if sleeve_hand_change_ratio > 0.15 {
                reasons.push("direction_grid_sleeve_hand_detail_review".into());
            }
            if cape_exterior_change_ratio > 0.16 {
                reasons.push("direction_grid_cape_exterior_detail_review".into());
            }
            verdict = ConsistencyVerdict::AwaitingReview;
        }
        nodes.push(DirectionGridImportConsistencyNodeV1 {
            node_id: node_id.into(),
            source_path: source_path.clone(),
            source_sha256: source_sha256.clone(),
            candidate_path: candidate_path.clone(),
            candidate_sha256: candidate_sha256.clone(),
            identity_scores: identity.scores,
            body_width_ratio,
            body_height_ratio,
            center_drift_px,
            foot_baseline_drift_px,
            added_foreground_ratio,
            removed_foreground_ratio,
            face_hair_change_ratio,
            upper_covering_change_ratio,
            torso_detail_change_ratio,
            belt_pouch_change_ratio,
            sleeve_hand_change_ratio,
            cape_exterior_change_ratio,
            source_exterior_elongation_score,
            candidate_exterior_elongation_score,
            exterior_elongation_delta,
            new_torso_linear_component_count,
            verdict,
            reasons,
        });
    }
    if nodes.len() != 4 {
        report_reasons.push("direction_grid_import_consistency_incomplete".into());
    }
    if nodes.iter().any(|node| {
        matches!(
            node.verdict,
            ConsistencyVerdict::Blocked | ConsistencyVerdict::Regenerate
        )
    }) {
        report_reasons.push("direction_grid_import_relative_hard_failure".into());
    } else if nodes
        .iter()
        .any(|node| node.verdict == ConsistencyVerdict::AwaitingReview)
    {
        report_reasons.push("direction_grid_import_relative_review_required".into());
    }
    report_reasons.extend(nodes.iter().flat_map(|node| {
        node.reasons
            .iter()
            .map(|reason| format!("{}:{reason}", node.node_id))
    }));
    let verdict = if nodes.len() != 4
        || nodes.iter().any(|node| {
            matches!(
                node.verdict,
                ConsistencyVerdict::Blocked | ConsistencyVerdict::Regenerate
            )
        }) {
        ConsistencyVerdict::Regenerate
    } else if nodes
        .iter()
        .any(|node| node.verdict == ConsistencyVerdict::AwaitingReview)
    {
        ConsistencyVerdict::AwaitingReview
    } else {
        ConsistencyVerdict::GameReady
    };
    DirectionGridImportConsistencyReportV1 {
        schema_version: "1".into(),
        profile: DIRECTION_GRID_IMPORT_CONSISTENCY_PROFILE.into(),
        absolute_appearance_profile: DIRECTION_GRID_APPEARANCE_PROFILE.into(),
        absolute_appearance_sha256: absolute_appearance_sha256.into(),
        nodes,
        verdict,
        reasons: report_reasons,
    }
}

fn relative_body_geometry(source: &RgbaImage, candidate: &RgbaImage) -> (f32, f32, f32, f32) {
    let (Some(source_body), Some(candidate_body)) =
        (character_body_bbox(source), character_body_bbox(candidate))
    else {
        return (0.0, 0.0, f32::MAX, f32::MAX);
    };
    (
        candidate_body.width / source_body.width.max(1.0),
        candidate_body.height / source_body.height.max(1.0),
        (candidate_body.center_x - source_body.center_x).abs(),
        (candidate_body.bottom_y - source_body.bottom_y).abs(),
    )
}

fn silhouette_change_ratios(source: &RgbaImage, candidate: &RgbaImage) -> (f32, f32) {
    if source.dimensions() != candidate.dimensions() {
        return (1.0, 1.0);
    }
    let mut union = 0usize;
    let mut added = 0usize;
    let mut removed = 0usize;
    for (source_pixel, candidate_pixel) in source.pixels().zip(candidate.pixels()) {
        let source_foreground = source_pixel[3] > 48;
        let candidate_foreground = candidate_pixel[3] > 48;
        if source_foreground || candidate_foreground {
            union += 1;
        }
        added += usize::from(!source_foreground && candidate_foreground);
        removed += usize::from(source_foreground && !candidate_foreground);
    }
    (
        added as f32 / union.max(1) as f32,
        removed as f32 / union.max(1) as f32,
    )
}

fn region_change_ratio(
    source: &RgbaImage,
    candidate: &RgbaImage,
    left_fraction: f32,
    right_fraction: f32,
    top_fraction: f32,
    bottom_fraction: f32,
) -> f32 {
    if source.dimensions() != candidate.dimensions() {
        return 1.0;
    }
    let Some(body) = character_body_bbox(source) else {
        return 1.0;
    };
    let left = (body.left + body.width * left_fraction).floor().max(0.0) as u32;
    let right = (body.left + body.width * right_fraction)
        .ceil()
        .min(source.width() as f32) as u32;
    let top = (body.top + body.height * top_fraction).floor().max(0.0) as u32;
    let bottom = (body.top + body.height * bottom_fraction)
        .ceil()
        .min(source.height() as f32) as u32;
    changed_ratio_in_ranges(source, candidate, left, right, top, bottom, None)
}

fn outer_region_change_ratio(
    source: &RgbaImage,
    candidate: &RgbaImage,
    left_cut_fraction: f32,
    right_cut_fraction: f32,
) -> f32 {
    if source.dimensions() != candidate.dimensions() {
        return 1.0;
    }
    let Some(body) = character_body_bbox(source) else {
        return 1.0;
    };
    let left = body.left.floor().max(0.0) as u32;
    let right = body.right.ceil().min(source.width() as f32) as u32;
    let top = (body.top + body.height * 0.25).floor().max(0.0) as u32;
    let bottom = (body.top + body.height * 0.86)
        .ceil()
        .min(source.height() as f32) as u32;
    let left_cut = body.left + body.width * left_cut_fraction;
    let right_cut = body.left + body.width * right_cut_fraction;
    changed_ratio_in_ranges(
        source,
        candidate,
        left,
        right,
        top,
        bottom,
        Some((left_cut, right_cut)),
    )
}

fn changed_ratio_in_ranges(
    source: &RgbaImage,
    candidate: &RgbaImage,
    left: u32,
    right: u32,
    top: u32,
    bottom: u32,
    exterior_cut: Option<(f32, f32)>,
) -> f32 {
    let mut relevant = 0usize;
    let mut changed = 0usize;
    for y in top..bottom {
        for x in left..right {
            if exterior_cut.is_some_and(|(inner_left, inner_right)| {
                x as f32 >= inner_left && x as f32 <= inner_right
            }) {
                continue;
            }
            let source_pixel = source.get_pixel(x, y);
            let candidate_pixel = candidate.get_pixel(x, y);
            if source_pixel[3] > 32 || candidate_pixel[3] > 32 {
                relevant += 1;
                if (source_pixel[3] > 48) != (candidate_pixel[3] > 48)
                    || rgb_distance_squared(source_pixel, candidate_pixel) > 55 * 55
                {
                    changed += 1;
                }
            }
        }
    }
    changed as f32 / relevant.max(1) as f32
}

fn changed_torso_linear_components(source: &RgbaImage, candidate: &RgbaImage) -> u32 {
    if source.dimensions() != candidate.dimensions() {
        return 1;
    }
    let Some(body) = character_body_bbox(source) else {
        return 1;
    };
    let left = (body.left + body.width * 0.15).floor().max(0.0) as u32;
    let right = (body.left + body.width * 0.85)
        .ceil()
        .min(source.width() as f32) as u32;
    let top = (body.top + body.height * 0.28).floor().max(0.0) as u32;
    let bottom = (body.top + body.height * 0.68)
        .ceil()
        .min(source.height() as f32) as u32;
    let mut changed = std::collections::BTreeSet::new();
    for y in top..bottom {
        for x in left..right {
            let first = source.get_pixel(x, y);
            let second = candidate.get_pixel(x, y);
            if first[3] > 48 && second[3] > 48 && rgb_distance_squared(first, second) > 90 * 90 {
                changed.insert((x, y));
            }
        }
    }
    let minimum_area = ((body.width * body.height) / 1200.0).round().max(6.0) as usize;
    let minimum_span = body.width.min(body.height) * 0.18;
    let mut visited = std::collections::BTreeSet::new();
    let mut count = 0u32;
    for start in changed.iter().copied() {
        if !visited.insert(start) {
            continue;
        }
        let mut queue = std::collections::VecDeque::from([start]);
        let mut points = Vec::new();
        while let Some((x, y)) = queue.pop_front() {
            points.push((x as f32, y as f32));
            for next_y in y.saturating_sub(1)..=(y + 1).min(candidate.height() - 1) {
                for next_x in x.saturating_sub(1)..=(x + 1).min(candidate.width() - 1) {
                    if changed.contains(&(next_x, next_y)) && visited.insert((next_x, next_y)) {
                        queue.push_back((next_x, next_y));
                    }
                }
            }
        }
        if points.len() < minimum_area {
            continue;
        }
        let mean_x = points.iter().map(|point| point.0).sum::<f32>() / points.len() as f32;
        let mean_y = points.iter().map(|point| point.1).sum::<f32>() / points.len() as f32;
        let (mut xx, mut yy, mut xy) = (0.0_f32, 0.0_f32, 0.0_f32);
        for (x, y) in &points {
            let dx = *x - mean_x;
            let dy = *y - mean_y;
            xx += dx * dx;
            yy += dy * dy;
            xy += dx * dy;
        }
        xx /= points.len() as f32;
        yy /= points.len() as f32;
        xy /= points.len() as f32;
        let trace = xx + yy;
        let discriminant = ((xx - yy) * (xx - yy) + 4.0 * xy * xy).sqrt();
        let major = ((trace + discriminant) * 0.5).max(0.0);
        let minor = ((trace - discriminant) * 0.5).max(0.01);
        if major / minor >= 8.0 && major.sqrt() * 4.0 >= minimum_span {
            count = count.saturating_add(1);
        }
    }
    count
}

fn equipment_kind_label(kind: CharacterEquipmentKindV1) -> &'static str {
    match kind {
        CharacterEquipmentKindV1::None => "none",
        CharacterEquipmentKindV1::StaffLike => "staff_like",
    }
}

fn minimum_exposed_pixels(image: &RgbaImage) -> u32 {
    character_body_bbox(image)
        .map(|body| ((body.width * body.height) / 3000.0).round() as u32)
        .unwrap_or(3)
        .clamp(3, 16)
}

fn exterior_elongation_score(image: &RgbaImage) -> f32 {
    let Some(body) = character_body_bbox(image) else {
        return 0.0;
    };
    let mut exterior = std::collections::BTreeSet::new();
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] > 48
            && y as f32 >= body.top
            && y as f32 <= body.bottom_y
            && (x as f32 + 2.0 < body.left || x as f32 > body.right + 2.0)
        {
            exterior.insert((x, y));
        }
    }
    let minimum_area = ((body.width * body.height) * 0.002).round().max(12.0) as usize;
    let maximum_width = (body.width * 0.25).ceil().max(4.0) as u32;
    let minimum_height = (body.height * 0.10).ceil().max(8.0) as u32;
    let mut visited = std::collections::BTreeSet::new();
    let mut maximum = 0.0_f32;
    for start in exterior.iter().copied() {
        if !visited.insert(start) {
            continue;
        }
        let mut queue = std::collections::VecDeque::from([start]);
        let (mut left, mut right, mut top, mut bottom) = (start.0, start.0, start.1, start.1);
        let mut area = 0usize;
        while let Some((x, y)) = queue.pop_front() {
            area += 1;
            left = left.min(x);
            right = right.max(x);
            top = top.min(y);
            bottom = bottom.max(y);
            for next_y in y.saturating_sub(1)..=(y + 1).min(image.height() - 1) {
                for next_x in x.saturating_sub(1)..=(x + 1).min(image.width() - 1) {
                    if exterior.contains(&(next_x, next_y)) && visited.insert((next_x, next_y)) {
                        queue.push_back((next_x, next_y));
                    }
                }
            }
        }
        let width = right - left + 1;
        let height = bottom - top + 1;
        if area >= minimum_area && width <= maximum_width && height >= minimum_height {
            maximum = maximum.max(height as f32 / width.max(1) as f32);
        }
    }
    maximum
}

fn exposed_surface_seed(image: &RgbaImage) -> Option<Rgba<u8>> {
    let body = character_body_bbox(image)?;
    let left = (body.left + body.width * 0.25).floor().max(0.0) as u32;
    let right = (body.right - body.width * 0.25)
        .ceil()
        .min(image.width() as f32) as u32;
    let top = (body.top + body.height * 0.10).floor().max(0.0) as u32;
    let bottom = (body.top + body.height * 0.42)
        .ceil()
        .min(image.height() as f32) as u32;
    let mut candidates = Vec::new();
    for y in top..bottom {
        for x in left..right {
            let pixel = *image.get_pixel(x, y);
            if probable_exposed_surface(&pixel) {
                candidates.push(pixel);
            }
        }
    }
    candidates.sort_by_key(|pixel| {
        std::cmp::Reverse(u32::from(pixel[0]) * 3 + u32::from(pixel[1]) * 5 + u32::from(pixel[2]))
    });
    candidates.into_iter().next()
}

fn distal_exposed_counts(image: &RgbaImage, seed: &Rgba<u8>) -> Option<(u32, u32)> {
    let body = character_body_bbox(image)?;
    let top = (body.top + body.height * 0.36).floor().max(0.0) as u32;
    let bottom = (body.top + body.height * 0.76)
        .ceil()
        .min(image.height() as f32) as u32;
    let left_start = (body.left - body.width * 0.08).floor().max(0.0) as u32;
    let left_end = (body.left + body.width * 0.34)
        .ceil()
        .min(image.width() as f32) as u32;
    let right_start = (body.right - body.width * 0.34).floor().max(0.0) as u32;
    let right_end = (body.right + body.width * 0.08)
        .ceil()
        .min(image.width() as f32) as u32;
    let count = |start: u32, end: u32| {
        (top..bottom)
            .flat_map(|y| (start..end).map(move |x| (x, y)))
            .filter(|(x, y)| {
                let pixel = image.get_pixel(*x, *y);
                pixel[3] > 48 && rgb_distance_squared(pixel, seed) <= 85 * 85
            })
            .count()
            .min(u32::MAX as usize) as u32
    };
    Some((count(left_start, left_end), count(right_start, right_end)))
}

fn probable_exposed_surface(pixel: &Rgba<u8>) -> bool {
    let red = pixel[0];
    let green = pixel[1];
    let blue = pixel[2];
    let maximum = red.max(green).max(blue);
    let minimum = red.min(green).min(blue);
    pixel[3] > 48
        && red.saturating_add(8) >= green
        && green.saturating_add(12) >= blue
        && red > blue.saturating_add(10)
        && maximum.saturating_sub(minimum) >= 12
        && maximum >= 55
        && minimum <= 235
}

fn rgb_distance_squared(left: &Rgba<u8>, right: &Rgba<u8>) -> u32 {
    (0..3)
        .map(|channel| {
            let delta = i32::from(left[channel]) - i32::from(right[channel]);
            (delta * delta) as u32
        })
        .sum()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectionGridApprovalV1 {
    pub schema_version: String,
    pub profile: String,
    pub source_job_id: String,
    pub lock_sha256: String,
    pub node_sha256: BTreeMap<String, String>,
    pub accepted: bool,
    pub reason: String,
    pub reviewed_at: DateTime<Utc>,
}

impl DirectionGridApprovalV1 {
    pub fn matches_lock(&self, lock: &DirectionGridLockV1, lock_sha256: &str) -> bool {
        self.accepted
            && self.profile == GRID_APPROVAL_PROFILE
            && self.lock_sha256 == lock_sha256
            && self.node_sha256 == lock.node_hashes()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GridActionCellV1 {
    pub frame_index: u8,
    pub path: PathBuf,
    pub sha256: String,
    pub reused: bool,
    pub provider_request_occurred: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GridKeyframePhaseV1 {
    LeftContact,
    LeftPassing,
    RightContact,
    RightPassing,
}

impl GridKeyframePhaseV1 {
    pub const fn for_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::LeftContact),
            1 => Some(Self::LeftPassing),
            2 => Some(Self::RightContact),
            3 => Some(Self::RightPassing),
            _ => None,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::LeftContact => "left contact",
            Self::LeftPassing => "left passing",
            Self::RightContact => "right contact",
            Self::RightPassing => "right passing",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GridKeyframeGenerationMethodV1 {
    IndependentDirectionAnchor,
    StaticFreshRetry,
    LateralityFreshRetry,
    AsymmetricGuideFreshRetry,
    PlatformSafeGuideFreshRetry,
    FootwearCleanupEdit,
    MotionDiagnosticEdit,
    ChildFrameRetry,
    ByteReuse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GridKeyframeFrameV1 {
    pub frame_index: u8,
    pub phase: GridKeyframePhaseV1,
    pub attempt: u8,
    pub generation_method: GridKeyframeGenerationMethodV1,
    pub path: PathBuf,
    pub sha256: String,
    pub reused: bool,
    pub provider_request_occurred: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_frame_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaces_frame_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pose_structure_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pose_structure_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pose_structure_profile: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GridKeyframeActionReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub animation: String,
    pub direction_node_id: String,
    pub direction_anchor_sha256: String,
    pub phase_profile: String,
    pub frames: Vec<GridKeyframeFrameV1>,
    pub motion_report_path: PathBuf,
    pub motion_report_sha256: String,
    pub motion_verdict: ConsistencyVerdict,
    pub equipment_report_path: PathBuf,
    pub equipment_report_sha256: String,
    pub equipment_verdict: ConsistencyVerdict,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footwear_report_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footwear_report_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footwear_verdict: Option<ConsistencyVerdict>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub laterality_report_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub laterality_report_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub laterality_verdict: Option<ConsistencyVerdict>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recommended_retry_frames: Vec<u8>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retried_frames: Vec<u8>,
}

impl GridKeyframeActionReportV1 {
    pub fn validate_structure(&self, expected_animation: &str) -> Result<(), String> {
        if self.schema_version != "1"
            || !matches!(
                self.profile.as_str(),
                GRID_KEYFRAME_ACTION_REPORT_PROFILE
                    | GRID_STRUCTURED_GAIT_ACTION_REPORT_PROFILE_LEGACY
                    | GRID_STRUCTURED_GAIT_ACTION_REPORT_PROFILE
                    | GRID_ASYMMETRIC_GAIT_ACTION_REPORT_PROFILE
                    | GRID_PLATFORM_SAFE_GAIT_ACTION_REPORT_PROFILE
                    | GRID_FOOTWEAR_CLEANUP_ACTION_REPORT_PROFILE
            )
            || self.animation != expected_animation
            || self.phase_profile != GRID_ACTION_PHASE_PROFILE
        {
            return Err("grid_keyframe_report_contract_invalid".into());
        }
        let expected_direction = match expected_animation {
            "walk_down" => "front_idle",
            "walk_up" => "back_idle",
            "walk_right" => "right_idle",
            "walk_left" => "left_idle",
            _ => return Err("grid_keyframe_report_animation_invalid".into()),
        };
        if self.direction_node_id != expected_direction || self.frames.len() != 4 {
            return Err("grid_keyframe_report_not_reusable".into());
        }
        let structured = matches!(
            self.profile.as_str(),
            GRID_STRUCTURED_GAIT_ACTION_REPORT_PROFILE_LEGACY
                | GRID_STRUCTURED_GAIT_ACTION_REPORT_PROFILE
                | GRID_ASYMMETRIC_GAIT_ACTION_REPORT_PROFILE
                | GRID_PLATFORM_SAFE_GAIT_ACTION_REPORT_PROFILE
                | GRID_FOOTWEAR_CLEANUP_ACTION_REPORT_PROFILE
        );
        if structured
            && (self.animation != "walk_down"
                || self.laterality_verdict.is_none()
                || self.laterality_report_path.is_none()
                || self
                    .laterality_report_sha256
                    .as_deref()
                    .is_none_or(|hash| hash.len() != 64))
        {
            return Err("grid_keyframe_report_laterality_invalid".into());
        }
        if !structured
            && (self.laterality_report_path.is_some()
                || self.laterality_report_sha256.is_some()
                || self.laterality_verdict.is_some())
        {
            return Err("grid_keyframe_report_workflow_version_mismatch".into());
        }
        for (index, frame) in self.frames.iter().enumerate() {
            let index = index as u8;
            if frame.frame_index != index
                || GridKeyframePhaseV1::for_index(index) != Some(frame.phase)
                || frame.sha256.len() != 64
                || (frame.reused
                    != matches!(
                        frame.generation_method,
                        GridKeyframeGenerationMethodV1::ByteReuse
                    ))
                || frame.provider_request_occurred == frame.reused
                || (matches!(
                    frame.generation_method,
                    GridKeyframeGenerationMethodV1::MotionDiagnosticEdit
                        | GridKeyframeGenerationMethodV1::FootwearCleanupEdit
                ) && frame.input_frame_sha256.is_none())
                || (!matches!(
                    frame.generation_method,
                    GridKeyframeGenerationMethodV1::MotionDiagnosticEdit
                        | GridKeyframeGenerationMethodV1::FootwearCleanupEdit
                ) && frame.input_frame_sha256.is_some())
                || (matches!(
                    frame.generation_method,
                    GridKeyframeGenerationMethodV1::LateralityFreshRetry
                ) && (self.profile != GRID_STRUCTURED_GAIT_ACTION_REPORT_PROFILE
                    || frame.attempt < 2))
                || (matches!(
                    frame.generation_method,
                    GridKeyframeGenerationMethodV1::AsymmetricGuideFreshRetry
                ) && (self.profile != GRID_ASYMMETRIC_GAIT_ACTION_REPORT_PROFILE
                    || frame.attempt < 2
                    || frame.pose_structure_profile.as_deref()
                        != Some(crate::grid_pose_structure::GRID_POSE_STRUCTURE_PROFILE)))
                || (matches!(
                    frame.generation_method,
                    GridKeyframeGenerationMethodV1::PlatformSafeGuideFreshRetry
                ) && (self.profile != GRID_PLATFORM_SAFE_GAIT_ACTION_REPORT_PROFILE
                    || frame.attempt < 2
                    || frame.pose_structure_profile.as_deref()
                        != Some(crate::grid_pose_structure::GRID_POSE_STRUCTURE_PROFILE_PLATFORM_SAFE)))
                || (matches!(
                    frame.generation_method,
                    GridKeyframeGenerationMethodV1::FootwearCleanupEdit
                ) && (self.profile != GRID_FOOTWEAR_CLEANUP_ACTION_REPORT_PROFILE
                    || frame.attempt < 2))
                || (matches!(
                    frame.generation_method,
                    GridKeyframeGenerationMethodV1::ChildFrameRetry
                ) && frame.replaces_frame_sha256.is_none())
                || (structured
                    && !(self.profile == GRID_FOOTWEAR_CLEANUP_ACTION_REPORT_PROFILE
                        && frame.frame_index == 2)
                    && (frame.pose_structure_path.is_none()
                        || frame
                            .pose_structure_sha256
                            .as_deref()
                            .is_none_or(|hash| hash.len() != 64)))
                || (matches!(
                    self.profile.as_str(),
                    GRID_ASYMMETRIC_GAIT_ACTION_REPORT_PROFILE
                        | GRID_PLATFORM_SAFE_GAIT_ACTION_REPORT_PROFILE
                        | GRID_FOOTWEAR_CLEANUP_ACTION_REPORT_PROFILE
                )
                    && !(self.profile == GRID_FOOTWEAR_CLEANUP_ACTION_REPORT_PROFILE
                        && frame.frame_index == 2)
                    && frame
                        .pose_structure_profile
                        .as_deref()
                        .is_none_or(|profile| {
                            profile != crate::grid_pose_structure::GRID_POSE_STRUCTURE_PROFILE_LEGACY
                                && !((self.profile == GRID_ASYMMETRIC_GAIT_ACTION_REPORT_PROFILE
                                    && profile
                                        == crate::grid_pose_structure::GRID_POSE_STRUCTURE_PROFILE)
                                    || (self.profile
                                        == GRID_PLATFORM_SAFE_GAIT_ACTION_REPORT_PROFILE
                                        && profile
                                            == crate::grid_pose_structure::GRID_POSE_STRUCTURE_PROFILE_PLATFORM_SAFE))
                        }))
                || (!matches!(
                    self.profile.as_str(),
                    GRID_ASYMMETRIC_GAIT_ACTION_REPORT_PROFILE
                        | GRID_PLATFORM_SAFE_GAIT_ACTION_REPORT_PROFILE
                        | GRID_FOOTWEAR_CLEANUP_ACTION_REPORT_PROFILE
                )
                    && frame.pose_structure_profile.is_some())
                || (!structured
                    && (frame.pose_structure_path.is_some()
                        || frame.pose_structure_sha256.is_some()
                        || frame.pose_structure_profile.is_some()))
            {
                return Err("grid_keyframe_report_frame_contract_invalid".into());
            }
        }
        if self.profile == GRID_ASYMMETRIC_GAIT_ACTION_REPORT_PROFILE {
            let asymmetric = self
                .frames
                .iter()
                .filter(|frame| {
                    frame.generation_method
                        == GridKeyframeGenerationMethodV1::AsymmetricGuideFreshRetry
                        && frame.provider_request_occurred
                        && !frame.reused
                })
                .map(|frame| frame.frame_index)
                .collect::<Vec<_>>();
            if asymmetric != [2]
                || self.retried_frames != asymmetric
                || self
                    .frames
                    .iter()
                    .filter(|frame| {
                        frame.generation_method == GridKeyframeGenerationMethodV1::ByteReuse
                    })
                    .count()
                    != 3
            {
                return Err("grid_keyframe_report_asymmetric_child_invalid".into());
            }
        }
        if self.profile == GRID_PLATFORM_SAFE_GAIT_ACTION_REPORT_PROFILE {
            let platform_safe = self
                .frames
                .iter()
                .filter(|frame| {
                    frame.generation_method
                        == GridKeyframeGenerationMethodV1::PlatformSafeGuideFreshRetry
                        && frame.provider_request_occurred
                        && !frame.reused
                })
                .map(|frame| frame.frame_index)
                .collect::<Vec<_>>();
            if platform_safe != [2]
                || self.retried_frames != platform_safe
                || self
                    .frames
                    .iter()
                    .filter(|frame| {
                        frame.generation_method == GridKeyframeGenerationMethodV1::ByteReuse
                    })
                    .count()
                    != 3
                || self.footwear_verdict.is_none()
                || self.footwear_report_path.is_none()
                || self
                    .footwear_report_sha256
                    .as_deref()
                    .is_none_or(|hash| hash.len() != 64)
            {
                return Err("grid_keyframe_report_platform_safe_child_invalid".into());
            }
        } else if self.profile == GRID_FOOTWEAR_CLEANUP_ACTION_REPORT_PROFILE {
            let cleanup = self
                .frames
                .iter()
                .filter(|frame| {
                    frame.generation_method == GridKeyframeGenerationMethodV1::FootwearCleanupEdit
                        && frame.provider_request_occurred
                        && !frame.reused
                })
                .map(|frame| frame.frame_index)
                .collect::<Vec<_>>();
            if cleanup != [2]
                || self.retried_frames != cleanup
                || self.frames.iter().filter(|frame| frame.reused).count() != 3
                || self.footwear_verdict.is_none()
                || self.footwear_report_path.is_none()
                || self
                    .footwear_report_sha256
                    .as_deref()
                    .is_none_or(|hash| hash.len() != 64)
            {
                return Err("grid_keyframe_report_footwear_cleanup_child_invalid".into());
            }
        } else if self.footwear_report_path.is_some()
            || self.footwear_report_sha256.is_some()
            || self.footwear_verdict.is_some()
        {
            return Err("grid_keyframe_report_workflow_version_mismatch".into());
        }
        Ok(())
    }

    pub fn validate_for_reuse(&self, expected_animation: &str) -> Result<(), String> {
        self.validate_structure(expected_animation)?;
        if self.motion_verdict != ConsistencyVerdict::GameReady
            || self.equipment_verdict != ConsistencyVerdict::GameReady
            || (matches!(
                self.profile.as_str(),
                GRID_STRUCTURED_GAIT_ACTION_REPORT_PROFILE_LEGACY
                    | GRID_STRUCTURED_GAIT_ACTION_REPORT_PROFILE
                    | GRID_ASYMMETRIC_GAIT_ACTION_REPORT_PROFILE
                    | GRID_PLATFORM_SAFE_GAIT_ACTION_REPORT_PROFILE
                    | GRID_FOOTWEAR_CLEANUP_ACTION_REPORT_PROFILE
            ) && self.laterality_verdict != Some(ConsistencyVerdict::GameReady))
            || (matches!(
                self.profile.as_str(),
                GRID_PLATFORM_SAFE_GAIT_ACTION_REPORT_PROFILE
                    | GRID_FOOTWEAR_CLEANUP_ACTION_REPORT_PROFILE
            ) && self.footwear_verdict != Some(ConsistencyVerdict::GameReady))
        {
            return Err("grid_keyframe_report_not_reusable".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GridActionPhaseEvidenceV1 {
    pub profile: String,
    pub verified_cyclic_order: bool,
    pub distinct_pose_count: usize,
    pub phase_order_score: f32,
    pub opposing_contact_change_ratio: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GridActionReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub animation: String,
    pub attempt: u8,
    pub pose_guidance: GridPoseGuidanceV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase_profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase_evidence: Option<GridActionPhaseEvidenceV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_method: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repair_reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repaired_frames: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_sheet_sha256: Option<String>,
    pub sheet_path: PathBuf,
    pub sheet_sha256: String,
    pub cells: Vec<GridActionCellV1>,
}

impl GridActionReportV1 {
    pub fn validate_for_reuse(&self, expected_animation: &str) -> Result<(), String> {
        if self.animation != expected_animation {
            return Err("grid_action_report_animation_mismatch".into());
        }
        if !matches!(
            self.profile.as_str(),
            "grid-action-report@1.0.0" | GRID_ACTION_REPORT_PROFILE
        ) {
            return Err("grid_action_report_profile_unsupported".into());
        }
        let indices = self
            .cells
            .iter()
            .map(|cell| cell.frame_index)
            .collect::<std::collections::BTreeSet<_>>();
        if self.cells.len() != 4 || indices != std::collections::BTreeSet::from([0, 1, 2, 3]) {
            return Err("grid_action_report_frame_contract_invalid".into());
        }
        if self.profile == GRID_ACTION_REPORT_PROFILE {
            if self.phase_profile.as_deref() != Some(GRID_ACTION_PHASE_PROFILE) {
                return Err("grid_action_report_phase_profile_invalid".into());
            }
            let evidence = self
                .phase_evidence
                .as_ref()
                .ok_or("grid_action_report_phase_evidence_missing")?;
            if evidence.profile != GRID_ACTION_PHASE_EVIDENCE_PROFILE
                || !evidence.verified_cyclic_order
                || evidence.distinct_pose_count < 4
                || evidence.phase_order_score < 0.45
                || evidence.opposing_contact_change_ratio < 0.04
            {
                return Err("grid_action_report_phase_evidence_invalid".into());
            }
            let method = self
                .generation_method
                .as_deref()
                .ok_or("grid_action_report_generation_method_missing")?;
            if !matches!(
                method,
                "fresh_action_grid"
                    | "diagnostic_edit_retry"
                    | "targeted_cell_retry"
                    | "legacy_local_recheck"
            ) {
                return Err("grid_action_report_generation_method_invalid".into());
            }
            if method == "diagnostic_edit_retry"
                && (self.input_sheet_sha256.is_none()
                    || self.repair_reasons.is_empty()
                    || self.repaired_frames.is_empty())
            {
                return Err("grid_action_report_retry_provenance_missing".into());
            }
        }
        Ok(())
    }
}

pub fn direction_grid_node_id(direction: DirectionViewV1) -> &'static str {
    match direction {
        DirectionViewV1::Front => "front_idle",
        DirectionViewV1::Rear => "back_idle",
        DirectionViewV1::Right => "right_idle",
        DirectionViewV1::Left => "left_idle",
    }
}

pub fn grid_animations() -> [&'static str; 4] {
    ["walk_down", "walk_up", "walk_right", "walk_left"]
}

pub fn grid_idle_animations() -> [(&'static str, &'static str); 4] {
    [
        ("idle_down", "front_idle"),
        ("idle_up", "back_idle"),
        ("idle_right", "right_idle"),
        ("idle_left", "left_idle"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bare_hand_sprite() -> RgbaImage {
        let mut image = RgbaImage::from_pixel(96, 96, Rgba([0, 0, 0, 0]));
        let cloth = Rgba([112, 58, 156, 255]);
        for y in 26..68 {
            for x in 34..62 {
                image.put_pixel(x, y, cloth);
            }
        }
        for y in 68..86 {
            for x in 32..40 {
                image.put_pixel(x, y, cloth);
            }
            for x in 54..62 {
                image.put_pixel(x, y, cloth);
            }
        }
        let skin = Rgba([190, 126, 88, 255]);
        for y in 33..51 {
            for x in 39..57 {
                image.put_pixel(x, y, skin);
            }
        }
        for y in 54..63 {
            for x in 28..35 {
                image.put_pixel(x, y, skin);
            }
            for x in 61..68 {
                image.put_pixel(x, y, skin);
            }
        }
        image
    }

    fn four_nodes(image: &RgbaImage) -> BTreeMap<String, RgbaImage> {
        ["front_idle", "back_idle", "right_idle", "left_idle"]
            .into_iter()
            .map(|node| (node.into(), image.clone()))
            .collect()
    }

    fn four_bindings(prefix: &str) -> BTreeMap<String, (PathBuf, String)> {
        ["front_idle", "back_idle", "right_idle", "left_idle"]
            .into_iter()
            .enumerate()
            .map(|(index, node)| {
                (
                    node.into(),
                    (
                        format!("/{prefix}/{node}.png").into(),
                        format!("{index:x}").repeat(64),
                    ),
                )
            })
            .collect()
    }

    #[test]
    fn appearance_accepts_consistent_bare_hands_without_equipment() {
        let canonical = bare_hand_sprite();
        let report = assess_direction_grid_appearance(
            &canonical,
            &four_nodes(&canonical),
            CharacterEquipmentKindV1::None,
        );
        assert!(report.hand_state_baseline.available);
        assert_eq!(report.equipment.verdict, ConsistencyVerdict::GameReady);
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
    }

    #[test]
    fn appearance_blocks_floating_undeclared_staff_shape() {
        let canonical = bare_hand_sprite();
        let mut nodes = four_nodes(&canonical);
        for image in nodes.values_mut() {
            for y in 24..86 {
                for x in 76..79 {
                    image.put_pixel(x, y, Rgba([92, 58, 32, 255]));
                }
            }
        }
        let report =
            assess_direction_grid_appearance(&canonical, &nodes, CharacterEquipmentKindV1::None);
        assert_eq!(report.equipment.verdict, ConsistencyVerdict::Blocked);
        assert_eq!(report.verdict, ConsistencyVerdict::Regenerate);
        assert!(report
            .reasons
            .contains(&"direction_grid_equipment_contract_failed".into()));
    }

    #[test]
    fn appearance_flags_one_glove_for_review_without_overclaiming_a_hard_failure() {
        let canonical = bare_hand_sprite();
        let mut nodes = four_nodes(&canonical);
        let right = nodes.get_mut("right_idle").unwrap();
        for y in 54..63 {
            for x in 28..35 {
                right.put_pixel(x, y, Rgba([28, 24, 34, 255]));
            }
        }
        let report =
            assess_direction_grid_appearance(&canonical, &nodes, CharacterEquipmentKindV1::None);
        assert_eq!(report.verdict, ConsistencyVerdict::AwaitingReview);
        assert!(report.hand_state_nodes.iter().any(|node| {
            node.node_id == "right_idle"
                && node.reasons.contains(&"mirrored_hand_state_review".into())
        }));
    }

    #[test]
    fn appearance_does_not_impose_bare_hand_rule_without_a_baseline() {
        let canonical = RgbaImage::from_pixel(96, 96, Rgba([75, 90, 110, 255]));
        let report = assess_direction_grid_appearance(
            &canonical,
            &four_nodes(&canonical),
            CharacterEquipmentKindV1::None,
        );
        assert!(!report.hand_state_baseline.available);
        assert_eq!(report.verdict, ConsistencyVerdict::AwaitingReview);
    }

    #[test]
    fn import_consistency_accepts_byte_identical_same_direction_authorities() {
        let source = bare_hand_sprite();
        let nodes = four_nodes(&source);
        let report = assess_direction_grid_import_consistency(
            &nodes,
            &nodes,
            &four_bindings("source"),
            &four_bindings("candidate"),
            &"a".repeat(64),
        );
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
        assert!(report.nodes.iter().all(|node| {
            node.identity_scores.composite > 0.999
                && node.added_foreground_ratio == 0.0
                && node.removed_foreground_ratio == 0.0
                && node.new_torso_linear_component_count == 0
        }));
    }

    #[test]
    fn import_consistency_blocks_a_new_diagonal_chest_strap() {
        let source = bare_hand_sprite();
        let source_nodes = four_nodes(&source);
        let mut candidate_nodes = source_nodes.clone();
        let candidate = candidate_nodes.get_mut("front_idle").unwrap();
        for offset in 0..20u32 {
            for thickness in 0..2u32 {
                candidate.put_pixel(
                    38 + offset,
                    43 + offset + thickness,
                    Rgba([48, 30, 18, 255]),
                );
            }
        }
        let report = assess_direction_grid_import_consistency(
            &source_nodes,
            &candidate_nodes,
            &four_bindings("source"),
            &four_bindings("candidate"),
            &"a".repeat(64),
        );
        let front = report
            .nodes
            .iter()
            .find(|node| node.node_id == "front_idle")
            .unwrap();
        assert_eq!(front.verdict, ConsistencyVerdict::Regenerate);
        assert!(front.new_torso_linear_component_count > 0);
        assert!(front
            .reasons
            .contains(&"direction_grid_new_torso_linear_detail".into()));
    }

    #[test]
    fn import_consistency_blocks_source_relative_scale_or_baseline_drift() {
        let source = bare_hand_sprite();
        let source_nodes = four_nodes(&source);
        let mut candidate_nodes = source_nodes.clone();
        let smaller =
            image::imageops::resize(&source, 72, 72, image::imageops::FilterType::Lanczos3);
        let mut candidate = RgbaImage::from_pixel(96, 96, Rgba([0, 0, 0, 0]));
        image::imageops::overlay(&mut candidate, &smaller, 12, 20);
        candidate_nodes.insert("front_idle".into(), candidate);
        let report = assess_direction_grid_import_consistency(
            &source_nodes,
            &candidate_nodes,
            &four_bindings("source"),
            &four_bindings("candidate"),
            &"a".repeat(64),
        );
        let front = report
            .nodes
            .iter()
            .find(|node| node.node_id == "front_idle")
            .unwrap();
        assert_eq!(front.verdict, ConsistencyVerdict::Regenerate);
        assert!(front.body_width_ratio < 0.90 || front.body_height_ratio < 0.90);
        assert!(front
            .reasons
            .contains(&"direction_grid_source_relative_geometry_changed".into()));
    }

    #[test]
    fn import_consistency_does_not_reclassify_an_existing_cape_edge_as_a_new_prop() {
        let mut source = bare_hand_sprite();
        for y in 32..82 {
            for x in 72..75 {
                source.put_pixel(x, y, Rgba([78, 94, 54, 255]));
            }
        }
        let source_nodes = four_nodes(&source);
        let report = assess_direction_grid_import_consistency(
            &source_nodes,
            &source_nodes,
            &four_bindings("source"),
            &four_bindings("candidate"),
            &"a".repeat(64),
        );
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
        assert!(report.nodes.iter().all(|node| {
            node.exterior_elongation_delta == 0.0
                && !node
                    .reasons
                    .contains(&"direction_grid_new_exterior_object".into())
        }));
    }

    #[test]
    fn import_consistency_blocks_a_genuinely_new_exterior_prop() {
        let source = bare_hand_sprite();
        let source_nodes = four_nodes(&source);
        let mut candidate_nodes = source_nodes.clone();
        let candidate = candidate_nodes.get_mut("front_idle").unwrap();
        for y in 24..88 {
            for x in 73..77 {
                candidate.put_pixel(x, y, Rgba([62, 40, 24, 255]));
            }
        }
        let report = assess_direction_grid_import_consistency(
            &source_nodes,
            &candidate_nodes,
            &four_bindings("source"),
            &four_bindings("candidate"),
            &"a".repeat(64),
        );
        let front = report
            .nodes
            .iter()
            .find(|node| node.node_id == "front_idle")
            .unwrap();
        assert_eq!(front.verdict, ConsistencyVerdict::Regenerate);
        assert!(front.reasons.iter().any(|reason| {
            matches!(
                reason.as_str(),
                "direction_grid_new_exterior_object" | "direction_grid_permanent_silhouette_change"
            )
        }));
    }

    #[test]
    fn import_consistency_does_not_mark_swapped_direction_authorities_game_ready() {
        let base = bare_hand_sprite();
        let mut source_nodes = four_nodes(&base);
        for (node_id, color) in [
            ("right_idle", Rgba([196, 54, 42, 255])),
            ("left_idle", Rgba([36, 74, 198, 255])),
        ] {
            let image = source_nodes.get_mut(node_id).unwrap();
            for y in 26..86 {
                for x in 32..62 {
                    if image.get_pixel(x, y)[3] > 48 {
                        image.put_pixel(x, y, color);
                    }
                }
            }
        }
        let mut candidate_nodes = source_nodes.clone();
        candidate_nodes.insert(
            "right_idle".into(),
            source_nodes.get("left_idle").unwrap().clone(),
        );
        candidate_nodes.insert(
            "left_idle".into(),
            source_nodes.get("right_idle").unwrap().clone(),
        );
        let report = assess_direction_grid_import_consistency(
            &source_nodes,
            &candidate_nodes,
            &four_bindings("source"),
            &four_bindings("candidate"),
            &"a".repeat(64),
        );
        assert_ne!(report.verdict, ConsistencyVerdict::GameReady);
        assert!(report.nodes.iter().any(|node| {
            matches!(node.node_id.as_str(), "right_idle" | "left_idle")
                && node.verdict != ConsistencyVerdict::GameReady
        }));
    }

    fn cape_sprite(with_gold_hem: bool) -> RgbaImage {
        let mut image = RgbaImage::from_pixel(96, 96, Rgba([0, 0, 0, 0]));
        for y in 18..82 {
            for x in 31..65 {
                image.put_pixel(x, y, Rgba([88, 104, 48, 255]));
            }
        }
        if with_gold_hem {
            for y in 67..72 {
                for x in 31..65 {
                    image.put_pixel(x, y, Rgba([196, 132, 42, 255]));
                }
            }
        }
        image
    }

    #[test]
    fn cape_hem_contract_blocks_a_front_without_the_shared_gold_lower_hem() {
        let mut images = BTreeMap::new();
        for node_id in ["front_idle", "back_idle", "right_idle", "left_idle"] {
            images.insert(node_id.into(), cape_sprite(node_id != "front_idle"));
        }
        let report = assess_cape_hem_consistency(
            CapeHemContractV1::ContinuousGoldLowerHem,
            &images,
            &four_bindings("candidate"),
        );
        assert_eq!(report.verdict, ConsistencyVerdict::Regenerate);
        let front = report
            .nodes
            .iter()
            .find(|node| node.node_id == "front_idle")
            .unwrap();
        assert_eq!(front.verdict, ConsistencyVerdict::Regenerate);
        assert!(front.reasons.contains(&"cape_hem_gold_trim_missing".into()));
        assert!(report
            .nodes
            .iter()
            .filter(|node| node.node_id != "front_idle")
            .all(|node| node.verdict == ConsistencyVerdict::GameReady));
    }

    #[test]
    fn cape_hem_contract_accepts_four_consistent_gold_lower_hems() {
        let images = ["front_idle", "back_idle", "right_idle", "left_idle"]
            .into_iter()
            .map(|node_id| (node_id.into(), cape_sprite(true)))
            .collect::<BTreeMap<_, _>>();
        let report = assess_cape_hem_consistency(
            CapeHemContractV1::ContinuousGoldLowerHem,
            &images,
            &four_bindings("candidate"),
        );
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
        assert!(report.nodes.iter().all(|node| {
            node.gold_adjacent_to_cape_ratio >= node.minimum_gold_adjacent_to_cape_ratio
        }));
    }

    #[test]
    fn front_authoritative_no_skirt_contract_rejects_added_rotated_hems() {
        let mut images = BTreeMap::new();
        for node_id in ["front_idle", "back_idle", "right_idle", "left_idle"] {
            images.insert(node_id.into(), cape_sprite(node_id != "front_idle"));
        }
        let report = assess_cape_hem_consistency(
            CapeHemContractV1::FrontAuthoritativeNoSkirtHem,
            &images,
            &four_bindings("candidate"),
        );
        assert_eq!(report.verdict, ConsistencyVerdict::Regenerate);
        assert_eq!(report.nodes[0].verdict, ConsistencyVerdict::GameReady);
        assert!(report.nodes[1..].iter().all(|node| {
            node.verdict == ConsistencyVerdict::Regenerate
                && node.maximum_gold_adjacent_to_cape_ratio == Some(0.004)
                && node
                    .reasons
                    .contains(&"cape_skirt_like_gold_lower_hem_present".into())
        }));
    }

    #[test]
    fn front_authoritative_no_skirt_contract_accepts_four_plain_lower_hems() {
        let images = ["front_idle", "back_idle", "right_idle", "left_idle"]
            .into_iter()
            .map(|node_id| (node_id.into(), cape_sprite(false)))
            .collect::<BTreeMap<_, _>>();
        let report = assess_cape_hem_consistency(
            CapeHemContractV1::FrontAuthoritativeNoSkirtHem,
            &images,
            &four_bindings("candidate"),
        );
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
        assert!(report.nodes.iter().all(|node| {
            node.minimum_gold_adjacent_to_cape_ratio == 0.0
                && node.maximum_gold_adjacent_to_cape_ratio == Some(0.004)
                && node.gold_adjacent_to_cape_ratio <= 0.004
        }));
    }

    fn action_report(profile: &str) -> GridActionReportV1 {
        GridActionReportV1 {
            schema_version: "1".into(),
            profile: profile.into(),
            animation: "walk_down".into(),
            attempt: 1,
            pose_guidance: GridPoseGuidanceV1::Enabled,
            phase_profile: (profile == GRID_ACTION_REPORT_PROFILE)
                .then(|| GRID_ACTION_PHASE_PROFILE.into()),
            phase_evidence: (profile == GRID_ACTION_REPORT_PROFILE).then(|| {
                GridActionPhaseEvidenceV1 {
                    profile: GRID_ACTION_PHASE_EVIDENCE_PROFILE.into(),
                    verified_cyclic_order: true,
                    distinct_pose_count: 4,
                    phase_order_score: 0.8,
                    opposing_contact_change_ratio: 0.2,
                }
            }),
            generation_method: (profile == GRID_ACTION_REPORT_PROFILE)
                .then(|| "fresh_action_grid".into()),
            repair_reasons: Vec::new(),
            repaired_frames: Vec::new(),
            input_sheet_sha256: None,
            sheet_path: "sheet.png".into(),
            sheet_sha256: "a".repeat(64),
            cells: (0..4)
                .map(|frame_index| GridActionCellV1 {
                    frame_index,
                    path: format!("frame-{frame_index}.png").into(),
                    sha256: format!("{frame_index}").repeat(64),
                    reused: false,
                    provider_request_occurred: true,
                })
                .collect(),
        }
    }

    #[test]
    fn action_report_keeps_legacy_phase_unspecified_but_validates_v11_evidence() {
        let legacy = action_report("grid-action-report@1.0.0");
        assert!(legacy.phase_profile.is_none());
        legacy.validate_for_reuse("walk_down").unwrap();

        let current = action_report(GRID_ACTION_REPORT_PROFILE);
        current.validate_for_reuse("walk_down").unwrap();
    }

    #[test]
    fn action_report_rejects_duplicate_indices_and_incomplete_diagnostic_provenance() {
        let mut duplicate = action_report(GRID_ACTION_REPORT_PROFILE);
        duplicate.cells[3].frame_index = 2;
        assert_eq!(
            duplicate.validate_for_reuse("walk_down").unwrap_err(),
            "grid_action_report_frame_contract_invalid"
        );

        let mut diagnostic = action_report(GRID_ACTION_REPORT_PROFILE);
        diagnostic.generation_method = Some("diagnostic_edit_retry".into());
        assert_eq!(
            diagnostic.validate_for_reuse("walk_down").unwrap_err(),
            "grid_action_report_retry_provenance_missing"
        );
    }
}
