use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::thread;

use image::{Rgba, RgbaImage};
use serde_json::{json, Value};
use tempfile::tempdir;

#[test]
fn local_png_uses_same_job_and_review_contract_without_regeneration() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("input.png");
    let mut image = RgbaImage::new(32, 32);
    for y in 6..26 {
        for x in 8..24 {
            image.put_pixel(x, y, Rgba([255, 0, 50, 255]));
        }
    }
    image.save(&source).unwrap();
    let request = dir.path().join("request.json");
    fs::write(
        &request,
        json!({
            "schemaVersion": "1", "mediaKind": "image", "source": source,
            "assetId": "test_icon", "name": "Test icon", "purpose": "HUD test",
            "kind": "icon_set", "license": "private", "sampling": "nearest", "canvasSize": 64
        })
        .to_string(),
    )
    .unwrap();
    let call = |args: &[&str]| -> Value {
        let output = Command::new(env!("CARGO_BIN_EXE_forge"))
            .args(args)
            .env("FORGE_JOB_STORE", dir.path().join("jobs"))
            .env("FORGE_PLAN_STORE", dir.path().join("plans"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    };
    let first = call(&[
        "asset",
        "create",
        "--input",
        request.to_str().unwrap(),
        "--wait",
        "--json",
    ]);
    assert_eq!(first["ok"], true);
    assert_eq!(first["data"]["state"], "awaiting_review");
    assert!(first["data"]["previewPath"]
        .as_str()
        .is_some_and(|p| std::path::Path::new(p).is_file()));
    assert!(first["data"]["packPath"]
        .as_str()
        .is_some_and(|p| std::path::Path::new(p).is_dir()));
    let job = first["data"]["jobId"].as_str().unwrap();
    let source_sha = first["data"]["sourceSha256"].as_str().unwrap();
    let invalid_review = dir.path().join("wrong-review.json");
    fs::write(
        &invalid_review,
        json!({"schemaVersion":"1", "sourceSha256":"0".repeat(64),
        "approved":true, "reviewer":"fixture reviewer"})
        .to_string(),
    )
    .unwrap();
    let refused = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args([
            "asset",
            "create",
            "--resume",
            job,
            "--review",
            invalid_review.to_str().unwrap(),
            "--json",
        ])
        .env("FORGE_JOB_STORE", dir.path().join("jobs"))
        .env("FORGE_PLAN_STORE", dir.path().join("plans"))
        .output()
        .unwrap();
    assert!(!refused.status.success());
    let failure: Value = serde_json::from_slice(&refused.stdout).unwrap();
    assert_eq!(failure["error"]["code"], "asset_review_invalid");
    let review = dir.path().join("review.json");
    fs::write(
        &review,
        json!({"schemaVersion":"1", "sourceSha256":source_sha,
        "approved":true, "reviewer":"fixture reviewer"})
        .to_string(),
    )
    .unwrap();
    let second = call(&[
        "asset",
        "create",
        "--resume",
        job,
        "--review",
        review.to_str().unwrap(),
        "--wait",
        "--json",
    ]);
    assert_eq!(second["data"]["state"], "succeeded");
    assert_eq!(
        second["data"]["sourceSha256"],
        first["data"]["sourceSha256"]
    );
    assert_eq!(
        second["data"]["prepareJobId"],
        first["data"]["prepareJobId"]
    );
    fs::write(first["data"]["sourcePath"].as_str().unwrap(), b"changed").unwrap();
    let tampered = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(["asset", "create", "--resume", job, "--json"])
        .env("FORGE_JOB_STORE", dir.path().join("jobs"))
        .env("FORGE_PLAN_STORE", dir.path().join("plans"))
        .output()
        .unwrap();
    assert!(!tampered.status.success());
    let failure: Value = serde_json::from_slice(&tampered.stdout).unwrap();
    assert_eq!(failure["error"]["code"], "asset_source_changed");
}

#[test]
fn comfy_image_request_uses_one_prompt_and_prepares_source() {
    let dir = tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let workflow = dir.path().join("workflow.json");
    fs::write(
        &workflow,
        json!({
            "1": {"class_type":"CLIPTextEncode", "inputs":{"text":"old"}},
            "2": {"class_type":"SaveImage", "inputs":{"images":["1",0]}}
        })
        .to_string(),
    )
    .unwrap();
    let profile = dir.path().join("profile.json");
    fs::write(
        &profile,
        json!({"schemaVersion":1,"endpoint":endpoint,"mediaKind":"image",
        "modelId":"fixture-qwen","workflow":workflow,"promptInput":{"node":"1","input":"text"},
        "outputNode":"2","outputField":"images","maxOutputBytes":1000000,"timeoutSeconds":5})
        .to_string(),
    )
    .unwrap();
    let root = dir.path().join("profiles");
    let configured = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args([
            "provider",
            "configure",
            "--provider",
            "comfyui",
            "--profile",
            "mock",
            "--config",
            profile.to_str().unwrap(),
            "--json",
        ])
        .env("FORGE_COMFYUI_PROFILE_DIR", &root)
        .output()
        .unwrap();
    assert!(
        configured.status.success(),
        "{}",
        String::from_utf8_lossy(&configured.stdout)
    );
    let source = dir.path().join("mock.png");
    let mut image = RgbaImage::new(32, 32);
    for y in 4..28 {
        for x in 4..28 {
            image.put_pixel(x, y, Rgba([5, 30, 200, 255]));
        }
    }
    image.save(&source).unwrap();
    let png = fs::read(source).unwrap();
    let worker = thread::spawn(move || {
        let mut prompt_id = String::new();
        for index in 0..4 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .unwrap();
            let mut request = Vec::new();
            let mut chunk = [0u8; 4096];
            loop {
                let n = stream.read(&mut chunk).unwrap();
                request.extend_from_slice(&chunk[..n]);
                if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&request[..end]);
                    let length = header
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length: ")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let text = String::from_utf8_lossy(&request);
            let body = match index {
                0 => json!({"CLIPTextEncode":{"input":{"required":{"text":[]}}},
                    "SaveImage":{"input":{"required":{"images":[]}},"output_node":true}})
                .to_string()
                .into_bytes(),
                1 => {
                    assert!(text.starts_with("POST /prompt"));
                    let end = request.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
                    let posted: Value = serde_json::from_slice(&request[end + 4..]).unwrap();
                    assert_eq!(posted["prompt"]["1"]["inputs"]["text"], "a blue icon");
                    prompt_id = posted["prompt_id"].as_str().unwrap().into();
                    json!({"prompt_id":prompt_id}).to_string().into_bytes()
                }
                2 => {
                    assert!(text.starts_with(&format!("GET /history/{prompt_id}")));
                    json!({prompt_id.clone(): {"status":{"completed":true},
                        "outputs":{"2":{"images":[{"filename":"generated.png","subfolder":"","type":"output"}]}}}}).to_string().into_bytes()
                }
                _ => {
                    assert!(text.starts_with("GET /view?"));
                    png.clone()
                }
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
            stream.write_all(&body).unwrap();
        }
    });
    let request = dir.path().join("request.json");
    fs::write(&request, json!({"schemaVersion":"1","mediaKind":"image", "workflowProfile":"mock",
        "prompt":"a blue icon","assetId":"blue_icon","name":"Blue icon","purpose":"HUD",
        "kind":"icon_set","license":"private","sampling":"nearest","canvasSize":64,"maxWaitSeconds":10}).to_string()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args([
            "asset",
            "create",
            "--input",
            request.to_str().unwrap(),
            "--wait",
            "--json",
        ])
        .env("FORGE_COMFYUI_PROFILE_DIR", root)
        .env("FORGE_JOB_STORE", dir.path().join("jobs"))
        .env("FORGE_PLAN_STORE", dir.path().join("plans"))
        .output()
        .unwrap();
    worker.join().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["data"]["state"], "awaiting_review");
    assert!(result["data"]["sourceSha256"].as_str().is_some());
    assert!(result["data"]["previewPath"].as_str().is_some());
    // The server has exited; resume must use the retained source and Pack.
    let resumed = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args([
            "asset",
            "create",
            "--resume",
            result["data"]["jobId"].as_str().unwrap(),
            "--wait",
            "--json",
        ])
        .env("FORGE_COMFYUI_PROFILE_DIR", dir.path().join("profiles"))
        .env("FORGE_JOB_STORE", dir.path().join("jobs"))
        .env("FORGE_PLAN_STORE", dir.path().join("plans"))
        .output()
        .unwrap();
    assert!(resumed.status.success());
    let again: Value = serde_json::from_slice(&resumed.stdout).unwrap();
    assert_eq!(
        again["data"]["sourceSha256"],
        result["data"]["sourceSha256"]
    );
    assert_eq!(
        again["data"]["prepareJobId"],
        result["data"]["prepareJobId"]
    );
}

#[test]
#[ignore = "requires a native Godot executable via GODOT_BIN"]
fn reviewed_image_installs_and_loads_in_native_godot() {
    let godot = std::env::var("GODOT_BIN").expect("set GODOT_BIN to Godot console executable");
    let dir = tempdir().unwrap();
    let game = dir.path().join("game");
    fs::create_dir_all(&game).unwrap();
    fs::write(game.join("project.godot"), "config_version=5\n[application]\nconfig/name=\"ComfyUI image delivery test\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n").unwrap();
    let source = dir.path().join("icon.png");
    let mut image = RgbaImage::new(32, 32);
    for y in 4..28 {
        for x in 4..28 {
            image.put_pixel(x, y, Rgba([5, 200, 30, 255]));
        }
    }
    image.save(&source).unwrap();
    let request = dir.path().join("request.json");
    fs::write(
        &request,
        json!({"schemaVersion":"1","mediaKind":"image","source":source,
        "assetId":"native_icon","name":"Native icon","purpose":"HUD test", "kind":"icon_set",
        "license":"private","sampling":"nearest","canvasSize":64,
        "godotProject":game,"installTarget":"addons/forge_assets/native_icon","assetKey":"native_icon"})
        .to_string(),
    )
    .unwrap();
    let run = |args: &[&str]| -> Value {
        let output = Command::new(env!("CARGO_BIN_EXE_forge"))
            .args(args)
            .env("FORGE_JOB_STORE", dir.path().join("jobs"))
            .env("FORGE_PLAN_STORE", dir.path().join("plans"))
            .env("FORGE_GODOT_PATH", &godot)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    };
    let first = run(&[
        "asset",
        "create",
        "--input",
        request.to_str().unwrap(),
        "--wait",
        "--json",
    ]);
    assert_eq!(first["data"]["state"], "awaiting_review");
    let review = dir.path().join("review.json");
    fs::write(
        &review,
        json!({"schemaVersion":"1","sourceSha256":first["data"]["sourceSha256"],
        "approved":true,"reviewer":"native fixture review"})
        .to_string(),
    )
    .unwrap();
    let second = run(&[
        "asset",
        "create",
        "--resume",
        first["data"]["jobId"].as_str().unwrap(),
        "--review",
        review.to_str().unwrap(),
        "--wait",
        "--json",
    ]);
    assert_eq!(second["data"]["state"], "succeeded");
    assert!(second["data"]["installJobId"].as_str().is_some());
    let script = game.join("verify.gd");
    fs::write(&script, "extends SceneTree\nfunc _initialize() -> void:\n\tassert(load(\"res://addons/forge_assets/native_icon/items/native_icon.png\") is Texture2D)\n\tprint(\"PASS local asset create native Godot\")\n\tquit(0)\n").unwrap();
    let checked = Command::new(&godot)
        .args([
            "--headless",
            "--path",
            game.to_str().unwrap(),
            "--script",
            "res://verify.gd",
            "--quit-after",
            "120",
        ])
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    assert!(
        String::from_utf8_lossy(&checked.stdout).contains("PASS local asset create native Godot")
    );
}
