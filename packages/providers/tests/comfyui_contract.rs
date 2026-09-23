use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use forge_providers::comfyui::{
    check_upgrade, configure, export_descriptor, import_descriptor, load, parse_history,
    ComfyClient, ComfyError, HistoryState, MediaKind, NodeInput, OutputField, WorkflowProfile,
};
use serde_json::json;
use tempfile::tempdir;

fn fixture(endpoint: &str) -> (tempfile::TempDir, WorkflowProfile) {
    let dir = tempdir().unwrap();
    let workflow = dir.path().join("workflow.json");
    fs::write(&workflow, json!({"1": {"class_type": "CLIPTextEncode", "inputs": {"text": "old"}}, "2": {"class_type": "SaveImage", "inputs": {"images": ["1", 0]}}}).to_string()).unwrap();
    let profile = WorkflowProfile {
        reference_input: None,
        schema_version: 1,
        endpoint: endpoint.into(),
        allow_remote: false,
        media_kind: MediaKind::Image,
        model_id: "qwen-image-2.1".into(),
        workflow,
        prompt_input: NodeInput {
            node: "1".into(),
            input: "text".into(),
        },
        output_node: "2".into(),
        output_field: OutputField::Images,
        max_output_bytes: 1024,
        timeout_seconds: 5,
    };
    (dir, profile)
}

#[test]
fn profile_import_is_explicit_immutable_and_rejects_ui_workflow() {
    let (dir, profile) = fixture("http://127.0.0.1:8188");
    let config = dir.path().join("profile.json");
    fs::write(&config, serde_json::to_vec(&profile).unwrap()).unwrap();
    let root = dir.path().join("profiles");
    let stored = configure(&root, "local", &config).unwrap();
    assert_eq!(stored.profile_id, "local");
    assert_eq!(
        configure(&root, "local", &config).unwrap().workflow_sha256,
        stored.workflow_sha256
    );
    assert!(matches!(
        configure(&root, "../escape", &config),
        Err(ComfyError::InvalidProfile(_))
    ));
    fs::write(&profile.workflow, r#"{"nodes": []}"#).unwrap();
    assert!(matches!(
        load(&root, "local"),
        Err(ComfyError::InvalidProfile(_))
    ));
}

#[test]
fn portable_descriptor_requires_matching_workflow_and_upgrade_is_read_only() {
    let (dir, profile) = fixture("http://127.0.0.1:8188");
    let config = dir.path().join("profile.json");
    fs::write(&config, serde_json::to_vec(&profile).unwrap()).unwrap();
    let root = dir.path().join("profiles");
    let original = configure(&root, "local", &config).unwrap();
    let export = dir.path().join("descriptor.json");
    let descriptor = export_descriptor(&root, "local", &export).unwrap();
    let text = fs::read_to_string(&export).unwrap();
    assert!(!text.contains("workflow.json"));
    assert!(!text.contains("class_type"));
    assert_eq!(descriptor.workflow_sha256, original.workflow_sha256);
    let second_root = dir.path().join("second");
    assert_eq!(
        import_descriptor(&second_root, "copy", &export, &profile.workflow)
            .unwrap()
            .workflow_sha256,
        original.workflow_sha256
    );
    let altered = dir.path().join("altered.json");
    fs::write(
        &altered,
        json!({
            "1":{"class_type":"CLIPTextEncode", "inputs":{"text":"changed"}},
            "2":{"class_type":"SaveImage", "inputs":{"images":["1",0]}}
        })
        .to_string(),
    )
    .unwrap();
    assert!(matches!(
        import_descriptor(&second_root, "bad", &export, &altered),
        Err(ComfyError::WorkflowChanged)
    ));
    assert!(!second_root.join("bad.json").exists());
    let mut candidate = profile;
    candidate.workflow = altered;
    let candidate_config = dir.path().join("candidate.json");
    fs::write(&candidate_config, serde_json::to_vec(&candidate).unwrap()).unwrap();
    let check = check_upgrade(&root, "local", &candidate_config).unwrap();
    assert!(check.requires_new_profile_id);
    assert_eq!(check.changed_fields, ["workflowSha256"]);
    assert_eq!(
        load(&root, "local").unwrap().workflow_sha256,
        original.workflow_sha256
    );
}

#[test]
fn history_is_scoped_to_prompt_and_named_output() {
    let (_dir, profile) = fixture("http://127.0.0.1:8188");
    let history = json!({"other": {"status": {"completed": true}, "outputs": {"2": {"images": [{"filename": "other.png"}]}}}});
    assert_eq!(
        parse_history(&history, "ours", &profile).unwrap(),
        HistoryState::Pending
    );
    let history = json!({"ours": {"status": {"completed": true}, "outputs": {"2": {"images": [{"filename": "ours.png", "subfolder": "x", "type": "output"}]}}}});
    assert!(
        matches!(parse_history(&history, "ours", &profile).unwrap(), HistoryState::Succeeded(output) if output.filename == "ours.png")
    );
    let bad = json!({"ours": {"status": {"completed": true}, "outputs": {"9": {"images": []}}}});
    assert!(matches!(
        parse_history(&bad, "ours", &profile),
        Err(ComfyError::InvalidOutput(_))
    ));
}

#[test]
fn transport_checks_nodes_submits_exact_prompt_and_bounds_media() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (dir, profile) = fixture(&endpoint);
    let mut workflow: serde_json::Value =
        serde_json::from_slice(&fs::read(&profile.workflow).unwrap()).unwrap();
    workflow["2"]["inputs"]["format.bit_depth"] = json!("8-bit");
    fs::write(&profile.workflow, workflow.to_string()).unwrap();
    let config = dir.path().join("profile.json");
    fs::write(&config, serde_json::to_vec(&profile).unwrap()).unwrap();
    let stored = configure(&dir.path().join("profiles"), "local", &config).unwrap();
    let worker = thread::spawn(move || {
        for index in 0..4 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut chunk = [0; 4096];
            loop {
                let n = stream.read(&mut chunk).unwrap();
                bytes.extend_from_slice(&chunk[..n]);
                if let Some(header_end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&bytes[..header_end]);
                    let length = header
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length: ")
                                .and_then(|s| s.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if bytes.len() >= header_end + 4 + length {
                        break;
                    }
                }
            }
            let request = String::from_utf8_lossy(&bytes);
            let body = match index {
                0 => json!({
                    "CLIPTextEncode": {"input": {"required": {"text": []}}},
                    "SaveImage": {"input": {"required": {"images": []}}, "output_node": true}
                })
                .to_string(),
                1 => {
                    assert!(request.starts_with("POST /prompt"));
                    assert!(request.contains("new prompt"));
                    assert!(request.contains("00000000-0000-4000-8000-000000000001"));
                    json!({"prompt_id": "00000000-0000-4000-8000-000000000001"}).to_string()
                }
                2 => {
                    assert!(
                        request.starts_with("GET /history/00000000-0000-4000-8000-000000000001")
                    );
                    json!({"00000000-0000-4000-8000-000000000001": {"status": {"completed": false}}}).to_string()
                }
                _ => {
                    assert!(request.starts_with("GET /view?"));
                    "12345".into()
                }
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
    });
    let client = ComfyClient::new(&stored.profile).unwrap();
    assert!(client.doctor(&stored).unwrap().workflow_valid);
    client
        .submit(
            &stored.profile,
            "new prompt",
            "00000000-0000-4000-8000-000000000001",
        )
        .unwrap();
    assert_eq!(
        parse_history(
            &client
                .history("00000000-0000-4000-8000-000000000001")
                .unwrap(),
            "00000000-0000-4000-8000-000000000001",
            &profile
        )
        .unwrap(),
        HistoryState::Pending
    );
    assert!(matches!(
        client.download("file.png", "", "output", 4),
        Err(ComfyError::InvalidOutput(_))
    ));
    worker.join().unwrap();
}

#[test]
fn h3_reference_upload_and_mp4_in_images_field_use_explicit_mapping() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (dir, mut profile) = fixture(&endpoint);
    profile.media_kind = MediaKind::Video;
    profile.reference_input = Some(NodeInput {
        node: "3".into(),
        input: "image".into(),
    });
    fs::write(
        &profile.workflow,
        json!({
            "1":{"class_type":"Text", "inputs":{"text":"old"}},
            "2":{"class_type":"SaveVideo", "inputs":{"video":["1",0]}},
            "3":{"class_type":"LoadImage", "inputs":{"image":"old.png"}}
        })
        .to_string(),
    )
    .unwrap();
    let config = dir.path().join("profile.json");
    fs::write(&config, serde_json::to_vec(&profile).unwrap()).unwrap();
    let stored = configure(&dir.path().join("profiles"), "h3", &config).unwrap();
    let worker = thread::spawn(move || {
        for index in 0..3 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut chunk = [0; 4096];
            loop {
                let n = stream.read(&mut chunk).unwrap();
                bytes.extend_from_slice(&chunk[..n]);
                if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&bytes[..end]);
                    let length = header
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length: ")
                                .and_then(|s| s.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if bytes.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let body = match index {
                0 => json!({
                    "Text":{"input":{"required":{"text":[]}}},
                    "SaveVideo":{"input":{"required":{"video":[]}},"output_node":true},
                    "LoadImage":{"input":{"required":{"image":[]}}}
                })
                .to_string(),
                1 => {
                    assert!(bytes.starts_with(b"POST /upload/image"));
                    assert!(bytes.windows(4).any(|w| w == b"test"));
                    json!({"name":"forge-reference.png","type":"input"}).to_string()
                }
                _ => {
                    assert!(bytes.starts_with(b"POST /prompt"));
                    let end = bytes.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
                    let posted: serde_json::Value =
                        serde_json::from_slice(&bytes[end + 4..]).unwrap();
                    assert_eq!(posted["prompt"]["1"]["inputs"]["text"], "robot idle");
                    assert_eq!(
                        posted["prompt"]["3"]["inputs"]["image"],
                        "forge-reference.png"
                    );
                    json!({"prompt_id":"00000000-0000-4000-8000-000000000001"}).to_string()
                }
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
    });
    let client = ComfyClient::new(&stored.profile).unwrap();
    assert!(client.doctor(&stored).unwrap().workflow_valid);
    let filename = client
        .upload_image("reference.png", b"test".to_vec())
        .unwrap();
    client
        .submit_with_reference(
            &stored.profile,
            "robot idle",
            "00000000-0000-4000-8000-000000000001",
            Some(&filename),
        )
        .unwrap();
    let history = json!({"00000000-0000-4000-8000-000000000001": {
        "status":{"completed":true},
        "outputs":{"2":{"images":[{"filename":"h3.mp4","subfolder":"video","type":"output"}]}}
    }});
    assert!(
        matches!(parse_history(&history, "00000000-0000-4000-8000-000000000001", &profile).unwrap(), HistoryState::Succeeded(output) if output.filename == "h3.mp4")
    );
    worker.join().unwrap();
}

#[test]
fn cancellation_deletes_only_exact_pending_forge_prompt() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (_dir, profile) = fixture(&endpoint);
    let worker = thread::spawn(move || {
        for index in 0..3 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut chunk = [0; 4096];
            loop {
                let n = stream.read(&mut chunk).unwrap();
                bytes.extend_from_slice(&chunk[..n]);
                if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&bytes[..end]);
                    let length = header
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length: ")
                                .and_then(|s| s.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if bytes.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let body = match index {
                0 => json!({"queue_running":[],"queue_pending":[[1,"our-id",{}, {"client_id":"forge"}], [2,"other-id",{}, {"client_id":"other"}]]}).to_string(),
                1 => {
                    assert!(bytes.starts_with(b"POST /queue"));
                    let end = bytes.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
                    let posted: serde_json::Value = serde_json::from_slice(&bytes[end+4..]).unwrap();
                    assert_eq!(posted, json!({"delete":["our-id"]}));
                    "{}".into()
                },
                _ => json!({"queue_running":[],"queue_pending":[[2,"other-id",{}, {"client_id":"other"}]]}).to_string(),
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
    });
    assert!(ComfyClient::new(&profile)
        .unwrap()
        .cancel_pending("our-id")
        .unwrap());
    worker.join().unwrap();
}

#[test]
fn cancellation_never_interrupts_a_running_prompt() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (_dir, profile) = fixture(&endpoint);
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut bytes = [0; 1024];
        let n = stream.read(&mut bytes).unwrap();
        assert!(bytes[..n].starts_with(b"GET /queue"));
        let body =
            json!({"queue_running":[[1,"our-id",{}, {"client_id":"forge"}]],"queue_pending":[]})
                .to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).unwrap();
    });
    assert!(matches!(
        ComfyClient::new(&profile).unwrap().cancel_pending("our-id"),
        Err(ComfyError::PromptRunning)
    ));
    worker.join().unwrap();
}
