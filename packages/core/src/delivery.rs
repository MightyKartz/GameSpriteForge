//! Portable byte inventories and read-only installed-resource audits.
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const INSTALL_SNAPSHOT: &str = ".forge-install.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileDigest {
    pub path: PathBuf,
    pub bytes: u64,
    pub sha256: String,
}

pub fn hash_file(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

pub fn inventory(root: &Path) -> io::Result<Vec<FileDigest>> {
    fn visit(root: &Path, path: &Path, files: &mut Vec<FileDigest>) -> io::Result<()> {
        let metadata = fs::symlink_metadata(path)?;
        if metadata.file_type().is_symlink() {
            return Err(invalid(format!(
                "inventory contains symbolic link: {}",
                path.display()
            )));
        }
        if metadata.is_dir() {
            for entry in fs::read_dir(path)? {
                visit(root, &entry?.path(), files)?;
            }
        } else if metadata.is_file() {
            files.push(FileDigest {
                path: path.strip_prefix(root).map_err(invalid)?.to_path_buf(),
                bytes: metadata.len(),
                sha256: hash_file(path)?,
            });
        } else {
            return Err(invalid(format!(
                "unsupported inventory entry: {}",
                path.display()
            )));
        }
        Ok(())
    }
    if !root.is_dir() {
        return Err(invalid("inventory root must be a directory"));
    }
    let mut files = Vec::new();
    visit(root, root, &mut files)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// Matches the existing Forge directory hash v2 used by plans and project registries.
pub fn directory_sha256(root: &Path) -> io::Result<String> {
    let mut digest = Sha256::new();
    digest.update(b"forge-directory-hash-v2\0");
    for file in inventory(root)? {
        let name = file.path.to_string_lossy();
        let bytes = fs::read(root.join(&file.path))?;
        digest.update(b"file\0");
        digest.update((name.len() as u64).to_le_bytes());
        digest.update(name.as_bytes());
        digest.update((bytes.len() as u64).to_le_bytes());
        digest.update(bytes);
    }
    Ok(format!("{:x}", digest.finalize()))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallSnapshot {
    pub schema_version: String,
    pub files: Vec<FileDigest>,
    pub import_files: Vec<FileDigest>,
    pub cache_files: Vec<FileDigest>,
}

pub fn installed_inventory(target: &Path) -> io::Result<Vec<FileDigest>> {
    Ok(inventory(target)?
        .into_iter()
        .filter(|file| {
            file.path != Path::new(INSTALL_SNAPSHOT)
                && file.path.extension().is_none_or(|ext| ext != "import")
        })
        .collect())
}

/// Called only inside the installation transaction after native resource verification.
pub fn write_install_snapshot(project: &Path, target: &Path) -> io::Result<()> {
    let cache_files = crate::godot_cache::cache_files_for_target(project, target)?
        .into_iter()
        .map(|path| {
            let absolute = project.join(&path);
            Ok(FileDigest {
                bytes: fs::metadata(&absolute)?.len(),
                sha256: hash_file(&absolute)?,
                path,
            })
        })
        .collect::<io::Result<Vec<_>>>()?;
    let snapshot = InstallSnapshot {
        schema_version: "2".into(),
        files: installed_inventory(target)?,
        import_files: inventory(target)?
            .into_iter()
            .filter(|file| file.path.extension().is_some_and(|ext| ext == "import"))
            .collect(),
        cache_files,
    };
    fs::write(
        target.join(INSTALL_SNAPSHOT),
        serde_json::to_vec_pretty(&snapshot)?,
    )
}

pub fn safe_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path.to_str().is_some()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

pub(crate) fn godot_resource_path(relative: &Path) -> String {
    format!("res://{}", relative.to_string_lossy().replace('\\', "/"))
}

pub fn safe_project_target(project: &Path, relative: &Path) -> io::Result<PathBuf> {
    if !safe_relative(relative)
        || !relative.starts_with("addons/forge_assets")
        || relative.components().count() < 3
    {
        return Err(invalid(
            "installed target must be a directory below addons/forge_assets",
        ));
    }
    let mut path = fs::canonicalize(project)?;
    for part in relative.components() {
        path.push(part);
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            return Err(invalid("installed target may not traverse symbolic links"));
        }
    }
    Ok(path)
}

/// Does not start Godot, reimport textures, write caches or update any manifests.
pub fn verify_install(
    project: &Path,
    asset_key: &str,
    pack_override: Option<&Path>,
) -> io::Result<Value> {
    let manifest = crate::project::read_project_manifest(project).map_err(invalid)?;
    let entry = manifest
        .assets
        .get(asset_key)
        .ok_or_else(|| invalid("asset key is not installed"))?;
    let target = safe_project_target(project, &entry.godot_target)?;
    let snapshot_path = target.join(INSTALL_SNAPSHOT);
    let registered_snapshot_sha256 = entry.install_snapshot_sha256.as_deref().ok_or_else(|| invalid(
        "installation has no independently registered snapshot hash; legacy installs require a separately retained delivery receipt"))?;
    let snapshot_sha256 = hash_file(&snapshot_path)?;
    if registered_snapshot_sha256 != snapshot_sha256 {
        return Err(invalid(
            "installation baseline differs from its registered SHA-256",
        ));
    }
    let snapshot: InstallSnapshot = read_json(&snapshot_path).map_err(|e| invalid(format!("installation baseline unavailable or invalid: {e}; legacy installs require a separately retained delivery receipt")))?;
    if snapshot.schema_version != "2" || snapshot.files.is_empty() {
        return Err(invalid("unsupported or empty installation baseline"));
    }
    let actual = installed_inventory(&target)?;
    if actual != snapshot.files {
        return Err(invalid(
            "installed resources differ from the recorded installation baseline",
        ));
    }
    let cache_status = verify_cache_baseline(project, &target, &snapshot)?;
    let usage: Value = read_json(&target.join("forge_usage.json"))?;
    let owner: Value = read_json(&target.join(".forge-owned.json"))?;
    if usage["schemaVersion"] != "1"
        || owner["schemaVersion"] != "1"
        || usage["assetKey"] != asset_key
        || owner["assetKey"] != asset_key
        || usage["assetId"] != entry.asset_id
        || usage["packSha256"] != entry.pack_sha256
        || owner["jobId"] != entry.last_job_id
        || owner["owner"] != "Game Sprite Forge"
        || owner["projectManifest"] != crate::project::PROJECT_MANIFEST_RELATIVE
        || entry.usage_path != entry.godot_target.join("forge_usage.json")
    {
        return Err(invalid(
            "installed ownership, usage and project registry disagree",
        ));
    }
    for (field, relative) in [
        ("scenePath", &entry.scene_path),
        ("spriteFramesPath", &entry.sprite_frames_path),
    ] {
        // Earlier Windows builds emitted native separators in usage JSON.
        // Keep their recorded bytes intact while comparing the same resource URI.
        let recorded_uri = usage[field].as_str().map(|uri| uri.replace('\\', "/"));
        if !relative.starts_with(&entry.godot_target)
            || !safe_relative(relative)
            || recorded_uri != Some(godot_resource_path(relative))
            || !project.join(relative).exists()
        {
            return Err(invalid(format!(
                "installed {field} disagrees with registry or is missing"
            )));
        }
    }
    let pack = pack_override
        .map(Path::to_path_buf)
        .unwrap_or_else(|| match entry.pack.kind {
            crate::project::ProjectPathKind::ProjectRelative => project.join(&entry.pack.path),
            crate::project::ProjectPathKind::ExternalAbsolute => entry.pack.path.clone(),
        });
    match entry.pack.kind {
        crate::project::ProjectPathKind::ProjectRelative if !safe_relative(&entry.pack.path) => {
            return Err(invalid("registered project-relative Pack path is unsafe"))
        }
        crate::project::ProjectPathKind::ExternalAbsolute if !entry.pack.path.is_absolute() => {
            return Err(invalid("registered external Pack path must be absolute"))
        }
        _ => {}
    }
    if pack_override.is_none()
        && entry.pack.kind == crate::project::ProjectPathKind::ProjectRelative
        && !fs::canonicalize(&pack)?.starts_with(fs::canonicalize(project)?)
    {
        return Err(invalid(
            "registered project-relative Pack escapes the project",
        ));
    }
    if directory_sha256(&pack)? != entry.pack_sha256 {
        return Err(invalid(
            "original Pack bytes differ from the installed packSha256",
        ));
    }
    forge_pack::validate_pack_layout(&pack).map_err(invalid)?;
    let summary = forge_pack::inspect_pack(&pack).map_err(invalid)?;
    if entry.asset_id != summary.id
        || entry.name != summary.name
        || usage["kind"] != summary.asset_type
        || entry.default_animation != summary.default_animation
        || usage["defaultAnimation"] != summary.default_animation
        || entry.animations.len() != summary.animations.len()
        || usage["items"] != serde_json::to_value(&summary.items).map_err(invalid)?
    {
        return Err(invalid(
            "installed identity or animation contract differs from the original Pack",
        ));
    }
    let usage_animations = usage["animations"]
        .as_array()
        .ok_or_else(|| invalid("usage animations missing"))?;
    if usage_animations.len() != summary.animations.len() {
        return Err(invalid(
            "installed animation count differs from the original Pack",
        ));
    }
    for ((expected, registered), installed) in summary
        .animations
        .iter()
        .zip(&entry.animations)
        .zip(usage_animations)
    {
        if registered.name != expected.name
            || registered.frame_count != expected.frame_count
            || registered.fps != expected.fps
            || registered.loop_animation != expected.loop_animation
            || installed["name"] != expected.name
            || installed["frameCount"] != expected.frame_count
            || installed["fps"] != serde_json::json!(expected.fps)
            || installed["loop"] != expected.loop_animation
        {
            return Err(invalid(
                "installed animation timing differs from the original Pack",
            ));
        }
    }
    let (scene, frames) = match summary.asset_type.as_str() {
        "icon_set" => ("items", "items"),
        "prop_set" => ("scenes", "items"),
        "terrain_set" => ("forge_terrain_preview.tscn", "forge_terrain_set.tres"),
        "building_kit" => ("scenes", "forge_building_kit.tres"),
        "map" => ("forge_world.tscn", "forge_terrain_set.tres"),
        _ => ("forge_animated_sprite.tscn", "forge_sprite_frames.tres"),
    };
    if entry.scene_path != entry.godot_target.join(scene)
        || entry.sprite_frames_path != entry.godot_target.join(frames)
    {
        return Err(invalid(
            "registered native resource paths differ from the Pack delivery contract",
        ));
    }
    let helper: Value = read_json(&pack.join("assets/godot_import.json"))?;
    let spec = if matches!(summary.asset_type.as_str(), "animation" | "character") {
        &helper["spriteFrames"]
    } else {
        &helper
    };
    for key in ["rendering", "anchor", "frameWidth", "frameHeight"] {
        if let Some(expected) = spec.get(key) {
            if usage.get(key) != Some(expected) {
                return Err(invalid(format!(
                    "installed {key} differs from the original Pack"
                )));
            }
        }
    }
    if let Some(animations) = spec["animations"].as_array() {
        for animation in animations {
            if let Some(durations) = animation.get("frameDurationsMs") {
                let installed = usage_animations
                    .iter()
                    .find(|item| item["name"] == animation["name"])
                    .ok_or_else(|| invalid("usage animation missing"))?;
                if installed.get("frameDurationsMs") != Some(durations) {
                    return Err(invalid(
                        "installed frame durations differ from the original Pack",
                    ));
                }
            }
        }
    }
    let mut textures = BTreeMap::<PathBuf, PathBuf>::new();
    if matches!(summary.asset_type.as_str(), "icon_set" | "prop_set") {
        let items = helper["items"]
            .as_array()
            .ok_or_else(|| invalid("Pack items missing"))?;
        for item in items {
            let id = item["id"]
                .as_str()
                .ok_or_else(|| invalid("Pack item has no id"))?;
            let source = item["texture"]
                .as_str()
                .ok_or_else(|| invalid("Pack item has no texture"))?;
            if id.is_empty()
                || !id
                    .bytes()
                    .all(|value| value.is_ascii_alphanumeric() || value == b'-' || value == b'_')
            {
                return Err(invalid("invalid Pack item ID"));
            }
            insert_texture(
                &mut textures,
                PathBuf::from(source),
                PathBuf::from(format!("items/{id}.png")),
            )?;
        }
    } else {
        let mut sources = Vec::new();
        if let Some(items) = helper
            .pointer("/spriteFrames/textures")
            .and_then(Value::as_array)
        {
            for value in items {
                sources.push(
                    value
                        .as_str()
                        .ok_or_else(|| invalid("Pack texture must be a string"))?,
                );
            }
        }
        for key in ["atlas", "terrainAtlas", "buildingAtlas"] {
            if let Some(path) = helper[key].as_str() {
                sources.push(path);
            }
        }
        if let Some(items) = helper["propTextures"].as_array() {
            for value in items {
                sources.push(
                    value["texture"]
                        .as_str()
                        .ok_or_else(|| invalid("Pack prop texture missing"))?,
                );
            }
        }
        for source in sources {
            let source = PathBuf::from(source);
            let name = source
                .file_name()
                .ok_or_else(|| invalid("texture filename missing"))?
                .into();
            insert_texture(&mut textures, source, name)?;
        }
    }
    if textures.is_empty() {
        return Err(invalid("Pack has no source texture mapping"));
    }
    for (installed, source) in &textures {
        if !safe_relative(source)
            || !safe_relative(installed)
            || hash_file(&pack.join(source))? != hash_file(&target.join(installed))?
        {
            return Err(invalid(format!(
                "installed texture differs from Pack: {}",
                installed.display()
            )));
        }
    }
    Ok(
        json!({"schemaVersion":"1", "assetKey":asset_key, "target":target, "pack":pack,
        "packSha256":entry.pack_sha256, "verifiedFiles":actual.len(), "verifiedTextures":textures.len(),
        "nativeLoad":"not_run", "visualReview":"not_assessed", "readOnly":true,
        "cacheCheck":cache_status,
        "snapshotSha256":snapshot_sha256,
        "baselineAuthority":"project_registry",
        "notes":["This detects drift against the registered installation baseline and original Pack. An external retained receipt hash is required to detect coordinated changes to every local record."]}),
    )
}

fn verify_cache_baseline(
    project: &Path,
    target: &Path,
    snapshot: &InstallSnapshot,
) -> io::Result<Value> {
    let actual_imports = inventory(target)?
        .into_iter()
        .filter(|file| file.path.extension().is_some_and(|ext| ext == "import"))
        .collect::<Vec<_>>();
    let mut missing = 0;
    for actual in &actual_imports {
        if !snapshot.import_files.contains(actual) {
            return Err(invalid(
                "Godot import settings or routing differ from the installation baseline",
            ));
        }
    }
    missing += snapshot
        .import_files
        .len()
        .saturating_sub(actual_imports.len());
    let actual_caches = crate::godot_cache::cache_files_for_target(project, target)?;
    for path in &actual_caches {
        if !snapshot.cache_files.iter().any(|entry| &entry.path == path) {
            return Err(invalid(
                "unexpected Godot texture cache for this installed asset",
            ));
        }
    }
    let mut verified = 0;
    for expected in &snapshot.cache_files {
        if !safe_relative(&expected.path)
            || !expected.path.starts_with(".godot/imported")
            || expected.path.components().count() != 3
        {
            return Err(invalid("invalid Godot cache path in installation baseline"));
        }
        if actual_caches.contains(&expected.path) {
            if hash_file(&project.join(&expected.path))? != expected.sha256
                || fs::metadata(project.join(&expected.path))?.len() != expected.bytes
            {
                return Err(invalid("Godot texture cache differs from the installation baseline; reimport the restored original sources before loading"));
            }
            verified += 1;
        } else {
            missing += 1;
        }
    }
    Ok(
        json!({"status":if snapshot.cache_files.is_empty(){"not_recorded"}else if missing>0{"not_materialized"}else{"verified"},
        "verifiedFiles":verified,"missingFiles":missing,
        "materialized":!snapshot.cache_files.is_empty() && missing==0,
        "note":"Missing caches require a Godot import before native loading. Existing caches and import routing must match the recorded baseline; this command never rebuilds them."}),
    )
}

fn insert_texture(
    textures: &mut BTreeMap<PathBuf, PathBuf>,
    source: PathBuf,
    installed: PathBuf,
) -> io::Result<()> {
    if !safe_relative(&source) || !safe_relative(&installed) {
        return Err(invalid("Pack texture mapping escapes its root"));
    }
    if let Some(previous) = textures.insert(installed, source.clone()) {
        if previous != source {
            return Err(invalid("Pack textures collide at an installed filename"));
        }
    }
    Ok(())
}

pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> io::Result<T> {
    serde_json::from_slice(&fs::read(path)?).map_err(invalid)
}

fn invalid(message: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_string())
}
