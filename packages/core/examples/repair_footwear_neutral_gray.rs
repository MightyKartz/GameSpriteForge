use std::{fs, path::PathBuf};

use forge_core::footwear_platform::repair_neutral_gray_footwear_leak;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1).map(PathBuf::from);
    let input = arguments
        .next()
        .ok_or("usage: repair_footwear_neutral_gray <input.png> <output.png> <report.json>")?;
    let output = arguments.next().ok_or("missing output PNG path")?;
    let report_path = arguments.next().ok_or("missing report JSON path")?;
    if arguments.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let image = image::open(&input)?.to_rgba8();
    let (repaired, report) = repair_neutral_gray_footwear_leak(&image);
    repaired.save(&output)?;
    fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
