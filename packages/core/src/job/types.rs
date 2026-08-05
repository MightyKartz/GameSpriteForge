use std::fmt;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::quality::{QualityReport, QualityVerdict};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceKind {
    ImportVideo,
    ImportFrames,
    ImportSpriteSheet,
    ImportGsfpack,
    FromCode,
}

impl SourceKind {
    pub fn from_code(value: &str) -> Self {
        match value {
            "import_video" => SourceKind::ImportVideo,
            "import_frames" => SourceKind::ImportFrames,
            "import_sprite_sheet" => SourceKind::ImportSpriteSheet,
            "import_gsfpack" => SourceKind::ImportGsfpack,
            _ => SourceKind::FromCode,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ImportVideo => "import_video",
            Self::ImportFrames => "import_frames",
            Self::ImportSpriteSheet => "import_sprite_sheet",
            Self::ImportGsfpack => "import_gsfpack",
            Self::FromCode => "from_code",
        }
    }
}

impl fmt::Display for SourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for SourceKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for SourceKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SourceKindVisitor;

        impl<'de> Visitor<'de> for SourceKindVisitor {
            type Value = SourceKind;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a source kind string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(match value {
                    "import_video" => SourceKind::ImportVideo,
                    "import_frames" => SourceKind::ImportFrames,
                    "import_sprite_sheet" => SourceKind::ImportSpriteSheet,
                    "import_gsfpack" => SourceKind::ImportGsfpack,
                    "from_code" => SourceKind::FromCode,
                    _ => SourceKind::FromCode,
                })
            }
        }

        deserializer.deserialize_str(SourceKindVisitor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Created,
    SourceReady,
    PreviewReady,
    FramesExtracted,
    Processed,
    QualityChecked,
    Exported,
    Failed,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobLifecycleState {
    #[default]
    Idle,
    Queued,
    Running,
    AwaitingReview,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobOperationKind {
    #[default]
    LegacyPipeline,
    PrepareAsset,
    PrepareCharacterPack,
    GenerateCharacterPack,
    CreateStyleLock,
    CreateSubjectLock,
    GenerateStaticAssetSet,
    CreateEnvironmentLock,
    GenerateTerrainSet,
    GenerateBuildingKit,
    CompileMap,
    InstallGodot,
    BuildProject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobStepRecord {
    pub name: String,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobArtifactRecord {
    pub kind: String,
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairChange {
    pub id: String,
    pub scope: String,
    pub parameter: String,
    pub before: serde_json::Value,
    pub after: serde_json::Value,
    pub reason: String,
    pub confidence: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairAnimationQuality {
    pub name: String,
    pub report: QualityReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairQualitySnapshot {
    pub verdict: QualityVerdict,
    pub animations: Vec<RepairAnimationQuality>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairContext {
    pub source_job_id: String,
    pub attempt: u32,
    pub changes: Vec<RepairChange>,
    pub baseline: RepairQualitySnapshot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobRecord {
    pub job_id: String,
    pub source_kind: SourceKind,
    pub state: JobState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub job_dir: PathBuf,
    pub error_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_job_id: Option<String>,
    #[serde(default)]
    pub operation_kind: JobOperationKind,
    #[serde(default)]
    pub lifecycle_state: JobLifecycleState,
    #[serde(default)]
    pub progress: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair: Option<RepairContext>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<JobStepRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<JobArtifactRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(default)]
    pub recoverable: bool,
    #[serde(default)]
    pub cancellation_requested: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_pid: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub next_actions: Vec<String>,
}
