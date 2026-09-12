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
