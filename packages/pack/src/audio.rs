//! Version 4 audio Packs. Sources are retained evidence; origin is user asserted.
use crate::PackError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Component, Path},
};

pub const AUDIO_SCHEMA_VERSION: &str = "4.0.0";
const AUDIO_PACK_SCHEMA: &str = include_str!("../../../schemas/audio-pack.schema.json");
pub const MAX_AUDIO_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_AUDIO_SECONDS: f64 = 3600.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioOrigin {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioRole {
    Music,
    Sfx,
    Ambience,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioProcessing {
    pub trim_start_seconds: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trim_end_seconds: Option<f64>,
    pub fade_in_ms: u32,
    pub fade_out_ms: u32,
    pub gain_db: f64,
    pub crossfade_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioInfo {
    pub duration_seconds: f64,
    pub sample_rate: u32,
    pub channels: u16,
    pub frame_count: u64,
    pub bits_per_sample: u16,
    pub peak_amplitude: f64,
    pub rms_amplitude: f64,
    pub clipped_sample_count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioSource {
    pub path: String,
    pub sha256: String,
    pub original_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<AudioOrigin>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioItem {
    pub id: String,
    pub name: String,
    pub role: AudioRole,
    pub path: String,
    #[serde(rename = "loop")]
    pub loop_audio: bool,
    pub sha256: String,
    pub source: AudioSource,
    pub audio: AudioInfo,
    pub processing: AudioProcessing,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioManifest {
    pub schema_version: String,
    pub asset_type: String,
    pub id: String,
    pub name: String,
    pub items: Vec<AudioItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioQualityReport {
    pub schema_version: String,
    pub asset_type: String,
    pub profile: String,
    pub verdict: String,
    pub provider_request_occurred: bool,
    pub provider_request_count: u32,
    pub generation_performed: bool,
    pub origin_verified: bool,
    pub license_verified: bool,
    pub listening_review_performed: bool,
    pub seamless_loop_verified: bool,
    pub items: Vec<AudioItem>,
    pub warnings: Vec<String>,
}

fn invalid(message: impl Into<String>) -> PackError {
    PackError::SchemaValidation {
        document: "audio Pack".into(),
        message: message.into(),
    }
}

pub fn valid_audio_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id.as_bytes()[0].is_ascii_alphanumeric()
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}

pub fn hash_audio_file(path: &Path) -> Result<String, PackError> {
    let mut reader = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

/// Validate retained RIFF/WAVE container boundaries without an external decoder.
/// Compressed WAV codecs remain FFmpeg's responsibility at intake.
pub fn validate_wav_container(path: &Path) -> Result<(), PackError> {
    validated_wav_source_sample_rate(path).map(|_| ())
}

fn validated_wav_source_sample_rate(path: &Path) -> Result<u32, PackError> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    if !(44..=MAX_AUDIO_BYTES).contains(&length) {
        return Err(invalid("WAV source size is outside 44 bytes..512 MiB"));
    }
    let mut header = [0; 12];
    file.read_exact(&mut header)?;
    if &header[..4] != b"RIFF"
        || &header[8..] != b"WAVE"
        || u32::from_le_bytes(header[4..8].try_into().unwrap()) as u64 + 8 != length
    {
        return Err(invalid("invalid retained WAV RIFF header or length"));
    }
    let mut source_rate = 0;
    let mut has_format = false;
    let mut has_data = false;
    let mut count = 0;
    while file.stream_position()? < length {
        count += 1;
        if count > 1024 {
            return Err(invalid("too many retained WAV chunks"));
        }
        let mut chunk = [0; 8];
        file.read_exact(&mut chunk)?;
        let size = u32::from_le_bytes(chunk[4..8].try_into().unwrap()) as u64;
        let end = file
            .stream_position()?
            .checked_add(size + size % 2)
            .ok_or_else(|| invalid("WAV chunk overflow"))?;
        if end > length {
            return Err(invalid("retained WAV chunk extends past its file"));
        }
        match &chunk[..4] {
            b"fmt " => {
                if has_format || !(16..=4096).contains(&size) {
                    return Err(invalid("invalid retained WAV format chunk"));
                }
                let mut format = [0; 16];
                file.read_exact(&mut format)?;
                let channels = u16::from_le_bytes(format[2..4].try_into().unwrap());
                let sample_rate = u32::from_le_bytes(format[4..8].try_into().unwrap());
                if !(1..=8).contains(&channels) || !(8000..=192000).contains(&sample_rate) {
                    return Err(invalid(
                        "retained WAV channel count or sample rate is unsupported",
                    ));
                }
                source_rate = sample_rate;
                has_format = true;
            }
            b"data" => {
                if has_data || size == 0 {
                    return Err(invalid("empty or duplicate retained WAV data chunk"));
                }
                has_data = true;
            }
            _ => {}
        }
        file.seek(SeekFrom::Start(end))?;
    }
    if !has_format || !has_data {
        return Err(invalid("retained WAV lacks format or samples"));
    }
    Ok(source_rate)
}

/// Read canonical RIFF/WAVE PCM16 and measure decoded sample statistics offline.
/// Chunk lengths, RIFF length, frame alignment and decode bounds are all checked.
pub fn inspect_pcm_wav(path: &Path) -> Result<AudioInfo, PackError> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    if !(44..=MAX_AUDIO_BYTES).contains(&length) {
        return Err(invalid("WAV must be 44 bytes through 512 MiB"));
    }
    let mut header = [0u8; 12];
    file.read_exact(&mut header)?;
    if &header[0..4] != b"RIFF"
        || &header[8..12] != b"WAVE"
        || u32::from_le_bytes(header[4..8].try_into().unwrap()) as u64 + 8 != length
    {
        return Err(invalid("invalid WAV RIFF header or length"));
    }
    let mut format = None;
    let mut data = None;
    let mut chunk_count = 0;
    while file.stream_position()? < length {
        chunk_count += 1;
        if chunk_count > 1024 {
            return Err(invalid("too many WAV chunks"));
        }
        let mut chunk = [0u8; 8];
        file.read_exact(&mut chunk)?;
        let size = u32::from_le_bytes(chunk[4..8].try_into().unwrap()) as u64;
        let start = file.stream_position()?;
        let end = start
            .checked_add(size + size % 2)
            .ok_or_else(|| invalid("WAV chunk overflow"))?;
        if end > length {
            return Err(invalid("WAV chunk extends past RIFF data"));
        }
        match &chunk[..4] {
            b"fmt " => {
                if format.is_some() || !(16..=40).contains(&size) {
                    return Err(invalid("invalid or duplicate WAV format chunk"));
                }
                let mut bytes = vec![0; size as usize];
                file.read_exact(&mut bytes)?;
                let u16_at = |n| u16::from_le_bytes(bytes[n..n + 2].try_into().unwrap());
                let u32_at = |n| u32::from_le_bytes(bytes[n..n + 4].try_into().unwrap());
                let channels = u16_at(2);
                let rate = u32_at(4);
                // Canonical delivery uses classic PCM: Godot 4.6 cannot load
                // WAVE_FORMAT_EXTENSIBLE even when its subtype is PCM16.
                if u16_at(0) != 1
                    || !(1..=2).contains(&channels)
                    || !(8000..=96000).contains(&rate)
                    || u16_at(14) != 16
                    || u16_at(12) != channels * 2
                    || u32_at(8) != rate * channels as u32 * 2
                {
                    return Err(invalid("expected classic-format PCM16 WAV, 1 or 2 channels, sampleRate 8000..=96000 with consistent frame layout"));
                }
                format = Some((channels, rate));
            }
            b"data" => {
                if data.is_some() || size == 0 {
                    return Err(invalid("empty or duplicate WAV data chunk"));
                }
                data = Some((start, size));
            }
            _ => {}
        }
        file.seek(SeekFrom::Start(end))?;
    }
    let (channels, sample_rate) = format.ok_or_else(|| invalid("missing WAV format"))?;
    let (offset, size) = data.ok_or_else(|| invalid("missing WAV samples"))?;
    if size % (channels as u64 * 2) != 0 {
        return Err(invalid("WAV samples are not frame aligned"));
    }
    let frame_count = size / (channels as u64 * 2);
    let duration_seconds = frame_count as f64 / sample_rate as f64;
    if duration_seconds > MAX_AUDIO_SECONDS {
        return Err(invalid("audio exceeds 3600 seconds"));
    }
    let mut sum_squares = 0.0_f64;
    let mut peak = 0i32;
    let mut clipped_sample_count = 0;
    let mut remaining = size;
    let mut buffer = [0u8; 65536];
    file.seek(SeekFrom::Start(offset))?;
    while remaining > 0 {
        let count = remaining.min(buffer.len() as u64) as usize;
        file.read_exact(&mut buffer[..count])?;
        for bytes in buffer[..count].chunks_exact(2) {
            let value = i16::from_le_bytes([bytes[0], bytes[1]]);
            if value == i16::MIN || value == i16::MAX {
                clipped_sample_count += 1;
            }
            peak = peak.max((value as i32).abs());
            sum_squares += (value as f64 / 32768.0).powi(2);
        }
        remaining -= count as u64;
    }
    Ok(AudioInfo {
        duration_seconds,
        sample_rate,
        channels,
        frame_count,
        bits_per_sample: 16,
        peak_amplitude: peak as f64 / 32768.0,
        rms_amplitude: (sum_squares / (size / 2) as f64).sqrt(),
        clipped_sample_count,
    })
}

fn safe_file(root: &Path, relative: &str) -> Result<(), PackError> {
    if relative.contains('\\')
        || Path::new(relative)
            .components()
            .any(|p| !matches!(p, Component::Normal(_)))
    {
        return Err(PackError::InvalidAssetPath(relative.into()));
    }
    let mut path = root.to_path_buf();
    for part in Path::new(relative).components() {
        path.push(part);
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            return Err(PackError::InvalidAssetPath(relative.into()));
        }
    }
    crate::require_regular_pack_file(root, relative)
}

pub fn read_audio_manifest(pack_path: &Path) -> Result<AudioManifest, PackError> {
    validate_audio_pack(pack_path)?;
    Ok(serde_json::from_slice(&fs::read(
        pack_path.join("assets/manifest.json"),
    )?)?)
}

pub fn validate_audio_pack(root: &Path) -> Result<(), PackError> {
    crate::require_pack_root_directory(root)?;
    for path in [
        "forgepack.json",
        "assets/manifest.json",
        "assets/godot_import.json",
        "quality-report.json",
    ] {
        safe_file(root, path)?;
    }
    crate::validate_json_file(
        root.join("forgepack.json"),
        "audio-pack.schema.json",
        AUDIO_PACK_SCHEMA,
    )?;
    for (path, definition) in [
        ("assets/manifest.json", "manifest"),
        ("assets/godot_import.json", "manifest"),
        ("quality-report.json", "report"),
    ] {
        let mut schema: serde_json::Value = serde_json::from_str(AUDIO_PACK_SCHEMA)?;
        schema["$ref"] = serde_json::Value::String(format!("#/$defs/{definition}"));
        crate::validate_json_file(
            root.join(path),
            "audio-pack.schema.json",
            &serde_json::to_string(&schema)?,
        )?;
    }
    let header = crate::read_json(root.join("forgepack.json"))?;
    let manifest: AudioManifest =
        serde_json::from_slice(&fs::read(root.join("assets/manifest.json"))?)?;
    let helper: AudioManifest =
        serde_json::from_slice(&fs::read(root.join("assets/godot_import.json"))?)?;
    let report: AudioQualityReport =
        serde_json::from_slice(&fs::read(root.join("quality-report.json"))?)?;
    if manifest.schema_version != AUDIO_SCHEMA_VERSION
        || manifest.asset_type != "audio_set"
        || header["id"] != manifest.id
        || header["name"] != manifest.name
        || helper != manifest
        || report.schema_version != AUDIO_SCHEMA_VERSION
        || report.asset_type != "audio_set"
        || report.profile != "local-audio-import@1.0.0"
        || report.verdict != "technical_pass"
        || report.provider_request_occurred
        || report.provider_request_count != 0
        || report.generation_performed
        || report.origin_verified
        || report.license_verified
        || report.listening_review_performed
        || report.seamless_loop_verified
        || report.items != manifest.items
    {
        return Err(invalid(
            "audio manifest, helper, provenance or quality report disagree",
        ));
    }
    if manifest.items.is_empty() || manifest.items.len() > 64 {
        return Err(invalid("audio Pack requires 1..=64 items"));
    }
    let mut ids = HashSet::new();
    let mut inventory: HashSet<String> = [
        "forgepack.json",
        "assets/manifest.json",
        "assets/godot_import.json",
        "quality-report.json",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    for item in &manifest.items {
        if !valid_audio_id(&item.id)
            || item.name.trim().is_empty()
            || !ids.insert(item.id.to_ascii_lowercase())
            || item.path != format!("assets/audio/{}.wav", item.id)
            || item.source.path != format!("sources/{}.wav", item.id)
            || item.source.original_path.is_empty()
        {
            return Err(invalid("invalid or duplicate audio item identity/path"));
        }
        let processing = &item.processing;
        if !processing.trim_start_seconds.is_finite()
            || !(0.0..MAX_AUDIO_SECONDS).contains(&processing.trim_start_seconds)
            || processing.trim_end_seconds.is_some_and(|end| {
                !end.is_finite() || end <= processing.trim_start_seconds || end > MAX_AUDIO_SECONDS
            })
            || !processing.gain_db.is_finite()
            || !(-60.0..=24.0).contains(&processing.gain_db)
            || processing.fade_in_ms as f64 / 1000.0 > item.audio.duration_seconds
            || processing.fade_out_ms as f64 / 1000.0 > item.audio.duration_seconds
            || processing.crossfade_ms > 30000
            || processing.crossfade_ms as f64 / 1000.0 >= item.audio.duration_seconds
            || (processing.crossfade_ms > 0 && !item.loop_audio)
        {
            return Err(invalid("invalid audio processing metadata"));
        }
        for (path, hash) in [
            (&item.path, &item.sha256),
            (&item.source.path, &item.source.sha256),
        ] {
            safe_file(root, path)?;
            if fs::metadata(root.join(path))?.len() > MAX_AUDIO_BYTES {
                return Err(invalid("audio Pack file exceeds 512 MiB"));
            }
            if !inventory.insert(path.clone())
                || hash.len() != 64
                || !hash
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                || hash_audio_file(&root.join(path))? != *hash
            {
                return Err(invalid(format!("audio inventory/hash mismatch: {path}")));
            }
        }
        let source_rate = validated_wav_source_sample_rate(&root.join(&item.source.path))?;
        let measured = inspect_pcm_wav(&root.join(&item.path))?;
        if let Some(end) = processing.trim_end_seconds {
            let expected_duration =
                end - processing.trim_start_seconds - processing.crossfade_ms as f64 / 1000.0;
            let tolerance = 1.0 / source_rate as f64 + 2.0 / measured.sample_rate as f64;
            if (expected_duration - measured.duration_seconds).abs() > tolerance {
                return Err(invalid(format!(
                    "audio trim/crossfade metadata disagrees with output duration: {}",
                    item.id
                )));
            }
        }
        if item.audio.sample_rate != measured.sample_rate
            || item.audio.channels != measured.channels
            || item.audio.frame_count != measured.frame_count
            || item.audio.bits_per_sample != 16
            || item.audio.clipped_sample_count != measured.clipped_sample_count
            || (item.audio.duration_seconds - measured.duration_seconds).abs() > 1e-9
            || (item.audio.peak_amplitude - measured.peak_amplitude).abs() > 1e-9
            || (item.audio.rms_amplitude - measured.rms_amplitude).abs() > 1e-9
        {
            return Err(invalid(format!(
                "audio technical metadata differs from decoded samples: {}",
                item.id
            )));
        }
    }
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|e| invalid(e.to_string()))?;
        if entry.file_type().is_symlink() {
            return Err(invalid("audio Packs cannot contain symlinks"));
        }
        if entry.file_type().is_file() {
            let relative = entry
                .path()
                .strip_prefix(root)
                .map_err(|e| invalid(e.to_string()))?
                .to_string_lossy()
                .replace('\\', "/");
            if !inventory.contains(&relative) {
                return Err(invalid(format!("unlisted audio Pack file: {relative}")));
            }
        } else if !entry.file_type().is_dir() {
            return Err(invalid(
                "audio Packs only contain regular files and directories",
            ));
        }
    }
    Ok(())
}
