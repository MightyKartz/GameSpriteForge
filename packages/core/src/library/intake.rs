//! Explicit, offline intake. Scan output is an editable batch request: content
//! assertions are rechecked under the catalog lock before any head is changed.
use super::*;
use crate::content_digest::{digest_inventory, ContentFile, ContentInventory, CONTENT_ALGORITHM};
use serde::{Deserialize, Serialize};
use std::io::Read;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OriginAssertion {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IntakeItem {
    pub asset_id: String,
    pub name: String,
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<OriginAssertion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parent_revisions: Vec<String>,
    pub kind: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub expected_content: ContentInventory,
    /// Explicit opt-in when this ID already has different content.
    #[serde(default)]
    pub new_revision: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IntakeBatch {
    pub schema_version: String,
    pub items: Vec<IntakeItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    #[serde(flatten)]
    pub batch: IntakeBatch,
    pub issues: Vec<ScanIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScanIssue {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredItem {
    pub asset_id: String,
    pub revision: String,
    pub outcome: String,
}

pub fn content_at(path: &Path) -> Result<ContentInventory, CatalogError> {
    let metadata = fs::symlink_metadata(path)?;
    if is_link(&metadata) {
        return Err(CatalogError::Symlink);
    }
    if metadata.is_dir() {
        return Ok(directory_inventory(path)?);
    }
    if !metadata.is_file() {
        return Err(invalid("resource must be a regular file or Pack"));
    }
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut bytes = 0;
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
        bytes += n as u64;
    }
    // A single-file resource has a fixed logical name; renaming/moving its
    // external source does not manufacture a different content identity.
    let files = vec![ContentFile {
        path: "source".into(),
        bytes,
        sha256: format!("{:x}", hasher.finalize()),
    }];
    Ok(ContentInventory {
        algorithm: CONTENT_ALGORITHM.into(),
        sha256: digest_inventory(&files)?,
        files,
    })
}

pub(super) fn classify(path: &Path) -> Result<(String, Vec<String>), CatalogError> {
    if path.is_dir() {
        forge_pack::validate_pack_layout(path).map_err(|e| invalid(e.to_string()))?;
        let pack = forge_pack::inspect_pack(path).map_err(|e| invalid(e.to_string()))?;
        let mut members: Vec<String> = pack.items.into_iter().map(|i| i.id).collect();
        members.extend(pack.audio_items.into_iter().map(|i| i.id));
        members.extend(pack.animations.into_iter().map(|i| i.name));
        if let Some(layered) = pack.layered {
            members.extend(layered.layers.into_iter().map(|layer| layer.id));
            members.extend(layered.clips.into_iter().map(|clip| clip.id));
        }
        members.sort();
        members.dedup();
        return Ok(("pack".into(), members));
    }
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    Ok((
        match extension.as_str() {
            "png" | "jpg" | "jpeg" | "webp" => "image",
            "wav" | "mp3" | "ogg" | "flac" | "m4a" => "audio",
            "gif" | "mp4" | "webm" => "animation",
            _ => "file",
        }
        .into(),
        vec![],
    ))
}

pub fn scan(root: &Path) -> Result<ScanReport, CatalogError> {
    if is_link(&fs::symlink_metadata(root)?) {
        return Err(CatalogError::Symlink);
    }
    let root = fs::canonicalize(root)?;
    let mut report = ScanReport {
        batch: IntakeBatch {
            schema_version: "1".into(),
            items: vec![],
        },
        issues: vec![],
    };
    if let Err(error) = scan_path(&root, &root, &mut report) {
        report.issues.push(ScanIssue {
            path: root.clone(),
            message: error.to_string(),
        });
    }
    report.batch.items.sort_by(|a, b| a.path.cmp(&b.path));
    let mut names = BTreeMap::<String, PathBuf>::new();
    let mut contents = BTreeMap::<String, PathBuf>::new();
    for item in &report.batch.items {
        if let Some(first) = names.insert(item.name.clone(), item.path.clone()) {
            report.issues.push(ScanIssue {
                path: item.path.clone(),
                message: format!("duplicate name; first location: {}", first.display()),
            });
        }
        if let Some(first) =
            contents.insert(item.expected_content.sha256.clone(), item.path.clone())
        {
            report.issues.push(ScanIssue {
                path: item.path.clone(),
                message: format!("identical bytes; first location: {}", first.display()),
            });
        }
    }
    Ok(report)
}

fn scan_path(root: &Path, path: &Path, report: &mut ScanReport) -> Result<(), CatalogError> {
    let metadata = fs::symlink_metadata(path)?;
    if is_link(&metadata) {
        report.issues.push(ScanIssue {
            path: path.into(),
            message: "link skipped".into(),
        });
        return Ok(());
    }
    let pack_candidate = metadata.is_dir()
        && (path.extension().is_some_and(|e| e == "gsfpack")
            || path.join("forgepack.json").is_file());
    if metadata.is_dir() && !pack_candidate {
        let mut children = fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
        children.sort_by_key(|a| a.file_name());
        for child in children {
            if matches!(
                child.file_name().to_str(),
                Some(
                    ".git"
                        | ".godot"
                        | ".forge"
                        | "target"
                        | "build"
                        | "dist"
                        | "node_modules"
                        | "__pycache__"
                )
            ) {
                continue;
            }
            if let Err(error) = scan_path(root, &child.path(), report) {
                report.issues.push(ScanIssue {
                    path: child.path(),
                    message: error.to_string(),
                });
            }
        }
        return Ok(());
    }
    let relative = path.strip_prefix(root).unwrap_or(path);
    let identity = if relative.as_os_str().is_empty() {
        "root".into()
    } else {
        crate::content_digest::relative_path(relative)?
    };
    let (kind, _) = classify(path)?;
    report.batch.items.push(IntakeItem {
        origin: None,
        purpose: None,
        variant: None,
        role: None,
        parent_revisions: vec![],
        asset_id: format!("local-{}", &sha(identity.as_bytes())[..20]),
        name: path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Asset")
            .into(),
        path: path.into(),
        kind,
        tags: vec![],
        expected_content: content_at(path)?,
        new_revision: false,
    });
    Ok(())
}

pub fn register(root: &Path, batch: &IntakeBatch) -> Result<Vec<RegisteredItem>, CatalogError> {
    register_inner(root, batch, None)
}

pub(super) fn register_production(
    root: &Path,
    batch: &IntakeBatch,
    source: &BTreeMap<String, serde_json::Value>,
) -> Result<Vec<RegisteredItem>, CatalogError> {
    if source
        .get("executionId")
        .and_then(|v| v.as_str())
        .is_none_or(|s| s.is_empty())
    {
        return Err(invalid(
            "production publication requires a stable execution ID",
        ));
    }
    register_inner(root, batch, Some(source))
}

fn register_inner(
    root: &Path,
    batch: &IntakeBatch,
    production: Option<&BTreeMap<String, serde_json::Value>>,
) -> Result<Vec<RegisteredItem>, CatalogError> {
    if batch.schema_version != "1" || batch.items.is_empty() {
        return Err(invalid("expected nonempty intake schemaVersion 1"));
    }
    let _lock = crate::catalog::lock_catalog(root)?;
    let expected_head = read_bytes(root, PROJECT_CATALOG_RELATIVE)?;
    let mut catalog = read_catalog(root)?;
    let mut config = local_config(root)?;
    let mut results = vec![];
    for item in &batch.items {
        if item.asset_id.trim().is_empty() || item.name.trim().is_empty() {
            return Err(invalid("assetId and name cannot be empty"));
        }
        if item.purpose.as_ref().is_some_and(|p| p.trim().is_empty())
            || item
                .parent_revisions
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != item.parent_revisions.len()
        {
            return Err(invalid(
                "purpose must be nonempty and parent revisions must be unique",
            ));
        }
        if item.expected_content != content_at(&item.path)? {
            return Err(invalid(format!("content drift: {}", item.path.display())));
        }
        let (actual_kind, members) = classify(&item.path)?;
        if actual_kind != item.kind {
            return Err(invalid("kind must match inspected resource type"));
        }
        let mut asset = if catalog.assets.contains_key(&item.asset_id) {
            read_asset(root, &catalog, &item.asset_id)?
        } else {
            AssetRecord {
                asset_id: item.asset_id.clone(),
                name: item.name.clone(),
                kind: item.kind.clone(),
                tags: item.tags.clone(),
                purpose: item.purpose.clone(),
                dispositions: BTreeMap::new(),
                selected_revision: None,
                build_revision: None,
                revisions: vec![],
                locations: BTreeMap::new(),
                additional_locations: BTreeMap::new(),
                spec_locations: BTreeMap::new(),
                installations: vec![],
                reviews: vec![],
            }
        };
        if asset.kind != item.kind && production.is_none() {
            return Err(invalid(format!("ID kind conflict: {}", item.asset_id)));
        }
        for parent in &item.parent_revisions {
            let prior: AssetRevision = read_object(root, parent)?;
            if prior.schema_version != "1" {
                return Err(invalid("parent must identify an existing revision"));
            }
        }
        let mut existing = None;
        for digest in &asset.revisions {
            let prior = read_revision(root, &asset, digest)?;
            if prior.content.as_ref() == Some(&item.expected_content)
                && prior
                    .source
                    .get("origin")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null)
                    == serde_json::to_value(&item.origin)?
                && prior.source.get("variant").and_then(|v| v.as_str()) == item.variant.as_deref()
                && prior.source.get("role").and_then(|v| v.as_str()) == item.role.as_deref()
                && prior.parent_revisions == item.parent_revisions
                && production.is_none_or(|source| {
                    source.get("executionId") == prior.source.get("executionId")
                })
            {
                existing = Some(digest.clone());
                break;
            }
        }
        let location = locate(root, &fs::canonicalize(&item.path)?, &mut config)?;
        let (digest, outcome) = if let Some(digest) = existing {
            if asset.locations.get(&digest) != Some(&location) {
                let alternatives = asset
                    .additional_locations
                    .entry(digest.clone())
                    .or_default();
                if !alternatives.contains(&location) {
                    alternatives.push(location);
                }
            }
            (digest, "existing")
        } else {
            if !asset.revisions.is_empty() && !item.new_revision {
                return Err(invalid(format!(
                    "ID conflict: {}; set newRevision explicitly",
                    item.asset_id
                )));
            }
            let mut source = production.cloned().unwrap_or_default();
            source
                .entry("method".into())
                .or_insert(serde_json::json!("local_registration"));
            source.insert("registeredAt".into(), serde_json::json!(Utc::now()));
            source.insert("resourceType".into(), serde_json::json!(actual_kind));
            if let Some(origin) = &item.origin {
                source.insert("origin".into(), serde_json::to_value(origin)?);
            }
            if let Some(variant) = &item.variant {
                source.insert("variant".into(), serde_json::json!(variant));
            }
            if let Some(role) = &item.role {
                source.insert("role".into(), serde_json::json!(role));
            }
            if !item.parent_revisions.is_empty() {
                source.insert(
                    "lineageAssertion".into(),
                    serde_json::json!("user_declared"),
                );
            }
            source.insert("members".into(), serde_json::json!(members));
            let revision = AssetRevision {
                schema_version: "1".into(),
                asset_id: item.asset_id.clone(),
                content: Some(item.expected_content.clone()),
                legacy: None,
                parent_revisions: item.parent_revisions.clone(),
                source,
            };
            let digest = write_object(root, &revision)?;
            asset.revisions.push(digest.clone());
            asset.locations.insert(digest.clone(), location);
            (digest, "registered")
        };
        catalog
            .assets
            .insert(item.asset_id.clone(), write_object(root, &asset)?);
        results.push(RegisteredItem {
            asset_id: item.asset_id.clone(),
            revision: digest,
            outcome: outcome.into(),
        });
    }
    // Detect changes during a large batch before publishing the new atomic head.
    for item in &batch.items {
        if content_at(&item.path)? != item.expected_content {
            return Err(invalid("content drift during registration"));
        }
    }
    let old: LibraryCatalog = serde_json::from_slice(&expected_head)?;
    if catalog.assets != old.assets {
        catalog.updated_at = Utc::now();
        write_json(root, LOCAL, &config)?;
        commit_head(root, &expected_head, &catalog)?;
    }
    Ok(results)
}

#[derive(Debug, Default)]
pub struct SearchFilter {
    pub query: Option<String>,
    pub kind: Option<String>,
    pub tag: Option<String>,
    pub purpose: Option<String>,
    pub disposition: Option<String>,
    pub status: Option<String>,
    pub review_domain: Option<String>,
    pub review_verdict: Option<String>,
    pub offset: usize,
    pub limit: usize,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub asset_id: String,
    pub name: String,
    pub kind: String,
    pub tags: Vec<String>,
    pub revision: String,
    pub registered_at: Option<String>,
    pub status: String,
    pub selected: bool,
    pub review_states: BTreeMap<String, String>,
    pub purpose: Option<String>,
    pub disposition: String,
    pub known_installations: usize,
    pub members: Vec<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub items: Vec<SearchHit>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

pub fn history(root: &Path, id: &str) -> Result<Vec<SearchHit>, CatalogError> {
    let catalog = read_catalog(root)?;
    let asset = read_asset(root, &catalog, id)?;
    asset
        .revisions
        .iter()
        .rev()
        .map(|digest| hit(root, &asset, digest))
        .collect()
}
fn hit(root: &Path, asset: &AssetRecord, digest: &str) -> Result<SearchHit, CatalogError> {
    let revision = read_revision(root, asset, digest)?;
    let mut members: Vec<String> = revision
        .source
        .get("members")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let paths = asset
        .locations
        .get(digest)
        .into_iter()
        .chain(asset.additional_locations.get(digest).into_iter().flatten());
    let mut status = "unavailable";
    for location in paths {
        if let Ok(path) = resolve_location(root, location) {
            if let Ok(actual) = content_at(&path) {
                status = if revision.content.as_ref() == Some(&actual) {
                    "available"
                } else {
                    "changed"
                };
                if status == "available" {
                    if members.is_empty() && path.is_dir() {
                        members = classify(&path)
                            .map(|(_, members)| members)
                            .unwrap_or_default();
                    }
                    break;
                }
            }
        }
    }
    let mut review_states = BTreeMap::new();
    for review in review::read_reviews(root, asset, digest)? {
        review_states.insert(review.domain, review.verdict);
    }
    Ok(SearchHit {
        review_states,
        purpose: asset.purpose.clone(),
        disposition: disposition(asset, digest).into(),
        known_installations: asset
            .installations
            .iter()
            .filter(|i| i.revision == digest)
            .count(),
        asset_id: asset.asset_id.clone(),
        name: asset.name.clone(),
        kind: asset.kind.clone(),
        tags: asset.tags.clone(),
        revision: digest.into(),
        registered_at: revision
            .source
            .get("registeredAt")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| revision.legacy.as_ref().map(|l| l.created_at.to_rfc3339())),
        status: status.into(),
        selected: asset.selected_revision.as_deref() == Some(digest),
        members,
    })
}
pub fn search(root: &Path, filter: &SearchFilter) -> Result<SearchResult, CatalogError> {
    if filter
        .disposition
        .as_deref()
        .is_some_and(|s| !matches!(s, "candidate" | "selected" | "discarded"))
    {
        return Err(invalid("unknown revision disposition"));
    }
    if filter.limit == 0 || filter.limit > 1000 {
        return Err(invalid("limit must be 1..1000"));
    }
    if filter
        .status
        .as_deref()
        .is_some_and(|s| !matches!(s, "available" | "changed" | "unavailable"))
    {
        return Err(invalid("unknown availability status"));
    }
    if filter.review_domain.is_some() != filter.review_verdict.is_some() {
        return Err(invalid(
            "review domain and verdict filters must be provided together",
        ));
    }
    if filter
        .review_domain
        .as_deref()
        .is_some_and(|d| !matches!(d, "technical" | "visual" | "auditory" | "license"))
        || filter
            .review_verdict
            .as_deref()
            .is_some_and(|v| !matches!(v, "approved" | "rejected" | "needs_review" | "unknown"))
    {
        return Err(invalid("invalid review filter"));
    }
    let catalog = read_catalog(root)?;
    let mut items = vec![];
    for asset in index::assets(root, &catalog)? {
        if filter.kind.as_ref().is_some_and(|k| k != &asset.kind)
            || filter.tag.as_ref().is_some_and(|t| !asset.tags.contains(t))
        {
            continue;
        }
        for digest in &asset.revisions {
            if filter
                .purpose
                .as_ref()
                .is_some_and(|p| asset.purpose.as_ref() != Some(p))
                || filter
                    .disposition
                    .as_ref()
                    .is_some_and(|d| disposition(&asset, digest) != d)
            {
                continue;
            }
            let hit = hit(root, &asset, digest)?;
            if let (Some(domain), Some(verdict)) = (&filter.review_domain, &filter.review_verdict) {
                if hit
                    .review_states
                    .get(domain)
                    .map(String::as_str)
                    .unwrap_or("unknown")
                    != verdict
                {
                    continue;
                }
            }
            if filter.status.as_ref().is_some_and(|s| s != &hit.status) {
                continue;
            }
            if let Some(query) = &filter.query {
                let text = format!("{} {} {}", hit.asset_id, hit.name, hit.members.join(" "))
                    .to_lowercase();
                if !text.contains(&query.to_lowercase()) {
                    continue;
                }
            }
            items.push(hit);
        }
    }
    items.sort_by(|a, b| {
        b.registered_at
            .cmp(&a.registered_at)
            .then_with(|| a.asset_id.cmp(&b.asset_id))
            .then_with(|| a.revision.cmp(&b.revision))
    });
    let total = items.len();
    Ok(SearchResult {
        items: items
            .into_iter()
            .skip(filter.offset)
            .take(filter.limit)
            .collect(),
        total,
        offset: filter.offset,
        limit: filter.limit,
    })
}

pub fn disposition<'a>(asset: &'a AssetRecord, revision: &str) -> &'a str {
    if asset.selected_revision.as_deref() == Some(revision) {
        "selected"
    } else {
        asset
            .dispositions
            .get(revision)
            .map(String::as_str)
            .unwrap_or("candidate")
    }
}
