use std::path::PathBuf;

use forge_core::grid_retry_source::validate_v93_platform_source_closure;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: validate_v93_platform_source <v9.3-job-dir>")?;
    let validated = validate_v93_platform_source_closure(&source, &[2])?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "sourceJobId": validated.job.job_id,
            "sourceWorkflow": format!("{}@{}", validated.request.workflow.id, validated.request.workflow.version),
            "selectedFrame": 2,
            "sourceFrameSha256": validated.report.frames[2].sha256,
            "sourcePoseStructureProfile": validated.report.frames[2].pose_structure_profile,
            "sourceGraphWorkflow": validated.graph.workflow,
            "verdict": "platform_failure_reproduced"
        }))?
    );
    Ok(())
}
