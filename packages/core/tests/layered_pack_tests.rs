use std::{fs, path::Path};

use forge_core::layered::{
    manifest_for_request, prepare_layered_pack, validate_request, LayerBlend, LayerKeyframe,
    LayerTrack, LayerTransform, LayeredCanvas, LayeredClip, LayeredSampling, LayeredSourceLayer,
    PrepareLayeredRequest,
};
use image::{Rgba, RgbaImage};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn fixture(root: &Path) -> PrepareLayeredRequest {
    let mut layers = Vec::new();
    for (i, id) in ["rear", "front"].iter().enumerate() {
        let mut image = RgbaImage::from_pixel(37, 23, Rgba([0, 0, 0, 0]));
        // Distinct positions must remain registered, including a touching edge.
        image.put_pixel(i as u32 * 20, 8, Rgba([80 + i as u8 * 90, 40, 100, 220]));
        let path = root.join(format!("{id}.png"));
        image.save(&path).unwrap();
        let hash = format!("{:x}", Sha256::digest(fs::read(&path).unwrap()));
        layers.push(LayeredSourceLayer {
            id: (*id).into(),
            name: (*id).into(),
            path,
            sha256: hash,
            pivot: [12.0, 8.0],
            transform: LayerTransform::default(),
            blend: if i == 0 {
                LayerBlend::Normal
            } else {
                LayerBlend::Add
            },
        });
    }
    let middle = LayerTransform {
        position: [3.5, -1.0],
        rotation_degrees: 17.0,
        scale: [0.8, 1.2],
        opacity: 0.45,
    };
    PrepareLayeredRequest {
        schema_version: "1".into(),
        id: "layered_fixture".into(),
        name: "Layered fixture".into(),
        license: "CC0-1.0".into(),
        canvas: LayeredCanvas {
            width: 37,
            height: 23,
            origin: [0.0, 0.0],
        },
        sampling: LayeredSampling::Linear,
        layers,
        clips: vec![LayeredClip {
            id: "cast".into(),
            duration_ms: 1400,
            loop_animation: false,
            tracks: vec![LayerTrack {
                layer_id: "front".into(),
                keyframes: vec![
                    LayerKeyframe {
                        time_ms: 0,
                        transform: LayerTransform::default(),
                    },
                    LayerKeyframe {
                        time_ms: 350,
                        transform: middle,
                    },
                    LayerKeyframe {
                        time_ms: 1400,
                        transform: LayerTransform::default(),
                    },
                ],
            }],
        }],
        default_clip: Some("cast".into()),
    }
}

fn json_at(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn change_json(path: &Path, mutate: impl FnOnce(&mut Value)) {
    let mut value = json_at(path);
    mutate(&mut value);
    fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}

#[test]
fn shared_rectangle_and_raw_layer_bytes_survive_pack_inspection() {
    let temp = tempfile::tempdir().unwrap();
    let request = fixture(temp.path());
    let output = prepare_layered_pack(&request, &temp.path().join("fixture.gsfpack")).unwrap();
    assert_eq!((output.layer_count, output.clip_count), (2, 1));
    let inspected = forge_pack::inspect_pack(&output.pack_dir).unwrap();
    assert_eq!(inspected.asset_type, "layered");
    assert_eq!(inspected.frame_count, 0);
    assert!(inspected.animations.is_empty());
    assert_eq!(inspected.default_animation, "cast");
    assert_eq!(
        inspected.layered.as_ref().unwrap(),
        &manifest_for_request(&request)
    );
    let imported = forge_pack::import_pack(&output.pack_dir).unwrap();
    assert!(imported.frame_paths.is_empty());
    assert!(imported.atlas.is_null());
    for source in &request.layers {
        assert_eq!(
            fs::read(&source.path).unwrap(),
            fs::read(
                output
                    .pack_dir
                    .join(format!("assets/layers/{}.png", source.id))
            )
            .unwrap()
        );
    }
    let report = json_at(&output.quality_report_path);
    assert_eq!(report["scope"], "layered_structure_only");
    assert_eq!(report["verdict"], "review_required");
}

#[test]
fn static_layered_pack_needs_no_fake_animation_frames() {
    let temp = tempfile::tempdir().unwrap();
    let mut request = fixture(temp.path());
    request.clips.clear();
    request.default_clip = None;
    request.layers[1].blend = LayerBlend::Multiply;
    let output = prepare_layered_pack(&request, &temp.path().join("static.gsfpack")).unwrap();
    let inspect = forge_pack::inspect_pack(&output.pack_dir).unwrap();
    assert_eq!(inspect.default_animation, "");
    assert!(inspect.layered.unwrap().clips.is_empty());
    let scene = fs::read_to_string(output.scene_path).unwrap();
    assert!(scene.contains("[sub_resource type=\"ShaderMaterial\" id=\"blend_multiply\"]"));
    assert!(scene.contains("material = SubResource(\"blend_multiply\")"));
}

#[test]
fn malformed_canvas_hash_and_source_fail_before_creating_output() {
    let temp = tempfile::tempdir().unwrap();
    let request = fixture(temp.path());
    for mutation in ["canvas", "origin", "hash", "png", "license"] {
        let mut bad = request.clone();
        match mutation {
            "canvas" => bad.canvas.height += 1,
            "origin" => bad.canvas.origin[0] = 1.0,
            "hash" => bad.layers[0].sha256 = "0".repeat(64),
            "png" => bad.layers[0].path = temp.path().join("missing.png"),
            "license" => bad.license = " ".into(),
            _ => unreachable!(),
        }
        let destination = temp.path().join(format!("{mutation}.gsfpack"));
        assert!(
            prepare_layered_pack(&bad, &destination).is_err(),
            "{mutation}"
        );
        assert!(
            !destination.exists(),
            "invalid input created output: {mutation}"
        );
    }
}

#[test]
fn portable_ids_and_case_collisions_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let request = fixture(temp.path());
    for id in [
        "../outside",
        "front/part",
        "front:part",
        "front.part",
        "CON",
        "lpt1",
        "8layer",
        "",
    ] {
        let mut bad = request.clone();
        bad.layers[0].id = id.into();
        assert!(validate_request(&bad).is_err(), "{id}");
    }
    let mut bad = request;
    bad.layers[0].id = "FRONT".into();
    assert!(validate_request(&bad).is_err());
}

#[test]
fn finite_transforms_and_keyframe_contract_are_enforced() {
    let temp = tempfile::tempdir().unwrap();
    let request = fixture(temp.path());
    for mutation in [
        "nan",
        "infinity",
        "scale",
        "opacity",
        "pivot",
        "track",
        "repeat_track",
        "repeat_time",
        "start",
        "end",
        "duration",
        "default",
    ] {
        let mut bad = request.clone();
        match mutation {
            "nan" => bad.layers[0].transform.position[0] = f64::NAN,
            "infinity" => {
                bad.clips[0].tracks[0].keyframes[0]
                    .transform
                    .rotation_degrees = f64::INFINITY
            }
            "scale" => bad.layers[0].transform.scale[0] = 0.0,
            "opacity" => bad.layers[0].transform.opacity = 1.1,
            "pivot" => bad.layers[0].pivot[1] = 24.0,
            "track" => bad.clips[0].tracks[0].layer_id = "missing".into(),
            "repeat_track" => {
                let track = bad.clips[0].tracks[0].clone();
                bad.clips[0].tracks.push(track);
            }
            "repeat_time" => bad.clips[0].tracks[0].keyframes[1].time_ms = 0,
            "start" => bad.clips[0].tracks[0].keyframes[0].time_ms = 2,
            "end" => bad.clips[0].tracks[0].keyframes[2].time_ms = 1399,
            "duration" => bad.clips[0].duration_ms = 0,
            "default" => bad.default_clip = Some("missing".into()),
            _ => unreachable!(),
        }
        assert!(validate_request(&bad).is_err(), "{mutation}");
    }
}

#[test]
fn output_is_no_clobber_and_resource_generation_is_deterministic() {
    let temp = tempfile::tempdir().unwrap();
    let request = fixture(temp.path());
    let first = prepare_layered_pack(&request, &temp.path().join("first.gsfpack")).unwrap();
    let second = prepare_layered_pack(&request, &temp.path().join("second.gsfpack")).unwrap();
    for relative in [
        "assets/manifest.json",
        "assets/layered.tscn",
        "assets/forge_layered_player.gd",
        "assets/forge_alpha_multiply.gdshader",
        "assets/godot_import.json",
        "previews/layers.png",
    ] {
        assert_eq!(
            fs::read(first.pack_dir.join(relative)).unwrap(),
            fs::read(second.pack_dir.join(relative)).unwrap()
        );
    }
    change_json(&first.quality_report_path, |value| {
        value["notes"] =
            json!(["Updated explanatory text does not change the structural contract."]);
    });
    forge_pack::validate_pack_layout(&first.pack_dir).unwrap();
    let sentinel = first.pack_dir.join("user-note.txt");
    fs::write(&sentinel, "do not overwrite").unwrap();
    assert!(prepare_layered_pack(&request, &first.pack_dir).is_err());
    assert_eq!(fs::read_to_string(sentinel).unwrap(), "do not overwrite");
}

#[test]
fn pack_rejects_native_resource_texture_and_manifest_tampering() {
    let temp = tempfile::tempdir().unwrap();
    let request = fixture(temp.path());
    for mutation in [
        "script",
        "scene",
        "helper",
        "texture",
        "quality",
        "hash",
        "unknown_field",
        "undeclared",
        "schema",
        "runtime_profile",
        "blend_shader",
    ] {
        let output =
            prepare_layered_pack(&request, &temp.path().join(format!("{mutation}.gsfpack")))
                .unwrap();
        match mutation {
            "script" => fs::write(
                output.pack_dir.join("assets/forge_layered_player.gd"),
                "extends Node2D\n",
            )
            .unwrap(),
            "blend_shader" => fs::write(
                output.pack_dir.join("assets/forge_alpha_multiply.gdshader"),
                "shader_type canvas_item;\n",
            )
            .unwrap(),
            "scene" => fs::write(
                &output.scene_path,
                "[gd_scene format=3]\n[node name=\"unexpected\" type=\"Node2D\"]\n",
            )
            .unwrap(),
            "helper" => change_json(&output.pack_dir.join("assets/godot_import.json"), |v| {
                v["layerOrder"] = json!(["front", "rear"])
            }),
            "texture" => fs::write(
                output.pack_dir.join("assets/layers/rear.png"),
                fs::read(&request.layers[1].path).unwrap(),
            )
            .unwrap(),
            "quality" => change_json(&output.quality_report_path, |v| {
                v["verdict"] = json!("game_ready")
            }),
            "hash" => change_json(&output.manifest_path, |v| {
                v["layers"][0]["sha256"] = json!("0".repeat(64))
            }),
            "unknown_field" => change_json(&output.manifest_path, |v| {
                v["layers"][0]["parent"] = json!("front")
            }),
            "undeclared" => fs::write(
                output.pack_dir.join("assets/layers/extra.png"),
                fs::read(&request.layers[0].path).unwrap(),
            )
            .unwrap(),
            "schema" => change_json(&output.pack_dir.join("forgepack.json"), |v| {
                v["schemaVersion"] = json!("2.0.0")
            }),
            "runtime_profile" => {
                change_json(&output.pack_dir.join("assets/godot_import.json"), |v| {
                    v["profile"] = json!("godot-layered@999.0.0")
                })
            }
            _ => unreachable!(),
        }
        assert!(
            forge_pack::validate_pack_layout(&output.pack_dir).is_err(),
            "{mutation}"
        );
    }
}

#[test]
fn malformed_rgba_and_opaque_rgb_sources_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let request = fixture(temp.path());
    let rgb_path = temp.path().join("rgb.png");
    image::RgbImage::new(37, 23).save(&rgb_path).unwrap();
    let broken_path = temp.path().join("broken.png");
    fs::write(&broken_path, b"not an image").unwrap();
    for path in [rgb_path, broken_path] {
        let mut bad = request.clone();
        bad.layers[0].path = path.clone();
        bad.layers[0].sha256 = format!("{:x}", Sha256::digest(fs::read(path).unwrap()));
        assert!(validate_request(&bad).is_err());
    }
    // Fully opaque RGBA is legal for a background layer; alpha is not cropped.
    let rgba_path = temp.path().join("opaque.png");
    RgbaImage::from_pixel(37, 23, Rgba([20, 40, 60, 255]))
        .save(&rgba_path)
        .unwrap();
    let mut okay = request;
    okay.layers[0].path = rgba_path.clone();
    okay.layers[0].sha256 = format!("{:x}", Sha256::digest(fs::read(rgba_path).unwrap()));
    assert!(validate_request(&okay).is_ok());
}

#[test]
fn truncated_corrupt_iend_and_apng_fail_even_with_matching_source_hashes() {
    let temp = tempfile::tempdir().unwrap();
    let request = fixture(temp.path());
    let original = fs::read(&request.layers[0].path).unwrap();
    let mut truncated = original[..original.len() - 4].to_vec();
    let mut corrupt = original.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    let mut apng = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut apng, 37, 23);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_animated(2, 0).unwrap();
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&vec![255; 37 * 23 * 4]).unwrap();
        writer.write_image_data(&vec![128; 37 * 23 * 4]).unwrap();
        writer.finish().unwrap();
    }
    for (label, bytes) in [
        ("truncated", &mut truncated),
        ("corrupt", &mut corrupt),
        ("apng", &mut apng),
    ] {
        let path = temp.path().join(format!("{label}.png"));
        fs::write(&path, &*bytes).unwrap();
        let mut bad = request.clone();
        bad.layers[0].path = path;
        bad.layers[0].sha256 = format!("{:x}", Sha256::digest(&*bytes));
        assert!(validate_request(&bad).is_err(), "{label}");
        let pack =
            prepare_layered_pack(&request, &temp.path().join(format!("{label}.gsfpack"))).unwrap();
        fs::write(pack.pack_dir.join("assets/layers/rear.png"), &*bytes).unwrap();
        // Change both provenance declarations; PNG container validation still
        // fails independently of the declared content hash.
        change_json(&pack.manifest_path, |v| {
            v["layers"][0]["sha256"] = json!(bad.layers[0].sha256)
        });
        change_json(&pack.pack_dir.join("forgepack.json"), |v| {
            v["source"]["metadata"]["sources"][0]["sha256"] = json!(bad.layers[0].sha256)
        });
        assert!(
            forge_pack::validate_pack_layout(&pack.pack_dir).is_err(),
            "{label}"
        );
    }
}
