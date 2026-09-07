use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use forge_core::keyframe_cleanup::cleanup_keyframe_background;

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let input = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("usage: cleanup_keyframe_background <input> <output> <report>")?;
    let output = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("usage: cleanup_keyframe_background <input> <output> <report>")?;
    let report_path = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("usage: cleanup_keyframe_background <input> <output> <report>")?;
    if arguments.next().is_some() {
        return Err("usage: cleanup_keyframe_background <input> <output> <report>".into());
    }
    for path in [&output, &report_path] {
        if path.exists() {
            return Err(format!("refusing to overwrite {}", path.display()).into());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
    }
    let source = image::open(&input)?.to_rgba8();
    let (cleaned, report) = cleanup_keyframe_background(&source);
    cleaned.save(&output)?;
    fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
