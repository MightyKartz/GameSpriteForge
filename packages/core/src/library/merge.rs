//! Explicit three-way reconciliation after Git has brought both branches'
//! immutable objects into one store. Selection conflicts never use timestamps.
use super::*;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeReport {
    pub expected_sha256: String,
    pub conflicts: Vec<String>,
    pub assets: usize,
    pub applied: bool,
}

fn input(path: &Path) -> Result<(Vec<u8>, LibraryCatalog), CatalogError> {
    let metadata = fs::symlink_metadata(path)?;
    if is_link(&metadata) {
        return Err(CatalogError::Symlink);
    }
    if metadata.len() > MAX_METADATA_BYTES {
        return Err(invalid("merge input exceeds 16 MiB"));
    }
    let bytes = fs::read(path)?;
    let catalog: LibraryCatalog = serde_json::from_slice(&bytes)?;
    if catalog.schema_version != "3" {
        return Err(invalid("merge requires V3 catalog snapshots"));
    }
    Ok((bytes, catalog))
}

fn reconcile(
    base: Option<&Value>,
    ours: Option<&Value>,
    theirs: Option<&Value>,
    path: &str,
    conflicts: &mut Vec<String>,
) -> Option<Value> {
    // Preserve causal order within each input branch. Hash/JSON ordering only
    // breaks ties between unrelated additions; it must never reverse a branch.
    if ["/revisions", "/reviews", "/installations"]
        .iter()
        .any(|suffix| path.ends_with(suffix))
    {
        if let (Some(a), Some(b)) = (
            ours.and_then(Value::as_array),
            theirs.and_then(Value::as_array),
        ) {
            let ancestor = base
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or_default();
            if let Some(merged) = merge_history(ancestor, a, b) {
                return Some(Value::Array(merged));
            }
            conflicts.push(path.into());
            return ours.cloned();
        }
    }
    if ours == theirs {
        return ours.cloned();
    }
    if ours == base {
        return theirs.cloned();
    }
    if theirs == base {
        return ours.cloned();
    }
    if let (Some(Value::Object(a)), Some(Value::Object(b))) = (ours, theirs) {
        let empty = serde_json::Map::new();
        let ancestor = base.and_then(Value::as_object).unwrap_or(&empty);
        let keys: BTreeSet<_> = ancestor.keys().chain(a.keys()).chain(b.keys()).collect();
        let mut output = serde_json::Map::new();
        for key in keys {
            if let Some(value) = reconcile(
                ancestor.get(key),
                a.get(key),
                b.get(key),
                &format!("{path}/{key}"),
                conflicts,
            ) {
                output.insert(key.clone(), value);
            }
        }
        return Some(Value::Object(output));
    }
    conflicts.push(path.into());
    ours.cloned()
}

/// Append-only histories retain their common prefix and both branch orderings.
/// Incompatible order constraints are conflicts, never timestamp/hash winners.
fn merge_history(base: &[Value], ours: &[Value], theirs: &[Value]) -> Option<Vec<Value>> {
    if !ours.starts_with(base) || !theirs.starts_with(base) {
        return None;
    }
    let mut nodes = BTreeMap::new();
    let mut edges = BTreeMap::<String, BTreeSet<String>>::new();
    let mut incoming = BTreeMap::<String, usize>::new();
    for branch in [ours, theirs] {
        let additions = &branch[base.len()..];
        for value in additions {
            let key = value.to_string();
            nodes.insert(key.clone(), value.clone());
            incoming.entry(key).or_default();
        }
        for pair in additions.windows(2) {
            let from = pair[0].to_string();
            let to = pair[1].to_string();
            if from != to && edges.entry(from).or_default().insert(to.clone()) {
                *incoming.entry(to).or_default() += 1;
            }
        }
    }
    let mut ready: BTreeSet<_> = incoming
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(key, _)| key.clone())
        .collect();
    let mut merged = base.to_vec();
    while let Some(key) = ready.pop_first() {
        merged.push(nodes.remove(&key)?);
        for next in edges.get(&key).into_iter().flatten() {
            let count = incoming.get_mut(next)?;
            *count -= 1;
            if *count == 0 {
                ready.insert(next.clone());
            }
        }
    }
    nodes.is_empty().then_some(merged)
}

fn review_conflicts(
    root: &Path,
    base: Option<&Value>,
    ours: Option<&Value>,
    theirs: Option<&Value>,
    id: &str,
    conflicts: &mut Vec<String>,
) -> Result<(), CatalogError> {
    let refs = |value: Option<&Value>| -> BTreeSet<String> {
        value
            .and_then(|v| v.get("reviews"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(String::from)
            .collect()
    };
    let ancestor = refs(base);
    let left = refs(ours);
    let right = refs(theirs);
    let mut left_domains = BTreeMap::new();
    for digest in left.difference(&ancestor) {
        let review: review::ReviewRecord = read_object(root, digest)?;
        left_domains
            .entry((review.revision, review.domain))
            .or_insert_with(BTreeSet::new)
            .insert(digest.clone());
    }
    for digest in right.difference(&ancestor) {
        let review: review::ReviewRecord = read_object(root, digest)?;
        let key = (review.revision, review.domain);
        if left_domains
            .get(&key)
            .is_some_and(|values| !values.contains(digest))
        {
            conflicts.push(format!("asset/{id}/reviews/{}/{}", key.0, key.1));
        }
    }
    Ok(())
}

pub fn run(
    root: &Path,
    base_path: &Path,
    ours_path: &Path,
    theirs_path: &Path,
    expected: Option<&str>,
) -> Result<MergeReport, CatalogError> {
    let _lock = if expected.is_some() {
        Some(crate::catalog::lock_catalog(root)?)
    } else {
        None
    };
    let head = read_bytes(root, PROJECT_CATALOG_RELATIVE)?; // May contain Git conflict markers.
    let identity: Identity = read_json(root, IDENTITY)?;
    let (base_bytes, base) = input(base_path)?;
    let (ours_bytes, ours) = input(ours_path)?;
    let (theirs_bytes, theirs) = input(theirs_path)?;
    for catalog in [&base, &ours, &theirs] {
        if catalog.project_id != identity.project_id
            || catalog.migrated_from_sha256 != identity.migrated_from_sha256
        {
            return Err(invalid(
                "merge inputs belong to different library identities",
            ));
        }
        for id in catalog.assets.keys() {
            let asset = read_asset(root, catalog, id)?;
            for revision in &asset.revisions {
                read_revision(root, &asset, revision)?;
            }
        }
    }
    // Framing prevents ambiguous concatenation of independent inputs.
    let mut fingerprint = b"forge-library-merge-v1\0".to_vec();
    for bytes in [&head, &base_bytes, &ours_bytes, &theirs_bytes] {
        fingerprint.extend((bytes.len() as u64).to_le_bytes());
        fingerprint.extend(bytes);
    }
    let digest = sha(&fingerprint);
    if expected.is_some_and(|value| value != digest) {
        return Err(invalid("merge preview is stale; no changes applied"));
    }
    let mut conflicts = vec![];
    let mut catalog = ours.clone();
    catalog.name = reconcile(
        Some(&serde_json::json!(base.name)),
        Some(&serde_json::json!(ours.name)),
        Some(&serde_json::json!(theirs.name)),
        "catalog/name",
        &mut conflicts,
    )
    .unwrap()
    .as_str()
    .unwrap()
    .into();
    catalog.assets.clear();
    let ids: BTreeSet<_> = base
        .assets
        .keys()
        .chain(ours.assets.keys())
        .chain(theirs.assets.keys())
        .collect();
    let mut merged_records = vec![];
    for id in ids {
        let values = [&base, &ours, &theirs].map(|c| {
            if c.assets.contains_key(id) {
                read_asset(root, c, id)
                    .and_then(|a| Ok(serde_json::to_value(a)?))
                    .map(Some)
            } else {
                Ok(None)
            }
        });
        let [ancestor, left, right] = values;
        let (ancestor, left, right) = (ancestor?, left?, right?);
        review_conflicts(
            root,
            ancestor.as_ref(),
            left.as_ref(),
            right.as_ref(),
            id,
            &mut conflicts,
        )?;
        if let Some(value) = reconcile(
            ancestor.as_ref(),
            left.as_ref(),
            right.as_ref(),
            &format!("asset/{id}"),
            &mut conflicts,
        ) {
            let record: AssetRecord = serde_json::from_value(value)?;
            // A selection and a concurrent discard cannot silently coexist.
            if record
                .selected_revision
                .as_ref()
                .is_some_and(|r| record.dispositions.get(r).is_some_and(|s| s == "discarded"))
            {
                conflicts.push(format!("asset/{id}/selectedRevision+dispositions"));
            }
            let object = sha(&serde_json::to_vec(&record)?);
            catalog.assets.insert(id.clone(), object);
            merged_records.push(record);
        }
    }
    conflicts.sort();
    conflicts.dedup();
    let mut report = MergeReport {
        expected_sha256: digest,
        conflicts,
        assets: catalog.assets.len(),
        applied: false,
    };
    if expected.is_some() && report.conflicts.is_empty() {
        for record in merged_records {
            write_object(root, &record)?;
        }
        // Validate merged references before updating the only authoritative head.
        for id in catalog.assets.keys() {
            read_asset(root, &catalog, id)?;
        }
        catalog.updated_at = Utc::now();
        commit_head(root, &head, &catalog)?;
        report.applied = true;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn append_histories_preserve_each_branch_order_in_both_hash_orders() {
        for (first, last) in [("z-approved", "a-rejected"), ("a-approved", "z-rejected")] {
            for path in [
                "asset/id/reviews",
                "asset/id/revisions",
                "asset/id/installations",
            ] {
                let base = json!(["base"]);
                let ordered = json!(["base", first, last]);
                let independent = json!(["base", "m-independent"]);
                for (ours, theirs) in [(&ordered, &independent), (&independent, &ordered)] {
                    let mut conflicts = vec![];
                    let value =
                        reconcile(Some(&base), Some(ours), Some(theirs), path, &mut conflicts)
                            .unwrap();
                    let merged = value.as_array().unwrap();
                    assert!(conflicts.is_empty());
                    assert_eq!(merged.len(), 4);
                    assert_eq!(merged[0], "base");
                    assert!(
                        merged.iter().position(|v| v == first).unwrap()
                            < merged.iter().position(|v| v == last).unwrap()
                    );
                }
            }
        }
    }

    #[test]
    fn incompatible_order_or_removal_is_a_history_conflict() {
        for (base, ours, theirs) in [
            (
                json!(["base"]),
                json!(["base", "a", "b"]),
                json!(["base", "b", "a"]),
            ),
            (json!(["base", "a"]), json!(["base"]), json!(["base", "a"])),
        ] {
            let mut conflicts = vec![];
            reconcile(
                Some(&base),
                Some(&ours),
                Some(&theirs),
                "asset/id/reviews",
                &mut conflicts,
            );
            assert_eq!(conflicts, ["asset/id/reviews"]);
        }
    }
}
