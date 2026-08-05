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

use super::types::{GameArtError, GameArtProviderV1, LockKind, LockRef};
use super::ValidatedManifest;
use crate::asset_project::{read_project, resolve_relative, AssetProjectError, STYLE_LOCK_FILE};
use crate::catalog::{read_project_catalog, CatalogError, ProjectCatalogEntryV2};
use crate::subject::subject_lock_path;

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
    /// The recorded pack path no longer exists on disk.
    pub const PACK_MISSING: &str = "pack_missing";
    /// The pack exists but its recomputed content hash differs from the
    /// recorded `packSha256`.
    pub const PACK_HASH_MISMATCH: &str = "pack_hash_mismatch";
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

/// Tolerant read of the fields the diff layer needs from a Style Lock. Full
/// integrity verification (style board hash) stays with
/// `crate::asset_project::read_style_lock` at build time.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StyleLockHeader {
    revision: String,
    provider_id: String,
    profile_id: String,
}

/// Tolerant read of the fields the diff layer needs from a Subject Lock. Full
/// integrity verification (canonical/mask hashes) stays with
/// `crate::subject::read_subject_lock` at build time.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubjectLockHeader {
    id: String,
    revision: String,
    provider_id: String,
    profile_id: String,
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
    let style_revision = resolve_style_revision(
        project_root,
        manifest
            .style_revision
            .as_deref()
            .or(project.current_style_revision.as_deref()),
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
        let lock_refs = resolve_asset_lock_refs(project_root, &asset.depends_on, &graph)?;
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
                if entry.spec_sha256.as_deref() != Some(spec.spec_sha256.as_str()) {
                    failed.push(reasons::SPEC_CHANGED);
                }
                if let Some(style_revision) = &style_revision {
                    if recorded_style_revision(entry).as_deref() != Some(style_revision.as_str()) {
                        failed.push(reasons::STYLE_REVISION_CHANGED);
                    }
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
) -> Result<Option<String>, GameArtError> {
    let Some(revision) = style_revision else {
        return Ok(None);
    };
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
    let header: StyleLockHeader = read_lock_json(&lock_path, &format!("style@{revision}"))?;
    if header.revision != revision {
        return Err(GameArtError::LockRevisionMismatch(format!(
            "style lock at {} records revision \"{}\", expected \"{revision}\"",
            lock_path.display(),
            header.revision
        )));
    }
    Ok(Some(revision.to_string()))
}

/// Resolve every `dependsOn` lock reference of one asset against the project.
/// Declared asset ids (graph edges) are skipped; malformed references were
/// already rejected by manifest validation.
fn resolve_asset_lock_refs(
    project_root: &Path,
    depends_on: &[String],
    graph: &BTreeMap<String, Vec<String>>,
) -> Result<Vec<ResolvedLockRefV1>, GameArtError> {
    let mut resolved = Vec::new();
    for dependency in depends_on {
        if graph.contains_key(dependency) {
            continue;
        }
        let reference = LockRef::parse(dependency)?;
        resolved.push(resolve_lock_ref(project_root, &reference)?);
    }
    resolved.sort();
    resolved.dedup();
    Ok(resolved)
}

fn resolve_lock_ref(
    project_root: &Path,
    reference: &LockRef,
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
            let header: SubjectLockHeader = read_lock_json(&lock_path, &reference.to_string())?;
            if header.id != reference.id || header.revision != reference.revision {
                return Err(GameArtError::LockRevisionMismatch(format!(
                    "subject lock at {} records \"{}@{}\", expected \"{reference}\"",
                    lock_path.display(),
                    header.id,
                    header.revision
                )));
            }
            Ok(ResolvedLockRefV1 {
                kind: LockKind::Subject,
                id: reference.id.clone(),
                revision: reference.revision.clone(),
                provider_id: header.provider_id,
                profile_id: header.profile_id,
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
            let header: StyleLockHeader = read_lock_json(&lock_path, &reference.to_string())?;
            if header.revision != reference.revision {
                return Err(GameArtError::LockRevisionMismatch(format!(
                    "style lock at {} records revision \"{}\", expected \"{reference}\"",
                    lock_path.display(),
                    header.revision
                )));
            }
            Ok(ResolvedLockRefV1 {
                kind: LockKind::Style,
                id: reference.id.clone(),
                revision: reference.revision.clone(),
                provider_id: header.provider_id,
                profile_id: header.profile_id,
            })
        }
    }
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
    lock_refs.iter().all(|reference| match reference.kind {
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
    })
}

fn recorded_provider_matches(entry: &ProjectCatalogEntryV2, provider: &GameArtProviderV1) -> bool {
    entry.provider.as_ref().is_some_and(|recorded| {
        recorded.provider_id == provider.id && recorded.profile_id == provider.profile_id
    })
}

enum PackVerdict {
    Intact,
    Missing,
    HashMismatch,
}

/// Re-verify the catalog entry's pack on disk. Pack paths may be absolute or
/// project-relative; directories are hashed with the same algorithm the
/// automation runner used at registration time, plain files by content hash.
fn verify_pack(
    project_root: &Path,
    entry: &ProjectCatalogEntryV2,
) -> Result<PackVerdict, GameArtError> {
    let pack_path = resolve_relative(project_root, &entry.pack_path);
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

/// Content hash of a pack. Directory packs mirror the automation runner's
/// `hash_directory` exactly (files collected recursively, sorted by relative
/// path, hashing relative-path bytes followed by file contents); single-file
/// packs are hashed by raw content, matching `asset_project::hash_file`.
pub(crate) fn hash_pack(path: &Path, is_directory: bool) -> Result<String, std::io::Error> {
    if !is_directory {
        return Ok(format!("{:x}", Sha256::digest(fs::read(path)?)));
    }
    let mut relative_paths = Vec::new();
    collect_pack_files(path, path, &mut relative_paths)?;
    relative_paths.sort();
    let mut hasher = Sha256::new();
    for relative in relative_paths {
        hasher.update(relative.to_string_lossy().as_bytes());
        hasher.update(fs::read(path.join(relative))?);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn collect_pack_files(
    root: &Path,
    directory: &Path,
    paths: &mut Vec<PathBuf>,
) -> Result<(), std::io::Error> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_pack_files(root, &entry.path(), paths)?;
        } else if file_type.is_file() {
            paths.push(
                entry
                    .path()
                    .strip_prefix(root)
                    .unwrap_or(&entry.path())
                    .to_path_buf(),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_project::{ForgeProjectV1, ProviderSelection, FORGE_PROJECT_FILE};
    use crate::catalog::{
        register_catalog_asset_v2, CatalogLockRevisionsV1, CatalogProviderRefV1, CatalogStyleRefV1,
        CatalogSubjectRefV1,
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
        fs::write(
            directory.join(STYLE_LOCK_FILE),
            json!({
                "schemaVersion": "1",
                "revision": revision,
                "providerId": "xai",
                "profileId": "default"
            })
            .to_string(),
        )
        .unwrap();
    }

    fn write_subject_lock(root: &Path, id: &str, revision: &str, recorded_revision: &str) {
        let directory = root.join(".forge/subjects").join(id).join(revision);
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("subject-lock.json"),
            json!({
                "schemaVersion": "1",
                "id": id,
                "revision": recorded_revision,
                "providerId": "xai",
                "profileId": "default"
            })
            .to_string(),
        )
        .unwrap();
    }

    fn write_pack(root: &Path, relative: &str, files: &[(&str, &[u8])]) -> String {
        let pack_dir = root.join(relative);
        fs::create_dir_all(&pack_dir).unwrap();
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
        }
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
            "hero",
            "character",
            "topdown@1.0.0",
            Some("subject-rev-1"),
            &[("pack.json", b"{}")],
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
            ("a", "character", "topdown@1.0.0"),
            ("b", "icon_set", "static-set@1.0.0"),
            ("c", "prop_set", "static-set@1.0.0"),
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
    fn pack_directory_hash_matches_runner_algorithm() {
        // automation::runner::hash_directory hashes relative-path bytes followed
        // by file contents over recursively collected, sorted files. Pin the
        // single-file case so the diff's mirror cannot silently drift.
        let temp = tempfile::tempdir().unwrap();
        let pack = temp.path().join("pack");
        fs::create_dir_all(&pack).unwrap();
        fs::write(pack.join("a.txt"), b"x").unwrap();
        assert_eq!(
            hash_pack(&pack, true).unwrap(),
            format!("{:x}", Sha256::digest(b"a.txtx"))
        );
        // Nested file: hash(relpath("a.txt") ++ "x" ++ relpath("sub/b.txt") ++ "y").
        fs::create_dir_all(pack.join("sub")).unwrap();
        fs::write(pack.join("sub/b.txt"), b"y").unwrap();
        let expected = {
            let mut hasher = Sha256::new();
            hasher.update(b"a.txtx");
            hasher.update(Path::new("sub").join("b.txt").to_string_lossy().as_bytes());
            hasher.update(b"y");
            format!("{:x}", hasher.finalize())
        };
        assert_eq!(hash_pack(&pack, true).unwrap(), expected);
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
