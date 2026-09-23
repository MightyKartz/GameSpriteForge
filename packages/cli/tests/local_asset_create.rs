use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;
use std::thread;

use image::{Rgba, RgbaImage};
use serde_json::{json, Value};
use tempfile::tempdir;

#[test]
fn batch_budget_rejects_before_creating_any_job() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("icon.png");
    RgbaImage::from_pixel(16, 16, Rgba([10, 20, 30, 255]))
        .save(&source)
        .unwrap();
    let request = dir.path().join("request.json");
    fs::write(
        &request,
        json!({"schemaVersion":"1","mediaKind":"image","source":source,
        "assetId":"budget_icon","name":"Budget icon","purpose":"QA","kind":"icon_set",
        "license":"test-only","canvasSize":64,"maxWaitSeconds":900})
        .to_string(),
    )
    .unwrap();
    let manifest = dir.path().join("batch.json");
    fs::write(
        &manifest,
        json!({"schemaVersion":"1","requests":["request.json"],
        "maxRequests":1,"maxTotalWaitSeconds":899,"maxTotalOutputBytes":1})
        .to_string(),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args([
            "asset",
            "batch",
            "--input",
            manifest.to_str().unwrap(),
            "--json",
        ])
        .env("FORGE_JOB_STORE", dir.path().join("jobs"))
        .env("FORGE_PLAN_STORE", dir.path().join("plans"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["error"]["code"], "asset_batch_budget_exceeded");
    assert!(!dir.path().join("jobs").exists());
}

#[test]
fn batch_starts_distinct_recoverable_jobs_within_budget() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("icon.png");
    let mut image = RgbaImage::new(32, 32);
    for y in 4..28 {
        for x in 4..28 {
            image.put_pixel(x, y, Rgba([10, 20, 30, 255]));
        }
    }
    image.save(&source).unwrap();
    for (filename, id) in [("first.json", "first_icon"), ("second.json", "second_icon")] {
        fs::write(
            dir.path().join(filename),
            json!({"schemaVersion":"1","mediaKind":"image","source":source,
            "assetId":id,"name":id,"purpose":"QA","kind":"icon_set",
            "license":"test-only","canvasSize":64,"maxWaitSeconds":60})
            .to_string(),
        )
        .unwrap();
    }
    let manifest = dir.path().join("batch.json");
    fs::write(
        &manifest,
        json!({"schemaVersion":"1","requests":["first.json","second.json"],
        "maxRequests":2,"maxTotalWaitSeconds":120,
        "maxTotalOutputBytes":fs::metadata(&source).unwrap().len()*2})
        .to_string(),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args([
            "asset",
            "batch",
            "--input",
            manifest.to_str().unwrap(),
            "--json",
        ])
        .env("FORGE_JOB_STORE", dir.path().join("jobs"))
        .env("FORGE_PLAN_STORE", dir.path().join("plans"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["data"]["requestCount"], 2);
    assert_eq!(result["data"]["reservedWaitSeconds"], 120);
    let jobs = result["data"]["jobs"].as_array().unwrap();
    assert_eq!(jobs.len(), 2);
    assert_ne!(jobs[0]["jobId"], jobs[1]["jobId"]);
    assert!(jobs.iter().all(|job| job["state"] == "awaiting_review"));
    for job in jobs {
        let id = job["jobId"].as_str().unwrap();
        assert!(dir.path().join("jobs").join(id).exists());
        assert!(job["sourcePath"]
            .as_str()
            .is_some_and(|p| std::path::Path::new(p).exists()));
    }
}

#[test]
fn explicit_static_matting_preserves_generated_source_and_prepares_derived_png() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("opaque.png");
    let mut image = RgbaImage::from_pixel(64, 64, Rgba([90, 30, 130, 255]));
    for y in 12..52 {
        for x in 20..44 {
            image.put_pixel(x, y, Rgba([240, 20, 20, 255]));
        }
    }
    image.save(&source).unwrap();
    let original = fs::read(&source).unwrap();
    let request = dir.path().join("request.json");
    fs::write(
        &request,
        json!({
            "schemaVersion":"1", "mediaKind":"image", "source":source,
            "assetId":"matted_icon", "name":"Matted icon", "purpose":"QA icon",
            "kind":"icon_set", "license":"test-only", "canvasSize":64,
            "staticMatting":"auto_corners"
        })
        .to_string(),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args([
            "asset",
            "create",
            "--input",
            request.to_str().unwrap(),
            "--json",
        ])
        .env("FORGE_JOB_STORE", dir.path().join("jobs"))
        .env("FORGE_PLAN_STORE", dir.path().join("plans"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["data"]["state"], "awaiting_review");
    let retained = result["data"]["sourcePath"].as_str().unwrap();
    assert_eq!(fs::read(retained).unwrap(), original);
    let derived = result["data"]["processedSourcePath"].as_str().unwrap();
    let decoded = image::open(derived).unwrap().to_rgba8();
    assert_eq!(decoded.get_pixel(0, 0)[3], 0);
    assert_eq!(decoded.get_pixel(32, 32)[3], 255);
    assert_ne!(
        result["data"]["processedSourceSha256"],
        result["data"]["sourceSha256"]
    );
}

#[test]
#[ignore = "requires native ffmpeg/ffprobe through GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS"]
fn local_mp4_prepares_character_pack_and_waits_for_visual_review() {
    let dir = tempdir().unwrap();
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = repo
        .join("docs/media/showcase/thunder/godot-demo.mp4")
        .canonicalize()
        .unwrap();
    let frame = repo
        .join("docs/qa/artifacts/forge-character-v2-real-20260804/run-3/sample-idle-frame-00.png")
        .canonicalize()
        .unwrap();
    let request = dir.path().join("request.json");
    fs::write(
        &request,
        json!({
            "schemaVersion":"1", "mediaKind":"video", "source":source,
            "assetId":"local_h3_fixture", "name":"Local video fixture", "purpose":"character QA",
            "kind":"character", "license":"test-only",
            "animationName":"motion", "animationFps":8, "targetFrameCount":8,
            "canvasSize":256,
            "loopAnimation":false, "mattingMode":"auto_corners",
            "supportAnimations":[{"name":"idle", "fps":2, "loop":true,
                "input":{"kind":"png_sequence", "paths":[frame,frame]},
                "matting":{"mode":"preserve_alpha"}}]
        })
        .to_string(),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args([
            "asset",
            "create",
            "--input",
            request.to_str().unwrap(),
            "--json",
        ])
        .env("FORGE_JOB_STORE", dir.path().join("jobs"))
        .env("FORGE_PLAN_STORE", dir.path().join("plans"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["data"]["state"], "awaiting_review");
    assert_eq!(result["data"]["mediaKind"], "video");
    assert!(
        result["data"]["videoProbe"]["frameCountEstimate"]
            .as_u64()
            .unwrap()
            > 0
    );
    let pack = Path::new(result["data"]["packPath"].as_str().unwrap());
    assert!(pack.is_dir());
    let frames = pack.join("assets/frames");
    let frame_paths: Vec<_> = fs::read_dir(frames)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "png"))
        .collect();
    assert!(!frame_paths.is_empty());
    for path in frame_paths {
        assert_eq!(image::image_dimensions(path).unwrap(), (256, 256));
    }
    assert!(std::path::Path::new(result["data"]["previewPath"].as_str().unwrap()).is_file());
    assert!(result["data"]["installJobId"].is_null());
}

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
fn concurrent_resume_observes_active_job_without_reprocessing() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("icon.png");
    let mut image = RgbaImage::new(32, 32);
    for y in 4..28 {
        for x in 4..28 {
            image.put_pixel(x, y, Rgba([20, 160, 210, 255]));
        }
    }
    image.save(&source).unwrap();
    let request = dir.path().join("request.json");
    fs::write(
        &request,
        json!({"schemaVersion":"1","mediaKind":"image","source":source,
        "assetId":"locked_icon","name":"Locked icon","purpose":"QA","kind":"icon_set",
        "license":"test-only","canvasSize":64})
        .to_string(),
    )
    .unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_forge"))
            .args(args)
            .env("FORGE_JOB_STORE", dir.path().join("jobs"))
            .env("FORGE_PLAN_STORE", dir.path().join("plans"))
            .output()
            .unwrap()
    };
    let first = run(&[
        "asset",
        "create",
        "--input",
        request.to_str().unwrap(),
        "--json",
    ]);
    assert!(first.status.success());
    let first: Value = serde_json::from_slice(&first.stdout).unwrap();
    let job_id = first["data"]["jobId"].as_str().unwrap();
    let prepare_id = first["data"]["prepareJobId"].clone();
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(
            dir.path()
                .join("jobs")
                .join(job_id)
                .join(".asset-create.lock"),
        )
        .unwrap();
    lock.lock().unwrap();
    let concurrent = run(&["asset", "create", "--resume", job_id, "--wait", "--json"]);
    assert!(concurrent.status.success());
    let concurrent: Value = serde_json::from_slice(&concurrent.stdout).unwrap();
    assert_eq!(concurrent["data"]["prepareJobId"], prepare_id);
    assert_eq!(concurrent["data"]["nextActions"][0], "wait_for_active_call");
    let review = dir.path().join("review.json");
    fs::write(
        &review,
        json!({"schemaVersion":"1","sourceSha256":first["data"]["sourceSha256"],
        "approved":true,"reviewer":"QA"})
        .to_string(),
    )
    .unwrap();
    let busy_review = run(&[
        "asset",
        "create",
        "--resume",
        job_id,
        "--review",
        review.to_str().unwrap(),
        "--json",
    ]);
    assert!(!busy_review.status.success());
    let busy_review: Value = serde_json::from_slice(&busy_review.stdout).unwrap();
    assert_eq!(busy_review["error"]["code"], "asset_busy");
    drop(lock);
    let resumed = run(&["asset", "create", "--resume", job_id, "--json"]);
    assert!(resumed.status.success());
    let resumed: Value = serde_json::from_slice(&resumed.stdout).unwrap();
    assert_eq!(resumed["data"]["prepareJobId"], prepare_id);
    assert_eq!(resumed["data"]["state"], "awaiting_review");
}

#[test]
fn cancellation_requested_during_active_generation_is_served_by_owner() {
    let dir = tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let workflow = dir.path().join("workflow.json");
    fs::write(
        &workflow,
        json!({"1":{"class_type":"CLIPTextEncode","inputs":{"text":"old"}},
        "2":{"class_type":"SaveImage","inputs":{"images":["1",0]}}})
        .to_string(),
    )
    .unwrap();
    let profile = dir.path().join("profile.json");
    fs::write(
        &profile,
        json!({"schemaVersion":1,"endpoint":endpoint,"mediaKind":"image",
        "modelId":"fixture","workflow":workflow,"promptInput":{"node":"1","input":"text"},
        "outputNode":"2","outputField":"images","maxOutputBytes":1000000,"timeoutSeconds":5})
        .to_string(),
    )
    .unwrap();
    let profile_root = dir.path().join("profiles");
    let configured = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args([
            "provider",
            "configure",
            "--provider",
            "comfyui",
            "--profile",
            "pending",
            "--config",
            profile.to_str().unwrap(),
            "--json",
        ])
        .env("FORGE_COMFYUI_PROFILE_DIR", &profile_root)
        .output()
        .unwrap();
    assert!(configured.status.success());
    let worker = thread::spawn(move || {
        let mut prompt_id = String::new();
        for index in 0..5 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .unwrap();
            let mut request = Vec::new();
            let mut chunk = [0u8; 4096];
            loop {
                let n = stream.read(&mut chunk).unwrap();
                request.extend_from_slice(&chunk[..n]);
                let end = request.windows(4).position(|w| w == b"\r\n\r\n");
                if let Some(end) = end {
                    let header = String::from_utf8_lossy(&request[..end]);
                    let length = header
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length: ")
                                .and_then(|value| value.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let text = String::from_utf8_lossy(&request);
            let body = match index {
                0 => {
                    assert!(text.starts_with("GET /object_info"));
                    json!({"CLIPTextEncode":{"input":{"required":{"text":[]}}},
                    "SaveImage":{"input":{"required":{"images":[]}},"output_node":true}})
                }
                1 => {
                    assert!(text.starts_with("POST /prompt"));
                    let end = request.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
                    let posted: Value = serde_json::from_slice(&request[end + 4..]).unwrap();
                    prompt_id = posted["prompt_id"].as_str().unwrap().into();
                    json!({"prompt_id":prompt_id})
                }
                2 => {
                    assert!(text.starts_with("GET /queue"));
                    json!({"queue_running":[],"queue_pending":[[0,prompt_id,{},
                        {"client_id":"forge"}]]})
                }
                3 => {
                    assert!(text.starts_with("POST /queue"));
                    let end = request.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
                    let posted: Value = serde_json::from_slice(&request[end + 4..]).unwrap();
                    assert_eq!(posted["delete"][0], prompt_id);
                    json!({})
                }
                _ => {
                    assert!(text.starts_with("GET /queue"));
                    json!({"queue_running":[],"queue_pending":[]})
                }
            }
            .to_string();
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .as_bytes(),
                )
                .unwrap();
            stream.write_all(body.as_bytes()).unwrap();
        }
    });
    let request = dir.path().join("request.json");
    fs::write(
        &request,
        json!({"schemaVersion":"1","mediaKind":"image","workflowProfile":"pending",
        "prompt":"blue icon","assetId":"pending_icon","name":"Pending icon",
        "purpose":"QA","kind":"icon_set","license":"test-only","canvasSize":64})
        .to_string(),
    )
    .unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_forge"))
            .args(args)
            .env("FORGE_COMFYUI_PROFILE_DIR", &profile_root)
            .env("FORGE_JOB_STORE", dir.path().join("jobs"))
            .env("FORGE_PLAN_STORE", dir.path().join("plans"))
            .output()
            .unwrap()
    };
    let first = run(&[
        "asset",
        "create",
        "--input",
        request.to_str().unwrap(),
        "--json",
    ]);
    assert!(first.status.success());
    let first: Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(first["data"]["phase"], "generating");
    let id = first["data"]["jobId"].as_str().unwrap();
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(dir.path().join("jobs").join(id).join(".asset-create.lock"))
        .unwrap();
    lock.lock().unwrap();
    let queued = run(&["asset", "create", "--resume", id, "--cancel", "--json"]);
    assert!(queued.status.success());
    let queued: Value = serde_json::from_slice(&queued.stdout).unwrap();
    assert_eq!(queued["data"]["nextActions"][0], "wait_for_cancellation");
    drop(lock);
    let cancelled = run(&["asset", "create", "--resume", id, "--wait", "--json"]);
    worker.join().unwrap();
    assert!(
        cancelled.status.success(),
        "{}",
        String::from_utf8_lossy(&cancelled.stdout)
    );
    let cancelled: Value = serde_json::from_slice(&cancelled.stdout).unwrap();
    assert_eq!(cancelled["data"]["state"], "cancelled");
    assert!(cancelled["data"]["prepareJobId"].is_null());
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
            .env("FORGE_JOB_STORE", game.join("jobs"))
            .env("FORGE_PLAN_STORE", game.join("plans"))
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
    let automatic_receipt = second["data"]["receiptPath"].as_str().unwrap();
    let automatic_hash = second["data"]["receiptSha256"].as_str().unwrap();
    assert!(Path::new(automatic_receipt).is_file());
    assert_eq!(automatic_hash.len(), 64);
    assert!(game.join("jobs/.gdignore").is_file());
    assert!(game.join("plans/.gdignore").is_file());
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
    let verified = run(&[
        "godot",
        "verify-install",
        "--project",
        game.to_str().unwrap(),
        "--asset-key",
        "native_icon",
        "--json",
    ]);
    assert_eq!(verified["data"]["verifiedTextures"], 1);
    let automatic_verified = run(&[
        "receipt",
        "verify",
        "--path",
        automatic_receipt,
        "--expected-sha256",
        automatic_hash,
        "--json",
    ]);
    assert_eq!(automatic_verified["data"]["verified"], true);
    assert_eq!(automatic_verified["data"]["installationVerified"], true);
    let receipt = dir.path().join("receipt.json");
    let exported = run(&[
        "receipt",
        "export",
        "--job",
        second["data"]["prepareJobId"].as_str().unwrap(),
        "--install-job",
        second["data"]["installJobId"].as_str().unwrap(),
        "--out",
        receipt.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(exported["ok"], true);
    let checked_receipt = run(&[
        "receipt",
        "verify",
        "--path",
        receipt.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(checked_receipt["data"]["verified"], true);
}
