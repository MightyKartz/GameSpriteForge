//! Read-only reconciliation of declared asset requirements against the library.
//! Matching reuses the verified search contract; no generation, Provider call
//! or write happens here, and a `covered` status is not visual approval.
use super::*;
use intake::{SearchFilter, SearchHit};
use serde::{Deserialize, Serialize};

const MAX_REQUIREMENTS: usize = 200;
const MAX_HITS_PER_REQUIREMENT: usize = 5;
/// Matches are evaluated in search order and capped per requirement; `total`
/// counts evaluated matches, so libraries with more matches per requirement
/// report the evaluated prefix rather than the full cardinality.
const SEARCH_LIMIT: usize = 1000;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Requirement {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_domain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_verdict: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequirementsBatch {
    pub schema_version: String,
    pub kind: String,
    pub requirements: Vec<Requirement>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequirementReport {
    pub id: String,
    /// covered | needs_review | incomplete | missing
    pub status: String,
    pub total: usize,
    pub hits: Vec<SearchHit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_request: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequirementsReport {
    pub covered: usize,
    pub needs_review: usize,
    pub incomplete: usize,
    pub missing: usize,
    pub results: Vec<RequirementReport>,
}

fn validate(batch: &RequirementsBatch) -> Result<(), CatalogError> {
    if batch.schema_version != "1" || batch.kind != "asset_requirements" {
        return Err(invalid(
            "requirements batch must declare schemaVersion 1 and kind asset_requirements",
        ));
    }
    if batch.requirements.is_empty() || batch.requirements.len() > MAX_REQUIREMENTS {
        return Err(invalid("requirements must contain 1..=200 items"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for requirement in &batch.requirements {
        if requirement.id.trim().is_empty() {
            return Err(invalid("requirement id is required"));
        }
        if !seen.insert(requirement.id.clone()) {
            return Err(invalid(format!(
                "duplicate requirement id: {}",
                requirement.id
            )));
        }
        if requirement.tags.iter().any(|tag| tag.trim().is_empty()) {
            return Err(invalid("requirement tags must be non-empty"));
        }
        for value in [&requirement.kind, &requirement.purpose, &requirement.query] {
            if value.as_deref().is_some_and(|v| v.trim().is_empty()) {
                return Err(invalid(
                    "requirement kind, purpose and query must be non-empty",
                ));
            }
        }
        if requirement.review_domain.is_some() != requirement.review_verdict.is_some() {
            return Err(invalid(
                "reviewDomain and reviewVerdict must be provided together",
            ));
        }
        if requirement
            .review_domain
            .as_deref()
            .is_some_and(|d| !matches!(d, "technical" | "visual" | "auditory" | "license"))
            || requirement
                .review_verdict
                .as_deref()
                .is_some_and(|v| !matches!(v, "approved" | "rejected" | "needs_review" | "unknown"))
        {
            return Err(invalid("invalid review filter"));
        }
    }
    Ok(())
}

fn suggested_request(requirement: &Requirement) -> Option<serde_json::Value> {
    let kind = requirement.kind.as_deref()?;
    if !matches!(kind, "icon_set" | "prop_set") {
        return None;
    }
    let item_id: String = requirement
        .id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    // Static requests require an engine-safe id starting with an ASCII
    // alphanumeric; keep non-Latin logical ids usable in the skeleton.
    let item_id = if item_id
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric())
    {
        item_id
    } else {
        format!("asset-{item_id}")
    };
    Some(serde_json::json!({
        "schemaVersion": "1",
        "kind": kind,
        "id": item_id,
        "name": requirement.id,
        "license": "TODO: rights statement for the actual sources",
        "sampling": "linear",
        "canvasSize": 256,
        "items": [{
            "id": item_id,
            "name": requirement.id,
            "path": "TODO: absolute path to a transparent PNG"
        }]
    }))
}

fn check_one(root: &Path, requirement: &Requirement) -> Result<RequirementReport, CatalogError> {
    let found = intake::search(
        root,
        &SearchFilter {
            query: requirement.query.clone(),
            kind: requirement.kind.clone(),
            purpose: requirement.purpose.clone(),
            limit: SEARCH_LIMIT,
            ..Default::default()
        },
    )?;
    let matched: Vec<SearchHit> = found
        .items
        .into_iter()
        .filter(|hit| requirement.tags.iter().all(|tag| hit.tags.contains(tag)))
        .collect();
    let total = matched.len();
    let review_ok =
        |hit: &SearchHit| match (&requirement.review_domain, &requirement.review_verdict) {
            (Some(domain), Some(verdict)) => {
                hit.review_states
                    .get(domain)
                    .map(String::as_str)
                    .unwrap_or("unknown")
                    == verdict
            }
            _ => true,
        };
    let available: Vec<&SearchHit> = matched
        .iter()
        .filter(|hit| hit.status == "available")
        .collect();
    let status = if matched.is_empty() {
        "missing"
    } else if available.iter().any(|hit| review_ok(hit)) {
        "covered"
    } else if !available.is_empty() {
        "needs_review"
    } else {
        "incomplete"
    };
    let suggested_request = if status == "missing" {
        suggested_request(requirement)
    } else {
        None
    };
    Ok(RequirementReport {
        id: requirement.id.clone(),
        status: status.into(),
        total,
        hits: matched.into_iter().take(MAX_HITS_PER_REQUIREMENT).collect(),
        suggested_request,
    })
}

/// Reconcile declared requirements against the library using verified search.
/// Read-only: the catalog is never written, and the only media read is the
/// same source-byte verification a normal search performs.
pub fn check_requirements(
    root: &Path,
    batch: &RequirementsBatch,
) -> Result<RequirementsReport, CatalogError> {
    validate(batch)?;
    read_catalog(root)?;
    let mut report = RequirementsReport {
        covered: 0,
        needs_review: 0,
        incomplete: 0,
        missing: 0,
        results: vec![],
    };
    for requirement in &batch.requirements {
        let result = check_one(root, requirement)?;
        match result.status.as_str() {
            "covered" => report.covered += 1,
            "needs_review" => report.needs_review += 1,
            "incomplete" => report.incomplete += 1,
            _ => report.missing += 1,
        }
        report.results.push(result);
    }
    Ok(report)
}
