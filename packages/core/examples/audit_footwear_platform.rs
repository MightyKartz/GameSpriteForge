use std::path::PathBuf;

use forge_core::footwear_platform::assess_footwear_platform;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: audit_footwear_platform <sprite.png> [frame-index]")?;
    let frame_index = std::env::args()
        .nth(2)
        .map(|value| value.parse::<u8>())
        .transpose()?
        .unwrap_or(2);
    let image = image::open(&path)?.to_rgba8();
    let report = assess_footwear_platform("walk_down", frame_index, &image);
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
