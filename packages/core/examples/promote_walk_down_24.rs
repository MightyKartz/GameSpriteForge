#![recursion_limit = "256"]

use std::env;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use forge_core::character_animation_review::{
    review_is_approved, validate_animation_review_closure, AnimationHumanReviewV1,
};
use forge_core::keyframe_cleanup::cleanup_keyframe_background;
use forge_core::quality::{
    sample_source_cycle_frames, validate_source_cycle_sampling_report, SourceCycleSamplingPolicyV1,
};
use image::{imageops::FilterType, RgbaImage};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const FRAME_COUNT: usize = 24;
const SPEED_PIXELS_PER_SECOND: f64 = 70.0;
const PIVOT_X: f64 = 720.0;
const PIVOT_Y: f64 = 1351.0;
const COLLISION_WIDTH: f64 = 120.0;
const COLLISION_HEIGHT: f64 = 160.0;
const COLLISION_OFFSET_X: f64 = 0.0;
const COLLISION_OFFSET_Y: f64 = -80.0;
const VISUAL_SCALE: f64 = 0.36;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplaySummary {
    action: String,
    direction: String,
    frame_durations_ms: Vec<u64>,
    frames: Vec<PathBuf>,
    selected_frame_indices: Vec<usize>,
    selected_timestamps_ms: Vec<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeSourceReport {
    frame_timestamps_ms: Vec<u64>,
    frames: Vec<PathBuf>,
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn sha256(path: &Path) -> Result<String, Box<dyn Error>> {
    Ok(sha256_bytes(&fs::read(path)?))
}

fn read_json(path: &Path) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn pretty_json_bytes(value: &Value) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn write_json(path: &Path, value: &Value) -> Result<(), Box<dyn Error>> {
    fs::write(path, pretty_json_bytes(value)?)?;
    Ok(())
}

fn write_new_bytes(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
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

fn absolute(path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(env::current_dir()?.join(path))
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
            "sha256": sha256_bytes(&contents),
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

fn resize_for_cycle_analysis(frame: &RgbaImage) -> RgbaImage {
    const MAXIMUM_DIMENSION: u32 = 256;
    let longest = frame.width().max(frame.height());
    if longest <= MAXIMUM_DIMENSION {
        return frame.clone();
    }
    let scale = MAXIMUM_DIMENSION as f64 / longest as f64;
    let width = (frame.width() as f64 * scale).round().max(1.0) as u32;
    let height = (frame.height() as f64 * scale).round().max(1.0) as u32;
    image::imageops::resize(frame, width, height, FilterType::Triangle)
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if arguments.len() != 10 {
        return Err("usage: promote_walk_down_24 <replay-summary> <pending-review> <approved-review> <review-lineage> <godot-evidence> <candidate-pack> <candidate-inventory> <sampling-report-output> <promotion-record-output> <production-pack-output>".into());
    }
    let replay_summary_path = &arguments[0];
    let pending_review_path = &arguments[1];
    let approved_review_path = &arguments[2];
    let review_lineage_path = &arguments[3];
    let godot_evidence_path = &arguments[4];
    let candidate_pack = &arguments[5];
    let candidate_inventory_path = &arguments[6];
    let sampling_output = &arguments[7];
    let promotion_output = &arguments[8];
    let production_output = &arguments[9];
    for output in [sampling_output, promotion_output, production_output] {
        if output.exists() {
            return Err(format!("refusing to overwrite {}", output.display()).into());
        }
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
    if replay.action != "walk_down"
        || replay.direction != "down"
        || replay.frames.len() != FRAME_COUNT
        || replay.frame_durations_ms.len() != FRAME_COUNT
        || replay.selected_frame_indices.len() != FRAME_COUNT
        || replay.selected_timestamps_ms.len() != FRAME_COUNT
        || approved.animation != replay.action
    {
        return Err("this promotion is restricted to the approved 24-frame walk_down".into());
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

    let evidence = read_json(godot_evidence_path)?;
    let lineage = read_json(review_lineage_path)?;
    let inventory = read_json(candidate_inventory_path)?;
    if compute_pack_inventory(candidate_pack)? != inventory {
        return Err("candidate Pack no longer matches its immutable inventory".into());
    }
    let pending_sha = sha256(pending_review_path)?;
    let approved_sha = sha256(approved_review_path)?;
    let lineage_sha = sha256(review_lineage_path)?;
    let evidence_sha = sha256(godot_evidence_path)?;
    let inventory_sha = sha256(candidate_inventory_path)?;
    let candidate_forgepack_sha = sha256(&candidate_pack.join("forgepack.json"))?;
    let candidate_content_sha = require_string(&inventory, "/packContentSha256")?.to_owned();

    for (path_pointer, hash_pointer) in [
        ("/pendingReviewPath", "/pendingReviewSha256"),
        ("/reviewLineagePath", "/reviewLineageSha256"),
        (
            "/candidatePackInventoryPath",
            "/candidatePackInventorySha256",
        ),
        ("/candidateManifestPath", "/candidateManifestSha256"),
        ("/projectGodotPath", "/projectGodotSha256"),
        ("/mainScenePath", "/mainSceneSha256"),
        ("/movementScriptPath", "/movementScriptSha256"),
        ("/movementReportPath", "/movementReportSha256"),
        ("/screenshotPath", "/screenshotSha256"),
        ("/moviePath", "/movieSha256"),
    ] {
        verify_path_hash(&evidence, path_pointer, hash_pointer)?;
    }
    if require_string(&evidence, "/profile")? != "godot-animation-review-evidence@1.1.0"
        || require_string(&evidence, "/pendingReviewSha256")? != pending_sha
        || require_string(&evidence, "/reviewLineageSha256")? != lineage_sha
        || require_string(&evidence, "/candidatePackInventorySha256")? != inventory_sha
        || require_string(&evidence, "/candidateForgepackSha256")? != candidate_forgepack_sha
        || require_string(&evidence, "/candidatePackContentSha256")? != candidate_content_sha
        || require_u64(&evidence, "/frameCount")? != FRAME_COUNT as u64
        || evidence["frameDurationsMs"] != json!(replay.frame_durations_ms)
        || evidence
            .pointer("/speedPixelsPerSecond")
            .and_then(Value::as_f64)
            != Some(SPEED_PIXELS_PER_SECOND)
        || evidence.pointer("/pivot/x").and_then(Value::as_f64) != Some(PIVOT_X)
        || evidence.pointer("/pivot/y").and_then(Value::as_f64) != Some(PIVOT_Y)
        || evidence.pointer("/collision/width").and_then(Value::as_f64) != Some(COLLISION_WIDTH)
        || evidence
            .pointer("/collision/height")
            .and_then(Value::as_f64)
            != Some(COLLISION_HEIGHT)
        || evidence
            .pointer("/collision/offsetX")
            .and_then(Value::as_f64)
            != Some(COLLISION_OFFSET_X)
        || evidence
            .pointer("/collision/offsetY")
            .and_then(Value::as_f64)
            != Some(COLLISION_OFFSET_Y)
        || evidence.pointer("/visualScale").and_then(Value::as_f64) != Some(VISUAL_SCALE)
    {
        return Err("Godot evidence does not close over the approved walk_down candidate".into());
    }

    for (path_pointer, hash_pointer) in [
        ("/candidateLockPath", "/candidateLockSha256"),
        ("/sourceVideoPath", "/sourceVideoSha256"),
        ("/promptPath", "/promptSha256"),
        ("/motionDriverLockPath", "/motionDriverLockSha256"),
        ("/identityAnchorPath", "/identityAnchorSha256"),
        ("/nativeSourceReportPath", "/nativeSourceReportSha256"),
        ("/nativeFrameLockPath", "/nativeFrameLockSha256"),
        ("/gaitReportPath", "/gaitReportSha256"),
        ("/replaySummaryPath", "/replaySummarySha256"),
        ("/replayFrameLockPath", "/replayFrameLockSha256"),
        ("/pendingReviewPath", "/pendingReviewSha256"),
    ] {
        verify_path_hash(&lineage, path_pointer, hash_pointer)?;
    }
    if require_string(&lineage, "/profile")? != "animation-review-lineage@1.0.0"
        || require_string(&lineage, "/pendingReviewSha256")? != pending_sha
        || require_string(&lineage, "/replaySummarySha256")? != sha256(replay_summary_path)?
        || require_u64(&lineage, "/selectedFrameCount")? != FRAME_COUNT as u64
        || lineage["frameDurationsMs"] != json!(replay.frame_durations_ms)
        || lineage["selectedNativeFrameIndices"] != json!(replay.selected_frame_indices)
        || lineage["replayFrameSha256"] != approved_value["frameSha256"]
    {
        return Err("review lineage does not bind the approved walk_down replay".into());
    }

    let native_source_path = PathBuf::from(require_string(&lineage, "/nativeSourceReportPath")?);
    let gait_report_path = PathBuf::from(require_string(&lineage, "/gaitReportPath")?);
    let native: NativeSourceReport = serde_json::from_slice(&fs::read(&native_source_path)?)?;
    let gait = read_json(&gait_report_path)?;
    let start = require_u64(&gait, "/selectedStartFrame")? as usize;
    let end_boundary = require_u64(&gait, "/selectedEndBoundaryFrame")? as usize;
    if require_string(&gait, "/profile")? != "gait-cycle@1.0.0"
        || require_string(&gait, "/verdict")? != "game_ready"
        || start >= end_boundary
        || end_boundary >= native.frames.len()
        || native.frames.len() != native.frame_timestamps_ms.len()
        || replay.selected_frame_indices.first().copied() != Some(start)
        || replay
            .selected_frame_indices
            .iter()
            .any(|index| *index < start || *index >= end_boundary)
        || replay.selected_timestamps_ms
            != replay
                .selected_frame_indices
                .iter()
                .map(|index| native.frame_timestamps_ms[*index])
                .collect::<Vec<_>>()
    {
        return Err("native/gait closure does not match the approved 24 frames".into());
    }

    let analysis_frames = native.frames[..=end_boundary]
        .iter()
        .map(|path| {
            let image = image::open(path)?.to_rgba8();
            let resized = resize_for_cycle_analysis(&image);
            Ok(cleanup_keyframe_background(&resized).0)
        })
        .collect::<Result<Vec<_>, image::ImageError>>()?;
    let sampling = sample_source_cycle_frames(
        &analysis_frames,
        &native.frame_timestamps_ms[..=end_boundary],
        start,
        end_boundary,
        &replay.selected_frame_indices,
        SourceCycleSamplingPolicyV1::reviewed_exact_24(),
    )?;
    validate_source_cycle_sampling_report(&sampling.report)?;
    let sampling_value = serde_json::to_value(&sampling.report)?;
    if require_string(&sampling_value, "/profile")? != "source-cycle-sampling@1.1.0"
        || sampling_value["outputFrameIndices"] != json!(replay.selected_frame_indices)
        || sampling_value["outputTimestampsMs"] != json!(replay.selected_timestamps_ms)
        || require_u64(&sampling_value, "/outputFrameCount")? != FRAME_COUNT as u64
    {
        return Err("24-frame sampling report changed the approved frame selection".into());
    }
    let sampling_bytes = pretty_json_bytes(&sampling_value)?;
    let sampling_sha = sha256_bytes(&sampling_bytes);

    let replay_sha = sha256(replay_summary_path)?;
    let native_lock_path = PathBuf::from(require_string(&lineage, "/nativeFrameLockPath")?);
    let native_lock_sha = require_string(&lineage, "/nativeFrameLockSha256")?.to_owned();
    let gait_sha = require_string(&lineage, "/gaitReportSha256")?.to_owned();
    let replay_lock_path = PathBuf::from(require_string(&lineage, "/replayFrameLockPath")?);
    let replay_lock_sha = require_string(&lineage, "/replayFrameLockSha256")?.to_owned();
    let approved_at = approved_value["reviewedAt"].clone();
    let approved_at_string = approved_at
        .as_str()
        .ok_or("approved review lacks reviewedAt")?
        .to_owned();
    let production_output_absolute = absolute(production_output)?;
    let production = json!({
        "schemaVersion": "1",
        "profile": "forge-character-animation-production-promotion@1.1.0",
        "providerRequestCount": 0,
        "providerRequestCountThisOperation": 0,
        "inheritedProviderRequestCount": 1,
        "totalProviderRequestCount": 1,
        "status": "approved",
        "productionEligible": true,
        "animation": "walk_down",
        "direction": "down",
        "selectedFrameCount": FRAME_COUNT,
        "approvedAt": approved_at,
        "approvalNote": approved.review_note,
        "pendingReviewPath": absolute(pending_review_path)?,
        "pendingReviewSha256": pending_sha,
        "approvedReviewPath": absolute(approved_review_path)?,
        "approvedReviewSha256": approved_sha,
        "reviewLineagePath": absolute(review_lineage_path)?,
        "reviewLineageSha256": lineage_sha,
        "godotEvidencePath": absolute(godot_evidence_path)?,
        "godotEvidenceSha256": evidence_sha,
        "candidatePackPath": absolute(candidate_pack)?,
        "candidateForgepackSha256": candidate_forgepack_sha,
        "candidatePackInventoryPath": absolute(candidate_inventory_path)?,
        "candidatePackInventorySha256": inventory_sha,
        "candidatePackContentSha256": candidate_content_sha,
        "nativeFrameLockPath": native_lock_path,
        "nativeFrameLockSha256": native_lock_sha,
        "gaitReportPath": gait_report_path,
        "gaitReportSha256": gait_sha,
        "sourceCycleSamplingReportPath": absolute(sampling_output)?,
        "sourceCycleSamplingReportSha256": sampling_sha,
        "replaySummaryPath": absolute(replay_summary_path)?,
        "replaySummarySha256": replay_sha,
        "replayFrameLockPath": replay_lock_path,
        "replayFrameLockSha256": replay_lock_sha,
        "frameDurationsMs": replay.frame_durations_ms,
        "runtimePlayback": {
            "sourceCycleDurationMs": replay.frame_durations_ms.iter().sum::<u64>(),
            "targetCycleDurationMs": replay.frame_durations_ms.iter().sum::<u64>(),
            "speedScale": 1.0,
            "speedPixelsPerSecond": SPEED_PIXELS_PER_SECOND
        },
        "productionPack": {
            "id": "direction-grid-down-walk-q2-v1-24f-production",
            "name": "DirectionGrid Down Walk Q2 V1 — 24 Frame Production",
            "version": "1.0.0",
            "plannedPath": production_output_absolute
        }
    });
    require_true(&production, "/productionEligible")?;
    let production_bytes = pretty_json_bytes(&production)?;
    let production_sha = sha256_bytes(&production_bytes);

    let parent = production_output
        .parent()
        .ok_or("production Pack has no parent")?;
    fs::create_dir_all(parent)?;
    let temporary = tempfile::Builder::new()
        .prefix(".forge-walk-down-24-production-")
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
        candidate_inventory_path,
        quality_dir.join("candidate-pack-inventory.json"),
    )?;
    fs::write(
        quality_dir.join("source-cycle-sampling-report.json"),
        &sampling_bytes,
    )?;
    fs::write(
        quality_dir.join("production-promotion.json"),
        &production_bytes,
    )?;

    let forgepack_path = staged.join("forgepack.json");
    let mut forgepack = read_json(&forgepack_path)?;
    forgepack["id"] = production["productionPack"]["id"].clone();
    forgepack["name"] = production["productionPack"]["name"].clone();
    forgepack["version"] = production["productionPack"]["version"].clone();
    forgepack["createdAt"] = json!(approved_at_string);
    forgepack["license"]["type"] = json!("User-owned provider-generated asset; human-approved");
    let metadata = forgepack
        .pointer_mut("/source/metadata")
        .and_then(Value::as_object_mut)
        .ok_or("candidate Pack source metadata is missing")?;
    metadata.insert("animationHumanReviewStatus".into(), json!("approved"));
    metadata.insert("animationHumanReviewSha256".into(), json!(approved_sha));
    metadata.insert("animationReviewLineageSha256".into(), json!(lineage_sha));
    metadata.insert("godotReviewEvidenceSha256".into(), json!(evidence_sha));
    metadata.insert("productionPromotionSha256".into(), json!(production_sha));
    metadata.insert("candidatePackInventorySha256".into(), json!(inventory_sha));
    metadata.insert(
        "candidatePackContentSha256".into(),
        json!(candidate_content_sha),
    );
    metadata.insert(
        "candidateForgepackSha256".into(),
        json!(candidate_forgepack_sha),
    );
    metadata.insert(
        "sourceCycleSamplingReportSha256".into(),
        json!(sampling_sha),
    );
    metadata.insert("productionEligible".into(), json!(true));
    metadata.insert("approvedFrameCount".into(), json!(FRAME_COUNT));
    metadata.insert(
        "approvedCycleDurationMs".into(),
        json!(replay.frame_durations_ms.iter().sum::<u64>()),
    );
    metadata.insert("approvedPlaybackSpeedScale".into(), json!(1.0));
    metadata.insert(
        "approvedSpeedPixelsPerSecond".into(),
        json!(SPEED_PIXELS_PER_SECOND),
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
        "source_cycle_sampling_24_verified",
        "production_eligible_true",
    ] {
        if !notes.iter().any(|value| value.as_str() == Some(note)) {
            notes.push(json!(note));
        }
    }
    write_json(&quality_report_path, &quality_report)?;

    forge_pack::validate_pack_layout(&staged)?;
    let summary = forge_pack::inspect_pack(&staged)?;
    if summary.frame_count != FRAME_COUNT
        || summary.animations.len() != 1
        || summary.animations[0].name != "walk_down"
        || summary.animations[0].frame_count != FRAME_COUNT
    {
        return Err("production Pack summary mismatch".into());
    }
    for (path, expected) in [
        (
            quality_dir.join("animation-human-review.json"),
            &approved_sha,
        ),
        (
            quality_dir.join("animation-review-lineage.json"),
            &lineage_sha,
        ),
        (
            quality_dir.join("godot-review-evidence.json"),
            &evidence_sha,
        ),
        (
            quality_dir.join("candidate-pack-inventory.json"),
            &inventory_sha,
        ),
        (
            quality_dir.join("source-cycle-sampling-report.json"),
            &sampling_sha,
        ),
        (
            quality_dir.join("production-promotion.json"),
            &production_sha,
        ),
    ] {
        if sha256(&path)? != *expected {
            return Err(format!("embedded evidence hash mismatch: {}", path.display()).into());
        }
    }
    for (index, expected_hash) in approved.frame_sha256.iter().enumerate() {
        let frame = staged
            .join("assets/frames")
            .join(format!("frame_{:03}.png", index + 1));
        if sha256(&frame)? != *expected_hash {
            return Err(format!("production frame {} hash mismatch", index + 1).into());
        }
    }
    let manifest = read_json(&staged.join("assets/manifest.json"))?;
    let godot_helper = read_json(&staged.join("assets/godot_import.json"))?;
    if manifest["animations"][0]["frameDurationsMs"] != json!(replay.frame_durations_ms)
        || godot_helper["spriteFrames"]["animations"][0]["frameDurationsMs"]
            != json!(replay.frame_durations_ms)
        || forgepack["animations"][0]["frameDurationsMs"] != json!(replay.frame_durations_ms)
    {
        return Err("production Pack changed exact per-frame durations".into());
    }

    write_new_bytes(sampling_output, &sampling_bytes)?;
    write_new_bytes(promotion_output, &production_bytes)?;
    fs::rename(&staged, production_output)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "profile": "walk-down-24-production-promotion-result@1.0.0",
            "productionPack": production_output,
            "frameCount": FRAME_COUNT,
            "animation": "walk_down",
            "approvedReviewSha256": approved_sha,
            "godotEvidenceSha256": evidence_sha,
            "sourceCycleSamplingReportSha256": sampling_sha,
            "productionPromotionSha256": production_sha,
            "productionEligible": true,
            "providerRequestCountThisOperation": 0,
            "inheritedProviderRequestCount": 1,
            "totalProviderRequestCount": 1
        }))?
    );
    Ok(())
}
