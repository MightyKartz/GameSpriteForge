//! Stage 2 project build plan: turns a [`ProjectDiffV1`] into the pure-content
//! `ProjectBuildPlanV1` returned by `forge project plan-build` (implementation
//! plan §7.2). The plan layer is offline and provider-free: provider
//! capabilities arrive as the caller-supplied [`ProviderCapabilityInput`] and
//! are only *checked*, never resolved. Plan token and expiry deliberately do
//! not exist here — they belong to the automation layer's single-use plan
//! token flow.
//!
//! Stage 2 semantics, documented per field:
//!
//! - Workflow assignment is static per asset kind: `character` builds through
//!   the stable video workflow `topdown@1.0.0`, `icon_set`/`prop_set` through
//!   `static-set@1.0.0`.
//! - Provider request estimates mirror `automation::plan::estimate_operation`
//!   for a fresh (non-retry) build: a character pack estimates 9 requests with
//!   a maximum of 13 (the non-keyframe `GenerateCharacterPack` branch); a
//!   static asset set estimates one request per spec item with a maximum of
//!   two per item (the `GenerateStaticAssetSet` branch). Item counts come from
//!   parsing each asset's spec file (`CharacterAssetSpecV1` for characters,
//!   `StaticAssetSetSpecV1` for icon/prop sets).
//! - `cacheHits`/`cacheMisses`: a `reuse` verdict is a catalog-level cache hit
//!   of kind `catalog_reuse` (the only cache kind stage 2 knows); every
//!   `build`/`rebuild` is a miss requiring provider work.
//! - `estimatedCostTicks`/`maximumCostTicks` are always `null`: the existing
//!   estimate logic counts requests only and has no cost model.
//! - `localNodeCount` counts plan nodes that never call a provider: 1 for the
//!   project diff itself plus 2 per built/rebuilt asset (pack validation and
//!   catalog registration). Reused assets add none.
//! - `unmetCapabilities` lists statically required capabilities missing from
//!   [`ProviderCapabilityInput`]: `character` needs `edit_image` +
//!   `image_to_video` (the character video path's hard requirements;
//!   `edit_video` is optional there and falls back to image-to-video), static
//!   sets need `edit_image`. Reported, not fatal — the automation layer
//!   decides whether to block execution.
//! - `canonicalSpecPath` is the only host-absolute path in the payload (kept
//!   for the executor) and is excluded from [`ProjectBuildPlanV1::plan_sha256`]
//!   so identical content at different checkouts hashes identically.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::diff::{DiffActionKindV1, ProjectDiffV1, ResolvedLockRefV1};
use super::types::{AssetKind, GameArtError};
use super::ValidatedManifest;
use crate::asset_project::{CharacterAssetSpecV1, StaticAssetSetSpecV1};

/// On-disk discriminator stored in the plan `kind` field.
pub const PROJECT_BUILD_PLAN_KIND: &str = "project_build_plan";
/// Only schema version emitted by this stage.
pub const PROJECT_BUILD_PLAN_SCHEMA_VERSION: &str = "1";
/// Stable character video workflow assigned to `character` builds.
pub const CHARACTER_VIDEO_WORKFLOW: &str = "topdown@1.0.0";
/// Stable static-set workflow assigned to `icon_set`/`prop_set` builds.
pub const STATIC_SET_WORKFLOW: &str = "static-set@1.0.0";
/// The only cache hit kind stage 2 records: a catalog entry reusable as-is.
pub const CACHE_KIND_CATALOG_REUSE: &str = "catalog_reuse";

// Mirrors `automation::plan::estimate_operation`'s non-keyframe
// `GenerateCharacterPack` branch (fresh build, `topdown@1.0.0` video path).
const CHARACTER_ESTIMATED_PROVIDER_REQUESTS: u32 = 9;
const CHARACTER_MAXIMUM_PROVIDER_REQUESTS: u32 = 13;

/// Provider facts the offline plan layer is allowed to see: the resolved
/// provider's capability strings (snake_case, matching `ProviderCapability`
/// serde names) plus any model pins the caller already resolved. The CLI layer
/// fills this from the real provider; tests fill it by hand.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderCapabilityInput {
    #[serde(default)]
    pub capabilities: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_model: Option<String>,
}

/// Provider/profile/model lock recorded into the plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectBuildPlanProviderV1 {
    pub id: String,
    pub profile_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_model: Option<String>,
}

/// Plan-level action kinds: the diff verdicts minus `orphan`, which is
/// reported through `deleteCandidates` only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanActionKindV1 {
    Reuse,
    Build,
    Rebuild,
}

impl PlanActionKindV1 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reuse => "reuse",
            Self::Build => "build",
            Self::Rebuild => "rebuild",
        }
    }
}

/// One asset in the build plan: the diff verdict plus the assigned workflow
/// and per-action provider request estimates (0 for `reuse`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectPlanActionV1 {
    pub asset_id: String,
    pub kind: AssetKind,
    pub action: PlanActionKindV1,
    /// Machine-readable reason codes from `super::diff::reasons`.
    pub reasons: Vec<String>,
    pub spec_sha256: String,
    pub spec_size_bytes: u64,
    /// Absolute canonical spec path recorded at validation time — the only
    /// host-specific path in the plan, excluded from the plan hash.
    pub canonical_spec_path: PathBuf,
    pub depends_on_assets: Vec<String>,
    pub lock_refs: Vec<ResolvedLockRefV1>,
    pub workflow: String,
    pub provider_request_estimate: u32,
    pub maximum_provider_requests: u32,
}

/// Pure-content project build plan (implementation plan §7.2). No token,
/// `createdAt` or `expiresAt` — those live in the automation layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectBuildPlanV1 {
    pub schema_version: String,
    pub kind: String,
    pub manifest_sha256: String,
    pub graph_sha256: String,
    pub provider: ProjectBuildPlanProviderV1,
    #[serde(default)]
    pub style_revision: Option<String>,
    /// Build/rebuild/reuse actions in the diff's topological order.
    pub actions: Vec<ProjectPlanActionV1>,
    /// Orphaned catalog asset ids, sorted. Report only — never deleted.
    pub delete_candidates: Vec<String>,
    pub cache_hits: u32,
    pub cache_misses: u32,
    /// Hit counts by cache kind; stage 2 only records `catalog_reuse`.
    pub cache_hit_kinds: BTreeMap<String, u32>,
    pub provider_request_estimate: u32,
    pub maximum_provider_requests: u32,
    /// Always `null` in stage 2: no provider cost model exists yet (the
    /// underlying estimate logic counts requests only).
    pub estimated_cost_ticks: Option<u64>,
    /// Always `null` in stage 2, see `estimatedCostTicks`.
    pub maximum_cost_ticks: Option<u64>,
    pub local_node_count: u32,
    pub unmet_capabilities: Vec<String>,
}

impl ProjectBuildPlanV1 {
    /// Canonical form for hashing: actions sorted by asset id with reasons,
    /// dependencies and lock refs sorted, `canonicalSpecPath` blanked
    /// (host-specific), and `deleteCandidates`/`unmetCapabilities` sorted.
    /// Semantically identical plans built from reordered manifests, catalogs
    /// or lock directories normalize to the same value.
    pub fn normalized_plan(&self) -> Value {
        let mut normalized = self.clone();
        normalized
            .actions
            .sort_by(|left, right| left.asset_id.cmp(&right.asset_id));
        for action in &mut normalized.actions {
            action.reasons.sort();
            action.depends_on_assets.sort();
            action.lock_refs.sort();
            action.canonical_spec_path = PathBuf::new();
        }
        normalized.delete_candidates.sort();
        normalized.unmet_capabilities.sort();
        serde_json::to_value(&normalized).expect("ProjectBuildPlanV1 serialization cannot fail")
    }

    /// SHA-256 over the canonical serialization of [`Self::normalized_plan`].
    /// Deterministic: identical manifest, spec contents, locks and provider
    /// input produce an identical plan hash, independent of checkout location.
    pub fn plan_sha256(&self) -> String {
        let bytes =
            serde_json::to_vec(&self.normalized_plan()).expect("canonical JSON cannot fail");
        format!("{:x}", Sha256::digest(bytes))
    }
}

/// Build the stage 2 project build plan from a validated manifest and its
/// diff. `_project_root` is accepted for symmetry with
/// [`super::diff::compute_project_diff`] and reserved for later stages (Godot
/// install planning); stage 2 reads specs through the validated canonical
/// absolute paths, so nothing is resolved against it.
///
/// Errors: `invalid_manifest` when the diff does not match the validated
/// manifest or a spec's kind contradicts the manifest kind, `invalid_json`
/// when a spec file does not parse as its kind's spec struct, `io_error` when
/// a spec file cannot be read.
pub fn compute_build_plan(
    _project_root: &Path,
    validated: &ValidatedManifest,
    diff: &ProjectDiffV1,
    provider_caps: &ProviderCapabilityInput,
) -> Result<ProjectBuildPlanV1, GameArtError> {
    let manifest = &validated.manifest;
    if diff.manifest_sha256 != manifest.manifest_sha256()
        || diff.graph_sha256 != manifest.graph_sha256()
    {
        return Err(GameArtError::InvalidManifest(
            "diff was computed for a different manifest than the validated one".into(),
        ));
    }

    let mut actions = Vec::new();
    let mut cache_hits = 0u32;
    let mut cache_misses = 0u32;
    let mut provider_request_estimate = 0u32;
    let mut maximum_provider_requests = 0u32;
    let mut required_capabilities: BTreeSet<&'static str> = BTreeSet::new();

    for diff_action in &diff.actions {
        if diff_action.action == DiffActionKindV1::Orphan {
            continue;
        }
        let asset = manifest
            .assets
            .iter()
            .find(|asset| asset.id == diff_action.asset_id)
            .ok_or_else(|| {
                GameArtError::InvalidManifest(format!(
                    "diff action \"{}\" is not a declared manifest asset",
                    diff_action.asset_id
                ))
            })?;
        let spec = validated.asset(&asset.id).ok_or_else(|| {
            GameArtError::InvalidManifest(format!(
                "validated manifest has no resolved spec for asset \"{}\"",
                asset.id
            ))
        })?;
        let action = match diff_action.action {
            DiffActionKindV1::Reuse => PlanActionKindV1::Reuse,
            DiffActionKindV1::Build => PlanActionKindV1::Build,
            DiffActionKindV1::Rebuild => PlanActionKindV1::Rebuild,
            DiffActionKindV1::Orphan => unreachable!("orphans filtered above"),
        };
        let (estimate, maximum) = if action == PlanActionKindV1::Reuse {
            cache_hits += 1;
            (0, 0)
        } else {
            cache_misses += 1;
            required_capabilities.extend(required_capabilities_for(asset.kind));
            estimate_for_asset(asset.kind, &spec.canonical_spec_path)?
        };
        provider_request_estimate += estimate;
        maximum_provider_requests += maximum;
        actions.push(ProjectPlanActionV1 {
            asset_id: asset.id.clone(),
            kind: asset.kind,
            action,
            reasons: diff_action.reasons.clone(),
            spec_sha256: spec.spec_sha256.clone(),
            spec_size_bytes: spec.spec_size_bytes,
            canonical_spec_path: spec.canonical_spec_path.clone(),
            depends_on_assets: diff_action.depends_on_assets.clone(),
            lock_refs: diff_action.lock_refs.clone(),
            workflow: workflow_for(asset.kind).to_string(),
            provider_request_estimate: estimate,
            maximum_provider_requests: maximum,
        });
    }

    let unmet_capabilities = required_capabilities
        .iter()
        .filter(|capability| !provider_caps.capabilities.contains(**capability))
        .map(|capability| (*capability).to_string())
        .collect();
    let mut cache_hit_kinds = BTreeMap::new();
    if cache_hits > 0 {
        cache_hit_kinds.insert(CACHE_KIND_CATALOG_REUSE.to_string(), cache_hits);
    }

    Ok(ProjectBuildPlanV1 {
        schema_version: PROJECT_BUILD_PLAN_SCHEMA_VERSION.to_string(),
        kind: PROJECT_BUILD_PLAN_KIND.to_string(),
        manifest_sha256: diff.manifest_sha256.clone(),
        graph_sha256: diff.graph_sha256.clone(),
        provider: ProjectBuildPlanProviderV1 {
            id: manifest.provider.id.clone(),
            profile_id: manifest.provider.profile_id.clone(),
            image_model: provider_caps.image_model.clone(),
            video_model: provider_caps.video_model.clone(),
        },
        style_revision: diff.style_revision.clone(),
        actions,
        delete_candidates: diff.delete_candidates.clone(),
        cache_hits,
        cache_misses,
        cache_hit_kinds,
        provider_request_estimate,
        maximum_provider_requests,
        estimated_cost_ticks: None,
        maximum_cost_ticks: None,
        // The diff node plus pack validation and catalog registration per
        // built/rebuilt asset; reused assets add no local work.
        local_node_count: 1 + 2 * cache_misses,
        unmet_capabilities,
    })
}

/// Workflow assigned per asset kind for stage 2 stable builds.
fn workflow_for(kind: AssetKind) -> &'static str {
    match kind {
        AssetKind::Character => CHARACTER_VIDEO_WORKFLOW,
        AssetKind::IconSet | AssetKind::PropSet => STATIC_SET_WORKFLOW,
    }
}

/// Statically required provider capabilities per asset kind, mirroring the
/// runner's hard gates: the character video path requires
/// `ProviderCapability::EditImage` + `ImageToVideo` (its `EditVideo` use is an
/// optional repair path with an image-to-video fallback), static asset sets
/// require `ProviderCapability::EditImage`.
fn required_capabilities_for(kind: AssetKind) -> &'static [&'static str] {
    match kind {
        AssetKind::Character => &["edit_image", "image_to_video"],
        AssetKind::IconSet | AssetKind::PropSet => &["edit_image"],
    }
}

/// Per-action provider request estimate for one built/rebuilt asset, mirroring
/// `automation::plan::estimate_operation` fresh-build branches: characters use
/// the fixed video-path numbers; static sets scale with the spec's item count
/// (estimate = items, maximum = 2 * items). The spec file is parsed with its
/// kind's `deny_unknown_fields` struct, which also proves the spec matches the
/// manifest-declared kind.
fn estimate_for_asset(
    kind: AssetKind,
    canonical_spec_path: &Path,
) -> Result<(u32, u32), GameArtError> {
    let bytes = fs::read(canonical_spec_path).map_err(|error| {
        GameArtError::Io(format!(
            "cannot read spec {}: {error}",
            canonical_spec_path.display()
        ))
    })?;
    match kind {
        AssetKind::Character => {
            let spec: CharacterAssetSpecV1 = serde_json::from_slice(&bytes).map_err(|error| {
                GameArtError::InvalidJson(format!(
                    "character spec {}: {error}",
                    canonical_spec_path.display()
                ))
            })?;
            if spec.kind != AssetKind::Character.as_str() {
                return Err(GameArtError::InvalidManifest(format!(
                    "character spec {} declares kind \"{}\"",
                    canonical_spec_path.display(),
                    spec.kind
                )));
            }
            Ok((
                CHARACTER_ESTIMATED_PROVIDER_REQUESTS,
                CHARACTER_MAXIMUM_PROVIDER_REQUESTS,
            ))
        }
        AssetKind::IconSet | AssetKind::PropSet => {
            let spec: StaticAssetSetSpecV1 = serde_json::from_slice(&bytes).map_err(|error| {
                GameArtError::InvalidJson(format!(
                    "static asset set spec {}: {error}",
                    canonical_spec_path.display()
                ))
            })?;
            if spec.kind.as_str() != kind.as_str() {
                return Err(GameArtError::InvalidManifest(format!(
                    "spec {} declares kind \"{}\", manifest declares \"{kind}\"",
                    canonical_spec_path.display(),
                    spec.kind.as_str()
                )));
            }
            let items = spec.items.len() as u32;
            Ok((items, items.saturating_mul(2)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_project::{ForgeProjectV1, ProviderSelection, FORGE_PROJECT_FILE};
    use crate::catalog::{
        register_catalog_asset_v2, CatalogLockRevisionsV1, CatalogProviderRefV1, CatalogStyleRefV1,
        CatalogSubjectRefV1, ProjectCatalogEntryV2,
    };
    use crate::game_art::diff::hash_pack;
    use crate::game_art::{compute_project_diff, reasons, GameArtManifestV1, ProjectDiffV1};
    use chrono::{DateTime, Utc};
    use serde_json::json;

    const CHARACTER_SPEC: &str = r#"{
        "schemaVersion": "1",
        "kind": "character",
        "id": "hero",
        "name": "Hero",
        "prompt": "a brave knight",
        "license": "private"
    }"#;

    fn static_spec(id: &str, kind: &str, item_prompts: &[&str]) -> String {
        let items: Vec<_> = item_prompts
            .iter()
            .enumerate()
            .map(|(index, prompt)| {
                json!({
                    "id": format!("item-{index}"),
                    "name": format!("Item {index}"),
                    "prompt": prompt
                })
            })
            .collect();
        json!({
            "schemaVersion": "1",
            "kind": kind,
            "id": id,
            "name": format!("Set {id}"),
            "items": items,
            "license": "private"
        })
        .to_string()
    }

    fn write_project(root: &Path) {
        let project = ForgeProjectV1 {
            schema_version: "1".into(),
            project_id: "forest-rpg".into(),
            name: "Forest RPG".into(),
            provider: ProviderSelection {
                id: "xai".into(),
                profile_id: "default".into(),
            },
            output_dir: PathBuf::from("build"),
            current_style_revision: Some("style-rev-1".into()),
            current_environment_revision: None,
        };
        fs::write(
            root.join(FORGE_PROJECT_FILE),
            serde_json::to_vec_pretty(&project).unwrap(),
        )
        .unwrap();
    }

    fn write_style_lock(root: &Path) {
        let directory = root.join(".forge/styles/style-rev-1");
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("style-lock.json"),
            json!({
                "schemaVersion": "1",
                "revision": "style-rev-1",
                "providerId": "xai",
                "profileId": "default"
            })
            .to_string(),
        )
        .unwrap();
    }

    fn write_subject_lock(root: &Path) {
        let directory = root.join(".forge/subjects/hero/subject-rev-1");
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("subject-lock.json"),
            json!({
                "schemaVersion": "1",
                "id": "hero",
                "revision": "subject-rev-1",
                "providerId": "xai",
                "profileId": "default"
            })
            .to_string(),
        )
        .unwrap();
    }

    fn fixed_time() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    /// Faithful catalog entry for `asset_id`: spec hash and pack hash match
    /// the bytes currently on disk, so the diff verdict is `reuse`.
    fn register_faithful_entry(
        root: &Path,
        validated: &ValidatedManifest,
        asset_id: &str,
        kind: &str,
        workflow: &str,
        with_subject: bool,
    ) {
        let pack_dir = root.join("packs").join(asset_id);
        fs::create_dir_all(&pack_dir).unwrap();
        fs::write(pack_dir.join("pack.json"), b"{}").unwrap();
        let entry = ProjectCatalogEntryV2 {
            asset_id: asset_id.into(),
            name: format!("Asset {asset_id}"),
            kind: kind.into(),
            pack_path: PathBuf::from(format!("packs/{asset_id}")),
            pack_sha256: hash_pack(&pack_dir, true).unwrap(),
            source_job_id: "job-1".into(),
            parent_job_id: None,
            style: Some(CatalogStyleRefV1 {
                revision: "style-rev-1".into(),
            }),
            subject: with_subject.then(|| CatalogSubjectRefV1 {
                id: asset_id.into(),
                revision: "subject-rev-1".into(),
            }),
            workflow: workflow.into(),
            provider: Some(CatalogProviderRefV1 {
                provider_id: "xai".into(),
                profile_id: "default".into(),
                model: Some("grok-image".into()),
            }),
            installed: None,
            created_at: fixed_time(),
            spec_path: Some(PathBuf::from(format!("specs/{asset_id}.json"))),
            spec_sha256: Some(validated.asset(asset_id).unwrap().spec_sha256.clone()),
            dependencies: None,
            locks: Some(CatalogLockRevisionsV1 {
                style: Some("style-rev-1".into()),
                subject: with_subject.then(|| "subject-rev-1".to_string()),
                ..CatalogLockRevisionsV1::default()
            }),
            workflow_profile: None,
            workflow_version: None,
            pack_version: None,
            quality_verdict: None,
            quality_profile: None,
            game_ready: None,
            generated_at: None,
            reviewed_at: None,
            license: None,
            provenance_summary: None,
        };
        register_catalog_asset_v2(root, entry).unwrap();
    }

    fn manifest_json(assets: &[String]) -> String {
        format!(
            r#"{{
                "schemaVersion": "1",
                "kind": "game_art_manifest",
                "projectId": "forest-rpg",
                "name": "Forest RPG",
                "provider": {{ "id": "xai", "profileId": "default" }},
                "defaults": {{
                    "outputDirectory": "packs",
                    "godotRoot": "addons/forge_assets",
                    "license": "private"
                }},
                "assets": [ {} ]
            }}"#,
            assets.join(", ")
        )
    }

    fn asset_json(id: &str, kind: &str, extra: &str) -> String {
        let separator = if extra.is_empty() { "" } else { ", " };
        format!(
            r#"{{ "id": "{id}", "kind": "{kind}", "spec": "specs/{id}.json"{separator}{extra} }}"#
        )
    }

    fn write_specs(root: &Path, specs: &[(&str, String)]) {
        fs::create_dir_all(root.join("specs")).unwrap();
        for (id, contents) in specs {
            fs::write(root.join("specs").join(format!("{id}.json")), contents).unwrap();
        }
    }

    fn validate(root: &Path) -> ValidatedManifest {
        GameArtManifestV1::load_validated(&root.join("game-art.json")).unwrap()
    }

    fn full_caps() -> ProviderCapabilityInput {
        ProviderCapabilityInput {
            capabilities: ["edit_image", "image_to_video"]
                .iter()
                .map(|capability| capability.to_string())
                .collect(),
            image_model: Some("grok-image".into()),
            video_model: Some("grok-video".into()),
        }
    }

    fn plan_action<'a>(plan: &'a ProjectBuildPlanV1, asset_id: &str) -> &'a ProjectPlanActionV1 {
        plan.actions
            .iter()
            .find(|action| action.asset_id == asset_id)
            .unwrap_or_else(|| panic!("no plan action for {asset_id}"))
    }

    /// Two-asset project (hero character depending on hud-icons + a Subject
    /// Lock) where every catalog entry is faithful, built in one of two
    /// declaration/registration orders.
    fn setup_reused_project(root: &Path, hero_first: bool) -> ValidatedManifest {
        write_project(root);
        write_style_lock(root);
        write_subject_lock(root);
        let hero = asset_json(
            "hero",
            "character",
            r#""dependsOn": ["subject:hero@subject-rev-1", "hud-icons"]"#,
        );
        let icons = asset_json("hud-icons", "icon_set", "");
        let assets = if hero_first {
            vec![hero, icons]
        } else {
            vec![icons, hero]
        };
        write_specs(
            root,
            &[
                ("hero", CHARACTER_SPEC.to_string()),
                ("hud-icons", static_spec("hud-icons", "icon_set", &["coin"])),
            ],
        );
        fs::write(root.join("game-art.json"), manifest_json(&assets)).unwrap();
        let validated = validate(root);
        let registration_order: [&str; 2] = if hero_first {
            ["hero", "hud-icons"]
        } else {
            ["hud-icons", "hero"]
        };
        for asset_id in registration_order {
            let (kind, workflow, with_subject) = if asset_id == "hero" {
                ("character", "topdown@1.0.0", true)
            } else {
                ("icon_set", "static-set@1.0.0", false)
            };
            register_faithful_entry(root, &validated, asset_id, kind, workflow, with_subject);
        }
        validate(root)
    }

    #[test]
    fn plan_hash_is_deterministic_across_declaration_and_registration_order() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let validated_first = setup_reused_project(first.path(), true);
        let validated_second = setup_reused_project(second.path(), false);

        let diff_first = compute_project_diff(first.path(), &validated_first).unwrap();
        let diff_second = compute_project_diff(second.path(), &validated_second).unwrap();
        let plan_first =
            compute_build_plan(first.path(), &validated_first, &diff_first, &full_caps()).unwrap();
        let plan_second =
            compute_build_plan(second.path(), &validated_second, &diff_second, &full_caps())
                .unwrap();

        assert_eq!(plan_first.plan_sha256(), plan_second.plan_sha256());
        assert_eq!(plan_first.normalized_plan(), plan_second.normalized_plan());
        // Recomputing is stable too.
        assert_eq!(plan_first.plan_sha256(), plan_first.plan_sha256());
        // Both projects reuse everything.
        assert!(plan_first
            .actions
            .iter()
            .all(|action| action.action == PlanActionKindV1::Reuse));
    }

    #[test]
    fn plan_hash_is_sensitive_to_a_single_icon_spec_change() {
        let temp = tempfile::tempdir().unwrap();
        // hero and hud-icons are independent, so no dependency propagation.
        write_project(temp.path());
        write_style_lock(temp.path());
        write_specs(
            temp.path(),
            &[
                ("hero", CHARACTER_SPEC.to_string()),
                (
                    "hud-icons",
                    static_spec("hud-icons", "icon_set", &["coin", "gem"]),
                ),
            ],
        );
        let assets = [
            asset_json("hero", "character", ""),
            asset_json("hud-icons", "icon_set", ""),
        ];
        fs::write(temp.path().join("game-art.json"), manifest_json(&assets)).unwrap();
        let validated = validate(temp.path());
        register_faithful_entry(
            temp.path(),
            &validated,
            "hero",
            "character",
            "topdown@1.0.0",
            false,
        );
        register_faithful_entry(
            temp.path(),
            &validated,
            "hud-icons",
            "icon_set",
            "static-set@1.0.0",
            false,
        );

        let diff_before = compute_project_diff(temp.path(), &validate(temp.path())).unwrap();
        let plan_before = compute_build_plan(
            temp.path(),
            &validate(temp.path()),
            &diff_before,
            &full_caps(),
        )
        .unwrap();

        // Touch exactly one icon's prompt.
        fs::write(
            temp.path().join("specs/hud-icons.json"),
            static_spec("hud-icons", "icon_set", &["coin", "ruby gem"]),
        )
        .unwrap();
        let validated_after = validate(temp.path());
        let diff_after = compute_project_diff(temp.path(), &validated_after).unwrap();
        let plan_after =
            compute_build_plan(temp.path(), &validated_after, &diff_after, &full_caps()).unwrap();

        // Only the icon set's action flips; the character still reuses.
        assert_eq!(
            plan_action(&plan_after, "hud-icons").action,
            PlanActionKindV1::Rebuild
        );
        assert_eq!(
            plan_action(&plan_after, "hud-icons").reasons,
            vec![reasons::SPEC_CHANGED.to_string()]
        );
        assert_eq!(
            plan_action(&plan_after, "hero").action,
            PlanActionKindV1::Reuse
        );
        for asset_id in ["hero", "hud-icons"] {
            let before = plan_action(&plan_before, asset_id);
            let after = plan_action(&plan_after, asset_id);
            if asset_id == "hero" {
                assert_eq!(before, after, "untouched asset action must not change");
            } else {
                assert_ne!(before, after);
                assert_ne!(before.spec_sha256, after.spec_sha256);
            }
        }
        assert_ne!(plan_before.plan_sha256(), plan_after.plan_sha256());
    }

    #[test]
    fn plan_json_contains_no_host_paths_beyond_canonical_spec_paths() {
        let temp = tempfile::tempdir().unwrap();
        let validated = setup_reused_project(temp.path(), true);
        let diff = compute_project_diff(temp.path(), &validated).unwrap();
        let plan = compute_build_plan(temp.path(), &validated, &diff, &full_caps()).unwrap();

        let serialized = serde_json::to_value(&plan).unwrap();
        let canonical_root = temp.path().canonicalize().unwrap();
        let prefix = canonical_root.to_string_lossy().to_string();
        let mut host_paths = Vec::new();
        collect_prefixed_strings(&serialized, &prefix, None, &mut host_paths);
        assert!(
            !host_paths.is_empty(),
            "test is vacuous without at least one canonical spec path"
        );
        for (key, value) in &host_paths {
            assert_eq!(
                *key,
                Some("canonicalSpecPath"),
                "unexpected host-absolute path {value} under key {key:?}"
            );
        }
        // And the plan hash must not change when those paths move: normalized
        // form contains no host prefix at all.
        let normalized = plan.normalized_plan();
        let mut normalized_paths = Vec::new();
        collect_prefixed_strings(&normalized, &prefix, None, &mut normalized_paths);
        assert!(normalized_paths.is_empty());
    }

    fn collect_prefixed_strings<'a>(
        value: &'a Value,
        prefix: &str,
        key: Option<&'a str>,
        out: &mut Vec<(Option<&'a str>, &'a str)>,
    ) {
        match value {
            Value::String(text) if text.starts_with(prefix) => out.push((key, text)),
            Value::Array(items) => {
                for item in items {
                    collect_prefixed_strings(item, prefix, key, out);
                }
            }
            Value::Object(map) => {
                for (field, nested) in map {
                    collect_prefixed_strings(nested, prefix, Some(field.as_str()), out);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn plan_estimates_cache_capabilities_and_null_cost_fields() {
        let temp = tempfile::tempdir().unwrap();
        write_project(temp.path());
        write_style_lock(temp.path());
        write_specs(
            temp.path(),
            &[
                ("hero", CHARACTER_SPEC.to_string()),
                (
                    "hud-icons",
                    static_spec("hud-icons", "icon_set", &["coin", "gem", "key"]),
                ),
                ("props", static_spec("props", "prop_set", &["crate"])),
            ],
        );
        let assets = [
            asset_json("hero", "character", ""),
            asset_json("hud-icons", "icon_set", ""),
            asset_json("props", "prop_set", ""),
        ];
        fs::write(temp.path().join("game-art.json"), manifest_json(&assets)).unwrap();
        let validated = validate(temp.path());
        // props reuses; hero and hud-icons are new builds. Plus one orphan.
        register_faithful_entry(
            temp.path(),
            &validated,
            "props",
            "prop_set",
            "static-set@1.0.0",
            false,
        );
        let orphan = ProjectCatalogEntryV2 {
            asset_id: "old-asset".into(),
            name: "Old".into(),
            kind: "icon_set".into(),
            pack_path: PathBuf::from("packs/old-asset"),
            pack_sha256: "c".repeat(64),
            source_job_id: "job-0".into(),
            parent_job_id: None,
            style: None,
            subject: None,
            workflow: "static-set@1.0.0".into(),
            provider: None,
            installed: None,
            created_at: fixed_time(),
            spec_path: None,
            spec_sha256: None,
            dependencies: None,
            locks: None,
            workflow_profile: None,
            workflow_version: None,
            pack_version: None,
            quality_verdict: None,
            quality_profile: None,
            game_ready: None,
            generated_at: None,
            reviewed_at: None,
            license: None,
            provenance_summary: None,
        };
        register_catalog_asset_v2(temp.path(), orphan).unwrap();

        let diff = compute_project_diff(temp.path(), &validated).unwrap();
        let plan = compute_build_plan(temp.path(), &validated, &diff, &full_caps()).unwrap();

        assert_eq!(plan.schema_version, "1");
        assert_eq!(plan.kind, "project_build_plan");
        assert_eq!(plan.style_revision.as_deref(), Some("style-rev-1"));
        assert_eq!(plan.provider.id, "xai");
        assert_eq!(plan.provider.profile_id, "default");
        assert_eq!(plan.provider.image_model.as_deref(), Some("grok-image"));
        assert_eq!(plan.provider.video_model.as_deref(), Some("grok-video"));

        // Orphans leave actions and land in deleteCandidates only.
        assert_eq!(plan.delete_candidates, vec!["old-asset".to_string()]);
        assert_eq!(plan.actions.len(), 3);
        assert!(plan
            .actions
            .iter()
            .all(|action| action.asset_id != "old-asset"));

        // Per-action estimates mirror estimate_operation: character 9/13,
        // static set items/2*items, reuse 0/0.
        let hero = plan_action(&plan, "hero");
        assert_eq!(hero.action, PlanActionKindV1::Build);
        assert_eq!(hero.workflow, "topdown@1.0.0");
        assert_eq!(hero.provider_request_estimate, 9);
        assert_eq!(hero.maximum_provider_requests, 13);
        let icons = plan_action(&plan, "hud-icons");
        assert_eq!(icons.workflow, "static-set@1.0.0");
        assert_eq!(icons.provider_request_estimate, 3);
        assert_eq!(icons.maximum_provider_requests, 6);
        let props = plan_action(&plan, "props");
        assert_eq!(props.action, PlanActionKindV1::Reuse);
        assert_eq!(props.provider_request_estimate, 0);
        assert_eq!(props.maximum_provider_requests, 0);

        // Totals, cache accounting and the local node formula.
        assert_eq!(plan.provider_request_estimate, 12);
        assert_eq!(plan.maximum_provider_requests, 19);
        assert_eq!(plan.cache_hits, 1);
        assert_eq!(plan.cache_misses, 2);
        assert_eq!(
            plan.cache_hit_kinds.get("catalog_reuse"),
            Some(&1),
            "catalog reuse is the only stage 2 cache kind"
        );
        assert_eq!(plan.local_node_count, 1 + 2 * 2);
        assert!(plan.unmet_capabilities.is_empty());

        // Cost fields exist and are explicit nulls (no stage 2 cost model).
        let serialized = serde_json::to_value(&plan).unwrap();
        assert_eq!(serialized["estimatedCostTicks"], Value::Null);
        assert_eq!(serialized["maximumCostTicks"], Value::Null);
        // No token/expiry in the pure-content plan.
        assert!(serialized.get("token").is_none());
        assert!(serialized.get("createdAt").is_none());
        assert!(serialized.get("expiresAt").is_none());

        // Missing image_to_video is reported (character video path needs it).
        let limited = ProviderCapabilityInput {
            capabilities: ["edit_image".to_string()].into_iter().collect(),
            image_model: None,
            video_model: None,
        };
        let plan_limited = compute_build_plan(temp.path(), &validated, &diff, &limited).unwrap();
        assert_eq!(
            plan_limited.unmet_capabilities,
            vec!["image_to_video".to_string()]
        );
    }

    #[test]
    fn plan_rejects_diff_of_another_manifest() {
        let temp = tempfile::tempdir().unwrap();
        let validated = setup_reused_project(temp.path(), true);
        let diff: ProjectDiffV1 = compute_project_diff(temp.path(), &validated).unwrap();
        // Rewrite the manifest with a different name (same assets) so the
        // manifest hash changes under the same diff.
        let rewritten = fs::read_to_string(temp.path().join("game-art.json"))
            .unwrap()
            .replace("Forest RPG", "Swamp RPG");
        fs::write(temp.path().join("game-art.json"), rewritten).unwrap();
        let other = validate(temp.path());
        let error = compute_build_plan(temp.path(), &other, &diff, &full_caps()).unwrap_err();
        assert_eq!(error.code(), "invalid_manifest");
    }

    #[test]
    fn plan_rejects_spec_kind_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        write_project(temp.path());
        write_style_lock(temp.path());
        // Manifest says icon_set, the spec file is a prop_set.
        write_specs(
            temp.path(),
            &[("mixed", static_spec("mixed", "prop_set", &["crate"]))],
        );
        let assets = [asset_json("mixed", "icon_set", "")];
        fs::write(temp.path().join("game-art.json"), manifest_json(&assets)).unwrap();
        let validated = validate(temp.path());
        let diff = compute_project_diff(temp.path(), &validated).unwrap();
        let error = compute_build_plan(temp.path(), &validated, &diff, &full_caps()).unwrap_err();
        assert_eq!(error.code(), "invalid_manifest");
        assert!(error.to_string().contains("prop_set"));
    }
}
