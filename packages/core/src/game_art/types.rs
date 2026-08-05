use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// On-disk discriminator stored in the manifest `kind` field.
pub const GAME_ART_MANIFEST_KIND: &str = "game_art_manifest";
/// Only schema version accepted by this stage.
pub const GAME_ART_MANIFEST_SCHEMA_VERSION: &str = "1";

/// Structured manifest error with a stable snake_case code, so CLI and
/// automation consumers can branch on `code()` instead of message text.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GameArtError {
    #[error("invalid manifest JSON: {0}")]
    InvalidJson(String),
    #[error("unknown field: {0}")]
    UnknownField(String),
    #[error("invalid kind: {0}")]
    InvalidKind(String),
    #[error("unsupported schema version: {0}")]
    UnsupportedSchemaVersion(String),
    #[error("invalid manifest: {0}")]
    InvalidManifest(String),
    #[error("invalid id: {0}")]
    InvalidId(String),
    #[error("duplicate asset id: {0}")]
    DuplicateAssetId(String),
    #[error("unknown dependency: {0}")]
    UnknownDependency(String),
    #[error("invalid lock reference: {0}")]
    InvalidLockRef(String),
    #[error("self dependency: {0}")]
    SelfDependency(String),
    #[error("dependency cycle: {0}")]
    DependencyCycle(String),
    #[error("required asset depends on optional asset: {0}")]
    RequiredDependsOnOptional(String),
    #[error("absolute spec path not allowed: {0}")]
    AbsolutePath(String),
    #[error("spec URL not allowed: {0}")]
    UrlNotAllowed(String),
    #[error("spec path traversal not allowed: {0}")]
    PathTraversal(String),
    #[error("spec symlink escapes manifest directory: {0}")]
    SymlinkEscape(String),
    #[error("spec not found: {0}")]
    SpecNotFound(String),
    #[error("spec is not a regular file: {0}")]
    SpecNotFile(String),
    #[error("unknown lock: {0}")]
    UnknownLock(String),
    #[error("lock revision mismatch: {0}")]
    LockRevisionMismatch(String),
    #[error("io error: {0}")]
    Io(String),
}

impl GameArtError {
    /// Stable machine-readable code for JSON error reporting.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidJson(_) => "invalid_json",
            Self::UnknownField(_) => "unknown_field",
            Self::InvalidKind(_) => "invalid_kind",
            Self::UnsupportedSchemaVersion(_) => "unsupported_schema_version",
            Self::InvalidManifest(_) => "invalid_manifest",
            Self::InvalidId(_) => "invalid_id",
            Self::DuplicateAssetId(_) => "duplicate_asset_id",
            Self::UnknownDependency(_) => "unknown_dependency",
            Self::InvalidLockRef(_) => "invalid_lock_ref",
            Self::SelfDependency(_) => "self_dependency",
            Self::DependencyCycle(_) => "dependency_cycle",
            Self::RequiredDependsOnOptional(_) => "required_depends_on_optional",
            Self::AbsolutePath(_) => "absolute_path",
            Self::UrlNotAllowed(_) => "url_not_allowed",
            Self::PathTraversal(_) => "path_traversal",
            Self::SymlinkEscape(_) => "symlink_escape",
            Self::SpecNotFound(_) => "spec_not_found",
            Self::SpecNotFile(_) => "spec_not_file",
            Self::UnknownLock(_) => "unknown_lock",
            Self::LockRevisionMismatch(_) => "lock_revision_mismatch",
            Self::Io(_) => "io_error",
        }
    }
}

/// Filesystem-safe identifier charset shared by project ids, asset ids and
/// lock ids: `[A-Za-z0-9_-]+`.
pub fn is_valid_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

/// Stage 2 closes the asset kind set to these three kinds. Later stages
/// extend this enum (environment, collection, ui, effect, ...) together with
/// `schemas/game-art-manifest.schema.json`; anything else is rejected at
/// parse time with `invalid_kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Character,
    IconSet,
    PropSet,
}

impl AssetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Character => "character",
            Self::IconSet => "icon_set",
            Self::PropSet => "prop_set",
        }
    }
}

impl fmt::Display for AssetKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for AssetKind {
    type Err = GameArtError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "character" => Ok(Self::Character),
            "icon_set" => Ok(Self::IconSet),
            "prop_set" => Ok(Self::PropSet),
            other => Err(GameArtError::InvalidKind(format!(
                "unsupported asset kind \"{other}\" (stage 2 supports character, icon_set, prop_set)"
            ))),
        }
    }
}

/// Stage 2 accepts style and subject locks; environment, collection, ui and
/// effect locks arrive with later stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockKind {
    Style,
    Subject,
}

impl LockKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Style => "style",
            Self::Subject => "subject",
        }
    }
}

impl fmt::Display for LockKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Immutable lock reference allowed in `dependsOn` next to declared asset
/// ids: `"<lockKind>:<id>@<revision>"`. Parsing is purely syntactic —
/// checking that the referenced lock actually exists in a project belongs to
/// the plan/diff layer, not to this manifest layer.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LockRef {
    pub kind: LockKind,
    pub id: String,
    pub revision: String,
}

impl LockRef {
    pub fn parse(raw: &str) -> Result<Self, GameArtError> {
        let invalid = |reason: &str| {
            GameArtError::InvalidLockRef(format!(
                "\"{raw}\" ({reason}); expected \"<lockKind>:<id>@<revision>\""
            ))
        };
        let (kind_raw, rest) = raw
            .split_once(':')
            .ok_or_else(|| invalid("missing lock kind separator ':'"))?;
        let kind = match kind_raw {
            "style" => LockKind::Style,
            "subject" => LockKind::Subject,
            other => {
                return Err(invalid(&format!(
                    "unsupported lock kind \"{other}\" (stage 2 supports style, subject)"
                )));
            }
        };
        let (id, revision) = rest
            .split_once('@')
            .ok_or_else(|| invalid("missing revision separator '@'"))?;
        if !is_valid_id(id) {
            return Err(invalid("lock id must match [A-Za-z0-9_-]+"));
        }
        if !is_valid_lock_revision(revision) {
            return Err(invalid("revision must be non-empty [A-Za-z0-9_.-]+"));
        }
        Ok(Self {
            kind,
            id: id.to_string(),
            revision: revision.to_string(),
        })
    }
}

impl fmt::Display for LockRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}@{}", self.kind, self.id, self.revision)
    }
}

/// Lock revisions are immutable identifiers or digests: `[A-Za-z0-9_.-]+`.
pub(crate) fn is_valid_lock_revision(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
        })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameArtProviderV1 {
    pub id: String,
    pub profile_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameArtDefaultsV1 {
    pub output_directory: PathBuf,
    pub godot_root: PathBuf,
    pub license: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameArtAssetV1 {
    pub id: String,
    pub kind: AssetKind,
    /// Spec path resolved relative to the manifest file's directory.
    pub spec: PathBuf,
    #[serde(default = "default_required")]
    pub required: bool,
    /// Declared asset ids and/or immutable lock references.
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_required() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameArtManifestV1 {
    pub schema_version: String,
    pub kind: String,
    pub project_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_revision: Option<String>,
    pub provider: GameArtProviderV1,
    pub defaults: GameArtDefaultsV1,
    pub assets: Vec<GameArtAssetV1>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_kind_roundtrips_snake_case() {
        for (kind, raw) in [
            (AssetKind::Character, "character"),
            (AssetKind::IconSet, "icon_set"),
            (AssetKind::PropSet, "prop_set"),
        ] {
            assert_eq!(kind.as_str(), raw);
            assert_eq!(kind.to_string(), raw);
            assert_eq!(raw.parse::<AssetKind>().unwrap(), kind);
            assert_eq!(serde_json::to_string(&kind).unwrap(), format!("\"{raw}\""));
        }
    }

    #[test]
    fn asset_kind_rejects_unknown_values() {
        for raw in ["environment", "ui", "Character", "", "icon-set"] {
            let error = raw.parse::<AssetKind>().unwrap_err();
            assert_eq!(error.code(), "invalid_kind");
        }
        assert!(serde_json::from_str::<AssetKind>("\"environment\"").is_err());
    }

    #[test]
    fn is_valid_id_enforces_filesystem_safe_charset() {
        for valid in ["forest-rpg", "hero_knight", "A0_-"] {
            assert!(is_valid_id(valid), "{valid} should be valid");
        }
        for invalid in ["", "forest rpg", "hero/main", "a:b", "a@b", "é", ".hidden"] {
            assert!(!is_valid_id(invalid), "{invalid} should be invalid");
        }
    }

    #[test]
    fn lock_ref_parses_valid_references() {
        let style = LockRef::parse("style:pixel-art@rev-1").unwrap();
        assert_eq!(style.kind, LockKind::Style);
        assert_eq!(style.id, "pixel-art");
        assert_eq!(style.revision, "rev-1");
        assert_eq!(style.to_string(), "style:pixel-art@rev-1");

        let subject = LockRef::parse("subject:forest-ranger@1.0.0_beta").unwrap();
        assert_eq!(subject.kind, LockKind::Subject);
        assert_eq!(subject.id, "forest-ranger");
        assert_eq!(subject.revision, "1.0.0_beta");
    }

    #[test]
    fn lock_ref_rejects_invalid_references() {
        for raw in [
            // Stage 2 closes the lock kind set to style and subject.
            "environment:forest@r1",
            "collection:pack@r1",
            // Missing separators.
            "style-no-colon",
            "subject:no-revision",
            ":id@rev",
            // Empty or malformed id / revision.
            "style:@rev",
            "style:id@",
            "style:bad id@rev",
            "style:id@bad rev",
            "style:id@.",
            "style:id@..",
            // Extra separators leak into id/revision and fail the charset.
            "style:a:b@rev",
            "style:id@rev@extra",
        ] {
            let error = LockRef::parse(raw).unwrap_err();
            assert_eq!(error.code(), "invalid_lock_ref", "input: {raw}");
        }
    }
}
