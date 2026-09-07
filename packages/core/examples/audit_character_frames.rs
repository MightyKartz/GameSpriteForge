use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use forge_core::motion_semantics::assess_character_motion_semantics;

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let animation = arguments
        .next()
        .and_then(|value| value.into_string().ok())
        .ok_or("usage: audit_character_frames <animation> <frames-dir> [report.json]")?;
    let frames_dir = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("usage: audit_character_frames <animation> <frames-dir> [report.json]")?;
    let report_path = arguments.next().map(PathBuf::from);

    let mut frame_paths = fs::read_dir(&frames_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
        })
        .collect::<Vec<_>>();
    frame_paths.sort();
    if frame_paths.is_empty() {
        return Err(format!("no PNG frames found in {}", frames_dir.display()).into());
    }

    let frames = frame_paths
        .iter()
        .map(|path| Ok(image::open(path)?.to_rgba8()))
        .collect::<Result<Vec<_>, image::ImageError>>()?;
    let animations = BTreeMap::from([(animation, frames)]);
    let report = assess_character_motion_semantics(&animations);
    let serialized = serde_json::to_string_pretty(&report)? + "\n";
    if let Some(path) = report_path {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, &serialized)?;
    }
    print!("{serialized}");
    Ok(())
}
