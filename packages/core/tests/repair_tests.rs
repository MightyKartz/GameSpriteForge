use std::fs;

use forge_core::automation::{
    analyze_repair, prepare_repair_plan, stage_plan_job, write_repair_comparison,
    AutomationOperation, PlanStore, RepairError,
};
use forge_core::export::{AnimationQualityEntry, CharacterQualityReport};
use forge_core::job::{
    JobLifecycleState, JobStore, RepairAnimationQuality, RepairContext, RepairQualitySnapshot,
};
use forge_core::quality::{QualityMetrics, QualityRecommendationId, QualityReport, QualityVerdict};
use tempfile::tempdir;

#[test]
fn repair_plan_applies_safe_chroma_change_and_links_the_new_job() {
    let temp = tempdir().unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let frame_a = temp.path().join("frame_a.png");
    let frame_b = temp.path().join("frame_b.png");
    fs::write(&frame_a, b"a").unwrap();
    fs::write(&frame_b, b"b").unwrap();
    let operation: AutomationOperation = serde_json::from_value(serde_json::json!({
        "kind": "prepare_asset",
        "request": {
            "schemaVersion": "1",
            "input": { "kind": "png_sequence", "paths": [frame_a, frame_b] },
            "metadata": { "name": "Repair Knight", "animation": "idle" },
            "matting": {
                "mode": "auto_corners",
                "keyMode": "auto_corners",
                "manualKeyColor": "#00FF00",
                "threshold": 100,
                "softness": 18,
                "despillStrength": 0.5,
                "haloPixels": 0
            }
        }
    }))
    .unwrap();
    let source = jobs
        .create_job(forge_core::job::SourceKind::ImportFrames)
        .unwrap();
    jobs.update_record(&source.job_id, |record| {
        record.lifecycle_state = JobLifecycleState::AwaitingReview;
        record.recipe = Some(serde_json::to_value(&operation).unwrap());
    })
    .unwrap();
    fs::write(
        source.job_dir.join("quality-report.json"),
        serde_json::to_vec_pretty(&quality_report(
            QualityVerdict::Blocked,
            vec![QualityRecommendationId::ReduceChromaThreshold],
        ))
        .unwrap(),
    )
    .unwrap();

    let analysis = analyze_repair(&jobs, &source.job_id).unwrap();
    assert!(analysis.can_auto_repair);
    assert_eq!(analysis.attempt, 1);
    assert_eq!(analysis.changes.len(), 1);
    assert_eq!(analysis.changes[0].before, 100);
    assert_eq!(analysis.changes[0].after, 88);

    let prepared = prepare_repair_plan(&plans, &jobs, &source.job_id).unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    assert_eq!(
        claimed.repair.as_ref().unwrap().source_job_id,
        source.job_id
    );
    let repair_job = stage_plan_job(&jobs, &claimed).unwrap();
    assert_eq!(
        repair_job.repair.as_ref().unwrap().source_job_id,
        source.job_id
    );

    let after = RepairQualitySnapshot {
        verdict: QualityVerdict::GameReady,
        animations: vec![RepairAnimationQuality {
            name: "idle".into(),
            report: quality_report(QualityVerdict::GameReady, vec![]),
        }],
    };
    let artifact = write_repair_comparison(&repair_job, after)
        .unwrap()
        .unwrap();
    let comparison: serde_json::Value =
        serde_json::from_slice(&fs::read(artifact.path).unwrap()).unwrap();
    assert_eq!(comparison["improved"], true);
    assert_eq!(comparison["attempt"], 1);
    assert!(matches!(
        analyze_repair(&jobs, &source.job_id),
        Err(RepairError::AlreadyRepaired(_))
    ));
}

#[test]
fn repair_analysis_keeps_semantic_changes_manual_and_caps_attempts() {
    let temp = tempdir().unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let operation: AutomationOperation = serde_json::from_value(serde_json::json!({
        "kind": "prepare_asset",
        "request": {
            "schemaVersion": "1",
            "input": { "kind": "png_sequence", "paths": ["/tmp/a.png", "/tmp/b.png"] },
            "metadata": { "name": "Loop", "animation": "idle" },
            "matting": { "mode": "preserve_alpha" }
        }
    }))
    .unwrap();
    let source = jobs
        .create_job(forge_core::job::SourceKind::ImportFrames)
        .unwrap();
    jobs.update_record(&source.job_id, |record| {
        record.lifecycle_state = JobLifecycleState::AwaitingReview;
        record.recipe = Some(serde_json::to_value(&operation).unwrap());
    })
    .unwrap();
    fs::write(
        source.job_dir.join("quality-report.json"),
        serde_json::to_vec_pretty(&quality_report(
            QualityVerdict::NeedsCleanup,
            vec![
                QualityRecommendationId::TrimLoopRange,
                QualityRecommendationId::UseShorterClip,
            ],
        ))
        .unwrap(),
    )
    .unwrap();

    let analysis = analyze_repair(&jobs, &source.job_id).unwrap();
    assert!(!analysis.can_auto_repair);
    assert!(analysis
        .manual_actions
        .contains(&"asset:review_loop_range".to_string()));
    assert!(analysis
        .manual_actions
        .contains(&"asset:choose_shorter_frame_range".to_string()));
    assert!(matches!(
        prepare_repair_plan(
            &PlanStore::new(temp.path().join("plans")).unwrap(),
            &jobs,
            &source.job_id
        ),
        Err(RepairError::NoAutomaticChanges)
    ));

    jobs.update_record(&source.job_id, |record| {
        record.repair = Some(RepairContext {
            source_job_id: "root".into(),
            attempt: 3,
            changes: vec![],
            baseline: RepairQualitySnapshot {
                verdict: QualityVerdict::Blocked,
                animations: vec![],
            },
        });
    })
    .unwrap();
    assert!(matches!(
        analyze_repair(&jobs, &source.job_id),
        Err(RepairError::AttemptLimit)
    ));
}

#[test]
fn character_repair_scopes_matting_changes_to_the_affected_animation() {
    let temp = tempdir().unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let operation: AutomationOperation = serde_json::from_value(serde_json::json!({
        "kind": "prepare_character_pack",
        "request": {
            "schemaVersion": "2",
            "metadata": { "name": "Knight", "defaultAnimation": "idle" },
            "workflow": { "id": "custom", "version": "1.0.0" },
            "animations": [
                {
                    "name": "idle",
                    "input": { "kind": "png_sequence", "paths": ["/tmp/idle-a.png", "/tmp/idle-b.png"] },
                    "fps": 8,
                    "loop": true,
                    "matting": {
                        "mode": "auto_corners", "keyMode": "auto_corners", "manualKeyColor": "#00FF00",
                        "threshold": 48, "softness": 18, "despillStrength": 0.5, "haloPixels": 0
                    }
                },
                {
                    "name": "attack",
                    "input": { "kind": "png_sequence", "paths": ["/tmp/attack-a.png", "/tmp/attack-b.png"] },
                    "fps": 12,
                    "loop": false,
                    "matting": {
                        "mode": "auto_corners", "keyMode": "auto_corners", "manualKeyColor": "#00FF00",
                        "threshold": 60, "softness": 18, "despillStrength": 0.5, "haloPixels": 0
                    }
                }
            ]
        }
    }))
    .unwrap();
    let source = jobs
        .create_job(forge_core::job::SourceKind::ImportFrames)
        .unwrap();
    jobs.update_record(&source.job_id, |record| {
        record.lifecycle_state = JobLifecycleState::AwaitingReview;
        record.recipe = Some(serde_json::to_value(&operation).unwrap());
    })
    .unwrap();
    let character_quality = CharacterQualityReport {
        quality_profile: "animation-quality@2.0.0".into(),
        verdict: QualityVerdict::Blocked,
        default_animation: "idle".into(),
        frame_count: 4,
        animations: vec![
            AnimationQualityEntry {
                name: "idle".into(),
                report: quality_report(QualityVerdict::GameReady, vec![]),
                loop_selection_report: None,
                loop_selection: None,
            },
            AnimationQualityEntry {
                name: "attack".into(),
                report: quality_report(
                    QualityVerdict::Blocked,
                    vec![QualityRecommendationId::ReduceChromaThreshold],
                ),
                loop_selection_report: None,
                loop_selection: None,
            },
        ],
    };
    fs::write(
        source.job_dir.join("animation-quality-report.json"),
        serde_json::to_vec_pretty(&character_quality).unwrap(),
    )
    .unwrap();

    let analysis = analyze_repair(&jobs, &source.job_id).unwrap();
    assert_eq!(analysis.changes.len(), 1);
    assert_eq!(analysis.changes[0].scope, "animation:attack");
    let proposed = match analysis.proposed_operation.unwrap() {
        AutomationOperation::PrepareCharacterPack(request) => request,
        _ => panic!("expected Character Pack repair"),
    };
    let thresholds = proposed
        .animations
        .iter()
        .map(|animation| match &animation.matting {
            forge_core::automation::MattingRecipe::AutoCorners { parameters } => {
                (animation.name.as_str(), parameters.threshold)
            }
            _ => panic!("expected auto-corners matting"),
        })
        .collect::<Vec<_>>();
    assert_eq!(thresholds, vec![("idle", 48), ("attack", 48)]);
}

#[test]
fn preserved_single_animation_repair_keeps_coordinate_changes_manual() {
    assert_preserved_repair_contract(false);
}

#[test]
fn preserved_character_repair_keeps_coordinate_changes_manual() {
    assert_preserved_repair_contract(true);
}

fn assert_preserved_repair_contract(character: bool) {
    for recommendations in [
        vec![QualityRecommendationId::AdjustAnchor],
        vec![QualityRecommendationId::IncreaseCanvasMargin],
        vec![
            QualityRecommendationId::AdjustAnchor,
            QualityRecommendationId::IncreaseCanvasMargin,
        ],
    ] {
        // A separate safe matting repair must remain available without silently
        // adding margins or translating preserved frames in the same plan.
        for repair_matting in [false, true] {
            let temp = tempdir().unwrap();
            let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
            let plans = PlanStore::new(temp.path().join("plans")).unwrap();
            let paths: Vec<_> = (0..2)
                .map(|index| {
                    let path = temp.path().join(format!("frame-{index}.png"));
                    image::RgbaImage::from_pixel(64, 64, image::Rgba([30, 80, 120, 255]))
                        .save(&path)
                        .unwrap();
                    path
                })
                .collect();
            let matting = serde_json::json!({
                "mode": "auto_corners", "keyMode": "auto_corners", "manualKeyColor": "#00FF00",
                "threshold": 60, "softness": 18, "despillStrength": 0.5, "haloPixels": 0
            });
            let mut request = serde_json::json!({
                "schemaVersion": "1",
                "input": {"kind": "png_sequence", "paths": paths},
                "metadata": {"name": "Preserved", "animation": "idle", "frameDurationsMs": [70, 150]},
                "normalize": {"mode": "preserve_source", "margin": 0, "marginBottom": 0,
                    "alphaThreshold": 0, "manualAnchor": {"x": 32.5, "y": 52.25, "lockedByUser": true}},
                "rendering": {"textureFilter": "linear", "pixelSnap": false},
                "matting": matting
            });
            if character {
                let input = request.as_object_mut().unwrap().remove("input").unwrap();
                request.as_object_mut().unwrap().remove("matting");
                request["schemaVersion"] = serde_json::json!("2");
                request["metadata"] =
                    serde_json::json!({"name": "Preserved", "defaultAnimation": "idle"});
                request["animations"] = serde_json::json!([
                    {"name": "idle", "input": input, "frameDurationsMs": [70, 150], "matting": matting},
                    {"name": "attack", "input": input, "frameDurationsMs": [90, 180], "matting": matting}
                ]);
            }
            let operation: AutomationOperation = serde_json::from_value(serde_json::json!({
                "kind": if character { "prepare_character_pack" } else { "prepare_asset" },
                "request": request
            }))
            .unwrap();
            let source = jobs
                .create_job(forge_core::job::SourceKind::ImportFrames)
                .unwrap();
            jobs.update_record(&source.job_id, |record| {
                record.lifecycle_state = JobLifecycleState::AwaitingReview;
                record.recipe = Some(serde_json::to_value(&operation).unwrap());
            })
            .unwrap();
            let mut reported = recommendations.clone();
            if repair_matting {
                reported.push(QualityRecommendationId::ReduceChromaThreshold);
            }
            let report = quality_report(QualityVerdict::NeedsCleanup, reported);
            if character {
                fs::write(
                    source.job_dir.join("animation-quality-report.json"),
                    serde_json::to_vec_pretty(&CharacterQualityReport {
                        quality_profile: "animation-quality@2.0.0".into(),
                        verdict: QualityVerdict::NeedsCleanup,
                        default_animation: "idle".into(),
                        frame_count: 4,
                        animations: vec![
                            AnimationQualityEntry {
                                name: "idle".into(),
                                report: quality_report(QualityVerdict::GameReady, vec![]),
                                loop_selection_report: None,
                                loop_selection: None,
                            },
                            AnimationQualityEntry {
                                name: "attack".into(),
                                report,
                                loop_selection_report: None,
                                loop_selection: None,
                            },
                        ],
                    })
                    .unwrap(),
                )
                .unwrap();
            } else {
                fs::write(
                    source.job_dir.join("quality-report.json"),
                    serde_json::to_vec_pretty(&report).unwrap(),
                )
                .unwrap();
            }

            let analysis = analyze_repair(&jobs, &source.job_id).unwrap();
            assert_eq!(analysis.can_auto_repair, repair_matting);
            assert_eq!(analysis.manual_actions.len(), recommendations.len());
            for recommendation in &recommendations {
                let action = match recommendation {
                    QualityRecommendationId::AdjustAnchor => "shared:inspect_anchor_drift",
                    QualityRecommendationId::IncreaseCanvasMargin => {
                        "shared:review_source_canvas_boundaries"
                    }
                    _ => unreachable!(),
                };
                assert!(analysis.manual_actions.iter().any(|item| item == action));
            }
            if repair_matting {
                assert_eq!(analysis.changes.len(), 1);
                assert_eq!(analysis.changes[0].parameter, "matting.threshold");
                let mut expected = serde_json::to_value(&operation).unwrap();
                let threshold = if character {
                    "/request/animations/1/matting/threshold"
                } else {
                    "/request/matting/threshold"
                };
                *expected.pointer_mut(threshold).unwrap() = serde_json::json!(48);
                assert_eq!(
                    serde_json::to_value(analysis.proposed_operation.as_ref().unwrap()).unwrap(),
                    expected
                );
                let prepared = prepare_repair_plan(&plans, &jobs, &source.job_id).unwrap();
                let claimed = plans.claim(&prepared.token).unwrap();
                assert_eq!(serde_json::to_value(&claimed.operation).unwrap(), expected);
            } else {
                assert!(analysis.changes.is_empty());
                assert!(analysis.proposed_operation.is_none());
                assert!(matches!(
                    prepare_repair_plan(&plans, &jobs, &source.job_id),
                    Err(RepairError::NoAutomaticChanges)
                ));
            }
            assert_eq!(
                jobs.read_record(&source.job_id).unwrap().recipe,
                Some(serde_json::to_value(&operation).unwrap())
            );
        }
    }
}

fn quality_report(
    verdict: QualityVerdict,
    recommendations: Vec<QualityRecommendationId>,
) -> QualityReport {
    QualityReport {
        pixel_diagnostics: None,
        verdict,
        metrics: QualityMetrics {
            bbox_bottom_drift_px: if verdict == QualityVerdict::GameReady {
                0.0
            } else {
                4.0
            },
            bbox_center_x_drift_px: 0.0,
            bbox_center_y_drift_px: 0.0,
            bbox_width_variation_px: 0.0,
            alpha_coverage_avg: 0.01,
            loop_match_score: if verdict == QualityVerdict::GameReady {
                1.0
            } else {
                0.4
            },
            frame_count: 4,
            frame_size_consistent: true,
            cell_boundary_safe: true,
        },
        recommendations,
        notes: vec![],
    }
}
