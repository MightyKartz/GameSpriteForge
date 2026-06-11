use std::fs;
use std::path::Path;
use std::process::Command;

use forge_core::video::extract::{extract_frames, ExtractFramesParams};
use forge_core::video::ffmpeg::{resolve_binary, FFMPEG_MISSING_CODE, FFMPEG_MISSING_MESSAGE};
use forge_core::video::probe::{probe_video, ProbeVideoParams};

#[test]
fn missing_configured_binary_uses_exact_error() {
    let temp = tempfile::tempdir().expect("tempdir");
    let error = resolve_binary("definitely-missing-forge-ffmpeg", None, Some(temp.path()))
        .expect_err("missing ffmpeg should return typed error");

    assert_eq!(error.code, FFMPEG_MISSING_CODE);
    assert_eq!(error.message, FFMPEG_MISSING_MESSAGE);
}

#[test]
fn configured_binary_wins_before_bundled_resource_path() {
    let temp = tempfile::tempdir().expect("tempdir");
    let configured = temp.path().join("configured-ffmpeg");
    let bundled_dir = temp.path().join("bundle");
    let bundled = bundled_dir.join("ffmpeg");
    fs::create_dir_all(&bundled_dir).expect("bundle dir");
    fs::write(&configured, b"").expect("configured binary");
    fs::write(&bundled, b"").expect("bundled binary");

    let resolved = resolve_binary("ffmpeg", Some(&configured), Some(&bundled_dir))
        .expect("configured binary should resolve");

    assert_eq!(resolved, configured);
}

#[test]
fn bundled_resource_path_wins_before_system_path() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundled_dir = temp.path().join("bundle");
    let bundled = bundled_dir.join("ffmpeg");
    fs::create_dir_all(&bundled_dir).expect("bundle dir");
    fs::write(&bundled, b"").expect("bundled binary");

    let resolved =
        resolve_binary("ffmpeg", None, Some(&bundled_dir)).expect("bundled binary should resolve");

    assert_eq!(resolved, bundled);
}

#[test]
fn probe_and_extract_green_box_fixture_when_ffmpeg_is_available() {
    let Some(ffmpeg) = forge_core::video::ffmpeg::find_in_path("ffmpeg") else {
        eprintln!("skipping integration test because ffmpeg is unavailable");
        return;
    };
    let Some(ffprobe) = forge_core::video::ffmpeg::find_in_path("ffprobe") else {
        eprintln!("skipping integration test because ffprobe is unavailable");
        return;
    };

    let temp = tempfile::tempdir().expect("tempdir");
    let input_path = temp.path().join("green-box-character.mp4");
    create_fixture(&ffmpeg, &input_path);

    let probe = probe_video(&ProbeVideoParams {
        input_path: input_path.clone(),
        configured_ffprobe_path: Some(ffprobe.into()),
        bundled_resource_path: None,
    })
    .expect("probe fixture");

    assert_eq!(probe.width, 256);
    assert_eq!(probe.height, 256);
    assert!((probe.fps - 24.0).abs() < 0.01, "fps was {}", probe.fps);
    assert!(probe.duration_seconds > 0.9);
    assert!(probe.frame_count_estimate >= 20);
    assert!(!probe.codec.is_empty());
    assert!(!probe.pixel_format.is_empty());

    let output_dir = temp.path().join("frames");
    let result = extract_frames(&ExtractFramesParams {
        input_path,
        start_time_seconds: 0.0,
        end_time_seconds: 1.0,
        keep_every_n_frames: 6,
        output_directory: output_dir,
        configured_ffmpeg_path: Some(ffmpeg.into()),
        bundled_resource_path: None,
    })
    .expect("extract frames");

    assert!(!result.frames.is_empty());
    assert_eq!(
        result.frames[0].file_name().and_then(|name| name.to_str()),
        Some("frame_00001.png")
    );
    assert!(result.raw_directory.join("frame_00001.png").is_file());
}

fn create_fixture(ffmpeg: &str, input_path: &Path) {
    let output = Command::new(ffmpeg)
        .args(["-y", "-f", "lavfi", "-i"])
        .arg("color=c=0x00ff00:s=256x256:d=1:r=24")
        .args(["-vf", "drawbox=x=96:y=88:w=64:h=96:color=white:t=fill"])
        .arg(input_path)
        .output()
        .expect("run ffmpeg fixture command");

    assert!(
        output.status.success(),
        "fixture ffmpeg failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
