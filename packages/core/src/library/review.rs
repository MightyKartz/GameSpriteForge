//! Human assertions and their evidence are revision-scoped immutable records.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewRecord {
    pub record_type: String,
    pub revision: String,
    pub domain: String,
    pub verdict: String,
    pub statement: String,
    pub reviewer: String,
    pub recorded_at: chrono::DateTime<Utc>,
    pub evidence_sha256: String,
    pub evidence_bytes: u64,
    pub evidence_path: String,
}

#[derive(Debug)]
pub struct ReviewRequest {
    pub reference: delivery::VersionRef,
    pub domain: String,
    pub verdict: String,
    pub statement: String,
    pub reviewer: String,
    pub evidence: PathBuf,
}

/// Evidence can change type after recording; a directory inventory is never
/// equivalent to the retained regular file, even if it contains matching bytes.
pub(super) fn verify_evidence(root: &Path, review: &ReviewRecord) -> Result<PathBuf, CatalogError> {
    let path = storage_path(root, &review.evidence_path)?;
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.is_file() || is_link(&metadata) {
        return Err(invalid("review evidence must remain a regular file"));
    }
    let inventory = intake::content_at(&path)?;
    let [file] = inventory.files.as_slice() else {
        return Err(invalid("review evidence must contain exactly one file"));
    };
    if file.sha256 != review.evidence_sha256 || file.bytes != review.evidence_bytes {
        return Err(invalid("review evidence is missing or changed"));
    }
    Ok(path)
}

pub fn read_reviews(
    root: &Path,
    asset: &AssetRecord,
    revision: &str,
) -> Result<Vec<ReviewRecord>, CatalogError> {
    let mut reviews = vec![];
    for digest in &asset.reviews {
        let review: ReviewRecord = read_object(root, digest)?;
        validate_review(&review)?;
        if !asset.revisions.contains(&review.revision) {
            return Err(invalid("review refers to an unknown revision"));
        }
        if review.revision == revision {
            reviews.push(review);
        }
    }
    Ok(reviews)
}
fn validate_review(review: &ReviewRecord) -> Result<(), CatalogError> {
    if review.record_type != "asset_review_v1"
        || !matches!(
            review.domain.as_str(),
            "technical" | "visual" | "auditory" | "license"
        )
        || !matches!(
            review.verdict.as_str(),
            "approved" | "rejected" | "needs_review" | "unknown"
        )
        || review.statement.trim().is_empty()
        || review.reviewer.trim().is_empty()
        || !valid_digest(&review.evidence_sha256)
        || !valid_digest(&review.revision)
        || review.evidence_path
            != format!(
                ".forge/library/media/evidence/{}.bin",
                review.evidence_sha256
            )
    {
        return Err(invalid("invalid revision review or evidence identity"));
    }
    Ok(())
}

pub fn record(root: &Path, request: &ReviewRequest) -> Result<ReviewRecord, CatalogError> {
    let _lock = crate::catalog::lock_catalog(root)?;
    let head = read_bytes(root, PROJECT_CATALOG_RELATIVE)?;
    let mut catalog = read_catalog(root)?;
    let mut asset = read_asset(root, &catalog, &request.reference.asset_id)?;
    read_revision(root, &asset, &request.reference.revision)?;
    // Recording a conclusion requires the exact media to be available now.
    delivery::resolve(root, &request.reference)?;
    let inventory = intake::content_at(&request.evidence)?;
    if !request.evidence.is_file() || inventory.files.len() != 1 {
        return Err(invalid("review evidence must be a regular file"));
    }
    let evidence = &inventory.files[0];
    let review = ReviewRecord {
        record_type: "asset_review_v1".into(),
        revision: request.reference.revision.clone(),
        domain: request.domain.clone(),
        verdict: request.verdict.clone(),
        statement: request.statement.clone(),
        reviewer: request.reviewer.clone(),
        recorded_at: Utc::now(),
        evidence_sha256: evidence.sha256.clone(),
        evidence_bytes: evidence.bytes,
        evidence_path: format!(".forge/library/media/evidence/{}.bin", evidence.sha256),
    };
    validate_review(&review)?;
    let path = storage_path(root, &review.evidence_path)?;
    fs::create_dir_all(path.parent().unwrap())?;
    if !path.exists() {
        let mut temporary = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
        let mut source = fs::File::open(&request.evidence)?;
        std::io::copy(&mut source, &mut temporary)?;
        temporary.as_file().sync_all()?;
        if intake::content_at(temporary.path())? != inventory {
            return Err(invalid("review evidence changed while retaining it"));
        }
        temporary
            .persist_noclobber(&path)
            .map_err(|e| CatalogError::Io(e.error))?;
    }
    if intake::content_at(&path)? != inventory {
        return Err(invalid("retained review evidence is corrupt"));
    }
    delivery::resolve(root, &request.reference)?;
    let object = write_object(root, &review)?;
    asset.reviews.push(object);
    catalog
        .assets
        .insert(asset.asset_id.clone(), write_object(root, &asset)?);
    catalog.updated_at = Utc::now();
    commit_head(root, &head, &catalog)?;
    Ok(review)
}

pub fn annotate(
    root: &Path,
    id: &str,
    name: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<AssetRecord, CatalogError> {
    let _lock = crate::catalog::lock_catalog(root)?;
    let head = read_bytes(root, PROJECT_CATALOG_RELATIVE)?;
    let mut catalog = read_catalog(root)?;
    let mut asset = read_asset(root, &catalog, id)?;
    if let Some(name) = name {
        if name.trim().is_empty() {
            return Err(invalid("name cannot be empty"));
        }
        asset.name = name;
    }
    if let Some(mut tags) = tags {
        if tags.iter().any(|tag| tag.trim().is_empty()) {
            return Err(invalid("tags cannot be empty strings"));
        }
        tags.sort();
        tags.dedup();
        asset.tags = tags;
    }
    let digest = write_object(root, &asset)?;
    if catalog.assets.get(id) != Some(&digest) {
        catalog.assets.insert(id.into(), digest);
        catalog.updated_at = Utc::now();
        commit_head(root, &head, &catalog)?;
    }
    Ok(asset)
}
