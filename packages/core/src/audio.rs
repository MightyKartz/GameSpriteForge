//! Optional local audio intake. No generation, network retrieval or model runtime.
use crate::automation::SourceLock;
use crate::video::{resolve_ffmpeg_paths, FfmpegSearch};
use forge_pack::audio::{
    hash_audio_file, inspect_pcm_wav, valid_audio_id, AudioItem, AudioManifest, AudioProcessing,
    AudioQualityReport, AudioSource, MAX_AUDIO_BYTES, MAX_AUDIO_SECONDS,
};
pub use forge_pack::audio::{AudioOrigin, AudioRole};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn schema_version() -> String {
    "1".into()
}
fn sample_rate() -> u32 {
    48000
}
fn channels() -> u16 {
    2
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrepareAudioRequest {
    #[serde(default = "schema_version")]
    pub schema_version: String,
    pub id: String,
    pub name: String,
    pub items: Vec<PrepareAudioItem>,
    #[serde(default = "sample_rate")]
    pub sample_rate: u32,
    #[serde(default = "channels")]
    pub channels: u16,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_locks: Vec<SourceLock>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrepareAudioItem {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub path: PathBuf,
    pub role: AudioRole,
    #[serde(default, rename = "loop")]
    pub loop_audio: bool,
    #[serde(default)]
    pub trim_start_seconds: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trim_end_seconds: Option<f64>,
    #[serde(default)]
    pub fade_in_ms: u32,
    #[serde(default)]
    pub fade_out_ms: u32,
    #[serde(default)]
    pub gain_db: f64,
    #[serde(default)]
    pub crossfade_ms: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<AudioOrigin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedAudio {
    pub pack_path: PathBuf,
    pub manifest_path: PathBuf,
    pub quality_report_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSourceInspection {
    pub path: PathBuf,
    pub sha256: String,
    pub duration_seconds: f64,
    pub sample_rate: u32,
    pub channels: u16,
    pub codec_name: String,
}

fn local_wav(path: &Path) -> Result<(), String> {
    let text = path.to_string_lossy();
    if text.contains("://")
        || text.starts_with("file:")
        || text.starts_with("pipe:")
        || text.contains('\\')
    {
        return Err("audio source must be a local regular WAV path without protocols".into());
    }
    let metadata = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || !(44..=MAX_AUDIO_BYTES).contains(&metadata.len())
    {
        return Err(format!(
            "audio source must be a regular WAV of 44 bytes through 512 MiB: {}",
            path.display()
        ));
    }
    let mut header = [0; 12];
    File::open(path)
        .and_then(|mut f| f.read_exact(&mut header))
        .map_err(|e| e.to_string())?;
    if &header[..4] != b"RIFF"
        || &header[8..] != b"WAVE"
        || u32::from_le_bytes(header[4..8].try_into().unwrap()) as u64 + 8 != metadata.len()
    {
        return Err("audio intake currently requires RIFF/WAVE content with a valid length; convert other formats to WAV externally".into());
    }
    forge_pack::audio::validate_wav_container(path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn validate_request(request: &PrepareAudioRequest) -> Result<(), String> {
    if request.schema_version != "1"
        || !valid_audio_id(&request.id)
        || request.name.trim().is_empty()
        || request.name.len() > 256
    {
        return Err("prepare-audio requires schemaVersion 1, an engine-safe id and a nonempty name (at most 256 bytes)".into());
    }
    if !(8000..=96000).contains(&request.sample_rate) || !(1..=2).contains(&request.channels) {
        return Err(
            "audio output sampleRate must be 8000..=96000 and channels must be 1 or 2".into(),
        );
    }
    if request.items.is_empty() || request.items.len() > 64 {
        return Err("prepare-audio requires 1..=64 items".into());
    }
    let mut ids = HashSet::new();
    for item in &request.items {
        if !valid_audio_id(&item.id)
            || !ids.insert(item.id.to_ascii_lowercase())
            || item.name.len() > 256
        {
            return Err("audio item IDs must be engine-safe and unique including case; names must be at most 256 bytes".into());
        }
        if !item.trim_start_seconds.is_finite()
            || !(0.0..MAX_AUDIO_SECONDS).contains(&item.trim_start_seconds)
            || item.trim_end_seconds.is_some_and(|v| {
                !v.is_finite() || v <= item.trim_start_seconds || v > MAX_AUDIO_SECONDS
            })
            || !item.gain_db.is_finite()
            || !(-60.0..=24.0).contains(&item.gain_db)
            || item.fade_in_ms > 3600000
            || item.fade_out_ms > 3600000
            || item.crossfade_ms > 30000
            || (item.crossfade_ms > 0 && !item.loop_audio)
        {
            return Err(format!("invalid finite trim, fade, gain or crossfade controls for {}; crossfade requires loop=true", item.id));
        }
        if let Some(origin) = &item.origin {
            for (value, max) in [
                (&origin.tool, 1024),
                (&origin.model, 1024),
                (&origin.prompt, 16384),
                (&origin.seed, 1024),
                (&origin.license, 4096),
            ] {
                if value.as_ref().is_some_and(|v| v.len() > max) {
                    return Err("audio origin assertion is too long".into());
                }
            }
        }
        local_wav(&item.path)?;
    }
    if !request.source_locks.is_empty() {
        let expected: HashSet<PathBuf> = request
            .items
            .iter()
            .map(|item| fs::canonicalize(&item.path).map_err(|e| e.to_string()))
            .collect::<Result<_, _>>()?;
        let mut locked = HashSet::new();
        for lock in &request.source_locks {
            local_wav(&lock.path)?;
            let path = fs::canonicalize(&lock.path).map_err(|e| e.to_string())?;
            if lock.sha256.len() != 64
                || !lock.sha256.bytes().all(|c| c.is_ascii_hexdigit())
                || !locked.insert(path)
            {
                return Err(
                    "audio sourceLocks require unique local paths and SHA-256 hex hashes".into(),
                );
            }
            if !hash_audio_file(&lock.path)
                .map_err(|e| e.to_string())?
                .eq_ignore_ascii_case(&lock.sha256)
            {
                return Err(format!("source lock mismatch: {}", lock.path.display()));
            }
        }
        if locked != expected {
            return Err("audio sourceLocks must cover exactly all source files".into());
        }
    }
    Ok(())
}

/// Commands use argument vectors and local WAV demuxing. Child processes are killed
/// on cancellation or timeout; temporary output captures avoid full stderr pipes.
fn run_command(command: &mut Command, cancelled: &impl Fn() -> bool) -> Result<Vec<u8>, String> {
    if cancelled() {
        return Err("audio_cancelled".into());
    }
    let mut stdout = tempfile::tempfile().map_err(|e| e.to_string())?;
    let mut stderr = tempfile::tempfile().map_err(|e| e.to_string())?;
    command
        .stdin(Stdio::null())
        .stdout(stdout.try_clone().map_err(|e| e.to_string())?)
        .stderr(stderr.try_clone().map_err(|e| e.to_string())?);
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let started = Instant::now();
    let status = loop {
        if cancelled() || started.elapsed() > Duration::from_secs(180) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(if cancelled() {
                "audio_cancelled"
            } else {
                "audio_tool_timeout"
            }
            .into());
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.to_string());
            }
        }
    };
    if !status.success() {
        stderr.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
        let mut message = String::new();
        stderr
            .take(8192)
            .read_to_string(&mut message)
            .map_err(|e| e.to_string())?;
        return Err(format!("audio tool failed ({status}): {}", message.trim()));
    }
    stdout.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    stdout
        .take(65536)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}

pub fn inspect_audio_source(path: &Path) -> Result<AudioSourceInspection, String> {
    inspect_source_cancellable(path, &|| false)
}

fn inspect_source_cancellable(
    path: &Path,
    cancelled: &impl Fn() -> bool,
) -> Result<AudioSourceInspection, String> {
    local_wav(path)?;
    let path = fs::canonicalize(path).map_err(|e| e.to_string())?;
    let tools = resolve_ffmpeg_paths(&FfmpegSearch::default()).map_err(|e| e.to_string())?;
    let output = run_command(
        Command::new(tools.ffprobe_path)
            .args([
                "-v",
                "error",
                "-protocol_whitelist",
                "file",
                "-f",
                "wav",
                "-show_entries",
                "stream=codec_name,codec_type,sample_rate,channels,duration:format=duration",
                "-of",
                "json",
            ])
            .arg(&path),
        cancelled,
    )?;
    let probe: serde_json::Value = serde_json::from_slice(&output).map_err(|e| e.to_string())?;
    let streams = probe["streams"]
        .as_array()
        .ok_or("FFprobe returned no audio streams")?;
    if streams.len() != 1 || streams[0]["codec_type"] != "audio" {
        return Err("source WAV must contain exactly one audio stream".into());
    }
    let stream = &streams[0];
    let duration_seconds = stream["duration"]
        .as_str()
        .or_else(|| probe["format"]["duration"].as_str())
        .and_then(|v| v.parse::<f64>().ok())
        .ok_or("source WAV has no valid duration")?;
    let sample_rate = stream["sample_rate"]
        .as_str()
        .and_then(|v| v.parse::<u32>().ok())
        .ok_or("source WAV has no sample rate")?;
    let channel_count = stream["channels"]
        .as_u64()
        .ok_or("source WAV has no channel count")?;
    if !duration_seconds.is_finite()
        || duration_seconds <= 0.0
        || duration_seconds > MAX_AUDIO_SECONDS
        || !(8000..=192000).contains(&sample_rate)
        || !(1..=8).contains(&channel_count)
    {
        return Err("source WAV must contain 0..=3600 seconds of audio, 8000..=192000 Hz and 1..=8 channels".into());
    }
    let sha256 = hash_audio_file(&path).map_err(|e| e.to_string())?;
    Ok(AudioSourceInspection {
        path,
        sha256,
        duration_seconds,
        sample_rate,
        channels: channel_count as u16,
        codec_name: stream["codec_name"].as_str().unwrap_or("unknown").into(),
    })
}

/// Wrap untouched signed little-endian PCM16 in a classic RIFF/WAVE header.
/// Godot 4.6 rejects WAVE_FORMAT_EXTENSIBLE, which FFmpeg otherwise chooses
/// automatically above 48 kHz. The wrapper changes no audio samples.
fn write_classic_pcm_wav(
    raw_path: &Path,
    output: &Path,
    sample_rate: u32,
    channels: u16,
) -> Result<(), String> {
    let mut raw = File::open(raw_path).map_err(|e| e.to_string())?;
    let data_length = raw.metadata().map_err(|e| e.to_string())?.len();
    let block_align = channels.checked_mul(2).ok_or("PCM frame size overflow")?;
    if !(1..=2).contains(&channels)
        || !(8000..=96000).contains(&sample_rate)
        || data_length == 0
        || data_length > MAX_AUDIO_BYTES - 44
        || data_length % block_align as u64 != 0
    {
        return Err("invalid or oversized PCM16 data for canonical WAV delivery".into());
    }
    let data_length_u32 = u32::try_from(data_length).map_err(|e| e.to_string())?;
    let mut header = Vec::with_capacity(44);
    header.extend(b"RIFF");
    header.extend((data_length_u32 + 36).to_le_bytes());
    header.extend(b"WAVEfmt ");
    header.extend(16_u32.to_le_bytes());
    header.extend(1_u16.to_le_bytes());
    header.extend(channels.to_le_bytes());
    header.extend(sample_rate.to_le_bytes());
    header.extend((sample_rate * block_align as u32).to_le_bytes());
    header.extend(block_align.to_le_bytes());
    header.extend(16_u16.to_le_bytes());
    header.extend(b"data");
    header.extend(data_length_u32.to_le_bytes());
    let mut wav = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    wav.write_all(&header).map_err(|e| e.to_string())?;
    let copied = std::io::copy(&mut raw, &mut wav).map_err(|e| e.to_string())?;
    if copied != data_length {
        return Err("PCM data changed while writing canonical WAV".into());
    }
    wav.flush().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn prepare_audio(
    request: &PrepareAudioRequest,
    output: &Path,
) -> Result<PreparedAudio, String> {
    prepare_audio_cancellable(request, output, || false)
}

pub fn prepare_audio_cancellable(
    request: &PrepareAudioRequest,
    output: &Path,
    cancelled: impl Fn() -> bool,
) -> Result<PreparedAudio, String> {
    validate_request(request)?;
    if cancelled() {
        return Err("audio_cancelled".into());
    }
    let tools = resolve_ffmpeg_paths(&FfmpegSearch::default()).map_err(|e| e.to_string())?;
    if output.extension().and_then(|v| v.to_str()) != Some("gsfpack") {
        return Err("audio output must be a .gsfpack directory".into());
    }
    if let Some(parent) = output.parent().filter(|v| !v.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // create_dir intentionally rejects existing paths; error cleanup only owns this directory.
    fs::create_dir(output)
        .map_err(|e| format!("cannot create new audio Pack {}: {e}", output.display()))?;
    let result = (|| {
        let output = fs::canonicalize(output).map_err(|e| e.to_string())?;
        for directory in ["assets/audio", "sources"] {
            fs::create_dir_all(output.join(directory)).map_err(|e| e.to_string())?;
        }
        let mut items = Vec::new();
        let mut warnings = Vec::new();
        for item in &request.items {
            if cancelled() {
                return Err("audio_cancelled".into());
            }
            let input_hash = hash_audio_file(&item.path).map_err(|e| e.to_string())?;
            let input_path = fs::canonicalize(&item.path).map_err(|e| e.to_string())?;
            for lock in &request.source_locks {
                if fs::canonicalize(&lock.path).map_err(|e| e.to_string())? == input_path
                    && !lock.sha256.eq_ignore_ascii_case(&input_hash)
                {
                    return Err(format!("source lock mismatch for {}", item.id));
                }
            }
            let source_path = format!("sources/{}.wav", item.id);
            let retained_path = output.join(&source_path);
            fs::copy(&input_path, &retained_path).map_err(|e| e.to_string())?;
            let retained_hash = hash_audio_file(&retained_path).map_err(|e| e.to_string())?;
            if retained_hash != input_hash {
                return Err(format!("source changed during audio intake: {}", item.id));
            }
            let inspected = inspect_source_cancellable(&retained_path, &cancelled)?;
            let trim_end = item.trim_end_seconds.unwrap_or(inspected.duration_seconds);
            if item.trim_start_seconds >= inspected.duration_seconds
                || trim_end > inspected.duration_seconds + 0.000001
            {
                return Err(format!(
                    "trim range exceeds decoded source duration for {}",
                    item.id
                ));
            }
            let estimated_frames =
                ((trim_end - item.trim_start_seconds) * request.sample_rate as f64).ceil() as u64
                    + 2;
            if estimated_frames * request.channels as u64 * 2 + 128 > MAX_AUDIO_BYTES {
                return Err(format!(
                    "processed audio would exceed the 512 MiB per-file limit: {}",
                    item.id
                ));
            }
            let intermediate = output.join(format!(".{}.working.f32", item.id));
            let normalize = format!(
                "atrim=start={}:end={},asetpts=PTS-STARTPTS",
                item.trim_start_seconds, trim_end
            );
            run_command(
                Command::new(&tools.ffmpeg_path)
                    .args([
                        "-v",
                        "error",
                        "-nostdin",
                        "-n",
                        "-protocol_whitelist",
                        "file",
                        "-f",
                        "wav",
                        "-i",
                    ])
                    .arg(&retained_path)
                    .args([
                        "-map",
                        "0:a:0",
                        "-vn",
                        "-sn",
                        "-dn",
                        "-map_metadata",
                        "-1",
                        "-af",
                        &normalize,
                        "-ar",
                        &request.sample_rate.to_string(),
                        "-ac",
                        &request.channels.to_string(),
                        "-c:a",
                        "pcm_f32le",
                        "-f",
                        "f32le",
                    ])
                    .arg(&intermediate),
                &cancelled,
            )?;
            let intermediate_bytes = fs::metadata(&intermediate)
                .map_err(|e| e.to_string())?
                .len();
            let frame_bytes = request.channels as u64 * 4;
            if intermediate_bytes == 0
                || intermediate_bytes > MAX_AUDIO_BYTES * 2
                || intermediate_bytes % frame_bytes != 0
            {
                return Err("invalid or oversized decoded float audio intermediate".into());
            }
            let normalized_duration =
                (intermediate_bytes / frame_bytes) as f64 / request.sample_rate as f64;
            let crossfade_seconds = item.crossfade_ms as f64 / 1000.0;
            if crossfade_seconds * 2.0 >= normalized_duration {
                return Err(format!(
                    "crossfade must be shorter than half of the trimmed audio: {}",
                    item.id
                ));
            }
            let final_duration = normalized_duration - crossfade_seconds;
            if item.fade_in_ms as f64 / 1000.0 > final_duration
                || item.fade_out_ms as f64 / 1000.0 > final_duration
            {
                return Err(format!(
                    "fade duration exceeds processed audio duration: {}",
                    item.id
                ));
            }
            let mut filters = Vec::new();
            if item.fade_in_ms > 0 {
                filters.push(format!("afade=t=in:d={}", item.fade_in_ms as f64 / 1000.0));
            }
            if item.fade_out_ms > 0 {
                filters.push(format!(
                    "afade=t=out:st={}:d={}",
                    final_duration - item.fade_out_ms as f64 / 1000.0,
                    item.fade_out_ms as f64 / 1000.0
                ));
            }
            filters.push(format!("volume={}dB", item.gain_db));
            let path = format!("assets/audio/{}.wav", item.id);
            let final_path = output.join(&path);
            let final_pcm = output.join(format!(".{}.final.s16", item.id));
            let mut command = Command::new(&tools.ffmpeg_path);
            command
                .args([
                    "-v",
                    "error",
                    "-nostdin",
                    "-n",
                    "-protocol_whitelist",
                    "file",
                    "-f",
                    "f32le",
                    "-ar",
                    &request.sample_rate.to_string(),
                    "-ac",
                    &request.channels.to_string(),
                    "-i",
                ])
                .arg(&intermediate);
            if item.crossfade_ms > 0 {
                // Rotate to the end of the head segment; join middle + blended tail/head.
                // This removes one crossfade duration and leaves the wrap at head->middle.
                let end = normalized_duration - crossfade_seconds;
                let graph = format!("[0:a]asplit=3[m][t][h];[m]atrim=start={crossfade_seconds}:end={end},asetpts=PTS-STARTPTS[mid];[t]atrim=start={end},asetpts=PTS-STARTPTS[tail];[h]atrim=end={crossfade_seconds},asetpts=PTS-STARTPTS[head];[tail][head]acrossfade=d={crossfade_seconds}:c1=tri:c2=tri[blend];[mid][blend]concat=n=2:v=0:a=1,{}[out]", filters.join(","));
                command.args(["-filter_complex", &graph, "-map", "[out]"]);
            } else {
                command.args(["-map", "0:a:0", "-af", &filters.join(",")]);
            }
            command
                .args([
                    "-vn",
                    "-sn",
                    "-dn",
                    "-map_metadata",
                    "-1",
                    "-ar",
                    &request.sample_rate.to_string(),
                    "-ac",
                    &request.channels.to_string(),
                    "-c:a",
                    "pcm_s16le",
                    "-f",
                    "s16le",
                ])
                .arg(&final_pcm);
            run_command(&mut command, &cancelled)?;
            write_classic_pcm_wav(
                &final_pcm,
                &final_path,
                request.sample_rate,
                request.channels,
            )?;
            fs::remove_file(&final_pcm).map_err(|e| e.to_string())?;
            fs::remove_file(&intermediate).map_err(|e| e.to_string())?;
            let audio = inspect_pcm_wav(&final_path).map_err(|e| e.to_string())?;
            if (audio.duration_seconds - final_duration).abs() > 2.0 / request.sample_rate as f64 {
                return Err(format!(
                    "processed audio duration differs from requested operations: {}",
                    item.id
                ));
            }
            if audio.clipped_sample_count > 0 {
                warnings.push(format!(
                    "{}: {} samples reach PCM16 limits; listen for clipping",
                    item.id, audio.clipped_sample_count
                ));
            }
            if audio.peak_amplitude == 0.0 {
                warnings.push(format!("{}: decoded output is silent", item.id));
            }
            if item.loop_audio {
                warnings.push(format!(
                    "{}: loop playback requested; seamless loop quality requires listening review",
                    item.id
                ));
            }
            items.push(AudioItem {
                id: item.id.clone(),
                name: if item.name.trim().is_empty() {
                    item.id.clone()
                } else {
                    item.name.clone()
                },
                role: item.role,
                path,
                loop_audio: item.loop_audio,
                sha256: hash_audio_file(&final_path).map_err(|e| e.to_string())?,
                source: AudioSource {
                    path: source_path,
                    sha256: retained_hash,
                    original_path: input_path.to_string_lossy().into_owned(),
                    origin: item.origin.clone(),
                },
                audio,
                processing: AudioProcessing {
                    trim_start_seconds: item.trim_start_seconds,
                    trim_end_seconds: item.trim_end_seconds,
                    fade_in_ms: item.fade_in_ms,
                    fade_out_ms: item.fade_out_ms,
                    gain_db: item.gain_db,
                    crossfade_ms: item.crossfade_ms,
                },
            });
        }
        let manifest = AudioManifest {
            schema_version: "4.0.0".into(),
            asset_type: "audio_set".into(),
            id: request.id.clone(),
            name: request.name.clone(),
            items: items.clone(),
        };
        let report = AudioQualityReport {
            schema_version: "4.0.0".into(),
            asset_type: "audio_set".into(),
            profile: "local-audio-import@1.0.0".into(),
            verdict: "technical_pass".into(),
            provider_request_occurred: false,
            provider_request_count: 0,
            generation_performed: false,
            origin_verified: false,
            license_verified: false,
            listening_review_performed: false,
            seamless_loop_verified: false,
            items,
            warnings,
        };
        let header = json!({"schemaVersion":"4.0.0","assetType":"audio_set","id":request.id,"name":request.name,"version":"1.0.0","previews":{},
            "assets":{"manifest":"assets/manifest.json","qualityReport":"quality-report.json","godotHelper":"assets/godot_import.json"},
            "provenance":{"kind":"local_audio_import","generationPerformed":false,"licenseVerified":false}});
        for (path, value) in [
            ("forgepack.json", header),
            ("assets/manifest.json", json!(manifest)),
            ("assets/godot_import.json", json!(manifest)),
            ("quality-report.json", json!(report)),
        ] {
            fs::write(
                output.join(path),
                serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        }
        if cancelled() {
            return Err("audio_cancelled".into());
        }
        forge_pack::validate_pack_layout(&output).map_err(|e| e.to_string())?;
        if cancelled() {
            return Err("audio_cancelled".into());
        }
        Ok(PreparedAudio {
            manifest_path: output.join("assets/manifest.json"),
            quality_report_path: output.join("quality-report.json"),
            pack_path: output,
        })
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(output);
    }
    result
}
