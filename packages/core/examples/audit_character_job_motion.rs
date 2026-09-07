use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use forge_core::motion_semantics::assess_character_motion_semantics;

fn main() -> Result<(), Box<dyn Error>> {
    let job_dir = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: audit_character_job_motion <job-dir>")?;
    let normalized: serde_json::Value =
        serde_json::from_slice(&fs::read(job_dir.join("normalized-frames.json"))?)?;
    let entries = normalized
        .as_array()
        .ok_or("normalized-frames.json must contain an array")?;
    let mut animations = BTreeMap::new();
    for entry in entries {
        let animation = entry
            .get("animation")
            .and_then(serde_json::Value::as_str)
            .ok_or("normalized frame is missing animation")?;
        let index = entry
            .get("index")
            .and_then(serde_json::Value::as_u64)
            .ok_or("normalized frame is missing index")?;
        let path = job_dir
            .join("processed/normalized")
            .join(format!("frame_{:05}.png", index + 1));
        animations
            .entry(animation.to_string())
            .or_insert_with(Vec::new)
            .push(image::open(path)?.to_rgba8());
    }
    let report = assess_character_motion_semantics(&animations);
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
