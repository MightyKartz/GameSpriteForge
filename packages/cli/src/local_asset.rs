//! One request contract for local ComfyUI image generation and Forge delivery.
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use clap::Args;
use forge_core::automation::{
    stage_plan_job, AssetInput, AutomationOperation, CharacterAnimationRecipe, GodotInstallRequest,
    PrepareCharacterPackRequest, PrepareStaticRequest, SourceLock,
};
use forge_core::job::{
    JobArtifactRecord, JobLifecycleState, JobOperationKind, JobRecord, JobState, SourceKind,
};
use forge_core::library::finalize::ProjectBinding;
use forge_core::video::{probe_video, ProbeVideoParams};
use forge_providers::comfyui::{self, ComfyClient, HistoryState, MediaKind};
use image::ImageFormat;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

type CliError = (String, String);
type Result<T> = std::result::Result<T, CliError>;

#[derive(Args)]
pub struct CreateArgs {
    /// A versioned JSON request. Mutually exclusive with --resume.
    #[arg(long, conflicts_with = "resume", required_unless_present = "resume")]
    input: Option<PathBuf>,
    /// Resume exactly the stored request and prompt; never resubmit automatically.
    #[arg(long, conflicts_with = "input")]
    resume: Option<String>,
    /// Explicit visual review JSON, only valid with --resume.
    #[arg(long, requires = "resume")]
    review: Option<PathBuf>,
    /// Cancel this Job; only pending Forge-owned ComfyUI prompts can be removed remotely.
    #[arg(long, requires = "resume", conflicts_with = "review")]
    cancel: bool,
    /// Poll this prompt and continue processing; without it return a Job ID promptly.
    #[arg(long)]
    wait: bool,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AssetRequest {
    schema_version: String,
    media_kind: MediaKind,
    #[serde(default)]
    workflow_profile: Option<String>,
    #[serde(default)]
    source: Option<PathBuf>,
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    reference_image: Option<PathBuf>,
    asset_id: String,
    name: String,
    purpose: String,
    kind: String,
    license: String,
    #[serde(default = "default_sampling")]
    sampling: String,
    #[serde(default = "default_canvas_size")]
    canvas_size: u32,
    #[serde(default)]
    animation_name: Option<String>,
    #[serde(default)]
    animation_fps: Option<f32>,
    #[serde(default)]
    target_frame_count: Option<u32>,
    #[serde(default)]
    loop_animation: Option<bool>,
    #[serde(default)]
    matting_mode: Option<String>,
    #[serde(default)]
    support_animations: Vec<CharacterAnimationRecipe>,
    #[serde(default = "default_alpha_threshold")]
    foreground_alpha_threshold: u8,
    #[serde(default)]
    edge_padding_px: u32,
    #[serde(default)]
    asset_project: Option<ProjectBinding>,
    #[serde(default)]
    godot_project: Option<PathBuf>,
    #[serde(default)]
    install_target: Option<PathBuf>,
    #[serde(default)]
    asset_key: Option<String>,
    #[serde(default = "default_max_wait")]
    max_wait_seconds: u64,
}

fn default_alpha_threshold() -> u8 {
    1
}
fn default_sampling() -> String {
    "nearest".into()
}
fn default_canvas_size() -> u32 {
    256
}
fn default_max_wait() -> u64 {
    900
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Review {
    schema_version: String,
    source_sha256: String,
    approved: bool,
    reviewer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssetState {
    request: AssetRequest,
    #[serde(default)]
    prompt_id: Option<String>,
    #[serde(default)]
    submitted: bool,
    #[serde(default)]
    source_path: Option<PathBuf>,
    #[serde(default)]
    source_sha256: Option<String>,
    #[serde(default)]
    workflow_sha256: Option<String>,
    #[serde(default)]
    model_id: Option<String>,
    #[serde(default)]
    reference_sha256: Option<String>,
    #[serde(default)]
    uploaded_reference_filename: Option<String>,
    #[serde(default)]
    video_probe: Option<serde_json::Value>,
    #[serde(default)]
    support_locks: Vec<SourceLock>,
    #[serde(default)]
    prepare_job_id: Option<String>,
    #[serde(default)]
    pack_path: Option<PathBuf>,
    #[serde(default)]
    preview_path: Option<PathBuf>,
    #[serde(default)]
    install_job_id: Option<String>,
    #[serde(default)]
    review: Option<Review>,
    phase: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetResult {
    job_id: String,
    state: String,
    phase: String,
    prompt_id: Option<String>,
    source_path: Option<PathBuf>,
    source_sha256: Option<String>,
    media_kind: MediaKind,
    video_probe: Option<serde_json::Value>,
    prepare_job_id: Option<String>,
    pack_path: Option<PathBuf>,
    preview_path: Option<PathBuf>,
    install_job_id: Option<String>,
    next_actions: Vec<String>,
}

pub fn create(args: CreateArgs) -> Result<AssetResult> {
    let store = super::job_store()?;
    let record = if let Some(input) = args.input {
        let input_bytes = fs::read(&input).map_err(super::io_error)?;
        if input_bytes.len() > 1024 * 1024 {
            return Err((
                "asset_request_invalid".into(),
                "request exceeds 1 MiB".into(),
            ));
        }
        let mut request: AssetRequest =
            serde_json::from_slice(&input_bytes).map_err(super::json_error)?;
        validate_request(&mut request)?;
        let mut support_locks = Vec::new();
        for animation in &request.support_animations {
            let paths: Vec<&PathBuf> = match &animation.input {
                AssetInput::PngSequence { paths } => paths.iter().collect(),
                AssetInput::SpriteSheet { path, .. } | AssetInput::VideoClip { path, .. } => {
                    vec![path]
                }
                AssetInput::Gsfpack { .. } => vec![],
            };
            for path in paths {
                if !support_locks
                    .iter()
                    .any(|lock: &SourceLock| lock.path == *path)
                {
                    support_locks.push(SourceLock {
                        path: path.clone(),
                        sha256: super::hash_asset_file(path).map_err(super::display_error)?,
                    });
                }
            }
        }
        let kind = match (request.media_kind, request.source.is_some()) {
            (MediaKind::Image, true) => SourceKind::ImportImage,
            (MediaKind::Image, false) => SourceKind::ComfyuiImage,
            (MediaKind::Video, true) => SourceKind::ImportVideo,
            (MediaKind::Video, false) => SourceKind::ComfyuiVideo,
        };
        let record = store.create_job(kind).map_err(super::display_error)?;
        let input_hash = format!("{:x}", Sha256::digest(&input_bytes));
        store
            .update_record(&record.job_id, |record| {
                record.input_hash = Some(input_hash);
            })
            .map_err(super::display_error)?;
        let state = AssetState {
            request,
            prompt_id: None,
            submitted: false,
            source_path: None,
            source_sha256: None,
            workflow_sha256: None,
            model_id: None,
            reference_sha256: None,
            uploaded_reference_filename: None,
            video_probe: None,
            support_locks,
            prepare_job_id: None,
            pack_path: None,
            preview_path: None,
            install_job_id: None,
            review: None,
            phase: "created".into(),
        };
        save(
            &record.job_id,
            &state,
            JobLifecycleState::Queued,
            vec!["resume".into()],
        )?;
        record
    } else {
        store
            .read_record(args.resume.as_deref().unwrap())
            .map_err(super::display_error)?
    };
    let record = store
        .read_record(&record.job_id)
        .map_err(super::display_error)?;
    let mut state = read_state(&record)?;
    if args.cancel {
        return cancel(&record.job_id, &mut state);
    }
    if state.phase == "cancelled" {
        if args.review.is_some() {
            return Err((
                "asset_review_invalid".into(),
                "cancelled Job cannot accept a review".into(),
            ));
        }
        return result(&record.job_id, &state);
    }
    if let (Some(source), Some(expected)) = (&state.source_path, &state.source_sha256) {
        let actual = super::hash_asset_file(source).map_err(super::display_error)?;
        if &actual != expected {
            return Err((
                "asset_source_changed".into(),
                "retained source no longer matches the Job SHA-256".into(),
            ));
        }
    }
    if let Some(path) = args.review {
        let review: Review = serde_json::from_slice(&fs::read(&path).map_err(super::io_error)?)
            .map_err(super::json_error)?;
        if review.schema_version != "1"
            || review.reviewer.trim().is_empty()
            || review.source_sha256 != state.source_sha256.as_deref().unwrap_or("")
        {
            return Err((
                "asset_review_invalid".into(),
                "review must identify this source SHA-256 and a nonempty reviewer".into(),
            ));
        }
        if state.review.is_some() {
            return Err((
                "asset_review_exists".into(),
                "review already recorded for this Job".into(),
            ));
        }
        state.review = Some(review);
        save(&record.job_id, &state, JobLifecycleState::Running, vec![])?;
    }
    if matches!(
        state.phase.as_str(),
        "succeeded" | "failed" | "rejected" | "cancelled"
    ) {
        return result(&record.job_id, &state);
    }
    if state.phase == "needs_recovery" && (!args.wait || state.prompt_id.is_none()) {
        return result(&record.job_id, &state);
    }
    advance(&record.job_id, &mut state, args.wait)?;
    result(&record.job_id, &state)
}

fn cancel(job_id: &str, state: &mut AssetState) -> Result<AssetResult> {
    if matches!(state.phase.as_str(), "succeeded" | "rejected" | "cancelled") {
        return result(job_id, state);
    }
    if let Some(prompt_id) = &state.prompt_id {
        if state.source_path.is_none() {
            let root = comfyui::profile_root().map_err(comfy_error)?;
            let stored = comfyui::load(&root, state.request.workflow_profile.as_deref().unwrap())
                .map_err(comfy_error)?;
            let client = ComfyClient::new(&stored.profile).map_err(comfy_error)?;
            if !client.cancel_pending(prompt_id).map_err(comfy_error)? {
                let history = client.history(prompt_id).map_err(comfy_error)?;
                if matches!(
                    comfyui::parse_history(&history, prompt_id, &stored.profile)
                        .map_err(comfy_error)?,
                    HistoryState::Pending
                ) {
                    return Err(("comfyui_cancel_uncertain".into(), "prompt is absent from queue and history; cannot confirm remote cancellation".into()));
                }
            }
        }
    }
    state.phase = "cancelled".into();
    save(
        job_id,
        state,
        JobLifecycleState::Cancelled,
        vec!["new_request_if_needed".into()],
    )?;
    result(job_id, state)
}

fn validate_request(request: &mut AssetRequest) -> Result<()> {
    if request.schema_version != "1" {
        return Err((
            "asset_request_invalid".into(),
            "schemaVersion must be 1".into(),
        ));
    }
    if request.asset_id.trim().is_empty()
        || request.name.trim().is_empty()
        || request.purpose.trim().is_empty()
        || request.license.trim().is_empty()
        || !match request.media_kind {
            MediaKind::Image => matches!(request.kind.as_str(), "icon_set" | "prop_set"),
            MediaKind::Video => request.kind == "character",
        }
        || request.canvas_size == 0
        || request.max_wait_seconds == 0
        || request.max_wait_seconds > 86400
    {
        return Err((
            "asset_request_invalid".into(),
            "invalid static asset identity, kind, canvas or max wait".into(),
        ));
    }
    if request.source.is_some() == request.workflow_profile.is_some() {
        return Err((
            "asset_request_invalid".into(),
            "choose exactly one of source or workflowProfile".into(),
        ));
    }
    if request.source.is_none()
        && request
            .prompt
            .as_deref()
            .is_none_or(|p| p.trim().is_empty())
    {
        return Err((
            "asset_request_invalid".into(),
            "ComfyUI generation requires a prompt".into(),
        ));
    }
    if let Some(source) = &mut request.source {
        if !source.is_absolute() {
            return Err((
                "asset_request_invalid".into(),
                "source must be an absolute path".into(),
            ));
        }
        *source = source.canonicalize().map_err(super::io_error)?;
    }
    if request.media_kind == MediaKind::Video {
        if request
            .animation_name
            .as_deref()
            .is_none_or(|name| name.trim().is_empty())
            || !request
                .animation_fps
                .is_some_and(|fps| fps.is_finite() && (1.0..=60.0).contains(&fps))
            || !request
                .target_frame_count
                .is_some_and(|count| (2..=24).contains(&count))
            || !matches!(
                request.matting_mode.as_deref().unwrap_or("auto_corners"),
                "auto_corners" | "preserve_alpha"
            )
        {
            return Err(("asset_request_invalid".into(), "video requires animationName, animationFps 1..60, targetFrameCount 2..24 and supported mattingMode".into()));
        }
        if request.source.is_none() != request.reference_image.is_some() {
            return Err((
                "asset_request_invalid".into(),
                "generated video requires referenceImage; imported video cannot set it".into(),
            ));
        }
        if request.support_animations.is_empty() {
            return Err((
                "asset_request_invalid".into(),
                "character video requires at least one existing supportAnimation".into(),
            ));
        }
        for animation in &mut request.support_animations {
            let paths: Vec<&mut PathBuf> = match &mut animation.input {
                AssetInput::PngSequence { paths } => paths.iter_mut().collect(),
                AssetInput::SpriteSheet { path, .. } | AssetInput::VideoClip { path, .. } => {
                    vec![path]
                }
                AssetInput::Gsfpack { .. } => {
                    return Err((
                        "asset_request_invalid".into(),
                        "supportAnimations cannot use gsfpack".into(),
                    ))
                }
            };
            for path in paths {
                if !path.is_absolute() {
                    return Err((
                        "asset_request_invalid".into(),
                        "support animation sources must be absolute paths".into(),
                    ));
                }
                *path = path.canonicalize().map_err(super::io_error)?;
            }
        }
        if let Some(reference) = &mut request.reference_image {
            if !reference.is_absolute() {
                return Err((
                    "asset_request_invalid".into(),
                    "referenceImage must be absolute".into(),
                ));
            }
            *reference = reference.canonicalize().map_err(super::io_error)?;
        }
    } else if request.reference_image.is_some()
        || request.animation_name.is_some()
        || !request.support_animations.is_empty()
    {
        return Err((
            "asset_request_invalid".into(),
            "image request cannot include video fields".into(),
        ));
    }
    if let Some(project) = &mut request.godot_project {
        if !project.is_absolute() || request.install_target.is_none() {
            return Err((
                "asset_request_invalid".into(),
                "godotProject requires an absolute path and installTarget".into(),
            ));
        }
        *project = project.canonicalize().map_err(super::io_error)?;
    } else if request.install_target.is_some() {
        return Err((
            "asset_request_invalid".into(),
            "installTarget requires godotProject".into(),
        ));
    }
    if let Some(binding) = &request.asset_project {
        binding.validate().map_err(super::display_error)?;
    }
    Ok(())
}

fn read_state(record: &JobRecord) -> Result<AssetState> {
    if record.operation_kind != JobOperationKind::LocalComfyAsset {
        return Err((
            "asset_job_invalid".into(),
            "Job was not created by forge asset create".into(),
        ));
    }
    serde_json::from_value(
        record
            .recipe
            .clone()
            .ok_or_else(|| ("asset_job_invalid".into(), "Job state missing".into()))?,
    )
    .map_err(super::json_error)
}

fn save(
    job_id: &str,
    state: &AssetState,
    lifecycle: JobLifecycleState,
    next: Vec<String>,
) -> Result<()> {
    let value = serde_json::to_value(state).map_err(super::json_error)?;
    super::job_store()?
        .update_record(job_id, |record| {
            record.operation_kind = JobOperationKind::LocalComfyAsset;
            record.lifecycle_state = lifecycle;
            record.recipe = Some(value);
            record.next_actions = next;
        })
        .map_err(super::display_error)?;
    Ok(())
}

fn result(job_id: &str, state: &AssetState) -> Result<AssetResult> {
    let record = super::job_store()?
        .read_record(job_id)
        .map_err(super::display_error)?;
    Ok(AssetResult {
        job_id: job_id.into(),
        state: serde_json::to_value(record.lifecycle_state)
            .map_err(super::json_error)?
            .as_str()
            .unwrap()
            .into(),
        phase: state.phase.clone(),
        prompt_id: state.prompt_id.clone(),
        source_path: state.source_path.clone(),
        source_sha256: state.source_sha256.clone(),
        media_kind: state.request.media_kind,
        video_probe: state.video_probe.clone(),
        prepare_job_id: state.prepare_job_id.clone(),
        pack_path: state.pack_path.clone(),
        preview_path: state.preview_path.clone(),
        install_job_id: state.install_job_id.clone(),
        next_actions: record.next_actions,
    })
}

fn advance(job_id: &str, state: &mut AssetState, wait: bool) -> Result<()> {
    if state.source_path.is_none() {
        if let Some(source) = &state.request.source {
            let limit = if state.request.media_kind == MediaKind::Image {
                64 * 1024 * 1024
            } else {
                512 * 1024 * 1024
            };
            if fs::metadata(source).map_err(super::io_error)?.len() > limit {
                return Err((
                    "asset_source_invalid".into(),
                    "source PNG exceeds 64 MiB".into(),
                ));
            }
            let bytes = fs::read(source).map_err(super::io_error)?;
            match state.request.media_kind {
                MediaKind::Image => import_png(job_id, state, &bytes)?,
                MediaKind::Video => import_video(job_id, state, &bytes)?,
            }
        } else {
            generate(job_id, state, wait)?;
            if state.source_path.is_none() {
                return Ok(());
            }
        }
    }
    if let Some(child_id) = &state.prepare_job_id {
        let child = super::job_store()?
            .read_record(child_id)
            .map_err(super::display_error)?;
        if child.lifecycle_state != JobLifecycleState::Succeeded {
            state.phase = "needs_recovery".into();
            save(
                job_id,
                state,
                JobLifecycleState::Failed,
                vec!["inspect_prepare_job".into(), "new_request_if_failed".into()],
            )?;
            return Ok(());
        }
        if state.pack_path.is_none() {
            state.pack_path = child
                .artifacts
                .iter()
                .find(|a| a.kind == "gsfpack")
                .map(|a| a.path.clone());
            state.preview_path = child
                .artifacts
                .iter()
                .find(|a| {
                    a.kind
                        == if state.request.media_kind == MediaKind::Image {
                            "contact_sheet"
                        } else {
                            "preview_gif"
                        }
                })
                .map(|a| a.path.clone());
            save(
                job_id,
                state,
                JobLifecycleState::Running,
                vec!["inspect_preview".into()],
            )?;
        }
    }
    if state.pack_path.is_none() {
        match state.request.media_kind {
            MediaKind::Image => prepare(job_id, state)?,
            MediaKind::Video => prepare_video(job_id, state)?,
        }
    }
    if state.review.as_ref().is_some_and(|review| !review.approved) {
        state.phase = "rejected".into();
        save(
            job_id,
            state,
            JobLifecycleState::Failed,
            vec!["new_request_for_revision".into()],
        )?;
        return Ok(());
    }
    if state.review.is_none() {
        state.phase = "awaiting_review".into();
        save(
            job_id,
            state,
            JobLifecycleState::AwaitingReview,
            vec!["inspect_preview".into(), "resume_with_review".into()],
        )?;
        return Ok(());
    }
    if state.request.godot_project.is_some() {
        if let Some(child_id) = &state.install_job_id {
            let child = super::job_store()?
                .read_record(child_id)
                .map_err(super::display_error)?;
            if child.lifecycle_state != JobLifecycleState::Succeeded {
                state.phase = "needs_recovery".into();
                save(
                    job_id,
                    state,
                    JobLifecycleState::Failed,
                    vec!["inspect_install_job".into()],
                )?;
                return Ok(());
            }
        } else {
            install(job_id, state)?;
        }
    }
    state.phase = "succeeded".into();
    save(
        job_id,
        state,
        JobLifecycleState::Succeeded,
        vec!["inspect_receipt".into()],
    )
}

fn generate(job_id: &str, state: &mut AssetState, wait: bool) -> Result<()> {
    let root = comfyui::profile_root().map_err(comfy_error)?;
    let stored = comfyui::load(&root, state.request.workflow_profile.as_deref().unwrap())
        .map_err(comfy_error)?;
    if state
        .workflow_sha256
        .as_ref()
        .is_some_and(|hash| hash != &stored.workflow_sha256)
    {
        return Err((
            "asset_workflow_changed".into(),
            "workflow hash differs from the submitted Job".into(),
        ));
    }
    if stored.profile.media_kind != state.request.media_kind {
        return Err((
            "asset_profile_mismatch".into(),
            "workflow profile mediaKind differs from the request".into(),
        ));
    }
    let client = ComfyClient::new(&stored.profile).map_err(comfy_error)?;
    if state.prompt_id.is_none() {
        let doctor = client.doctor(&stored).map_err(comfy_error)?;
        if !doctor.workflow_valid {
            return Err((
                "asset_profile_invalid".into(),
                doctor.message.unwrap_or_default(),
            ));
        }
        state.workflow_sha256 = Some(stored.workflow_sha256.clone());
        state.model_id = Some(stored.profile.model_id.clone());
        if let Some(reference) = &state.request.reference_image {
            let metadata = fs::metadata(reference).map_err(super::io_error)?;
            if metadata.len() > 32 * 1024 * 1024 {
                return Err((
                    "asset_reference_invalid".into(),
                    "reference image exceeds 32 MiB".into(),
                ));
            }
            let bytes = fs::read(reference).map_err(super::io_error)?;
            image::load_from_memory_with_format(&bytes, ImageFormat::Png)
                .map_err(|e| ("asset_reference_invalid".into(), e.to_string()))?;
            let hash = format!("{:x}", Sha256::digest(&bytes));
            if state
                .reference_sha256
                .as_ref()
                .is_some_and(|old| old != &hash)
            {
                return Err((
                    "asset_reference_changed".into(),
                    "reference image changed before submission".into(),
                ));
            }
            state.reference_sha256 = Some(hash);
            if state.uploaded_reference_filename.is_none() {
                let filename = format!("forge-{job_id}.png");
                state.uploaded_reference_filename =
                    Some(client.upload_image(&filename, bytes).map_err(comfy_error)?);
                save(
                    job_id,
                    state,
                    JobLifecycleState::Running,
                    vec!["submit_prompt".into()],
                )?;
            }
        }
        state.prompt_id = Some(Uuid::new_v4().to_string());
        state.phase = "submitting".into();
        save(
            job_id,
            state,
            JobLifecycleState::Running,
            vec!["query_prompt_history".into()],
        )?;
        // The ID is durable before POST. An uncertain response must never cause another POST.
        if let Err(error) = client.submit_with_reference(
            &stored.profile,
            state.request.prompt.as_deref().unwrap(),
            state.prompt_id.as_deref().unwrap(),
            state.uploaded_reference_filename.as_deref(),
        ) {
            state.phase = "needs_recovery".into();
            save(
                job_id,
                state,
                JobLifecycleState::Failed,
                vec![
                    "inspect_prompt_history".into(),
                    "new_request_if_absent".into(),
                ],
            )?;
            return Err(comfy_error(error));
        }
        state.submitted = true;
        state.phase = "generating".into();
        save(
            job_id,
            state,
            JobLifecycleState::Queued,
            vec!["resume".into()],
        )?;
    }
    if !wait {
        return Ok(());
    }
    if !state.submitted {
        let history = client
            .history(state.prompt_id.as_deref().unwrap())
            .map_err(comfy_error)?;
        if matches!(
            comfyui::parse_history(
                &history,
                state.prompt_id.as_deref().unwrap(),
                &stored.profile
            )
            .map_err(comfy_error)?,
            HistoryState::Pending
        ) {
            state.phase = "needs_recovery".into();
            save(
                job_id,
                state,
                JobLifecycleState::Failed,
                vec![
                    "inspect_prompt_history".into(),
                    "new_request_if_absent".into(),
                ],
            )?;
            return Ok(());
        }
    }
    let deadline = Instant::now() + Duration::from_secs(state.request.max_wait_seconds);
    loop {
        let history = client
            .history(state.prompt_id.as_deref().unwrap())
            .map_err(comfy_error)?;
        match comfyui::parse_history(
            &history,
            state.prompt_id.as_deref().unwrap(),
            &stored.profile,
        )
        .map_err(comfy_error)?
        {
            HistoryState::Pending => {
                if Instant::now() >= deadline {
                    save(
                        job_id,
                        state,
                        JobLifecycleState::Queued,
                        vec!["resume".into()],
                    )?;
                    return Ok(());
                }
                thread::sleep(Duration::from_secs(2));
            }
            HistoryState::Failed(message) => {
                state.phase = "failed".into();
                save(
                    job_id,
                    state,
                    JobLifecycleState::Failed,
                    vec!["inspect_comfyui_error".into()],
                )?;
                return Err(("comfyui_generation_failed".into(), message));
            }
            HistoryState::Succeeded(output) => {
                let bytes = client
                    .download(
                        &output.filename,
                        &output.subfolder,
                        &output.kind,
                        stored.profile.max_output_bytes,
                    )
                    .map_err(comfy_error)?;
                match state.request.media_kind {
                    MediaKind::Image => import_png(job_id, state, &bytes)?,
                    MediaKind::Video => import_video(job_id, state, &bytes)?,
                }
                return Ok(());
            }
        }
    }
}

fn import_png(job_id: &str, state: &mut AssetState, bytes: &[u8]) -> Result<()> {
    const MAX_SOURCE_BYTES: usize = 64 * 1024 * 1024;
    if bytes.len() < 24 || bytes.len() > MAX_SOURCE_BYTES || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
        return Err((
            "asset_source_invalid".into(),
            "source must be a PNG of at most 64 MiB".into(),
        ));
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    if width == 0
        || height == 0
        || width > 8192
        || height > 8192
        || u64::from(width) * u64::from(height) > 16_777_216
    {
        return Err((
            "asset_source_invalid".into(),
            "PNG dimensions exceed the 8192 side / 16 megapixel budget".into(),
        ));
    }
    let decoded = image::load_from_memory_with_format(bytes, ImageFormat::Png).map_err(|e| {
        (
            "asset_source_invalid".into(),
            format!("source is not a decodable PNG: {e}"),
        )
    })?;
    if !decoded.color().has_alpha() {
        return Err((
            "asset_source_invalid".into(),
            "image source must contain an alpha channel".into(),
        ));
    }
    let record = super::job_store()?
        .read_record(job_id)
        .map_err(super::display_error)?;
    let path = record.job_dir.join("source").join("original.png");
    fs::write(&path, bytes).map_err(super::io_error)?;
    let hash = super::hash_asset_file(&path).map_err(super::display_error)?;
    state.source_path = Some(path.clone());
    state.source_sha256 = Some(hash.clone());
    state.phase = "source_ready".into();
    super::job_store()?
        .update_record(job_id, |record| {
            record.state = JobState::SourceReady;
            record.artifacts.push(JobArtifactRecord {
                kind: "source_png".into(),
                path,
                sha256: Some(hash),
            });
        })
        .map_err(super::display_error)?;
    save(
        job_id,
        state,
        JobLifecycleState::Running,
        vec!["prepare_static".into()],
    )
}

fn import_video(job_id: &str, state: &mut AssetState, bytes: &[u8]) -> Result<()> {
    if bytes.len() < 32 || bytes.len() > 512 * 1024 * 1024 || &bytes[4..8] != b"ftyp" {
        return Err((
            "asset_source_invalid".into(),
            "video source must be an MP4 of at most 512 MiB".into(),
        ));
    }
    let record = super::job_store()?
        .read_record(job_id)
        .map_err(super::display_error)?;
    let path = record.job_dir.join("source").join("original.mp4");
    fs::write(&path, bytes).map_err(super::io_error)?;
    let probe = probe_video(&ProbeVideoParams {
        input_path: path.clone(),
        configured_ffprobe_path: None,
        bundled_resource_path: None,
    })
    .map_err(|e| (e.code, e.message))?;
    if probe.width == 0
        || probe.height == 0
        || probe.width > 2048
        || probe.height > 2048
        || !probe.duration_seconds.is_finite()
        || !(0.1..=30.0).contains(&probe.duration_seconds)
        || !probe.fps.is_finite()
        || !(1.0..=60.0).contains(&probe.fps)
        || probe.frame_count_estimate > 1800
    {
        return Err((
            "asset_source_invalid".into(),
            "video exceeds dimension, duration, fps or frame budget".into(),
        ));
    }
    let hash = super::hash_asset_file(&path).map_err(super::display_error)?;
    state.source_path = Some(path.clone());
    state.source_sha256 = Some(hash.clone());
    state.video_probe = Some(serde_json::to_value(probe).map_err(super::json_error)?);
    state.phase = "source_ready".into();
    super::job_store()?
        .update_record(job_id, |record| {
            record.state = JobState::SourceReady;
            record.artifacts.push(JobArtifactRecord {
                kind: "source_mp4".into(),
                path,
                sha256: Some(hash),
            });
        })
        .map_err(super::display_error)?;
    save(
        job_id,
        state,
        JobLifecycleState::Running,
        vec!["prepare_character".into()],
    )
}

fn prepare_video(job_id: &str, state: &mut AssetState) -> Result<()> {
    for lock in &state.support_locks {
        let actual = super::hash_asset_file(&lock.path).map_err(super::display_error)?;
        if actual != lock.sha256 {
            return Err((
                "asset_support_changed".into(),
                "support animation source changed after Job creation".into(),
            ));
        }
    }
    let source = state.source_path.as_ref().unwrap();
    let name = state.request.animation_name.as_deref().unwrap();
    let matting_mode = state
        .request
        .matting_mode
        .as_deref()
        .unwrap_or("auto_corners");
    let mut matting = if matting_mode == "auto_corners" {
        serde_json::to_value(forge_core::matting::ChromaParameters::default())
            .map_err(super::json_error)?
    } else {
        json!({})
    };
    matting["mode"] = json!(matting_mode);
    let mut source_locks = state.support_locks.clone();
    source_locks.push(SourceLock {
        path: source.clone(),
        sha256: state.source_sha256.clone().unwrap(),
    });
    let mut animations = vec![json!({
        "name": name,
        "fps": state.request.animation_fps.unwrap(),
        "loop": state.request.loop_animation.unwrap_or(false),
        "input": {"kind": "video_clip", "path": source, "startTimeMs": 0,
                  "targetFrameCount": state.request.target_frame_count.unwrap()},
        "matting": matting
    })];
    animations.extend(
        state
            .request
            .support_animations
            .iter()
            .map(|animation| json!(animation)),
    );
    let request: PrepareCharacterPackRequest = serde_json::from_value(json!({
        "schemaVersion": "2",
        "assetProject": state.request.asset_project,
        "sourceLocks": source_locks,
        "metadata": {"name": state.request.name, "defaultAnimation": name, "license": state.request.license},
        "quality": {"requireGameReady": false},
        "animations": animations
    })).map_err(super::json_error)?;
    let plans = super::plan_store()?;
    let plan = plans
        .prepare(AutomationOperation::PrepareCharacterPack(request))
        .map_err(super::display_error)?;
    let claimed = plans.claim(&plan.token).map_err(super::display_error)?;
    let store = super::job_store()?;
    let child = stage_plan_job(&store, &claimed).map_err(super::display_error)?;
    state.prepare_job_id = Some(child.job_id.clone());
    state.phase = "preparing".into();
    save(
        job_id,
        state,
        JobLifecycleState::Running,
        vec!["inspect_prepare_job".into()],
    )?;
    store
        .update_record(&child.job_id, |record| {
            record.parent_job_id = Some(job_id.into())
        })
        .map_err(super::display_error)?;
    let complete = super::run_claimed_plan(&store, &child.job_id, &claimed)?;
    state.pack_path = complete
        .artifacts
        .iter()
        .find(|a| a.kind == "gsfpack")
        .map(|a| a.path.clone());
    state.preview_path = complete
        .artifacts
        .iter()
        .find(|a| a.kind == "preview_gif")
        .map(|a| a.path.clone());
    let artifacts: Vec<_> = complete
        .artifacts
        .iter()
        .filter(|a| {
            matches!(
                a.kind.as_str(),
                "gsfpack" | "preview_gif" | "animation_quality_report" | "loop_selection_report"
            )
        })
        .cloned()
        .collect();
    store
        .update_record(job_id, |record| record.artifacts.extend(artifacts))
        .map_err(super::display_error)?;
    state.phase = "prepared".into();
    save(
        job_id,
        state,
        JobLifecycleState::Running,
        vec!["inspect_preview".into()],
    )
}

fn prepare(job_id: &str, state: &mut AssetState) -> Result<()> {
    let source = state.source_path.as_ref().unwrap();
    let request: PrepareStaticRequest = serde_json::from_value(json!({
        "assetProject": state.request.asset_project,
        "schemaVersion": "1", "kind": state.request.kind, "id": state.request.asset_id,
        "name": state.request.name, "license": state.request.license,
        "sampling": state.request.sampling, "canvasSize": state.request.canvas_size,
        "foregroundAlphaThreshold": state.request.foreground_alpha_threshold,
        "edgePaddingPx": state.request.edge_padding_px,
        "sourceLocks": [{"path": source, "sha256": state.source_sha256}],
        "items": [{"id": state.request.asset_id, "name": state.request.name, "path": source}]
    }))
    .map_err(super::json_error)?;
    let plans = super::plan_store()?;
    let plan = plans
        .prepare(AutomationOperation::PrepareStatic(request))
        .map_err(super::display_error)?;
    let claimed = plans.claim(&plan.token).map_err(super::display_error)?;
    let store = super::job_store()?;
    let child = stage_plan_job(&store, &claimed).map_err(super::display_error)?;
    state.prepare_job_id = Some(child.job_id.clone());
    state.phase = "preparing".into();
    save(
        job_id,
        state,
        JobLifecycleState::Running,
        vec!["inspect_prepare_job".into()],
    )?;
    store
        .update_record(&child.job_id, |record| {
            record.parent_job_id = Some(job_id.into())
        })
        .map_err(super::display_error)?;
    let complete = super::run_claimed_plan(&store, &child.job_id, &claimed)?;
    let pack = complete
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .ok_or_else(|| {
            (
                "asset_pack_missing".into(),
                "PrepareStatic produced no gsfpack artifact".into(),
            )
        })?;
    state.pack_path = Some(pack.path.clone());
    state.preview_path = complete
        .artifacts
        .iter()
        .find(|a| a.kind == "contact_sheet")
        .map(|a| a.path.clone());
    let artifacts: Vec<_> = complete
        .artifacts
        .iter()
        .filter(|a| {
            matches!(
                a.kind.as_str(),
                "gsfpack" | "contact_sheet" | "quality_report"
            )
        })
        .cloned()
        .collect();
    store
        .update_record(job_id, |record| record.artifacts.extend(artifacts))
        .map_err(super::display_error)?;
    state.phase = "prepared".into();
    save(
        job_id,
        state,
        JobLifecycleState::Running,
        vec!["inspect_preview".into()],
    )
}

fn install(job_id: &str, state: &mut AssetState) -> Result<()> {
    let request: GodotInstallRequest = serde_json::from_value(json!({
        "schemaVersion": "1", "packPath": state.pack_path, "projectPath": state.request.godot_project,
        "target": state.request.install_target, "assetKey": state.request.asset_key
    })).map_err(super::json_error)?;
    let plans = super::plan_store()?;
    let plan = plans
        .prepare(AutomationOperation::InstallGodot(request))
        .map_err(super::display_error)?;
    let claimed = plans.claim(&plan.token).map_err(super::display_error)?;
    let store = super::job_store()?;
    let child = stage_plan_job(&store, &claimed).map_err(super::display_error)?;
    state.install_job_id = Some(child.job_id.clone());
    state.phase = "installing".into();
    save(
        job_id,
        state,
        JobLifecycleState::Running,
        vec!["inspect_install_job".into()],
    )?;
    store
        .update_record(&child.job_id, |record| {
            record.parent_job_id = Some(job_id.into())
        })
        .map_err(super::display_error)?;
    super::run_claimed_plan(&store, &child.job_id, &claimed)?;
    Ok(())
}

fn comfy_error(error: comfyui::ComfyError) -> CliError {
    (error.code().into(), error.to_string())
}
