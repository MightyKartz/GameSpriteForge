//! Stage 2 Wave 3 integration tests for the project build orchestrator
//! (`forge_core::game_art::run_build_project`). All tests run against a local
//! fixture provider that mirrors `packages/providers/src/fixture.rs` (core
//! cannot depend on the providers crate), including the
//! `[fixture:hard_multiple_subjects]` failure-injection prompt.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

use forge_core::asset_project::{
    init_project, read_project, ForgeProjectV1, SamplingMode, StyleSpecV1, FORGE_PROJECT_FILE,
};
use forge_core::automation::{
    run_operation_with_provider, stage_plan_job, AutomationOperation, BuildProjectRequestV1,
    CreateStyleLockRequest, PlanStore,
};
use forge_core::catalog::{read_project_catalog, PROJECT_CATALOG_RELATIVE};
use forge_core::game_art::{
    reconcile_interrupted_builds, run_build_project, BuildResultStatusV1, BUILD_STATE_FILE,
    PROJECT_BUILD_REPORT_FILE,
};
use forge_core::job::{JobLifecycleState, JobOperationKind, JobStore, SourceKind};
use forge_core::provider::{
    CredentialKind, EditImageRequest, EditVideoRequest, GenerateImageRequest, GenerateVideoRequest,
    MediaGenerationProvider, ProviderCapability, ProviderError, ProviderHealth, ProviderMedia,
    ProviderPoll, ProviderTicket, ProviderUsage,
};
use gif::{Encoder, Frame, Repeat};
use image::{ImageBuffer, Rgba, RgbaImage};
use tempfile::tempdir;

// ---------------------------------------------------------------------------
// Fixture provider (mirror of packages/providers/src/fixture.rs)
// ---------------------------------------------------------------------------

struct FixtureTicket;

struct FixtureProvider {
    tickets: Mutex<HashMap<String, FixtureTicket>>,
    usage: Mutex<ProviderUsage>,
}

impl Default for FixtureProvider {
    fn default() -> Self {
        Self {
            tickets: Mutex::new(HashMap::new()),
            usage: Mutex::new(ProviderUsage::default()),
        }
    }
}

impl FixtureProvider {
    fn write_image(
        &self,
        output_path: &Path,
        prompt: &str,
    ) -> Result<ProviderMedia, ProviderError> {
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut image: RgbaImage = ImageBuffer::from_pixel(96, 96, Rgba([0, 255, 0, 255]));
        let color = if prompt.contains("[fixture:palette_drift]") {
            Rgba([220, 90, 60, 255])
        } else {
            Rgba([130, 60, 210, 255])
        };
        if prompt.contains("[fixture:hard_multiple_subjects]") {
            for offset in [16_u32, 54_u32] {
                for y in 28..68 {
                    for x in offset..(offset + 24) {
                        image.put_pixel(x, y, color);
                    }
                }
                for y in 68..84 {
                    for x in offset..(offset + 8) {
                        image.put_pixel(x, y, color);
                    }
                    for x in (offset + 16)..(offset + 24) {
                        image.put_pixel(x, y, color);
                    }
                }
            }
        } else {
            for y in 28..84 {
                for x in 34..62 {
                    image.put_pixel(x, y, color);
                }
            }
        }
        image
            .save(output_path)
            .map_err(|error| ProviderError::InvalidOutput(error.to_string()))?;
        Ok(ProviderMedia {
            path: output_path.to_path_buf(),
            mime_type: "image/png".into(),
            provider_asset_id: Some("fixture-image".into()),
            revised_prompt: None,
        })
    }

    fn write_video(&self, output_path: &Path) -> Result<ProviderMedia, ProviderError> {
        let gif_path = output_path.with_extension("gif");
        if let Some(parent) = gif_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = fs::File::create(&gif_path)?;
        let mut encoder = Encoder::new(file, 96, 96, &[])
            .map_err(|error| ProviderError::InvalidOutput(error.to_string()))?;
        encoder
            .set_repeat(Repeat::Infinite)
            .map_err(|error| ProviderError::InvalidOutput(error.to_string()))?;
        for index in 0..24u8 {
            let mut pixels = vec![0u8; 96 * 96 * 4];
            for pixel in pixels.chunks_exact_mut(4) {
                pixel.copy_from_slice(&[0, 255, 0, 255]);
            }
            let phase = index % 8;
            let swing = if phase <= 4 {
                phase as usize
            } else {
                (8 - phase) as usize
            };
            for y in 28..68 {
                for x in 34..62 {
                    let start = (y * 96 + x) * 4;
                    pixels[start..start + 4].copy_from_slice(&[210, 70, 90, 255]);
                }
            }
            let left_leg = 32 + swing.min(18);
            let right_leg = 54usize.saturating_sub(swing.min(18));
            for y in 68..84 {
                for x in left_leg..(left_leg + 8) {
                    let start = (y * 96 + x) * 4;
                    pixels[start..start + 4].copy_from_slice(&[210, 70, 90, 255]);
                }
                for x in right_leg..(right_leg + 8) {
                    let start = (y * 96 + x) * 4;
                    pixels[start..start + 4].copy_from_slice(&[210, 70, 90, 255]);
                }
            }
            let mut frame = Frame::from_rgba_speed(96, 96, &mut pixels, 10);
            frame.delay = 8;
            encoder
                .write_frame(&frame)
                .map_err(|error| ProviderError::InvalidOutput(error.to_string()))?;
        }
        Ok(ProviderMedia {
            path: gif_path,
            mime_type: "image/gif".into(),
            provider_asset_id: Some("fixture-video".into()),
            revised_prompt: None,
        })
    }
}

impl MediaGenerationProvider for FixtureProvider {
    fn id(&self) -> &'static str {
        "fixture"
    }

    fn capabilities(&self) -> Vec<ProviderCapability> {
        vec![
            ProviderCapability::GenerateImage,
            ProviderCapability::EditImage,
            ProviderCapability::GenerateVideo,
            ProviderCapability::ImageToVideo,
            ProviderCapability::ReferenceToVideo,
            ProviderCapability::EditVideo,
            ProviderCapability::PrivateFileInput,
            ProviderCapability::Usage,
        ]
    }

    fn health_check(&self) -> ProviderHealth {
        ProviderHealth {
            provider_id: self.id().into(),
            available: true,
            authenticated: true,
            auth_kind: CredentialKind::None,
            capabilities: self.capabilities(),
            constraints: None,
            message: Some("deterministic offline test provider".into()),
        }
    }

    fn resolved_image_model(&self, requested: Option<&str>) -> Option<String> {
        Some(requested.unwrap_or("fixture-image").to_string())
    }

    fn resolved_video_model(&self, requested: Option<&str>) -> Option<String> {
        Some(requested.unwrap_or("fixture-video").to_string())
    }

    fn generate_image(
        &self,
        request: &GenerateImageRequest,
        output_path: &Path,
    ) -> Result<ProviderMedia, ProviderError> {
        self.usage.lock().unwrap().requests += 1;
        self.usage.lock().unwrap().generated_images += 1;
        let prompt = format!(
            "{} {}",
            request.prompt,
            request.model.as_deref().unwrap_or_default()
        );
        self.write_image(output_path, &prompt)
    }

    fn edit_image(
        &self,
        request: &EditImageRequest,
        output_path: &Path,
    ) -> Result<ProviderMedia, ProviderError> {
        self.usage.lock().unwrap().requests += 1;
        self.usage.lock().unwrap().generated_images += 1;
        let prompt = format!(
            "{} {}",
            request.prompt,
            request.model.as_deref().unwrap_or_default()
        );
        self.write_image(output_path, &prompt)
    }

    fn generate_video(
        &self,
        _request: &GenerateVideoRequest,
    ) -> Result<ProviderTicket, ProviderError> {
        let id = format!("fixture-{}", self.tickets.lock().unwrap().len() + 1);
        self.tickets
            .lock()
            .unwrap()
            .insert(id.clone(), FixtureTicket);
        self.usage.lock().unwrap().requests += 1;
        Ok(ProviderTicket {
            provider_id: self.id().into(),
            request_id: id,
        })
    }

    fn edit_video(&self, _request: &EditVideoRequest) -> Result<ProviderTicket, ProviderError> {
        let id = format!("fixture-edit-{}", self.tickets.lock().unwrap().len() + 1);
        self.tickets
            .lock()
            .unwrap()
            .insert(id.clone(), FixtureTicket);
        let mut usage = self.usage.lock().unwrap();
        usage.requests += 1;
        usage.edited_videos += 1;
        Ok(ProviderTicket {
            provider_id: self.id().into(),
            request_id: id,
        })
    }

    fn poll(
        &self,
        ticket: &ProviderTicket,
        output_path: &Path,
    ) -> Result<ProviderPoll, ProviderError> {
        let mut tickets = self.tickets.lock().unwrap();
        if tickets.remove(&ticket.request_id).is_none() {
            return Ok(ProviderPoll::Failed {
                code: "unknown_fixture_ticket".into(),
                message: "fixture ticket was not found".into(),
            });
        }
        drop(tickets);
        self.usage.lock().unwrap().generated_videos += 1;
        Ok(ProviderPoll::Succeeded(self.write_video(output_path)?))
    }

    fn cancel(&self, ticket: &ProviderTicket) -> Result<(), ProviderError> {
        self.tickets.lock().unwrap().remove(&ticket.request_id);
        Ok(())
    }

    fn usage(&self) -> ProviderUsage {
        self.usage.lock().unwrap().clone()
    }
}

/// Serialize the two tests that mutate process-wide FORGE_PLAN_STORE so they
/// cannot observe each other's value (integration tests in one binary share
/// the process environment).
static ENV_LOCK: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------------------
// Project / manifest fixtures
// ---------------------------------------------------------------------------

const STYLE_SPEC: &str = "compact jewel-tone pixel art";

/// Create a Forge project pinned to the fixture provider and run the real
/// CreateStyleLock operation flow so the project has a current style
/// revision exactly the way CLI-driven projects get one.
fn setup_project(root: &Path, plans: &PlanStore, jobs: &JobStore, provider: &FixtureProvider) {
    init_project(root, "Game Assets").unwrap();
    let mut project: ForgeProjectV1 = read_project(root).unwrap();
    project.provider.id = "fixture".into();
    fs::write(
        root.join(FORGE_PROJECT_FILE),
        serde_json::to_vec_pretty(&project).unwrap(),
    )
    .unwrap();
    let style_spec_path = root.join("specs/style.json");
    let style_spec = StyleSpecV1 {
        schema_version: "1".into(),
        prompt: STYLE_SPEC.into(),
        reference_images: vec![],
        perspective: "topdown".into(),
        lighting: "upper_left".into(),
        outline: "dark".into(),
        background: "transparent".into(),
        sampling: SamplingMode::Nearest,
        character_canvas_size: 256,
        icon_canvas_size: 128,
        prop_canvas_size: 256,
        image_model: None,
    };
    fs::write(
        &style_spec_path,
        serde_json::to_vec_pretty(&style_spec).unwrap(),
    )
    .unwrap();
    let prepared = plans
        .prepare(AutomationOperation::CreateStyleLock(
            CreateStyleLockRequest {
                schema_version: "1".into(),
                project_path: root.to_path_buf(),
                spec_path: style_spec_path,
                provider_id: "fixture".into(),
                profile_id: "default".into(),
            },
        ))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let style_job = stage_plan_job(jobs, &claimed).unwrap();
    let result =
        run_operation_with_provider(jobs, &style_job.job_id, &claimed.operation, Some(provider))
            .unwrap();
    assert_eq!(result.lifecycle_state, JobLifecycleState::Succeeded);
    assert!(read_project(root).unwrap().current_style_revision.is_some());
}

fn write_character_spec(root: &Path, id: &str, prompt: &str) {
    fs::write(
        root.join(format!("specs/{id}.json")),
        serde_json::json!({
            "schemaVersion": "1",
            "kind": "character",
            "id": id,
            "name": format!("Name {id}"),
            "prompt": prompt,
            "license": "private",
        })
        .to_string(),
    )
    .unwrap();
}

fn write_static_spec(root: &Path, id: &str, kind: &str, item_prompts: &[&str]) {
    let items: Vec<_> = item_prompts
        .iter()
        .enumerate()
        .map(|(index, prompt)| {
            serde_json::json!({
                "id": format!("item-{index}"),
                "name": format!("Item {index}"),
                "prompt": prompt,
            })
        })
        .collect();
    fs::write(
        root.join(format!("specs/{id}.json")),
        serde_json::json!({
            "schemaVersion": "1",
            "kind": kind,
            "id": id,
            "name": format!("Name {id}"),
            "items": items,
            "license": "private",
        })
        .to_string(),
    )
    .unwrap();
}

/// One manifest asset entry: `("id", "kind", extra_json)`.
fn write_manifest(root: &Path, assets: &[(&str, &str, &str)]) {
    let assets = assets
        .iter()
        .map(|(id, kind, extra)| {
            let separator = if extra.is_empty() { "" } else { ", " };
            format!(
                r#"{{ "id": "{id}", "kind": "{kind}", "spec": "specs/{id}.json"{separator}{extra} }}"#
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        root.join("game-art.json"),
        format!(
            r#"{{
                "schemaVersion": "1",
                "kind": "game_art_manifest",
                "projectId": "game-assets",
                "name": "Game Assets",
                "provider": {{ "id": "fixture", "profileId": "default" }},
                "defaults": {{
                    "outputDirectory": "packs",
                    "godotRoot": "addons/forge_assets",
                    "license": "private"
                }},
                "assets": [ {assets} ]
            }}"#
        ),
    )
    .unwrap();
}

fn build_request(root: &Path) -> BuildProjectRequestV1 {
    BuildProjectRequestV1 {
        schema_version: "1".into(),
        project_path: root.to_path_buf(),
        manifest_path: root.join("game-art.json"),
    }
}

/// Stage a parent build job through the real plan flow, mark it running the
/// way `run_operation_with_provider` would, then invoke the orchestrator.
/// Returns the parent job id plus the orchestrator outcome.
#[allow(clippy::type_complexity)]
fn run_build(
    plans: &PlanStore,
    jobs: &JobStore,
    root: &Path,
    provider: &FixtureProvider,
) -> (
    String,
    Result<forge_core::game_art::ProjectBuildReportV1, forge_core::automation::AutomationRunError>,
) {
    let request = build_request(root);
    let prepared = plans
        .prepare(AutomationOperation::BuildProject(request.clone()))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let parent = stage_plan_job(jobs, &claimed).unwrap();
    jobs.update_record(&parent.job_id, |record| {
        record.lifecycle_state = JobLifecycleState::Running;
        record.worker_pid = Some(std::process::id());
    })
    .unwrap();
    let result = run_build_project(jobs, plans, &parent.job_id, &request, Some(provider));
    (parent.job_id, result)
}

/// Stage a parent build job without running it; returns the parent job id.
fn stage_parent(plans: &PlanStore, jobs: &JobStore, root: &Path) -> String {
    let prepared = plans
        .prepare(AutomationOperation::BuildProject(build_request(root)))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let parent = stage_plan_job(jobs, &claimed).unwrap();
    parent.job_id
}

fn children_of(jobs: &JobStore, parent_job_id: &str) -> Vec<forge_core::job::JobRecord> {
    jobs.list_children(parent_job_id).unwrap()
}

fn read_report(parent_dir: &Path) -> serde_json::Value {
    serde_json::from_slice(&fs::read(parent_dir.join(PROJECT_BUILD_REPORT_FILE)).unwrap()).unwrap()
}

fn media_tools_available() -> bool {
    forge_core::video::ffmpeg::find_in_path("ffmpeg").is_some()
        && forge_core::video::ffmpeg::find_in_path("ffprobe").is_some()
}

// ---------------------------------------------------------------------------
// (a) End-to-end build
// ---------------------------------------------------------------------------

#[test]
fn build_project_end_to_end_registers_catalog_and_report() {
    if !media_tools_available() {
        eprintln!("skipping character build E2E because ffmpeg/ffprobe are unavailable");
        return;
    }
    let temp = tempdir().unwrap();
    let root = temp.path().join("project");
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let provider = FixtureProvider::default();
    setup_project(&root, &plans, &jobs, &provider);
    write_character_spec(&root, "hero", "a brave knight");
    write_static_spec(
        &root,
        "hud-icons",
        "icon_set",
        &["a gold coin", "a red gem"],
    );
    write_static_spec(&root, "forest-props", "prop_set", &["a wooden crate"]);
    write_manifest(
        &root,
        &[
            ("hero", "character", ""),
            ("hud-icons", "icon_set", ""),
            ("forest-props", "prop_set", ""),
        ],
    );

    let parent_id = stage_parent(&plans, &jobs, &root);
    jobs.update_record(&parent_id, |record| {
        record.lifecycle_state = JobLifecycleState::Running;
        record.worker_pid = Some(std::process::id());
    })
    .unwrap();
    let report = run_build_project(
        &jobs,
        &plans,
        &parent_id,
        &build_request(&root),
        Some(&provider),
    )
    .unwrap();

    assert_eq!(report.kind, "project_build_report");
    assert_eq!(report.schema_version, "1");
    assert_eq!(report.summary.built, 3);
    assert_eq!(report.summary.reused, 0);
    assert_eq!(report.summary.failed, 0);
    assert_eq!(report.summary.skipped, 0);
    assert!(report.provider_usage.requests > 0);

    // Exactly three child jobs, each linked to the parent, each succeeded.
    let children = children_of(&jobs, &parent_id);
    assert_eq!(children.len(), 3);
    for child in &children {
        assert_eq!(child.parent_job_id.as_deref(), Some(parent_id.as_str()));
        assert_eq!(child.lifecycle_state, JobLifecycleState::Succeeded);
    }
    let kinds = children
        .iter()
        .map(|child| child.operation_kind)
        .collect::<Vec<_>>();
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == JobOperationKind::GenerateCharacterPack)
            .count(),
        1
    );
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == JobOperationKind::GenerateStaticAssetSet)
            .count(),
        2
    );

    // The V2 catalog has exactly one entry per manifest asset, with the
    // stage 2 provenance populated (test g: exactly-once registration).
    let catalog = read_project_catalog(&root).unwrap();
    assert_eq!(catalog.assets.len(), 3);
    let hero = catalog.assets.get("hero").unwrap();
    assert_eq!(hero.kind, "character");
    assert_eq!(hero.workflow, "topdown@1.0.0");
    assert_eq!(hero.workflow_profile.as_deref(), Some("topdown"));
    assert_eq!(hero.workflow_version.as_deref(), Some("1.0.0"));
    assert_eq!(hero.parent_job_id.as_deref(), Some(parent_id.as_str()));
    assert!(hero.pack_path.is_absolute() || root.join(&hero.pack_path).is_dir());
    assert_eq!(hero.pack_sha256.len(), 64);
    assert!(hero.spec_sha256.as_ref().is_some_and(|sha| sha.len() == 64));
    assert_eq!(
        hero.locks.as_ref().unwrap().style,
        read_project(&root).unwrap().current_style_revision
    );
    assert_eq!(hero.provider.as_ref().unwrap().provider_id, "fixture");
    assert_eq!(hero.game_ready, Some(true));
    assert!(hero.generated_at.is_some());
    let icons = catalog.assets.get("hud-icons").unwrap();
    assert_eq!(icons.kind, "icon_set");
    assert_eq!(icons.workflow, "static-set@1.0.0");
    assert_eq!(icons.game_ready, Some(true));
    let props = catalog.assets.get("forest-props").unwrap();
    assert_eq!(props.kind, "prop_set");
    assert_eq!(props.workflow, "static-set@1.0.0");

    // The report artifact is on the parent record and on disk.
    let parent = jobs.read_record(&parent_id).unwrap();
    assert_eq!(parent.lifecycle_state, JobLifecycleState::Succeeded);
    let artifact = parent
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "project_build_report")
        .expect("report artifact");
    assert!(artifact.path.is_file());
    let on_disk = read_report(&parent.job_dir);
    assert_eq!(on_disk["kind"], "project_build_report");
    assert_eq!(on_disk["summary"]["built"], 3);
    assert!(on_disk["planSha256"].is_string());
    assert_eq!(on_disk["results"].as_array().unwrap().len(), 3);

    // build-state.json recorded every asset as succeeded with pack facts.
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(parent.job_dir.join(BUILD_STATE_FILE)).unwrap()).unwrap();
    assert_eq!(state["schemaVersion"], "1");
    for entry in state["assets"].as_array().unwrap() {
        assert_eq!(entry["status"], "succeeded");
        assert!(entry["childJobId"].is_string());
        assert!(entry["packSha256"].is_string());
    }
}

// ---------------------------------------------------------------------------
// (b) Re-execution reuses satisfied assets
// ---------------------------------------------------------------------------

#[test]
fn build_project_reexecution_reuses_everything_without_child_jobs() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("project");
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let provider = FixtureProvider::default();
    setup_project(&root, &plans, &jobs, &provider);
    write_static_spec(&root, "hud-icons", "icon_set", &["a gold coin"]);
    write_static_spec(&root, "forest-props", "prop_set", &["a wooden crate"]);
    write_manifest(
        &root,
        &[
            ("hud-icons", "icon_set", ""),
            ("forest-props", "prop_set", ""),
        ],
    );

    let (_first_parent, first) = run_build(&plans, &jobs, &root, &provider);
    let first = first.unwrap();
    assert_eq!(first.summary.built, 2);
    let usage_after_first = provider.usage();

    // Same manifest, new parent job: every action is a catalog reuse and no
    // provider job is created.
    let parent_id = stage_parent(&plans, &jobs, &root);
    jobs.update_record(&parent_id, |record| {
        record.lifecycle_state = JobLifecycleState::Running;
        record.worker_pid = Some(std::process::id());
    })
    .unwrap();
    let report = run_build_project(
        &jobs,
        &plans,
        &parent_id,
        &build_request(&root),
        Some(&provider),
    )
    .unwrap();

    assert_eq!(report.summary.reused, 2);
    assert_eq!(report.summary.built, 0);
    assert_eq!(report.summary.failed, 0);
    assert!(children_of(&jobs, &parent_id).is_empty());
    assert_eq!(report.provider_usage.requests, 0);
    assert_eq!(provider.usage(), usage_after_first);
    for result in &report.results {
        assert_eq!(result.status, BuildResultStatusV1::Reused);
        assert!(result.child_job_id.is_none());
        assert!(result.pack_path.is_some());
        assert!(result.pack_sha256.is_some());
    }
}

// ---------------------------------------------------------------------------
// (c) Targeted invalidation rebuilds only the touched asset
// ---------------------------------------------------------------------------

#[test]
fn build_project_rebuilds_only_the_asset_whose_spec_changed() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("project");
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let provider = FixtureProvider::default();
    setup_project(&root, &plans, &jobs, &provider);
    write_character_spec(&root, "hero", "a brave knight");
    write_static_spec(&root, "hud-icons", "icon_set", &["a gold coin"]);
    write_static_spec(&root, "forest-props", "prop_set", &["a wooden crate"]);
    write_manifest(
        &root,
        &[
            ("hero", "character", ""),
            ("hud-icons", "icon_set", ""),
            ("forest-props", "prop_set", ""),
        ],
    );
    if !media_tools_available() {
        // Character-free variant keeps the invalidation semantics covered on
        // hosts without ffmpeg: two independent static sets, touch one.
        write_manifest(
            &root,
            &[
                ("hud-icons", "icon_set", ""),
                ("forest-props", "prop_set", ""),
            ],
        );
    }
    let (_first_parent, first) = run_build(&plans, &jobs, &root, &provider);
    let first = first.unwrap();
    assert_eq!(first.summary.failed, 0);

    // Touch exactly one icon's prompt: only the icon set may rebuild.
    write_static_spec(&root, "hud-icons", "icon_set", &["a silver coin"]);
    let (parent_id, report) = run_build(&plans, &jobs, &root, &provider);
    let report = report.unwrap();
    assert_eq!(report.summary.built, 1);
    assert_eq!(report.summary.reused as usize, report.results.len() - 1);
    let children = children_of(&jobs, &parent_id);
    assert_eq!(children.len(), 1);
    assert_eq!(
        children[0].operation_kind,
        JobOperationKind::GenerateStaticAssetSet
    );
    assert_eq!(children[0].asset_id.as_deref(), Some("hud-icons"));
    let icon_result = report
        .results
        .iter()
        .find(|result| result.asset_id == "hud-icons")
        .unwrap();
    assert_eq!(icon_result.status, BuildResultStatusV1::Succeeded);
    assert!(icon_result
        .reasons
        .iter()
        .any(|reason| reason == "spec_changed"));
    for result in &report.results {
        if result.asset_id != "hud-icons" {
            assert_eq!(result.status, BuildResultStatusV1::Reused);
        }
    }
}

// ---------------------------------------------------------------------------
// (d) Dependency behavior on failure
// ---------------------------------------------------------------------------

#[test]
fn build_project_failure_skips_dependents_but_not_unrelated_assets() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("project");
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let provider = FixtureProvider::default();
    setup_project(&root, &plans, &jobs, &provider);
    write_character_spec(&root, "hero", "a brave knight");
    // The magic prompt makes the fixture emit a two-subject image, which the
    // consistency hard gate blocks on every attempt.
    write_static_spec(
        &root,
        "hud-icons",
        "icon_set",
        &["a coin [fixture:hard_multiple_subjects]"],
    );
    write_static_spec(&root, "bonus-props", "prop_set", &["a wooden crate"]);
    write_manifest(
        &root,
        &[
            ("hero", "character", r#""dependsOn": ["hud-icons"]"#),
            ("hud-icons", "icon_set", ""),
            ("bonus-props", "prop_set", r#""required": false"#),
        ],
    );

    let parent_id = stage_parent(&plans, &jobs, &root);
    jobs.update_record(&parent_id, |record| {
        record.lifecycle_state = JobLifecycleState::Running;
        record.worker_pid = Some(std::process::id());
    })
    .unwrap();
    let error = run_build_project(
        &jobs,
        &plans,
        &parent_id,
        &build_request(&root),
        Some(&provider),
    )
    .unwrap_err();
    assert_eq!(error.code(), "project_build_failed");

    let parent = jobs.read_record(&parent_id).unwrap();
    let report = read_report(&parent.job_dir);
    let results = report["results"].as_array().unwrap();
    let status_of = |asset_id: &str| {
        results
            .iter()
            .find(|result| result["assetId"] == asset_id)
            .map(|result| result["status"].as_str().unwrap().to_string())
            .unwrap_or_else(|| panic!("no result for {asset_id}"))
    };
    assert_eq!(status_of("hud-icons"), "failed");
    assert_eq!(status_of("hero"), "skipped");
    let hero = results
        .iter()
        .find(|result| result["assetId"] == "hero")
        .unwrap();
    assert_eq!(hero["reasons"], serde_json::json!(["dependency_failed"]));
    // The optional, unrelated asset still builds.
    assert_eq!(status_of("bonus-props"), "succeeded");
    assert_eq!(report["summary"]["failed"], 1);
    assert_eq!(report["summary"]["skipped"], 1);
    assert_eq!(report["summary"]["built"], 1);

    // The failed icon set produced a child job stuck awaiting review; the
    // hero never produced one; the optional prop set built and registered.
    let children = children_of(&jobs, &parent_id);
    assert_eq!(children.len(), 2);
    let icon_child = children
        .iter()
        .find(|child| child.asset_id.as_deref() == Some("hud-icons"))
        .unwrap();
    assert_eq!(
        icon_child.lifecycle_state,
        JobLifecycleState::AwaitingReview
    );
    assert!(children
        .iter()
        .all(|child| child.asset_id.as_deref() != Some("hero")));
    let catalog = read_project_catalog(&root).unwrap();
    assert!(catalog.assets.contains_key("bonus-props"));
    assert!(!catalog.assets.contains_key("hud-icons"));
    assert!(!catalog.assets.contains_key("hero"));

    // Runner-level mapping: through run_operation_with_provider the parent
    // lands Failed with the stable code and recoverable=true.
    let prepared = plans
        .prepare(AutomationOperation::BuildProject(build_request(&root)))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let second = stage_plan_job(&jobs, &claimed).unwrap();
    let _env_guard = ENV_LOCK.lock().unwrap();
    let error = temp_env::with_var(
        "FORGE_PLAN_STORE",
        Some(temp.path().join("child-plans-2")),
        || run_operation_with_provider(&jobs, &second.job_id, &claimed.operation, Some(&provider)),
    )
    .unwrap_err();
    assert_eq!(error.code(), "project_build_failed");
    let record = jobs.read_record(&second.job_id).unwrap();
    assert_eq!(record.lifecycle_state, JobLifecycleState::Failed);
    assert_eq!(record.error_code.as_deref(), Some("project_build_failed"));
    assert!(record.recoverable);
    drop(_env_guard);
}

// ---------------------------------------------------------------------------
// Dependent rebuild: a rebuilt dependency forces a fresh child even when the
// same parent's build-state says the dependent already succeeded
// ---------------------------------------------------------------------------

#[test]
fn build_project_dependency_rebuilt_forces_fresh_dependent_child() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("project");
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let provider = FixtureProvider::default();
    setup_project(&root, &plans, &jobs, &provider);
    write_static_spec(&root, "shared-icons", "icon_set", &["a gold coin"]);
    write_static_spec(&root, "dependent-props", "prop_set", &["a wooden crate"]);
    write_manifest(
        &root,
        &[
            ("shared-icons", "icon_set", ""),
            (
                "dependent-props",
                "prop_set",
                r#""dependsOn": ["shared-icons"]"#,
            ),
        ],
    );

    let (parent_id, first) = run_build(&plans, &jobs, &root, &provider);
    assert_eq!(first.unwrap().summary.built, 2);
    let run_one_children: std::collections::BTreeSet<String> = children_of(&jobs, &parent_id)
        .into_iter()
        .map(|child| child.job_id)
        .collect();
    assert_eq!(run_one_children.len(), 2);

    // Touch the dependency's spec and retry with the SAME parent: the
    // dependent's build-state entry still says succeeded for the same spec
    // hash, but `dependency_rebuilt` must force a fresh child anyway.
    write_static_spec(&root, "shared-icons", "icon_set", &["a silver coin"]);
    jobs.update_record(&parent_id, |record| {
        record.lifecycle_state = JobLifecycleState::Running;
        record.worker_pid = Some(std::process::id());
    })
    .unwrap();
    let report = run_build_project(
        &jobs,
        &plans,
        &parent_id,
        &build_request(&root),
        Some(&provider),
    )
    .unwrap();
    assert_eq!(report.summary.built, 2);
    assert_eq!(report.summary.reused, 0);
    let new_children: Vec<_> = children_of(&jobs, &parent_id)
        .into_iter()
        .filter(|child| !run_one_children.contains(&child.job_id))
        .collect();
    assert_eq!(
        new_children.len(),
        2,
        "both assets rebuild with fresh children"
    );
    let dependent = report
        .results
        .iter()
        .find(|result| result.asset_id == "dependent-props")
        .unwrap();
    assert_eq!(dependent.status, BuildResultStatusV1::Succeeded);
    assert!(dependent
        .reasons
        .iter()
        .any(|reason| reason == "dependency_rebuilt"));
}

// ---------------------------------------------------------------------------
// (e) Cancellation: cascade + cooperative between-children check
// ---------------------------------------------------------------------------
#[test]
fn request_cancellation_cascade_flags_non_terminal_descendants_only() {
    let temp = tempdir().unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();

    let parent = jobs.create_job(SourceKind::FromCode).unwrap();
    let running_child = jobs.create_job(SourceKind::FromCode).unwrap();
    let terminal_child = jobs.create_job(SourceKind::FromCode).unwrap();
    let grandchild = jobs.create_job(SourceKind::FromCode).unwrap();
    jobs.update_record(&parent.job_id, |record| {
        record.lifecycle_state = JobLifecycleState::Running;
    })
    .unwrap();
    jobs.update_record(&running_child.job_id, |record| {
        record.parent_job_id = Some(parent.job_id.clone());
        record.lifecycle_state = JobLifecycleState::Running;
    })
    .unwrap();
    jobs.update_record(&terminal_child.job_id, |record| {
        record.parent_job_id = Some(parent.job_id.clone());
        record.lifecycle_state = JobLifecycleState::Succeeded;
    })
    .unwrap();
    jobs.update_record(&grandchild.job_id, |record| {
        record.parent_job_id = Some(running_child.job_id.clone());
        record.lifecycle_state = JobLifecycleState::Queued;
    })
    .unwrap();

    let flagged = jobs.request_cancellation_cascade(&parent.job_id).unwrap();
    assert_eq!(flagged, 3, "parent + running child + queued grandchild");
    assert!(
        jobs.read_record(&parent.job_id)
            .unwrap()
            .cancellation_requested
    );
    assert!(
        jobs.read_record(&running_child.job_id)
            .unwrap()
            .cancellation_requested
    );
    assert!(
        jobs.read_record(&grandchild.job_id)
            .unwrap()
            .cancellation_requested
    );
    assert!(
        !jobs
            .read_record(&terminal_child.job_id)
            .unwrap()
            .cancellation_requested,
        "terminal child stays untouched"
    );
}

#[test]
fn build_project_cancel_flag_skips_all_children_cooperatively() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("project");
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let provider = FixtureProvider::default();
    setup_project(&root, &plans, &jobs, &provider);
    write_static_spec(&root, "hud-icons", "icon_set", &["a gold coin"]);
    write_static_spec(&root, "forest-props", "prop_set", &["a wooden crate"]);
    write_manifest(
        &root,
        &[
            ("hud-icons", "icon_set", ""),
            ("forest-props", "prop_set", ""),
        ],
    );

    let parent_id = stage_parent(&plans, &jobs, &root);
    jobs.update_record(&parent_id, |record| {
        record.lifecycle_state = JobLifecycleState::Running;
        record.worker_pid = Some(std::process::id());
        record.cancellation_requested = true;
    })
    .unwrap();
    let error = run_build_project(
        &jobs,
        &plans,
        &parent_id,
        &build_request(&root),
        Some(&provider),
    )
    .unwrap_err();
    assert_eq!(error.code(), "cancelled");

    // No child was ever started; every asset is skipped with `cancelled`,
    // and the report still landed on disk for evidence.
    assert!(children_of(&jobs, &parent_id).is_empty());
    let parent = jobs.read_record(&parent_id).unwrap();
    let report = read_report(&parent.job_dir);
    for result in report["results"].as_array().unwrap() {
        assert_eq!(result["status"], "skipped");
        assert_eq!(result["reasons"], serde_json::json!(["cancelled"]));
    }
    assert_eq!(report["summary"]["skipped"], 2);
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(parent.job_dir.join(BUILD_STATE_FILE)).unwrap()).unwrap();
    for entry in state["assets"].as_array().unwrap() {
        assert_eq!(entry["status"], "skipped");
        assert_eq!(entry["error"], "cancelled");
    }
}

// ---------------------------------------------------------------------------
// (f) Crash recovery: worker reconciliation + build-state resume
// ---------------------------------------------------------------------------

#[test]
fn reconcile_marks_dead_workers_and_resume_skips_completed_assets() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("project");
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let provider = FixtureProvider::default();
    setup_project(&root, &plans, &jobs, &provider);
    write_static_spec(&root, "hud-icons", "icon_set", &["a gold coin"]);
    write_static_spec(
        &root,
        "forest-props",
        "prop_set",
        &["a crate [fixture:hard_multiple_subjects]"],
    );
    write_manifest(
        &root,
        &[
            ("hud-icons", "icon_set", ""),
            ("forest-props", "prop_set", ""),
        ],
    );

    // First run: the icon set succeeds, the prop set fails. build-state.json
    // is the partially-completed record; the catalog has only hud-icons.
    let parent_id = stage_parent(&plans, &jobs, &root);
    jobs.update_record(&parent_id, |record| {
        record.lifecycle_state = JobLifecycleState::Running;
        record.worker_pid = Some(std::process::id());
    })
    .unwrap();
    let error = run_build_project(
        &jobs,
        &plans,
        &parent_id,
        &build_request(&root),
        Some(&provider),
    )
    .unwrap_err();
    assert_eq!(error.code(), "project_build_failed");
    let parent = jobs.read_record(&parent_id).unwrap();
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(parent.job_dir.join(BUILD_STATE_FILE)).unwrap()).unwrap();
    let icon_state = state["assets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["assetId"] == "hud-icons")
        .cloned()
        .unwrap();
    assert_eq!(icon_state["status"], "succeeded");
    let icon_child_id = icon_state["childJobId"].as_str().unwrap().to_string();
    let icon_pack_sha = icon_state["packSha256"].as_str().unwrap().to_string();

    // Fabricate a crashed build worker: a Running build_project job whose
    // recorded pid already exited. Reconciliation fails it recoverably.
    let mut dead = Command::new("true").spawn().unwrap();
    let dead_pid = dead.id();
    dead.wait().unwrap();
    let crashed = jobs.create_job(SourceKind::FromCode).unwrap();
    jobs.update_record(&crashed.job_id, |record| {
        record.operation_kind = JobOperationKind::BuildProject;
        record.lifecycle_state = JobLifecycleState::Running;
        record.worker_pid = Some(dead_pid);
    })
    .unwrap();
    let reconciled = reconcile_interrupted_builds(&jobs).unwrap();
    assert_eq!(reconciled, 1, "the live parent (this pid) must be skipped");
    let crashed = jobs.read_record(&crashed.job_id).unwrap();
    assert_eq!(crashed.lifecycle_state, JobLifecycleState::Failed);
    assert_eq!(crashed.error_code.as_deref(), Some("worker_lost"));
    assert!(crashed.recoverable);
    assert_eq!(
        crashed.next_actions,
        vec!["prepare_new_plan".to_string(), "job_report".to_string()]
    );

    // Simulate a crash between child success and catalog registration: the
    // catalog loses the hud-icons entry, so only build-state.json can prove
    // the asset is done. The prop set spec is fixed (new spec hash).
    let catalog_path = root.join(PROJECT_CATALOG_RELATIVE);
    let mut catalog_json: serde_json::Value =
        serde_json::from_slice(&fs::read(&catalog_path).unwrap()).unwrap();
    catalog_json["assets"]
        .as_object_mut()
        .unwrap()
        .remove("hud-icons");
    fs::write(
        &catalog_path,
        serde_json::to_vec_pretty(&catalog_json).unwrap(),
    )
    .unwrap();
    write_static_spec(&root, "forest-props", "prop_set", &["a wooden crate"]);

    let prior_child_ids: std::collections::BTreeSet<String> = children_of(&jobs, &parent_id)
        .into_iter()
        .map(|child| child.job_id)
        .collect();

    // Retry with the SAME parent job: the completed icon set is skipped
    // without a child (build-state resume) and re-registered from the
    // recorded pack facts; the prop set builds fresh.
    jobs.update_record(&parent_id, |record| {
        record.lifecycle_state = JobLifecycleState::Running;
        record.worker_pid = Some(std::process::id());
    })
    .unwrap();
    let report = run_build_project(
        &jobs,
        &plans,
        &parent_id,
        &build_request(&root),
        Some(&provider),
    )
    .unwrap();
    assert_eq!(report.summary.failed, 0);
    assert_eq!(report.summary.skipped, 0);

    let new_children: Vec<_> = children_of(&jobs, &parent_id)
        .into_iter()
        .filter(|child| !prior_child_ids.contains(&child.job_id))
        .collect();
    assert_eq!(new_children.len(), 1, "only the prop set gets a new child");
    assert_eq!(new_children[0].asset_id.as_deref(), Some("forest-props"));

    let icon_result = report
        .results
        .iter()
        .find(|result| result.asset_id == "hud-icons")
        .unwrap();
    assert_eq!(icon_result.status, BuildResultStatusV1::Succeeded);
    assert_eq!(
        icon_result.child_job_id.as_deref(),
        Some(icon_child_id.as_str()),
        "resumed asset keeps its original child job id"
    );
    assert_eq!(
        icon_result.pack_sha256.as_deref(),
        Some(icon_pack_sha.as_str())
    );

    // The catalog was healed from build-state: hud-icons is back with the
    // same pack hash, so the next diff reuses it.
    let catalog = read_project_catalog(&root).unwrap();
    let icon_entry = catalog.assets.get("hud-icons").unwrap();
    assert_eq!(icon_entry.pack_sha256, icon_pack_sha);
    assert_eq!(icon_entry.source_job_id, icon_child_id);
    assert!(catalog.assets.contains_key("forest-props"));
}

// ---------------------------------------------------------------------------
// Dispatch wiring through the runner (FORGE_PLAN_STORE honoured)
// ---------------------------------------------------------------------------

#[test]
fn runner_dispatch_executes_build_project_end_to_end() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("project");
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let provider = FixtureProvider::default();
    setup_project(&root, &plans, &jobs, &provider);
    write_static_spec(&root, "hud-icons", "icon_set", &["a gold coin"]);
    write_manifest(&root, &[("hud-icons", "icon_set", "")]);

    let prepared = plans
        .prepare(AutomationOperation::BuildProject(build_request(&root)))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let parent = stage_plan_job(&jobs, &claimed).unwrap();

    let child_plans = temp.path().join("child-plans");
    let _env_guard = ENV_LOCK.lock().unwrap();
    let completed = temp_env::with_var("FORGE_PLAN_STORE", Some(&child_plans), || {
        run_operation_with_provider(&jobs, &parent.job_id, &claimed.operation, Some(&provider))
    })
    .unwrap();
    assert_eq!(completed.lifecycle_state, JobLifecycleState::Succeeded);
    assert!(completed
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "project_build_report"));
    // Children were planned through the env-pointed PlanStore, not the
    // default app store.
    assert!(fs::read_dir(&child_plans).unwrap().any(|entry| entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .ends_with(".claimed.json")));
    let children = children_of(&jobs, &parent.job_id);
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].lifecycle_state, JobLifecycleState::Succeeded);
    assert!(read_project_catalog(&root)
        .unwrap()
        .assets
        .contains_key("hud-icons"));
    drop(_env_guard);
}
