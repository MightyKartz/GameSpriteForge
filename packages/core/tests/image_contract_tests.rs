use std::{fs, path::Path};

use forge_core::image_contract::{verify_images, ImageVerification};
use image::{ImageBuffer, Rgb, RgbImage, Rgba, RgbaImage};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn fixture(root: &Path) -> Value {
    fs::create_dir_all(root.join("game/items")).unwrap();
    let mut image = RgbaImage::new(4, 4);
    image.put_pixel(1, 1, Rgba([255, 100, 0, 255]));
    image.put_pixel(2, 1, Rgba([255, 100, 0, 127]));
    image.save(root.join("game/items/portrait.png")).unwrap();
    json!({
        "schemaVersion":1,
        "images":[{
            "id":"portrait", "path":"game/items/portrait.png", "width":4, "height":4,
            "sha256":hash(root), "requiresAlpha":true
        }],
        "expectedCounts":{"images":1}
    })
}

fn hash(root: &Path) -> String {
    format!(
        "{:x}",
        Sha256::digest(fs::read(root.join("game/items/portrait.png")).unwrap())
    )
}

fn run(root: &Path, lock: &Value) -> Result<ImageVerification, String> {
    fs::write(root.join("lock.json"), serde_json::to_vec(lock).unwrap()).unwrap();
    verify_images(root, Path::new("lock.json"), &["game".into()])
}

fn codes(report: &ImageVerification) -> Vec<&str> {
    report.issues.iter().map(|issue| issue.code).collect()
}

#[test]
fn verifies_exact_set_and_pixels_without_altering_inputs() {
    let root = tempfile::tempdir().unwrap();
    let mut lock = fixture(root.path());
    lock["images"][0]["sourceManifest"] = json!("private-source.json");
    lock["fonts"] = json!([]);
    lock["expectedCounts"]["rigs"] = json!(2);
    let before = fs::read(root.path().join("game/items/portrait.png")).unwrap();
    let report = run(root.path(), &lock).unwrap();
    assert!(report.verified, "{report:?}");
    assert_eq!(report.scope, "image_contract_only");
    assert_eq!(report.count_check, "matched");
    let actual = report.checks[0].actual.as_ref().unwrap();
    assert_eq!(actual.transparent_pixels, 14);
    assert_eq!(actual.partial_alpha_pixels, 1);
    assert_eq!(actual.visible_border_pixels, 0);
    assert!(report
        .not_checked_fields
        .contains("images[].sourceManifest"));
    assert!(report.not_checked_fields.contains("fonts"));
    assert!(report.not_checked_fields.contains("expectedCounts.rigs"));
    assert_eq!(
        before,
        fs::read(root.path().join("game/items/portrait.png")).unwrap()
    );
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
}

#[test]
fn aggregates_hash_dimensions_count_missing_and_unlocked_file_failures() {
    let root = tempfile::tempdir().unwrap();
    let mut lock = fixture(root.path());
    lock["images"][0]["sha256"] = json!("0".repeat(64));
    lock["images"][0]["width"] = json!(5);
    let mut missing = lock["images"][0].clone();
    missing["id"] = json!("missing");
    missing["path"] = json!("game/items/missing.png");
    lock["images"].as_array_mut().unwrap().push(missing);
    fs::copy(
        root.path().join("game/items/portrait.png"),
        root.path().join("game/extra.PNG"),
    )
    .unwrap();
    let report = run(root.path(), &lock).unwrap();
    assert!(!report.verified);
    for expected in [
        "sha256_mismatch",
        "dimensions_mismatch",
        "image_count_mismatch",
        "missing_image",
        "unexpected_image",
    ] {
        assert!(
            codes(&report).contains(&expected),
            "missing {expected}: {report:?}"
        );
    }
    assert_eq!(report.checks.len(), 2);
    assert_eq!(report.scanned_image_count, 2);
}

#[test]
fn honors_border_opt_out_and_distinguishes_transparent_from_opaque_contracts() {
    let root = tempfile::tempdir().unwrap();
    let mut lock = fixture(root.path());
    let path = root.path().join("game/items/portrait.png");
    let mut pixels = RgbaImage::new(4, 4);
    pixels.put_pixel(0, 1, Rgba([20, 30, 40, 1]));
    pixels.save(&path).unwrap();
    lock["images"][0]["sha256"] = json!(hash(root.path()));
    assert_eq!(
        codes(&run(root.path(), &lock).unwrap()),
        ["clear_border_required"]
    );
    lock["images"][0]["requiresClearBorder"] = json!(false);
    assert!(run(root.path(), &lock).unwrap().verified);
    lock["images"][0]["requiresAlpha"] = json!(false);
    lock["images"][0]["requiresOpaque"] = json!(true);
    assert_eq!(
        codes(&run(root.path(), &lock).unwrap()),
        ["opaque_required"]
    );
    RgbImage::from_pixel(4, 4, Rgb([5, 6, 7]))
        .save(&path)
        .unwrap();
    lock["images"][0]["sha256"] = json!(hash(root.path()));
    assert!(run(root.path(), &lock).unwrap().verified);
}

#[test]
fn rejects_empty_or_fully_opaque_alpha_images() {
    let root = tempfile::tempdir().unwrap();
    let mut lock = fixture(root.path());
    for alpha in [0, 255] {
        RgbaImage::from_pixel(4, 4, Rgba([20, 30, 40, alpha]))
            .save(root.path().join("game/items/portrait.png"))
            .unwrap();
        lock["images"][0]["sha256"] = json!(hash(root.path()));
        assert!(codes(&run(root.path(), &lock).unwrap()).contains(&"alpha_required"));
    }
}

#[test]
fn does_not_round_away_low_sixteen_bit_alpha_at_the_border() {
    let root = tempfile::tempdir().unwrap();
    let mut lock = fixture(root.path());
    let mut image: ImageBuffer<Rgba<u16>, Vec<u16>> = ImageBuffer::new(4, 4);
    image.put_pixel(0, 1, Rgba([10, 20, 30, 1]));
    image
        .save(root.path().join("game/items/portrait.png"))
        .unwrap();
    lock["images"][0]["sha256"] = json!(hash(root.path()));
    let report = run(root.path(), &lock).unwrap();
    assert_eq!(codes(&report), ["clear_border_required"]);
    let facts = report.checks[0].actual.as_ref().unwrap();
    assert_eq!(facts.png_bit_depth, 16);
    assert_eq!(facts.visible_border_pixels, 1);
    assert_eq!(facts.visible_pixels, 1);
}

#[test]
fn accepts_bom_string_schema_and_optional_count_and_deduplicates_overlapping_scans() {
    let root = tempfile::tempdir().unwrap();
    let mut lock = fixture(root.path());
    lock["schemaVersion"] = json!("1");
    lock.as_object_mut().unwrap().remove("expectedCounts");
    fs::create_dir_all(root.path().join("game/.godot/imported")).unwrap();
    fs::write(
        root.path().join("game/.godot/imported/cache.png"),
        "not a PNG",
    )
    .unwrap();
    let mut bytes = vec![0xef, 0xbb, 0xbf];
    bytes.extend(serde_json::to_vec(&lock).unwrap());
    fs::write(root.path().join("lock.json"), &bytes).unwrap();
    let report = verify_images(
        root.path(),
        Path::new("lock.json"),
        &["game".into(), "game/items".into()],
    )
    .unwrap();
    assert!(report.verified);
    assert_eq!(report.scanned_image_count, 1);
    assert_eq!(report.expected_image_count, None);
    assert_eq!(report.count_check, "not_declared");
    assert_eq!(report.lock_sha256, format!("{:x}", Sha256::digest(&bytes)));
}

#[test]
fn rejects_malformed_known_fields_and_duplicate_portable_identity() {
    let root = tempfile::tempdir().unwrap();
    let lock = fixture(root.path());
    for (field, invalid) in [
        ("width", json!(0)),
        ("height", json!(1.5)),
        ("requiresAlpha", json!("true")),
        ("sha256", json!("g".repeat(64))),
        ("requiresOpaque", json!(true)),
    ] {
        let mut broken = lock.clone();
        broken["images"][0][field] = invalid;
        assert!(
            run(root.path(), &broken).is_err(),
            "accepted invalid {field}"
        );
    }
    for invalid in [Value::Null, json!(-1), json!("1")] {
        let mut broken = lock.clone();
        broken["expectedCounts"]["images"] = invalid;
        assert!(run(root.path(), &broken).is_err());
    }
    let mut duplicate = lock.clone();
    let mut other = duplicate["images"][0].clone();
    other["id"] = json!("second");
    other["path"] = json!("game\\items\\PORTRAIT.png");
    duplicate["images"].as_array_mut().unwrap().push(other);
    assert!(run(root.path(), &duplicate)
        .unwrap_err()
        .contains("duplicate image path"));
    duplicate["images"][1]["path"] = json!("game/items/other.png");
    duplicate["images"][1]["id"] = json!("PORTRAIT");
    assert!(run(root.path(), &duplicate)
        .unwrap_err()
        .contains("duplicate image id"));
}

#[test]
fn rejects_escaping_and_unscanned_paths_and_missing_scan_directories() {
    let root = tempfile::tempdir().unwrap();
    let lock = fixture(root.path());
    for path in [
        "../outside.png",
        "/outside.png",
        "C:\\outside.png",
        "game/../outside.png",
        "game//x.png",
        "other/x.png",
        "game/.godot/cache.png",
    ] {
        let mut broken = lock.clone();
        broken["images"][0]["path"] = json!(path);
        assert!(run(root.path(), &broken).is_err(), "accepted {path}");
    }
    run(root.path(), &lock).unwrap();
    assert!(verify_images(root.path(), Path::new("lock.json"), &[]).is_err());
    assert!(verify_images(
        root.path(),
        Path::new("lock.json"),
        &["game".into(), "missing".into()]
    )
    .is_err());
    assert!(verify_images(root.path(), Path::new("lock.json"), &["game/.godot".into()]).is_err());
}

#[test]
fn corrupt_png_reports_issue_instead_of_accepting_matching_bytes() {
    let root = tempfile::tempdir().unwrap();
    let mut lock = fixture(root.path());
    fs::write(root.path().join("game/items/portrait.png"), b"not an image").unwrap();
    lock["images"][0]["sha256"] = json!(hash(root.path()));
    let report = run(root.path(), &lock).unwrap();
    assert_eq!(codes(&report), ["invalid_image"]);
    assert!(!report.checks[0].verified);
}

#[test]
fn rejects_truncated_and_corrupt_iend_even_when_pixels_and_locked_hash_match() {
    let root = tempfile::tempdir().unwrap();
    let mut lock = fixture(root.path());
    let path = root.path().join("game/items/portrait.png");
    let valid = fs::read(&path).unwrap();
    let truncated = valid[..valid.len() - 4].to_vec();
    let mut corrupt = valid.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    for bytes in [truncated, corrupt] {
        fs::write(&path, bytes).unwrap();
        lock["images"][0]["sha256"] = json!(hash(root.path()));
        assert_eq!(codes(&run(root.path(), &lock).unwrap()), ["invalid_image"]);
    }
}

#[cfg(unix)]
#[test]
fn rejects_symlinks_even_inside_root_and_directory_cycles() {
    let root = tempfile::tempdir().unwrap();
    let lock = fixture(root.path());
    std::os::unix::fs::symlink(root.path().join("game"), root.path().join("game/cycle")).unwrap();
    assert!(run(root.path(), &lock)
        .unwrap_err()
        .contains("symbolic links"));
}

#[cfg(windows)]
#[test]
fn rejects_scan_root_case_alias_even_when_the_lock_uses_the_same_alias() {
    let root = tempfile::tempdir().unwrap();
    let mut lock = fixture(root.path());
    lock["images"][0]["path"] = json!("GAME/items/portrait.png");
    fs::write(
        root.path().join("lock.json"),
        serde_json::to_vec(&lock).unwrap(),
    )
    .unwrap();
    assert!(
        verify_images(root.path(), Path::new("lock.json"), &["GAME".into()])
            .unwrap_err()
            .contains("spelling")
    );
}

#[cfg(windows)]
#[test]
fn rejects_windows_junction_cycles_without_following_them() {
    let root = tempfile::tempdir().unwrap();
    let lock = fixture(root.path());
    let link = root.path().join("game").join("cycle");
    // mklink /J creates directory junctions without the symlink privilege.
    let output = std::process::Command::new("cmd")
        .args(["/d", "/c", "mklink", "/J"])
        .arg(&link)
        .arg(root.path().join("game"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result = run(root.path(), &lock);
    // Remove just the junction before TempDir cleanup; never recurse through it.
    fs::remove_dir(&link).unwrap();
    assert!(result.unwrap_err().contains("symbolic links"));
}
