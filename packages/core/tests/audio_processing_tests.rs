use forge_core::{
    audio::{
        inspect_audio_source, prepare_audio, prepare_audio_cancellable, validate_request,
        PrepareAudioRequest,
    },
    video::{resolve_ffmpeg_paths, FfmpegSearch},
};
use forge_pack::audio::{inspect_pcm_wav, read_audio_manifest};
use serde_json::{json, Value};
use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

fn wav(path: &Path, seconds: f64, float: bool) {
    let rate = 48000_u32;
    let frames = (rate as f64 * seconds).round() as u32;
    let bytes_per_sample = if float { 4_u32 } else { 2 };
    let size = frames * bytes_per_sample;
    let mut data = Vec::new();
    data.extend(b"RIFF");
    data.extend((36 + size).to_le_bytes());
    data.extend(b"WAVEfmt ");
    data.extend(16_u32.to_le_bytes());
    data.extend((if float { 3_u16 } else { 1 }).to_le_bytes());
    data.extend(1_u16.to_le_bytes());
    data.extend(rate.to_le_bytes());
    data.extend((rate * bytes_per_sample).to_le_bytes());
    data.extend((bytes_per_sample as u16).to_le_bytes());
    data.extend((bytes_per_sample as u16 * 8).to_le_bytes());
    data.extend(b"data");
    data.extend(size.to_le_bytes());
    for n in 0..frames {
        let sample = (n as f64 / rate as f64 * std::f64::consts::TAU * 440.0).sin();
        if float {
            data.extend((sample as f32 * 1.5).to_le_bytes());
        } else {
            data.extend(((sample * 24000.0) as i16).to_le_bytes());
        }
    }
    fs::write(path, data).unwrap();
}

fn request(path: &Path) -> PrepareAudioRequest {
    serde_json::from_value(json!({"id":"game_audio","name":"Game audio","items":[{"id":"theme","path":path,"role":"music","origin":{"tool":"external_test_fixture","license":"user assertion"}}]})).unwrap()
}
fn tools_available() -> bool {
    let available = resolve_ffmpeg_paths(&FfmpegSearch::default()).is_ok();
    if !available {
        eprintln!("skipping audio integration: FFmpeg/FFprobe unavailable");
    }
    available
}
fn rewrite_json(path: &Path, change: impl FnOnce(&mut Value)) {
    let mut value: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    change(&mut value);
    fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}

#[test]
fn rejects_invalid_requests_and_non_wav_input() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("input.wav");
    wav(&source, 0.2, false);
    let mut input = request(&source);
    validate_request(&input).unwrap();
    input.items[0].gain_db = f64::NAN;
    assert!(validate_request(&input).is_err());
    input.items[0].gain_db = 0.0;
    input.items[0].crossfade_ms = 10;
    assert!(validate_request(&input).is_err());
    input.items[0].crossfade_ms = 0;
    input.items.push(input.items[0].clone());
    input.items[1].id = "THEME".into();
    assert!(validate_request(&input).is_err());
    input.items.pop();
    input.items[0].id = "../escape".into();
    assert!(validate_request(&input).is_err());
    fs::write(&source, b"#EXTM3U\nhttps://example.com/media.wav\n").unwrap();
    assert!(validate_request(&request(&source)).is_err());
    input.items[0].path = "https://example.com/music.wav".into();
    assert!(validate_request(&input).is_err());
}

#[test]
fn source_locks_are_complete_case_insensitive_and_allow_parent_paths() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("input.wav");
    wav(&source, 0.1, false);
    fs::create_dir(temp.path().join("specs")).unwrap();
    let mut input = request(&temp.path().join("specs/../input.wav"));
    let hash = forge_pack::audio::hash_audio_file(&source).unwrap();
    input.source_locks = vec![forge_core::automation::SourceLock {
        path: source.clone(),
        sha256: hash.to_ascii_uppercase(),
    }];
    validate_request(&input).unwrap();
    if tools_available() {
        prepare_audio(&input, &temp.path().join("locked.gsfpack")).unwrap();
    }
    let second = temp.path().join("other.wav");
    wav(&second, 0.1, false);
    input.source_locks[0].path = second;
    assert!(validate_request(&input).is_err());
    input.source_locks[0].path = source;
    input.source_locks[0].sha256 = "0".repeat(64);
    assert!(validate_request(&input).is_err());
}

#[test]
fn prepares_audio_pack_with_trim_fades_resampling_fractional_crossfade_and_provenance() {
    if !tools_available() {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("input.wav");
    wav(&source, 2.0, false);
    let inspected = inspect_audio_source(&source).unwrap();
    assert_eq!(inspected.channels, 1);
    assert_eq!(inspected.duration_seconds, 2.0);
    let mut input = request(&source);
    input.sample_rate = 44100;
    input.channels = 2;
    let item = &mut input.items[0];
    item.loop_audio = true;
    item.trim_start_seconds = 0.123;
    item.trim_end_seconds = Some(1.913);
    item.fade_in_ms = 15;
    item.fade_out_ms = 17;
    item.crossfade_ms = 37;
    item.gain_db = -3.0;
    let output = temp.path().join("audio.gsfpack");
    let result = prepare_audio(&input, &output).unwrap();
    assert_eq!(result.pack_path, fs::canonicalize(&output).unwrap());
    forge_pack::validate_pack_layout(&output).unwrap();
    let manifest = read_audio_manifest(&output).unwrap();
    let audio = &manifest.items[0].audio;
    assert!((audio.duration_seconds - 1.753).abs() < 2.0 / 44100.0);
    assert_eq!(
        (audio.sample_rate, audio.channels, audio.bits_per_sample),
        (44100, 2, 16)
    );
    // FFmpeg uses its standard equal-power mono-to-stereo rematrix.
    assert!(
        audio.peak_amplitude > 0.35 && audio.peak_amplitude < 0.39,
        "unexpected output peak {}",
        audio.peak_amplitude
    );
    assert_eq!(audio.clipped_sample_count, 0);
    assert_eq!(
        fs::read(output.join("sources/theme.wav")).unwrap(),
        fs::read(&source).unwrap()
    );
    assert_eq!(manifest.items[0].source.sha256, inspected.sha256);
    let report: Value =
        serde_json::from_slice(&fs::read(result.quality_report_path).unwrap()).unwrap();
    assert_eq!(report["providerRequestCount"], 0);
    assert_eq!(report["licenseVerified"], false);
    assert_eq!(report["seamlessLoopVerified"], false);
    let imported = forge_pack::import_pack(&output).unwrap();
    assert!(imported.frame_paths.is_empty());
    assert!(imported.atlas.is_null());
    let summary = forge_pack::inspect_pack(&output).unwrap();
    assert_eq!(summary.asset_type, "audio_set");
    assert_eq!(summary.audio_items.len(), 1);
    assert!(!output.join("previews/preview.gif").exists());
}

#[test]
fn float_sources_are_not_clipped_before_requested_attenuation() {
    if !tools_available() {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("float.wav");
    wav(&source, 0.2, true);
    let mut input = request(&source);
    input.channels = 1;
    input.items[0].gain_db = -6.0;
    let output = temp.path().join("float.gsfpack");
    prepare_audio(&input, &output).unwrap();
    let audio = inspect_pcm_wav(&output.join("assets/audio/theme.wav")).unwrap();
    assert!(
        audio.peak_amplitude > 0.74 && audio.peak_amplitude < 0.76,
        "unexpected attenuated peak {}",
        audio.peak_amplitude
    );
    assert_eq!(audio.frame_count, 9600);
    assert_eq!(audio.clipped_sample_count, 0);
}

#[test]
fn loop_intent_alone_preserves_samples_and_rejects_overwrite() {
    if !tools_available() {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.wav");
    wav(&source, 0.1, false);
    let mut input = request(&source);
    input.channels = 1;
    input.items[0].loop_audio = true;
    let output = temp.path().join("loop.gsfpack");
    prepare_audio(&input, &output).unwrap();
    assert_eq!(
        read_audio_manifest(&output).unwrap().items[0]
            .audio
            .frame_count,
        4800
    );
    let before = fs::read(output.join("forgepack.json")).unwrap();
    assert!(prepare_audio(&input, &output).is_err());
    assert_eq!(fs::read(output.join("forgepack.json")).unwrap(), before);
}

#[test]
fn cancellation_and_invalid_trim_clean_partial_packs() {
    if !tools_available() {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("input.wav");
    wav(&source, 0.2, false);
    let mut input = request(&source);
    let output = temp.path().join("cancel.gsfpack");
    let calls = AtomicUsize::new(0);
    let error =
        prepare_audio_cancellable(&input, &output, || calls.fetch_add(1, Ordering::SeqCst) > 3)
            .unwrap_err();
    assert!(error.contains("audio_cancelled"));
    assert!(!output.exists());
    input.items[0].trim_end_seconds = Some(1.0);
    assert!(prepare_audio(&input, &output).is_err());
    assert!(!output.exists());
    input.items[0].trim_end_seconds = None;
    input.items[0].fade_out_ms = 500;
    assert!(prepare_audio(&input, &output).is_err());
    assert!(!output.exists());
}

#[test]
fn audio_pack_validation_rejects_corrupt_bytes_timing_inventory_and_helpers() {
    if !tools_available() {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("input.wav");
    wav(&source, 0.1, false);
    for scenario in [
        "output",
        "source",
        "timing",
        "helper",
        "unlisted",
        "traversal",
        "crossfade",
        "trim_metadata",
        "oversize_name",
        "oversize_origin",
    ] {
        let output = temp.path().join(format!("{scenario}.gsfpack"));
        prepare_audio(&request(&source), &output).unwrap();
        match scenario {
            "output" | "source" => {
                let path = output.join(if scenario == "output" {
                    "assets/audio/theme.wav"
                } else {
                    "sources/theme.wav"
                });
                let mut bytes = fs::read(&path).unwrap();
                let index = bytes.len() - 2;
                bytes[index] ^= 1;
                fs::write(path, bytes).unwrap();
            }
            "timing" => {
                for path in [
                    "assets/manifest.json",
                    "assets/godot_import.json",
                    "quality-report.json",
                ] {
                    rewrite_json(&output.join(path), |v| {
                        v["items"][0]["audio"]["frameCount"] = json!(7)
                    });
                }
            }
            "crossfade" | "trim_metadata" => {
                for path in [
                    "assets/manifest.json",
                    "assets/godot_import.json",
                    "quality-report.json",
                ] {
                    rewrite_json(&output.join(path), |v| {
                        if scenario == "crossfade" {
                            v["items"][0]["loop"] = json!(true);
                            v["items"][0]["processing"]["crossfadeMs"] = json!(30000);
                        } else {
                            v["items"][0]["processing"]["trimEndSeconds"] = json!(12.0);
                        }
                    });
                }
            }
            "oversize_name" | "oversize_origin" => {
                for path in [
                    "assets/manifest.json",
                    "assets/godot_import.json",
                    "quality-report.json",
                ] {
                    rewrite_json(&output.join(path), |v| {
                        if scenario == "oversize_name" {
                            v["items"][0]["name"] = json!("n".repeat(257));
                        } else {
                            v["items"][0]["source"]["origin"]["tool"] = json!("t".repeat(1025));
                        }
                    });
                }
            }
            "helper" => rewrite_json(&output.join("assets/godot_import.json"), |v| {
                v["items"][0]["loop"] = json!(true)
            }),
            "unlisted" => fs::write(output.join("hidden.bin"), b"not an audio asset").unwrap(),
            "traversal" => {
                for path in [
                    "assets/manifest.json",
                    "assets/godot_import.json",
                    "quality-report.json",
                ] {
                    rewrite_json(&output.join(path), |v| {
                        v["items"][0]["path"] = json!("../input.wav")
                    });
                }
            }
            _ => unreachable!(),
        }
        assert!(
            forge_pack::validate_pack_layout(&output).is_err(),
            "accepted {scenario}"
        );
        assert!(
            forge_pack::import_pack(&output).is_err(),
            "import accepted {scenario}"
        );
        assert!(
            forge_pack::read_pack_summary(&output).is_err(),
            "summary accepted {scenario}"
        );
    }
}

#[cfg(unix)]
#[test]
fn audio_pack_rejects_symlinked_source_and_parent_inventory_directory() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("input.wav");
    wav(&source, 0.1, false);
    let link = temp.path().join("link.wav");
    symlink(&source, &link).unwrap();
    assert!(validate_request(&request(&link)).is_err());
    if !tools_available() {
        return;
    }
    let output = temp.path().join("pack.gsfpack");
    prepare_audio(&request(&source), &output).unwrap();
    fs::rename(output.join("sources"), temp.path().join("retained")).unwrap();
    symlink(temp.path().join("retained"), output.join("sources")).unwrap();
    assert!(forge_pack::validate_pack_layout(&output).is_err());
}

#[test]
fn low_rate_source_trim_accounts_for_source_frame_quantization() {
    if !tools_available() {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("low-rate.wav");
    wav(&source, 0.1, false);
    // Keep the same samples, reinterpret them at 8 kHz: a 0.6-second source.
    let mut bytes = fs::read(&source).unwrap();
    bytes[24..28].copy_from_slice(&8000_u32.to_le_bytes());
    bytes[28..32].copy_from_slice(&16000_u32.to_le_bytes());
    fs::write(&source, bytes).unwrap();
    let mut input = request(&source);
    input.sample_rate = 96000;
    input.channels = 1;
    input.items[0].trim_start_seconds = 0.01304;
    input.items[0].trim_end_seconds = Some(0.51407);
    let output = temp.path().join("resampled.gsfpack");
    prepare_audio(&input, &output).unwrap();
    forge_pack::validate_pack_layout(&output).unwrap();
    let info = inspect_pcm_wav(&output.join("assets/audio/theme.wav")).unwrap();
    assert!((info.duration_seconds - 0.50103).abs() < 1.0 / 8000.0 + 2.0 / 96000.0);
}

#[test]
fn pcm16_output_at_96khz_uses_classic_header_and_preserves_samples() {
    if !tools_available() {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source96k.wav");
    wav(&source, 0.1, false);
    let mut original = fs::read(&source).unwrap();
    original[24..28].copy_from_slice(&96000_u32.to_le_bytes());
    original[28..32].copy_from_slice(&192000_u32.to_le_bytes());
    fs::write(&source, &original).unwrap();
    let mut input = request(&source);
    input.sample_rate = 96000;
    input.channels = 1;
    let output = temp.path().join("classic96k.gsfpack");
    prepare_audio(&input, &output).unwrap();
    let delivered = fs::read(output.join("assets/audio/theme.wav")).unwrap();
    assert_eq!(u16::from_le_bytes(delivered[20..22].try_into().unwrap()), 1);
    assert_eq!(
        u32::from_le_bytes(delivered[16..20].try_into().unwrap()),
        16
    );
    assert_eq!(&delivered[36..40], b"data");
    assert_eq!(&delivered[44..], &original[44..]);
    assert_eq!(
        inspect_pcm_wav(&output.join("assets/audio/theme.wav"))
            .unwrap()
            .sample_rate,
        96000
    );
    forge_pack::validate_pack_layout(&output).unwrap();
}

#[test]
fn native_absolute_paths_with_spaces_are_valid_audio_sources() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("audio sources");
    fs::create_dir(&directory).unwrap();
    let source = directory.join("theme cue.wav");
    wav(&source, 0.2, false);
    let canonical = fs::canonicalize(&source).unwrap();
    validate_request(&request(&canonical)).unwrap();
    // Windows canonical paths use a verbatim prefix; ordinary drive paths must
    // also pass intake before they are canonicalized for FFprobe.
    #[cfg(windows)]
    {
        let native = canonical.to_string_lossy();
        assert!(native.contains('\\'));
        if let Some(drive_path) = native.strip_prefix(r"\\?\") {
            validate_request(&request(Path::new(drive_path))).unwrap();
        }
    }
}
