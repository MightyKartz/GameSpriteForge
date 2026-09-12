use forge_core::content_digest::{
    digest_inventory, directory_inventory, legacy_directory_digest, validate_relative, ContentFile,
    LegacyPathStyle, CONTENT_ALGORITHM,
};
use serde_json::Value;
use std::fs;

#[test]
fn shared_vector_preserves_both_legacy_styles_and_one_content_identity() {
    let vector: Value = serde_json::from_str(include_str!(
        "../../../examples/asset-library/content-digest-vector.json"
    ))
    .unwrap();
    let temp = tempfile::tempdir().unwrap();
    for file in vector["files"].as_array().unwrap().iter().rev() {
        let path = temp.path().join(file["path"].as_str().unwrap());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let hex = file["contentHex"].as_str().unwrap();
        let bytes: Vec<_> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        fs::write(path, bytes).unwrap();
    }
    let inventory = directory_inventory(temp.path()).unwrap();
    assert_eq!(inventory.algorithm, CONTENT_ALGORITHM);
    assert_eq!(inventory.sha256, vector["sha256"]);
    assert_eq!(
        legacy_directory_digest(temp.path(), LegacyPathStyle::Posix).unwrap(),
        vector["legacyV2"]["posix"]
    );
    assert_eq!(
        legacy_directory_digest(temp.path(), LegacyPathStyle::Windows).unwrap(),
        vector["legacyV2"]["windows"]
    );
    let native = if cfg!(windows) { "windows" } else { "posix" };
    assert_eq!(
        forge_core::delivery::directory_sha256(temp.path()).unwrap(),
        vector["legacyV2"][native]
    );

    let mut reversed = inventory.files.clone();
    reversed.reverse();
    assert_eq!(digest_inventory(&reversed).unwrap(), inventory.sha256);
    fs::write(temp.path().join("a.txt"), b"changed").unwrap();
    assert_ne!(
        directory_inventory(temp.path()).unwrap().sha256,
        inventory.sha256
    );
}

fn file(path: &str) -> ContentFile {
    ContentFile {
        path: path.into(),
        bytes: 0,
        sha256: "0".repeat(64),
    }
}

#[test]
fn rejects_ambiguous_or_nonportable_inventories() {
    for path in [
        "",
        "../file",
        "/root",
        "a//b",
        "a/./b",
        "a\\b",
        "C:/file",
        "a/NUL.wav",
        "CON.txt",
        "a.",
        "LPT1",
        "a?b",
        "a\nb",
    ] {
        assert!(validate_relative(path).is_err(), "{path:?}");
    }
    for files in [
        vec![file("a"), file("a")],
        vec![file("A/one"), file("a/two")],
        vec![file("a"), file("a/b")],
    ] {
        assert!(digest_inventory(&files).is_err());
    }
    assert!(digest_inventory(&[ContentFile {
        sha256: "bad".into(),
        ..file("ok")
    }])
    .is_err());
}

#[cfg(unix)]
#[test]
fn rejects_symlinked_files_and_roots() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    fs::create_dir(&root).unwrap();
    fs::write(temp.path().join("outside"), b"outside").unwrap();
    symlink(temp.path().join("outside"), root.join("link")).unwrap();
    assert!(directory_inventory(&root).is_err());
    symlink(&root, temp.path().join("root-link")).unwrap();
    assert!(directory_inventory(&temp.path().join("root-link")).is_err());
}

#[cfg(windows)]
#[test]
fn windows_junctions_are_rejected_by_inventory_scan_and_catalog_writes() {
    use std::path::Path;
    fn junction(link: &Path, target: &Path) {
        let output = std::process::Command::new("cmd.exe")
            .args(["/D", "/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "junction creation failed: {output:?}"
        );
    }
    // Junctions need no symlink privilege; this exercises real reparse points
    // rather than skipping a test when developer-mode symlinks are unavailable.
    let temp = tempfile::tempdir().unwrap();
    let outside = temp.path().join("outside with spaces");
    let root = temp.path().join("sources with spaces");
    fs::create_dir(&outside).unwrap();
    fs::create_dir(&root).unwrap();
    fs::write(outside.join("protected.bin"), b"unchanged outside bytes").unwrap();
    let link = root.join("junction with spaces");
    junction(&link, &outside);
    assert!(directory_inventory(&link).is_err());
    assert!(directory_inventory(&root).is_err());
    let scan = forge_core::library::intake::scan(&root).unwrap();
    assert!(scan.batch.items.is_empty());
    assert_eq!(scan.issues.len(), 1);
    assert!(scan.issues[0].message.contains("link skipped"));
    fs::remove_dir(&link).unwrap();

    let library = temp.path().join("library with spaces");
    fs::create_dir(&library).unwrap();
    junction(&library.join(".forge"), &outside);
    assert!(matches!(
        forge_core::library::initialize(&library, "Redirected"),
        Err(forge_core::catalog::CatalogError::Symlink)
    ));
    fs::remove_dir(library.join(".forge")).unwrap();
    assert_eq!(
        fs::read(outside.join("protected.bin")).unwrap(),
        b"unchanged outside bytes"
    );
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 1);
}
