use std::fs;
use std::path::Path;

use forge_core::asset_project::{build_style_lock, init_project};
use forge_core::provider::MediaGenerationProvider;
use forge_core::world::{
    build_environment_lock, compile_map_pack, generate_building_kit, generate_terrain_set,
    read_environment_lock, BuildingKitSpecV1, EnvironmentSpecV1, MapSpecV1, TerrainSetSpecV1,
};
use forge_providers::fixture::FixtureProvider;

#[test]
fn fixture_generates_world_packs_and_deterministic_map_without_text_provider() {
    let temp = tempfile::tempdir().unwrap();
    let project_root = temp.path().join("project");
    let mut project = init_project(&project_root, "Fixture World").unwrap();
    project.provider.id = "fixture".into();
    fs::write(
        project_root.join("forge-project.json"),
        serde_json::to_vec_pretty(&project).unwrap(),
    )
    .unwrap();
    let provider = FixtureProvider::default();

    let style_spec = temp.path().join("style.json");
    write_json(
        &style_spec,
        &serde_json::json!({
            "schemaVersion": "1",
            "prompt": "cozy forest pixel art",
            "perspective": "top_down",
            "characterCanvasSize": 64,
            "iconCanvasSize": 64,
            "propCanvasSize": 64
        }),
    );
    build_style_lock(
        &project_root,
        &style_spec,
        "fixture",
        "default",
        &provider,
        &temp.path().join("style-work"),
    )
    .unwrap();

    let environment_spec = temp.path().join("environment.json");
    write_json(
        &environment_spec,
        &EnvironmentSpecV1 {
            schema_version: "1".into(),
            id: "forest".into(),
            name: "Forest".into(),
            prompt: "mossy forest village".into(),
            perspective: "top_down".into(),
            tile_size: 16,
            image_model: None,
            license: "MIT".into(),
        },
    );
    let environment_output = build_environment_lock(
        &project_root,
        &environment_spec,
        "fixture",
        "default",
        &provider,
        &temp.path().join("environment-work"),
    )
    .unwrap();
    let environment = read_environment_lock(&environment_output.lock_path).unwrap();

    let terrain_spec: TerrainSetSpecV1 = serde_json::from_value(serde_json::json!({
        "schemaVersion": "1",
        "kind": "terrain_set",
        "id": "ground",
        "name": "Ground",
        "environmentRevision": environment.revision,
        "base": {"id": "grass", "name": "Grass", "prompt": "grass"},
        "overlay": {"id": "path", "name": "Path", "prompt": "dirt path"},
        "license": "MIT"
    }))
    .unwrap();
    let terrain = generate_terrain_set(
        &temp.path().join("terrain-exports"),
        &temp.path().join("terrain-job"),
        &terrain_spec,
        &environment,
        "fixture",
        "default",
        &provider,
    )
    .unwrap();

    let building_spec: BuildingKitSpecV1 = serde_json::from_value(serde_json::json!({
        "schemaVersion": "1",
        "kind": "building_kit",
        "id": "houses",
        "name": "Houses",
        "prompt": "timber cottages",
        "environmentRevision": environment.revision,
        "variantCount": 3,
        "license": "MIT"
    }))
    .unwrap();
    let buildings = generate_building_kit(
        &temp.path().join("building-exports"),
        &temp.path().join("building-job"),
        &building_spec,
        &environment,
        "fixture",
        "default",
        &provider,
    )
    .unwrap();

    let spec_root = temp.path().join("map-spec");
    fs::create_dir_all(spec_root.join("packs")).unwrap();
    copy_dir(&terrain.pack_dir, &spec_root.join("packs/ground.gsfpack"));
    copy_dir(&buildings.pack_dir, &spec_root.join("packs/houses.gsfpack"));
    let map_spec = spec_root.join("map.json");
    let map: MapSpecV1 = serde_json::from_value(serde_json::json!({
        "schemaVersion": "1",
        "kind": "map",
        "id": "village",
        "name": "Village",
        "seed": 99,
        "size": {"width": 64, "height": 48},
        "dependencies": {
            "terrainSets": ["packs/ground.gsfpack"],
            "buildingKits": ["packs/houses.gsfpack"],
            "propSets": []
        },
        "regions": [],
        "connections": [],
        "landmarks": [],
        "requirements": {
            "reachableExit": true,
            "buildingCount": {"min": 3, "max": 5},
            "propDensity": 0.02
        },
        "license": "MIT"
    }))
    .unwrap();
    write_json(&map_spec, &map);
    let first = compile_map_pack(&temp.path().join("map-a"), &map_spec).unwrap();
    let second = compile_map_pack(&temp.path().join("map-b"), &map_spec).unwrap();

    assert_eq!(first.layout_sha256, second.layout_sha256);
    assert_eq!(provider.usage().requests, 7);
    for pack in [terrain.pack_dir, buildings.pack_dir, first.pack_dir] {
        forge_pack::validate_pack_layout(&pack).unwrap();
    }
}

fn write_json(path: &Path, value: &impl serde::Serialize) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

fn copy_dir(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let destination = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).unwrap();
        }
    }
}
