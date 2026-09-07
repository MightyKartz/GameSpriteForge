use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use forge_core::keyframe_cleanup::{assess_alpha_edge_halo, matte_checkerboard_sheet_background};

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let input = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("usage: matte_checkerboard_sheet <input> <output> <matting-report> <halo-report>")?;
    let output = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("usage: matte_checkerboard_sheet <input> <output> <matting-report> <halo-report>")?;
    let matting_report = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("usage: matte_checkerboard_sheet <input> <output> <matting-report> <halo-report>")?;
    let halo_report = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("usage: matte_checkerboard_sheet <input> <output> <matting-report> <halo-report>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: matte_checkerboard_sheet <input> <output> <matting-report> <halo-report>"
                .into(),
        );
    }
    for path in [&output, &matting_report, &halo_report] {
        if path.exists() {
            return Err(format!("refusing to overwrite {}", path.display()).into());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
    }
    let source = image::open(&input)?.to_rgba8();
    let (matted, report) = matte_checkerboard_sheet_background(&source);
    matted.save(&output)?;
    fs::write(&matting_report, serde_json::to_vec_pretty(&report)?)?;
    let halo = assess_alpha_edge_halo(&matted, &report.transparency_origin);
    fs::write(&halo_report, serde_json::to_vec_pretty(&halo)?)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "output": output,
            "mattingReport": report,
            "haloReport": halo,
        }))?
    );
    Ok(())
}
