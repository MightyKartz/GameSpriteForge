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
    frame_durations_ms: Vec<u64>,
    frames: Vec<PathBuf>,
}

fn sha256(path: &Path) -> Result<String, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
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

fn require_true(value: &Value, pointer: &str) -> Result<(), Box<dyn Error>> {
    if value.pointer(pointer).and_then(Value::as_bool) == Some(true) {
        Ok(())
    } else {
        Err(format!("expected true at {pointer}").into())
    }
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
    if arguments.len() != 10 {
        return Err("usage: promote_direction_grid_right_walk_12 <replay-summary> <pending-review> <approved-review> <review-lineage> <godot-evidence> <timing-approval> <promotion-record> <candidate-pack> <candidate-inventory> <production-pack>".into());
    }
    let replay_summary_path = &arguments[0];
    let pending_review_path = &arguments[1];
    let approved_review_path = &arguments[2];
    let review_lineage_path = &arguments[3];
    let godot_evidence_path = &arguments[4];
    let timing_approval_path = &arguments[5];
    let promotion_path = &arguments[6];
    let candidate_pack = &arguments[7];
    let candidate_inventory_path = &arguments[8];
    let output = &arguments[9];
    if output.exists() {
        return Err(format!("refusing to overwrite {}", output.display()).into());
    }

    let replay: ReplaySummary = serde_json::from_slice(&fs::read(replay_summary_path)?)?;
    let pending: AnimationHumanReviewV1 = serde_json::from_slice(&fs::read(pending_review_path)?)?;
    let approved: AnimationHumanReviewV1 =
        serde_json::from_slice(&fs::read(approved_review_path)?)?;
    if review_is_approved(&pending) || !review_is_approved(&approved) {
        return Err("pending/approved review status transition is invalid".into());
    }
    validate_animation_review_closure(
        &approved,
        replay_summary_path,
        &replay.frames,
        &replay.frame_durations_ms,
    )?;
    if replay.action != "walk_right" || replay.frames.len() != 12 {
        return Err("this promotion is restricted to the approved 12-frame walk_right".into());
    }
    let pending_value = serde_json::to_value(&pending)?;
    let approved_value = serde_json::to_value(&approved)?;
    if pending_value["frameSha256"] != approved_value["frameSha256"]
        || pending_value["frameDurationsMs"] != approved_value["frameDurationsMs"]
        || pending_value["sourceVideoSha256"] != approved_value["sourceVideoSha256"]
        || pending_value["promptSha256"] != approved_value["promptSha256"]
        || pending_value["replaySummarySha256"] != approved_value["replaySummarySha256"]
    {
        return Err("approved review changed immutable pending-review closure".into());
    }

    let evidence = read_json(godot_evidence_path)?;
    let timing = read_json(timing_approval_path)?;
    let promotion = read_json(promotion_path)?;
    let lineage = read_json(review_lineage_path)?;
    let inventory = read_json(candidate_inventory_path)?;
    let pending_sha = sha256(pending_review_path)?;
    let approved_sha = sha256(approved_review_path)?;
    let lineage_sha = sha256(review_lineage_path)?;
    let evidence_sha = sha256(godot_evidence_path)?;
    let timing_sha = sha256(timing_approval_path)?;
    let promotion_sha = sha256(promotion_path)?;
    let inventory_sha = sha256(candidate_inventory_path)?;
    let candidate_forgepack_sha = sha256(&candidate_pack.join("forgepack.json"))?;

    if require_string(&evidence, "/pendingReviewSha256")? != pending_sha
        || require_string(&evidence, "/reviewLineageSha256")? != lineage_sha
        || require_string(&evidence, "/candidatePackInventorySha256")? != inventory_sha
        || require_string(&evidence, "/candidateForgepackSha256")? != candidate_forgepack_sha
        || evidence.pointer("/frameCount").and_then(Value::as_u64) != Some(12)
    {
        return Err("Godot evidence does not close over the selected candidate".into());
    }
    if require_string(&lineage, "/pendingReviewSha256")? != pending_sha {
        return Err("review lineage does not bind the immutable pending review".into());
    }
    if require_string(&timing, "/selectedAnimationApprovalSha256")? != approved_sha
        || require_string(&timing, "/godotEvidenceSha256")? != evidence_sha
        || timing
            .pointer("/comparison/preferredFrameCount")
            .and_then(Value::as_u64)
            != Some(12)
    {
        return Err("timing approval does not select the approved 12-frame evidence".into());
    }
    require_true(&timing, "/promotionAuthorized")?;
    for key in [
        "twelveFrameCadenceAcceptable",
        "twentyFourFrameCadenceAcceptable",
        "twelveFrameFootPlantAcceptable",
        "twentyFourFrameFootPlantAcceptable",
        "stopPoseHoldsWithoutMotion",
        "restartContinuityAcceptable",
        "verticalStabilityAcceptable",
        "alphaEdgesAcceptable",
        "characterIdentityConsistent",
    ] {
        require_true(&timing, &format!("/requiredChecks/{key}"))?;
    }
    if require_string(&promotion, "/approvedReviewSha256")? != approved_sha
        || require_string(&promotion, "/reviewLineageSha256")? != lineage_sha
        || require_string(&promotion, "/timingApprovalSha256")? != timing_sha
        || require_string(&promotion, "/godotEvidenceSha256")? != evidence_sha
        || require_string(&promotion, "/candidatePackInventorySha256")? != inventory_sha
        || require_string(&promotion, "/candidateForgepackSha256")? != candidate_forgepack_sha
        || require_string(&promotion, "/candidatePackContentSha256")?
            != require_string(&inventory, "/packContentSha256")?
        || promotion
            .pointer("/selectedFrameCount")
            .and_then(Value::as_u64)
            != Some(12)
    {
        return Err("production promotion record closure mismatch".into());
    }
    require_true(&promotion, "/productionEligible")?;

    let parent = output.parent().ok_or("production Pack has no parent")?;
    fs::create_dir_all(parent)?;
    let temporary = tempfile::Builder::new()
        .prefix(".forge-walk-right-12-production-")
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
        timing_approval_path,
        quality_dir.join("timing-human-review-approved.json"),
    )?;
    fs::copy(
        promotion_path,
        quality_dir.join("production-promotion.json"),
    )?;

    let forgepack_path = staged.join("forgepack.json");
    let mut forgepack = read_json(&forgepack_path)?;
    forgepack["id"] = json!("direction-grid-right-walk-q2-v2-12f-production");
    forgepack["name"] = json!("DirectionGrid Right Walk Q2 V2 — 12 Frame Production");
    forgepack["version"] = json!("1.0.0");
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
    metadata.insert("timingHumanReviewSha256".into(), json!(timing_sha));
    metadata.insert("productionPromotionSha256".into(), json!(promotion_sha));
    metadata.insert("candidatePackInventorySha256".into(), json!(inventory_sha));
    metadata.insert(
        "candidatePackContentSha256".into(),
        inventory["packContentSha256"].clone(),
    );
    metadata.insert(
        "candidateForgepackSha256".into(),
        json!(candidate_forgepack_sha),
    );
    metadata.insert("productionEligible".into(), json!(true));
    metadata.insert("approvedFrameCount".into(), json!(12));
    metadata.insert("approvedTargetCycleDurationMs".into(), json!(1500));
    metadata.insert(
        "approvedPlaybackSpeedScale".into(),
        json!(1.3606666666666667_f64),
    );
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
        "timing_human_review_approved_12_frames",
        "production_eligible_true",
    ] {
        if !notes.iter().any(|value| value.as_str() == Some(note)) {
            notes.push(json!(note));
        }
    }
    write_json(&quality_report_path, &quality_report)?;

    forge_pack::validate_pack_layout(&staged)?;
    let summary = forge_pack::inspect_pack(&staged)?;
    if summary.frame_count != 12
        || summary.animations.len() != 1
        || summary.animations[0].name != "walk_right"
        || summary.animations[0].frame_count != 12
    {
        return Err("production Pack summary mismatch".into());
    }
    if sha256(&quality_dir.join("animation-human-review.json"))? != approved_sha {
        return Err("embedded approved review hash mismatch".into());
    }
    fs::rename(&staged, output)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "profile": "direction-grid-right-walk-12-production-promotion@1.0.0",
            "productionPack": output,
            "frameCount": 12,
            "animation": "walk_right",
            "approvedReviewSha256": approved_sha,
            "godotEvidenceSha256": evidence_sha,
            "timingApprovalSha256": timing_sha,
            "productionPromotionSha256": promotion_sha,
            "productionEligible": true,
            "providerRequestCount": 0
        }))?
    );
    Ok(())
}
