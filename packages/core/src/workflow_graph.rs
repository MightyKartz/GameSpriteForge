use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use dirs_next::data_dir;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const WORKFLOW_GRAPH_FILE: &str = "workflow-graph.json";

#[derive(Debug, Error)]
pub enum WorkflowGraphError {
    #[error("invalid workflow graph: {0}")]
    Invalid(String),
    #[error("workflow cache is unavailable")]
    CacheUnavailable,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowArtifactV1 {
    pub sha256: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowNodeV1 {
    pub id: String,
    pub stage: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<u8>,
    pub implementation_version: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub invalidates: Vec<String>,
    #[serde(default)]
    pub inputs: Vec<WorkflowArtifactV1>,
    #[serde(default)]
    pub outputs: Vec<WorkflowArtifactV1>,
    pub cache_key: String,
    pub provider_request: bool,
    #[serde(default)]
    pub cache_hit: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowGraphV1 {
    pub schema_version: String,
    pub workflow: String,
    pub job_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_job_id: Option<String>,
    pub nodes: Vec<WorkflowNodeV1>,
}

impl WorkflowGraphV1 {
    pub fn validate(&self) -> Result<(), WorkflowGraphError> {
        if self.schema_version != "1" || self.workflow.trim().is_empty() {
            return Err(WorkflowGraphError::Invalid(
                "schemaVersion 1 and workflow are required".into(),
            ));
        }
        let ids = self
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<BTreeSet<_>>();
        if ids.len() != self.nodes.len() {
            return Err(WorkflowGraphError::Invalid("duplicate node id".into()));
        }
        for node in &self.nodes {
            if node.id.trim().is_empty()
                || node.stage.trim().is_empty()
                || node.implementation_version.trim().is_empty()
                || node.cache_key.len() != 64
            {
                return Err(WorkflowGraphError::Invalid(format!(
                    "invalid node {}",
                    node.id
                )));
            }
            if node
                .depends_on
                .iter()
                .any(|dependency| !ids.contains(dependency.as_str()))
            {
                return Err(WorkflowGraphError::Invalid(format!(
                    "node {} references an unknown dependency",
                    node.id
                )));
            }
        }
        let mut indegree = BTreeMap::<&str, usize>::new();
        let mut children = BTreeMap::<&str, Vec<&str>>::new();
        for node in &self.nodes {
            indegree.insert(&node.id, node.depends_on.len());
            for dependency in &node.depends_on {
                children.entry(dependency).or_default().push(&node.id);
            }
        }
        let mut queue = indegree
            .iter()
            .filter_map(|(id, degree)| (*degree == 0).then_some(*id))
            .collect::<VecDeque<_>>();
        let mut visited = 0usize;
        while let Some(id) = queue.pop_front() {
            visited += 1;
            for child in children.get(id).into_iter().flatten() {
                let degree = indegree.get_mut(child).expect("known workflow child");
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(child);
                }
            }
        }
        if visited != self.nodes.len() {
            return Err(WorkflowGraphError::Invalid(
                "workflow graph contains a cycle".into(),
            ));
        }
        Ok(())
    }
}

pub fn compute_cache_key<T: Serialize>(
    stage: &str,
    implementation_version: &str,
    provider_id: Option<&str>,
    model: Option<&str>,
    parameters: &T,
    input_sha256: &[String],
) -> Result<String, WorkflowGraphError> {
    let value = serde_json::json!({
        "stage": stage,
        "implementationVersion": implementation_version,
        "providerId": provider_id,
        "model": model,
        "parameters": parameters,
        "inputSha256": input_sha256,
    });
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(&value)?)))
}

pub fn write_workflow_graph(
    path: &Path,
    graph: &WorkflowGraphV1,
) -> Result<(), WorkflowGraphError> {
    graph.validate()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, serde_json::to_vec_pretty(graph)?)?;
    fs::rename(temp, path)?;
    Ok(())
}

pub fn read_workflow_graph(path: &Path) -> Result<WorkflowGraphV1, WorkflowGraphError> {
    let graph: WorkflowGraphV1 = serde_json::from_slice(&fs::read(path)?)?;
    graph.validate()?;
    Ok(graph)
}

#[derive(Debug, Clone)]
pub struct ContentCache {
    root: PathBuf,
}

impl ContentCache {
    pub fn default_store() -> Result<Self, WorkflowGraphError> {
        if let Ok(root) = std::env::var("FORGE_CACHE_STORE") {
            return Self::new(root);
        }
        let root = data_dir()
            .ok_or(WorkflowGraphError::CacheUnavailable)?
            .join("Game Sprite Forge")
            .join("cache/v1/objects/sha256");
        Self::new(root)
    }

    pub fn new(root: impl Into<PathBuf>) -> Result<Self, WorkflowGraphError> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn put_file(&self, cache_key: &str, source: &Path) -> Result<String, WorkflowGraphError> {
        validate_sha(cache_key, "cache key")?;
        let bytes = fs::read(source)?;
        let output_sha256 = format!("{:x}", Sha256::digest(&bytes));
        let target = self.object_path(cache_key);
        fs::create_dir_all(target.parent().expect("cache object has parent"))?;
        let temp = target.with_extension("tmp");
        fs::write(&temp, bytes)?;
        fs::rename(temp, &target)?;
        fs::write(
            target.with_extension("json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schemaVersion": "1",
                "cacheKey": cache_key,
                "outputSha256": output_sha256,
            }))?,
        )?;
        Ok(output_sha256)
    }

    pub fn materialize_file(
        &self,
        cache_key: &str,
        expected_output_sha256: &str,
        target: &Path,
    ) -> Result<bool, WorkflowGraphError> {
        validate_sha(cache_key, "cache key")?;
        validate_sha(expected_output_sha256, "output SHA-256")?;
        let object = self.object_path(cache_key);
        if !object.is_file() {
            return Ok(false);
        }
        let bytes = fs::read(&object)?;
        let actual = format!("{:x}", Sha256::digest(&bytes));
        if actual != expected_output_sha256 {
            return Err(WorkflowGraphError::Invalid(format!(
                "cache object {cache_key} failed SHA-256 verification"
            )));
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let temp = target.with_extension("cache.tmp");
        fs::write(&temp, bytes)?;
        fs::rename(temp, target)?;
        Ok(true)
    }

    pub fn lookup_output_sha256(
        &self,
        cache_key: &str,
    ) -> Result<Option<String>, WorkflowGraphError> {
        validate_sha(cache_key, "cache key")?;
        let metadata_path = self.object_path(cache_key).with_extension("json");
        if !metadata_path.is_file() {
            return Ok(None);
        }
        let metadata: serde_json::Value = serde_json::from_slice(&fs::read(metadata_path)?)?;
        let output = metadata
            .get("outputSha256")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                WorkflowGraphError::Invalid(format!(
                    "cache metadata {cache_key} has no outputSha256"
                ))
            })?;
        validate_sha(output, "cached output SHA-256")?;
        Ok(Some(output.to_string()))
    }

    fn object_path(&self, cache_key: &str) -> PathBuf {
        self.root.join(&cache_key[..2]).join(&cache_key[2..])
    }
}

fn validate_sha(value: &str, label: &str) -> Result<(), WorkflowGraphError> {
    if value.len() != 64 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err(WorkflowGraphError::Invalid(format!(
            "{label} must be a 64-character SHA-256"
        )));
    }
    Ok(())
}
