//! Derived previews of validated flat Packs. PNGs and game resources stay immutable.
use crate::{
    content_digest::directory_inventory,
    video::{resolve_binary, FfmpegSearch},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub type PreviewResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Animation {
    pub name: String,
    pub frames: Vec<usize>,
    pub durations_ms: Vec<f64>,
    pub loop_animation: bool,
}
#[derive(Debug)]
pub struct AnimationSource {
    pub frames: Vec<PathBuf>,
    pub animations: Vec<Animation>,
}

/// None means an audio/world/layered/static Pack, whose existing viewer still applies.
pub fn read(pack: &Path) -> PreviewResult<Option<AnimationSource>> {
    let summary = forge_pack::inspect_pack(pack)?;
    if !matches!(summary.asset_type.as_str(), "animation" | "character") {
        return Ok(None);
    }
    let imported = forge_pack::import_pack(pack)?;
    if !matches!(
        imported.forgepack["schemaVersion"].as_str(),
        Some("1.0.0" | "2.0.0")
    ) || matches!(
        imported.forgepack["assetType"].as_str(),
        Some("icon_set" | "prop_set")
    ) {
        return Ok(None);
    }
    // Atlas indices define the animation, not filesystem iteration order.
    let entries = imported.atlas["frames"]
        .as_array()
        .ok_or_else(|| invalid("missing atlas frames"))?;
    let mut frames = vec![PathBuf::new(); entries.len()];
    for entry in entries {
        let i = entry["index"]
            .as_u64()
            .ok_or_else(|| invalid("invalid atlas index"))? as usize;
        let name = entry["name"]
            .as_str()
            .ok_or_else(|| invalid("missing atlas frame name"))?;
        let source = imported
            .frame_paths
            .iter()
            .find(|p| p.file_name().and_then(|v| v.to_str()) == Some(name))
            .ok_or_else(|| invalid("atlas frame is not a validated PNG"))?;
        if i >= frames.len() || !frames[i].as_os_str().is_empty() {
            return Err(invalid("invalid or duplicate atlas index").into());
        }
        frames[i] = source.clone();
    }
    if frames.is_empty() || frames.iter().any(|p| p.as_os_str().is_empty()) {
        return Err(invalid("incomplete atlas").into());
    }
    let mut animations = vec![];
    for value in imported.manifest["animations"]
        .as_array()
        .ok_or_else(|| invalid("missing animations"))?
    {
        let indices: Vec<usize> = serde_json::from_value(value["frames"].clone())?;
        if indices.is_empty() || indices.iter().any(|i| *i >= frames.len()) {
            return Err(invalid("invalid animation frame indices").into());
        }
        let fps = value["fps"]
            .as_f64()
            .filter(|v| v.is_finite() && *v > 0.0)
            .ok_or_else(|| invalid("invalid animation fps"))?;
        let durations_ms: Vec<f64> = if value["frameDurationsMs"].is_null() {
            vec![1000.0 / fps; indices.len()]
        } else {
            serde_json::from_value(value["frameDurationsMs"].clone())?
        };
        if durations_ms.len() != indices.len()
            || durations_ms.iter().any(|d| !d.is_finite() || *d <= 0.0)
        {
            return Err(invalid("invalid animation durations").into());
        }
        animations.push(Animation {
            name: value["name"]
                .as_str()
                .ok_or_else(|| invalid("missing animation name"))?
                .into(),
            frames: indices,
            durations_ms,
            loop_animation: value["loop"].as_bool().unwrap_or(false),
        });
    }
    if animations.is_empty() {
        return Err(invalid("empty animation list").into());
    }
    Ok(Some(AnimationSource { frames, animations }))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Background {
    Dark,
    Light,
    Checkerboard,
}
impl Background {
    fn rgb(self, x: u32, y: u32) -> [u8; 3] {
        match self {
            Self::Dark => [32, 36, 40],
            Self::Light => [240, 240, 240],
            Self::Checkerboard => {
                if (x / 16 + y / 16).is_multiple_of(2) {
                    [180; 3]
                } else {
                    [220; 3]
                }
            }
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoReport {
    pub output: PathBuf,
    pub source_sha256: String,
    pub cache_key: String,
    pub cache_hit: bool,
    pub video_sha256: String,
    pub animation: String,
    pub background: Background,
    pub encoder: String,
    pub ffmpeg_sha256: String,
    pub native_duration_ms: f64,
    pub encoded_duration_ms: f64,
    pub fps: u32,
    pub encoded_frames: usize,
    pub width: u32,
    pub height: u32,
    pub notes: Vec<String>,
}

// Resolve even a not-yet-created destination, including existing ancestor links.
fn destination(path: &Path) -> io::Result<PathBuf> {
    if path.exists() {
        return path.canonicalize();
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let parent = absolute
        .parent()
        .ok_or_else(|| invalid("invalid destination"))?;
    Ok(destination(parent)?.join(
        absolute
            .file_name()
            .ok_or_else(|| invalid("invalid destination"))?,
    ))
}
fn outside_pack(pack: &Path, path: &Path) -> io::Result<()> {
    if destination(path)?.starts_with(pack.canonicalize()?) {
        return Err(invalid(
            "preview destinations must be outside the source Pack",
        ));
    }
    Ok(())
}
fn publish(source: &Path, output: &Path) -> PreviewResult<()> {
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    io::copy(&mut fs::File::open(source)?, &mut temp)?;
    temp.as_file().sync_all()?;
    temp.persist_noclobber(output)?;
    Ok(())
}

pub fn export_mp4(
    pack: &Path,
    output: &Path,
    animation_name: Option<&str>,
    background: Background,
    cache: Option<&Path>,
    search: &FfmpegSearch,
) -> PreviewResult<VideoReport> {
    if fs::symlink_metadata(output).is_ok() {
        return Err(invalid("output already exists; choose a new MP4 path").into());
    }
    if output.extension().and_then(|v| v.to_str()) != Some("mp4") {
        return Err(invalid("output must end in .mp4").into());
    }
    outside_pack(pack, output)?;
    if let Some(cache) = cache {
        outside_pack(pack, cache)?;
    }
    let inventory = directory_inventory(pack)?;
    let source = read(pack)?.ok_or_else(|| invalid("MP4 preview supports flat animation/character Packs; use Godot preview for layered Packs"))?;
    let animation = match animation_name {
        Some(name) => source
            .animations
            .iter()
            .find(|a| a.name == name)
            .ok_or_else(|| invalid("animation not found"))?,
        None => &source.animations[0],
    };
    let total: f64 = animation.durations_ms.iter().sum();
    if !(1.0..=300_000.0).contains(&total) {
        return Err(invalid("MP4 preview duration must be between 1 ms and 5 minutes").into());
    }
    let ffmpeg = resolve_binary(
        "ffmpeg",
        search.configured_ffmpeg_path.as_deref(),
        search.bundled_resource_path.as_deref(),
    )?;
    let ffmpeg_sha256 = digest(&fs::read(&ffmpeg)?);
    let encoders = Command::new(&ffmpeg)
        .args(["-hide_banner", "-encoders"])
        .output()?;
    if !encoders.status.success() {
        return Err(invalid("could not inspect FFmpeg encoders").into());
    }
    let listing = String::from_utf8_lossy(&encoders.stdout);
    let encoder = ["libx264", "h264_videotoolbox", "h264_mf"]
        .into_iter()
        .find(|e| {
            listing
                .lines()
                .any(|line| line.split_whitespace().nth(1) == Some(*e))
        })
        .ok_or_else(|| {
            invalid("FFmpeg has no supported H.264 encoder (libx264, h264_videotoolbox or h264_mf)")
        })?;
    let key = digest(&serde_json::to_vec(
        &serde_json::json!({"profile":"forge-mp4-v1", "source":inventory.sha256, "animation":animation, "background":background, "encoder":encoder, "ffmpeg":ffmpeg_sha256, "fps":60}),
    )?);
    if let Some(cache) = cache {
        let entry = cache.join(&key);
        if entry.exists() {
            // Never trust a cache's filename without checking its report and bytes.
            let cached_inventory = directory_inventory(&entry)?;
            let mut report: VideoReport =
                serde_json::from_slice(&fs::read(entry.join("report.json"))?)?;
            if report.cache_key != key
                || report.source_sha256 != inventory.sha256
                || !cached_inventory
                    .files
                    .iter()
                    .any(|f| f.path == "preview.mp4" && f.sha256 == report.video_sha256)
            {
                return Err(
                    invalid("preview cache is corrupt; remove this cache entry and retry").into(),
                );
            }
            if directory_inventory(pack)? != inventory {
                return Err(invalid("Pack changed while preparing preview").into());
            }
            publish(&entry.join("preview.mp4"), output)?;
            report.output = output.into();
            report.cache_hit = true;
            return Ok(report);
        }
    }
    let temp = tempfile::tempdir()?;
    let mut images = Vec::new();
    let mut dimensions = None;
    for index in &animation.frames {
        let image = image::open(&source.frames[*index])?.to_rgba8();
        let size = image.dimensions();
        if size.0 == 0
            || size.1 == 0
            || size.0 > 4096
            || size.1 > 4096
            || dimensions.is_some_and(|d| d != size)
        {
            return Err(
                invalid("preview frames must share a canvas of at most 4096 × 4096").into(),
            );
        }
        dimensions = Some(size);
        // Odd canvases are padded on the right/bottom; never resize the source.
        let (width, height) = ((size.0 + 1) & !1, (size.1 + 1) & !1);
        let mut rgb = Vec::with_capacity((width * height * 3) as usize);
        for y in 0..height {
            for x in 0..width {
                let bg = background.rgb(x, y);
                let rgba = if x < size.0 && y < size.1 {
                    image.get_pixel(x, y).0
                } else {
                    [0; 4]
                };
                for c in 0..3 {
                    rgb.push(
                        ((u32::from(rgba[c]) * u32::from(rgba[3])
                            + u32::from(bg[c]) * (255 - u32::from(rgba[3]))
                            + 127)
                            / 255) as u8,
                    );
                }
            }
        }
        // Spool each composited frame; memory use does not grow with animation length.
        let path = temp.path().join(format!("{}.rgb", images.len()));
        fs::write(&path, rgb)?;
        images.push(path);
    }
    let (w, h) = dimensions.unwrap();
    let (width, height) = ((w + 1) & !1, (h + 1) & !1);
    let count = ((total * 60.0 / 1000.0).round() as usize).max(1);
    let video = temp.path().join("preview.mp4");
    let stderr = fs::File::create(temp.path().join("ffmpeg.log"))?;
    let mut command = Command::new(&ffmpeg);
    command.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-nostdin",
        "-f",
        "rawvideo",
        "-pixel_format",
        "rgb24",
        "-video_size",
        &format!("{width}x{height}"),
        "-framerate",
        "60",
        "-i",
        "pipe:0",
        "-an",
        "-c:v",
        encoder,
    ]);
    if encoder == "libx264" {
        command.args(["-crf", "18", "-preset", "medium"]);
    } else {
        command.args(["-b:v", "12M"]);
    }
    command
        .args([
            "-pix_fmt",
            "yuv420p",
            "-movflags",
            "+faststart",
            "-f",
            "mp4",
        ])
        .arg(&video)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(stderr);
    let mut child = command.spawn()?;
    let write_result = (|| -> io::Result<()> {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| invalid("missing encoder input"))?;
        let mut index = 0;
        let mut end = animation.durations_ms[0];
        let mut rgb = fs::read(&images[0])?;
        for tick in 0..count {
            let time = tick as f64 * 1000.0 / 60.0;
            while time >= end && index + 1 < images.len() {
                index += 1;
                end += animation.durations_ms[index];
                rgb = fs::read(&images[index])?;
            }
            stdin.write_all(&rgb)?;
        }
        Ok(())
    })();
    let status = child.wait()?;
    if !status.success() {
        return Err(invalid(&format!(
            "H.264 preview encoding failed ({encoder}): {}",
            fs::read_to_string(temp.path().join("ffmpeg.log"))?
        ))
        .into());
    }
    write_result?;
    if directory_inventory(pack)? != inventory {
        return Err(invalid("Pack changed while encoding preview").into());
    }
    let report = VideoReport { output: output.into(), source_sha256: inventory.sha256, cache_key: key.clone(), cache_hit: false, video_sha256: digest(&fs::read(&video)?), animation: animation.name.clone(), background, encoder: encoder.into(), ffmpeg_sha256, native_duration_ms: total, encoded_duration_ms: count as f64 * 1000.0 / 60.0, fps: 60, encoded_frames: count, width, height, notes: vec!["Composited preview: background is baked in; PNGs retain original transparency.".into(), "60 fps timing is quantized; short source frames may be skipped. Playback contains one animation cycle.".into(), "Normal alpha composition; use Godot to judge engine blend modes and runtime rendering.".into()] };
    if let Some(cache) = cache {
        fs::create_dir_all(cache)?;
        let staging = tempfile::Builder::new()
            .prefix(".preview-")
            .tempdir_in(cache)?;
        fs::copy(&video, staging.path().join("preview.mp4"))?;
        fs::write(
            staging.path().join("report.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
        let target = cache.join(key);
        // A concurrent producer may already have completed the same cache key.
        if let Err(error) = fs::rename(staging.path(), &target) {
            if !target.is_dir() {
                return Err(error.into());
            }
        }
    }
    publish(&video, output)?;
    Ok(report)
}

pub const PLAYER_SCRIPT: &str = include_str!("animation_preview_player.js");
pub fn script_hash() -> String {
    html_script_hash(PLAYER_SCRIPT)
}

fn html_script_hash(script: &str) -> String {
    use base64::Engine;
    // The HTML tokenizer normalizes CRLF and lone CR before the CSP hash check.
    // Also cover source archives and editors that don't honor .gitattributes.
    let normalized = script.replace("\r\n", "\n").replace('\r', "\n");
    base64::engine::general_purpose::STANDARD.encode(Sha256::digest(normalized.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_csp_matches_html_line_ending_normalization() {
        let lf = PLAYER_SCRIPT.replace("\r\n", "\n");
        assert_eq!(
            html_script_hash(&lf),
            html_script_hash(&lf.replace('\n', "\r\n"))
        );
        assert_eq!(
            html_script_hash(&lf),
            html_script_hash(&lf.replace('\n', "\r"))
        );
    }
}
