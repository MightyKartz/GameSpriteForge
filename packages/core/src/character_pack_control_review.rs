use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const CHARACTER_PACK_CONTROL_REVIEW_PROFILE: &str = "character-pack-control-human-review@1.0.0";
pub const CHARACTER_PACK_CONTROL_REVIEW_SCOPE_V1: [&str; 6] = [
    "idle_right_to_walk_right_transition",
    "walk_right_to_idle_right_transition",
    "shared_pivot_visual_continuity",
    "speed_and_gait_sync",
    "wall_stop_without_foot_sliding",
    "camera_follow_and_repeated_start_stop",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterPackControlReviewStatusV1 {
    Approved,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterPackControlReviewChecksV1 {
    pub idle_to_walk_transition_acceptable: bool,
    pub walk_to_idle_transition_acceptable: bool,
    pub shared_pivot_visual_continuity_acceptable: bool,
    pub speed_and_gait_sync_acceptable: bool,
    pub wall_stop_without_foot_sliding_acceptable: bool,
    pub camera_follow_and_repeated_start_stop_acceptable: bool,
}

impl CharacterPackControlReviewChecksV1 {
    pub const fn approved() -> Self {
        Self {
            idle_to_walk_transition_acceptable: true,
            walk_to_idle_transition_acceptable: true,
            shared_pivot_visual_continuity_acceptable: true,
            speed_and_gait_sync_acceptable: true,
            wall_stop_without_foot_sliding_acceptable: true,
            camera_follow_and_repeated_start_stop_acceptable: true,
        }
    }

    fn all_passed(&self) -> bool {
        self.idle_to_walk_transition_acceptable
            && self.walk_to_idle_transition_acceptable
            && self.shared_pivot_visual_continuity_acceptable
            && self.speed_and_gait_sync_acceptable
            && self.wall_stop_without_foot_sliding_acceptable
            && self.camera_follow_and_repeated_start_stop_acceptable
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterPackControlReviewClosureV1 {
    pub godot_evidence_sha256: String,
    pub candidate_pack_forgepack_sha256: String,
    pub candidate_pack_manifest_sha256: String,
    pub candidate_pack_inventory_sha256: String,
    pub candidate_pack_content_sha256: String,
    pub candidate_pack_provenance_sha256: String,
    pub idle_right_approved_review_sha256: String,
    pub walk_right_approved_review_sha256: String,
    pub runtime_report_sha256: String,
    pub authoritative_movie_sha256: String,
    pub delivery_movie_sha256: String,
    pub human_review_scope: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterPackControlHumanReviewV1 {
    pub schema_version: String,
    pub profile: String,
    pub reviewer: String,
    pub status: CharacterPackControlReviewStatusV1,
    pub checks: CharacterPackControlReviewChecksV1,
    pub review_note: String,
    pub reviewed_at: DateTime<Utc>,
    pub godot_evidence_sha256: String,
    pub candidate_pack_forgepack_sha256: String,
    pub candidate_pack_manifest_sha256: String,
    pub candidate_pack_inventory_sha256: String,
    pub candidate_pack_content_sha256: String,
    pub candidate_pack_provenance_sha256: String,
    pub idle_right_approved_review_sha256: String,
    pub walk_right_approved_review_sha256: String,
    pub runtime_report_sha256: String,
    pub authoritative_movie_sha256: String,
    pub delivery_movie_sha256: String,
    pub human_review_scope: Vec<String>,
}

#[derive(Debug, Error)]
pub enum CharacterPackControlReviewError {
    #[error("character Pack control review is unsupported or incomplete")]
    InvalidReview,
    #[error("character Pack control review no longer matches its immutable evidence closure")]
    ClosureMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterPackControlAnimationTimingV1 {
    pub frame_durations_ms: Vec<u64>,
    pub cycle_duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterPackControlRuntimeContractV1 {
    pub idle_right: CharacterPackControlAnimationTimingV1,
    pub walk_right: CharacterPackControlAnimationTimingV1,
    pub rendering: Value,
    pub anchor_x: f64,
    pub anchor_y: f64,
    pub collision_width: f64,
    pub collision_height: f64,
    pub collision_offset_x: f64,
    pub collision_offset_y: f64,
    pub visual_scale: f64,
    pub move_speed_pixels_per_second: f64,
}

pub fn approve_character_pack_control_review(
    closure: &CharacterPackControlReviewClosureV1,
    review_note: impl Into<String>,
    reviewed_at: DateTime<Utc>,
) -> Result<CharacterPackControlHumanReviewV1, CharacterPackControlReviewError> {
    validate_closure_shape(closure)?;
    let review = CharacterPackControlHumanReviewV1 {
        schema_version: "1".into(),
        profile: CHARACTER_PACK_CONTROL_REVIEW_PROFILE.into(),
        reviewer: "human".into(),
        status: CharacterPackControlReviewStatusV1::Approved,
        checks: CharacterPackControlReviewChecksV1::approved(),
        review_note: review_note.into(),
        reviewed_at,
        godot_evidence_sha256: closure.godot_evidence_sha256.clone(),
        candidate_pack_forgepack_sha256: closure.candidate_pack_forgepack_sha256.clone(),
        candidate_pack_manifest_sha256: closure.candidate_pack_manifest_sha256.clone(),
        candidate_pack_inventory_sha256: closure.candidate_pack_inventory_sha256.clone(),
        candidate_pack_content_sha256: closure.candidate_pack_content_sha256.clone(),
        candidate_pack_provenance_sha256: closure.candidate_pack_provenance_sha256.clone(),
        idle_right_approved_review_sha256: closure.idle_right_approved_review_sha256.clone(),
        walk_right_approved_review_sha256: closure.walk_right_approved_review_sha256.clone(),
        runtime_report_sha256: closure.runtime_report_sha256.clone(),
        authoritative_movie_sha256: closure.authoritative_movie_sha256.clone(),
        delivery_movie_sha256: closure.delivery_movie_sha256.clone(),
        human_review_scope: closure.human_review_scope.clone(),
    };
    validate_character_pack_control_review_closure(&review, closure)?;
    Ok(review)
}

pub fn character_pack_control_review_is_approved(
    review: &CharacterPackControlHumanReviewV1,
) -> bool {
    review.status == CharacterPackControlReviewStatusV1::Approved && review.checks.all_passed()
}

pub fn validate_character_pack_control_review_closure(
    review: &CharacterPackControlHumanReviewV1,
    closure: &CharacterPackControlReviewClosureV1,
) -> Result<(), CharacterPackControlReviewError> {
    validate_review_shape(review)?;
    validate_closure_shape(closure)?;
    if review.godot_evidence_sha256 != closure.godot_evidence_sha256
        || review.candidate_pack_forgepack_sha256 != closure.candidate_pack_forgepack_sha256
        || review.candidate_pack_manifest_sha256 != closure.candidate_pack_manifest_sha256
        || review.candidate_pack_inventory_sha256 != closure.candidate_pack_inventory_sha256
        || review.candidate_pack_content_sha256 != closure.candidate_pack_content_sha256
        || review.candidate_pack_provenance_sha256 != closure.candidate_pack_provenance_sha256
        || review.idle_right_approved_review_sha256 != closure.idle_right_approved_review_sha256
        || review.walk_right_approved_review_sha256 != closure.walk_right_approved_review_sha256
        || review.runtime_report_sha256 != closure.runtime_report_sha256
        || review.authoritative_movie_sha256 != closure.authoritative_movie_sha256
        || review.delivery_movie_sha256 != closure.delivery_movie_sha256
        || review.human_review_scope != closure.human_review_scope
    {
        return Err(CharacterPackControlReviewError::ClosureMismatch);
    }
    Ok(())
}

pub fn validate_character_pack_control_runtime_contract(
    forgepack: &Value,
    manifest: &Value,
    godot_helper: &Value,
    provenance: &Value,
    runtime_config: &Value,
    runtime_report: &Value,
) -> Result<CharacterPackControlRuntimeContractV1, CharacterPackControlReviewError> {
    let rendering = manifest
        .pointer("/rendering")
        .ok_or(CharacterPackControlReviewError::InvalidReview)?;
    if godot_helper.pointer("/spriteFrames/rendering") != Some(rendering)
        || provenance.pointer("/rendering") != Some(rendering)
    {
        return Err(CharacterPackControlReviewError::ClosureMismatch);
    }

    let idle_right = validated_animation_timing(
        "idle_right",
        "/idleRight",
        forgepack,
        manifest,
        godot_helper,
        provenance,
        runtime_config,
        runtime_report,
    )?;
    let walk_right = validated_animation_timing(
        "walk_right",
        "/walkRight",
        forgepack,
        manifest,
        godot_helper,
        provenance,
        runtime_config,
        runtime_report,
    )?;

    let anchor_x = matching_number(
        &[
            (provenance, "/sharedAnchor/x"),
            (manifest, "/anchor/x"),
            (godot_helper, "/spriteFrames/anchor/x"),
            (forgepack, "/source/metadata/sharedAnchor/x"),
            (runtime_config, "/movement/pivot/x"),
            (runtime_report, "/movement/pivot/x"),
        ],
        false,
    )?;
    let anchor_y = matching_number(
        &[
            (provenance, "/sharedAnchor/y"),
            (manifest, "/anchor/y"),
            (godot_helper, "/spriteFrames/anchor/y"),
            (forgepack, "/source/metadata/sharedAnchor/y"),
            (runtime_config, "/movement/pivot/y"),
            (runtime_report, "/movement/pivot/y"),
        ],
        false,
    )?;
    for (document, pointer) in [
        (provenance, "/sharedAnchor/type"),
        (manifest, "/anchor/type"),
        (godot_helper, "/spriteFrames/anchor/type"),
        (forgepack, "/source/metadata/sharedAnchor/type"),
    ] {
        if document.pointer(pointer).and_then(Value::as_str) != Some("custom") {
            return Err(CharacterPackControlReviewError::ClosureMismatch);
        }
    }

    let collision_width = matching_number(
        &[
            (provenance, "/collision/width"),
            (forgepack, "/source/metadata/collision/width"),
            (runtime_config, "/movement/collision/width"),
            (runtime_report, "/movement/collision/width"),
        ],
        true,
    )?;
    let collision_height = matching_number(
        &[
            (provenance, "/collision/height"),
            (forgepack, "/source/metadata/collision/height"),
            (runtime_config, "/movement/collision/height"),
            (runtime_report, "/movement/collision/height"),
        ],
        true,
    )?;
    let collision_offset_x = matching_number(
        &[
            (provenance, "/collision/offsetX"),
            (forgepack, "/source/metadata/collision/offsetX"),
            (runtime_config, "/movement/collision/offsetX"),
            (runtime_report, "/movement/collision/offsetX"),
        ],
        false,
    )?;
    let collision_offset_y = matching_number(
        &[
            (provenance, "/collision/offsetY"),
            (forgepack, "/source/metadata/collision/offsetY"),
            (runtime_config, "/movement/collision/offsetY"),
            (runtime_report, "/movement/collision/offsetY"),
        ],
        false,
    )?;
    let visual_scale = matching_number(
        &[
            (provenance, "/visualScale"),
            (forgepack, "/source/metadata/visualScale"),
            (runtime_config, "/movement/visualScale"),
            (runtime_report, "/movement/visualScale"),
        ],
        true,
    )?;
    let move_speed_pixels_per_second = matching_number(
        &[
            (provenance, "/moveSpeedPixelsPerSecond"),
            (forgepack, "/source/metadata/moveSpeedPixelsPerSecond"),
            (runtime_config, "/movement/speedPixelsPerSecond"),
            (runtime_report, "/movement/speedPixelsPerSecond"),
        ],
        true,
    )?;

    Ok(CharacterPackControlRuntimeContractV1 {
        idle_right,
        walk_right,
        rendering: rendering.clone(),
        anchor_x,
        anchor_y,
        collision_width,
        collision_height,
        collision_offset_x,
        collision_offset_y,
        visual_scale,
        move_speed_pixels_per_second,
    })
}

#[allow(clippy::too_many_arguments)]
fn validated_animation_timing(
    animation: &str,
    provenance_pointer: &str,
    forgepack: &Value,
    manifest: &Value,
    godot_helper: &Value,
    provenance: &Value,
    runtime_config: &Value,
    runtime_report: &Value,
) -> Result<CharacterPackControlAnimationTimingV1, CharacterPackControlReviewError> {
    let manifest_animation = named_animation(manifest, "/animations", animation)?;
    let helper_animation = named_animation(godot_helper, "/spriteFrames/animations", animation)?;
    let forgepack_animation = named_animation(forgepack, "/animations", animation)?;
    for item in [manifest_animation, helper_animation, forgepack_animation] {
        if item
            .pointer("/frames")
            .and_then(Value::as_array)
            .map(Vec::len)
            != Some(24)
        {
            return Err(CharacterPackControlReviewError::ClosureMismatch);
        }
    }
    let config_pointer = format!("/animations/{animation}");
    let config_animation = runtime_config
        .pointer(&config_pointer)
        .ok_or(CharacterPackControlReviewError::InvalidReview)?;
    let report_animation = runtime_report
        .pointer(&config_pointer)
        .ok_or(CharacterPackControlReviewError::InvalidReview)?;
    for item in [config_animation, report_animation] {
        if integer_at(item, "/frameCount")? != 24 {
            return Err(CharacterPackControlReviewError::ClosureMismatch);
        }
    }
    let provenance_animation = provenance
        .pointer(provenance_pointer)
        .ok_or(CharacterPackControlReviewError::InvalidReview)?;
    if provenance_animation
        .pointer("/frameSha256")
        .and_then(Value::as_array)
        .map(Vec::len)
        != Some(24)
    {
        return Err(CharacterPackControlReviewError::ClosureMismatch);
    }

    let durations = duration_array(manifest_animation, "/frameDurationsMs")?;
    if durations.len() != 24
        || duration_array(helper_animation, "/frameDurationsMs")? != durations
        || duration_array(forgepack_animation, "/frameDurationsMs")? != durations
        || duration_array(config_animation, "/frameDurationsMs")? != durations
        || duration_array(report_animation, "/frameDurationsMs")? != durations
        || duration_array(provenance_animation, "/frameDurationsMs")? != durations
    {
        return Err(CharacterPackControlReviewError::ClosureMismatch);
    }
    let cycle_duration_ms = durations.iter().try_fold(0_u64, |sum, duration| {
        sum.checked_add(*duration)
            .ok_or(CharacterPackControlReviewError::InvalidReview)
    })?;
    if cycle_duration_ms == 0 {
        return Err(CharacterPackControlReviewError::InvalidReview);
    }
    Ok(CharacterPackControlAnimationTimingV1 {
        frame_durations_ms: durations,
        cycle_duration_ms,
    })
}

fn named_animation<'a>(
    document: &'a Value,
    pointer: &str,
    animation: &str,
) -> Result<&'a Value, CharacterPackControlReviewError> {
    document
        .pointer(pointer)
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .find(|item| item.pointer("/name").and_then(Value::as_str) == Some(animation))
        })
        .ok_or(CharacterPackControlReviewError::InvalidReview)
}

fn duration_array(
    document: &Value,
    pointer: &str,
) -> Result<Vec<u64>, CharacterPackControlReviewError> {
    document
        .pointer(pointer)
        .and_then(Value::as_array)
        .ok_or(CharacterPackControlReviewError::InvalidReview)?
        .iter()
        .map(integer_value)
        .collect()
}

fn integer_at(document: &Value, pointer: &str) -> Result<u64, CharacterPackControlReviewError> {
    integer_value(
        document
            .pointer(pointer)
            .ok_or(CharacterPackControlReviewError::InvalidReview)?,
    )
}

fn integer_value(value: &Value) -> Result<u64, CharacterPackControlReviewError> {
    value
        .as_u64()
        .or_else(|| {
            value
                .as_f64()
                .filter(|number| number.is_finite() && *number >= 0.0 && number.fract() == 0.0)
                .map(|number| number as u64)
        })
        .filter(|number| *number > 0)
        .ok_or(CharacterPackControlReviewError::InvalidReview)
}

fn matching_number(
    values: &[(&Value, &str)],
    positive: bool,
) -> Result<f64, CharacterPackControlReviewError> {
    let first = values
        .first()
        .and_then(|(document, pointer)| document.pointer(pointer))
        .and_then(Value::as_f64)
        .filter(|number| number.is_finite())
        .ok_or(CharacterPackControlReviewError::InvalidReview)?;
    if positive && first <= 0.0 {
        return Err(CharacterPackControlReviewError::InvalidReview);
    }
    if values.iter().skip(1).any(|(document, pointer)| {
        document
            .pointer(pointer)
            .and_then(Value::as_f64)
            .filter(|number| number.is_finite())
            != Some(first)
    }) {
        return Err(CharacterPackControlReviewError::ClosureMismatch);
    }
    Ok(first)
}

fn validate_review_shape(
    review: &CharacterPackControlHumanReviewV1,
) -> Result<(), CharacterPackControlReviewError> {
    let hashes = [
        review.godot_evidence_sha256.as_str(),
        review.candidate_pack_forgepack_sha256.as_str(),
        review.candidate_pack_manifest_sha256.as_str(),
        review.candidate_pack_inventory_sha256.as_str(),
        review.candidate_pack_content_sha256.as_str(),
        review.candidate_pack_provenance_sha256.as_str(),
        review.idle_right_approved_review_sha256.as_str(),
        review.walk_right_approved_review_sha256.as_str(),
        review.runtime_report_sha256.as_str(),
        review.authoritative_movie_sha256.as_str(),
        review.delivery_movie_sha256.as_str(),
    ];
    if review.schema_version != "1"
        || review.profile != CHARACTER_PACK_CONTROL_REVIEW_PROFILE
        || review.reviewer != "human"
        || review.status != CharacterPackControlReviewStatusV1::Approved
        || !review.checks.all_passed()
        || review.review_note.trim().is_empty()
        || hashes.into_iter().any(|value| !is_sha256(value))
        || !scope_is_valid(&review.human_review_scope)
    {
        return Err(CharacterPackControlReviewError::InvalidReview);
    }
    Ok(())
}

fn validate_closure_shape(
    closure: &CharacterPackControlReviewClosureV1,
) -> Result<(), CharacterPackControlReviewError> {
    let hashes = [
        closure.godot_evidence_sha256.as_str(),
        closure.candidate_pack_forgepack_sha256.as_str(),
        closure.candidate_pack_manifest_sha256.as_str(),
        closure.candidate_pack_inventory_sha256.as_str(),
        closure.candidate_pack_content_sha256.as_str(),
        closure.candidate_pack_provenance_sha256.as_str(),
        closure.idle_right_approved_review_sha256.as_str(),
        closure.walk_right_approved_review_sha256.as_str(),
        closure.runtime_report_sha256.as_str(),
        closure.authoritative_movie_sha256.as_str(),
        closure.delivery_movie_sha256.as_str(),
    ];
    if hashes.into_iter().any(|value| !is_sha256(value))
        || !scope_is_valid(&closure.human_review_scope)
    {
        return Err(CharacterPackControlReviewError::InvalidReview);
    }
    Ok(())
}

fn scope_is_valid(scope: &[String]) -> bool {
    scope == CHARACTER_PACK_CONTROL_REVIEW_SCOPE_V1.map(str::to_owned)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn closure() -> CharacterPackControlReviewClosureV1 {
        CharacterPackControlReviewClosureV1 {
            godot_evidence_sha256: "a".repeat(64),
            candidate_pack_forgepack_sha256: "b".repeat(64),
            candidate_pack_manifest_sha256: "c".repeat(64),
            candidate_pack_inventory_sha256: "d".repeat(64),
            candidate_pack_content_sha256: "e".repeat(64),
            candidate_pack_provenance_sha256: "f".repeat(64),
            idle_right_approved_review_sha256: "1".repeat(64),
            walk_right_approved_review_sha256: "2".repeat(64),
            runtime_report_sha256: "3".repeat(64),
            authoritative_movie_sha256: "4".repeat(64),
            delivery_movie_sha256: "5".repeat(64),
            human_review_scope: CHARACTER_PACK_CONTROL_REVIEW_SCOPE_V1
                .map(str::to_owned)
                .to_vec(),
        }
    }

    #[test]
    fn approval_sets_all_six_checks_and_preserves_closure() {
        let closure = closure();
        let review = approve_character_pack_control_review(
            &closure,
            "human approved all six composite checks",
            Utc::now(),
        )
        .unwrap();
        assert!(character_pack_control_review_is_approved(&review));
        validate_character_pack_control_review_closure(&review, &closure).unwrap();
    }

    #[test]
    fn changed_evidence_is_rejected() {
        let closure = closure();
        let review = approve_character_pack_control_review(
            &closure,
            "human approved all six composite checks",
            Utc::now(),
        )
        .unwrap();
        let mut changed = closure;
        changed.godot_evidence_sha256 = "9".repeat(64);
        assert!(matches!(
            validate_character_pack_control_review_closure(&review, &changed),
            Err(CharacterPackControlReviewError::ClosureMismatch)
        ));
    }

    #[test]
    fn runtime_contract_closes_timing_rendering_and_movement() {
        let (forgepack, manifest, helper, provenance, config, report) = runtime_documents();
        let contract = validate_character_pack_control_runtime_contract(
            &forgepack,
            &manifest,
            &helper,
            &provenance,
            &config,
            &report,
        )
        .unwrap();

        assert_eq!(contract.idle_right.frame_durations_ms.len(), 24);
        assert_eq!(contract.walk_right.cycle_duration_ms, 1_992);
        assert_eq!(contract.move_speed_pixels_per_second, 120.0);
    }

    #[test]
    fn runtime_contract_rejects_report_drift() {
        let (forgepack, manifest, helper, provenance, config, mut report) = runtime_documents();
        report["animations"]["walk_right"]["frameDurationsMs"][4] = json!(84);
        assert!(matches!(
            validate_character_pack_control_runtime_contract(
                &forgepack,
                &manifest,
                &helper,
                &provenance,
                &config,
                &report,
            ),
            Err(CharacterPackControlReviewError::ClosureMismatch)
        ));
    }

    fn runtime_documents() -> (Value, Value, Value, Value, Value, Value) {
        let frames = (0..24).collect::<Vec<_>>();
        let durations = vec![83_u64; 24];
        let animation = |name: &str| {
            json!({
                "name": name,
                "frames": frames,
                "frameDurationsMs": durations,
            })
        };
        let rendering = json!({
            "profile": "godot-sprite-rendering@1.0.0",
            "textureFilter": "linear",
            "pixelSnap": false,
            "mirrorPolicy": "auto",
        });
        let anchor = json!({"type": "custom", "x": 720.0, "y": 1316.0});
        let collision = json!({
            "width": 54.0,
            "height": 88.0,
            "offsetX": 0.0,
            "offsetY": -44.0,
        });
        let movement = json!({
            "pivot": {"x": 720.0, "y": 1316.0},
            "collision": collision,
            "visualScale": 0.28,
            "speedPixelsPerSecond": 120.0,
        });
        let runtime_animations = json!({
            "idle_right": {"frameCount": 24, "frameDurationsMs": durations},
            "walk_right": {"frameCount": 24, "frameDurationsMs": durations},
        });
        let forgepack = json!({
            "animations": [animation("idle_right"), animation("walk_right")],
            "source": {"metadata": {
                "sharedAnchor": anchor,
                "collision": collision,
                "visualScale": 0.28,
                "moveSpeedPixelsPerSecond": 120.0,
            }},
        });
        let manifest = json!({
            "animations": [animation("idle_right"), animation("walk_right")],
            "anchor": anchor,
            "rendering": rendering,
        });
        let helper = json!({"spriteFrames": {
            "animations": [animation("idle_right"), animation("walk_right")],
            "anchor": anchor,
            "rendering": rendering,
        }});
        let provenance = json!({
            "rendering": rendering,
            "sharedAnchor": anchor,
            "collision": collision,
            "visualScale": 0.28,
            "moveSpeedPixelsPerSecond": 120.0,
            "idleRight": {
                "frameSha256": vec!["a".repeat(64); 24],
                "frameDurationsMs": durations,
            },
            "walkRight": {
                "frameSha256": vec!["b".repeat(64); 24],
                "frameDurationsMs": durations,
            },
        });
        let config = json!({"movement": movement, "animations": runtime_animations});
        let report = config.clone();
        (forgepack, manifest, helper, provenance, config, report)
    }
}
