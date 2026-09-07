use std::path::PathBuf;

use forge_core::quality::evaluate_calibration_manifest;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut manifest = None;
    let mut output = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--manifest" => manifest = args.next().map(PathBuf::from),
            "--output" => output = args.next().map(PathBuf::from),
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }
    let manifest = manifest.ok_or("--manifest is required")?;
    let output = output.ok_or("--output is required")?;
    let report = evaluate_calibration_manifest(&manifest)?;
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&output, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "baseline separation={:.4} composite separation={:.4} baseline auc={:.4} composite auc={:.4}",
        report.baseline_phash.separation,
        report.composite.separation,
        report.baseline_phash.roc_auc,
        report.composite.roc_auc,
    );
    Ok(())
}
