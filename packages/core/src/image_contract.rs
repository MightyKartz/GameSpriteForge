//! Read-only verification of a consumer's PNG lock and explicitly selected directories.
//! This does not verify other game contracts, installation receipts or visual quality.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

const MAX_LOCK_BYTES: u64 = 16 * 1024 * 1024;
const MAX_PNG_BYTES: u64 = 128 * 1024 * 1024;
const MAX_PIXELS: u64 = 32 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Contract {
    schema_version: Value,
    images: Vec<LockedImage>,
    #[serde(default)]
    expected_counts: Option<ExpectedCounts>,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

#[derive(Deserialize)]
struct ExpectedCounts {
    #[serde(default)]
    images: Option<usize>,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LockedImage {
    id: String,
    path: String,
    sha256: String,
    width: u32,
    height: u32,
    requires_alpha: bool,
    #[serde(default = "default_clear_border")]
    requires_clear_border: bool,
    #[serde(default)]
    requires_opaque: bool,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

fn default_clear_border() -> bool {
    true
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageVerification {
    pub schema_version: &'static str,
    pub verified: bool,
    pub scope: &'static str,
    pub root: PathBuf,
    pub lock_path: PathBuf,
    pub lock_sha256: String,
    pub scan_roots: Vec<String>,
    pub excluded_directories: [&'static str; 2],
    pub expected_image_count: Option<usize>,
    pub count_check: &'static str,
    pub locked_image_count: usize,
    pub scanned_image_count: usize,
    pub checks: Vec<ImageCheck>,
    pub issues: Vec<ImageIssue>,
    pub not_checked_fields: BTreeSet<String>,
    pub visual_review: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageCheck {
    pub id: String,
    pub path: String,
    pub verified: bool,
    pub actual: Option<ImageFacts>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageFacts {
    pub sha256: String,
    pub width: u32,
    pub height: u32,
    pub png_bit_depth: u8,
    pub png_color_type: u8,
    pub transparent_pixels: u64,
    pub visible_pixels: u64,
    pub partial_alpha_pixels: u64,
    pub visible_border_pixels: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageIssue {
    pub code: &'static str,
    pub path: Option<String>,
    pub message: String,
}

impl ImageVerification {
    fn issue(&mut self, code: &'static str, path: Option<&str>, message: impl Into<String>) {
        self.issues.push(ImageIssue {
            code,
            path: path.map(str::to_owned),
            message: message.into(),
        });
    }
}

/// Relative lock paths and every scan directory resolve against `root`, never cwd.
/// Invalid inputs/unsafe paths return Err; image and membership drift accumulates in issues.
pub fn verify_images(
    root: &Path,
    lock_path: &Path,
    scan_roots: &[PathBuf],
) -> Result<ImageVerification, String> {
    let root = fs::canonicalize(root).map_err(|e| format!("cannot open project root: {e}"))?;
    if !root.is_dir() || scan_roots.is_empty() {
        return Err("root must be a directory and at least one --scan is required".into());
    }
    let lock_path = if lock_path.is_absolute() {
        lock_path.to_path_buf()
    } else {
        root.join(lock_path)
    };
    let bytes = read_limited(&lock_path, MAX_LOCK_BYTES)?;
    // PowerShell-generated UTF-8 JSON can include a BOM.
    let json_bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&bytes);
    let raw: Value =
        serde_json::from_slice(json_bytes).map_err(|e| format!("invalid image lock: {e}"))?;
    if let Some(counts) = raw.get("expectedCounts") {
        if !counts.is_object() || counts.get("images").is_some_and(|n| n.as_u64().is_none()) {
            return Err("expectedCounts must be an object with an optional nonnegative integer images count".into());
        }
    }
    let contract: Contract =
        serde_json::from_slice(json_bytes).map_err(|e| format!("invalid image lock: {e}"))?;
    if contract.schema_version != Value::from(1) && contract.schema_version != Value::from("1") {
        return Err("image lock schemaVersion must be 1 or \"1\"".into());
    }
    let mut scans = BTreeSet::new();
    for scan in scan_roots {
        let text = scan.to_str().ok_or("scan path must be UTF-8")?;
        scans.insert(if text == "." {
            String::new()
        } else {
            normalize_relative(text)?
        });
    }
    if scans.iter().any(|scan| excluded(scan)) {
        return Err("scan directories may not select .godot or .git metadata".into());
    }
    let mut ids = BTreeSet::new();
    let mut locked = BTreeMap::new();
    let mut portable_paths = BTreeSet::new();
    let mut unchecked: BTreeSet<_> = contract.extra.keys().cloned().collect();
    if let Some(counts) = &contract.expected_counts {
        unchecked.extend(
            counts
                .extra
                .keys()
                .map(|key| format!("expectedCounts.{key}")),
        );
    }
    for mut item in contract.images {
        item.path = normalize_relative(&item.path)?;
        if item.id.trim().is_empty() || !ids.insert(item.id.to_lowercase()) {
            return Err(format!("empty or duplicate image id: {}", item.id));
        }
        if !portable_paths.insert(item.path.to_lowercase()) {
            return Err(format!(
                "duplicate image path (case-insensitive): {}",
                item.path
            ));
        }
        if !is_png(Path::new(&item.path))
            || item.width == 0
            || item.height == 0
            || item.sha256.len() != 64
            || !item.sha256.bytes().all(|c| c.is_ascii_hexdigit())
            || (item.requires_alpha && item.requires_opaque)
        {
            return Err(format!(
                "invalid PNG dimensions, SHA256 or alpha contract: {}",
                item.id
            ));
        }
        if excluded(&item.path)
            || !scans
                .iter()
                .any(|scan| scan.is_empty() || Path::new(&item.path).starts_with(scan))
        {
            return Err(format!(
                "locked image is outside scanned directories: {}",
                item.path
            ));
        }
        unchecked.extend(item.extra.keys().map(|key| format!("images[].{key}")));
        locked.insert(item.path.clone(), item);
    }
    let expected = contract.expected_counts.and_then(|c| c.images);
    let mut report = ImageVerification {
        schema_version: "1",
        verified: false,
        scope: "image_contract_only",
        root: root.clone(),
        lock_path,
        lock_sha256: format!("{:x}", Sha256::digest(&bytes)),
        scan_roots: scans
            .iter()
            .map(|s| if s.is_empty() { ".".into() } else { s.clone() })
            .collect(),
        excluded_directories: [".godot", ".git"],
        expected_image_count: expected,
        count_check: if expected.is_some() {
            "matched"
        } else {
            "not_declared"
        },
        locked_image_count: locked.len(),
        scanned_image_count: 0,
        checks: Vec::new(),
        issues: Vec::new(),
        not_checked_fields: unchecked,
        visual_review: "not_evaluated",
    };
    if expected.is_some_and(|count| count != locked.len()) {
        report.count_check = "mismatch";
        report.issue(
            "image_count_mismatch",
            None,
            format!(
                "expectedCounts.images is {}, but the lock contains {} images",
                expected.unwrap(),
                locked.len()
            ),
        );
    }
    let mut actual = BTreeSet::new();
    for scan in &scans {
        let path = contained_path(&root, scan)?;
        if !path.is_dir() {
            return Err(format!(
                "scan directory is missing or not a directory: {}",
                path.display()
            ));
        }
        verify_scan_spelling(&root, scan)?;
        scan_pngs(&root, &path, &mut actual)?;
    }
    report.scanned_image_count = actual.len();
    // Case aliases of filenames have different semantics across hosts. Refuse an ambiguous set.
    let mut actual_folded = BTreeSet::new();
    for path in &actual {
        if !actual_folded.insert(path.to_lowercase()) {
            return Err(format!("scanned PNG paths collide ignoring case: {path}"));
        }
        if !locked.contains_key(path) {
            report.issue(
                "unexpected_image",
                Some(path),
                "PNG is present on disk but absent from the lock",
            );
        }
    }
    for (path, item) in locked {
        let start = report.issues.len();
        let facts = if !actual.contains(&path) {
            // Check ancestors even for missing files; never treat a link escape as ordinary drift.
            contained_path(&root, &path)?;
            report.issue(
                "missing_image",
                Some(&path),
                "locked PNG is absent from the scanned file set (paths are case-sensitive)",
            );
            None
        } else {
            let absolute = contained_path(&root, &path)?;
            match inspect_png(&absolute) {
                Err(message) => {
                    report.issue("invalid_image", Some(&path), message);
                    None
                }
                Ok(facts) => {
                    if !facts.sha256.eq_ignore_ascii_case(&item.sha256) {
                        report.issue(
                            "sha256_mismatch",
                            Some(&path),
                            "PNG bytes differ from the locked SHA256",
                        );
                    }
                    if (facts.width, facts.height) != (item.width, item.height) {
                        report.issue(
                            "dimensions_mismatch",
                            Some(&path),
                            format!(
                                "expected {}x{}, found {}x{}",
                                item.width, item.height, facts.width, facts.height
                            ),
                        );
                    }
                    if item.requires_alpha {
                        if facts.png_color_type != 6
                            || facts.transparent_pixels == 0
                            || facts.visible_pixels == 0
                        {
                            report.issue("alpha_required", Some(&path), "requiresAlpha needs an RGBA PNG with both transparent and visible pixels");
                        }
                        if item.requires_clear_border && facts.visible_border_pixels != 0 {
                            report.issue("clear_border_required", Some(&path), "visible pixels touch the border; requiresClearBorder defaults to true for alpha assets");
                        }
                    }
                    if item.requires_opaque
                        && (facts.transparent_pixels != 0 || facts.partial_alpha_pixels != 0)
                    {
                        report.issue(
                            "opaque_required",
                            Some(&path),
                            "requiresOpaque needs every pixel to be fully opaque",
                        );
                    }
                    Some(facts)
                }
            }
        };
        report.checks.push(ImageCheck {
            id: item.id,
            path,
            verified: report.issues.len() == start,
            actual: facts,
        });
    }
    report.verified = report.issues.is_empty();
    Ok(report)
}

fn verify_scan_spelling(root: &Path, relative: &str) -> Result<(), String> {
    let mut parent = root.to_path_buf();
    for part in relative.split('/').filter(|part| !part.is_empty()) {
        let mut found = false;
        for entry in fs::read_dir(&parent).map_err(|e| e.to_string())? {
            if entry.map_err(|e| e.to_string())?.file_name().to_str() == Some(part) {
                found = true;
                break;
            }
        }
        if !found {
            return Err(format!(
                "scan path spelling differs from the filesystem: {relative}"
            ));
        }
        parent.push(part);
    }
    Ok(())
}

fn normalize_relative(text: &str) -> Result<String, String> {
    let text = text.replace('\\', "/");
    if text.is_empty()
        || text.contains(':')
        || text.contains('\0')
        || text
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(format!("expected a safe project-relative path: {text:?}"));
    }
    Ok(text)
}

fn excluded(path: &str) -> bool {
    path.split('/')
        .any(|part| part.eq_ignore_ascii_case(".godot") || part.eq_ignore_ascii_case(".git"))
}

fn contained_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let mut path = root.to_path_buf();
    for part in relative.split('/').filter(|p| !p.is_empty()) {
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if is_link(&metadata)
                    || !fs::canonicalize(&path)
                        .map_err(|e| e.to_string())?
                        .starts_with(root)
                {
                    return Err(format!(
                        "image paths may not traverse symbolic links or escape root: {}",
                        path.display()
                    ));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("cannot inspect {}: {e}", path.display())),
        }
    }
    Ok(path)
}

fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // Junctions/reparse points can form cycles even when their destination is inside root.
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn scan_pngs(root: &Path, directory: &Path, paths: &mut BTreeSet<String>) -> Result<(), String> {
    for entry in
        fs::read_dir(directory).map_err(|e| format!("cannot scan {}: {e}", directory.display()))?
    {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("scanned paths must be UTF-8")?
            .replace('\\', "/");
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        if file_type.is_dir() && excluded(&relative) {
            continue;
        }
        // Reuse ancestor/containment checks for Windows junctions as well as file symlinks.
        contained_path(root, &relative)?;
        if file_type.is_symlink() {
            return Err(format!("scan may not traverse symbolic links: {relative}"));
        }
        if file_type.is_dir() {
            scan_pngs(root, &path, paths)?;
        } else if file_type.is_file() && is_png(&path) {
            paths.insert(relative);
        }
    }
    Ok(())
}

fn is_png(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
}

fn read_limited(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err(format!("expected a regular file: {}", path.display()));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err(format!(
            "file exceeds {limit}-byte inspection limit: {}",
            path.display()
        ));
    }
    Ok(bytes)
}

fn inspect_png(path: &Path) -> Result<ImageFacts, String> {
    let bytes = read_limited(path, MAX_PNG_BYTES)?;
    if bytes.len() < 33 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return Err("expected a valid PNG signature and IHDR".into());
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(format!("PNG exceeds {MAX_PIXELS}-pixel inspection limit"));
    }
    // Pixel decoding can stop before the final chunk. Validate through IEND, including its CRC.
    png::Decoder::new(std::io::Cursor::new(&bytes))
        .read_info()
        .and_then(|mut reader| reader.finish())
        .map_err(|e| format!("invalid PNG container: {e}"))?;
    let decoded = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
        .map_err(|e| format!("cannot decode PNG: {e}"))?;
    let mut result = ImageFacts {
        sha256: format!("{:x}", Sha256::digest(&bytes)),
        width: decoded.width(),
        height: decoded.height(),
        png_bit_depth: bytes[24],
        png_color_type: bytes[25],
        transparent_pixels: 0,
        visible_pixels: 0,
        partial_alpha_pixels: 0,
        visible_border_pixels: 0,
    };
    // Preserve 16-bit alpha: low nonzero values must not round to transparent for a border check.
    for (x, y, pixel) in decoded.to_rgba16().enumerate_pixels() {
        if pixel[3] == 0 {
            result.transparent_pixels += 1;
        } else {
            result.visible_pixels += 1;
            if pixel[3] < u16::MAX {
                result.partial_alpha_pixels += 1;
            }
            if x == 0 || y == 0 || x + 1 == result.width || y + 1 == result.height {
                result.visible_border_pixels += 1;
            }
        }
    }
    Ok(result)
}
