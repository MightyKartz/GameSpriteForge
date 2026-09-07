use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use forge_core::keyframe_cleanup::assess_alpha_edge_halo;

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let input = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("usage: assess_alpha_edge <input> <report> <transparency-origin>")?;
    let report_path = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("usage: assess_alpha_edge <input> <report> <transparency-origin>")?;
    let origin = arguments
        .next()
        .ok_or("usage: assess_alpha_edge <input> <report> <transparency-origin>")?
        .to_string_lossy()
        .into_owned();
    if arguments.next().is_some() {
        return Err("usage: assess_alpha_edge <input> <report> <transparency-origin>".into());
    }
    if report_path.exists() {
        return Err(format!("refusing to overwrite {}", report_path.display()).into());
    }
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let image = image::open(&input)?.to_rgba8();
    let report = assess_alpha_edge_halo(&image, &origin);
    fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
