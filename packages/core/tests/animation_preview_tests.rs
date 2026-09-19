use forge_core::{
    animation_preview::{self, Background},
    automation::{run_operation, stage_plan_job, AutomationOperation, PlanStore},
    content_digest::directory_inventory,
    job::{JobLifecycleState, JobStore},
    library::{self, delivery::VersionRef, intake},
};
use image::{Rgba, RgbaImage};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn pack(root: &Path) -> PathBuf {
    let paths: Vec<_> = (0..3)
        .map(|i| {
            let mut image = RgbaImage::new(65, 65);
            for y in 20..45 {
                for x in (10 + i * 10)..(20 + i * 10) {
                    image.put_pixel(x, y, Rgba([0, 180, 240, 180]));
                }
            }
            image.put_pixel(0, 0, Rgba([0, 0, 0, 1]));
            let path = root.join(format!("{i}.png"));
            image.save(&path).unwrap();
            path
        })
        .collect();
    let request = json!({"schemaVersion":"1","input":{"kind":"png_sequence","paths":paths},
        "metadata":{"name":"Preview QA","animation":"strike","fps":8,"frameDurationsMs":[80,240,110],"loop":false},
        "normalize":{"mode":"preserve_source","margin":0,"marginBottom":0,"alphaThreshold":0,
        "manualAnchor":{"x":32,"y":45,"lockedByUser":true}},"quality":{"requireGameReady":false}});
    let plans = PlanStore::new(root.join("plans")).unwrap();
    let prepared = plans
        .prepare(AutomationOperation::PrepareAsset(
            serde_json::from_value(request).unwrap(),
        ))
        .unwrap();
    let plan = plans.claim(&prepared.token).unwrap();
    let jobs = JobStore::new(root.join("jobs")).unwrap();
    let job = stage_plan_job(&jobs, &plan).unwrap();
    let completed = run_operation(&jobs, &job.job_id, &plan.operation).unwrap();
    assert_eq!(completed.lifecycle_state, JobLifecycleState::Succeeded);
    completed
        .artifacts
        .iter()
        .find(|a| a.kind == "gsfpack")
        .unwrap()
        .path
        .clone()
}
fn reorder(pack: &Path) {
    for name in ["forgepack.json", "assets/manifest.json"] {
        let path = pack.join(name);
        let mut data: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        data["animations"][0]["frames"] = json!([2, 0, 2]);
        fs::write(path, serde_json::to_vec_pretty(&data).unwrap()).unwrap();
    }
}

#[test]
fn png_review_uses_native_order_alpha_and_timing_without_mutating_pack_or_catalog() {
    let temp = tempfile::tempdir().unwrap();
    let pack = pack(temp.path());
    reorder(&pack);
    let before = directory_inventory(&pack).unwrap();
    let source = animation_preview::read(&pack).unwrap().unwrap();
    assert_eq!(source.animations[0].frames, vec![2, 0, 2]);
    assert_eq!(source.animations[0].durations_ms, vec![80.0, 240.0, 110.0]);
    let library = temp.path().join("library");
    library::initialize(&library, "Preview").unwrap();
    let mut scan = intake::scan(&pack).unwrap();
    scan.batch.items[0].name = "</script><script>alert(1)</script>".into();
    let registered = intake::register(&library, &scan.batch).unwrap();
    let references: Vec<_> = registered
        .iter()
        .map(|a| VersionRef {
            asset_id: a.asset_id.clone(),
            revision: a.revision.clone(),
        })
        .collect();
    let head = fs::read(library.join(".forge/catalog.json")).unwrap();
    let out = temp.path().join("review");
    let report = library::preview::create(&library, &references, &out).unwrap();
    assert!(report.issues.is_empty(), "{:?}", report.issues);
    assert_eq!(report.media_files, 3);
    let html = fs::read_to_string(out.join("index.html")).unwrap();
    assert!(html.contains("data-forge-animation="));
    assert!(html.contains("&lt;/script&gt;"));
    assert!(html.contains(&format!(
        "script-src 'sha256-{}'",
        animation_preview::script_hash()
    )));
    assert!(!html.contains("src=\"media/0/0.gif\""));
    for i in 0..3 {
        assert_eq!(
            fs::read(&source.frames[i]).unwrap(),
            fs::read(out.join(format!("media/0/{i}.png"))).unwrap()
        );
    }
    assert_eq!(before, directory_inventory(&pack).unwrap());
    assert_eq!(head, fs::read(library.join(".forge/catalog.json")).unwrap());
}

#[test]
fn gif_has_independent_frames_explicit_alpha_and_per_frame_delays() {
    let temp = tempfile::tempdir().unwrap();
    let pack = pack(temp.path());
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = options
        .read_info(fs::File::open(pack.join("previews/preview.gif")).unwrap())
        .unwrap();
    let mut delays = vec![];
    while let Some(frame) = decoder.read_next_frame().unwrap() {
        assert_eq!(frame.dispose, gif::DisposalMethod::Background);
        assert_eq!(
            frame.buffer[3], 0,
            "near-invisible black must not become opaque"
        );
        delays.push(frame.delay);
    }
    assert_eq!(delays, vec![8, 24, 11]);
    let source = animation_preview::read(&pack).unwrap().unwrap();
    for durations in [vec![1], vec![80, 0, 110], vec![80, 700000, 110]] {
        let out = temp.path().join("invalid.gif");
        assert!(forge_core::export::build_preview_gif_with_timing(
            &source.frames,
            &out,
            Default::default(),
            Some(&durations)
        )
        .is_err());
        assert!(!out.exists());
    }
}

#[test]
fn preview_rejects_writes_inside_pack_and_existing_outputs_before_using_tools() {
    let temp = tempfile::tempdir().unwrap();
    let pack = pack(temp.path());
    let before = directory_inventory(&pack).unwrap();
    assert!(animation_preview::export_mp4(
        &pack,
        &pack.join("previews/new.mp4"),
        None,
        Background::Dark,
        None,
        &Default::default()
    )
    .is_err());
    assert!(animation_preview::export_mp4(
        &pack,
        &temp.path().join("out.mp4"),
        None,
        Background::Dark,
        Some(&pack.join("cache")),
        &Default::default()
    )
    .is_err());
    let out = temp.path().join("out.mp4");
    fs::write(&out, b"keep").unwrap();
    assert!(animation_preview::export_mp4(
        &pack,
        &out,
        None,
        Background::Dark,
        None,
        &Default::default()
    )
    .is_err());
    assert_eq!(fs::read(out).unwrap(), b"keep");
    assert_eq!(directory_inventory(&pack).unwrap(), before);
}

#[test]
#[ignore = "requires native FFmpeg H.264 encoder and ffprobe; run explicitly in media CI"]
fn native_mp4_preserves_duration_canvas_and_cache_integrity() {
    let temp = tempfile::tempdir().unwrap();
    let pack = pack(temp.path());
    reorder(&pack);
    let before = directory_inventory(&pack).unwrap();
    let cache = temp.path().join("cache");
    let export = |name: &str, bg| {
        animation_preview::export_mp4(
            &pack,
            &temp.path().join(name),
            Some("strike"),
            bg,
            Some(&cache),
            &Default::default(),
        )
    };
    let first = export("one.mp4", Background::Dark).unwrap();
    // CI retains the actual encoded bytes and decoded frames even on assertion failure.
    let evidence = std::env::var_os("FORGE_PREVIEW_TEST_EVIDENCE").map(PathBuf::from);
    if let Some(path) = &evidence {
        fs::create_dir_all(path).unwrap();
        fs::copy(&first.output, path.join("preview.mp4")).unwrap();
        fs::write(
            path.join("report.json"),
            serde_json::to_vec_pretty(&first).unwrap(),
        )
        .unwrap();
    }
    assert!(!first.cache_hit);
    assert_eq!((first.width, first.height), (66, 66));
    assert!((first.encoded_duration_ms - first.native_duration_ms).abs() <= 1000.0 / 120.0);
    let second = export("two.mp4", Background::Dark).unwrap();
    assert!(second.cache_hit);
    assert_eq!(first.video_sha256, second.video_sha256);
    let light = export("light.mp4", Background::Light).unwrap();
    assert_ne!(light.cache_key, first.cache_key);
    let probe = forge_core::video::probe_video(&forge_core::video::ProbeVideoParams {
        input_path: first.output.clone(),
        configured_ffprobe_path: None,
        bundled_resource_path: None,
    })
    .unwrap();
    assert_eq!((probe.width, probe.height), (66, 66));
    assert_eq!(probe.codec, "h264");
    assert!((probe.duration_seconds * 1000.0 - first.encoded_duration_ms).abs() < 2.0);
    let ffmpeg = forge_core::video::resolve_binary("ffmpeg", None, None).unwrap();
    for (time, bright_x, dark_x) in [("0", 35, 15), ("0.1", 15, 35)] {
        let decoded = std::process::Command::new(&ffmpeg)
            .args(["-v", "error", "-ss", time, "-i"])
            .arg(&first.output)
            .args([
                "-frames:v",
                "1",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgb24",
                "pipe:1",
            ])
            .output()
            .unwrap();
        assert!(decoded.status.success());
        assert_eq!(decoded.stdout.len(), 66 * 66 * 3);
        if let Some(path) = &evidence {
            image::RgbImage::from_raw(66, 66, decoded.stdout.clone())
                .unwrap()
                .save(path.join(format!("frame-{time}.png")))
                .unwrap();
        }
        let channel = |x: usize| decoded.stdout[(30 * 66 + x) * 3 + 2];
        assert!(
            channel(bright_x) > 130,
            "requested frame must be visible at {time}: {}",
            channel(bright_x)
        );
        assert!(
            channel(dark_x) < 65,
            "prior frame must not accumulate at {time}: {}",
            channel(dark_x)
        );
        assert!(
            decoded.stdout[0] > 20,
            "near-transparent black must composite into background"
        );
    }
    fs::write(cache.join(&first.cache_key).join("preview.mp4"), b"corrupt").unwrap();
    assert!(export("corrupt.mp4", Background::Dark).is_err());
    assert!(!temp.path().join("corrupt.mp4").exists());
    assert_eq!(before, directory_inventory(&pack).unwrap());

    // Timing is part of content identity: an edited request must not reuse an
    // earlier video's bytes even when the PNG sequence is unchanged.
    for name in ["forgepack.json", "assets/manifest.json"] {
        let path = pack.join(name);
        let mut data: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        data["animations"][0]["frameDurationsMs"] = json!([120, 240, 110]);
        fs::write(path, serde_json::to_vec_pretty(&data).unwrap()).unwrap();
    }
    let path = pack.join("assets/godot_import.json");
    let mut data: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    data["spriteFrames"]["animations"][0]["frameDurationsMs"] = json!([120, 240, 110]);
    fs::write(path, serde_json::to_vec_pretty(&data).unwrap()).unwrap();
    let retimed = export("retimed.mp4", Background::Dark).unwrap();
    assert!(!retimed.cache_hit);
    assert_ne!(retimed.cache_key, first.cache_key);
    assert_eq!(retimed.native_duration_ms, 470.0);
}
