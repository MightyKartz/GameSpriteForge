use serde::{Deserialize, Serialize};

use crate::asset_project::{CharacterEquipmentKindV1, ConsistencyVerdict};
use crate::character_direction::{DirectionAnchorAssessmentV1, DirectionViewV1};

pub const DIRECTION_POSE_LOCK_PROFILE: &str = "direction-pose-lock@1.0.0";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectionPoseLockEntryV1 {
    pub animation: String,
    pub direction: DirectionViewV1,
    pub attempt: u8,
    pub path: std::path::PathBuf,
    pub sha256: String,
    pub generation_master_path: std::path::PathBuf,
    pub generation_master_sha256: String,
    pub direction_anchor_sha256: String,
    pub assessment: DirectionAnchorAssessmentV1,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectionPoseLockV1 {
    pub schema_version: String,
    pub profile: String,
    pub workflow: String,
    pub provider_id: String,
    pub profile_id: String,
    pub model: String,
    pub style_revision: String,
    pub subject_id: String,
    pub subject_revision: String,
    pub subject_sha256: String,
    pub equipment_kind: CharacterEquipmentKindV1,
    pub entries: Vec<DirectionPoseLockEntryV1>,
}

impl DirectionPoseLockV1 {
    pub fn entry(&self, animation: &str) -> Option<&DirectionPoseLockEntryV1> {
        self.entries
            .iter()
            .find(|entry| entry.animation == animation)
    }
}
