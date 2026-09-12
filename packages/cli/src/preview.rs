use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use clap::Args;
use forge_core::automation::{
    run_operation, stage_plan_job, AutomationOperation, GodotInstallRequest, PlanStore,
};
use forge_core::job::{JobLifecycleState, JobStore};

#[derive(Args)]
pub struct PreviewArgs {
    #[arg(long)]
    pub pack: PathBuf,
    /// A new directory for an isolated Godot project and its install evidence.
    #[arg(long)]
    pub output: PathBuf,
    #[arg(long)]
    pub godot: Option<PathBuf>,
    /// Open the generated preview window with playback controls.
    #[arg(long)]
    pub launch: bool,
    #[arg(long)]
    pub json: bool,
}

pub fn run(args: PreviewArgs) -> Result<serde_json::Value, (String, String)> {
    run_inner(args).map_err(|error| ("godot_preview_failed".into(), error))
}

fn run_inner(args: PreviewArgs) -> Result<serde_json::Value, String> {
    let pack = fs::canonicalize(&args.pack).map_err(|error| error.to_string())?;
    forge_pack::validate_pack_layout(&pack).map_err(|error| error.to_string())?;
    let summary = forge_pack::inspect_pack(&pack).map_err(|error| error.to_string())?;
    if !matches!(
        summary.asset_type.as_str(),
        "animation" | "character" | "layered"
    ) {
        return Err("preview currently accepts animation, character and layered Packs".into());
    }
    let godot = args
        .godot
        .or_else(super::locate_godot)
        .ok_or("Godot 4.6 executable was not found; set FORGE_GODOT_PATH or pass --godot")?;
    if !super::godot_version(&godot).is_some_and(|version| version.starts_with("4.6.")) {
        return Err("Godot preview requires 4.6.x".into());
    }
    let godot = fs::canonicalize(godot).map_err(|error| error.to_string())?;
    // create_dir refuses an existing project, including symlinks; previews never reinstall a consumer.
    fs::create_dir(&args.output)
        .map_err(|error| format!("preview output must be a new directory: {error}"))?;
    let output = fs::canonicalize(&args.output).map_err(|error| error.to_string())?;
    let project = output.join("project");
    fs::create_dir(&project).map_err(|error| error.to_string())?;
    let project_text = "config_version=5\n[application]\nconfig/name=\"Forge asset preview\"\nrun/main_scene=\"res://preview.tscn\"\n[display]\nwindow/size/viewport_width=1000\nwindow/size/viewport_height=820\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\ntextures/default_filters/use_nearest_mipmap_filter=false\n";
    fs::write(
        project.join("project.godot"),
        project_text.replace("run/main_scene=\"res://preview.tscn\"\n", ""),
    )
    .map_err(|error| error.to_string())?;
    std::env::set_var("FORGE_GODOT_PATH", &godot);
    let plans = PlanStore::new(output.join("plans")).map_err(|error| error.to_string())?;
    let jobs = JobStore::new(output.join("jobs")).map_err(|error| error.to_string())?;
    let prepared = plans
        .prepare(AutomationOperation::InstallGodot(GodotInstallRequest {
            catalog_revision: None,
            resource_lock_path: None,
            schema_version: "1".into(),
            pack_path: pack.clone(),
            project_path: project.clone(),
            catalog_project_path: None,
            target: "addons/forge_assets/preview".into(),
            asset_key: Some(summary.id.clone()),
            provider_refs: vec![],
        }))
        .map_err(|error| error.to_string())?;
    let claimed = plans
        .claim(&prepared.token)
        .map_err(|error| error.to_string())?;
    let job = stage_plan_job(&jobs, &claimed).map_err(|error| error.to_string())?;
    let job =
        run_operation(&jobs, &job.job_id, &claimed.operation).map_err(|error| error.to_string())?;
    if job.lifecycle_state != JobLifecycleState::Succeeded {
        return Err(format!(
            "native preview installation failed; inspect {}",
            job.job_dir.display()
        ));
    }
    let scene = if summary.asset_type == "layered" {
        "res://addons/forge_assets/preview/layered.tscn"
    } else {
        "res://addons/forge_assets/preview/forge_animated_sprite.tscn"
    };
    let config = serde_json::json!({"scene":scene,"kind":summary.asset_type,"name":summary.name,"canvas":summary.layered.as_ref().map(|manifest| &manifest.canvas)});
    fs::write(
        project.join("preview.json"),
        serde_json::to_vec_pretty(&config).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    fs::write(
        project.join("preview.gd"),
        include_str!("../../../scripts/godot/forge_preview.gd"),
    )
    .map_err(|error| error.to_string())?;
    fs::write(project.join("preview.tscn"), "[gd_scene load_steps=2 format=3]\n[ext_resource type=\"Script\" path=\"res://preview.gd\" id=\"1\"]\n[node name=\"ForgePreview\" type=\"Node2D\"]\nscript = ExtResource(\"1\")\n").map_err(|error| error.to_string())?;
    fs::write(project.join("project.godot"), project_text).map_err(|error| error.to_string())?;
    let verify = Command::new(&godot)
        .args(["--headless", "--path"])
        .arg(&project)
        .args(["--quit-after", "120", "--", "--forge-preview-verify"])
        .output()
        .map_err(|error| error.to_string())?;
    let log = format!(
        "{}\n{}",
        String::from_utf8_lossy(&verify.stdout),
        String::from_utf8_lossy(&verify.stderr)
    );
    fs::write(output.join("preview-verify.log"), &log).map_err(|error| error.to_string())?;
    if !verify.status.success()
        || !log.contains("FORGE_PREVIEW_READY")
        || log.contains("SCRIPT ERROR:")
        || log.contains("Parse Error:")
    {
        return Err(format!(
            "preview scene validation failed; inspect {}",
            output.join("preview-verify.log").display()
        ));
    }
    let child_id = if args.launch {
        Some(
            Command::new(&godot)
                .arg("--path")
                .arg(&project)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|error| error.to_string())?
                .id(),
        )
    } else {
        None
    };
    let report = serde_json::json!({"schemaVersion":"1","project":project,"pack":pack,"assetType":summary.asset_type,"scene":scene,"nativeLoad":"passed","jobId":job.job_id,"godot":godot,"launchedProcessId":child_id,"providerRequestCount":0,"visualReview":"not_assessed"});
    fs::write(
        output.join("preview-report.json"),
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(report)
}
