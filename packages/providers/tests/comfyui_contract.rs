use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use forge_providers::comfyui::{
    configure, load, parse_history, ComfyClient, ComfyError, HistoryState, MediaKind, NodeInput,
    OutputField, WorkflowProfile,
};
use serde_json::json;
use tempfile::tempdir;

fn fixture(endpoint: &str) -> (tempfile::TempDir, WorkflowProfile) {
    let dir = tempdir().unwrap();
    let workflow = dir.path().join("workflow.json");
    fs::write(&workflow, json!({"1": {"class_type": "CLIPTextEncode", "inputs": {"text": "old"}}, "2": {"class_type": "SaveImage", "inputs": {"images": ["1", 0]}}}).to_string()).unwrap();
    let profile = WorkflowProfile {
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
