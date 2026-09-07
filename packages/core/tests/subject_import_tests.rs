#![cfg(feature = "subject-import")]

use std::fs;
use std::path::{Path, PathBuf};

use forge_core::asset_project::{
    hash_file, init_project, PaletteColor, SamplingMode, StyleBaseline, StyleLockV1,
    FORGE_PROJECT_FILE, STYLE_LOCK_FILE,
};
use forge_core::automation::{
    fingerprint_operation_inputs, run_operation, stage_plan_job, AutomationOperation,
    CreateSubjectLockRequest, PlanStore,
};
use forge_core::job::{JobLifecycleState, JobStore};
use forge_core::subject::{
    read_subject_lock, SubjectOriginKindV1, SubjectSpecV1, SUBJECT_IMPORT_REPORT_FILE,
};
use image::{ImageBuffer, Rgba, RgbaImage};

struct Context {
    _temp: tempfile::TempDir,
    project: PathBuf,
    subject_spec: PathBuf,
    canonical: PathBuf,
    plans: PlanStore,
    jobs: JobStore,
}

fn setup(prompt: &str) -> Context {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    let mut manifest = init_project(&project, "Subject Import Contract").unwrap();
    manifest.provider.id = "fixture".into();
    manifest.provider.profile_id = "default".into();
    let style_revision = "import-style-v1";
    manifest.current_style_revision = Some(style_revision.into());
    fs::write(
        project.join(FORGE_PROJECT_FILE),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let style_dir = project.join(".forge/styles").join(style_revision);
    fs::create_dir_all(&style_dir).unwrap();
    let board_path = style_dir.join("style-board.png");
    RgbaImage::from_pixel(64, 64, Rgba([72, 96, 64, 255]))
        .save(&board_path)
        .unwrap();
    let style = StyleLockV1 {
        schema_version: "1".into(),
        revision: style_revision.into(),
        provider_id: "fixture".into(),
        profile_id: "default".into(),
        image_model: Some("fixture-image-v1".into()),
        prompt: "compact painted forest game art".into(),
        perspective: "topdown".into(),
        lighting: "soft".into(),
        outline: "dark".into(),
        background: "transparent".into(),
        sampling: SamplingMode::Nearest,
        character_canvas_size: 256,
        icon_canvas_size: 128,
        prop_canvas_size: 256,
        board_path: board_path.clone(),
        board_sha256: hash_file(&board_path).unwrap(),
        reference_sha256: vec![],
        baseline_profile: "style-baseline@2.3.0".into(),
        migrated_from_revision: None,
        baseline: StyleBaseline {
            palette: vec![PaletteColor {
                color: "#486040".into(),
                weight: 1.0,
            }],
            edge_density: 1.0,
            foreground_scale: 1.0,
            perceptual_hash: "0000000000000000".into(),
        },
    };
    fs::write(
        style_dir.join(STYLE_LOCK_FILE),
        serde_json::to_vec_pretty(&style).unwrap(),
    )
    .unwrap();
    let subject_spec = temp.path().join("subject.json");
    fs::write(
        &subject_spec,
        serde_json::to_vec_pretty(&SubjectSpecV1 {
            schema_version: "1".into(),
            id: "approved-subject".into(),
            name: "Approved Subject".into(),
            prompt: prompt.into(),
            reference_images: vec![],
            image_model: None,
            license: "private".into(),
        })
        .unwrap(),
    )
    .unwrap();
    let canonical = temp.path().join("approved.png");
    draw_subject(&canonical, true, false);
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    Context {
        _temp: temp,
        project,
        subject_spec,
        canonical,
        plans,
        jobs,
    }
}

fn draw_subject(path: &Path, transparent: bool, two_subjects: bool) {
    let background: Rgba<u8> = if transparent {
        Rgba([0, 0, 0, 0])
    } else {
        Rgba([220, 220, 220, 255])
    };
    let mut image = ImageBuffer::from_pixel(256, 256, background);
    for y in 48..224 {
        for x in 84..172 {
            image.put_pixel(x, y, Rgba([92_u8, 116, 70, 255]));
        }
    }
    if two_subjects {
        for y in 96..196 {
            for x in 24..56 {
                image.put_pixel(x, y, Rgba([120_u8, 72, 48, 255]));
            }
        }
    }
    image.save(path).unwrap();
}

fn request(context: &Context) -> CreateSubjectLockRequest {
    CreateSubjectLockRequest {
        schema_version: "1".into(),
        project_path: context.project.clone(),
        spec_path: context.subject_spec.clone(),
        provider_id: "fixture".into(),
        profile_id: "default".into(),
        canonical_import_path: Some(context.canonical.clone()),
        import_approval_note: Some("human approved existing canonical".into()),
    }
}

fn run_import_result(
    context: &Context,
) -> Result<forge_core::job::JobRecord, forge_core::automation::AutomationRunError> {
    let prepared = context
        .plans
        .prepare(AutomationOperation::CreateSubjectLock(request(context)))
        .unwrap();
    assert_eq!(prepared.estimate.provider_request_estimate, 0);
    assert_eq!(prepared.estimate.maximum_provider_requests, 0);
    assert!(prepared.estimate.provider_id.is_none());
    let claimed = context.plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&context.jobs, &claimed).unwrap();
    run_operation(&context.jobs, &job.job_id, &claimed.operation)
}

fn run_import(context: &Context) -> forge_core::job::JobRecord {
    run_import_result(context).unwrap()
}

#[test]
fn approved_canonical_import_is_zero_request_deterministic_and_auditable() {
    let context = setup("a fully helmeted clockwork guardian");
    let first = run_import(&context);
    assert_eq!(first.lifecycle_state, JobLifecycleState::Succeeded);
    assert!(first.authorization_id.is_none());
    assert!(!first
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "provider_usage"));
    let lock_path = first
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "subject_lock")
        .unwrap()
        .path
        .clone();
    let first_lock = read_subject_lock(&lock_path).unwrap();
    assert_eq!(first_lock.origin.kind, SubjectOriginKindV1::LocalImport);
    assert_eq!(
        first_lock.canonical_sha256,
        hash_file(&context.canonical).unwrap()
    );
    assert!(first_lock.origin.report_path.as_ref().unwrap().is_file());

    let second = run_import(&context);
    let second_lock = read_subject_lock(
        &second
            .artifacts
            .iter()
            .find(|artifact| artifact.kind == "subject_lock")
            .unwrap()
            .path,
    )
    .unwrap();
    assert_eq!(first_lock.revision, second_lock.revision);
    assert_eq!(first_lock.canonical_sha256, second_lock.canonical_sha256);
    assert_eq!(first_lock.mask_sha256, second_lock.mask_sha256);

    fs::write(
        lock_path.parent().unwrap().join(SUBJECT_IMPORT_REPORT_FILE),
        b"{}",
    )
    .unwrap();
    assert!(read_subject_lock(&lock_path)
        .unwrap_err()
        .to_string()
        .contains("import report changed"));
}

#[test]
fn import_fingerprints_the_canonical_and_rejects_opaque_or_multiple_subjects() {
    let context = setup("an animal companion without a visible human face");
    let operation = AutomationOperation::CreateSubjectLock(request(&context));
    let before = fingerprint_operation_inputs(&operation).unwrap();
    draw_subject(&context.canonical, true, true);
    let after = fingerprint_operation_inputs(&operation).unwrap();
    assert_ne!(before, after);

    let multi = run_import_result(&context).unwrap_err();
    assert!(multi.to_string().contains("single-subject"));

    let opaque_context = setup("an animal companion without a visible human face");
    draw_subject(&opaque_context.canonical, false, false);
    let opaque = run_import_result(&opaque_context).unwrap_err();
    assert!(opaque.to_string().contains("transparent canonical"));
}
