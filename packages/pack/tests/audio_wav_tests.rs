use forge_pack::audio::{inspect_pcm_wav, validate_wav_container};
use std::{fs, path::Path};

fn samples(path: &Path) {
    let mut bytes = Vec::new();
    bytes.extend(b"RIFF");
    bytes.extend(44_u32.to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16_u32.to_le_bytes());
    bytes.extend(1_u16.to_le_bytes());
    bytes.extend(2_u16.to_le_bytes());
    bytes.extend(8000_u32.to_le_bytes());
    bytes.extend(32000_u32.to_le_bytes());
    bytes.extend(4_u16.to_le_bytes());
    bytes.extend(16_u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(8_u32.to_le_bytes());
    for value in [i16::MIN, i16::MAX, 0, 16384] {
        bytes.extend(value.to_le_bytes());
    }
    fs::write(path, bytes).unwrap();
}
#[test]
fn measures_pcm_frames_and_clipping_samples() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("fixture.wav");
    samples(&path);
    validate_wav_container(&path).unwrap();
    let info = inspect_pcm_wav(&path).unwrap();
    assert_eq!(info.frame_count, 2);
    assert_eq!(info.duration_seconds, 2.0 / 8000.0);
    assert_eq!(info.peak_amplitude, 1.0);
    assert_eq!(info.clipped_sample_count, 2);
}
#[test]
fn rejects_truncated_chunks_bad_frame_alignment_and_non_pcm() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("fixture.wav");
    samples(&path);
    let valid = fs::read(&path).unwrap();
    for scenario in ["riff_length", "chunk_length", "alignment", "codec", "empty"] {
        let mut bytes = valid.clone();
        match scenario {
            "riff_length" => bytes[4..8].copy_from_slice(&36_u32.to_le_bytes()),
            "chunk_length" => bytes[40..44].copy_from_slice(&1000_u32.to_le_bytes()),
            "alignment" => bytes[32..34].copy_from_slice(&2_u16.to_le_bytes()),
            "codec" => bytes[20..22].copy_from_slice(&3_u16.to_le_bytes()),
            "empty" => bytes[40..44].copy_from_slice(&0_u32.to_le_bytes()),
            _ => unreachable!(),
        }
        fs::write(&path, bytes).unwrap();
        assert!(inspect_pcm_wav(&path).is_err(), "accepted {scenario}");
    }
}

#[test]
fn accepts_extensible_source_container_but_rejects_noncanonical_delivery() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("extensible.wav");
    samples(&path);
    let mut bytes = fs::read(&path).unwrap();
    bytes[4..8].copy_from_slice(&68_u32.to_le_bytes());
    bytes[16..20].copy_from_slice(&40_u32.to_le_bytes());
    bytes[20..22].copy_from_slice(&0xfffe_u16.to_le_bytes());
    let mut extension = Vec::new();
    extension.extend(22_u16.to_le_bytes());
    extension.extend(16_u16.to_le_bytes());
    extension.extend(3_u32.to_le_bytes());
    extension.extend([1, 0, 0, 0, 0, 0, 16, 0, 128, 0, 0, 170, 0, 56, 155, 113]);
    bytes.splice(36..36, extension);
    fs::write(&path, &bytes).unwrap();
    validate_wav_container(&path).unwrap();
    assert!(inspect_pcm_wav(&path).is_err());
    bytes[44] = 3;
    fs::write(&path, bytes).unwrap();
    assert!(inspect_pcm_wav(&path).is_err());
}
