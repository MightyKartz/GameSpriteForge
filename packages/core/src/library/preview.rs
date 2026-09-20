//! Offline HTML with byte-preserving PNG playback; no server or executable metadata.
use super::*;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewReport {
    pub index_path: PathBuf,
    pub revisions: usize,
    pub media_files: usize,
    pub issues: Vec<String>,
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub fn create(
    root: &Path,
    references: &[delivery::VersionRef],
    output: &Path,
) -> Result<PreviewReport, CatalogError> {
    if references.is_empty() || references.len() > 100 {
        return Err(invalid("preview requires 1..100 exact revisions"));
    }
    if fs::symlink_metadata(output).is_ok() {
        return Err(invalid(
            "preview output already exists; choose a new directory",
        ));
    }
    let catalog = read_catalog(root)?;
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let staging = tempfile::Builder::new()
        .prefix(".forge-preview-")
        .tempdir_in(parent)?;
    let mut report = PreviewReport {
        index_path: output.join("index.html"),
        revisions: references.len(),
        media_files: 0,
        issues: vec![],
    };
    let mut html = String::from("<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; img-src 'self' file: data:; media-src 'self' file:; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'; object-src 'none'\"><title>Forge resource review</title><style>body{margin:0;background:#101719;color:#e6eeeb;font:16px/1.5 system-ui}header,main{max-width:1400px;margin:auto;padding:28px}h1,h2{line-height:1.15}header p,.muted{color:#b0c5bc}.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(320px,1fr));gap:20px}.card{background:#1a2527;border:1px solid #39514b;border-radius:12px;padding:20px;overflow:hidden}img,video{max-width:100%;max-height:380px;object-fit:contain;background:repeating-conic-gradient(#35403e 0% 25%,#25312e 0% 50%) 50%/20px 20px}audio{width:100%}pre{white-space:pre-wrap;overflow-wrap:anywhere;background:#101719;padding:12px}code,small{overflow-wrap:anywhere}figure{margin:18px 0}figcaption{font-size:13px;color:#b0c5bc}summary{cursor:pointer}a{color:#a4e3bd}.badge{padding:3px 9px;border:1px solid #5b796b;border-radius:20px;display:inline-block;margin:2px}footer{padding:28px;color:#b0c5bc}</style><header><p>FORGE / LOCAL RESOURCE LIBRARY</p><h1>Review exact resource versions</h1><p>Offline copies of existing media. This page does not change selections or record approvals.</p></header><main class=\"grid\">");
    html = html.replace(
        "default-src 'none';",
        &format!(
            "default-src 'none'; script-src 'sha256-{}';",
            crate::animation_preview::script_hash()
        ),
    );
    for (index, reference) in references.iter().enumerate() {
        let asset = read_asset(root, &catalog, &reference.asset_id)?;
        let revision = read_revision(root, &asset, &reference.revision)?;
        let reviews = review::read_reviews(root, &asset, &reference.revision)?;
        let mut suggested_domain = if asset.kind == "audio" {
            "auditory"
        } else if asset.kind == "file" {
            "technical"
        } else {
            "visual"
        };
        html.push_str(&format!(
            "<article class=\"card\"><h2>{}</h2><p>{} · {}</p><code>{}</code>",
            escape(&asset.name),
            escape(&asset.asset_id),
            escape(&asset.kind),
            escape(&reference.revision)
        ));
        for domain in ["technical", "visual", "auditory", "license"] {
            let latest = reviews.iter().rev().find(|r| r.domain == domain);
            html.push_str(&format!(
                "<span class=\"badge\">{}: {}</span>",
                domain,
                latest.map_or("unknown", |r| r.verdict.as_str())
            ));
        }
        html.push_str(&format!(
            "<p>Version status: {} · Purpose: {} · Known installation records: {}</p>",
            intake::disposition(&asset, &reference.revision),
            escape(asset.purpose.as_deref().unwrap_or("unspecified")),
            asset
                .installations
                .iter()
                .filter(|i| i.revision == reference.revision)
                .count()
        ));
        if let Some(legacy) = &revision.legacy {
            if legacy.reviewed_at.is_some() || legacy.license.is_some() {
                html.push_str("<p class=\"muted\">Legacy review/license statements remain historical assertions in the source metadata.</p>");
            }
        }
        match delivery::resolve(root, reference) {
            Ok(resource) => {
                let directory = resource.path.is_dir();
                if directory {
                    if let Ok(pack) = forge_pack::inspect_pack(&resource.path) {
                        if pack.asset_type == "audio" {
                            suggested_domain = "auditory";
                        }
                        html.push_str(&format!("<details><summary>Pack members, animations and audio clips</summary><pre>{}</pre></details>", escape(&serde_json::to_string_pretty(&pack)?)));
                        for (label, path) in [
                            (
                                "Frame timing / layered tracks / audio manifest",
                                &pack.manifest_path,
                            ),
                            ("Recorded technical report", &pack.quality_report_path),
                        ] {
                            if fs::metadata(path).is_ok_and(|m| m.len() <= MAX_METADATA_BYTES) {
                                if let Ok(text) = fs::read_to_string(path) {
                                    html.push_str(&format!("<details><summary>{label}</summary><pre>{}</pre></details>", escape(&text)));
                                }
                            }
                        }
                    }
                }
                let content = revision
                    .content
                    .as_ref()
                    .expect("resolved revision has content");
                let files: Vec<(PathBuf, String)> = if directory {
                    content
                        .files
                        .iter()
                        .map(|f| (resource.path.join(&f.path), f.path.clone()))
                        .collect()
                } else {
                    vec![(
                        resource.path.clone(),
                        resource
                            .path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned(),
                    )]
                };
                let has_previews = files.iter().any(|(_, path)| path.starts_with("previews/"));
                let animation_source = if directory {
                    match crate::animation_preview::read(&resource.path) {
                        Ok(source) => source,
                        Err(error) => {
                            report
                                .issues
                                .push(format!("{} PNG player: {error}", reference.asset_id));
                            html.push_str(
                                "<p>PNG animation unavailable; compatibility media follows.</p>",
                            );
                            None
                        }
                    }
                } else {
                    None
                };
                let mut player_urls = std::collections::BTreeMap::new();
                let mut copied = 0;
                for (source, label) in files {
                    let player_frame = animation_source
                        .as_ref()
                        .is_some_and(|a| a.frames.contains(&source));
                    let extension = source
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    let audio =
                        matches!(extension.as_str(), "wav" | "mp3" | "ogg" | "flac" | "m4a");
                    let image =
                        matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp");
                    let video = matches!(extension.as_str(), "mp4" | "webm");
                    if !(audio || image || video)
                        || (!player_frame && animation_source.is_some() && !audio)
                        || (!player_frame
                            && directory
                            && has_previews
                            && !audio
                            && !label.starts_with("previews/"))
                    {
                        continue;
                    }
                    let relative = format!("media/{index}/{copied}.{extension}");
                    let target = staging.path().join(&relative);
                    fs::create_dir_all(target.parent().unwrap())?;
                    fs::copy(&source, &target)?;
                    let actual = intake::content_at(&target)?;
                    let expected = if directory {
                        content
                            .files
                            .iter()
                            .find(|f| f.path == label)
                            .expect("inventory entry")
                    } else {
                        &content.files[0]
                    };
                    if actual.files[0].sha256 != expected.sha256
                        || actual.files[0].bytes != expected.bytes
                    {
                        return Err(invalid("preview media changed during copy"));
                    }
                    if player_frame {
                        player_urls.insert(source, relative);
                        copied += 1;
                        continue;
                    }
                    let element = if audio {
                        format!("<audio controls preload=\"metadata\" src=\"{relative}\"></audio>")
                    } else if video {
                        format!("<video controls preload=\"metadata\" src=\"{relative}\"></video>")
                    } else {
                        format!(
                            "<img loading=\"lazy\" src=\"{relative}\" alt=\"{}\">",
                            escape(&label)
                        )
                    };
                    html.push_str(&format!(
                        "<figure>{element}<figcaption>{}</figcaption></figure>",
                        escape(&label)
                    ));
                    copied += 1;
                }
                if let Some(animation) = animation_source {
                    let urls = animation
                        .frames
                        .iter()
                        .map(|p| {
                            player_urls
                                .get(p)
                                .cloned()
                                .ok_or_else(|| invalid("animation frame missing from inventory"))
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    let data = serde_json::to_string(
                        &serde_json::json!({"urls":urls,"animations":animation.animations}),
                    )?;
                    html.push_str(&format!("<figure data-forge-animation=\"{}\"><img src=\"{}\" alt=\"PNG animation preview\"><div><label>Animation <select data-animation></select></label> <button data-play disabled>Play</button> <button data-prev aria-label=\"Previous frame\">◀</button> <button data-next aria-label=\"Next frame\">▶</button> <label>Background <select data-background><option value=\"dark\">Dark</option><option value=\"light\">Light</option><option value=\"checkerboard\">Checkerboard</option></select></label></div><input type=\"range\" min=\"0\" value=\"0\" step=\"1\" aria-label=\"Animation frame\"><output></output><figcaption>Original PNG frames · native frame order and durations · normal alpha composition. Use Godot for engine blend modes.</figcaption></figure>", escape(&data), escape(&urls[0])));
                }
                if intake::content_at(&resource.path)? != *content {
                    return Err(invalid("resource changed while preparing preview"));
                }
                report.media_files += copied;
                if copied == 0 {
                    html.push_str("<p>No browser preview is available for this format. Use the source application's viewer.</p>");
                }
                html.push_str("<p class=\"muted\">Media copies preserve source bytes. PNG playback follows Pack timing; browser scheduling may vary. GIF has limited colors and binary transparency. Audio/video decoding depends on this browser.</p>");
            }
            Err(error) => {
                let message = format!("{}: {error}", reference.asset_id);
                html.push_str(&format!(
                    "<p>Media unavailable: {}</p>",
                    escape(&error.to_string())
                ));
                report.issues.push(message);
            }
        }
        for review in reviews {
            let evidence = review::verify_evidence(root, &review);
            if let Err(error) = &evidence {
                report.issues.push(format!(
                    "{}/{} review evidence: {error}",
                    reference.asset_id, review.domain
                ));
            }
            let intact = evidence.is_ok();
            html.push_str(&format!("<details><summary>{}: {} · {}</summary><p>{}</p><p>Reviewer: {} · Evidence: {} ({})</p></details>", escape(&review.domain), escape(&review.verdict), review.recorded_at, escape(&review.statement), escape(&review.reviewer), review.evidence_sha256, if intact {"retained and verified"} else {"unavailable or changed"}));
        }
        let command = format!("forge asset review --project {} --id {} --revision {} --domain {suggested_domain} --verdict needs_review --reviewer YOUR_NAME --statement YOUR_NOTES --evidence /path/to/evidence", shell_quote(&fs::canonicalize(root)?.to_string_lossy()), shell_quote(&reference.asset_id), reference.revision);
        let powershell = format!("forge asset review --project '{}' --id '{}' --revision {} --domain {suggested_domain} --verdict needs_review --reviewer YOUR_NAME --statement YOUR_NOTES --evidence C:/path/to/evidence", fs::canonicalize(root)?.to_string_lossy().replace('\'', "''"), reference.asset_id.replace('\'', "''"), reference.revision);
        html.push_str(&format!("<details><summary>Record a review with CLI (PowerShell)</summary><pre>{}</pre></details>", escape(&powershell)));
        html.push_str(&format!("<details><summary>Record a review with CLI (POSIX shell)</summary><pre>{}</pre></details><details><summary>Source and processing metadata</summary><pre>{}</pre></details></article>", escape(&command), escape(&serde_json::to_string_pretty(&revision)?)));
    }
    html.push_str("</main><footer>Forge · File availability, technical checks, visual review, listening review and license statements are separate evidence.</footer>");
    html.push_str("<script type=\"text/javascript\">");
    html.push_str(crate::animation_preview::PLAYER_SCRIPT);
    html.push_str("</script></html>");
    fs::write(staging.path().join("index.html"), html)?;
    fs::write(
        staging.path().join("preview.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    // Reserve the requested output without overwriting a concurrent directory.
    fs::create_dir(output)?;
    for entry in fs::read_dir(staging.path())? {
        let entry = entry?;
        fs::rename(entry.path(), output.join(entry.file_name()))?;
    }
    Ok(report)
}
