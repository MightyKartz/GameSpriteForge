use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use forge_core::motion_video::{
    validate_motion_driver_lock, validate_video_candidate_lock, MotionDriverLockV1,
    VideoCandidateLockV1,
};
use forge_core::quality::source_cycle_sampling::{
    validate_source_cycle_sampling_report, SourceCycleSamplingReportV1,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplaySummary {
    candidate_video_sha256: String,
    frames: Vec<PathBuf>,
    stabilization_verdict: String,
    motion_verdict: String,
    #[serde(default)]
    frame_durations_ms: Vec<u64>,
    #[serde(default)]
    production_eligible: Option<bool>,
}

fn require_safe_animation_name(value: &str) -> Result<(), Box<dyn Error>> {
    if value.is_empty()
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return Err(format!("unsafe Godot animation name: {value}").into());
    }
    Ok(())
}

fn copy_frame(source: &Path, destination: &Path) -> Result<(), Box<dyn Error>> {
    fs::copy(source, destination)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let candidate_lock_path = arguments.next().map(PathBuf::from).ok_or(
        "usage: create_locked_godot_preview <candidate-lock> <replay-summary> <sampling-report> <output-directory>",
    )?;
    let replay_summary_path = arguments.next().map(PathBuf::from).ok_or(
        "usage: create_locked_godot_preview <candidate-lock> <replay-summary> <sampling-report> <output-directory>",
    )?;
    let timing_report = arguments.next().map(PathBuf::from).ok_or(
        "usage: create_locked_godot_preview <candidate-lock> <replay-summary> <sampling-report|-> <output-directory>",
    )?;
    let output_directory = arguments.next().map(PathBuf::from).ok_or(
        "usage: create_locked_godot_preview <candidate-lock> <replay-summary> <sampling-report> <output-directory>",
    )?;
    if arguments.next().is_some() {
        return Err("usage: create_locked_godot_preview <candidate-lock> <replay-summary> <sampling-report> <output-directory>".into());
    }
    if output_directory.exists() && output_directory.read_dir()?.next().is_some() {
        return Err(format!(
            "refusing to overwrite non-empty {}",
            output_directory.display()
        )
        .into());
    }

    let candidate: VideoCandidateLockV1 = serde_json::from_slice(&fs::read(&candidate_lock_path)?)?;
    validate_video_candidate_lock(&candidate)?;
    let driver: MotionDriverLockV1 =
        serde_json::from_slice(&fs::read(&candidate.motion_driver_lock_path)?)?;
    validate_motion_driver_lock(&driver)?;
    require_safe_animation_name(&driver.action)?;
    let replay: ReplaySummary = serde_json::from_slice(&fs::read(&replay_summary_path)?)?;
    if replay.candidate_video_sha256 != candidate.source_video_sha256 {
        return Err("preview inputs do not share one candidate and frame count".into());
    }
    let (durations, timing_source_profile) = if timing_report.as_os_str() == "-" {
        if replay.frame_durations_ms.len() != replay.frames.len()
            || replay.frame_durations_ms.contains(&0)
        {
            return Err("diagnostic replay has invalid frame durations".into());
        }
        (
            replay.frame_durations_ms.clone(),
            "diagnostic-replay-frame-durations".to_string(),
        )
    } else {
        let sampling: SourceCycleSamplingReportV1 =
            serde_json::from_slice(&fs::read(&timing_report)?)?;
        validate_source_cycle_sampling_report(&sampling)?;
        if replay.frames.len() != sampling.output_frame_count {
            return Err("preview inputs do not share one frame count".into());
        }
        let boundary_timestamp = sampling
            .source_timestamps_ms
            .last()
            .copied()
            .ok_or("sampling report has no boundary timestamp")?;
        let mut durations = sampling
            .output_timestamps_ms
            .windows(2)
            .map(|timestamps| timestamps[1] - timestamps[0])
            .collect::<Vec<_>>();
        let last_timestamp = sampling
            .output_timestamps_ms
            .last()
            .copied()
            .ok_or("sampling report has no output timestamp")?;
        durations.push(
            boundary_timestamp
                .checked_sub(last_timestamp)
                .filter(|duration| *duration > 0)
                .ok_or("sampling boundary does not follow the last output frame")?,
        );
        (durations, sampling.profile)
    };

    let frames_directory = output_directory.join("frames");
    fs::create_dir_all(&frames_directory)?;
    for (index, source) in replay.frames.iter().enumerate() {
        copy_frame(source, &frames_directory.join(format!("{index:02}.png")))?;
    }
    fs::create_dir_all(output_directory.join("qa-output"))?;
    let first = image::open(&replay.frames[0])?;
    let maximum_dimension = first.width().max(first.height()).max(1);
    let preview_scale = (520.0 / maximum_dimension as f32).min(1.0);
    let total_duration_ms = durations.iter().sum::<u64>();
    let auto_quit_seconds = total_duration_ms as f32 / 1000.0 * 2.0 + 0.25;
    let durations_literal = durations
        .iter()
        .map(|duration| format!("{duration}.0"))
        .collect::<Vec<_>>()
        .join(", ");
    let script = format!(
        r##"extends Node2D

const FRAME_COUNT := {frame_count}
const DURATIONS_MS := [{durations_literal}]
const ANIMATION := "{animation}"
const PREVIEW_SCALE := {preview_scale:.8}
const AUTO_QUIT_SECONDS := {auto_quit_seconds:.8}
var elapsed := 0.0
var auto_quit := false

func _ready() -> void:
	auto_quit = DisplayServer.get_name() == "headless" or "--auto-quit" in OS.get_cmdline_user_args()
	var frames := SpriteFrames.new()
	for name in frames.get_animation_names():
		frames.remove_animation(name)
	frames.add_animation(ANIMATION)
	frames.set_animation_speed(ANIMATION, 1000.0)
	frames.set_animation_loop(ANIMATION, true)
	for index in range(FRAME_COUNT):
		var texture := load("res://frames/%02d.png" % index) as Texture2D
		frames.add_frame(ANIMATION, texture, DURATIONS_MS[index])
	var sprite := AnimatedSprite2D.new()
	sprite.sprite_frames = frames
	sprite.animation = ANIMATION
	sprite.centered = true
	sprite.position = Vector2(320, 320)
	sprite.scale = Vector2(PREVIEW_SCALE, PREVIEW_SCALE)
	sprite.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
	add_child(sprite)
	sprite.play(ANIMATION)
	call_deferred("_capture")

func _process(delta: float) -> void:
	if not auto_quit:
		return
	elapsed += delta
	if elapsed >= AUTO_QUIT_SECONDS:
		get_tree().quit(0)

func _draw() -> void:
	draw_rect(Rect2(0, 0, 640, 640), Color("#202630"))

func _capture() -> void:
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	if image != null and not image.is_empty():
		image.save_png("res://qa-output/runtime.png")
"##,
        frame_count = replay.frames.len(),
        animation = driver.action,
    );
    fs::write(output_directory.join("preview.gd"), script)?;
    fs::write(
        output_directory.join("main.tscn"),
        "[gd_scene load_steps=2 format=3]\n[ext_resource type=\"Script\" path=\"res://preview.gd\" id=\"1\"]\n[node name=\"Preview\" type=\"Node2D\"]\nscript = ExtResource(\"1\")\n",
    )?;
    fs::write(
        output_directory.join("project.godot"),
        format!(
            "[application]\nconfig/name=\"Forge Locked Video {} Preview\"\nrun/main_scene=\"res://main.tscn\"\n[display]\nwindow/size/viewport_width=640\nwindow/size/viewport_height=640\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\nrenderer/rendering_method.mobile=\"gl_compatibility\"\n",
            driver.action
        ),
    )?;
    let manifest = serde_json::json!({
        "profile": "locked-video-godot-preview@1.0.0",
        "candidateVideoSha256": candidate.source_video_sha256,
        "action": driver.action,
        "direction": driver.direction,
        "frameCount": replay.frames.len(),
        "frameDurationsMs": durations,
        "timingSourceProfile": timing_source_profile,
        "playbackDurationMs": total_duration_ms,
        "previewScale": preview_scale,
        "stabilizationVerdict": replay.stabilization_verdict,
        "motionVerdict": replay.motion_verdict,
        "productionEligible": replay.production_eligible.unwrap_or(false),
        "productionPackWritten": false,
    });
    fs::write(
        output_directory.join("preview-manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    Ok(())
}
