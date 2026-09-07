use std::env;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use forge_core::character_animation_review::{
    review_is_approved, validate_animation_review_shape, AnimationHumanReviewV1,
};
use forge_core::character_pack_control_review::{
    approve_character_pack_control_review, character_pack_control_review_is_approved,
    validate_character_pack_control_review_closure,
    validate_character_pack_control_runtime_contract, CharacterPackControlReviewClosureV1,
    CHARACTER_PACK_CONTROL_REVIEW_SCOPE_V1,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn sha256(path: &Path) -> Result<String, Box<dyn Error>> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

fn read_json(path: &Path) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
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

fn duration_array(value: &Value, pointer: &str) -> Result<Vec<u64>, Box<dyn Error>> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .ok_or_else(|| -> Box<dyn Error> { format!("missing duration array at {pointer}").into() })?
        .iter()
        .map(|item| {
            item.as_u64()
                .or_else(|| {
                    item.as_f64()
                        .filter(|number| number.fract() == 0.0)
                        .map(|number| number as u64)
                })
                .ok_or_else(|| -> Box<dyn Error> {
                    format!("invalid duration in {pointer}").into()
                })
        })
        .collect()
}

fn verify_component(
    provenance: &Value,
    pointer: &str,
    expected_animation: &str,
    candidate_pack: &Path,
    candidate_start_index: usize,
) -> Result<(), Box<dyn Error>> {
    let component = provenance
        .pointer(pointer)
        .ok_or_else(|| format!("missing component at {pointer}"))?;
    let production_pack = PathBuf::from(require_string(component, "/productionPackPath")?);
    let component_inventory_path = PathBuf::from(require_string(component, "/packInventoryPath")?);
    let component_inventory = read_json(&component_inventory_path)?;
    if sha256(&production_pack.join("forgepack.json"))?
        != require_string(component, "/forgepackSha256")?
        || sha256(&component_inventory_path)? != require_string(component, "/packInventorySha256")?
        || compute_pack_inventory(&production_pack)? != component_inventory
        || require_string(&component_inventory, "/packContentSha256")?
            != require_string(component, "/packContentSha256")?
    {
        return Err(format!("component Pack closure mismatch at {pointer}").into());
    }
    let approved_review = PathBuf::from(require_string(component, "/approvedReviewPath")?);
    if sha256(&approved_review)? != require_string(component, "/approvedReviewSha256")? {
        return Err(format!("component review closure mismatch at {pointer}").into());
    }
    let review: AnimationHumanReviewV1 = serde_json::from_slice(&fs::read(&approved_review)?)?;
    validate_animation_review_shape(&review)?;
    let hashes = component
        .pointer("/frameSha256")
        .and_then(Value::as_array)
        .ok_or("component frame hashes are missing")?;
    if hashes.len() != 24 {
        return Err("component must contain 24 approved frames".into());
    }
    let expected_hashes = hashes
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or("invalid component frame hash")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let expected_durations = duration_array(component, "/frameDurationsMs")?;
    if !review_is_approved(&review)
        || review.animation != expected_animation
        || review.frame_sha256 != expected_hashes
        || review.frame_durations_ms != expected_durations
    {
        return Err(format!("component review is not semantically approved at {pointer}").into());
    }
    for (offset, expected) in hashes.iter().enumerate() {
        let candidate_frame = candidate_pack.join("assets/frames").join(format!(
            "frame_{:03}.png",
            candidate_start_index + offset + 1
        ));
        let component_frame = production_pack
            .join("assets/frames")
            .join(format!("frame_{:03}.png", offset + 1));
        let expected = expected.as_str().ok_or("invalid component frame hash")?;
        if sha256(&candidate_frame)? != expected || sha256(&component_frame)? != expected {
            return Err(format!("component frame {} closure mismatch", offset + 1).into());
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.len() != 4 {
        return Err("usage: approve_character_pack_control_review <godot-evidence> <approved-review> <reviewed-at-rfc3339> <review-note>".into());
    }
    let evidence_path = PathBuf::from(&arguments[0]);
    let output = PathBuf::from(&arguments[1]);
    let reviewed_at_raw = arguments[2]
        .to_str()
        .ok_or("reviewed-at must be valid UTF-8")?;
    let review_note = arguments[3]
        .to_str()
        .ok_or("review note must be valid UTF-8")?;
    if output.exists() {
        return Err(format!("refusing to overwrite {}", output.display()).into());
    }

    let evidence = read_json(&evidence_path)?;
    if require_string(&evidence, "/profile")? != "godot-composite-control-review-evidence@1.0.0"
        || require_string(&evidence, "/status")? != "pending_human_control_review"
        || evidence
            .pointer("/productionEligible")
            .and_then(Value::as_bool)
            != Some(false)
        || require_u64(&evidence, "/providerRequestCountThisOperation")? != 0
    {
        return Err("Godot composite evidence is not a pending zero-request review".into());
    }
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
    if scope != CHARACTER_PACK_CONTROL_REVIEW_SCOPE_V1.map(str::to_owned) {
        return Err("Godot evidence human-review scope is not the approved six-check scope".into());
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
    for screenshot in evidence
        .pointer("/screenshots")
        .and_then(Value::as_array)
        .ok_or("screenshots are missing")?
    {
        verify_path_hash(screenshot, "/path", "/sha256")?;
    }
    for texture in evidence
        .pointer("/installedResources/externalTexturePages")
        .and_then(Value::as_array)
        .ok_or("external texture pages are missing")?
    {
        verify_path_hash(texture, "/path", "/sha256")?;
    }

    let candidate_pack = PathBuf::from(require_string(&evidence, "/characterPackPath")?);
    forge_pack::validate_pack_layout(&candidate_pack)?;
    if sha256(&candidate_pack.join("forgepack.json"))?
        != require_string(&evidence, "/characterPackForgepackSha256")?
        || sha256(&candidate_pack.join("assets/manifest.json"))?
            != require_string(&evidence, "/characterPackManifestSha256")?
    {
        return Err("candidate Pack metadata changed after review".into());
    }
    let inventory_path = PathBuf::from(require_string(&evidence, "/characterPackInventoryPath")?);
    let inventory = read_json(&inventory_path)?;
    if compute_pack_inventory(&candidate_pack)? != inventory
        || require_string(&inventory, "/packContentSha256")?
            != require_string(&evidence, "/characterPackContentSha256")?
    {
        return Err("candidate Pack no longer matches its reviewed inventory".into());
    }

    let forgepack = read_json(&candidate_pack.join("forgepack.json"))?;
    let manifest = read_json(&candidate_pack.join("assets/manifest.json"))?;
    let godot_helper = read_json(&candidate_pack.join("assets/godot_import.json"))?;
    let provenance = read_json(&candidate_pack.join("quality/character-pack-provenance.json"))?;
    let runtime_config = read_json(&PathBuf::from(require_string(
        &evidence,
        "/runtimeConfigPath",
    )?))?;
    let runtime = read_json(&PathBuf::from(require_string(
        &evidence,
        "/runtimeReportPath",
    )?))?;
    if require_string(&forgepack, "/source/metadata/controlReviewStatus")? != "pending"
        || forgepack
            .pointer("/source/metadata/productionEligible")
            .and_then(Value::as_bool)
            != Some(false)
        || require_string(&provenance, "/controlReviewStatus")? != "pending"
        || provenance
            .pointer("/productionEligible")
            .and_then(Value::as_bool)
            != Some(false)
        || require_string(&runtime, "/verdict")? != "pending_human_control_review"
        || runtime
            .pointer("/errors")
            .and_then(Value::as_array)
            .map(Vec::is_empty)
            != Some(true)
    {
        return Err("candidate composite state is not pending and immutable".into());
    }
    for pointer in [
        "/runtimeAssertionPassed",
        "/componentPixelClosurePassed",
        "/wall/contactObserved",
        "/wall/frameFrozen",
    ] {
        require_true(&runtime, pointer)?;
    }
    require_true(
        &evidence,
        "/runtimeAssertions/bothComponentAtlasPixelSetsClosed",
    )?;
    require_true(&evidence, "/runtimeAssertions/wallContactObserved")?;
    require_true(&evidence, "/runtimeAssertions/wallAnimationFrozen")?;
    if require_u64(&evidence, "/runtimeAssertions/idleFramesSeen")? != 24
        || require_u64(&evidence, "/runtimeAssertions/walkFramesSeen")? != 24
        || require_u64(&evidence, "/runtimeAssertions/pressCount")? != 4
        || require_u64(&evidence, "/runtimeAssertions/releaseCount")? != 4
        || evidence
            .pointer("/runtimeAssertions/errors")
            .and_then(Value::as_array)
            .map(Vec::is_empty)
            != Some(true)
    {
        return Err("runtime assertions do not close over the reviewed control sequence".into());
    }

    validate_character_pack_control_runtime_contract(
        &forgepack,
        &manifest,
        &godot_helper,
        &provenance,
        &runtime_config,
        &runtime,
    )?;

    let summary = forge_pack::inspect_pack(&candidate_pack)?;
    if summary.frame_count != 48 || summary.animations.len() != 2 {
        return Err("candidate Pack must contain two 24-frame animations".into());
    }
    for (name, pointer) in [
        ("idle_right", "/animations/idle_right/frameDurationsMs"),
        ("walk_right", "/animations/walk_right/frameDurationsMs"),
    ] {
        let manifest_animation = manifest
            .pointer("/animations")
            .and_then(Value::as_array)
            .and_then(|animations| {
                animations
                    .iter()
                    .find(|animation| animation["name"] == name)
            })
            .ok_or_else(|| format!("manifest animation {name} is missing"))?;
        let durations = duration_array(manifest_animation, "/frameDurationsMs")?;
        if durations.len() != 24 || durations != duration_array(&runtime, pointer)? {
            return Err(format!("installed duration closure mismatch for {name}").into());
        }
    }

    verify_component(&provenance, "/idleRight", "idle_right", &candidate_pack, 0)?;
    verify_component(&provenance, "/walkRight", "walk_right", &candidate_pack, 24)?;
    if sha256(&candidate_pack.join("quality/animation-human-review.json"))?
        != require_string(&evidence, "/idleRightApprovedReviewSha256")?
        || sha256(&candidate_pack.join("quality/component-reviews/walk_right.json"))?
            != require_string(&evidence, "/walkRightApprovedReviewSha256")?
    {
        return Err("embedded component reviews changed after review".into());
    }

    for pointer in [
        "/installedResources/spriteFramesPath",
        "/installedResources/animatedSpriteScenePath",
    ] {
        let path = PathBuf::from(require_string(&evidence, pointer)?);
        if fs::metadata(&path)?.len() >= 1024 * 1024 {
            return Err(format!("installed Godot resource is >= 1 MiB: {}", path.display()).into());
        }
        let text = fs::read_to_string(&path)?;
        for marker in [
            "PackedByteArray",
            "ImageTexture.create_from_image",
            "data:image",
            "base64",
        ] {
            if text.contains(marker) {
                return Err(format!(
                    "installed Godot resource embeds raster data: {}",
                    path.display()
                )
                .into());
            }
        }
    }

    let closure = CharacterPackControlReviewClosureV1 {
        godot_evidence_sha256: sha256(&evidence_path)?,
        candidate_pack_forgepack_sha256: require_string(
            &evidence,
            "/characterPackForgepackSha256",
        )?
        .into(),
        candidate_pack_manifest_sha256: require_string(&evidence, "/characterPackManifestSha256")?
            .into(),
        candidate_pack_inventory_sha256: require_string(
            &evidence,
            "/characterPackInventorySha256",
        )?
        .into(),
        candidate_pack_content_sha256: require_string(&evidence, "/characterPackContentSha256")?
            .into(),
        candidate_pack_provenance_sha256: require_string(
            &evidence,
            "/characterPackProvenanceSha256",
        )?
        .into(),
        idle_right_approved_review_sha256: require_string(
            &evidence,
            "/idleRightApprovedReviewSha256",
        )?
        .into(),
        walk_right_approved_review_sha256: require_string(
            &evidence,
            "/walkRightApprovedReviewSha256",
        )?
        .into(),
        runtime_report_sha256: require_string(&evidence, "/runtimeReportSha256")?.into(),
        authoritative_movie_sha256: require_string(&evidence, "/authoritativeMovieSha256")?.into(),
        delivery_movie_sha256: require_string(&evidence, "/deliveryMovieSha256")?.into(),
        human_review_scope: scope,
    };
    let reviewed_at = DateTime::parse_from_rfc3339(reviewed_at_raw)?.with_timezone(&Utc);
    let approved = approve_character_pack_control_review(&closure, review_note, reviewed_at)?;
    validate_character_pack_control_review_closure(&approved, &closure)?;
    if !character_pack_control_review_is_approved(&approved) {
        return Err("Forge control-review API did not produce an approved review".into());
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut bytes = serde_json::to_vec_pretty(&approved)?;
    bytes.push(b'\n');
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "profile": "character-pack-control-approval-result@1.0.0",
            "approvedReview": output,
            "approvedReviewSha256": format!("{:x}", Sha256::digest(&bytes)),
            "godotEvidenceSha256": closure.godot_evidence_sha256,
            "sixChecksPassed": true,
            "providerRequestCount": 0,
        }))?
    );
    Ok(())
}
