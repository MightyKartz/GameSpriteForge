use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::asset_project::ConsistencyVerdict;
use crate::character_direction::DirectionViewV1;

pub const DIRECTION_MOTION_LOCK_PROFILE: &str = "direction-motion-lock@1.0.0";
pub const DIRECTION_MOTION_APPROVAL_PROFILE: &str = "direction-motion-approval@1.0.0";
pub const DIRECTION_MOTION_APPROVAL_FILE: &str = "direction-motion-approval.json";
pub const VISUAL_FEEDBACK_PROFILE: &str = "visual-generation-feedback@1.0.0";

/// Optional motion guidance. This selects prompts and diagnostics; it never
/// decides whether a subject class is allowed to be generated.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CharacterMotionProfileV1 {
    #[default]
    #[serde(rename = "freeform_motion@1.0.0")]
    Freeform,
    #[serde(rename = "biped_walk@1.0.0")]
    BipedWalk,
    #[serde(rename = "quadruped_walk@1.0.0")]
    QuadrupedWalk,
    #[serde(rename = "flying_cycle@1.0.0")]
    FlyingCycle,
    #[serde(rename = "slither_cycle@1.0.0")]
    SlitherCycle,
}

impl CharacterMotionProfileV1 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Freeform => "freeform_motion@1.0.0",
            Self::BipedWalk => "biped_walk@1.0.0",
            Self::QuadrupedWalk => "quadruped_walk@1.0.0",
            Self::FlyingCycle => "flying_cycle@1.0.0",
            Self::SlitherCycle => "slither_cycle@1.0.0",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectionMotionGenerationStageV1 {
    #[default]
    ImageLocks,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectionMotionNodeRoleV1 {
    FrontIdle,
    DirectionIdle,
    DirectionWalkPose,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualGenerationFeedbackV1 {
    pub schema_version: String,
    pub profile: String,
    pub verdict: ConsistencyVerdict,
    /// Provider-neutral reasons such as structural drift or framing changes.
    /// These drive a targeted retry and never encode a required costume,
    /// anatomy, species, or number of limbs.
    #[serde(default)]
    pub reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_palette_overlap: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_scale_ratio: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_foreground_ratio: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_occupancy_similarity: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_component_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_component_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permanent_feature_missing: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_bottom_support_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_bottom_span_ratio: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_lower_centroid_drift_ratio: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_lower_covering_width_ratio: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_lower_covering_width_ratio: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_lower_covering_area_share: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_lower_covering_area_share: Option<f32>,
}

impl VisualGenerationFeedbackV1 {
    pub fn accepted() -> Self {
        Self {
            schema_version: "1".into(),
            profile: VISUAL_FEEDBACK_PROFILE.into(),
            verdict: ConsistencyVerdict::GameReady,
            reasons: Vec::new(),
            parent_palette_overlap: None,
            parent_scale_ratio: None,
            parent_foreground_ratio: None,
            parent_occupancy_similarity: None,
            parent_component_count: None,
            candidate_component_count: None,
            permanent_feature_missing: None,
            canonical_bottom_support_count: None,
            canonical_bottom_span_ratio: None,
            canonical_lower_centroid_drift_ratio: None,
            parent_lower_covering_width_ratio: None,
            candidate_lower_covering_width_ratio: None,
            parent_lower_covering_area_share: None,
            candidate_lower_covering_area_share: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectionMotionNodeV1 {
    pub node_id: String,
    pub role: DirectionMotionNodeRoleV1,
    pub direction: DirectionViewV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_sha256: Option<String>,
    pub canonical_node_id: String,
    pub canonical_sha256: String,
    pub attempt: u8,
    pub provider_request_occurred: bool,
    pub path: PathBuf,
    pub sha256: String,
    pub generation_master_path: PathBuf,
    pub generation_master_sha256: String,
    pub feedback: VisualGenerationFeedbackV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectionMotionLockV1 {
    pub schema_version: String,
    pub profile: String,
    pub workflow: String,
    pub provider_id: String,
    pub profile_id: String,
    pub image_model: String,
    pub style_revision: String,
    pub motion_profile: CharacterMotionProfileV1,
    pub canonical_node_id: String,
    pub canonical_sha256: String,
    pub nodes: Vec<DirectionMotionNodeV1>,
}

/// An explicit, immutable bridge between the paid image-lock stage and the
/// separately authorized video stage. A Complete request cannot rely on a
/// mutable review flag: it must prove that the exact lock and every node hash
/// were accepted in the source Job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectionMotionApprovalV1 {
    pub schema_version: String,
    pub profile: String,
    pub source_job_id: String,
    pub lock_sha256: String,
    pub node_sha256: BTreeMap<String, String>,
    pub accepted: bool,
    pub reason: String,
    pub reviewed_at: DateTime<Utc>,
}

impl DirectionMotionApprovalV1 {
    pub fn matches_lock(&self, lock: &DirectionMotionLockV1, lock_sha256: &str) -> bool {
        self.accepted
            && self.profile == DIRECTION_MOTION_APPROVAL_PROFILE
            && self.lock_sha256 == lock_sha256
            && self.node_sha256
                == lock
                    .nodes
                    .iter()
                    .map(|node| (node.node_id.clone(), node.sha256.clone()))
                    .collect()
    }
}

impl DirectionMotionLockV1 {
    pub fn node(&self, node_id: &str) -> Option<&DirectionMotionNodeV1> {
        self.nodes.iter().find(|node| node.node_id == node_id)
    }

    pub fn idle_node_id(direction: DirectionViewV1) -> &'static str {
        match direction {
            DirectionViewV1::Front => "front_idle",
            DirectionViewV1::Rear => "back_idle",
            DirectionViewV1::Right => "right_idle",
            DirectionViewV1::Left => "left_idle",
        }
    }

    pub fn walk_node_id(animation: &str) -> Option<&'static str> {
        match animation {
            "walk_down" => Some("front_walk"),
            "walk_up" => Some("back_walk"),
            "walk_right" => Some("right_walk"),
            "walk_left" => Some("left_walk"),
            _ => None,
        }
    }

    pub fn idle_animation_node_id(animation: &str) -> Option<&'static str> {
        match animation {
            "idle" | "idle_down" => Some("front_idle"),
            "idle_up" => Some("back_idle"),
            "idle_right" => Some("right_idle"),
            "idle_left" => Some("left_idle"),
            _ => None,
        }
    }

    pub fn image_lock_node_for_animation(animation: &str) -> Option<&'static str> {
        Self::idle_animation_node_id(animation).or_else(|| Self::walk_node_id(animation))
    }

    pub fn image_retry_closure(animations: &[String]) -> Result<Vec<&'static str>, String> {
        const ORDER: [&str; 8] = [
            "front_idle",
            "back_idle",
            "right_idle",
            "left_idle",
            "front_walk",
            "back_walk",
            "right_walk",
            "left_walk",
        ];
        let mut selected = BTreeSet::new();
        for animation in animations {
            let node = Self::image_lock_node_for_animation(animation)
                .ok_or_else(|| format!("direction_motion_animation_unknown: {animation}"))?;
            match node {
                "front_idle" => selected.extend(ORDER),
                "back_idle" => selected.extend(["back_idle", "back_walk"]),
                "right_idle" => selected.extend(["right_idle", "right_walk"]),
                "left_idle" => selected.extend(["left_idle", "left_walk"]),
                "front_walk" | "back_walk" | "right_walk" | "left_walk" => {
                    selected.insert(node);
                }
                _ => unreachable!(),
            }
        }
        if selected.is_empty() {
            return Err("direction_motion_image_retry_scope_empty".into());
        }
        Ok(ORDER
            .into_iter()
            .filter(|node| selected.contains(node))
            .collect())
    }

    pub fn video_input(&self, animation: &str) -> Result<&DirectionMotionNodeV1, String> {
        let node_id = Self::walk_node_id(animation)
            .ok_or_else(|| format!("direction_motion_animation_unknown: {animation}"))?;
        let node = self
            .node(node_id)
            .ok_or_else(|| format!("direction_motion_node_missing: {node_id}"))?;
        let expected_direction = match animation {
            "walk_down" => DirectionViewV1::Front,
            "walk_up" => DirectionViewV1::Rear,
            "walk_right" => DirectionViewV1::Right,
            "walk_left" => DirectionViewV1::Left,
            _ => unreachable!(),
        };
        let expected_parent = Self::idle_node_id(expected_direction);
        if node.role != DirectionMotionNodeRoleV1::DirectionWalkPose
            || node.direction != expected_direction
            || node.animation.as_deref() != Some(animation)
            || node.parent_node_id.as_deref() != Some(expected_parent)
        {
            return Err(format!(
                "direction_motion_lineage_mismatch: {node_id} is not the typed input for {animation}"
            ));
        }
        let parent = self
            .node(expected_parent)
            .ok_or_else(|| format!("direction_motion_parent_missing: {expected_parent}"))?;
        if node.parent_sha256.as_deref() != Some(parent.sha256.as_str())
            || node.canonical_node_id != self.canonical_node_id
            || node.canonical_sha256 != self.canonical_sha256
        {
            return Err(format!(
                "direction_motion_lineage_hash_mismatch: {node_id} no longer matches its immutable parents"
            ));
        }
        Ok(node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(
        node_id: &str,
        role: DirectionMotionNodeRoleV1,
        direction: DirectionViewV1,
        animation: Option<&str>,
        parent: Option<(&str, &str)>,
        sha256: &str,
    ) -> DirectionMotionNodeV1 {
        DirectionMotionNodeV1 {
            node_id: node_id.into(),
            role,
            direction,
            animation: animation.map(str::to_string),
            parent_node_id: parent.map(|(id, _)| id.to_string()),
            parent_sha256: parent.map(|(_, sha)| sha.to_string()),
            canonical_node_id: "front_idle".into(),
            canonical_sha256: "front-sha".into(),
            attempt: 1,
            provider_request_occurred: true,
            path: PathBuf::from(format!("{node_id}.png")),
            sha256: sha256.into(),
            generation_master_path: PathBuf::from(format!("masters/{node_id}.png")),
            generation_master_sha256: format!("master-{sha256}"),
            feedback: VisualGenerationFeedbackV1::accepted(),
        }
    }

    fn lock() -> DirectionMotionLockV1 {
        DirectionMotionLockV1 {
            schema_version: "1".into(),
            profile: DIRECTION_MOTION_LOCK_PROFILE.into(),
            workflow: "topdown-direction-motion@8.0.0".into(),
            provider_id: "fixture".into(),
            profile_id: "default".into(),
            image_model: "fixture-image".into(),
            style_revision: "style".into(),
            motion_profile: CharacterMotionProfileV1::Freeform,
            canonical_node_id: "front_idle".into(),
            canonical_sha256: "front-sha".into(),
            nodes: vec![
                node(
                    "front_idle",
                    DirectionMotionNodeRoleV1::FrontIdle,
                    DirectionViewV1::Front,
                    None,
                    None,
                    "front-sha",
                ),
                node(
                    "back_idle",
                    DirectionMotionNodeRoleV1::DirectionIdle,
                    DirectionViewV1::Rear,
                    None,
                    Some(("front_idle", "front-sha")),
                    "back-idle-sha",
                ),
                node(
                    "back_walk",
                    DirectionMotionNodeRoleV1::DirectionWalkPose,
                    DirectionViewV1::Rear,
                    Some("walk_up"),
                    Some(("back_idle", "back-idle-sha")),
                    "back-walk-sha",
                ),
            ],
        }
    }

    #[test]
    fn typed_video_input_accepts_only_the_exact_direction_walk_lineage() {
        assert_eq!(lock().video_input("walk_up").unwrap().node_id, "back_walk");
    }

    #[test]
    fn typed_video_input_rejects_a_pose_bound_to_the_wrong_parent_hash() {
        let mut lock = lock();
        lock.node("back_walk").unwrap();
        lock.nodes
            .iter_mut()
            .find(|node| node.node_id == "back_walk")
            .unwrap()
            .parent_sha256 = Some("changed".into());
        assert!(lock
            .video_input("walk_up")
            .unwrap_err()
            .contains("lineage_hash_mismatch"));
    }

    #[test]
    fn motion_profiles_are_guidance_not_subject_classes() {
        assert_eq!(
            CharacterMotionProfileV1::Freeform.as_str(),
            "freeform_motion@1.0.0"
        );
        assert_eq!(
            CharacterMotionProfileV1::QuadrupedWalk.as_str(),
            "quadruped_walk@1.0.0"
        );
    }

    #[test]
    fn image_retry_closure_invalidates_only_dependent_nodes() {
        assert_eq!(
            DirectionMotionLockV1::image_retry_closure(&["idle_up".into()]).unwrap(),
            ["back_idle", "back_walk"]
        );
        assert_eq!(
            DirectionMotionLockV1::image_retry_closure(&["walk_right".into()]).unwrap(),
            ["right_walk"]
        );
        assert_eq!(
            DirectionMotionLockV1::image_retry_closure(&["idle_down".into()]).unwrap(),
            [
                "front_idle",
                "back_idle",
                "right_idle",
                "left_idle",
                "front_walk",
                "back_walk",
                "right_walk",
                "left_walk",
            ]
        );
    }
}
