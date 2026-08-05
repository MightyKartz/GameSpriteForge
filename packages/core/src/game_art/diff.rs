//! Stage 2 project diff: compares a validated [`GameArtManifestV1`] against
//! the on-disk project state (`.forge/catalog.json`, Style/Subject Locks) and
//! decides, per asset, whether the existing catalog entry can be reused or the
//! asset must be (re)built. Stage 2 never deletes: catalog assets missing from
//! the manifest are reported as delete candidates only.
//!
//! Reuse requires *all* of the following to hold for the catalog entry:
//!
//! - `specSha256` matches the validated spec content hash;
//! - the recorded Style revision matches the resolved Style revision
//!   (`manifest.styleRevision`, falling back to the project's
//!   `currentStyleRevision`);
//! - every `dependsOn` lock reference resolves to a lock in the project whose
//!   recorded revision matches;
//! - the recorded provider id/profile matches the manifest provider;
//! - the pack path still exists and its recomputed content hash matches the
//!   recorded `packSha256`.
//!
//! Any failed check flips the asset to `rebuild` with the machine-readable
//! reason codes in [`reasons`]. A rebuilt or newly built asset-id dependency
//! transitively forces `rebuild` of its dependents (`dependency_rebuilt`).
//!
//! [`GameArtManifestV1`]: super::GameArtManifestV1

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::types::{is_valid_lock_revision, GameArtError, GameArtProviderV1, LockKind, LockRef};
use super::ValidatedManifest;
use crate::asset_project::{
    read_project, read_style_lock, resolve_relative, AssetProjectError, StyleLockV1,
    STYLE_LOCK_FILE,
};
use crate::catalog::{read_project_catalog, CatalogError, ProjectCatalogEntryV2};
use crate::subject::{read_subject_lock, subject_lock_path, SubjectLockV1};

/// Only schema version emitted by this stage.
pub const PROJECT_DIFF_SCHEMA_VERSION: &str = "1";

/// Machine-readable reason codes recorded on `build`/`rebuild`/`orphan`
/// diff actions. `reuse` actions always carry an empty reason list.
pub mod reasons {
    /// No catalog entry exists for the asset (action `build`).
    pub const NEW_ASSET: &str = "new_asset";
    /// Catalog entry is missing `specSha256` or it differs from the
    /// validated spec content hash.
    pub const SPEC_CHANGED: &str = "spec_changed";
    /// Resolved Style revision differs from the revision the catalog entry
    /// was built against (or the entry recorded none).
    pub const STYLE_REVISION_CHANGED: &str = "style_revision_changed";
    /// A `dependsOn` lock reference revision differs from the revision the
    /// catalog entry recorded (or the entry recorded none).
    pub const LOCK_REVISION_CHANGED: &str = "lock_revision_changed";
    /// Manifest provider id/profile differs from the catalog entry's (or the
    /// entry recorded none).
    pub const PROVIDER_CHANGED: &str = "provider_changed";
    /// Manifest kind differs from the catalog entry's generated asset kind.
    pub const KIND_CHANGED: &str = "kind_changed";
    /// The workflow assigned to this manifest asset differs from the catalog.
    pub const WORKFLOW_CHANGED: &str = "workflow_changed";
    /// The declared asset dependency set differs from catalog provenance.
    pub const DEPENDENCIES_CHANGED: &str = "dependencies_changed";
    /// The recorded pack path no longer exists on disk.
    pub const PACK_MISSING: &str = "pack_missing";
    /// The pack exists but its recomputed content hash differs from the
    /// recorded `packSha256`.
    pub const PACK_HASH_MISMATCH: &str = "pack_hash_mismatch";
    /// Pack path/hash exist but the directory fails the `.gsfpack` contract.
    pub const PACK_INVALID: &str = "pack_invalid";
    /// An asset-id dependency is (transitively) rebuilt or newly built.
    pub const DEPENDENCY_REBUILT: &str = "dependency_rebuilt";
    /// Catalog asset id is not declared in the manifest (action `orphan`).
    pub const NOT_IN_MANIFEST: &str = "not_in_manifest";
}

/// A `dependsOn` lock reference resolved against the project's on-disk locks:
/// the immutable revision plus the provider identity recorded in the lock.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolvedLockRefV1 {
    pub kind: LockKind,
    pub id: String,
    pub revision: String,
    pub provider_id: String,
    pub profile_id: String,
    pub lock_sha256: String,
}

/// Per-asset diff verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffActionKindV1 {
    /// Catalog entry matches the manifest; no provider work needed.
    Reuse,
    /// No catalog entry exists; the asset must be built.
    Build,
    /// A catalog entry exists but is stale; the asset must be rebuilt.
    Rebuild,
    /// Catalog entry exists but the asset is not declared in the manifest.
    /// Report only — stage 2 never deletes.
    Orphan,
}

impl DiffActionKindV1 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reuse => "reuse",
            Self::Build => "build",
            Self::Rebuild => "rebuild",
            Self::Orphan => "orphan",
        }
    }
}

/// Diff verdict for one asset. Manifest assets carry the validated
/// `specSha256`, asset-id dependencies and resolved lock references; orphan
/// actions (catalog-only assets) carry the raw catalog kind and no spec hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetDiffActionV1 {
    pub asset_id: String,
    /// Manifest asset kind (`character`, `icon_set`, `prop_set`), or the raw
    /// catalog kind string for orphans, which may predate the stage 2 kind set.
    pub kind: String,
    pub action: DiffActionKindV1,
    /// Machine-readable codes from [`reasons`]; empty for `reuse`.
    pub reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_sha256: Option<String>,
    /// Declared asset-id dependencies (lock references excluded), sorted.
    pub depends_on_assets: Vec<String>,
    /// Resolved `dependsOn` lock references, sorted by kind/id/revision.
    pub lock_refs: Vec<ResolvedLockRefV1>,
}

/// Read-only project diff; the input of the plan layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectDiffV1 {
    pub schema_version: String,
    pub manifest_sha256: String,
    pub graph_sha256: String,
    pub provider: GameArtProviderV1,
    /// Resolved Style revision: `manifest.styleRevision` when set, otherwise
    /// the project's `currentStyleRevision`; `null` when neither exists.
    #[serde(default)]
    pub style_revision: Option<String>,
    /// Manifest assets in topological build order, then orphans sorted by id.
    pub actions: Vec<AssetDiffActionV1>,
    /// Catalog asset ids missing from the manifest, sorted. Report only.
    pub delete_candidates: Vec<String>,
}

/// Compute the project diff for a validated manifest against the Forge
/// project at `project_root` (holds `forge-project.json`, `.forge/catalog.json`
/// and the Style/Subject Locks).
///
/// Errors: `unknown_lock` when a referenced Style/Subject Lock does not exist
/// in the project, `lock_revision_mismatch` when the lock on disk records a
/// different id/revision than the reference, `io_error` for unreadable project
/// files, `invalid_manifest` for a corrupt catalog/project file.
pub fn compute_project_diff(
    project_root: &Path,
    validated: &ValidatedManifest,
) -> Result<ProjectDiffV1, GameArtError> {
    let project = read_project(project_root).map_err(|error| match error {
        AssetProjectError::Io(source) => GameArtError::Io(format!(
            "cannot read project {}: {source}",
            project_root.display()
        )),
        other => GameArtError::InvalidManifest(format!(
            "invalid project {}: {other}",
            project_root.display()
        )),
    })?;
    let catalog = read_project_catalog(project_root).map_err(|error| match error {
        CatalogError::Io(source) => GameArtError::Io(format!(
            "cannot read project catalog under {}: {source}",
            project_root.display()
        )),
        other => GameArtError::InvalidManifest(format!("invalid project catalog: {other}")),
    })?;

    let manifest = &validated.manifest;
    if manifest.project_id != project.project_id {
        return Err(GameArtError::InvalidManifest(format!(
            "manifest projectId \"{}\" does not match forge project \"{}\"",
            manifest.project_id, project.project_id
        )));
    }
    let style_revision = resolve_style_revision(
        project_root,
        manifest
            .style_revision
            .as_deref()
            .or(project.current_style_revision.as_deref()),
        &manifest.provider,
    )?;
    let graph = manifest.dependency_graph();

    // Base verdicts from on-disk state, before dependency propagation.
    let mut verdicts: BTreeMap<String, AssetDiffActionV1> = BTreeMap::new();
    for asset in &manifest.assets {
        let spec = validated.asset(&asset.id).ok_or_else(|| {
            GameArtError::InvalidManifest(format!(
                "validated manifest has no resolved spec for asset \"{}\"",
                asset.id
            ))
        })?;
        let depends_on_assets = graph.get(&asset.id).cloned().unwrap_or_default();
        let lock_refs =
            resolve_asset_lock_refs(project_root, &asset.depends_on, &graph, &manifest.provider)?;
        if let Some(reference) = lock_refs
            .iter()
            .find(|reference| reference.kind == LockKind::Style)
        {
            if style_revision.as_deref() != Some(reference.revision.as_str()) {
                return Err(GameArtError::InvalidLockRef(format!(
                    "asset \"{}\" style lock {}@{} does not match the effective project style revision {:?}",
                    asset.id, reference.id, reference.revision, style_revision
                )));
            }
        }
        let mut action = match catalog.assets.get(&asset.id) {
            None => AssetDiffActionV1 {
                asset_id: asset.id.clone(),
                kind: asset.kind.as_str().to_string(),
                action: DiffActionKindV1::Build,
                reasons: vec![reasons::NEW_ASSET.to_string()],
                spec_sha256: Some(spec.spec_sha256.clone()),
                depends_on_assets,
                lock_refs,
            },
            Some(entry) => {
                let mut failed = Vec::new();
                if entry.kind != asset.kind.as_str() {
                    failed.push(reasons::KIND_CHANGED);
                }
                if entry.workflow != expected_workflow(asset.kind) {
                    failed.push(reasons::WORKFLOW_CHANGED);
                }
                if !recorded_asset_dependencies_match(entry, &depends_on_assets, &catalog) {
                    failed.push(reasons::DEPENDENCIES_CHANGED);
                }
                if entry.spec_sha256.as_deref() != Some(spec.spec_sha256.as_str()) {
                    failed.push(reasons::SPEC_CHANGED);
                }
                if let Some(style_revision) = &style_revision {
                    if recorded_style_revision(entry).as_deref() != Some(style_revision.as_str()) {
                        failed.push(reasons::STYLE_REVISION_CHANGED);
                    }
                } else if recorded_style_revision(entry).is_some() {
                    failed.push(reasons::STYLE_REVISION_CHANGED);
                }
                if !recorded_lock_revisions_match(entry, &lock_refs) {
                    failed.push(reasons::LOCK_REVISION_CHANGED);
                }
                if !recorded_provider_matches(entry, &manifest.provider) {
                    failed.push(reasons::PROVIDER_CHANGED);
                }
                match verify_pack(project_root, entry)? {
                    PackVerdict::Intact => {}
                    PackVerdict::Missing => failed.push(reasons::PACK_MISSING),
                    PackVerdict::HashMismatch => failed.push(reasons::PACK_HASH_MISMATCH),
                    PackVerdict::Invalid => failed.push(reasons::PACK_INVALID),
                }
                let (action, reasons) = if failed.is_empty() {
                    (DiffActionKindV1::Reuse, Vec::new())
                } else {
                    (
                        DiffActionKindV1::Rebuild,
                        failed.into_iter().map(str::to_string).collect(),
                    )
                };
                AssetDiffActionV1 {
                    asset_id: asset.id.clone(),
                    kind: asset.kind.as_str().to_string(),
                    action,
                    reasons,
                    spec_sha256: Some(spec.spec_sha256.clone()),
                    depends_on_assets,
                    lock_refs,
                }
            }
        };
        action.depends_on_assets.sort();
        action.lock_refs.sort();
        verdicts.insert(asset.id.clone(), action);
    }

    // Transitive propagation in topological order: any built/rebuilt asset-id
    // dependency forces rebuild of its dependents.
    let order = topological_build_order(validated);
    for asset_id in &order {
        let dependency_rebuilt = graph
            .get(asset_id)
            .map(|dependencies| {
                dependencies.iter().any(|dependency| {
                    matches!(
                        verdicts.get(dependency).map(|entry| entry.action),
                        Some(DiffActionKindV1::Build | DiffActionKindV1::Rebuild)
                    )
                })
            })
            .unwrap_or(false);
        if !dependency_rebuilt {
            continue;
        }
        let entry = verdicts
            .get_mut(asset_id)
            .expect("graph nodes have verdicts");
        match entry.action {
            DiffActionKindV1::Reuse => {
                entry.action = DiffActionKindV1::Rebuild;
                entry.reasons = vec![reasons::DEPENDENCY_REBUILT.to_string()];
            }
            DiffActionKindV1::Rebuild => {
                entry.reasons.push(reasons::DEPENDENCY_REBUILT.to_string());
            }
            // A brand-new asset is built regardless of dependency state.
            DiffActionKindV1::Build | DiffActionKindV1::Orphan => {}
        }
    }

    // Orphans: catalog assets not declared in the manifest. Report only.
    let declared: BTreeSet<&str> = manifest
        .assets
        .iter()
        .map(|asset| asset.id.as_str())
        .collect();
    let mut delete_candidates = Vec::new();
    let mut orphans = Vec::new();
    for (asset_id, entry) in &catalog.assets {
        if declared.contains(asset_id.as_str()) {
            continue;
        }
        delete_candidates.push(asset_id.clone());
        orphans.push(AssetDiffActionV1 {
            asset_id: asset_id.clone(),
            kind: entry.kind.clone(),
            action: DiffActionKindV1::Orphan,
            reasons: vec![reasons::NOT_IN_MANIFEST.to_string()],
            spec_sha256: None,
            depends_on_assets: Vec::new(),
            lock_refs: Vec::new(),
        });
    }

    let mut actions: Vec<AssetDiffActionV1> = order
        .iter()
        .map(|asset_id| {
            verdicts
                .get(asset_id)
                .expect("topological order covers every manifest asset")
                .clone()
        })
        .collect();
    actions.extend(orphans);

    Ok(ProjectDiffV1 {
        schema_version: PROJECT_DIFF_SCHEMA_VERSION.to_string(),
        manifest_sha256: manifest.manifest_sha256(),
        graph_sha256: manifest.graph_sha256(),
        provider: manifest.provider.clone(),
        style_revision,
        actions,
        delete_candidates,
    })
}

/// Fingerprint immutable project sources that the build reads while running.
/// Catalog and pack state are intentionally excluded because the build writes
/// them between child operations; plan-token fingerprinting binds those
/// separately before execution. Specs include their nested reference bytes.
pub fn project_source_sha256(
    project_root: &Path,
    validated: &ValidatedManifest,
    diff: &ProjectDiffV1,
) -> Result<String, GameArtError> {
    let mut hasher = Sha256::new();
    hasher.update(b"forge-project-sources-v1\0");
    let project_bytes = fs::read(project_root.join("forge-project.json")).map_err(|error| {
        GameArtError::Io(format!(
            "cannot read project source {}: {error}",
            project_root.join("forge-project.json").display()
        ))
    })?;
    hash_source_field(&mut hasher, b"project", &project_bytes);
    hash_source_field(
        &mut hasher,
        b"manifest",
        validated.manifest.manifest_sha256().as_bytes(),
    );
    for asset in &validated.assets {
        hash_source_field(
            &mut hasher,
            asset.asset_id.as_bytes(),
            asset.spec_sha256.as_bytes(),
        );
    }
    if let Some(revision) = &diff.style_revision {
        let directory = project_root.join(".forge/styles").join(revision);
        let digest = hash_pack(&directory, true).map_err(|error| {
            GameArtError::Io(format!(
                "cannot hash style source {}: {error}",
                directory.display()
            ))
        })?;
        hash_source_field(&mut hasher, b"style", digest.as_bytes());
    }
    let mut lock_directories = BTreeSet::new();
    for action in &diff.actions {
        for reference in &action.lock_refs {
            let directory = match reference.kind {
                LockKind::Style => project_root.join(".forge/styles").join(&reference.revision),
                LockKind::Subject => project_root
                    .join(".forge/subjects")
                    .join(&reference.id)
                    .join(&reference.revision),
            };
            lock_directories.insert(directory);
        }
    }
    for directory in lock_directories {
        let digest = hash_pack(&directory, true).map_err(|error| {
            GameArtError::Io(format!(
                "cannot hash lock source {}: {error}",
                directory.display()
            ))
        })?;
        hash_source_field(
            &mut hasher,
            directory
                .strip_prefix(project_root)
                .unwrap_or(&directory)
                .to_string_lossy()
                .as_bytes(),
            digest.as_bytes(),
        );
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn hash_source_field(hasher: &mut Sha256, label: &[u8], contents: &[u8]) {
    hasher.update((label.len() as u64).to_le_bytes());
    hasher.update(label);
    hasher.update((contents.len() as u64).to_le_bytes());
    hasher.update(contents);
}

/// Deterministic asset-id dependency build order (dependencies first) using
/// Kahn's algorithm with a sorted ready set. The manifest is validated
/// cycle-free, so every declared asset is emitted exactly once; lock
/// references are not graph edges.
pub fn topological_build_order(validated: &ValidatedManifest) -> Vec<String> {
    let graph = validated.manifest.dependency_graph();
    let mut remaining: BTreeMap<&str, usize> = graph
        .iter()
        .map(|(asset, dependencies)| (asset.as_str(), dependencies.len()))
        .collect();
    let mut dependents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (asset, dependencies) in &graph {
        for dependency in dependencies {
            dependents
                .entry(dependency.as_str())
                .or_default()
                .push(asset.as_str());
        }
    }
    let mut ready: BTreeSet<&str> = remaining
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(asset, _)| *asset)
        .collect();
    let mut order = Vec::with_capacity(remaining.len());
    while let Some(asset) = ready.iter().next().copied() {
        ready.remove(asset);
        order.push(asset.to_string());
        if let Some(dependent_list) = dependents.get(asset) {
            for dependent in dependent_list {
                let count = remaining
                    .get_mut(dependent)
                    .expect("dependency edges reference declared assets");
                *count -= 1;
                if *count == 0 {
                    ready.insert(dependent);
                }
            }
        }
    }
    // The manifest is validated cycle-free, so this is unreachable in
    // practice; appended for totality should that guarantee ever be bypassed.
    debug_assert_eq!(
        order.len(),
        graph.len(),
        "dependency cycle escaped validation"
    );
    for (asset, count) in remaining {
        if count > 0 && !order.iter().any(|emitted| emitted == asset) {
            order.push(asset.to_string());
        }
    }
    order
}

/// Resolve the effective Style revision and verify the referenced Style Lock
/// exists in the project with a matching recorded revision.
fn resolve_style_revision(
    project_root: &Path,
    style_revision: Option<&str>,
    provider: &GameArtProviderV1,
) -> Result<Option<String>, GameArtError> {
    let Some(revision) = style_revision else {
        return Ok(None);
    };
    if !is_valid_lock_revision(revision) {
        return Err(GameArtError::InvalidLockRef(format!(
            "style revision \"{revision}\" must be a single [A-Za-z0-9_.-]+ path component"
        )));
    }
    let lock_path = project_root
        .join(".forge/styles")
        .join(revision)
        .join(STYLE_LOCK_FILE);
    if !lock_path.is_file() {
        return Err(GameArtError::UnknownLock(format!(
            "style lock revision \"{revision}\" not found at {}",
            lock_path.display()
        )));
    }
    ensure_lock_stays_in_project(project_root, &lock_path, &format!("style@{revision}"))?;
    let lock = read_validated_style_lock(project_root, &lock_path, &format!("style@{revision}"))?;
    if lock.revision != revision {
        return Err(GameArtError::LockRevisionMismatch(format!(
            "style lock at {} records revision \"{}\", expected \"{revision}\"",
            lock_path.display(),
            lock.revision
        )));
    }
    ensure_lock_provider(
        &format!("style@{revision}"),
        &lock.provider_id,
        &lock.profile_id,
        provider,
    )?;
    Ok(Some(revision.to_string()))
}

/// Resolve every `dependsOn` lock reference of one asset against the project.
/// Declared asset ids (graph edges) are skipped; malformed references were
/// already rejected by manifest validation.
fn resolve_asset_lock_refs(
    project_root: &Path,
    depends_on: &[String],
    graph: &BTreeMap<String, Vec<String>>,
    provider: &GameArtProviderV1,
) -> Result<Vec<ResolvedLockRefV1>, GameArtError> {
    let mut resolved = Vec::new();
    for dependency in depends_on {
        if graph.contains_key(dependency) {
            continue;
        }
        let reference = LockRef::parse(dependency)?;
        resolved.push(resolve_lock_ref(project_root, &reference, provider)?);
    }
    resolved.sort();
    resolved.dedup();
    Ok(resolved)
}

fn resolve_lock_ref(
    project_root: &Path,
    reference: &LockRef,
    provider: &GameArtProviderV1,
) -> Result<ResolvedLockRefV1, GameArtError> {
    match reference.kind {
        LockKind::Subject => {
            let lock_path = subject_lock_path(project_root, &reference.id, &reference.revision);
            if !lock_path.is_file() {
                return Err(GameArtError::UnknownLock(format!(
                    "subject lock \"{reference}\" not found at {}",
                    lock_path.display()
                )));
            }
            ensure_lock_stays_in_project(project_root, &lock_path, &reference.to_string())?;
            let parsed: SubjectLockV1 = read_lock_json(&lock_path, &reference.to_string())?;
            if parsed.id != reference.id || parsed.revision != reference.revision {
                return Err(GameArtError::LockRevisionMismatch(format!(
                    "subject lock at {} records \"{}@{}\", expected \"{reference}\"",
                    lock_path.display(),
                    parsed.id,
                    parsed.revision
                )));
            }
            ensure_lock_stays_in_project(
                project_root,
                &parsed.canonical_path,
                &format!("{reference} canonical image"),
            )?;
            ensure_lock_stays_in_project(
                project_root,
                &parsed.mask_path,
                &format!("{reference} mask"),
            )?;
            let lock = read_subject_lock(&lock_path).map_err(|error| {
                GameArtError::InvalidLockRef(format!(
                    "subject lock \"{reference}\" failed integrity validation: {error}"
                ))
            })?;
            ensure_lock_provider(
                &reference.to_string(),
                &lock.provider_id,
                &lock.profile_id,
                provider,
            )?;
            Ok(ResolvedLockRefV1 {
                kind: LockKind::Subject,
                id: reference.id.clone(),
                revision: reference.revision.clone(),
                provider_id: lock.provider_id,
                profile_id: lock.profile_id,
                lock_sha256: semantic_lock_sha256(project_root, &lock_path)?,
            })
        }
        LockKind::Style => {
            let lock_path = project_root
                .join(".forge/styles")
                .join(&reference.revision)
                .join(STYLE_LOCK_FILE);
            if !lock_path.is_file() {
                return Err(GameArtError::UnknownLock(format!(
                    "style lock \"{reference}\" not found at {}",
                    lock_path.display()
                )));
            }
            ensure_lock_stays_in_project(project_root, &lock_path, &reference.to_string())?;
            let lock = read_validated_style_lock(project_root, &lock_path, &reference.to_string())?;
            if lock.revision != reference.revision {
                return Err(GameArtError::LockRevisionMismatch(format!(
                    "style lock at {} records revision \"{}\", expected \"{reference}\"",
                    lock_path.display(),
                    lock.revision
                )));
            }
            ensure_lock_provider(
                &reference.to_string(),
                &lock.provider_id,
                &lock.profile_id,
                provider,
            )?;
            Ok(ResolvedLockRefV1 {
                kind: LockKind::Style,
                id: reference.id.clone(),
                revision: reference.revision.clone(),
                provider_id: lock.provider_id,
                profile_id: lock.profile_id,
                lock_sha256: semantic_lock_sha256(project_root, &lock_path)?,
            })
        }
    }
}

fn read_validated_style_lock(
    project_root: &Path,
    lock_path: &Path,
    reference: &str,
) -> Result<StyleLockV1, GameArtError> {
    let parsed: StyleLockV1 = read_lock_json(lock_path, reference)?;
    ensure_lock_stays_in_project(
        project_root,
        &parsed.board_path,
        &format!("{reference} style board"),
    )?;
    read_style_lock(lock_path).map_err(|error| {
        GameArtError::InvalidLockRef(format!(
            "style lock \"{reference}\" failed integrity validation: {error}"
        ))
    })
}

/// Hash a lock without binding its identity to the checkout's absolute path.
/// Forge-created Style and Subject locks store their media paths as absolute
/// paths, so hashing the raw JSON would make otherwise identical plans differ
/// between project directories. The referenced media hashes remain in the
/// document and the path values are normalized to project-relative paths.
pub(crate) fn semantic_lock_sha256(
    project_root: &Path,
    path: &Path,
) -> Result<String, GameArtError> {
    let bytes = fs::read(path)
        .map_err(|error| GameArtError::Io(format!("cannot read {}: {error}", path.display())))?;
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
        GameArtError::InvalidLockRef(format!("invalid lock JSON at {}: {error}", path.display()))
    })?;
    let canonical_project = project_root.canonicalize().map_err(|error| {
        GameArtError::Io(format!(
            "cannot canonicalize project {}: {error}",
            project_root.display()
        ))
    })?;
    let object = value.as_object_mut().ok_or_else(|| {
        GameArtError::InvalidLockRef(format!("lock at {} must be a JSON object", path.display()))
    })?;
    for field in ["boardPath", "canonicalPath", "maskPath"] {
        let Some(value) = object.get_mut(field) else {
            continue;
        };
        let Some(raw_path) = value.as_str() else {
            return Err(GameArtError::InvalidLockRef(format!(
                "lock field {field} at {} must be a path string",
                path.display()
            )));
        };
        let media_path = Path::new(raw_path);
        if media_path.is_absolute() {
            let canonical_media = media_path.canonicalize().map_err(|error| {
                GameArtError::Io(format!(
                    "cannot canonicalize lock media {}: {error}",
                    media_path.display()
                ))
            })?;
            let relative = canonical_media
                .strip_prefix(&canonical_project)
                .map_err(|_| {
                    GameArtError::SymlinkEscape(format!(
                        "lock media {} resolves outside project {}",
                        media_path.display(),
                        canonical_project.display()
                    ))
                })?;
            *value = serde_json::Value::String(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    let normalized = serde_json::to_vec(&value).map_err(|error| {
        GameArtError::InvalidLockRef(format!("cannot normalize lock {}: {error}", path.display()))
    })?;
    Ok(format!("{:x}", Sha256::digest(normalized)))
}

fn ensure_lock_provider(
    reference: &str,
    provider_id: &str,
    profile_id: &str,
    expected: &GameArtProviderV1,
) -> Result<(), GameArtError> {
    if provider_id == expected.id && profile_id == expected.profile_id {
        return Ok(());
    }
    Err(GameArtError::LockRevisionMismatch(format!(
        "lock \"{reference}\" belongs to provider {provider_id}/{profile_id}, expected {}/{}",
        expected.id, expected.profile_id
    )))
}

fn ensure_lock_stays_in_project(
    project_root: &Path,
    lock_path: &Path,
    reference: &str,
) -> Result<(), GameArtError> {
    let canonical_project = project_root.canonicalize().map_err(|error| {
        GameArtError::Io(format!(
            "cannot canonicalize project {}: {error}",
            project_root.display()
        ))
    })?;
    let canonical_lock = lock_path.canonicalize().map_err(|error| {
        GameArtError::Io(format!(
            "cannot canonicalize lock \"{reference}\" at {}: {error}",
            lock_path.display()
        ))
    })?;
    if !canonical_lock.starts_with(&canonical_project) {
        return Err(GameArtError::SymlinkEscape(format!(
            "lock \"{reference}\" resolves outside project {}",
            canonical_project.display()
        )));
    }
    Ok(())
}

/// Parse the small header struct the diff layer needs out of a lock file.
/// Unreadable files surface as `io_error`; corrupt JSON as `invalid_lock_ref`
/// (the reference cannot be resolved to a usable lock).
fn read_lock_json<T: for<'de> Deserialize<'de>>(
    path: &Path,
    reference: &str,
) -> Result<T, GameArtError> {
    let bytes = fs::read(path).map_err(|error| {
        GameArtError::Io(format!(
            "cannot read lock \"{reference}\" at {}: {error}",
            path.display()
        ))
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        GameArtError::InvalidLockRef(format!(
            "lock \"{reference}\" at {} is not a valid lock file: {error}",
            path.display()
        ))
    })
}

/// Style revision recorded on a catalog entry: the V2 `locks.style` wins over
/// the legacy V1 `style.revision`.
fn recorded_style_revision(entry: &ProjectCatalogEntryV2) -> Option<String> {
    entry
        .locks
        .as_ref()
        .and_then(|locks| locks.style.clone())
        .or_else(|| entry.style.as_ref().map(|style| style.revision.clone()))
}

/// Every resolved lock reference must match the revision the catalog entry
/// recorded. Subject references prefer the id-keyed V1 `subject` record and
/// fall back to the bare V2 `locks.subject` revision (the V2 schema keeps a
/// single revision string, so multiple subject references share it); style
/// references compare against the recorded Style revision. An entry with no
/// recorded revision cannot prove freshness and fails the check.
fn recorded_lock_revisions_match(
    entry: &ProjectCatalogEntryV2,
    lock_refs: &[ResolvedLockRefV1],
) -> bool {
    let expected_keys = lock_refs
        .iter()
        .map(|reference| format!("{}:{}", reference.kind, reference.id))
        .collect::<BTreeSet<_>>();
    let recorded_keys = entry
        .dependencies
        .as_ref()
        .map(|dependencies| {
            dependencies
                .iter()
                .filter(|dependency| {
                    dependency.id.starts_with("style:") || dependency.id.starts_with("subject:")
                })
                .map(|dependency| dependency.id.clone())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    if expected_keys != recorded_keys {
        return false;
    }
    lock_refs.iter().all(|reference| {
        let dependency_matches = entry.dependencies.as_ref().is_some_and(|dependencies| {
            dependencies.iter().any(|dependency| {
                dependency.id == format!("{}:{}", reference.kind, reference.id)
                    && dependency.revision.as_deref() == Some(reference.revision.as_str())
                    && dependency.hash.as_deref() == Some(reference.lock_sha256.as_str())
            })
        });
        if !dependency_matches {
            return false;
        }
        match reference.kind {
            LockKind::Subject => {
                if let Some(subject) = &entry.subject {
                    if subject.id == reference.id {
                        return subject.revision == reference.revision;
                    }
                }
                match entry
                    .locks
                    .as_ref()
                    .and_then(|locks| locks.subject.as_ref())
                {
                    Some(revision) => *revision == reference.revision,
                    None => false,
                }
            }
            LockKind::Style => {
                recorded_style_revision(entry).as_deref() == Some(reference.revision.as_str())
            }
        }
    })
}

fn recorded_provider_matches(entry: &ProjectCatalogEntryV2, provider: &GameArtProviderV1) -> bool {
    entry.provider.as_ref().is_some_and(|recorded| {
        recorded.provider_id == provider.id && recorded.profile_id == provider.profile_id
    })
}

fn expected_workflow(kind: super::types::AssetKind) -> &'static str {
    match kind {
        super::types::AssetKind::Character => "topdown@1.0.0",
        super::types::AssetKind::IconSet | super::types::AssetKind::PropSet => "static-set@1.0.0",
    }
}

fn recorded_asset_dependencies_match(
    entry: &ProjectCatalogEntryV2,
    expected: &[String],
    catalog: &crate::catalog::ProjectCatalogV2,
) -> bool {
    let Some(recorded) = &entry.dependencies else {
        return expected.is_empty();
    };
    let mut recorded_entries = recorded
        .iter()
        .filter(|dependency| {
            !dependency.id.starts_with("style:") && !dependency.id.starts_with("subject:")
        })
        .map(|dependency| (dependency.id.clone(), dependency.hash.clone()))
        .collect::<Vec<_>>();
    recorded_entries.sort();
    recorded_entries.dedup();
    let mut expected = expected.to_vec();
    expected.sort();
    expected.dedup();
    if recorded_entries
        .iter()
        .map(|(id, _)| id)
        .ne(expected.iter())
    {
        return false;
    }
    recorded_entries.into_iter().all(|(id, hash)| {
        catalog
            .assets
            .get(&id)
            .is_some_and(|dependency| hash.as_deref() == Some(dependency.pack_sha256.as_str()))
    })
}

enum PackVerdict {
    Intact,
    Missing,
    HashMismatch,
    Invalid,
}

/// Re-verify the catalog entry's pack on disk. Pack paths may be absolute or
/// project-relative; directories are hashed with the same algorithm the
/// automation runner used at registration time, plain files by content hash.
fn verify_pack(
    project_root: &Path,
    entry: &ProjectCatalogEntryV2,
) -> Result<PackVerdict, GameArtError> {
    let pack_path = resolve_relative(project_root, &entry.pack_path);
    let link_metadata = match fs::symlink_metadata(&pack_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(PackVerdict::Missing);
        }
        Err(error) => {
            return Err(GameArtError::Io(format!(
                "cannot stat pack {} for asset \"{}\": {error}",
                pack_path.display(),
                entry.asset_id
            )));
        }
    };
    if link_metadata.file_type().is_symlink() {
        return Ok(PackVerdict::Invalid);
    }
    let metadata = match fs::metadata(&pack_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(PackVerdict::Missing);
        }
        Err(error) => {
            return Err(GameArtError::Io(format!(
                "cannot stat pack {} for asset \"{}\": {error}",
                pack_path.display(),
                entry.asset_id
            )));
        }
    };
    if !metadata.is_dir() || forge_pack::validate_pack_layout(&pack_path).is_err() {
        return Ok(PackVerdict::Invalid);
    }
    let actual = hash_pack(&pack_path, metadata.is_dir()).map_err(|error| {
        GameArtError::Io(format!(
            "cannot hash pack {} for asset \"{}\": {error}",
            pack_path.display(),
            entry.asset_id
        ))
    })?;
    Ok(if actual == entry.pack_sha256 {
        PackVerdict::Intact
    } else {
        PackVerdict::HashMismatch
    })
}

/// Content hash of a pack. Directory entries are framed with domain, type,
/// path length and content length so different directory structures cannot
/// collide. Symbolic links and non-file entries are rejected rather than
/// silently disappearing from provenance.
pub(crate) fn hash_pack(path: &Path, is_directory: bool) -> Result<String, std::io::Error> {
    if fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("pack root is a symbolic link: {}", path.display()),
        ));
    }
    if !is_directory {
        return Ok(format!("{:x}", Sha256::digest(fs::read(path)?)));
    }
    let mut relative_paths = Vec::new();
    collect_pack_files(path, path, &mut relative_paths)?;
    relative_paths.sort();
    let mut hasher = Sha256::new();
    hasher.update(b"forge-directory-hash-v2\0");
    for relative in relative_paths {
        let relative = relative.to_string_lossy();
        let contents = fs::read(path.join(relative.as_ref()))?;
        hasher.update(b"file\0");
        hasher.update((relative.len() as u64).to_le_bytes());
        hasher.update(relative.as_bytes());
        hasher.update((contents.len() as u64).to_le_bytes());
        hasher.update(contents);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
pub(crate) fn write_test_pack_fixture(pack: &Path, id: &str) {
    use serde_json::json;

    fs::create_dir_all(pack.join("previews")).unwrap();
    fs::create_dir_all(pack.join("assets/frames")).unwrap();
    fs::write(pack.join("previews/preview.gif"), b"GIF89a").unwrap();
    fs::write(pack.join("assets/sprite_sheet.png"), b"png").unwrap();
    fs::write(pack.join("assets/frames/frame_001.png"), b"png").unwrap();
    fs::write(
        pack.join("assets/atlas.json"),
        json!({
            "image":"sprite_sheet.png", "frameWidth":16, "frameHeight":16,
            "columns":1, "rows":1,
            "frames":[{"index":0,"name":"frame_001.png","x":0,"y":0,"width":16,"height":16}]
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        pack.join("assets/manifest.json"),
        json!({
            "name":id,
            "sheet":{"image":"assets/sprite_sheet.png","frameWidth":16,"frameHeight":16,"columns":1,"rows":1},
            "animations":[{"name":"idle","frames":[0],"fps":12.0,"loop":true}],
            "anchor":{"type":"feet","x":8.0,"y":16.0}
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        pack.join("quality-report.json"),
        json!({
            "verdict":"game_ready",
            "metrics":{"bboxBottomDriftPx":0.0,"bboxCenterXDriftPx":0.0,"bboxCenterYDriftPx":0.0,"bboxWidthVariationPx":0.0,"alphaCoverageAvg":0.25,"loopMatchScore":1.0,"frameCount":1,"frameSizeConsistent":true,"cellBoundarySafe":true},
            "recommendations":[],"notes":[]
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        pack.join("forgepack.json"),
        json!({
            "schemaVersion":"1.0.0","id":id,"name":id,"version":"0.1.0",
            "createdAt":"2026-08-05T00:00:00Z","creator":{"name":"Forge"},
            "license":{"type":"private"},"source":{"kind":"import_frames"},
            "animations":[{"name":"idle","frames":[0],"fps":12.0,"loop":true}],
            "assets":{"frames":"assets/frames","spriteSheet":"assets/sprite_sheet.png","atlas":"assets/atlas.json","manifest":"assets/manifest.json","qualityReport":"quality-report.json"},
            "previews":{"gif":"previews/preview.gif"}
        })
        .to_string(),
    )
    .unwrap();
}

fn collect_pack_files(
    root: &Path,
    directory: &Path,
    paths: &mut Vec<PathBuf>,
) -> Result<(), std::io::Error> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("pack contains symbolic link: {}", entry.path().display()),
            ));
        } else if file_type.is_dir() {
            collect_pack_files(root, &entry.path(), paths)?;
        } else if file_type.is_file() {
            paths.push(
                entry
                    .path()
                    .strip_prefix(root)
                    .unwrap_or(&entry.path())
                    .to_path_buf(),
            );
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "pack contains unsupported entry: {}",
                    entry.path().display()
                ),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_project::{ForgeProjectV1, ProviderSelection, FORGE_PROJECT_FILE};
    use crate::catalog::{
        read_project_catalog, register_catalog_asset_v2, CatalogDependencyRefV1,
        CatalogLockRevisionsV1, CatalogProviderRefV1, CatalogStyleRefV1, CatalogSubjectRefV1,
    };
    use crate::game_art::GameArtManifestV1;
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

    fn write_project(root: &Path, current_style_revision: Option<&str>) {
        let project = ForgeProjectV1 {
            schema_version: "1".into(),
            project_id: "forest-rpg".into(),
            name: "Forest RPG".into(),
            provider: ProviderSelection {
                id: "xai".into(),
                profile_id: "default".into(),
            },
            output_dir: PathBuf::from("build"),
            current_style_revision: current_style_revision.map(str::to_string),
            current_environment_revision: None,
        };
        fs::write(
            root.join(FORGE_PROJECT_FILE),
            serde_json::to_vec_pretty(&project).unwrap(),
        )
        .unwrap();
    }

    fn write_style_lock(root: &Path, revision: &str) {
        let directory = root.join(".forge/styles").join(revision);
        fs::create_dir_all(&directory).unwrap();
        let board = directory.join("style-board.png");
        fs::write(&board, b"style-board").unwrap();
        fs::write(
            directory.join(STYLE_LOCK_FILE),
            json!({
                "schemaVersion": "1",
                "revision": revision,
                "providerId": "xai",
                "profileId": "default",
                "imageModel": "grok-image",
                "prompt": "test style",
                "perspective": "topdown",
                "lighting": "upper_left",
                "outline": "dark",
                "background": "transparent",
                "sampling": "nearest",
                "characterCanvasSize": 256,
                "iconCanvasSize": 128,
                "propCanvasSize": 256,
                "boardPath": board,
                "boardSha256": format!("{:x}", Sha256::digest(b"style-board")),
                "referenceSha256": [],
                "baselineProfile": "style-baseline@2.3.0",
                "baseline": {
                    "palette": [{"color": "#8250D2", "weight": 1.0}],
                    "edgeDensity": 0.25,
                    "foregroundScale": 0.5,
                    "perceptualHash": "0000000000000000"
                }
            })
            .to_string(),
        )
        .unwrap();
    }

    fn write_subject_lock(root: &Path, id: &str, revision: &str, recorded_revision: &str) {
        let directory = root.join(".forge/subjects").join(id).join(revision);
        fs::create_dir_all(&directory).unwrap();
        let canonical = directory.join("canonical.png");
        let mask = directory.join("mask.png");
        fs::write(&canonical, b"canonical").unwrap();
        fs::write(&mask, b"mask").unwrap();
        fs::write(
            directory.join("subject-lock.json"),
            json!({
                "schemaVersion": "1",
                "profile": "subject-lock@1.0.0",
                "id": id,
                "name": id,
                "revision": recorded_revision,
                "createdAt": "2026-08-05T00:00:00Z",
                "prompt": "test subject",
                "license": "private",
                "styleRevision": "style-rev-1",
                "styleSha256": "a".repeat(64),
                "providerId": "xai",
                "profileId": "default",
                "imageModel": "grok-image",
                "canonicalPath": canonical,
                "canonicalSha256": format!("{:x}", Sha256::digest(b"canonical")),
                "maskPath": mask,
                "maskSha256": format!("{:x}", Sha256::digest(b"mask")),
                "referenceSha256": [],
                "baseline": {
                    "perceptualHash": "0000000000000000",
                    "foregroundScale": 0.5,
                    "edgeDensity": 0.25,
                    "anchorX": 8.0,
                    "anchorY": 16.0,
                    "subjectCount": 1
                }
            })
            .to_string(),
        )
        .unwrap();
    }

    fn write_pack(root: &Path, relative: &str, files: &[(&str, &[u8])]) -> String {
        let pack_dir = root.join(relative);
        write_test_pack_fixture(
            &pack_dir,
            pack_dir
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("test-pack"),
        );
        for (name, contents) in files {
            fs::write(pack_dir.join(name), contents).unwrap();
        }
        hash_pack(&pack_dir, true).unwrap()
    }

    fn fixed_time() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    /// Catalog entry matching a fresh build against style revision `style-rev-1`
    /// and the xai/default provider; mutate fields per test.
    fn catalog_entry(asset_id: &str, kind: &str, workflow: &str) -> ProjectCatalogEntryV2 {
        ProjectCatalogEntryV2 {
            asset_id: asset_id.into(),
            name: format!("Asset {asset_id}"),
            kind: kind.into(),
            pack_path: PathBuf::from(format!("packs/{asset_id}")),
            pack_sha256: "0".repeat(64),
            source_job_id: "job-1".into(),
            parent_job_id: None,
            style: Some(CatalogStyleRefV1 {
                revision: "style-rev-1".into(),
            }),
            subject: None,
            workflow: workflow.into(),
            provider: Some(CatalogProviderRefV1 {
                provider_id: "xai".into(),
                profile_id: "default".into(),
                model: Some("grok-image".into()),
            }),
            installed: None,
            created_at: fixed_time(),
            spec_path: Some(PathBuf::from(format!("specs/{asset_id}.json"))),
            spec_sha256: None,
            dependencies: None,
            locks: Some(CatalogLockRevisionsV1 {
                style: Some("style-rev-1".into()),
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
        }
    }

    fn manifest_json(assets: &str, style_revision: Option<&str>) -> String {
        let style_line = style_revision
            .map(|revision| format!(r#", "styleRevision": "{revision}""#))
            .unwrap_or_default();
        format!(
            r#"{{
                "schemaVersion": "1",
                "kind": "game_art_manifest",
                "projectId": "forest-rpg",
                "name": "Forest RPG"{style_line},
                "provider": {{ "id": "xai", "profileId": "default" }},
                "defaults": {{
                    "outputDirectory": "packs",
                    "godotRoot": "addons/forge_assets",
                    "license": "private"
                }},
                "assets": [ {assets} ]
            }}"#
        )
    }

    fn asset_json(id: &str, kind: &str, extra: &str) -> String {
        let separator = if extra.is_empty() { "" } else { ", " };
        format!(
            r#"{{ "id": "{id}", "kind": "{kind}", "spec": "specs/{id}.json"{separator}{extra} }}"#
        )
    }

    /// Project with a style lock at `style-rev-1`, the given specs on disk and
    /// a manifest referencing them; returns the validated manifest.
    fn setup_project(
        root: &Path,
        assets: &str,
        specs: &[(&str, &str)],
        style_revision: Option<&str>,
    ) -> ValidatedManifest {
        write_project(root, Some("style-rev-1"));
        write_style_lock(root, "style-rev-1");
        fs::create_dir_all(root.join("specs")).unwrap();
        for (id, contents) in specs {
            fs::write(root.join("specs").join(format!("{id}.json")), contents).unwrap();
        }
        let manifest_path = root.join("game-art.json");
        fs::write(&manifest_path, manifest_json(assets, style_revision)).unwrap();
        GameArtManifestV1::load_validated(&manifest_path).unwrap()
    }

    /// Register a catalog entry whose spec hash and pack hash match the current
    /// on-disk spec/pack, i.e. a faithful record of a completed build. A
    /// `subject_revision` records the Subject Lock the build ran against.
    fn register_faithful_entry(
        root: &Path,
        validated: &ValidatedManifest,
        asset_id: &str,
        kind: &str,
        workflow: &str,
        subject_revision: Option<&str>,
        files: &[(&str, &[u8])],
    ) {
        let mut entry = catalog_entry(asset_id, kind, workflow);
        entry.spec_sha256 = Some(validated.asset(asset_id).unwrap().spec_sha256.clone());
        entry.pack_sha256 = write_pack(root, &format!("packs/{asset_id}"), files);
        let catalog = read_project_catalog(root).unwrap();
        let manifest_asset = validated
            .manifest
            .assets
            .iter()
            .find(|asset| asset.id == asset_id)
            .unwrap();
        let mut dependencies = manifest_asset
            .depends_on
            .iter()
            .filter(|dependency| !dependency.contains(':'))
            .map(|dependency| CatalogDependencyRefV1 {
                id: dependency.clone(),
                revision: None,
                hash: catalog
                    .assets
                    .get(dependency)
                    .map(|entry| entry.pack_sha256.clone()),
            })
            .collect::<Vec<_>>();
        if let Some(revision) = subject_revision {
            entry.subject = Some(CatalogSubjectRefV1 {
                id: asset_id.into(),
                revision: revision.into(),
            });
            entry.locks = Some(CatalogLockRevisionsV1 {
                style: Some("style-rev-1".into()),
                subject: Some(revision.into()),
                ..CatalogLockRevisionsV1::default()
            });
            dependencies.push(CatalogDependencyRefV1 {
                id: format!("subject:{asset_id}"),
                revision: Some(revision.into()),
                hash: Some(
                    semantic_lock_sha256(
                        root,
                        &root
                            .join(".forge/subjects")
                            .join(asset_id)
                            .join(revision)
                            .join("subject-lock.json"),
                    )
                    .unwrap(),
                ),
            });
        }
        entry.dependencies = Some(dependencies);
        register_catalog_asset_v2(root, entry).unwrap();
    }

    fn action_of<'a>(diff: &'a ProjectDiffV1, asset_id: &str) -> &'a AssetDiffActionV1 {
        diff.actions
            .iter()
            .find(|action| action.asset_id == asset_id)
            .unwrap_or_else(|| panic!("no diff action for {asset_id}"))
    }

    #[test]
    fn reuse_when_nothing_changed() {
        let temp = tempfile::tempdir().unwrap();
        let validated = setup_project(
            temp.path(),
            &format!(
                "{}, {}",
                asset_json(
                    "hero",
                    "character",
                    r#""dependsOn": ["subject:hero@subject-rev-1", "hud-icons"]"#
                ),
                asset_json("hud-icons", "icon_set", "")
            ),
            &[
                ("hero", CHARACTER_SPEC),
                (
                    "hud-icons",
                    &static_spec("hud-icons", "icon_set", &["coin"]),
                ),
            ],
            None,
        );
        write_subject_lock(temp.path(), "hero", "subject-rev-1", "subject-rev-1");
        register_faithful_entry(
            temp.path(),
            &validated,
            "hud-icons",
            "icon_set",
            "static-set@1.0.0",
            None,
            &[("pack.json", b"{}")],
        );
        register_faithful_entry(
            temp.path(),
            &validated,
            "hero",
            "character",
            "topdown@1.0.0",
            Some("subject-rev-1"),
            &[("pack.json", b"{}")],
        );

        let diff = compute_project_diff(temp.path(), &validated).unwrap();
        assert_eq!(diff.schema_version, "1");
        assert_eq!(diff.style_revision.as_deref(), Some("style-rev-1"));
        assert_eq!(diff.provider.id, "xai");
        assert_eq!(diff.provider.profile_id, "default");
        assert_eq!(diff.manifest_sha256, validated.manifest.manifest_sha256());
        assert_eq!(diff.graph_sha256, validated.manifest.graph_sha256());
        assert!(diff.delete_candidates.is_empty());
        assert_eq!(diff.actions.len(), 2);
        // Dependencies come first: hud-icons before hero.
        assert_eq!(diff.actions[0].asset_id, "hud-icons");
        assert_eq!(diff.actions[1].asset_id, "hero");
        for action in &diff.actions {
            assert_eq!(action.action, DiffActionKindV1::Reuse, "{action:?}");
            assert!(action.reasons.is_empty());
        }
        let hero = action_of(&diff, "hero");
        assert_eq!(hero.kind, "character");
        assert_eq!(hero.depends_on_assets, vec!["hud-icons".to_string()]);
        assert_eq!(hero.lock_refs.len(), 1);
        let lock = &hero.lock_refs[0];
        assert_eq!(lock.kind, LockKind::Subject);
        assert_eq!(lock.id, "hero");
        assert_eq!(lock.revision, "subject-rev-1");
        assert_eq!(lock.provider_id, "xai");
        assert_eq!(lock.profile_id, "default");
        assert_eq!(
            hero.spec_sha256.as_deref(),
            Some(validated.asset("hero").unwrap().spec_sha256.as_str())
        );
    }

    #[test]
    fn rebuild_on_spec_change() {
        let temp = tempfile::tempdir().unwrap();
        let validated = setup_project(
            temp.path(),
            &asset_json("hud-icons", "icon_set", ""),
            &[(
                "hud-icons",
                &static_spec("hud-icons", "icon_set", &["coin"]),
            )],
            None,
        );
        register_faithful_entry(
            temp.path(),
            &validated,
            "hud-icons",
            "icon_set",
            "static-set@1.0.0",
            None,
            &[("pack.json", b"{}")],
        );
        // Rewrite the spec after the catalog entry was registered.
        fs::write(
            temp.path().join("specs/hud-icons.json"),
            static_spec("hud-icons", "icon_set", &["silver coin"]),
        )
        .unwrap();
        let manifest_path = temp.path().join("game-art.json");
        let validated = GameArtManifestV1::load_validated(&manifest_path).unwrap();

        let diff = compute_project_diff(temp.path(), &validated).unwrap();
        let action = action_of(&diff, "hud-icons");
        assert_eq!(action.action, DiffActionKindV1::Rebuild);
        assert_eq!(action.reasons, vec![reasons::SPEC_CHANGED.to_string()]);
    }

    #[test]
    fn dependency_rebuilt_propagates_through_chain() {
        let temp = tempfile::tempdir().unwrap();
        // a depends on b, b depends on c.
        let assets = format!(
            "{}, {}, {}",
            asset_json("a", "character", r#""dependsOn": ["b"]"#),
            asset_json("b", "icon_set", r#""dependsOn": ["c"]"#),
            asset_json("c", "prop_set", "")
        );
        let validated = setup_project(
            temp.path(),
            &assets,
            &[
                ("a", CHARACTER_SPEC),
                ("b", &static_spec("b", "icon_set", &["coin"])),
                ("c", &static_spec("c", "prop_set", &["crate"])),
            ],
            None,
        );
        for (id, kind, workflow) in [
            ("c", "prop_set", "static-set@1.0.0"),
            ("b", "icon_set", "static-set@1.0.0"),
            ("a", "character", "topdown@1.0.0"),
        ] {
            register_faithful_entry(
                temp.path(),
                &validated,
                id,
                kind,
                workflow,
                None,
                &[("p", b"x")],
            );
        }
        // Only c's spec changes.
        fs::write(
            temp.path().join("specs/c.json"),
            static_spec("c", "prop_set", &["barrel"]),
        )
        .unwrap();
        let manifest_path = temp.path().join("game-art.json");
        let validated = GameArtManifestV1::load_validated(&manifest_path).unwrap();

        let diff = compute_project_diff(temp.path(), &validated).unwrap();
        assert_eq!(
            diff.actions
                .iter()
                .map(|action| action.asset_id.as_str())
                .collect::<Vec<_>>(),
            vec!["c", "b", "a"]
        );
        let c = action_of(&diff, "c");
        assert_eq!(c.action, DiffActionKindV1::Rebuild);
        assert_eq!(c.reasons, vec![reasons::SPEC_CHANGED.to_string()]);
        for id in ["b", "a"] {
            let action = action_of(&diff, id);
            assert_eq!(action.action, DiffActionKindV1::Rebuild, "{id}");
            assert_eq!(
                action.reasons,
                vec![reasons::DEPENDENCY_REBUILT.to_string()],
                "{id}"
            );
        }
    }

    #[test]
    fn pack_missing_forces_rebuild() {
        let temp = tempfile::tempdir().unwrap();
        let validated = setup_project(
            temp.path(),
            &asset_json("hud-icons", "icon_set", ""),
            &[(
                "hud-icons",
                &static_spec("hud-icons", "icon_set", &["coin"]),
            )],
            None,
        );
        let mut entry = catalog_entry("hud-icons", "icon_set", "static-set@1.0.0");
        entry.spec_sha256 = Some(validated.asset("hud-icons").unwrap().spec_sha256.clone());
        entry.pack_sha256 = "a".repeat(64);
        entry.pack_path = PathBuf::from("packs/does-not-exist");
        register_catalog_asset_v2(temp.path(), entry).unwrap();

        let diff = compute_project_diff(temp.path(), &validated).unwrap();
        let action = action_of(&diff, "hud-icons");
        assert_eq!(action.action, DiffActionKindV1::Rebuild);
        assert_eq!(action.reasons, vec![reasons::PACK_MISSING.to_string()]);
    }

    #[test]
    fn pack_hash_mismatch_forces_rebuild() {
        let temp = tempfile::tempdir().unwrap();
        let validated = setup_project(
            temp.path(),
            &asset_json("hud-icons", "icon_set", ""),
            &[(
                "hud-icons",
                &static_spec("hud-icons", "icon_set", &["coin"]),
            )],
            None,
        );
        register_faithful_entry(
            temp.path(),
            &validated,
            "hud-icons",
            "icon_set",
            "static-set@1.0.0",
            None,
            &[("pack.json", b"original")],
        );
        // Tamper with the pack after registration.
        fs::write(temp.path().join("packs/hud-icons/pack.json"), b"tampered").unwrap();

        let diff = compute_project_diff(temp.path(), &validated).unwrap();
        let action = action_of(&diff, "hud-icons");
        assert_eq!(action.action, DiffActionKindV1::Rebuild);
        assert_eq!(
            action.reasons,
            vec![reasons::PACK_HASH_MISMATCH.to_string()]
        );
    }

    #[test]
    fn ordinary_file_cannot_satisfy_a_catalog_pack_entry() {
        let temp = tempfile::tempdir().unwrap();
        let validated = setup_project(
            temp.path(),
            &asset_json("hud-icons", "icon_set", ""),
            &[(
                "hud-icons",
                &static_spec("hud-icons", "icon_set", &["coin"]),
            )],
            None,
        );
        register_faithful_entry(
            temp.path(),
            &validated,
            "hud-icons",
            "icon_set",
            "static-set@1.0.0",
            None,
            &[],
        );
        let pack = temp.path().join("packs/hud-icons");
        fs::remove_dir_all(&pack).unwrap();
        fs::write(&pack, b"not a pack").unwrap();

        let action = action_of(
            &compute_project_diff(temp.path(), &validated).unwrap(),
            "hud-icons",
        )
        .clone();
        assert_eq!(action.action, DiffActionKindV1::Rebuild);
        assert_eq!(action.reasons, vec![reasons::PACK_INVALID.to_string()]);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_root_cannot_satisfy_a_catalog_pack_entry() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let validated = setup_project(
            temp.path(),
            &asset_json("hud-icons", "icon_set", ""),
            &[(
                "hud-icons",
                &static_spec("hud-icons", "icon_set", &["coin"]),
            )],
            None,
        );
        register_faithful_entry(
            temp.path(),
            &validated,
            "hud-icons",
            "icon_set",
            "static-set@1.0.0",
            None,
            &[],
        );
        let pack = temp.path().join("packs/hud-icons");
        let real_pack = temp.path().join("real-pack");
        fs::rename(&pack, &real_pack).unwrap();
        symlink(&real_pack, &pack).unwrap();

        let action = action_of(
            &compute_project_diff(temp.path(), &validated).unwrap(),
            "hud-icons",
        )
        .clone();
        assert_eq!(action.action, DiffActionKindV1::Rebuild);
        assert_eq!(action.reasons, vec![reasons::PACK_INVALID.to_string()]);
    }

    #[test]
    fn orphans_reported_as_delete_candidates_only() {
        let temp = tempfile::tempdir().unwrap();
        let validated = setup_project(
            temp.path(),
            &asset_json("hud-icons", "icon_set", ""),
            &[(
                "hud-icons",
                &static_spec("hud-icons", "icon_set", &["coin"]),
            )],
            None,
        );
        register_faithful_entry(
            temp.path(),
            &validated,
            "hud-icons",
            "icon_set",
            "static-set@1.0.0",
            None,
            &[("p", b"x")],
        );
        // A catalog asset the manifest no longer declares, from an older kind set.
        let mut orphan = catalog_entry("old-tileset", "environment", "terrain-set@1.0.0");
        orphan.pack_sha256 = "b".repeat(64);
        register_catalog_asset_v2(temp.path(), orphan).unwrap();

        let diff = compute_project_diff(temp.path(), &validated).unwrap();
        assert_eq!(diff.delete_candidates, vec!["old-tileset".to_string()]);
        let orphan = action_of(&diff, "old-tileset");
        assert_eq!(orphan.action, DiffActionKindV1::Orphan);
        assert_eq!(
            orphan.kind, "environment",
            "orphan keeps the raw catalog kind"
        );
        assert_eq!(orphan.reasons, vec![reasons::NOT_IN_MANIFEST.to_string()]);
        assert!(orphan.spec_sha256.is_none());
        // Manifest assets come first, orphans last.
        assert_eq!(diff.actions.last().unwrap().asset_id, "old-tileset");
    }

    #[test]
    fn topological_order_is_dependencies_first() {
        let temp = tempfile::tempdir().unwrap();
        // Diamond: a -> {b, c} -> d; b sorts before c.
        let assets = format!(
            "{}, {}, {}, {}",
            asset_json("a", "character", r#""dependsOn": ["b", "c"]"#),
            asset_json("c", "prop_set", r#""dependsOn": ["d"]"#),
            asset_json("b", "icon_set", r#""dependsOn": ["d"]"#),
            asset_json("d", "prop_set", "")
        );
        let validated = setup_project(
            temp.path(),
            &assets,
            &[
                ("a", CHARACTER_SPEC),
                ("b", &static_spec("b", "icon_set", &["coin"])),
                ("c", &static_spec("c", "prop_set", &["crate"])),
                ("d", &static_spec("d", "prop_set", &["rock"])),
            ],
            None,
        );
        assert_eq!(
            topological_build_order(&validated),
            vec![
                "d".to_string(),
                "b".to_string(),
                "c".to_string(),
                "a".to_string()
            ]
        );
    }

    #[test]
    fn missing_subject_lock_yields_unknown_lock() {
        let temp = tempfile::tempdir().unwrap();
        let validated = setup_project(
            temp.path(),
            &asset_json(
                "hero",
                "character",
                r#""dependsOn": ["subject:ghost@subject-rev-9"]"#,
            ),
            &[("hero", CHARACTER_SPEC)],
            None,
        );
        let error = compute_project_diff(temp.path(), &validated).unwrap_err();
        assert_eq!(error.code(), "unknown_lock");
        assert!(error.to_string().contains("subject:ghost@subject-rev-9"));
    }

    #[test]
    fn subject_lock_with_wrong_recorded_revision_yields_lock_revision_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let validated = setup_project(
            temp.path(),
            &asset_json(
                "hero",
                "character",
                r#""dependsOn": ["subject:hero@subject-rev-1"]"#,
            ),
            &[("hero", CHARACTER_SPEC)],
            None,
        );
        // The lock directory matches the reference but the file records another revision.
        write_subject_lock(temp.path(), "hero", "subject-rev-1", "subject-rev-2");
        let error = compute_project_diff(temp.path(), &validated).unwrap_err();
        assert_eq!(error.code(), "lock_revision_mismatch");
    }

    #[test]
    fn subject_lock_media_outside_project_is_rejected_during_diff() {
        let temp = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let validated = setup_project(
            temp.path(),
            &asset_json(
                "hero",
                "character",
                r#""dependsOn": ["subject:hero@subject-rev-1"]"#,
            ),
            &[("hero", CHARACTER_SPEC)],
            None,
        );
        write_subject_lock(temp.path(), "hero", "subject-rev-1", "subject-rev-1");
        let external_canonical = external.path().join("canonical.png");
        fs::write(&external_canonical, b"canonical").unwrap();
        let lock_path = temp
            .path()
            .join(".forge/subjects/hero/subject-rev-1/subject-lock.json");
        let mut lock: serde_json::Value =
            serde_json::from_slice(&fs::read(&lock_path).unwrap()).unwrap();
        lock["canonicalPath"] = json!(external_canonical);
        fs::write(&lock_path, serde_json::to_vec(&lock).unwrap()).unwrap();

        let error = compute_project_diff(temp.path(), &validated).unwrap_err();
        assert!(matches!(error, GameArtError::SymlinkEscape(_)));
    }

    #[test]
    fn style_lock_board_outside_project_is_rejected_during_diff() {
        let temp = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let validated = setup_project(
            temp.path(),
            &asset_json("hud-icons", "icon_set", ""),
            &[(
                "hud-icons",
                &static_spec("hud-icons", "icon_set", &["coin"]),
            )],
            None,
        );
        let external_board = external.path().join("style-board.png");
        fs::write(&external_board, b"external-style").unwrap();
        let lock_path = temp
            .path()
            .join(".forge/styles/style-rev-1/style-lock.json");
        let mut lock: serde_json::Value =
            serde_json::from_slice(&fs::read(&lock_path).unwrap()).unwrap();
        lock["boardPath"] = json!(external_board);
        lock["boardSha256"] = json!(format!("{:x}", Sha256::digest(b"external-style")));
        fs::write(&lock_path, serde_json::to_vec(&lock).unwrap()).unwrap();

        let error = compute_project_diff(temp.path(), &validated).unwrap_err();
        assert!(matches!(error, GameArtError::SymlinkEscape(_)));
    }

    #[test]
    fn missing_manifest_style_lock_yields_unknown_lock() {
        let temp = tempfile::tempdir().unwrap();
        write_project(temp.path(), None);
        fs::create_dir_all(temp.path().join("specs")).unwrap();
        fs::write(temp.path().join("specs/hero.json"), CHARACTER_SPEC).unwrap();
        let manifest_path = temp.path().join("game-art.json");
        fs::write(
            &manifest_path,
            manifest_json(
                &asset_json("hero", "character", ""),
                Some("style-rev-missing"),
            ),
        )
        .unwrap();
        let validated = GameArtManifestV1::load_validated(&manifest_path).unwrap();
        let error = compute_project_diff(temp.path(), &validated).unwrap_err();
        assert_eq!(error.code(), "unknown_lock");
        assert!(error.to_string().contains("style-rev-missing"));
    }

    #[test]
    fn new_asset_builds_when_catalog_entry_absent() {
        let temp = tempfile::tempdir().unwrap();
        let validated = setup_project(
            temp.path(),
            &asset_json("hero", "character", ""),
            &[("hero", CHARACTER_SPEC)],
            None,
        );
        // No catalog at all.
        let diff = compute_project_diff(temp.path(), &validated).unwrap();
        let action = action_of(&diff, "hero");
        assert_eq!(action.action, DiffActionKindV1::Build);
        assert_eq!(action.reasons, vec![reasons::NEW_ASSET.to_string()]);
        assert!(diff.delete_candidates.is_empty());
    }

    #[test]
    fn provider_change_forces_rebuild() {
        let temp = tempfile::tempdir().unwrap();
        let validated = setup_project(
            temp.path(),
            &asset_json("hud-icons", "icon_set", ""),
            &[(
                "hud-icons",
                &static_spec("hud-icons", "icon_set", &["coin"]),
            )],
            None,
        );
        register_faithful_entry(
            temp.path(),
            &validated,
            "hud-icons",
            "icon_set",
            "static-set@1.0.0",
            None,
            &[("p", b"x")],
        );
        // Rewrite the entry with a different provider.
        let mut entry = catalog_entry("hud-icons", "icon_set", "static-set@1.0.0");
        entry.spec_sha256 = Some(validated.asset("hud-icons").unwrap().spec_sha256.clone());
        entry.pack_sha256 = hash_pack(&temp.path().join("packs/hud-icons"), true).unwrap();
        entry.provider = Some(CatalogProviderRefV1 {
            provider_id: "other-provider".into(),
            profile_id: "default".into(),
            model: None,
        });
        register_catalog_asset_v2(temp.path(), entry).unwrap();

        let diff = compute_project_diff(temp.path(), &validated).unwrap();
        let action = action_of(&diff, "hud-icons");
        assert_eq!(action.action, DiffActionKindV1::Rebuild);
        assert_eq!(action.reasons, vec![reasons::PROVIDER_CHANGED.to_string()]);
    }

    #[test]
    fn catalog_kind_and_workflow_mismatch_force_rebuild() {
        let temp = tempfile::tempdir().unwrap();
        let validated = setup_project(
            temp.path(),
            &asset_json("hud-icons", "icon_set", ""),
            &[(
                "hud-icons",
                &static_spec("hud-icons", "icon_set", &["coin"]),
            )],
            None,
        );
        register_faithful_entry(
            temp.path(),
            &validated,
            "hud-icons",
            "icon_set",
            "static-set@1.0.0",
            None,
            &[("p", b"x")],
        );
        let mut catalog = read_project_catalog(temp.path()).unwrap();
        let entry = catalog.assets.get_mut("hud-icons").unwrap();
        entry.kind = "prop_set".into();
        entry.workflow = "topdown@1.0.0".into();
        crate::catalog::write_project_catalog(temp.path(), &catalog).unwrap();

        let action = compute_project_diff(temp.path(), &validated)
            .unwrap()
            .actions
            .into_iter()
            .find(|action| action.asset_id == "hud-icons")
            .unwrap();
        assert_eq!(action.action, DiffActionKindV1::Rebuild);
        assert!(action.reasons.contains(&reasons::KIND_CHANGED.into()));
        assert!(action.reasons.contains(&reasons::WORKFLOW_CHANGED.into()));
    }

    #[test]
    fn declared_dependency_set_change_forces_rebuild() {
        let temp = tempfile::tempdir().unwrap();
        let initial_assets = format!(
            "{}, {}",
            asset_json("hud-icons", "icon_set", ""),
            asset_json("forest-props", "prop_set", "")
        );
        let validated = setup_project(
            temp.path(),
            &initial_assets,
            &[
                (
                    "hud-icons",
                    &static_spec("hud-icons", "icon_set", &["coin"]),
                ),
                (
                    "forest-props",
                    &static_spec("forest-props", "prop_set", &["crate"]),
                ),
            ],
            None,
        );
        register_faithful_entry(
            temp.path(),
            &validated,
            "forest-props",
            "prop_set",
            "static-set@1.0.0",
            None,
            &[("p", b"x")],
        );
        register_faithful_entry(
            temp.path(),
            &validated,
            "hud-icons",
            "icon_set",
            "static-set@1.0.0",
            None,
            &[("p", b"x")],
        );
        let changed_assets = format!(
            "{}, {}",
            asset_json("hud-icons", "icon_set", r#""dependsOn": ["forest-props"]"#,),
            asset_json("forest-props", "prop_set", "")
        );
        fs::write(
            temp.path().join("game-art.json"),
            manifest_json(&changed_assets, None),
        )
        .unwrap();
        let changed =
            GameArtManifestV1::load_validated(&temp.path().join("game-art.json")).unwrap();
        let action = compute_project_diff(temp.path(), &changed)
            .unwrap()
            .actions
            .into_iter()
            .find(|action| action.asset_id == "hud-icons")
            .unwrap();
        assert_eq!(action.action, DiffActionKindV1::Rebuild);
        assert!(action
            .reasons
            .contains(&reasons::DEPENDENCIES_CHANGED.into()));
    }

    #[test]
    fn style_revision_change_forces_rebuild() {
        let temp = tempfile::tempdir().unwrap();
        let validated = setup_project(
            temp.path(),
            &asset_json("hud-icons", "icon_set", ""),
            &[(
                "hud-icons",
                &static_spec("hud-icons", "icon_set", &["coin"]),
            )],
            None,
        );
        register_faithful_entry(
            temp.path(),
            &validated,
            "hud-icons",
            "icon_set",
            "static-set@1.0.0",
            None,
            &[("p", b"x")],
        );
        // Project moved to a new style revision after the entry was registered.
        write_project(temp.path(), Some("style-rev-2"));
        write_style_lock(temp.path(), "style-rev-2");

        let diff = compute_project_diff(temp.path(), &validated).unwrap();
        let action = action_of(&diff, "hud-icons");
        assert_eq!(action.action, DiffActionKindV1::Rebuild);
        assert_eq!(
            action.reasons,
            vec![reasons::STYLE_REVISION_CHANGED.to_string()]
        );
    }

    #[test]
    fn subject_lock_revision_change_forces_rebuild() {
        let temp = tempfile::tempdir().unwrap();
        let validated = setup_project(
            temp.path(),
            &asset_json(
                "hero",
                "character",
                r#""dependsOn": ["subject:hero@subject-rev-2"]"#,
            ),
            &[("hero", CHARACTER_SPEC)],
            None,
        );
        write_subject_lock(temp.path(), "hero", "subject-rev-2", "subject-rev-2");
        register_faithful_entry(
            temp.path(),
            &validated,
            "hero",
            "character",
            "topdown@1.0.0",
            Some("subject-rev-2"),
            &[("p", b"x")],
        );
        // Roll the entry back to the older subject revision.
        let mut entry = catalog_entry("hero", "character", "topdown@1.0.0");
        entry.spec_sha256 = Some(validated.asset("hero").unwrap().spec_sha256.clone());
        entry.pack_sha256 = hash_pack(&temp.path().join("packs/hero"), true).unwrap();
        entry.subject = Some(CatalogSubjectRefV1 {
            id: "hero".into(),
            revision: "subject-rev-1".into(),
        });
        entry.locks = Some(CatalogLockRevisionsV1 {
            style: Some("style-rev-1".into()),
            subject: Some("subject-rev-1".into()),
            ..CatalogLockRevisionsV1::default()
        });
        register_catalog_asset_v2(temp.path(), entry).unwrap();

        let diff = compute_project_diff(temp.path(), &validated).unwrap();
        let action = action_of(&diff, "hero");
        assert_eq!(action.action, DiffActionKindV1::Rebuild);
        assert_eq!(
            action.reasons,
            vec![reasons::LOCK_REVISION_CHANGED.to_string()]
        );
    }

    #[test]
    fn removing_subject_lock_dependency_forces_rebuild() {
        let temp = tempfile::tempdir().unwrap();
        let validated = setup_project(
            temp.path(),
            &asset_json(
                "hero",
                "character",
                r#""dependsOn": ["subject:hero@subject-rev-1"]"#,
            ),
            &[("hero", CHARACTER_SPEC)],
            None,
        );
        write_subject_lock(temp.path(), "hero", "subject-rev-1", "subject-rev-1");
        register_faithful_entry(
            temp.path(),
            &validated,
            "hero",
            "character",
            "topdown@1.0.0",
            Some("subject-rev-1"),
            &[("p", b"x")],
        );
        fs::write(
            temp.path().join("game-art.json"),
            manifest_json(&asset_json("hero", "character", ""), None),
        )
        .unwrap();
        let changed =
            GameArtManifestV1::load_validated(&temp.path().join("game-art.json")).unwrap();
        let action = action_of(
            &compute_project_diff(temp.path(), &changed).unwrap(),
            "hero",
        )
        .clone();
        assert_eq!(action.action, DiffActionKindV1::Rebuild);
        assert!(action
            .reasons
            .contains(&reasons::LOCK_REVISION_CHANGED.into()));
    }

    #[test]
    fn removing_effective_style_revision_forces_rebuild() {
        let temp = tempfile::tempdir().unwrap();
        let validated = setup_project(
            temp.path(),
            &asset_json("hud-icons", "icon_set", ""),
            &[(
                "hud-icons",
                &static_spec("hud-icons", "icon_set", &["coin"]),
            )],
            None,
        );
        register_faithful_entry(
            temp.path(),
            &validated,
            "hud-icons",
            "icon_set",
            "static-set@1.0.0",
            None,
            &[("p", b"x")],
        );
        write_project(temp.path(), None);
        let action = action_of(
            &compute_project_diff(temp.path(), &validated).unwrap(),
            "hud-icons",
        )
        .clone();
        assert_eq!(action.action, DiffActionKindV1::Rebuild);
        assert!(action
            .reasons
            .contains(&reasons::STYLE_REVISION_CHANGED.into()));
    }

    #[test]
    fn pack_directory_hash_matches_runner_algorithm() {
        // Pin the framed single-file case so the diff and runner algorithms
        // cannot silently drift back to ambiguous path/content concatenation.
        let temp = tempfile::tempdir().unwrap();
        let pack = temp.path().join("pack");
        fs::create_dir_all(&pack).unwrap();
        fs::write(pack.join("a.txt"), b"x").unwrap();
        let mut expected = Sha256::new();
        expected.update(b"forge-directory-hash-v2\0");
        expected.update(b"file\0");
        expected.update(5_u64.to_le_bytes());
        expected.update(b"a.txt");
        expected.update(1_u64.to_le_bytes());
        expected.update(b"x");
        assert_eq!(
            hash_pack(&pack, true).unwrap(),
            format!("{:x}", expected.finalize())
        );
        // Nested files retain the same framed representation in sorted order.
        fs::create_dir_all(pack.join("sub")).unwrap();
        fs::write(pack.join("sub/b.txt"), b"y").unwrap();
        let expected = {
            let mut hasher = Sha256::new();
            hasher.update(b"forge-directory-hash-v2\0");
            for (path, contents) in [("a.txt", b"x".as_slice()), ("sub/b.txt", b"y".as_slice())] {
                hasher.update(b"file\0");
                hasher.update((path.len() as u64).to_le_bytes());
                hasher.update(path.as_bytes());
                hasher.update((contents.len() as u64).to_le_bytes());
                hasher.update(contents);
            }
            format!("{:x}", hasher.finalize())
        };
        assert_eq!(hash_pack(&pack, true).unwrap(), expected);
    }

    #[test]
    fn pack_directory_hash_frames_paths_and_contents() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        fs::write(first.path().join("a"), b"bc").unwrap();
        fs::write(second.path().join("ab"), b"c").unwrap();
        assert_ne!(
            hash_pack(first.path(), true).unwrap(),
            hash_pack(second.path(), true).unwrap(),
            "path/content concatenation must not create a structural collision"
        );
    }

    #[cfg(unix)]
    #[test]
    fn pack_directory_hash_rejects_symbolic_links() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("real"), b"x").unwrap();
        symlink(temp.path().join("real"), temp.path().join("alias")).unwrap();
        let error = hash_pack(temp.path(), true).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }

    #[cfg(unix)]
    #[test]
    fn pack_directory_hash_rejects_symlink_root() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let real = temp.path().join("real");
        fs::create_dir(&real).unwrap();
        fs::write(real.join("pack.json"), b"{}").unwrap();
        let linked = temp.path().join("linked");
        symlink(&real, &linked).unwrap();
        let error = hash_pack(&linked, true).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_pack_fixture_is_a_valid_pack() {
        let temp = tempfile::tempdir().unwrap();
        let pack = temp.path().join("fixture.gsfpack");
        write_test_pack_fixture(&pack, "fixture");

        forge_pack::validate_pack_layout(&pack).unwrap();
    }

    #[test]
    fn manifest_style_revision_overrides_project_current() {
        let temp = tempfile::tempdir().unwrap();
        // Project current is rev-1, the manifest pins rev-2; the entry was
        // built against rev-2 and must be reused.
        let validated = setup_project(
            temp.path(),
            &asset_json("hud-icons", "icon_set", ""),
            &[(
                "hud-icons",
                &static_spec("hud-icons", "icon_set", &["coin"]),
            )],
            Some("style-rev-2"),
        );
        write_style_lock(temp.path(), "style-rev-2");
        let mut entry = catalog_entry("hud-icons", "icon_set", "static-set@1.0.0");
        entry.spec_sha256 = Some(validated.asset("hud-icons").unwrap().spec_sha256.clone());
        entry.pack_sha256 = write_pack(temp.path(), "packs/hud-icons", &[("p", b"x")]);
        entry.style = Some(CatalogStyleRefV1 {
            revision: "style-rev-2".into(),
        });
        entry.locks = Some(CatalogLockRevisionsV1 {
            style: Some("style-rev-2".into()),
            ..CatalogLockRevisionsV1::default()
        });
        register_catalog_asset_v2(temp.path(), entry).unwrap();

        let diff = compute_project_diff(temp.path(), &validated).unwrap();
        assert_eq!(diff.style_revision.as_deref(), Some("style-rev-2"));
        assert_eq!(
            action_of(&diff, "hud-icons").action,
            DiffActionKindV1::Reuse
        );
    }
}
