use std::env;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use forge_core::character_animation_review::{
    review_is_approved, validate_animation_review_shape, AnimationHumanReviewV1,
};
use forge_core::character_pack_control_review::{
    character_pack_control_review_is_approved, validate_character_pack_control_review_closure,
    validate_character_pack_control_runtime_contract, CharacterPackControlHumanReviewV1,
    CharacterPackControlReviewClosureV1, CHARACTER_PACK_CONTROL_REVIEW_SCOPE_V1,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn sha256(path: &Path) -> Result<String, Box<dyn Error>> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

fn read_json(path: &Path) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn json_bytes(value: &Value) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn write_json(path: &Path, value: &Value) -> Result<(), Box<dyn Error>> {
    fs::write(path, json_bytes(value)?)?;
    Ok(())
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
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
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
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

fn closure_from_evidence(
    evidence_path: &Path,
    evidence: &Value,
) -> Result<CharacterPackControlReviewClosureV1, Box<dyn Error>> {
    let scope = evidence
        .pointer("/humanReviewScope")
        .and_then(Value::as_array)
        .ok_or("humanReviewScope is missing")?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or("invalid human review scope")
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CharacterPackControlReviewClosureV1 {
        godot_evidence_sha256: sha256(evidence_path)?,
        candidate_pack_forgepack_sha256: require_string(evidence, "/characterPackForgepackSha256")?
            .into(),
        candidate_pack_manifest_sha256: require_string(evidence, "/characterPackManifestSha256")?
            .into(),
        candidate_pack_inventory_sha256: require_string(evidence, "/characterPackInventorySha256")?
            .into(),
        candidate_pack_content_sha256: require_string(evidence, "/characterPackContentSha256")?
            .into(),
        candidate_pack_provenance_sha256: require_string(
            evidence,
            "/characterPackProvenanceSha256",
        )?
        .into(),
        idle_right_approved_review_sha256: require_string(
            evidence,
            "/idleRightApprovedReviewSha256",
        )?
        .into(),
        walk_right_approved_review_sha256: require_string(
            evidence,
            "/walkRightApprovedReviewSha256",
        )?
        .into(),
        runtime_report_sha256: require_string(evidence, "/runtimeReportSha256")?.into(),
        authoritative_movie_sha256: require_string(evidence, "/authoritativeMovieSha256")?.into(),
        delivery_movie_sha256: require_string(evidence, "/deliveryMovieSha256")?.into(),
        human_review_scope: scope,
    })
}

fn verify_component_review_semantics(
    review_path: &Path,
    expected_sha256: &str,
    expected_animation: &str,
    expected_hashes: &Value,
    expected_durations: &Value,
) -> Result<(), Box<dyn Error>> {
    if sha256(review_path)? != expected_sha256 {
        return Err(format!("component review hash mismatch: {}", review_path.display()).into());
    }
    let review: AnimationHumanReviewV1 = serde_json::from_slice(&fs::read(review_path)?)?;
    validate_animation_review_shape(&review)?;
    let hashes = expected_hashes
        .as_array()
        .ok_or("component frame hashes are missing")?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or("invalid component frame hash")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let durations = expected_durations
        .as_array()
        .ok_or("component frame durations are missing")?
        .iter()
        .map(|value| value.as_u64().ok_or("invalid component frame duration"))
        .collect::<Result<Vec<_>, _>>()?;
    if !review_is_approved(&review)
        || review.animation != expected_animation
        || review.frame_sha256 != hashes
        || review.frame_durations_ms != durations
    {
        return Err(
            format!("component review is not semantically approved: {expected_animation}").into(),
        );
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.len() != 9 {
        return Err("usage: promote_character_pack_control <godot-evidence> <approved-control-review> <candidate-pack> <candidate-inventory> <promotion-record> <production-pack> <production-id> <production-name> <production-version>".into());
    }
    let evidence_path = PathBuf::from(&arguments[0]);
    let approved_review_path = PathBuf::from(&arguments[1]);
    let candidate_pack = PathBuf::from(&arguments[2]);
    let candidate_inventory_path = PathBuf::from(&arguments[3]);
    let promotion_path = PathBuf::from(&arguments[4]);
    let output = PathBuf::from(&arguments[5]);
    let production_id = arguments[6].to_str().ok_or("production id must be UTF-8")?;
    let production_name = arguments[7]
        .to_str()
        .ok_or("production name must be UTF-8")?;
    let production_version = arguments[8]
        .to_str()
        .ok_or("production version must be UTF-8")?;
    if promotion_path.exists() {
        return Err("refusing to overwrite production promotion record".into());
    }
    let recover_external_promotion = output.exists();
    if production_id.trim().is_empty()
        || production_name.trim().is_empty()
        || production_version.trim().is_empty()
    {
        return Err("production identity fields cannot be empty".into());
    }

    let evidence = read_json(&evidence_path)?;
    let approved: CharacterPackControlHumanReviewV1 =
        serde_json::from_slice(&fs::read(&approved_review_path)?)?;
    let closure = closure_from_evidence(&evidence_path, &evidence)?;
    validate_character_pack_control_review_closure(&approved, &closure)?;
    if !character_pack_control_review_is_approved(&approved) {
        return Err("control review is not approved".into());
    }
    if require_string(&evidence, "/profile")? != "godot-composite-control-review-evidence@1.0.0"
        || closure.human_review_scope != CHARACTER_PACK_CONTROL_REVIEW_SCOPE_V1.map(str::to_owned)
        || require_string(&evidence, "/status")? != "pending_human_control_review"
        || evidence
            .pointer("/productionEligible")
            .and_then(Value::as_bool)
            != Some(false)
        || Path::new(require_string(&evidence, "/characterPackPath")?) != candidate_pack
        || Path::new(require_string(&evidence, "/characterPackInventoryPath")?)
            != candidate_inventory_path
    {
        return Err("reviewed evidence does not bind the selected candidate Pack".into());
    }

    for (path_pointer, hash_pointer) in [
        (
            "/characterPackInventoryPath",
            "/characterPackInventorySha256",
        ),
        (
            "/characterPackProvenancePath",
            "/characterPackProvenanceSha256",
        ),
        ("/projectGodotPath", "/projectGodotSha256"),
        ("/mainScenePath", "/mainSceneSha256"),
        ("/movementScriptPath", "/movementScriptSha256"),
        ("/runtimeConfigPath", "/runtimeConfigSha256"),
        ("/runtimeReportPath", "/runtimeReportSha256"),
        ("/authoritativeMoviePath", "/authoritativeMovieSha256"),
        ("/deliveryMoviePath", "/deliveryMovieSha256"),
        (
            "/installedResources/spriteFramesPath",
            "/installedResources/spriteFramesSha256",
        ),
        (
            "/installedResources/animatedSpriteScenePath",
            "/installedResources/animatedSpriteSceneSha256",
        ),
    ] {
        verify_path_hash(&evidence, path_pointer, hash_pointer)?;
    }
    for item in evidence
        .pointer("/screenshots")
        .and_then(Value::as_array)
        .ok_or("screenshots are missing")?
    {
        verify_path_hash(item, "/path", "/sha256")?;
    }
    for item in evidence
        .pointer("/installedResources/externalTexturePages")
        .and_then(Value::as_array)
        .ok_or("installed textures are missing")?
    {
        verify_path_hash(item, "/path", "/sha256")?;
    }

    let inventory = read_json(&candidate_inventory_path)?;
    if compute_pack_inventory(&candidate_pack)? != inventory
        || sha256(&candidate_inventory_path)? != closure.candidate_pack_inventory_sha256
        || require_string(&inventory, "/packContentSha256")?
            != closure.candidate_pack_content_sha256
    {
        return Err("candidate Pack no longer matches its immutable inventory".into());
    }
    forge_pack::validate_pack_layout(&candidate_pack)?;
    if sha256(&candidate_pack.join("forgepack.json"))? != closure.candidate_pack_forgepack_sha256
        || sha256(&candidate_pack.join("assets/manifest.json"))?
            != closure.candidate_pack_manifest_sha256
        || sha256(&candidate_pack.join("quality/character-pack-provenance.json"))?
            != closure.candidate_pack_provenance_sha256
    {
        return Err("candidate Pack metadata closure mismatch".into());
    }

    let candidate_forgepack = read_json(&candidate_pack.join("forgepack.json"))?;
    let manifest = read_json(&candidate_pack.join("assets/manifest.json"))?;
    let godot_helper = read_json(&candidate_pack.join("assets/godot_import.json"))?;
    let candidate_provenance =
        read_json(&candidate_pack.join("quality/character-pack-provenance.json"))?;
    let runtime_config = read_json(&PathBuf::from(require_string(
        &evidence,
        "/runtimeConfigPath",
    )?))?;
    let runtime = read_json(&PathBuf::from(require_string(
        &evidence,
        "/runtimeReportPath",
    )?))?;
    if require_string(&candidate_forgepack, "/source/metadata/controlReviewStatus")? != "pending"
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
        )? != 2
        || require_u64(
            &candidate_forgepack,
            "/source/metadata/totalProviderRequestCount",
        )? != 2
        || require_string(&candidate_provenance, "/controlReviewStatus")? != "pending"
        || candidate_provenance
            .pointer("/productionEligible")
            .and_then(Value::as_bool)
            != Some(false)
    {
        return Err("candidate control/provider state is invalid".into());
    }
    for pointer in [
        "/runtimeAssertionPassed",
        "/componentPixelClosurePassed",
        "/wall/contactObserved",
        "/wall/frameFrozen",
    ] {
        require_true(&runtime, pointer)?;
    }
    if runtime
        .pointer("/errors")
        .and_then(Value::as_array)
        .map(Vec::is_empty)
        != Some(true)
        || require_string(&runtime, "/verdict")? != "pending_human_control_review"
    {
        return Err("runtime report is not a passing pending review".into());
    }
    let runtime_contract = validate_character_pack_control_runtime_contract(
        &candidate_forgepack,
        &manifest,
        &godot_helper,
        &candidate_provenance,
        &runtime_config,
        &runtime,
    )?;
    let summary = forge_pack::inspect_pack(&candidate_pack)?;
    if summary.frame_count != 48
        || summary.animations.len() != 2
        || summary.animations[0].name != "idle_right"
        || summary.animations[0].frame_count != 24
        || summary.animations[1].name != "walk_right"
        || summary.animations[1].frame_count != 24
    {
        return Err("candidate Pack summary is not idle_right + walk_right, 24 frames each".into());
    }
    let animations = manifest
        .pointer("/animations")
        .and_then(Value::as_array)
        .ok_or("manifest animations are missing")?;
    if animations.len() != 2
        || animations[0]["name"] != "idle_right"
        || animations[1]["name"] != "walk_right"
        || animations[0]
            .pointer("/frameDurationsMs")
            .and_then(Value::as_array)
            .map(Vec::len)
            != Some(24)
        || animations[1]
            .pointer("/frameDurationsMs")
            .and_then(Value::as_array)
            .map(Vec::len)
            != Some(24)
    {
        return Err("manifest animation timing closure is invalid".into());
    }
    let idle_hashes = candidate_provenance
        .pointer("/idleRight/frameSha256")
        .and_then(Value::as_array)
        .ok_or("idle component hashes are missing")?;
    let walk_hashes = candidate_provenance
        .pointer("/walkRight/frameSha256")
        .and_then(Value::as_array)
        .ok_or("walk component hashes are missing")?;
    if idle_hashes.len() != 24 || walk_hashes.len() != 24 {
        return Err("component frame closure must contain 24 + 24 hashes".into());
    }
    verify_component_review_semantics(
        &candidate_pack.join("quality/animation-human-review.json"),
        &closure.idle_right_approved_review_sha256,
        "idle_right",
        &candidate_provenance["idleRight"]["frameSha256"],
        &candidate_provenance["idleRight"]["frameDurationsMs"],
    )?;
    verify_component_review_semantics(
        &candidate_pack.join("quality/component-reviews/walk_right.json"),
        &closure.walk_right_approved_review_sha256,
        "walk_right",
        &candidate_provenance["walkRight"]["frameSha256"],
        &candidate_provenance["walkRight"]["frameDurationsMs"],
    )?;
    for (index, expected) in idle_hashes.iter().chain(walk_hashes.iter()).enumerate() {
        let path = candidate_pack
            .join("assets/frames")
            .join(format!("frame_{:03}.png", index + 1));
        if sha256(&path)? != expected.as_str().ok_or("invalid component frame hash")? {
            return Err(format!("candidate frame {} hash mismatch", index + 1).into());
        }
    }

    let approved_review_sha = sha256(&approved_review_path)?;
    let candidate_inventory_sha = sha256(&candidate_inventory_path)?;
    let evidence_sha = sha256(&evidence_path)?;
    let approved_at = approved
        .reviewed_at
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let mut production_rendering = runtime_contract.rendering.clone();
    if production_rendering
        .pointer("/mirrorPolicy")
        .and_then(Value::as_str)
        != Some("auto")
    {
        return Err("candidate rendering policy must remain the reviewed auto policy".into());
    }
    production_rendering["mirrorPolicy"] = json!("right_only");
    let promotion = json!({
        "schemaVersion": "1",
        "profile": "forge-character-pack-control-production-promotion@1.0.0",
        "status": "approved",
        "approvedAt": approved_at,
        "providerRequestCountThisOperation": 0,
        "inheritedProviderRequestCount": 2,
        "totalProviderRequestCount": 2,
        "characterPackControlReviewPath": approved_review_path,
        "characterPackControlReviewSha256": approved_review_sha,
        "godotControlEvidencePath": evidence_path,
        "godotControlEvidenceSha256": evidence_sha,
        "candidatePackPath": candidate_pack,
        "candidateForgepackSha256": closure.candidate_pack_forgepack_sha256,
        "candidateManifestSha256": closure.candidate_pack_manifest_sha256,
        "candidatePackInventoryPath": candidate_inventory_path,
        "candidatePackInventorySha256": candidate_inventory_sha,
        "candidatePackContentSha256": closure.candidate_pack_content_sha256,
        "candidatePackProvenanceSha256": closure.candidate_pack_provenance_sha256,
        "componentReviews": {
            "idle_right": closure.idle_right_approved_review_sha256,
            "walk_right": closure.walk_right_approved_review_sha256,
        },
        "animations": {
            "idle_right": {
                "frameCount": 24,
                "frameDurationsMs": runtime_contract.idle_right.frame_durations_ms,
                "cycleDurationMs": runtime_contract.idle_right.cycle_duration_ms,
            },
            "walk_right": {
                "frameCount": 24,
                "frameDurationsMs": runtime_contract.walk_right.frame_durations_ms,
                "cycleDurationMs": runtime_contract.walk_right.cycle_duration_ms,
            },
        },
        "sharedAnchor": {
            "type": "custom",
            "x": runtime_contract.anchor_x,
            "y": runtime_contract.anchor_y,
        },
        "candidateRendering": runtime_contract.rendering,
        "rendering": production_rendering,
        "collision": {
            "width": runtime_contract.collision_width,
            "height": runtime_contract.collision_height,
            "offsetX": runtime_contract.collision_offset_x,
            "offsetY": runtime_contract.collision_offset_y,
        },
        "visualScale": runtime_contract.visual_scale,
        "moveSpeedPixelsPerSecond": runtime_contract.move_speed_pixels_per_second,
        "availableDirections": ["right"],
        "leftMirrorAuthorized": false,
        "rightOnlyContract": true,
        "productionEligible": true,
        "productionPack": {
            "plannedPath": output,
            "id": production_id,
            "name": production_name,
            "version": production_version,
        },
    });
    let promotion_bytes = json_bytes(&promotion)?;
    let promotion_sha = format!("{:x}", Sha256::digest(&promotion_bytes));

    if recover_external_promotion {
        forge_pack::validate_pack_layout(&output)?;
        if fs::read(output.join("quality/character-pack-production-promotion.json"))?
            != promotion_bytes
        {
            return Err("existing production Pack does not contain the expected promotion".into());
        }
        let existing_forgepack = read_json(&output.join("forgepack.json"))?;
        if require_string(&existing_forgepack, "/id")? != production_id
            || require_string(&existing_forgepack, "/name")? != production_name
            || require_string(&existing_forgepack, "/version")? != production_version
            || require_string(
                &existing_forgepack,
                "/source/metadata/productionPromotionSha256",
            )? != promotion_sha
            || require_string(
                &existing_forgepack,
                "/source/metadata/productionMirrorPolicy",
            )? != "right_only"
        {
            return Err("existing production Pack identity or promotion closure mismatch".into());
        }
        write_new(&promotion_path, &promotion_bytes)?;
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "profile": "character-pack-control-production-promotion-recovery@1.0.0",
                "productionPack": output,
                "productionPromotionSha256": promotion_sha,
                "recoveredExternalPromotion": true,
                "providerRequestCountThisOperation": 0,
            }))?
        );
        return Ok(());
    }

    let parent = output.parent().ok_or("production Pack has no parent")?;
    fs::create_dir_all(parent)?;
    let temporary = tempfile::Builder::new()
        .prefix(".forge-character-pack-production-")
        .tempdir_in(parent)?;
    let staged = temporary.path().join("pack.gsfpack");
    copy_tree(&candidate_pack, &staged)?;

    let manifest_path = staged.join("assets/manifest.json");
    let mut production_manifest = read_json(&manifest_path)?;
    production_manifest["rendering"] = production_rendering.clone();
    write_json(&manifest_path, &production_manifest)?;

    let godot_helper_path = staged.join("assets/godot_import.json");
    let mut production_godot_helper = read_json(&godot_helper_path)?;
    production_godot_helper["spriteFrames"]["rendering"] = production_rendering.clone();
    write_json(&godot_helper_path, &production_godot_helper)?;

    let quality = staged.join("quality");
    fs::copy(
        quality.join("character-pack-provenance.json"),
        quality.join("candidate-character-pack-provenance.json"),
    )?;
    fs::copy(
        &approved_review_path,
        quality.join("character-pack-control-human-review.json"),
    )?;
    fs::copy(
        &evidence_path,
        quality.join("godot-control-review-evidence.json"),
    )?;
    fs::write(
        quality.join("character-pack-production-promotion.json"),
        &promotion_bytes,
    )?;
    fs::copy(
        &candidate_inventory_path,
        quality.join("candidate-pack-inventory.json"),
    )?;

    let mut production_provenance = candidate_provenance.clone();
    production_provenance["profile"] =
        json!("approved-component-character-pack-production-provenance@1.0.0");
    production_provenance["controlReviewStatus"] = json!("approved");
    production_provenance["productionEligible"] = json!(true);
    production_provenance["approvedAt"] = json!(approved_at);
    production_provenance["characterPackControlReviewSha256"] = json!(approved_review_sha);
    production_provenance["godotControlEvidenceSha256"] = json!(evidence_sha);
    production_provenance["productionPromotionSha256"] = json!(promotion_sha);
    production_provenance["candidatePackInventorySha256"] = json!(candidate_inventory_sha);
    production_provenance["candidatePackContentSha256"] =
        json!(closure.candidate_pack_content_sha256);
    production_provenance["candidateForgepackSha256"] =
        json!(closure.candidate_pack_forgepack_sha256);
    production_provenance["candidatePackProvenanceSha256"] =
        json!(closure.candidate_pack_provenance_sha256);
    production_provenance["rendering"] = production_rendering.clone();
    production_provenance["availableDirections"] = json!(["right"]);
    production_provenance["leftMirrorAuthorized"] = json!(false);
    production_provenance["rightOnlyContract"] = json!(true);
    production_provenance["componentReviewDocuments"] = json!({
        "idle_right": {
            "path": "quality/animation-human-review.json",
            "sha256": closure.idle_right_approved_review_sha256,
            "scope": "idle_right",
        },
        "walk_right": {
            "path": "quality/component-reviews/walk_right.json",
            "sha256": closure.walk_right_approved_review_sha256,
            "scope": "walk_right",
        },
    });
    production_provenance["controlReviewDocument"] = json!({
        "path": "quality/character-pack-control-human-review.json",
        "sha256": approved_review_sha,
        "scope": CHARACTER_PACK_CONTROL_REVIEW_SCOPE_V1,
    });
    let production_provenance_path = quality.join("character-pack-provenance.json");
    write_json(&production_provenance_path, &production_provenance)?;
    let production_provenance_sha = sha256(&production_provenance_path)?;

    let forgepack_path = staged.join("forgepack.json");
    let mut forgepack = candidate_forgepack;
    forgepack["id"] = json!(production_id);
    forgepack["name"] = json!(production_name);
    forgepack["version"] = json!(production_version);
    forgepack["createdAt"] = json!(approved_at);
    forgepack["license"]["type"] =
        json!("User-owned provider-generated components; composite control human-approved");
    let metadata = forgepack
        .pointer_mut("/source/metadata")
        .and_then(Value::as_object_mut)
        .ok_or("candidate Pack source metadata is missing")?;
    metadata.insert("controlReviewStatus".into(), json!("approved"));
    metadata.insert(
        "characterPackControlReviewSha256".into(),
        json!(approved_review_sha),
    );
    metadata.insert(
        "godotControlReviewEvidenceSha256".into(),
        json!(evidence_sha),
    );
    metadata.insert("productionPromotionSha256".into(), json!(promotion_sha));
    metadata.insert(
        "candidatePackInventorySha256".into(),
        json!(candidate_inventory_sha),
    );
    metadata.insert(
        "candidatePackContentSha256".into(),
        json!(closure.candidate_pack_content_sha256),
    );
    metadata.insert(
        "candidateForgepackSha256".into(),
        json!(closure.candidate_pack_forgepack_sha256),
    );
    metadata.insert(
        "candidatePackProvenanceSha256".into(),
        json!(closure.candidate_pack_provenance_sha256),
    );
    metadata.insert(
        "characterPackProvenanceSha256".into(),
        json!(production_provenance_sha),
    );
    metadata.insert("availableDirections".into(), json!(["right"]));
    metadata.insert("leftMirrorAuthorized".into(), json!(false));
    metadata.insert("rightOnlyContract".into(), json!(true));
    metadata.insert("productionMirrorPolicy".into(), json!("right_only"));
    metadata.insert(
        "animationHumanReviewScope".into(),
        json!("idle_right_component_only"),
    );
    metadata.insert(
        "componentReviews".into(),
        json!({
            "idle_right": {
                "path": "quality/animation-human-review.json",
                "sha256": closure.idle_right_approved_review_sha256,
            },
            "walk_right": {
                "path": "quality/component-reviews/walk_right.json",
                "sha256": closure.walk_right_approved_review_sha256,
            },
        }),
    );
    metadata.insert(
        "controlReviewDocument".into(),
        json!({
            "path": "quality/character-pack-control-human-review.json",
            "sha256": approved_review_sha,
        }),
    );
    metadata.insert("productionEligible".into(), json!(true));
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
            Some("composite_control_review_pending" | "production_eligible_false")
        )
    });
    for note in [
        "composite_control_review_approved",
        "godot_control_evidence_verified",
        "mirror_policy_right_only",
        "right_direction_only_no_left_mirror_authorization",
        "production_eligible_true",
    ] {
        if !notes.iter().any(|value| value.as_str() == Some(note)) {
            notes.push(json!(note));
        }
    }
    write_json(&quality_report_path, &quality_report)?;

    forge_pack::validate_pack_layout(&staged)?;
    let sealed_manifest = read_json(&staged.join("assets/manifest.json"))?;
    let sealed_helper = read_json(&staged.join("assets/godot_import.json"))?;
    if require_string(&sealed_manifest, "/rendering/mirrorPolicy")? != "right_only"
        || require_string(&sealed_helper, "/spriteFrames/rendering/mirrorPolicy")? != "right_only"
        || require_string(&production_provenance, "/rendering/mirrorPolicy")? != "right_only"
    {
        return Err("production Pack did not seal the right_only rendering contract".into());
    }
    let production_summary = forge_pack::inspect_pack(&staged)?;
    if production_summary.frame_count != 48
        || production_summary.animations.len() != 2
        || production_summary.animations[0].name != "idle_right"
        || production_summary.animations[0].frame_count != 24
        || production_summary.animations[1].name != "walk_right"
        || production_summary.animations[1].frame_count != 24
    {
        return Err("production Pack summary mismatch".into());
    }
    for (relative, expected) in [
        (
            "quality/character-pack-control-human-review.json",
            approved_review_sha.as_str(),
        ),
        (
            "quality/godot-control-review-evidence.json",
            evidence_sha.as_str(),
        ),
        (
            "quality/character-pack-production-promotion.json",
            promotion_sha.as_str(),
        ),
        (
            "quality/candidate-pack-inventory.json",
            candidate_inventory_sha.as_str(),
        ),
        (
            "quality/candidate-character-pack-provenance.json",
            closure.candidate_pack_provenance_sha256.as_str(),
        ),
        (
            "quality/character-pack-provenance.json",
            production_provenance_sha.as_str(),
        ),
    ] {
        if sha256(&staged.join(relative))? != expected {
            return Err(format!("embedded production evidence hash mismatch: {relative}").into());
        }
    }
    for (index, expected) in idle_hashes.iter().chain(walk_hashes.iter()).enumerate() {
        if sha256(
            &staged
                .join("assets/frames")
                .join(format!("frame_{:03}.png", index + 1)),
        )? != expected.as_str().ok_or("invalid component frame hash")?
        {
            return Err(format!("production frame {} hash mismatch", index + 1).into());
        }
    }

    fs::rename(&staged, &output)?;
    write_new(&promotion_path, &promotion_bytes)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "profile": "character-pack-control-production-promotion-result@1.0.0",
            "productionPack": output,
            "frameCount": 48,
            "animations": ["idle_right", "walk_right"],
            "characterPackControlReviewSha256": approved_review_sha,
            "godotControlEvidenceSha256": evidence_sha,
            "productionPromotionSha256": promotion_sha,
            "productionProvenanceSha256": production_provenance_sha,
            "productionEligible": true,
            "providerRequestCountThisOperation": 0,
            "inheritedProviderRequestCount": 2,
            "totalProviderRequestCount": 2,
            "availableDirections": ["right"],
            "leftMirrorAuthorized": false,
            "mirrorPolicy": "right_only",
        }))?
    );
    Ok(())
}
