use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::types::{
    is_valid_id, AssetKind, GameArtAssetV1, GameArtError, GameArtManifestV1, LockRef,
    GAME_ART_MANIFEST_KIND, GAME_ART_MANIFEST_SCHEMA_VERSION,
};

/// A manifest that passed full validation and whose asset specs were
/// resolved with hard path safety; consumed by the later plan/diff layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedManifest {
    pub manifest_path: PathBuf,
    pub manifest: GameArtManifestV1,
    /// Per-asset resolved spec facts, in manifest declaration order.
    pub assets: Vec<ValidatedAssetSpec>,
}

impl ValidatedManifest {
    pub fn asset(&self, id: &str) -> Option<&ValidatedAssetSpec> {
        self.assets.iter().find(|asset| asset.asset_id == id)
    }
}

/// Resolved on-disk facts for one asset spec, recorded into the plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValidatedAssetSpec {
    pub asset_id: String,
    pub canonical_spec_path: PathBuf,
    pub spec_size_bytes: u64,
    pub spec_sha256: String,
}

impl GameArtManifestV1 {
    /// Parse a manifest from raw JSON bytes and run full validation.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, GameArtError> {
        let value: Value = serde_json::from_slice(bytes)
            .map_err(|error| GameArtError::InvalidJson(error.to_string()))?;
        // Run targeted pre-checks so callers get the stable `unknown_field`
        // and `invalid_kind` codes instead of a generic serde message.
        reject_unknown_fields(&value)?;
        reject_unknown_asset_kinds(&value)?;
        let manifest: Self = serde_json::from_value(value)
            .map_err(|error| GameArtError::InvalidJson(error.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Full semantic validation. `from_slice` already runs this; call it
    /// directly for programmatically constructed manifests.
    pub fn validate(&self) -> Result<(), GameArtError> {
        if self.schema_version != GAME_ART_MANIFEST_SCHEMA_VERSION {
            return Err(GameArtError::UnsupportedSchemaVersion(format!(
                "schemaVersion must be \"{GAME_ART_MANIFEST_SCHEMA_VERSION}\", got \"{}\"",
                self.schema_version
            )));
        }
        if self.kind != GAME_ART_MANIFEST_KIND {
            return Err(GameArtError::InvalidKind(format!(
                "manifest kind must be \"{GAME_ART_MANIFEST_KIND}\", got \"{}\"",
                self.kind
            )));
        }
        if !is_valid_id(&self.project_id) {
            return Err(GameArtError::InvalidId(format!(
                "projectId \"{}\" must match [A-Za-z0-9_-]+",
                self.project_id
            )));
        }
        if self.name.trim().is_empty() {
            return Err(GameArtError::InvalidManifest(
                "name must be non-empty".into(),
            ));
        }
        if self.provider.id.trim().is_empty() || self.provider.profile_id.trim().is_empty() {
            return Err(GameArtError::InvalidManifest(
                "provider.id and provider.profileId must be non-empty".into(),
            ));
        }
        if self.defaults.output_directory.as_os_str().is_empty()
            || self.defaults.godot_root.as_os_str().is_empty()
            || self.defaults.license.trim().is_empty()
        {
            return Err(GameArtError::InvalidManifest(
                "defaults.outputDirectory, defaults.godotRoot and defaults.license must be non-empty"
                    .into(),
            ));
        }

        let mut declared = BTreeSet::new();
        for asset in &self.assets {
            if !is_valid_id(&asset.id) {
                return Err(GameArtError::InvalidId(format!(
                    "asset id \"{}\" must match [A-Za-z0-9_-]+",
                    asset.id
                )));
            }
            if !declared.insert(asset.id.as_str()) {
                return Err(GameArtError::DuplicateAssetId(asset.id.clone()));
            }
        }

        // Every dependsOn entry is either a declared asset id (graph edge) or
        // a syntactically valid immutable lock reference.
        for asset in &self.assets {
            for dependency in &asset.depends_on {
                if dependency == &asset.id {
                    return Err(GameArtError::SelfDependency(format!(
                        "asset \"{}\" must not depend on itself",
                        asset.id
                    )));
                }
                if declared.contains(dependency.as_str()) {
                    continue;
                }
                if dependency.contains(':') || dependency.contains('@') {
                    // Not a bare asset id, so it must parse as a lock reference.
                    LockRef::parse(dependency)?;
                } else {
                    return Err(GameArtError::UnknownDependency(format!(
                        "asset \"{}\" depends on \"{dependency}\", which is neither a declared asset id nor a lock reference",
                        asset.id
                    )));
                }
            }
        }

        detect_cycle(self)?;

        // A required asset must never be blocked by an optional one;
        // optional -> optional is fine.
        for asset in &self.assets {
            if !asset.required {
                continue;
            }
            for dependency in &asset.depends_on {
                if let Some(target) = self
                    .assets
                    .iter()
                    .find(|candidate| &candidate.id == dependency)
                {
                    if !target.required {
                        return Err(GameArtError::RequiredDependsOnOptional(format!(
                            "required asset \"{}\" depends on optional asset \"{}\"",
                            asset.id, target.id
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    /// Load a manifest from disk, validate it, and resolve every asset spec
    /// path against the manifest file's directory with hard path safety.
    pub fn load_validated(manifest_path: &Path) -> Result<ValidatedManifest, GameArtError> {
        let bytes = fs::read(manifest_path).map_err(|error| {
            GameArtError::Io(format!(
                "cannot read manifest {}: {error}",
                manifest_path.display()
            ))
        })?;
        let manifest = Self::from_slice(&bytes)?;
        let manifest_dir = manifest_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let canonical_root = manifest_dir.canonicalize().map_err(|error| {
            GameArtError::Io(format!(
                "cannot resolve manifest directory {}: {error}",
                manifest_dir.display()
            ))
        })?;
        let mut assets = Vec::with_capacity(manifest.assets.len());
        for asset in &manifest.assets {
            assets.push(resolve_spec(asset, &canonical_root)?);
        }
        Ok(ValidatedManifest {
            manifest_path: manifest_path.to_path_buf(),
            manifest,
            assets,
        })
    }

    /// Canonical JSON form: assets sorted by id, `dependsOn` and `tags`
    /// sorted, and struct-based serialization keeps object key order
    /// deterministic — semantically identical manifests written with
    /// different field order or whitespace normalize to the same value.
    pub fn normalized_manifest(&self) -> Value {
        let mut normalized = self.clone();
        normalized
            .assets
            .sort_by(|left, right| left.id.cmp(&right.id));
        for asset in &mut normalized.assets {
            asset.depends_on.sort();
            asset.tags.sort();
        }
        serde_json::to_value(&normalized).expect("GameArtManifestV1 serialization cannot fail")
    }

    /// SHA-256 over the canonical serialization of `normalized_manifest`.
    /// Spec *content* hashes are deliberately excluded: they enter the plan
    /// hash computed by the later plan/diff layer, not the manifest hash.
    pub fn manifest_sha256(&self) -> String {
        format!(
            "{:x}",
            Sha256::digest(canonical_bytes(&self.normalized_manifest()))
        )
    }

    /// Asset-level dependency graph: every asset id is a node; edges point at
    /// declared asset ids only (lock references are not graph edges). The
    /// canonical form sorts node ids and edge lists and dedupes edges.
    pub fn dependency_graph(&self) -> BTreeMap<String, Vec<String>> {
        let declared: BTreeSet<&str> = self.assets.iter().map(|asset| asset.id.as_str()).collect();
        self.assets
            .iter()
            .map(|asset| {
                let mut edges = asset
                    .depends_on
                    .iter()
                    .filter(|dependency| declared.contains(dependency.as_str()))
                    .cloned()
                    .collect::<Vec<_>>();
                edges.sort();
                edges.dedup();
                (asset.id.clone(), edges)
            })
            .collect()
    }

    /// SHA-256 over the canonical serialization of `dependency_graph`.
    pub fn graph_sha256(&self) -> String {
        let value = serde_json::to_value(self.dependency_graph())
            .expect("dependency graph serialization cannot fail");
        format!("{:x}", Sha256::digest(canonical_bytes(&value)))
    }
}

fn canonical_bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).expect("canonical JSON serialization cannot fail")
}

/// Mirror of serde's `deny_unknown_fields` that reports the JSON location,
/// giving callers the stable `unknown_field` code instead of a generic
/// parse error.
fn reject_unknown_fields(value: &Value) -> Result<(), GameArtError> {
    const MANIFEST_FIELDS: &[&str] = &[
        "schemaVersion",
        "kind",
        "projectId",
        "name",
        "styleRevision",
        "provider",
        "defaults",
        "assets",
    ];
    const PROVIDER_FIELDS: &[&str] = &["id", "profileId"];
    const DEFAULTS_FIELDS: &[&str] = &["outputDirectory", "godotRoot", "license"];
    const ASSET_FIELDS: &[&str] = &["id", "kind", "spec", "required", "dependsOn", "tags"];

    fn check_object(
        object: &serde_json::Map<String, Value>,
        known: &[&str],
        location: &str,
    ) -> Result<(), GameArtError> {
        for key in object.keys() {
            if !known.contains(&key.as_str()) {
                return Err(GameArtError::UnknownField(format!("{location}: \"{key}\"")));
            }
        }
        Ok(())
    }

    let object = value
        .as_object()
        .ok_or_else(|| GameArtError::InvalidJson("manifest must be a JSON object".into()))?;
    check_object(object, MANIFEST_FIELDS, "manifest")?;
    if let Some(provider) = value.get("provider").and_then(Value::as_object) {
        check_object(provider, PROVIDER_FIELDS, "provider")?;
    }
    if let Some(defaults) = value.get("defaults").and_then(Value::as_object) {
        check_object(defaults, DEFAULTS_FIELDS, "defaults")?;
    }
    if let Some(assets) = value.get("assets").and_then(Value::as_array) {
        for (index, asset) in assets.iter().enumerate() {
            if let Some(asset) = asset.as_object() {
                check_object(asset, ASSET_FIELDS, &format!("assets[{index}]"))?;
            }
        }
    }
    Ok(())
}

/// Report unsupported asset kinds with the `invalid_kind` code before typed
/// deserialization would turn them into a generic serde error.
fn reject_unknown_asset_kinds(value: &Value) -> Result<(), GameArtError> {
    if let Some(assets) = value.get("assets").and_then(Value::as_array) {
        for (index, asset) in assets.iter().enumerate() {
            if let Some(kind) = asset.get("kind").and_then(Value::as_str) {
                if kind.parse::<AssetKind>().is_err() {
                    let label = asset.get("id").and_then(Value::as_str).unwrap_or("?");
                    return Err(GameArtError::InvalidKind(format!(
                        "asset \"{label}\" (assets[{index}]) uses unsupported kind \"{kind}\" (stage 2 supports character, icon_set, prop_set)"
                    )));
                }
            }
        }
    }
    Ok(())
}

/// Depth-first cycle detection over asset-id dependency edges; the error
/// message carries the cycle path, e.g. "a -> b -> a".
fn detect_cycle(manifest: &GameArtManifestV1) -> Result<(), GameArtError> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Mark {
        Visiting,
        Done,
    }

    fn visit<'a>(
        node: &'a str,
        graph: &'a BTreeMap<String, Vec<String>>,
        marks: &mut BTreeMap<&'a str, Mark>,
        stack: &mut Vec<&'a str>,
    ) -> Result<(), GameArtError> {
        match marks.get(node) {
            Some(Mark::Done) => return Ok(()),
            Some(Mark::Visiting) => {
                let start = stack.iter().position(|entry| *entry == node).unwrap_or(0);
                let mut cycle = stack[start..].to_vec();
                cycle.push(node);
                return Err(GameArtError::DependencyCycle(cycle.join(" -> ")));
            }
            None => {}
        }
        marks.insert(node, Mark::Visiting);
        stack.push(node);
        if let Some(edges) = graph.get(node) {
            for next in edges {
                visit(next, graph, marks, stack)?;
            }
        }
        stack.pop();
        marks.insert(node, Mark::Done);
        Ok(())
    }

    let graph = manifest.dependency_graph();
    let mut marks = BTreeMap::new();
    let mut stack = Vec::new();
    for asset in &manifest.assets {
        visit(asset.id.as_str(), &graph, &mut marks, &mut stack)?;
    }
    Ok(())
}

/// Scheme detection: `<alpha><alnum|+|.|->*:` before any path separator
/// means a URL (a Windows drive prefix matches too and is just as
/// non-portable — reject it the same way).
fn looks_like_url(raw: &str) -> bool {
    let Some((scheme, _)) = raw.split_once(':') else {
        return false;
    };
    !scheme.is_empty()
        && scheme
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
}

/// Resolve one asset spec path against the canonical manifest directory.
/// Rejects URLs, absolute paths, `..` traversal and symlink escapes; the
/// target must exist and be a regular file (which also excludes device
/// files and directories).
fn resolve_spec(
    asset: &GameArtAssetV1,
    canonical_root: &Path,
) -> Result<ValidatedAssetSpec, GameArtError> {
    let raw = asset.spec.to_string_lossy();
    if looks_like_url(&raw) {
        return Err(GameArtError::UrlNotAllowed(format!(
            "asset \"{}\" spec \"{raw}\" is a URL, not a relative path",
            asset.id
        )));
    }
    for component in asset.spec.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => {
                return Err(GameArtError::AbsolutePath(format!(
                    "asset \"{}\" spec \"{raw}\" must be relative to the manifest",
                    asset.id
                )));
            }
            Component::ParentDir => {
                return Err(GameArtError::PathTraversal(format!(
                    "asset \"{}\" spec \"{raw}\" must not contain \"..\"",
                    asset.id
                )));
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }
    let candidate = canonical_root.join(&asset.spec);
    let metadata = fs::metadata(&candidate).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            GameArtError::SpecNotFound(format!(
                "asset \"{}\" spec \"{raw}\" does not exist",
                asset.id
            ))
        } else {
            GameArtError::Io(format!(
                "cannot stat spec \"{raw}\" for asset \"{}\": {error}",
                asset.id
            ))
        }
    })?;
    if !metadata.is_file() {
        return Err(GameArtError::SpecNotFile(format!(
            "asset \"{}\" spec \"{raw}\" is not a regular file",
            asset.id
        )));
    }
    let canonical_spec_path = candidate.canonicalize().map_err(|error| {
        GameArtError::Io(format!(
            "cannot canonicalize spec \"{raw}\" for asset \"{}\": {error}",
            asset.id
        ))
    })?;
    if !canonical_spec_path.starts_with(canonical_root) {
        return Err(GameArtError::SymlinkEscape(format!(
            "asset \"{}\" spec \"{raw}\" resolves outside the manifest directory",
            asset.id
        )));
    }
    let bytes = fs::read(&canonical_spec_path).map_err(|error| {
        GameArtError::Io(format!(
            "cannot read spec \"{raw}\" for asset \"{}\": {error}",
            asset.id
        ))
    })?;
    Ok(ValidatedAssetSpec {
        asset_id: asset.id.clone(),
        canonical_spec_path,
        spec_size_bytes: bytes.len() as u64,
        spec_sha256: format!("{:x}", Sha256::digest(&bytes)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_with_assets(assets: &str) -> String {
        format!(
            r#"{{
                "schemaVersion": "1",
                "kind": "game_art_manifest",
                "projectId": "forest-rpg",
                "name": "Forest RPG",
                "provider": {{ "id": "xai", "profileId": "default" }},
                "defaults": {{
                    "outputDirectory": "packs",
                    "godotRoot": "addons/forge_assets",
                    "license": "private"
                }},
                "assets": {assets}
            }}"#
        )
    }

    fn asset_json(id: &str, kind: &str, spec: &str, extra: &str) -> String {
        let separator = if extra.is_empty() { "" } else { ", " };
        format!(r#"{{ "id": "{id}", "kind": "{kind}", "spec": "{spec}"{separator}{extra} }}"#)
    }

    fn code_of<T: std::fmt::Debug>(result: Result<T, GameArtError>) -> &'static str {
        result.unwrap_err().code()
    }

    fn write_manifest(dir: &Path, contents: &str) -> PathBuf {
        let manifest_path = dir.join("game-art.json");
        fs::write(&manifest_path, contents).unwrap();
        manifest_path
    }

    #[test]
    fn parses_and_normalizes_valid_manifest() {
        let json = manifest_with_assets(&format!(
            "[{}, {}]",
            asset_json(
                "forest-ranger",
                "character",
                "specs/characters/forest-ranger.json",
                r#""dependsOn": ["subject:forest-ranger@subject-rev-1", "hud-icons"], "tags": ["player", "forest"]"#
            ),
            asset_json("hud-icons", "icon_set", "specs/icons/hud.json", "")
        ));
        let manifest = GameArtManifestV1::from_slice(json.as_bytes()).unwrap();
        assert_eq!(manifest.schema_version, "1");
        assert_eq!(manifest.kind, GAME_ART_MANIFEST_KIND);
        assert_eq!(manifest.project_id, "forest-rpg");
        assert_eq!(manifest.style_revision, None);
        assert_eq!(manifest.assets.len(), 2);
        // required defaults to true, dependsOn/tags default to empty.
        assert!(manifest.assets[1].required);
        assert!(manifest.assets[1].depends_on.is_empty());
        assert!(manifest.assets[1].tags.is_empty());
        assert_eq!(manifest.assets[0].kind, AssetKind::Character);

        // Normalization sorts assets by id and dependsOn/tags alphabetically.
        let normalized = manifest.normalized_manifest();
        let assets = normalized["assets"].as_array().unwrap();
        assert_eq!(assets[0]["id"], "forest-ranger");
        assert_eq!(assets[1]["id"], "hud-icons");
        assert_eq!(
            assets[0]["dependsOn"],
            serde_json::json!(["hud-icons", "subject:forest-ranger@subject-rev-1"])
        );
        assert_eq!(assets[0]["tags"], serde_json::json!(["forest", "player"]));
    }

    #[test]
    fn rejects_unknown_fields_with_location() {
        let json =
            manifest_with_assets("[]").replace("\"assets\": []", "\"assets\": [], \"bogus\": 1");
        assert_eq!(
            code_of(GameArtManifestV1::from_slice(json.as_bytes())),
            "unknown_field"
        );

        let nested = manifest_with_assets(&format!(
            "[{}]",
            asset_json("hero", "character", "specs/hero.json", r#""priority": 1"#)
        ));
        let error = GameArtManifestV1::from_slice(nested.as_bytes()).unwrap_err();
        assert_eq!(error.code(), "unknown_field");
        assert!(error.to_string().contains("assets[0]"));

        let provider = manifest_with_assets("[]").replace(
            r#""profileId": "default""#,
            r#""profileId": "default", "extra": 1"#,
        );
        assert_eq!(
            code_of(GameArtManifestV1::from_slice(provider.as_bytes())),
            "unknown_field"
        );
    }

    #[test]
    fn rejects_duplicate_asset_id() {
        let json = manifest_with_assets(&format!(
            "[{}, {}]",
            asset_json("hero", "character", "specs/hero.json", ""),
            asset_json("hero", "prop_set", "specs/hero-props.json", "")
        ));
        assert_eq!(
            code_of(GameArtManifestV1::from_slice(json.as_bytes())),
            "duplicate_asset_id"
        );
    }

    #[test]
    fn rejects_unknown_dependency_reference() {
        let json = manifest_with_assets(&format!(
            "[{}]",
            asset_json(
                "hero",
                "character",
                "specs/hero.json",
                r#""dependsOn": ["missing-asset"]"#
            )
        ));
        assert_eq!(
            code_of(GameArtManifestV1::from_slice(json.as_bytes())),
            "unknown_dependency"
        );
    }

    #[test]
    fn rejects_malformed_lock_reference_dependency() {
        let json = manifest_with_assets(&format!(
            "[{}]",
            asset_json(
                "hero",
                "character",
                "specs/hero.json",
                r#""dependsOn": ["style:pixel-art"]"#
            )
        ));
        assert_eq!(
            code_of(GameArtManifestV1::from_slice(json.as_bytes())),
            "invalid_lock_ref"
        );
    }

    #[test]
    fn rejects_self_dependency() {
        let json = manifest_with_assets(&format!(
            "[{}]",
            asset_json(
                "hero",
                "character",
                "specs/hero.json",
                r#""dependsOn": ["hero"]"#
            )
        ));
        assert_eq!(
            code_of(GameArtManifestV1::from_slice(json.as_bytes())),
            "self_dependency"
        );
    }

    #[test]
    fn detects_two_node_cycle_with_path() {
        let json = manifest_with_assets(&format!(
            "[{}, {}]",
            asset_json("a", "character", "specs/a.json", r#""dependsOn": ["b"]"#),
            asset_json("b", "prop_set", "specs/b.json", r#""dependsOn": ["a"]"#)
        ));
        let error = GameArtManifestV1::from_slice(json.as_bytes()).unwrap_err();
        assert_eq!(error.code(), "dependency_cycle");
        assert_eq!(error.to_string(), "dependency cycle: a -> b -> a");
    }

    #[test]
    fn detects_three_node_cycle_with_path() {
        let json = manifest_with_assets(&format!(
            "[{}, {}, {}]",
            asset_json("a", "character", "specs/a.json", r#""dependsOn": ["b"]"#),
            asset_json("b", "icon_set", "specs/b.json", r#""dependsOn": ["c"]"#),
            asset_json("c", "prop_set", "specs/c.json", r#""dependsOn": ["a"]"#)
        ));
        let error = GameArtManifestV1::from_slice(json.as_bytes()).unwrap_err();
        assert_eq!(error.code(), "dependency_cycle");
        assert_eq!(error.to_string(), "dependency cycle: a -> b -> c -> a");
    }

    #[test]
    fn acyclic_diamond_passes() {
        let json = manifest_with_assets(&format!(
            "[{}, {}, {}, {}]",
            asset_json(
                "a",
                "character",
                "specs/a.json",
                r#""dependsOn": ["b", "c"]"#
            ),
            asset_json("b", "icon_set", "specs/b.json", r#""dependsOn": ["d"]"#),
            asset_json("c", "prop_set", "specs/c.json", r#""dependsOn": ["d"]"#),
            asset_json("d", "prop_set", "specs/d.json", "")
        ));
        GameArtManifestV1::from_slice(json.as_bytes()).unwrap();
    }

    #[test]
    fn rejects_required_asset_depending_on_optional() {
        let json = manifest_with_assets(&format!(
            "[{}, {}]",
            asset_json(
                "hero",
                "character",
                "specs/hero.json",
                r#""dependsOn": ["extra-props"]"#
            ),
            asset_json(
                "extra-props",
                "prop_set",
                "specs/props.json",
                r#""required": false"#
            )
        ));
        assert_eq!(
            code_of(GameArtManifestV1::from_slice(json.as_bytes())),
            "required_depends_on_optional"
        );
    }

    #[test]
    fn allows_optional_asset_depending_on_optional() {
        let json = manifest_with_assets(&format!(
            "[{}, {}]",
            asset_json(
                "bonus-hero",
                "character",
                "specs/hero.json",
                r#""required": false, "dependsOn": ["extra-props"]"#
            ),
            asset_json(
                "extra-props",
                "prop_set",
                "specs/props.json",
                r#""required": false"#
            )
        ));
        GameArtManifestV1::from_slice(json.as_bytes()).unwrap();
    }

    #[test]
    fn rejects_invalid_id_charset() {
        let bad_project = manifest_with_assets("[]").replace("forest-rpg", "forest rpg");
        assert_eq!(
            code_of(GameArtManifestV1::from_slice(bad_project.as_bytes())),
            "invalid_id"
        );

        let bad_asset = manifest_with_assets(&format!(
            "[{}]",
            asset_json("hero/main", "character", "specs/hero.json", "")
        ));
        assert_eq!(
            code_of(GameArtManifestV1::from_slice(bad_asset.as_bytes())),
            "invalid_id"
        );
    }

    #[test]
    fn rejects_invalid_asset_kind() {
        let json = manifest_with_assets(&format!(
            "[{}]",
            asset_json("forest", "environment", "specs/forest.json", "")
        ));
        let error = GameArtManifestV1::from_slice(json.as_bytes()).unwrap_err();
        assert_eq!(error.code(), "invalid_kind");
        assert!(error.to_string().contains("environment"));
    }

    #[test]
    fn rejects_wrong_manifest_kind_and_schema_version() {
        let wrong_kind = manifest_with_assets("[]").replace("game_art_manifest", "asset_manifest");
        assert_eq!(
            code_of(GameArtManifestV1::from_slice(wrong_kind.as_bytes())),
            "invalid_kind"
        );

        let wrong_version = manifest_with_assets("[]")
            .replace(r#""schemaVersion": "1""#, r#""schemaVersion": "2""#);
        assert_eq!(
            code_of(GameArtManifestV1::from_slice(wrong_version.as_bytes())),
            "unsupported_schema_version"
        );
    }

    #[test]
    fn manifest_hash_is_stable_across_reordering() {
        let ordered = manifest_with_assets(&format!(
            "[{}, {}]",
            asset_json(
                "forest-ranger",
                "character",
                "specs/characters/forest-ranger.json",
                r#""required": true, "dependsOn": ["subject:forest-ranger@rev-1", "hud-icons"], "tags": ["player", "forest"]"#
            ),
            asset_json(
                "hud-icons",
                "icon_set",
                "specs/icons/hud.json",
                r#""tags": ["ui"]"#
            )
        ));
        // Same manifest, different field order, whitespace, asset order and
        // dependsOn/tags order — plus styleRevision present in both.
        let shuffled = r#"{
  "assets": [
    { "tags": ["ui"], "spec": "specs/icons/hud.json", "kind": "icon_set", "id": "hud-icons" },
    { "tags": ["forest", "player"], "dependsOn": ["hud-icons", "subject:forest-ranger@rev-1"],
      "required": true, "spec": "specs/characters/forest-ranger.json", "kind": "character",
      "id": "forest-ranger" }
  ],
  "defaults": { "license": "private", "godotRoot": "addons/forge_assets", "outputDirectory": "packs" },
  "provider": { "profileId": "default", "id": "xai" },
  "name": "Forest RPG",
  "projectId": "forest-rpg",
  "kind": "game_art_manifest",
  "schemaVersion": "1"
}"#;
        let first = GameArtManifestV1::from_slice(ordered.as_bytes()).unwrap();
        let second = GameArtManifestV1::from_slice(shuffled.as_bytes()).unwrap();
        // Declaration order still differs on the structs; the guarantee lives
        // in the normalized form and the hashes derived from it.
        assert_eq!(first.normalized_manifest(), second.normalized_manifest());
        assert_eq!(
            canonical_bytes(&first.normalized_manifest()),
            canonical_bytes(&second.normalized_manifest())
        );
        assert_eq!(first.manifest_sha256(), second.manifest_sha256());
        assert_eq!(first.graph_sha256(), second.graph_sha256());

        let changed = ordered.replace("private", "cc0");
        let third = GameArtManifestV1::from_slice(changed.as_bytes()).unwrap();
        assert_ne!(first.manifest_sha256(), third.manifest_sha256());
    }

    #[test]
    fn dependency_graph_contains_all_nodes_and_asset_edges_only() {
        let json = manifest_with_assets(&format!(
            "[{}, {}, {}]",
            asset_json(
                "hero",
                "character",
                "specs/hero.json",
                r#""dependsOn": ["style:pixel@rev-1", "hud-icons", "hud-icons"]"#
            ),
            asset_json("hud-icons", "icon_set", "specs/icons.json", ""),
            asset_json("props", "prop_set", "specs/props.json", "")
        ));
        let manifest = GameArtManifestV1::from_slice(json.as_bytes()).unwrap();
        let graph = manifest.dependency_graph();
        assert_eq!(graph.len(), 3);
        // Lock refs are not edges; duplicate edges collapse in canonical form.
        assert_eq!(graph["hero"], vec!["hud-icons".to_string()]);
        assert!(graph["hud-icons"].is_empty());
        assert!(graph["props"].is_empty());
    }

    #[test]
    fn load_validated_resolves_specs_with_hashes() {
        let temp = tempfile::tempdir().unwrap();
        let spec_dir = temp.path().join("specs");
        fs::create_dir_all(&spec_dir).unwrap();
        let spec_content = br#"{ "schemaVersion": "1", "prompt": "ranger" }"#;
        fs::write(spec_dir.join("hero.json"), spec_content).unwrap();
        let manifest_path = write_manifest(
            temp.path(),
            &manifest_with_assets(&format!(
                "[{}]",
                asset_json("hero", "character", "specs/hero.json", "")
            )),
        );

        let validated = GameArtManifestV1::load_validated(&manifest_path).unwrap();
        assert_eq!(validated.manifest_path, manifest_path);
        assert_eq!(validated.assets.len(), 1);
        let spec = validated.asset("hero").unwrap();
        assert_eq!(
            spec.canonical_spec_path,
            spec_dir.join("hero.json").canonicalize().unwrap()
        );
        assert!(spec.canonical_spec_path.is_absolute());
        assert_eq!(spec.spec_size_bytes, spec_content.len() as u64);
        assert_eq!(
            spec.spec_sha256,
            format!("{:x}", Sha256::digest(spec_content))
        );
    }

    #[test]
    fn load_validated_rejects_absolute_spec_path() {
        let temp = tempfile::tempdir().unwrap();
        let manifest_path = write_manifest(
            temp.path(),
            &manifest_with_assets(&format!(
                "[{}]",
                asset_json("hero", "character", "/etc/passwd", "")
            )),
        );
        assert_eq!(
            code_of(GameArtManifestV1::load_validated(&manifest_path)),
            "absolute_path"
        );
    }

    #[test]
    fn load_validated_rejects_parent_dir_traversal() {
        let temp = tempfile::tempdir().unwrap();
        let outside = temp.path().join("secret.json");
        fs::write(&outside, b"{}").unwrap();
        let project_dir = temp.path().join("project");
        fs::create_dir_all(&project_dir).unwrap();
        let manifest_path = write_manifest(
            &project_dir,
            &manifest_with_assets(&format!(
                "[{}]",
                asset_json("hero", "character", "../secret.json", "")
            )),
        );
        assert_eq!(
            code_of(GameArtManifestV1::load_validated(&manifest_path)),
            "path_traversal"
        );
    }

    #[test]
    fn load_validated_rejects_url_specs() {
        for url in [
            "https://example.com/hero.json",
            "file:///etc/passwd",
            "ipfs://cid/hero.json",
        ] {
            let temp = tempfile::tempdir().unwrap();
            let manifest_path = write_manifest(
                temp.path(),
                &manifest_with_assets(&format!("[{}]", asset_json("hero", "character", url, ""))),
            );
            assert_eq!(
                code_of(GameArtManifestV1::load_validated(&manifest_path)),
                "url_not_allowed",
                "input: {url}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn load_validated_rejects_symlink_escape() {
        let temp = tempfile::tempdir().unwrap();
        let outside = temp.path().join("outside.json");
        fs::write(&outside, b"{}").unwrap();
        let project_dir = temp.path().join("project");
        fs::create_dir_all(&project_dir).unwrap();
        std::os::unix::fs::symlink(&outside, project_dir.join("linked.json")).unwrap();
        let manifest_path = write_manifest(
            &project_dir,
            &manifest_with_assets(&format!(
                "[{}]",
                asset_json("hero", "character", "linked.json", "")
            )),
        );
        assert_eq!(
            code_of(GameArtManifestV1::load_validated(&manifest_path)),
            "symlink_escape"
        );
    }

    #[cfg(unix)]
    #[test]
    fn load_validated_allows_internal_symlink() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("real.json"), b"{}").unwrap();
        std::os::unix::fs::symlink(
            temp.path().join("real.json"),
            temp.path().join("linked.json"),
        )
        .unwrap();
        let manifest_path = write_manifest(
            temp.path(),
            &manifest_with_assets(&format!(
                "[{}]",
                asset_json("hero", "character", "linked.json", "")
            )),
        );
        GameArtManifestV1::load_validated(&manifest_path).unwrap();
    }

    #[test]
    fn load_validated_rejects_missing_spec() {
        let temp = tempfile::tempdir().unwrap();
        let manifest_path = write_manifest(
            temp.path(),
            &manifest_with_assets(&format!(
                "[{}]",
                asset_json("hero", "character", "specs/nope.json", "")
            )),
        );
        assert_eq!(
            code_of(GameArtManifestV1::load_validated(&manifest_path)),
            "spec_not_found"
        );
    }

    #[test]
    fn load_validated_rejects_non_regular_file() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("specs")).unwrap();
        let manifest_path = write_manifest(
            temp.path(),
            &manifest_with_assets(&format!(
                "[{}]",
                asset_json("hero", "character", "specs", "")
            )),
        );
        assert_eq!(
            code_of(GameArtManifestV1::load_validated(&manifest_path)),
            "spec_not_file"
        );
    }
}
