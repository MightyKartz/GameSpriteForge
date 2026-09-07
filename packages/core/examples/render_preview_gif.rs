use std::path::PathBuf;

use forge_core::export::{build_preview_gif_with_durations, GifBackground, PreviewGifParameters};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let frames_dir = PathBuf::from(args.next().ok_or("missing frames directory")?);
    let output = PathBuf::from(args.next().ok_or("missing output path")?);
    let durations = args
        .next()
        .ok_or("missing comma-separated frame durations")?
        .split(',')
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()?;
    let mut frames = std::fs::read_dir(&frames_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("png"))
        .collect::<Vec<_>>();
    frames.sort();
    if frames.len() != durations.len() {
        return Err(format!(
            "frame/duration mismatch: {} frames, {} durations",
            frames.len(),
            durations.len()
        )
        .into());
    }

    build_preview_gif_with_durations(
        &frames,
        &output,
        PreviewGifParameters {
            fps: 12.0,
            loop_animation: true,
            background: GifBackground::Checkerboard,
            scale: 1,
        },
        Some(&durations),
    )?;
    println!("{}", output.display());
    Ok(())
}
