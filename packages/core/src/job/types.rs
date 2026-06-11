use std::fmt;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobRecord {
    pub job_id: String,
    pub source_kind: SourceKind,
    pub state: JobState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub job_dir: PathBuf,
    pub error_summary: Option<String>,
}
