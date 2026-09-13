use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::Subcommand;
use forge_core::automation::{local_source_files, AutomationOperation, AutomationPlan};
use forge_core::delivery::{
    directory_sha256, hash_file, inventory, read_json, FileDigest, INSTALL_SNAPSHOT,
};
use forge_core::job::{JobLifecycleState, JobRecord, JobStore, JOB_WORKSPACE_JSON};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

type Result<T> = std::result::Result<T, (String, String)>;

#[derive(Subcommand)]
pub enum ReceiptCommand {
    /// Export immutable delivery evidence; never overwrites an existing receipt.
    Export {
        #[arg(long)]
        job: String,
        #[arg(long)]
        install_job: Option<String>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        review: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Verify retained evidence and bytes without a Job store or running Godot.
    Verify {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        pack: Option<PathBuf>,
        #[arg(long)]
        project: Option<PathBuf>,
        #[arg(long)]
        expected_sha256: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Document {
    kind: String,
    path: PathBuf,
    sha256: String,
    /// None means the producer did not record a hash; sha256 then binds export-time text only.
    recorded_producer_sha256: Option<String>,
    /// Exact UTF-8 source text; the digest covers these bytes, including whitespace.
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct JobEvidence {
    job: JobRecord,
    plan: AutomationPlan,
    execution: Option<Value>,
    reports: Vec<Document>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PackEvidence {
    path: PathBuf,
    sha256: String,
    files: Vec<FileDigest>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InstallEvidence {
    job: JobEvidence,
    project: PathBuf,
    asset_key: String,
    target: PathBuf,
    snapshot: Document,
    registry_entry: Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Receipt {
    schema_version: String,
    exported_at: String,
    exporter: Value,
    prepare: JobEvidence,
    pack: PackEvidence,
    install: Option<InstallEvidence>,
    review: Option<Value>,
}

pub fn identity() -> Result<Value> {
    let executable = std::env::current_exe().map_err(crate::io_error)?;
    Ok(
        json!({"version":env!("CARGO_PKG_VERSION"),"build":crate::build_info::current(),
        "executable":executable,"binarySha256":hash_file(&executable).map_err(crate::io_error)?}),
    )
}

/// Written by the actual worker before processing. Exporting a legacy job never fills this in.
pub fn capture_execution(store: &JobStore, job_id: &str, plan: &AutomationPlan) -> Result<()> {
    let job = store.read_record(job_id).map_err(crate::display_error)?;
    let sources = local_source_files(&plan.operation).map_err(invalid)?;
    let sources = sources
        .into_iter()
        .map(|path| {
            Ok(FileDigest {
                bytes: fs::metadata(&path).map_err(crate::io_error)?.len(),
                sha256: hash_file(&path).map_err(crate::io_error)?,
                path,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let input_pack_sha256 = if let AutomationOperation::InstallGodot(request) = &plan.operation {
        Some(directory_sha256(&request.pack_path).map_err(crate::io_error)?)
    } else {
        None
    };
    let evidence = json!({"schemaVersion":"1","startedAt":chrono::Utc::now().to_rfc3339(),
        "producer":identity()?,"recipeHash":plan.recipe_hash,"inputFingerprint":plan.input_fingerprint,
        "localSources":sources,"sourceScope":"local_prepare_inputs", "inputPackSha256": input_pack_sha256});
    write_new(
        &job.job_dir.join("execution-provenance.json"),
        &serde_json::to_vec_pretty(&evidence).map_err(crate::json_error)?,
    )
}

pub fn run(command: ReceiptCommand) -> Result<Value> {
    match command {
        ReceiptCommand::Export {
            job,
            install_job,
            out,
            review,
            ..
        } => export(&job, install_job.as_deref(), &out, review.as_deref()),
        ReceiptCommand::Verify {
            path,
            pack,
            project,
            expected_sha256,
            ..
        } => verify(
            &path,
            pack.as_deref(),
            project.as_deref(),
            expected_sha256.as_deref(),
        ),
    }
}

fn document(kind: &str, path: &Path) -> Result<Document> {
    let text = fs::read_to_string(path).map_err(crate::io_error)?;
    serde_json::from_str::<Value>(&text).map_err(crate::json_error)?;
    Ok(Document {
        kind: kind.into(),
        path: path.into(),
        sha256: digest(text.as_bytes()),
        recorded_producer_sha256: None,
        text,
    })
}

fn evidence(store: &JobStore, id: &str) -> Result<JobEvidence> {
    let job = store.read_record(id).map_err(crate::display_error)?;
    if job.lifecycle_state != JobLifecycleState::Succeeded {
        return Err(invalid("receipt export requires a succeeded Job; inspect failed or pending review jobs with job report"));
    }
    let plan = read_json(&job.job_dir.join(JOB_WORKSPACE_JSON)).map_err(crate::io_error)?;
    let execution_path = job.job_dir.join("execution-provenance.json");
    let execution = if execution_path.exists() {
        Some(read_json(&execution_path).map_err(crate::io_error)?)
    } else {
        None
    };
    let mut reports = Vec::new();
    for artifact in &job.artifacts {
        if retain_report(artifact) {
            let mut report = document(&artifact.kind, &artifact.path)?;
            report.recorded_producer_sha256 = artifact.sha256.clone();
            reports.push(report);
        }
    }
    let evidence = JobEvidence {
        job,
        plan,
        execution,
        reports,
    };
    validate_job_evidence(&evidence)?;
    Ok(evidence)
}

fn export(
    job_id: &str,
    install_id: Option<&str>,
    out: &Path,
    review_path: Option<&Path>,
) -> Result<Value> {
    let store = crate::job_store()?;
    export_from_store(&store, job_id, install_id, out, review_path)
}

fn export_from_store(
    store: &JobStore,
    job_id: &str,
    install_id: Option<&str>,
    out: &Path,
    review_path: Option<&Path>,
) -> Result<Value> {
    let prepare = evidence(store, job_id)?;
    let artifact = prepare
        .job
        .artifacts
        .iter()
        .find(|a| a.kind == "gsfpack")
        .ok_or_else(|| invalid("prepare Job has no Pack"))?;
    let pack_path = fs::canonicalize(&artifact.path).map_err(crate::io_error)?;
    forge_pack::validate_pack_layout(&pack_path).map_err(crate::display_error)?;
    let sha256 = directory_sha256(&pack_path).map_err(crate::io_error)?;
    if artifact.sha256.as_deref() != Some(&sha256) {
        return Err(invalid("Pack has changed or has no recorded producer hash"));
    }
    let pack = PackEvidence {
        files: inventory(&pack_path).map_err(crate::io_error)?,
        path: pack_path.clone(),
        sha256,
    };
    let install = if let Some(id) = install_id {
        let job = evidence(store, id)?;
        let AutomationOperation::InstallGodot(request) = &job.plan.operation else {
            return Err(invalid("install Job is not a Godot installation"));
        };
        if directory_sha256(&request.pack_path).map_err(crate::io_error)? != pack.sha256 {
            return Err(invalid(
                "prepare and install Jobs refer to different Pack bytes",
            ));
        }
        let key = request
            .asset_key
            .clone()
            .or_else(|| {
                request
                    .target
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
            })
            .ok_or_else(|| invalid("install asset key missing"))?;
        forge_core::delivery::verify_install(&request.project_path, &key, Some(&pack.path))
            .map_err(crate::io_error)?;
        let manifest = forge_core::project::read_project_manifest(&request.project_path)
            .map_err(crate::display_error)?;
        let entry = &manifest.assets[&key];
        if entry.last_job_id != id {
            return Err(invalid(
                "installed target now belongs to a different install Job",
            ));
        }
        let target = request.target.clone();
        let project = fs::canonicalize(&request.project_path).map_err(crate::io_error)?;
        let snapshot = document(
            "install_snapshot",
            &project.join(&target).join(INSTALL_SNAPSHOT),
        )?;
        Some(InstallEvidence {
            project,
            asset_key: key,
            target,
            snapshot,
            registry_entry: serde_json::to_value(entry).map_err(crate::json_error)?,
            job,
        })
    } else {
        None
    };
    let review = review_path
        .map(|p| read_json::<Value>(p).map_err(crate::io_error))
        .transpose()?;
    validate_review(review.as_ref(), &pack.sha256)?;
    let parent = fs::canonicalize(
        out.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )
    .map_err(crate::io_error)?;
    if parent.starts_with(&pack_path)
        || install
            .as_ref()
            .is_some_and(|i| parent.starts_with(i.project.join(&i.target)))
    {
        return Err(invalid(
            "receipt output must be outside the Pack and installed asset directories",
        ));
    }
    let receipt = Receipt {
        schema_version: "1".into(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        exporter: identity()?,
        prepare,
        pack,
        install,
        review,
    };
    validate_receipt_evidence(&receipt)?;
    let bytes = serde_json::to_vec_pretty(&receipt).map_err(crate::json_error)?;
    write_new(out, &bytes)?;
    Ok(
        json!({"path":out,"sha256":digest(&bytes),"packSha256":receipt.pack.sha256,
        "producerKnown":receipt.prepare.execution.is_some(),"visualReview":receipt.review.as_ref().and_then(|v|v["status"].as_str()).unwrap_or("not_recorded"),
        "listeningReview":"not_assessed"}),
    )
}

fn verify(
    path: &Path,
    pack_override: Option<&Path>,
    project_override: Option<&Path>,
    expected: Option<&str>,
) -> Result<Value> {
    let bytes = fs::read(path).map_err(crate::io_error)?;
    let sha = digest(&bytes);
    if expected.is_some_and(|value| value.len() != 64 || !value.eq_ignore_ascii_case(&sha)) {
        return Err(invalid(
            "receipt SHA-256 differs from the retained expected hash",
        ));
    }
    let receipt: Receipt = serde_json::from_slice(&bytes).map_err(crate::json_error)?;
    validate_receipt_evidence(&receipt)?;
    let pack = pack_override.unwrap_or(&receipt.pack.path);
    forge_pack::validate_pack_layout(pack).map_err(crate::display_error)?;
    if inventory(pack).map_err(crate::io_error)? != receipt.pack.files
        || directory_sha256(pack).map_err(crate::io_error)? != receipt.pack.sha256
    {
        return Err(invalid("Pack differs from receipt file inventory"));
    }
    let mut installation_cache_check = Value::Null;
    if let Some(install) = &receipt.install {
        let project = project_override.unwrap_or(&install.project);
        let target = forge_core::delivery::safe_project_target(project, &install.target)
            .map_err(crate::io_error)?;
        if hash_file(&target.join(INSTALL_SNAPSHOT)).map_err(crate::io_error)?
            != install.snapshot.sha256
        {
            return Err(invalid("installation baseline differs from receipt"));
        }
        let manifest =
            forge_core::project::read_project_manifest(project).map_err(crate::display_error)?;
        let current = manifest
            .assets
            .get(&install.asset_key)
            .ok_or_else(|| invalid("receipt asset missing from installed registry"))?;
        if serde_json::to_value(current).map_err(crate::json_error)? != install.registry_entry {
            return Err(invalid("installed registry entry differs from receipt"));
        }
        let audit = forge_core::delivery::verify_install(project, &install.asset_key, Some(pack))
            .map_err(crate::io_error)?;
        installation_cache_check = audit["cacheCheck"].clone();
    } else if project_override.is_some() {
        return Err(invalid("receipt has no installation evidence"));
    }
    Ok(
        json!({"schemaVersion":"1","verified":true,"receiptSha256":sha,"packSha256":receipt.pack.sha256,
        "verifiedFiles":receipt.pack.files.len(),"installationVerified":receipt.install.is_some(),"installationCacheCheck":installation_cache_check,"producerKnown":receipt.prepare.execution.is_some(),
        "integrityBasis":if expected.is_some(){"retained_expected_sha256"}else{"self_consistency_only"},
        "sourceVerification":"recorded_hashes_only","nativeLoad":"not_run","visualReview":receipt.review.as_ref().and_then(|v|v["status"].as_str()).unwrap_or("not_recorded"),"readOnly":true,
        "listeningReview":"not_assessed"}),
    )
}

fn retain_report(artifact: &forge_core::job::JobArtifactRecord) -> bool {
    // These registries are mutable project state, retained separately as the exact
    // installed entry. They must not be relabelled as original producer reports.
    !matches!(
        artifact.kind.as_str(),
        "project_manifest" | "project_catalog"
    ) && artifact
        .path
        .extension()
        .is_some_and(|value| value == "json")
}

fn validate_job_evidence(evidence: &JobEvidence) -> Result<()> {
    if evidence.plan.schema_version != "1"
        || evidence.job.lifecycle_state != JobLifecycleState::Succeeded
        || evidence.job.recipe_hash.as_deref() != Some(&evidence.plan.recipe_hash)
        || evidence.job.input_hash.as_deref() != Some(&evidence.plan.input_fingerprint)
    {
        return Err(invalid("receipt Job and Plan identities disagree"));
    }
    if digest(&serde_json::to_vec(&evidence.plan.operation).map_err(crate::json_error)?)
        != evidence.plan.recipe_hash
    {
        return Err(invalid("receipt operation recipe hash mismatch"));
    }
    if let Some(execution) = &evidence.execution {
        if execution["schemaVersion"] != "1"
            || execution["recipeHash"] != evidence.plan.recipe_hash
            || execution["inputFingerprint"] != evidence.plan.input_fingerprint
            || execution["sourceScope"] != "local_prepare_inputs"
            || execution["startedAt"]
                .as_str()
                .is_none_or(|value| chrono::DateTime::parse_from_rfc3339(value).is_err())
        {
            return Err(invalid("receipt execution provenance disagrees with Job"));
        }
        validate_identity(&execution["producer"])?;
        let sources: Vec<FileDigest> =
            serde_json::from_value(execution["localSources"].clone()).map_err(crate::json_error)?;
        let mut paths = BTreeSet::new();
        for source in sources {
            if !source.path.is_absolute()
                || !valid_sha256(&source.sha256)
                || !paths.insert(source.path)
            {
                return Err(invalid(
                    "receipt source evidence requires unique absolute paths and SHA-256 hashes",
                ));
            }
        }
    }
    let artifacts = evidence
        .job
        .artifacts
        .iter()
        .filter(|artifact| retain_report(artifact))
        .collect::<Vec<_>>();
    if artifacts.len() != evidence.reports.len() {
        return Err(invalid("receipt is missing an original JSON report"));
    }
    let mut seen = BTreeSet::new();
    for report in &evidence.reports {
        validate_document(report)?;
        let artifact = artifacts
            .iter()
            .find(|artifact| artifact.kind == report.kind && artifact.path == report.path)
            .ok_or_else(|| invalid("embedded report is not bound to a Job artifact"))?;
        if !seen.insert((&report.kind, &report.path))
            || report.recorded_producer_sha256 != artifact.sha256
            || artifact
                .sha256
                .as_ref()
                .is_some_and(|hash| hash != &report.sha256)
        {
            return Err(invalid(
                "embedded report differs from its recorded producer hash",
            ));
        }
    }
    Ok(())
}

fn validate_identity(identity: &Value) -> Result<()> {
    let build = &identity["build"];
    if identity["version"].as_str().is_none_or(str::is_empty)
        || identity["binarySha256"]
            .as_str()
            .is_none_or(|value| !valid_sha256(value))
        || identity["executable"].as_str().is_none_or(str::is_empty)
        || build["target"].as_str().is_none_or(str::is_empty)
        || build["profile"].as_str().is_none_or(str::is_empty)
        || build["features"]
            .as_array()
            .is_none_or(|values| values.iter().any(|value| !value.is_string()))
        || !(build
            .get("dirty")
            .is_some_and(|value| value.is_null() || value.is_boolean()))
        || !(build.get("gitCommit").is_some_and(|value| {
            value.is_null()
                || value.as_str().is_some_and(|text| {
                    matches!(text.len(), 40 | 64)
                        && text.bytes().all(|byte| byte.is_ascii_hexdigit())
                })
        }))
    {
        return Err(invalid("receipt build identity is incomplete or invalid"));
    }
    Ok(())
}

fn validate_receipt_evidence(receipt: &Receipt) -> Result<()> {
    if receipt.schema_version != "1" {
        return Err(invalid("unsupported receipt schemaVersion"));
    }
    validate_identity(&receipt.exporter)?;
    validate_job_evidence(&receipt.prepare)?;
    validate_review(receipt.review.as_ref(), &receipt.pack.sha256)?;
    if !valid_sha256(&receipt.pack.sha256) || receipt.pack.files.is_empty() {
        return Err(invalid("receipt Pack inventory is empty or invalid"));
    }
    let pack_artifacts = receipt
        .prepare
        .job
        .artifacts
        .iter()
        .filter(|artifact| artifact.kind == "gsfpack")
        .collect::<Vec<_>>();
    if pack_artifacts.len() != 1
        || pack_artifacts[0].sha256.as_deref() != Some(&receipt.pack.sha256)
    {
        return Err(invalid(
            "receipt Pack is not bound to exactly one prepare Job artifact",
        ));
    }
    if matches!(
        receipt.prepare.plan.operation,
        AutomationOperation::InstallGodot(_)
    ) {
        return Err(invalid(
            "receipt prepare evidence cannot be an installation Job",
        ));
    }
    let mut paths = BTreeSet::new();
    for file in &receipt.pack.files {
        if !forge_core::delivery::safe_relative(&file.path)
            || !valid_sha256(&file.sha256)
            || !paths.insert(&file.path)
        {
            return Err(invalid(
                "receipt Pack inventory has an unsafe, duplicate or invalid entry",
            ));
        }
    }
    for report in &receipt.prepare.reports {
        if let Ok(relative) = report.path.strip_prefix(&pack_artifacts[0].path) {
            if !receipt.pack.files.iter().any(|file| {
                file.path == relative
                    && file.sha256 == report.sha256
                    && file.bytes == report.text.len() as u64
            }) {
                return Err(invalid("embedded report differs from its Pack inventory"));
            }
        }
    }
    if let Some(install) = &receipt.install {
        validate_job_evidence(&install.job)?;
        validate_document(&install.snapshot)?;
        let AutomationOperation::InstallGodot(request) = &install.job.plan.operation else {
            return Err(invalid(
                "receipt installation evidence is not an install Job",
            ));
        };
        let key = request
            .asset_key
            .as_deref()
            .or_else(|| request.target.file_name().and_then(|value| value.to_str()));
        let entry: forge_core::project::ProjectAssetEntry =
            serde_json::from_value(install.registry_entry.clone()).map_err(crate::json_error)?;
        if key != Some(&install.asset_key)
            || request.target != install.target
            || entry.godot_target != install.target
            || entry.last_job_id != install.job.job.job_id
            || entry.pack_sha256 != receipt.pack.sha256
            || entry.install_snapshot_sha256.as_deref() != Some(&install.snapshot.sha256)
            || install.snapshot.kind != "install_snapshot"
        {
            return Err(invalid(
                "receipt installation Job, registry, snapshot and Pack disagree",
            ));
        }
        if install
            .job
            .execution
            .as_ref()
            .is_some_and(|execution| execution["inputPackSha256"] != receipt.pack.sha256)
        {
            return Err(invalid(
                "install worker recorded a different input Pack hash",
            ));
        }
        let native_reports = install
            .job
            .reports
            .iter()
            .filter(|report| report.kind == "godot_install_verification")
            .collect::<Vec<_>>();
        if native_reports.len() != 1 || native_reports[0].recorded_producer_sha256.is_none() {
            return Err(invalid(
                "receipt installation requires its original native verification report",
            ));
        }
        let native: Value =
            serde_json::from_str(&native_reports[0].text).map_err(crate::json_error)?;
        if native["schemaVersion"] != "1"
            || native["assetKey"] != install.asset_key
            || native["packSha256"] != receipt.pack.sha256
            || native["nativeLoadVerified"] != true
            || native["visualApproval"] != false
        {
            return Err(invalid(
                "native installation report refers to a different or unverified Pack",
            ));
        }
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_document(document: &Document) -> Result<()> {
    if digest(document.text.as_bytes()) != document.sha256 {
        return Err(invalid("embedded report digest mismatch"));
    }
    serde_json::from_str::<Value>(&document.text).map_err(crate::json_error)?;
    Ok(())
}

fn validate_review(review: Option<&Value>, pack_hash: &str) -> Result<()> {
    if let Some(review) = review {
        if review["schemaVersion"] != "1"
            || review["packSha256"] != pack_hash
            || !matches!(
                review["status"].as_str(),
                Some("accepted" | "rejected" | "pending")
            )
            || review["reviewer"]
                .as_str()
                .is_none_or(|v| v.trim().is_empty())
        {
            return Err(invalid("review requires schemaVersion 1, exact packSha256, status accepted/rejected/pending and nonempty reviewer; Forge never supplies an approval"));
        }
        if review.get("notes").is_some_and(|notes| !notes.is_string()) {
            return Err(invalid("review notes must be a string when provided"));
        }
    }
    Ok(())
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    // Publish fully written bytes without ever replacing a retained receipt.
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = format!(
        ".forge-receipt-{}-{}-{}.tmp",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    let temporary = parent.join(name);
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(crate::io_error)?;
        file.write_all(bytes).map_err(crate::io_error)?;
        file.sync_all().map_err(crate::io_error)?;
        fs::hard_link(&temporary, path).map_err(crate::io_error)
    })();
    let _ = fs::remove_file(temporary);
    result
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn invalid(message: impl std::fmt::Display) -> (String, String) {
    ("receipt_invalid".into(), message.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core::automation::{run_operation, stage_plan_job, PlanStore};
    use forge_core::job::{JobArtifactRecord, JobState};

    struct Fixture {
        root: PathBuf,
        jobs: JobStore,
        job: JobRecord,
    }
    impl Fixture {
        fn new(capture: bool) -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "forge-receipt-test-{}-{}-{}",
                std::process::id(),
                chrono::Utc::now().timestamp_nanos_opt().unwrap(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            let root = root.canonicalize().unwrap();
            let source = root.join("input.png");
            fs::write(
                &source,
                [
                    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 16, 0,
                    0, 0, 16, 8, 6, 0, 0, 0, 31, 243, 255, 97, 0, 0, 0, 40, 73, 68, 65, 84, 120,
                    156, 99, 96, 24, 5, 140, 232, 2, 39, 42, 228, 254, 227, 211, 96, 209, 241, 8,
                    69, 15, 19, 165, 46, 96, 26, 53, 128, 97, 224, 13, 24, 5, 12, 12, 0, 213, 209,
                    4, 16, 159, 221, 155, 156, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
                ],
            )
            .unwrap();
            let request = serde_json::from_value(json!({
                "schemaVersion":"1", "kind":"icon_set", "id":"receipt-fixture", "name":"Receipt fixture",
                "license":"CC0-1.0", "sampling":"nearest", "canvasSize":64,
                "items":[{"id":"fixture", "name":"Fixture", "path":source}]
            })).unwrap();
            let plans = PlanStore::new(root.join("plans")).unwrap();
            let prepared = plans
                .prepare(AutomationOperation::PrepareStatic(request))
                .unwrap();
            let plan = plans.claim(&prepared.token).unwrap();
            let jobs = JobStore::new(root.join("jobs")).unwrap();
            let staged = stage_plan_job(&jobs, &plan).unwrap();
            if capture {
                capture_execution(&jobs, &staged.job_id, &plan).unwrap();
            }
            let job = run_operation(&jobs, &staged.job_id, &plan.operation).unwrap();
            Self { root, jobs, job }
        }
        fn export(&self, name: &str) -> Result<Value> {
            export_from_store(
                &self.jobs,
                &self.job.job_id,
                None,
                &self.root.join(name),
                None,
            )
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn review_notes_follow_the_schema_before_export_and_during_verification() {
        let fixture = Fixture::new(false);
        fixture.export("receipt.json").unwrap();
        let mut receipt: Receipt = read_json(&fixture.root.join("receipt.json")).unwrap();
        let review_path = fixture.root.join("review.json");
        let out = fixture.root.join("reviewed.json");
        let mut review = json!({"schemaVersion":"1", "packSha256":receipt.pack.sha256,
            "status":"pending", "reviewer":"fixture reviewer"});
        validate_review(Some(&review), &receipt.pack.sha256).unwrap();
        for notes in [json!(123), Value::Null, json!({"text":"review"})] {
            review["notes"] = notes;
            fs::write(&review_path, serde_json::to_vec(&review).unwrap()).unwrap();
            assert!(export_from_store(
                &fixture.jobs,
                &fixture.job.job_id,
                None,
                &out,
                Some(&review_path),
            )
            .is_err());
            assert!(!out.exists());
            receipt.review = Some(review.clone());
            assert!(validate_receipt_evidence(&receipt).is_err());
        }
        review["notes"] = json!("Awaiting visual review at game scale.");
        receipt.review = Some(review);
        validate_receipt_evidence(&receipt).unwrap();
    }

    #[test]
    fn legacy_receipt_retains_unknown_producer_and_verifies_without_jobs_or_sources() {
        let fixture = Fixture::new(false);
        let exported = fixture.export("receipt.json").unwrap();
        assert_eq!(exported["producerKnown"], false);
        let path = fixture.root.join("receipt.json");
        let receipt: Receipt = read_json(&path).unwrap();
        assert!(receipt.prepare.execution.is_none());
        let saved_pack = fixture.root.join("retained.gsfpack");
        fs::rename(&receipt.pack.path, &saved_pack).unwrap();
        fs::remove_dir_all(fixture.jobs.root()).unwrap();
        fs::remove_file(fixture.root.join("input.png")).unwrap();
        let verified = verify(&path, Some(&saved_pack), None, exported["sha256"].as_str()).unwrap();
        assert_eq!(verified["verified"], true);
        assert_eq!(verified["producerKnown"], false);
        assert_eq!(verified["integrityBasis"], "retained_expected_sha256");
        assert_eq!(
            verify(&path, Some(&saved_pack), None, None).unwrap()["integrityBasis"],
            "self_consistency_only"
        );
        assert!(verify(&path, Some(&saved_pack), None, Some(&"0".repeat(64))).is_err());
    }

    #[test]
    fn export_rejects_bad_job_or_missing_changed_report_before_creating_receipt() {
        let fixture = Fixture::new(false);
        let report = fixture
            .job
            .artifacts
            .iter()
            .find(|a| a.kind == "quality_report")
            .unwrap();
        let original = fs::read(&report.path).unwrap();
        fs::write(&report.path, "{}").unwrap();
        assert!(fixture.export("changed.json").is_err());
        assert!(!fixture.root.join("changed.json").exists());
        fs::remove_file(&report.path).unwrap();
        assert!(fixture.export("missing.json").is_err());
        assert!(!fixture.root.join("missing.json").exists());
        fs::write(&report.path, original).unwrap();
        fixture
            .jobs
            .update_record(&fixture.job.job_id, |record| {
                record.recipe_hash = Some("0".repeat(64))
            })
            .unwrap();
        assert!(fixture.export("bad-job.json").is_err());
        assert!(!fixture.root.join("bad-job.json").exists());
    }

    #[test]
    fn fresh_execution_identity_is_retained_and_malformed_claims_are_rejected() {
        let fixture = Fixture::new(true);
        let first = fixture.export("receipt.json").unwrap();
        assert_eq!(first["producerKnown"], true);
        let bytes = fs::read(fixture.root.join("receipt.json")).unwrap();
        assert!(fixture.export("receipt.json").is_err());
        assert_eq!(fs::read(fixture.root.join("receipt.json")).unwrap(), bytes);
        let mut receipt: Receipt = serde_json::from_slice(&bytes).unwrap();
        let producer = receipt.prepare.execution.as_ref().unwrap()["producer"].clone();
        assert_eq!(
            producer["binarySha256"],
            identity().unwrap()["binarySha256"]
        );
        receipt.prepare.execution.as_mut().unwrap()["producer"] = json!({});
        assert!(validate_receipt_evidence(&receipt).is_err());
        let execution = fixture.job.job_dir.join("execution-provenance.json");
        let mut value: Value = read_json(&execution).unwrap();
        value["producer"] = json!({});
        fs::write(execution, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(fixture.export("forged.json").is_err());
        assert!(!fixture.root.join("forged.json").exists());
    }

    #[test]
    fn embedded_reports_cannot_be_replaced_relabelled_or_silently_omitted() {
        let fixture = Fixture::new(false);
        fixture.export("receipt.json").unwrap();
        let bytes = fs::read(fixture.root.join("receipt.json")).unwrap();
        let mut receipt: Receipt = serde_json::from_slice(&bytes).unwrap();
        receipt.prepare.reports[0].text = "{}".into();
        receipt.prepare.reports[0].sha256 = digest(b"{}");
        assert!(validate_receipt_evidence(&receipt).is_err());
        let mut receipt: Receipt = serde_json::from_slice(&bytes).unwrap();
        receipt.prepare.reports[0].path = fixture.root.join("unrelated.json");
        assert!(validate_receipt_evidence(&receipt).is_err());
        let mut receipt: Receipt = serde_json::from_slice(&bytes).unwrap();
        receipt.prepare.reports.clear();
        assert!(validate_receipt_evidence(&receipt).is_err());
        // A live project registry is not evidence from the original producer job.
        fixture
            .jobs
            .update_record(&fixture.job.job_id, |record| {
                record.artifacts.push(JobArtifactRecord {
                    kind: "project_manifest".into(),
                    path: fixture.root.join("deleted-live-registry.json"),
                    sha256: None,
                })
            })
            .unwrap();
        assert!(fixture.export("with-registry.json").is_ok());
    }

    #[test]
    fn installation_evidence_must_bind_the_actual_install_job_and_pack() {
        let fixture = Fixture::new(false);
        fixture.export("receipt.json").unwrap();
        let mut receipt: Receipt = read_json(&fixture.root.join("receipt.json")).unwrap();
        let project = fixture.root.join("game");
        fs::create_dir(&project).unwrap();
        fs::write(project.join("project.godot"), "config_version=5\n").unwrap();
        let target = PathBuf::from("addons/forge_assets/fixture");
        let request = forge_core::automation::GodotInstallRequest {
            catalog_revision: None,
            resource_lock_path: None,
            schema_version: "1".into(),
            pack_path: receipt.pack.path.clone(),
            project_path: project.clone(),
            catalog_project_path: None,
            target: target.clone(),
            asset_key: Some("fixture".into()),
            provider_refs: vec![],
        };
        let plans = PlanStore::new(fixture.root.join("install-plans")).unwrap();
        let prepared = plans
            .prepare(AutomationOperation::InstallGodot(request))
            .unwrap();
        let plan = plans.claim(&prepared.token).unwrap();
        let staged = stage_plan_job(&fixture.jobs, &plan).unwrap();
        fixture
            .jobs
            .update_record(&staged.job_id, |record| {
                record.lifecycle_state = JobLifecycleState::Succeeded;
                record.state = JobState::Exported;
            })
            .unwrap();
        let native_path = staged.job_dir.join("native-verification.json");
        let native = serde_json::to_vec(
            &json!({"schemaVersion":"1", "assetKey":"fixture", "packSha256":receipt.pack.sha256,
            "nativeLoadVerified":true, "visualApproval":false}),
        )
        .unwrap();
        fs::write(&native_path, &native).unwrap();
        fixture
            .jobs
            .update_record(&staged.job_id, |record| {
                record.artifacts.push(JobArtifactRecord {
                    kind: "godot_install_verification".into(),
                    path: native_path,
                    sha256: Some(digest(&native)),
                })
            })
            .unwrap();
        let snapshot_text = "{\"schemaVersion\":\"1\",\"files\":[]}".to_string();
        let snapshot_hash = digest(snapshot_text.as_bytes());
        let registry = json!({"assetId":"fixture","name":"Fixture","kind":"icon_set","revision":1,
            "packSha256":receipt.pack.sha256,"installSnapshotSha256":snapshot_hash,
            "pack":{"kind":"external_absolute","path":receipt.pack.path},"godotTarget":target,
            "scenePath":target.join("items"),"spriteFramesPath":target.join("items"),"usagePath":target.join("forge_usage.json"),
            "defaultAnimation":"","animations":[],"lastJobId":staged.job_id,"installedAt":chrono::Utc::now()});
        receipt.install = Some(InstallEvidence {
            job: evidence(&fixture.jobs, &staged.job_id).unwrap(),
            project,
            asset_key: "fixture".into(),
            target: target.clone(),
            snapshot: Document {
                kind: "install_snapshot".into(),
                path: target.join(INSTALL_SNAPSHOT),
                sha256: snapshot_hash,
                recorded_producer_sha256: None,
                text: snapshot_text,
            },
            registry_entry: registry,
        });
        validate_receipt_evidence(&receipt).unwrap();
        receipt.install.as_mut().unwrap().registry_entry["lastJobId"] = json!("different-job");
        assert!(validate_receipt_evidence(&receipt).is_err());
        receipt.install.as_mut().unwrap().registry_entry["lastJobId"] = json!(staged.job_id);
        receipt.install.as_mut().unwrap().registry_entry["packSha256"] = json!("0".repeat(64));
        assert!(validate_receipt_evidence(&receipt).is_err());
        receipt.install.as_mut().unwrap().registry_entry["packSha256"] = json!(receipt.pack.sha256);
        receipt.install.as_mut().unwrap().job =
            evidence(&fixture.jobs, &fixture.job.job_id).unwrap();
        assert!(validate_receipt_evidence(&receipt).is_err());
    }
}
