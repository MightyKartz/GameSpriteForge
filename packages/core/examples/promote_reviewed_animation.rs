use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use forge_core::character_animation_review::{
    review_is_approved, validate_animation_review_closure, AnimationHumanReviewV1,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplaySummary {
    action: String,
    direction: String,
    frame_durations_ms: Vec<u64>,
    frames: Vec<PathBuf>,
}

fn sha256(path: &Path) -> Result<String, Box<dyn Error>> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

fn read_json(path: &Path) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn write_json(path: &Path, value: &Value) -> Result<(), Box<dyn Error>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    fs::write(path, bytes)?;
    Ok(())
}

fn require_string<'a>(value: &'a Value, pointer: &str) -> Result<&'a str, Box<dyn Error>> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing string at {pointer}").into())
}

fn require_u64(value: &Value, pointer: &str) -> Result<u64, Box<dyn Error>> {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("missing unsigned integer at {pointer}").into())
}

fn require_true(value: &Value, pointer: &str) -> Result<(), Box<dyn Error>> {
    if value.pointer(pointer).and_then(Value::as_bool) == Some(true) {
        Ok(())
    } else {
        Err(format!("expected true at {pointer}").into())
    }
}

fn verify_path_hash(
    document: &Value,
    path_pointer: &str,
    hash_pointer: &str,
) -> Result<(), Box<dyn Error>> {
    let path = PathBuf::from(require_string(document, path_pointer)?);
    let expected = require_string(document, hash_pointer)?;
    let actual = sha256(&path)?;
    if actual != expected {
        return Err(format!(
            "hash mismatch for {}: expected {}, got {}",
            path.display(),
            expected,
            actual
        )
        .into());
    }
    Ok(())
}

fn collect_regular_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(String, PathBuf)>,
) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(format!("refusing symbolic link: {}", path.display()).into());
        }
        if metadata.is_dir() {
            collect_regular_files(root, &path, files)?;
        } else if metadata.is_file() {
            let relative = path
                .strip_prefix(root)?
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/");
            files.push((relative, path));
        } else {
            return Err(format!("refusing non-regular entry: {}", path.display()).into());
        }
    }
    Ok(())
}

fn compute_pack_inventory(pack: &Path) -> Result<Value, Box<dyn Error>> {
    let metadata = fs::symlink_metadata(pack)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(format!("Pack is not a regular directory: {}", pack.display()).into());
    }
    let mut files = Vec::new();
    collect_regular_files(pack, pack, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));
    if files.is_empty() {
        return Err("Pack inventory cannot be empty".into());
    }
    let mut aggregate = Sha256::new();
    aggregate.update(b"forge-directory-hash-v2\0");
    let mut total_bytes = 0_u64;
    let mut entries = Vec::with_capacity(files.len());
    for (relative, path) in files {
        let relative_bytes = relative.as_bytes();
        let contents = fs::read(path)?;
        total_bytes += contents.len() as u64;
        aggregate.update(b"file\0");
        aggregate.update((relative_bytes.len() as u64).to_le_bytes());
        aggregate.update(relative_bytes);
        aggregate.update((contents.len() as u64).to_le_bytes());
        aggregate.update(&contents);
        entries.push(json!({
            "path": relative,
            "bytes": contents.len(),
            "sha256": format!("{:x}", Sha256::digest(&contents)),
        }));
    }
    Ok(json!({
        "schemaVersion": "1",
        "profile": "gsfpack-content-inventory@1.0.0",
        "digestAlgorithm": "forge-directory-hash-v2",
        "fileCount": entries.len(),
        "totalBytes": total_bytes,
        "packContentSha256": format!("{:x}", aggregate.finalize()),
        "files": entries,
    }))
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), Box<dyn Error>> {
    let source_metadata = fs::symlink_metadata(source)?;
    if source_metadata.file_type().is_symlink() || !source_metadata.is_dir() {
        return Err(format!(
            "source tree is not a regular directory: {}",
            source.display()
        )
        .into());
    }
    fs::create_dir(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path)?;
        if metadata.file_type().is_symlink() {
            return Err(format!("refusing source symlink: {}", source_path.display()).into());
        }
        if metadata.is_dir() {
            copy_tree(&source_path, &destination_path)?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &destination_path)?;
        } else {
            return Err(format!(
                "refusing non-regular source entry: {}",
                source_path.display()
            )
            .into());
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if arguments.len() != 9 {
        return Err("usage: promote_reviewed_animation <replay-summary> <pending-review> <approved-review> <review-lineage> <godot-evidence> <promotion-record> <candidate-pack> <candidate-inventory> <production-pack>".into());
    }
    let replay_summary_path = &arguments[0];
    let pending_review_path = &arguments[1];
    let approved_review_path = &arguments[2];
    let review_lineage_path = &arguments[3];
    let godot_evidence_path = &arguments[4];
    let promotion_path = &arguments[5];
    let candidate_pack = &arguments[6];
    let candidate_inventory_path = &arguments[7];
    let output = &arguments[8];
    if output.exists() {
        return Err(format!("refusing to overwrite {}", output.display()).into());
    }

    let replay: ReplaySummary = serde_json::from_slice(&fs::read(replay_summary_path)?)?;
    let pending: AnimationHumanReviewV1 = serde_json::from_slice(&fs::read(pending_review_path)?)?;
    let approved: AnimationHumanReviewV1 =
        serde_json::from_slice(&fs::read(approved_review_path)?)?;
    validate_animation_review_closure(
        &pending,
        replay_summary_path,
        &replay.frames,
        &replay.frame_durations_ms,
    )?;
    validate_animation_review_closure(
        &approved,
        replay_summary_path,
        &replay.frames,
        &replay.frame_durations_ms,
    )?;
    if review_is_approved(&pending) || !review_is_approved(&approved) {
        return Err("pending/approved review status transition is invalid".into());
    }
    let pending_value = serde_json::to_value(&pending)?;
    let approved_value = serde_json::to_value(&approved)?;
    for key in [
        "frameSha256",
        "frameDurationsMs",
        "sourceVideoSha256",
        "promptSha256",
        "replaySummarySha256",
        "animation",
    ] {
        if pending_value[key] != approved_value[key] {
            return Err(format!("approved review changed immutable field {key}").into());
        }
    }
    if replay.action != approved.animation || replay.frames.len() != approved.frame_sha256.len() {
        return Err("replay/review animation closure mismatch".into());
    }

    let evidence = read_json(godot_evidence_path)?;
    let promotion = read_json(promotion_path)?;
    let lineage = read_json(review_lineage_path)?;
    let inventory = read_json(candidate_inventory_path)?;
    let computed_inventory = compute_pack_inventory(candidate_pack)?;
    if computed_inventory != inventory {
        return Err("candidate Pack no longer matches its immutable inventory".into());
    }
    let pending_sha = sha256(pending_review_path)?;
    let approved_sha = sha256(approved_review_path)?;
    let lineage_sha = sha256(review_lineage_path)?;
    let evidence_sha = sha256(godot_evidence_path)?;
    let promotion_sha = sha256(promotion_path)?;
    let inventory_sha = sha256(candidate_inventory_path)?;
    let candidate_forgepack_sha = sha256(&candidate_pack.join("forgepack.json"))?;
    let candidate_content_sha = require_string(&inventory, "/packContentSha256")?;

    if require_string(&evidence, "/pendingReviewSha256")? != pending_sha
        || require_string(&evidence, "/reviewLineageSha256")? != lineage_sha
        || require_string(&evidence, "/candidatePackInventorySha256")? != inventory_sha
        || require_string(&evidence, "/candidateForgepackSha256")? != candidate_forgepack_sha
        || require_string(&evidence, "/candidatePackContentSha256")? != candidate_content_sha
        || require_u64(&evidence, "/frameCount")? != replay.frames.len() as u64
        || evidence["frameDurationsMs"] != json!(replay.frame_durations_ms)
        || evidence
            .pointer("/speedPixelsPerSecond")
            .and_then(Value::as_f64)
            != Some(0.0)
        || evidence.pointer("/pivot/x").and_then(Value::as_f64) != Some(720.0)
        || evidence.pointer("/pivot/y").and_then(Value::as_f64) != Some(1316.0)
        || evidence.pointer("/collision/width").and_then(Value::as_f64) != Some(54.0)
        || evidence
            .pointer("/collision/height")
            .and_then(Value::as_f64)
            != Some(88.0)
        || evidence
            .pointer("/collision/offsetX")
            .and_then(Value::as_f64)
            != Some(0.0)
        || evidence
            .pointer("/collision/offsetY")
            .and_then(Value::as_f64)
            != Some(-44.0)
        || evidence.pointer("/visualScale").and_then(Value::as_f64) != Some(0.28)
    {
        return Err("Godot evidence does not close over the selected idle candidate".into());
    }
    if require_string(&lineage, "/pendingReviewSha256")? != pending_sha
        || require_string(&lineage, "/replaySummarySha256")? != sha256(replay_summary_path)?
        || require_string(&lineage, "/replayFrameLockSha256")?
            != require_string(&promotion, "/replayFrameLockSha256")?
        || require_string(&lineage, "/nativeFrameLockSha256")?
            != require_string(&promotion, "/nativeFrameLockSha256")?
    {
        return Err("review lineage does not bind the selected idle locks".into());
    }
    for (path_pointer, hash_pointer) in [
        ("/nativeFrameLockPath", "/nativeFrameLockSha256"),
        ("/idleCycleReportPath", "/idleCycleReportSha256"),
        ("/replaySummaryPath", "/replaySummarySha256"),
        ("/replayFrameLockPath", "/replayFrameLockSha256"),
    ] {
        verify_path_hash(&promotion, path_pointer, hash_pointer)?;
    }
    if require_string(&promotion, "/profile")?
        != "forge-character-animation-production-promotion@1.1.0"
        || require_string(&promotion, "/status")? != "approved"
        || require_string(&promotion, "/animation")? != replay.action
        || require_string(&promotion, "/direction")? != replay.direction
        || require_string(&promotion, "/pendingReviewSha256")? != pending_sha
        || require_string(&promotion, "/approvedReviewSha256")? != approved_sha
        || require_string(&promotion, "/reviewLineageSha256")? != lineage_sha
        || require_string(&promotion, "/godotEvidenceSha256")? != evidence_sha
        || require_string(&promotion, "/candidatePackInventorySha256")? != inventory_sha
        || require_string(&promotion, "/candidateForgepackSha256")? != candidate_forgepack_sha
        || require_string(&promotion, "/candidatePackContentSha256")? != candidate_content_sha
        || require_string(&promotion, "/replaySummarySha256")? != sha256(replay_summary_path)?
        || require_u64(&promotion, "/selectedFrameCount")? != replay.frames.len() as u64
        || promotion["frameDurationsMs"] != json!(replay.frame_durations_ms)
        || require_u64(&promotion, "/providerRequestCountThisOperation")? != 0
        || require_u64(&promotion, "/inheritedProviderRequestCount")? != 1
        || require_u64(&promotion, "/totalProviderRequestCount")? != 1
        || promotion
            .pointer("/runtimePlayback/sourceCycleDurationMs")
            .and_then(Value::as_u64)
            != Some(replay.frame_durations_ms.iter().sum())
        || promotion
            .pointer("/runtimePlayback/targetCycleDurationMs")
            .and_then(Value::as_u64)
            != Some(replay.frame_durations_ms.iter().sum())
        || promotion
            .pointer("/runtimePlayback/speedScale")
            .and_then(Value::as_f64)
            != Some(1.0)
        || promotion
            .pointer("/runtimePlayback/speedPixelsPerSecond")
            .and_then(Value::as_f64)
            != Some(0.0)
        || require_string(&promotion, "/productionPack/plannedPath")? != output.to_string_lossy()
    {
        return Err("production promotion record closure mismatch".into());
    }
    require_true(&promotion, "/productionEligible")?;

    let candidate_forgepack = read_json(&candidate_pack.join("forgepack.json"))?;
    if require_string(
        &candidate_forgepack,
        "/source/metadata/animationHumanReviewStatus",
    )? != "pending"
        || candidate_forgepack
            .pointer("/source/metadata/productionEligible")
            .and_then(Value::as_bool)
            != Some(false)
        || require_u64(
            &candidate_forgepack,
            "/source/metadata/providerRequestCountThisOperation",
        )? != 0
        || require_u64(
            &candidate_forgepack,
            "/source/metadata/inheritedProviderRequestCount",
        )? != 1
        || require_u64(
            &candidate_forgepack,
            "/source/metadata/totalProviderRequestCount",
        )? != 1
    {
        return Err("candidate Pack approval/provider state is invalid".into());
    }

    let parent = output.parent().ok_or("production Pack has no parent")?;
    fs::create_dir_all(parent)?;
    let temporary = tempfile::Builder::new()
        .prefix(".forge-reviewed-animation-production-")
        .tempdir_in(parent)?;
    let staged = temporary.path().join("pack.gsfpack");
    copy_tree(candidate_pack, &staged)?;

    let quality_dir = staged.join("quality");
    fs::create_dir_all(&quality_dir)?;
    fs::copy(
        approved_review_path,
        quality_dir.join("animation-human-review.json"),
    )?;
    fs::copy(
        review_lineage_path,
        quality_dir.join("animation-review-lineage.json"),
    )?;
    fs::copy(
        godot_evidence_path,
        quality_dir.join("godot-review-evidence.json"),
    )?;
    fs::copy(
        promotion_path,
        quality_dir.join("production-promotion.json"),
    )?;
    fs::copy(
        candidate_inventory_path,
        quality_dir.join("candidate-pack-inventory.json"),
    )?;

    let forgepack_path = staged.join("forgepack.json");
    let mut forgepack = read_json(&forgepack_path)?;
    forgepack["id"] = promotion["productionPack"]["id"].clone();
    forgepack["name"] = promotion["productionPack"]["name"].clone();
    forgepack["version"] = promotion["productionPack"]["version"].clone();
    forgepack["createdAt"] = promotion["approvedAt"].clone();
    forgepack["license"]["type"] = json!("User-owned provider-generated asset; human-approved");
    let metadata = forgepack
        .pointer_mut("/source/metadata")
        .and_then(Value::as_object_mut)
        .ok_or("candidate Pack source metadata is missing")?;
    metadata.insert("animationHumanReviewStatus".into(), json!("approved"));
    metadata.insert("animationHumanReviewSha256".into(), json!(approved_sha));
    metadata.insert("animationReviewLineageSha256".into(), json!(lineage_sha));
    metadata.insert("godotReviewEvidenceSha256".into(), json!(evidence_sha));
    metadata.insert("productionPromotionSha256".into(), json!(promotion_sha));
    metadata.insert("candidatePackInventorySha256".into(), json!(inventory_sha));
    metadata.insert(
        "candidatePackContentSha256".into(),
        json!(candidate_content_sha),
    );
    metadata.insert(
        "candidateForgepackSha256".into(),
        json!(candidate_forgepack_sha),
    );
    metadata.insert("productionEligible".into(), json!(true));
    metadata.insert("approvedFrameCount".into(), json!(replay.frames.len()));
    metadata.insert(
        "approvedCycleDurationMs".into(),
        json!(replay.frame_durations_ms.iter().sum::<u64>()),
    );
    metadata.insert("approvedPlaybackSpeedScale".into(), json!(1.0));
    metadata.insert("approvedSpeedPixelsPerSecond".into(), json!(0.0));
    write_json(&forgepack_path, &forgepack)?;

    let quality_report_path = staged.join("quality-report.json");
    let mut quality_report = read_json(&quality_report_path)?;
    let notes = quality_report
        .get_mut("notes")
        .and_then(Value::as_array_mut)
        .ok_or("quality report notes are missing")?;
    notes.retain(|note| {
        !matches!(
            note.as_str(),
            Some("animation_human_review_pending" | "production_eligible_false")
        )
    });
    for note in [
        "animation_human_review_approved",
        "godot_evidence_verified",
        "production_eligible_true",
    ] {
        if !notes.iter().any(|value| value.as_str() == Some(note)) {
            notes.push(json!(note));
        }
    }
    write_json(&quality_report_path, &quality_report)?;

    forge_pack::validate_pack_layout(&staged)?;
    let summary = forge_pack::inspect_pack(&staged)?;
    if summary.frame_count != replay.frames.len()
        || summary.animations.len() != 1
        || summary.animations[0].name != replay.action
        || summary.animations[0].frame_count != replay.frames.len()
    {
        return Err("production Pack summary mismatch".into());
    }
    if sha256(&quality_dir.join("animation-human-review.json"))? != approved_sha
        || sha256(&quality_dir.join("animation-review-lineage.json"))? != lineage_sha
        || sha256(&quality_dir.join("godot-review-evidence.json"))? != evidence_sha
        || sha256(&quality_dir.join("production-promotion.json"))? != promotion_sha
        || sha256(&quality_dir.join("candidate-pack-inventory.json"))? != inventory_sha
    {
        return Err("embedded production quality evidence hash mismatch".into());
    }
    for (index, expected_hash) in approved.frame_sha256.iter().enumerate() {
        let path = staged
            .join("assets/frames")
            .join(format!("frame_{:03}.png", index + 1));
        if sha256(&path)? != *expected_hash {
            return Err(format!("production frame {} hash mismatch", index + 1).into());
        }
    }
    fs::rename(&staged, output)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "profile": "reviewed-animation-production-promotion-result@1.0.0",
            "productionPack": output,
            "frameCount": replay.frames.len(),
            "animation": replay.action,
            "approvedReviewSha256": approved_sha,
            "godotEvidenceSha256": evidence_sha,
            "productionPromotionSha256": promotion_sha,
            "productionEligible": true,
            "providerRequestCountThisOperation": 0,
            "inheritedProviderRequestCount": 1,
            "totalProviderRequestCount": 1,
        }))?
    );
    Ok(())
}
