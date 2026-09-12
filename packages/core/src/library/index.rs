//! Disposable metadata index. Catalog objects remain authoritative; a cache
//! cannot hide a missing object, forge metadata, or cache media availability.
use super::*;
use serde::{Deserialize, Serialize};

const PATH: &str = ".forge/library/cache/index.json";
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Index {
    schema_version: String,
    snapshot_sha256: String,
    assets: BTreeMap<String, AssetRecord>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexReport {
    pub path: PathBuf,
    pub snapshot_sha256: String,
    pub assets: usize,
}

pub fn rebuild(root: &Path) -> Result<IndexReport, CatalogError> {
    let _lock = crate::catalog::lock_catalog(root)?;
    let snapshot = snapshot_sha256(root)?;
    let catalog = read_catalog(root)?;
    let assets = catalog
        .assets
        .keys()
        .map(|id| Ok((id.clone(), read_asset(root, &catalog, id)?)))
        .collect::<Result<BTreeMap<_, _>, CatalogError>>()?;
    let count = assets.len();
    if snapshot_sha256(root)? != snapshot {
        return Err(invalid("library changed during index rebuild"));
    }
    write_json(
        root,
        PATH,
        &Index {
            schema_version: "1".into(),
            snapshot_sha256: snapshot.clone(),
            assets,
        },
    )?;
    Ok(IndexReport {
        path: storage_path(root, PATH)?,
        snapshot_sha256: snapshot,
        assets: count,
    })
}

pub(super) fn assets(
    root: &Path,
    catalog: &LibraryCatalog,
) -> Result<Vec<AssetRecord>, CatalogError> {
    // Validate all authoritative objects even when a cache exists. This index
    // avoids repeated asset/review decoding in the search loop, not file checks.
    let snapshot = snapshot_sha256(root)?;
    if let Ok(index) = read_json::<Index>(root, PATH) {
        let matches = index.schema_version == "1"
            && index.snapshot_sha256 == snapshot
            && index.assets.len() == catalog.assets.len()
            && index.assets.iter().all(|(id, asset)| {
                asset.asset_id == *id
                    && serde_json::to_vec(asset)
                        .is_ok_and(|bytes| catalog.assets.get(id) == Some(&sha(&bytes)))
            });
        if matches {
            return Ok(index.assets.into_values().collect());
        }
    }
    // Queries neither repair nor write a stale, absent or corrupt cache.
    catalog
        .assets
        .keys()
        .map(|id| read_asset(root, catalog, id))
        .collect()
}
