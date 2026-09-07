use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use forge_core::provider::{
    CredentialKind, EditImageRequest, EditVideoRequest, GenerateImageRequest, GenerateVideoRequest,
    MediaGenerationProvider, ProviderCapability, ProviderConstraints, ProviderError,
    ProviderHealth, ProviderMedia, ProviderPoll, ProviderTicket, ProviderUsage, ReferenceRole,
    VideoGenerationMode,
};
use gif::{Encoder, Frame, Repeat};
use image::{ImageBuffer, Rgba, RgbaImage};

const VIDEO_CYCLE_V6_MARKER: &str = "Forge topdown video cycle v6";
const TOPDOWN_CYCLE_V10_MARKER: &str = "Forge approved direction continuous cycle v10";
const DIRECTION_POSE_V7_MARKER: &str = "Forge topdown direction pose v7";
const DIRECTION_MOTION_V8_MARKER: &str = "Forge canonical direction motion v8";

fn asymmetric_pose_structure_grounded_left(path: &Path) -> Option<bool> {
    let image = image::open(path).ok()?.to_rgba8();
    let center = image.width() / 2;
    let lower_start = image.height() * 2 / 3;
    let (mut left_bright, mut right_bright) = (0_usize, 0_usize);
    for (x, y, pixel) in image.enumerate_pixels() {
        if y < lower_start || pixel[3] <= 8 || pixel[0] < 200 {
            continue;
        }
        if x < center {
            left_bright += 1;
        } else {
            right_bright += 1;
        }
    }
    match left_bright.cmp(&right_bright) {
        std::cmp::Ordering::Greater => Some(true),
        std::cmp::Ordering::Less => Some(false),
        std::cmp::Ordering::Equal => None,
    }
}

struct FixtureTicket {
    polls: u8,
    edited: bool,
    back_facing: bool,
    right_facing: bool,
    left_facing: bool,
    detached_effect: bool,
    idle: bool,
    locked_video: bool,
    cycle_video: bool,
    direction_pose_video: bool,
    topdown_cycle_video: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureImageEditObservation {
    pub authorization_target: Option<String>,
    pub reference_roles: Vec<ReferenceRole>,
    pub reference_sha256: Vec<String>,
}

/// Offline request ledger for provider-contract assertions. It intentionally
/// records only non-secret request metadata and local input dimensions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureRequestObservation {
    pub kind: FixtureRequestKind,
    pub prompt: String,
    pub resolution: Option<String>,
    pub authorization_target: Option<String>,
    pub reference_roles: Vec<ReferenceRole>,
    pub video_input_path: Option<PathBuf>,
    pub video_input_sha256: Option<String>,
    pub video_input_dimensions: Option<(u32, u32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureRequestKind {
    GenerateImage,
    EditImage,
    GenerateVideo,
    EditVideo,
}

pub struct FixtureProvider {
    tickets: Mutex<HashMap<String, FixtureTicket>>,
    next_ticket: Mutex<u64>,
    usage: Mutex<ProviderUsage>,
    bad_loop_before_edit: bool,
    supports_video_edit: bool,
    review_keyframes: bool,
    detached_walk_up_effect: bool,
    wrong_right_direction: bool,
    static_keyposes: bool,
    same_side_keyposes: bool,
    wrong_right_contact_keypose: bool,
    ignore_asymmetric_pose_structure: bool,
    platform_sole_leak: bool,
    invalid_laterality_fresh_retry: bool,
    fail_edit_once_marker: Option<String>,
    failed_edit_markers: Mutex<HashMap<String, bool>>,
    edit_observations: Mutex<Vec<FixtureImageEditObservation>>,
    request_observations: Mutex<Vec<FixtureRequestObservation>>,
    authenticated: bool,
}

impl Default for FixtureProvider {
    fn default() -> Self {
        Self {
            tickets: Mutex::new(HashMap::new()),
            next_ticket: Mutex::new(1),
            usage: Mutex::new(ProviderUsage::default()),
            bad_loop_before_edit: false,
            supports_video_edit: true,
            review_keyframes: false,
            detached_walk_up_effect: false,
            wrong_right_direction: false,
            static_keyposes: false,
            same_side_keyposes: false,
            wrong_right_contact_keypose: false,
            ignore_asymmetric_pose_structure: false,
            platform_sole_leak: false,
            invalid_laterality_fresh_retry: false,
            fail_edit_once_marker: None,
            failed_edit_markers: Mutex::new(HashMap::new()),
            edit_observations: Mutex::new(Vec::new()),
            request_observations: Mutex::new(Vec::new()),
            authenticated: true,
        }
    }
}

impl FixtureProvider {
    pub fn with_bad_loop_before_edit(mut self) -> Self {
        self.bad_loop_before_edit = true;
        self
    }

    pub fn without_video_edit(mut self) -> Self {
        self.supports_video_edit = false;
        self
    }

    pub fn with_review_keyframes(mut self) -> Self {
        self.review_keyframes = true;
        self
    }

    pub fn with_detached_walk_up_effect(mut self) -> Self {
        self.detached_walk_up_effect = true;
        self
    }

    pub fn with_wrong_right_direction(mut self) -> Self {
        self.wrong_right_direction = true;
        self
    }

    pub fn with_static_keyposes(mut self) -> Self {
        self.static_keyposes = true;
        self
    }

    pub fn with_same_side_keyposes(mut self) -> Self {
        self.same_side_keyposes = true;
        self
    }

    pub fn with_wrong_right_contact_keypose(mut self) -> Self {
        self.wrong_right_contact_keypose = true;
        self
    }

    pub fn with_platform_sole_leak(mut self) -> Self {
        self.platform_sole_leak = true;
        self
    }

    pub fn ignoring_asymmetric_pose_structure(mut self) -> Self {
        self.ignore_asymmetric_pose_structure = true;
        self
    }

    pub fn with_invalid_laterality_fresh_retry(mut self) -> Self {
        self.invalid_laterality_fresh_retry = true;
        self
    }

    pub fn with_edit_failure_once(mut self, marker: impl Into<String>) -> Self {
        self.fail_edit_once_marker = Some(marker.into());
        self
    }

    pub fn without_authentication(mut self) -> Self {
        self.authenticated = false;
        self
    }

    pub fn edit_observations(&self) -> Vec<FixtureImageEditObservation> {
        self.edit_observations.lock().unwrap().clone()
    }

    pub fn request_observations(&self) -> Vec<FixtureRequestObservation> {
        self.request_observations.lock().unwrap().clone()
    }

    fn observe(&self, observation: FixtureRequestObservation) {
        self.request_observations.lock().unwrap().push(observation);
    }

    fn write_image(
        &self,
        output_path: &Path,
        prompt: &str,
        pose_structure: Option<&Path>,
    ) -> Result<ProviderMedia, ProviderError> {
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut image: RgbaImage = ImageBuffer::from_pixel(96, 96, Rgba([0, 255, 0, 255]));
        let keyframe_phase = prompt
            .split("Forge frame phase ")
            .nth(1)
            .and_then(|value| value.split('/').next())
            .and_then(|value| value.parse::<u8>().ok())
            .or_else(|| {
                prompt
                    .split("Forge key pose ")
                    .nth(1)
                    .and_then(|value| value.split('/').next())
                    .and_then(|value| value.parse::<u8>().ok())
            });
        let first_collection_outlier = prompt.contains("[fixture:outlier_then_success]")
            && output_path
                .components()
                .any(|component| component.as_os_str() == "attempt-1");
        let first_blank_front_direction = prompt.contains("[fixture:blank_front_direction]")
            && prompt.contains("Pose:")
            && output_path
                .components()
                .any(|component| component.as_os_str() == "attempt-1");
        let back_facing_direction = (prompt.contains("Face directly up and away from the viewer")
            || prompt.contains("rear/up DirectionAnchor")
            || prompt.contains("rear/up view with no visible face")
            || (prompt.contains(DIRECTION_MOTION_V8_MARKER) && prompt.contains("rear/up"))
            || (prompt.contains("Forge direction pose chain v7") && prompt.contains("rear/up")))
            && !prompt.contains("[fixture:wrong_up_direction]");
        let right_facing_direction = (prompt.contains("Strict right-facing profile")
            || prompt.contains("strict right-profile DirectionAnchor")
            || prompt.contains("strict right profile moving")
            || (prompt.contains(DIRECTION_MOTION_V8_MARKER) && prompt.contains("right-facing"))
            || (prompt.contains("Forge direction pose chain v7")
                && prompt.contains("right-facing")))
            && !prompt.contains("[fixture:wrong_right_direction]")
            && !self.wrong_right_direction;
        let left_facing_direction = (prompt.contains("Strict left-facing profile")
            || (prompt.contains(DIRECTION_MOTION_V8_MARKER) && prompt.contains("left-facing")))
            && !prompt.contains("[fixture:wrong_left_direction]");
        let color = if first_collection_outlier {
            Rgba([8, 12, 16, 255])
        } else if prompt.contains("[fixture:palette_drift]") {
            Rgba([220, 90, 60, 255])
        } else if prompt.contains("[fixture:review]")
            || (self.review_keyframes && keyframe_phase.is_some())
        {
            Rgba([180, 80, 120, 255])
        } else {
            Rgba([130, 60, 210, 255])
        };
        let articulated = (keyframe_phase.is_some()
            || prompt.contains("canonical identity image")
            || prompt.contains("full-body")
            || prompt.contains("dialogue-bust")
            || prompt.contains("Edit only the existing character")
            || prompt.contains("Full body")
            || prompt.contains(DIRECTION_MOTION_V8_MARKER))
            && !prompt.contains("[fixture:bust]");
        let invalid_laterality_fresh_retry =
            self.invalid_laterality_fresh_retry && prompt.contains("laterality-only fresh retry");
        if prompt.contains("[fixture:hard_multiple_subjects]") || invalid_laterality_fresh_retry {
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
        } else if first_collection_outlier {
            for y in 18..86 {
                let half_width = ((y - 18) / 3).min(26);
                for x in (48 - half_width)..=(48 + half_width) {
                    if (x + y) % 3 != 0 {
                        image.put_pixel(x, y, color);
                    }
                }
            }
        } else if articulated {
            let mut phase = keyframe_phase.unwrap_or(0) % 8;
            let locked_frames_workflow = prompt.contains("Forge locked animation frames 2x2")
                || prompt.contains("Forge locked frame repair");
            // A locked sheet cell and its single-frame repair share the same
            // four authored poses, even though the repair uses the frame-phase
            // marker also used by older eight-frame workflows.
            let keypose_workflow = locked_frames_workflow
                || prompt.contains("Forge key pose ")
                || prompt.contains("Forge independent grid keyframe V9.1")
                || prompt.contains("Forge structured gait keyframe V9.2")
                || prompt.contains("Forge asymmetric gait keyframe V9.3")
                || prompt.contains("Forge platform-safe gait keyframe V9.4")
                || prompt.contains("Forge footwear cleanup keyframe V9.5");
            let idle_action = prompt.contains("Action idle;");
            if self.static_keyposes && keypose_workflow {
                phase = 0;
            }
            for y in 28..68 {
                for x in 34..62 {
                    image.put_pixel(x, y, color);
                }
            }
            if keypose_workflow && idle_action && !locked_frames_workflow {
                let horizontal_range = match phase {
                    1 => Some(32..34),
                    3 => Some(62..64),
                    _ => None,
                };
                if let Some(horizontal_range) = horizontal_range {
                    for y in 35..65 {
                        for x in horizontal_range.clone() {
                            image.put_pixel(x, y, color);
                        }
                    }
                }
            }
            if locked_frames_workflow && idle_action {
                let shoulder = match phase {
                    1 => Some(32..34),
                    _ => None,
                };
                if let Some(shoulder) = shoulder {
                    for y in 42..50 {
                        for x in shoulder.clone() {
                            image.put_pixel(x, y, color);
                        }
                    }
                }
            }
            let swing = if idle_action {
                0
            } else {
                (if phase <= 4 { phase } else { 8 - phase } as u32).min(3)
            };
            let asymmetric_guide_side = pose_structure
                .filter(|_| {
                    prompt.contains("Forge asymmetric gait keyframe V9.3")
                        || prompt.contains("Forge platform-safe gait keyframe V9.4")
                })
                .filter(|_| !self.ignore_asymmetric_pose_structure)
                .and_then(asymmetric_pose_structure_grounded_left);
            let legs = if let Some(grounded_left) = asymmetric_guide_side {
                if grounded_left {
                    [(30, 38, 68, 84), (54, 62, 68, 79)]
                } else {
                    [(32, 40, 68, 79), (52, 60, 68, 84)]
                }
            } else if self.same_side_keyposes && keypose_workflow && !idle_action {
                match phase % 4 {
                    0 => [(30, 38, 68, 84), (54, 62, 68, 82)],
                    1 => [(31, 39, 68, 84), (53, 61, 68, 81)],
                    2 => [(32, 40, 68, 84), (52, 60, 68, 80)],
                    _ => [(33, 41, 68, 84), (51, 59, 68, 79)],
                }
            } else if self.wrong_right_contact_keypose
                && keypose_workflow
                && !idle_action
                && phase % 4 == 2
            {
                // Preserve a raster-distinct phase and most of the normal
                // motion ordering while keeping a slight screen-left contact
                // advantage. This models the real V9.2 failure where generic
                // motion semantics passed but explicit laterality did not.
                [(32, 40, 68, 84), (52, 60, 68, 83)]
            } else if keypose_workflow && !idle_action {
                match phase % 4 {
                    0 => [(30, 38, 68, 84), (54, 62, 68, 82)],
                    1 => [(31, 39, 68, 84), (53, 61, 68, 81)],
                    2 => [(32, 40, 68, 82), (52, 60, 68, 84)],
                    _ => [(31, 39, 68, 81), (53, 61, 68, 84)],
                }
            } else {
                let left = 30 + swing;
                let right = 54_u32.saturating_sub(swing);
                [(left, left + 8, 68, 84), (right, right + 8, 68, 84)]
            };
            for (left, right, top, bottom) in legs {
                for y in top..bottom {
                    for x in left..right {
                        image.put_pixel(x, y, color);
                    }
                }
            }
            if self.platform_sole_leak
                && prompt.contains("Forge asymmetric gait keyframe V9.3")
                && phase % 4 == 2
            {
                // Models the real V9.3 semantic guide leak: a wide, thin
                // platform under the screen-right planted boot. It is not a
                // literal grayscale guide copy, so the silhouette gate owns it.
                for y in 82..88 {
                    for x in 49..73 {
                        image.put_pixel(x, y, Rgba([126, 126, 126, 255]));
                    }
                }
            }
            let face_left = if right_facing_direction {
                48
            } else if left_facing_direction {
                34
            } else {
                39
            };
            let face_right = if right_facing_direction {
                62
            } else if left_facing_direction {
                48
            } else {
                57
            };
            for y in 34..51 {
                for x in face_left..face_right {
                    image.put_pixel(
                        x,
                        y,
                        if back_facing_direction {
                            Rgba([40, 110, 80, 255])
                        } else {
                            Rgba([190, 126, 88, 255])
                        },
                    );
                }
            }
            for x in face_left..face_right {
                image.put_pixel(x, 34, Rgba([55, 35, 25, 255]));
                image.put_pixel(x, 50, Rgba([55, 35, 25, 255]));
            }
            for y in 34..51 {
                image.put_pixel(face_left, y, Rgba([55, 35, 25, 255]));
                image.put_pixel(face_right - 1, y, Rgba([55, 35, 25, 255]));
            }
            if !first_blank_front_direction && !back_facing_direction {
                let eyes = if right_facing_direction {
                    vec![57_u32]
                } else if left_facing_direction {
                    vec![38_u32]
                } else {
                    vec![43_u32, 52_u32]
                };
                for x in eyes {
                    image.put_pixel(x, 39, Rgba([35, 22, 18, 255]));
                }
            }
            if back_facing_direction {
                // A rear-facing hood deliberately has no visible face signal.
            } else if prompt.contains("Edit only the existing character") {
                let expression_color = Rgba([45, 24, 20, 255]);
                if prompt.contains("happy") {
                    for x in 44..52 {
                        image.put_pixel(x, 48 + u32::from(x < 48), expression_color);
                    }
                } else if prompt.contains("angry") {
                    for x in 44..52 {
                        image.put_pixel(x, 48, expression_color);
                    }
                } else if prompt.contains("hurt") {
                    for x in 45..51 {
                        image.put_pixel(x, 49 - u32::from(x < 48), expression_color);
                    }
                } else if prompt.contains("surprised") {
                    for x in 46..50 {
                        image.put_pixel(x, 48, expression_color);
                    }
                }
            } else if !first_blank_front_direction {
                let mouth_left = if right_facing_direction {
                    56
                } else if left_facing_direction {
                    36
                } else {
                    45
                };
                let mouth_right = if right_facing_direction {
                    60
                } else if left_facing_direction {
                    40
                } else {
                    51
                };
                for x in mouth_left..mouth_right {
                    image.put_pixel(x, 48, Rgba([70, 36, 28, 255]));
                }
            }
            if prompt.contains("[fixture:grid_hand_baseline]") && !back_facing_direction {
                let hand = Rgba([190, 126, 88, 255]);
                for y in 54..62 {
                    for x in 29..34 {
                        image.put_pixel(x, y, hand);
                    }
                    for x in 62..67 {
                        image.put_pixel(x, y, hand);
                    }
                }
                if prompt.contains("[fixture:grid_one_glove]") && keyframe_phase == Some(2) {
                    for y in 54..62 {
                        for x in 29..34 {
                            image.put_pixel(x, y, Rgba([32, 28, 40, 255]));
                        }
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
        if prompt.contains("fixed cardinal orthographic RPG sprite camera")
            && prompt.contains("Pose:")
        {
            // Stable video V2 explicitly requires the direction still to keep
            // the normalized SubjectLock framing. Model that contract in the
            // offline Provider instead of returning the legacy loose 96px
            // framing used by older fixtures.
            let enlarged =
                image::imageops::resize(&image, 130, 130, image::imageops::FilterType::Nearest);
            image = image::imageops::crop_imm(&enlarged, 17, 24, 96, 96).to_image();
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

    fn write_animation_sheet(
        &self,
        output_path: &Path,
        prompt: &str,
    ) -> Result<ProviderMedia, ProviderError> {
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut sheet = RgbaImage::from_pixel(192, 192, Rgba([0, 255, 0, 255]));
        for frame_index in 0..4_u8 {
            let collapsed_once = prompt.contains("[fixture:sheet_duplicate_once]")
                && prompt.contains("Action walk_down;")
                && !prompt.contains("Forge Action Grid diagnostic retry");
            let rendered_phase = if prompt.contains("[fixture:sheet_duplicate]") || collapsed_once {
                0
            } else {
                frame_index
            };
            let cell_path =
                output_path.with_file_name(format!(".forge-fixture-sheet-cell-{frame_index}.png"));
            let cell_prompt =
                format!("{prompt} Forge key pose {rendered_phase}/4; ordered animation phase;");
            self.write_image(&cell_path, &cell_prompt, None)?;
            let mut cell = image::open(&cell_path)
                .map_err(|error| ProviderError::InvalidOutput(error.to_string()))?
                .to_rgba8();
            if prompt.contains("[fixture:frame_retry_unexpected_equipment]")
                && prompt.contains("Generate only the corrected cell")
            {
                for y in 28..86 {
                    for x in 72..74 {
                        cell.put_pixel(x, y, Rgba([92, 58, 32, 255]));
                    }
                }
                for y in 56..60 {
                    for x in 62..74 {
                        cell.put_pixel(x, y, Rgba([92, 58, 32, 255]));
                    }
                }
            }
            let x = u32::from(frame_index % 2) * 96;
            let y = u32::from(frame_index / 2) * 96;
            image::imageops::replace(&mut sheet, &cell, x.into(), y.into());
            let _ = fs::remove_file(&cell_path);
        }
        if prompt.contains("[fixture:sheet_bleed]") {
            sheet.put_pixel(0, 40, Rgba([130, 60, 210, 255]));
        }
        sheet
            .save(output_path)
            .map_err(|error| ProviderError::InvalidOutput(error.to_string()))?;
        Ok(ProviderMedia {
            path: output_path.to_path_buf(),
            mime_type: "image/png".into(),
            provider_asset_id: Some("fixture-animation-sheet".into()),
            revised_prompt: None,
        })
    }

    fn write_direction_sheet(
        &self,
        output_path: &Path,
        prompt: &str,
    ) -> Result<ProviderMedia, ProviderError> {
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        // V6 deliberately supplies a high-resolution master to image-to-video.
        // The marker is the only workflow discriminator: V1–V5 keep their
        // historical 192px fixture output exactly.
        let cell_size = if prompt.contains(VIDEO_CYCLE_V6_MARKER) {
            512
        } else {
            96
        };
        let directions = [
            "Face directly down and toward the viewer.",
            "Face directly up and away from the viewer.",
            "Strict right-facing profile.",
            "Strict left-facing profile.",
        ];
        let mut sheet = RgbaImage::from_pixel(cell_size * 2, cell_size * 2, Rgba([0, 255, 0, 255]));
        for (frame_index, direction) in directions.iter().enumerate() {
            let cell_path = output_path
                .with_file_name(format!(".forge-fixture-direction-cell-{frame_index}.png"));
            let cell_prompt =
                format!("{prompt} {direction} Full body. Forge key pose {frame_index}/4;");
            self.write_image(&cell_path, &cell_prompt, None)?;
            let cell = image::open(&cell_path)
                .map_err(|error| ProviderError::InvalidOutput(error.to_string()))?
                .to_rgba8();
            let cell = if cell_size == 96 {
                cell
            } else {
                image::imageops::resize(
                    &cell,
                    cell_size,
                    cell_size,
                    image::imageops::FilterType::Nearest,
                )
            };
            let mut cell = cell;
            if prompt.contains("[fixture:grid_unexpected_equipment]")
                && !prompt.contains("new targeted retry of a rejected grid")
            {
                let scale = cell_size / 96;
                let shaft_x = 72 * scale;
                for y in (28 * scale)..(86 * scale) {
                    for x in shaft_x..(shaft_x + 2 * scale).min(cell_size) {
                        cell.put_pixel(x, y, Rgba([92, 58, 32, 255]));
                    }
                }
                if !prompt.contains("[fixture:grid_floating_equipment]") {
                    for y in (56 * scale)..(60 * scale) {
                        for x in (62 * scale)..(74 * scale).min(cell_size) {
                            cell.put_pixel(x, y, Rgba([92, 58, 32, 255]));
                        }
                    }
                }
            }
            let x = (frame_index as u32 % 2) * cell_size;
            let y = (frame_index as u32 / 2) * cell_size;
            image::imageops::replace(&mut sheet, &cell, x.into(), y.into());
            let _ = fs::remove_file(&cell_path);
        }
        if prompt.contains("[fixture:direction_sheet_bleed]") {
            sheet.put_pixel(0, 40, Rgba([130, 60, 210, 255]));
        }
        sheet
            .save(output_path)
            .map_err(|error| ProviderError::InvalidOutput(error.to_string()))?;
        Ok(ProviderMedia {
            path: output_path.to_path_buf(),
            mime_type: "image/png".into(),
            provider_asset_id: Some("fixture-direction-sheet".into()),
            revised_prompt: None,
        })
    }

    fn write_video(
        &self,
        output_path: &Path,
        ticket: FixtureTicket,
    ) -> Result<ProviderMedia, ProviderError> {
        let FixtureTicket {
            edited,
            back_facing,
            right_facing,
            left_facing,
            detached_effect,
            idle,
            locked_video,
            cycle_video,
            direction_pose_video,
            topdown_cycle_video,
            ..
        } = ticket;
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
        let character_color = if locked_video || cycle_video {
            [130, 60, 210, 255]
        } else {
            [210, 70, 90, 255]
        };
        // V6 supplies three 12-sample source cycles. At the extracted 12 FPS
        // cadence this is 1,000 ms per ordinary walk and contains exact
        // quarter-cycle passing poses between the two opposed contacts.
        let frame_count = if cycle_video { 36 } else { 24 };
        for index in 0..frame_count {
            let mut pixels = vec![0u8; 96 * 96 * 4];
            for pixel in pixels.chunks_exact_mut(4) {
                pixel.copy_from_slice(&[0, 255, 0, 255]);
            }
            let phase = if cycle_video { index % 12 } else { index % 8 };
            let idle_expand = if cycle_video && idle {
                [0_usize, 1, 2, 2, 2, 1, 0, 1, 2, 2, 2, 1][phase as usize]
            } else {
                0
            };
            let large_cycle_character = direction_pose_video || topdown_cycle_video;
            let torso_top = if large_cycle_character { 8 } else { 28 };
            let torso_left = if topdown_cycle_video {
                28
            } else {
                34 - idle_expand
            };
            let torso_right = if topdown_cycle_video {
                68
            } else {
                62 + idle_expand
            };
            for y in torso_top..68 {
                for x in torso_left..torso_right {
                    let start = (y * 96 + x) * 4;
                    pixels[start..start + 4].copy_from_slice(&character_color);
                }
            }
            if (locked_video || cycle_video) && idle {
                for y in 66..68 {
                    for x in 28..68 {
                        let start = (y * 96 + x) * 4;
                        pixels[start..start + 4].copy_from_slice(&character_color);
                    }
                }
                for y in 68..76 {
                    for x in 28..36 {
                        let start = (y * 96 + x) * 4;
                        pixels[start..start + 4].copy_from_slice(&character_color);
                    }
                    for x in 60..68 {
                        let start = (y * 96 + x) * 4;
                        pixels[start..start + 4].copy_from_slice(&character_color);
                    }
                }
            }
            if right_facing {
                let face_top = if large_cycle_character { 18 } else { 34 };
                let face_bottom = if large_cycle_character { 35 } else { 51 };
                for y in face_top..face_bottom {
                    for x in 48..62 {
                        let start = (y * 96 + x) * 4;
                        pixels[start..start + 4].copy_from_slice(&[190, 126, 88, 255]);
                    }
                }
                let eye_y = if large_cycle_character { 23 } else { 39 };
                let mouth_y = if large_cycle_character { 32 } else { 48 };
                let start = (eye_y * 96 + 56) * 4;
                pixels[start..start + 4].copy_from_slice(&[35, 22, 18, 255]);
                for x in 54..60 {
                    let start = (mouth_y * 96 + x) * 4;
                    pixels[start..start + 4].copy_from_slice(&[70, 36, 28, 255]);
                }
            } else if left_facing {
                let face_top = if large_cycle_character { 18 } else { 34 };
                let face_bottom = if large_cycle_character { 35 } else { 51 };
                for y in face_top..face_bottom {
                    for x in 34..48 {
                        let start = (y * 96 + x) * 4;
                        pixels[start..start + 4].copy_from_slice(&[190, 126, 88, 255]);
                    }
                }
                let eye_y = if large_cycle_character { 23 } else { 39 };
                let mouth_y = if large_cycle_character { 32 } else { 48 };
                let start = (eye_y * 96 + 39) * 4;
                pixels[start..start + 4].copy_from_slice(&[35, 22, 18, 255]);
                for x in 36..42 {
                    let start = (mouth_y * 96 + x) * 4;
                    pixels[start..start + 4].copy_from_slice(&[70, 36, 28, 255]);
                }
            } else if !back_facing {
                let face_top = if large_cycle_character { 20 } else { 36 };
                let face_bottom = if large_cycle_character { 38 } else { 54 };
                for y in face_top..face_bottom {
                    for x in 39..57 {
                        let start = (y * 96 + x) * 4;
                        pixels[start..start + 4].copy_from_slice(&[190, 126, 88, 255]);
                    }
                }
                let eye_y = if large_cycle_character { 26 } else { 42 };
                let mouth_y = if large_cycle_character { 35 } else { 51 };
                for x in [43usize, 52usize] {
                    let start = (eye_y * 96 + x) * 4;
                    pixels[start..start + 4].copy_from_slice(&[35, 22, 18, 255]);
                }
                for x in 45..51 {
                    let start = (mouth_y * 96 + x) * 4;
                    pixels[start..start + 4].copy_from_slice(&[70, 36, 28, 255]);
                }
            }
            if cycle_video {
                // V6 uses three matching 12-sample cycles rather than an
                // eight-frame/667 ms source loop. The downstream selector
                // reduces each ordinary source cycle to eight semantic phases.
                let cycle_angle = std::f32::consts::TAU * phase as f32 / 12.0;
                let horizontal_amplitude = 4.0;
                let contact_amplitude = 4.0;
                let horizontal = if idle {
                    0
                } else {
                    (cycle_angle.sin() * horizontal_amplitude).round() as i32
                };
                let contact = if idle {
                    0
                } else {
                    (cycle_angle.cos() * contact_amplitude).round() as i32
                };
                let raw_left_bottom = 84 + contact;
                let raw_right_bottom = 84 - contact;
                let foot_plane_offset = 88 - raw_left_bottom.max(raw_right_bottom);
                let left_bottom = (raw_left_bottom + foot_plane_offset) as usize;
                let right_bottom = (raw_right_bottom + foot_plane_offset) as usize;
                let left_leg = (36 + horizontal) as usize;
                let right_leg = (52 - horizontal) as usize;
                // The torso stops at y=67 and the legs begin below it. These
                // narrow bridges keep the silhouette one connected subject;
                // they deliberately precede the ordinary leg rectangles.
                for y in 68..(left_bottom - 16) {
                    for x in (left_leg + 3)..(left_leg + 5) {
                        let start = (y * 96 + x) * 4;
                        pixels[start..start + 4].copy_from_slice(&character_color);
                    }
                }
                for y in 68..(right_bottom - 16) {
                    for x in (right_leg + 3)..(right_leg + 5) {
                        let start = (y * 96 + x) * 4;
                        pixels[start..start + 4].copy_from_slice(&character_color);
                    }
                }
                let leg_width = if direction_pose_video { 10 } else { 8 };
                for y in (left_bottom - 16)..left_bottom {
                    for x in left_leg..(left_leg + leg_width) {
                        let start = (y * 96 + x) * 4;
                        pixels[start..start + 4].copy_from_slice(&character_color);
                    }
                }
                for y in (right_bottom - 16)..right_bottom {
                    for x in right_leg..(right_leg + leg_width) {
                        let start = (y * 96 + x) * 4;
                        pixels[start..start + 4].copy_from_slice(&character_color);
                    }
                }
            } else if locked_video {
                // One 22-frame locomotion cycle matches the extracted candidate
                // timeline. Repeating an eight-frame pattern inside the source
                // video let a valid loop interval alias opposite output poses to
                // the same source phase, which is not a useful V5 gait fixture.
                let cycle_index = index % 22;
                let timeline_phase = match cycle_index {
                    0..=1 => 0,
                    2..=4 => 1,
                    5..=6 => 2,
                    7..=9 => 3,
                    10..=12 => 4,
                    13..=14 => 5,
                    15..=17 => 6,
                    18..=20 => 7,
                    _ => 0,
                };
                let cycle_angle = std::f32::consts::TAU * timeline_phase as f32 / 8.0;
                let horizontal = (cycle_angle.sin() * 6.0).round() as i32;
                let contact = (cycle_angle.cos() * 4.0).round() as i32;
                let (left_leg, right_leg, left_top, left_bottom, right_top, right_bottom) = if idle
                {
                    let idle_stride = [-2i32, -1, 0, 1, 2, 1, 0, -1][phase as usize];
                    (
                        (34 + idle_stride) as usize,
                        (54 - idle_stride) as usize,
                        68usize,
                        88usize,
                        68usize,
                        88usize,
                    )
                } else {
                    let raw_left_bottom = 84 + contact;
                    let raw_right_bottom = 84 - contact;
                    let foot_plane_offset = 88 - raw_left_bottom.max(raw_right_bottom);
                    let left_bottom = raw_left_bottom + foot_plane_offset;
                    let right_bottom = raw_right_bottom + foot_plane_offset;
                    (
                        (36 + horizontal) as usize,
                        (52 - horizontal) as usize,
                        (left_bottom - 16) as usize,
                        left_bottom as usize,
                        (right_bottom - 16) as usize,
                        right_bottom as usize,
                    )
                };
                for y in 68..left_top {
                    for x in (left_leg + 3)..(left_leg + 5) {
                        let start = (y * 96 + x) * 4;
                        pixels[start..start + 4].copy_from_slice(&character_color);
                    }
                }
                for y in 68..right_top {
                    for x in (right_leg + 3)..(right_leg + 5) {
                        let start = (y * 96 + x) * 4;
                        pixels[start..start + 4].copy_from_slice(&character_color);
                    }
                }
                for y in left_top..left_bottom {
                    for x in left_leg..(left_leg + 8) {
                        let start = (y * 96 + x) * 4;
                        pixels[start..start + 4].copy_from_slice(&character_color);
                    }
                }
                for y in right_top..right_bottom {
                    for x in right_leg..(right_leg + 8) {
                        let start = (y * 96 + x) * 4;
                        pixels[start..start + 4].copy_from_slice(&character_color);
                    }
                }
            } else {
                let swing = if idle || (!edited && self.bad_loop_before_edit) {
                    0
                } else if phase <= 4 {
                    phase as usize
                } else {
                    (8 - phase) as usize
                };
                let left_leg = 32 + swing.min(18);
                let right_leg = 56usize.saturating_sub(swing.min(18));
                for y in 68..84 {
                    for x in left_leg..(left_leg + 8) {
                        let start = (y * 96 + x) * 4;
                        pixels[start..start + 4].copy_from_slice(&character_color);
                    }
                    for x in right_leg..(right_leg + 8) {
                        let start = (y * 96 + x) * 4;
                        pixels[start..start + 4].copy_from_slice(&character_color);
                    }
                }
                if idle && (edited || !self.bad_loop_before_edit) {
                    let breath_color = if phase <= 3 {
                        [220, 82, 104, 255]
                    } else {
                        character_color
                    };
                    for y in 56..66 {
                        for x in 41..55 {
                            let start = (y * 96 + x) * 4;
                            pixels[start..start + 4].copy_from_slice(&breath_color);
                        }
                    }
                }
            }
            if detached_effect && index % 2 == 0 {
                for y in 17..22 {
                    for x in 78..83 {
                        let start = (y * 96 + x) * 4;
                        pixels[start..start + 4].copy_from_slice(&[255, 210, 45, 255]);
                    }
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
        let mut capabilities = vec![
            ProviderCapability::GenerateImage,
            ProviderCapability::EditImage,
            ProviderCapability::GenerateVideo,
            ProviderCapability::ImageToVideo,
            ProviderCapability::ReferenceToVideo,
            ProviderCapability::Usage,
        ];
        if self.supports_video_edit {
            capabilities.extend([
                ProviderCapability::EditVideo,
                ProviderCapability::PrivateFileInput,
            ]);
        }
        capabilities
    }

    fn health_check(&self) -> ProviderHealth {
        ProviderHealth {
            provider_id: self.id().into(),
            available: true,
            authenticated: self.authenticated,
            auth_kind: CredentialKind::None,
            capabilities: self.capabilities(),
            constraints: Some(ProviderConstraints {
                max_image_references: Some(3),
                max_video_references: Some(7),
                native_alpha: false,
                video_edit: self.supports_video_edit,
                end_frame: false,
                private_file_input: self.supports_video_edit,
            }),
            message: Some("deterministic offline test provider".into()),
        }
    }

    fn resolved_image_model(&self, requested: Option<&str>) -> Option<String> {
        Some(requested.unwrap_or("fixture-image").to_string())
    }

    fn resolved_video_model(&self, requested: Option<&str>) -> Option<String> {
        Some(requested.unwrap_or("fixture-video").to_string())
    }

    fn resolved_video_edit_model(&self, requested: Option<&str>) -> Option<String> {
        Some(requested.unwrap_or("fixture-video-edit").to_string())
    }

    fn generate_image(
        &self,
        request: &GenerateImageRequest,
        output_path: &Path,
    ) -> Result<ProviderMedia, ProviderError> {
        self.observe(FixtureRequestObservation {
            kind: FixtureRequestKind::GenerateImage,
            prompt: request.prompt.clone(),
            resolution: Some(request.resolution.clone()),
            authorization_target: request.authorization_target.clone(),
            reference_roles: Vec::new(),
            video_input_path: None,
            video_input_sha256: None,
            video_input_dimensions: None,
        });
        self.usage.lock().unwrap().requests += 1;
        self.usage.lock().unwrap().generated_images += 1;
        let prompt = format!(
            "{} {}",
            request.prompt,
            request.model.as_deref().unwrap_or_default()
        );
        if request.prompt.contains("Forge direction lock sheet 2x2") {
            self.write_direction_sheet(output_path, &prompt)
        } else if request.prompt.contains("Forge animation sheet 2x2")
            || request.prompt.contains("Forge locked animation frames 2x2")
        {
            self.write_animation_sheet(output_path, &prompt)
        } else {
            self.write_image(output_path, &prompt, None)
        }
    }

    fn edit_image(
        &self,
        request: &EditImageRequest,
        output_path: &Path,
    ) -> Result<ProviderMedia, ProviderError> {
        self.observe(FixtureRequestObservation {
            kind: FixtureRequestKind::EditImage,
            prompt: request.prompt.clone(),
            resolution: Some(request.resolution.clone()),
            authorization_target: request.authorization_target.clone(),
            reference_roles: request
                .references
                .iter()
                .map(|reference| reference.role)
                .collect(),
            video_input_path: None,
            video_input_sha256: None,
            video_input_dimensions: None,
        });
        self.usage.lock().unwrap().requests += 1;
        self.edit_observations
            .lock()
            .unwrap()
            .push(FixtureImageEditObservation {
                authorization_target: request.authorization_target.clone(),
                reference_roles: request
                    .references
                    .iter()
                    .map(|reference| reference.role)
                    .collect(),
                reference_sha256: request
                    .references
                    .iter()
                    .map(|reference| reference.sha256.clone())
                    .collect(),
            });
        if let Some(marker) = &self.fail_edit_once_marker {
            if request.prompt.contains(marker) {
                let mut failed = self.failed_edit_markers.lock().unwrap();
                if !failed.get(marker).copied().unwrap_or_default() {
                    failed.insert(marker.clone(), true);
                    return Err(ProviderError::Request(
                        "fixture one-shot image edit transport failure".into(),
                    ));
                }
            }
        }
        self.usage.lock().unwrap().generated_images += 1;
        let prompt = format!(
            "{} {}",
            request.prompt,
            request.model.as_deref().unwrap_or_default()
        );
        if request.prompt.contains("[fixture:wrong_output_path]")
            && request.prompt.contains("Forge animation sheet 2x2")
        {
            let wrong_path = output_path.with_file_name("wrong-provider-sheet.png");
            return self.write_animation_sheet(&wrong_path, &prompt);
        }
        if request.prompt.contains("Forge direction lock sheet 2x2") {
            self.write_direction_sheet(output_path, &prompt)
        } else if request.prompt.contains("Forge animation sheet 2x2")
            || request.prompt.contains("Forge locked animation frames 2x2")
        {
            self.write_animation_sheet(output_path, &prompt)
        } else {
            let pose_structure = request
                .references
                .iter()
                .find(|reference| reference.role == ReferenceRole::PoseStructure)
                .map(|reference| reference.path.as_path());
            self.write_image(output_path, &prompt, pose_structure)
        }
    }

    fn generate_video(
        &self,
        request: &GenerateVideoRequest,
    ) -> Result<ProviderTicket, ProviderError> {
        let topdown_cycle_v10 = request.prompt.contains(TOPDOWN_CYCLE_V10_MARKER);
        let direction_pose_video = request.prompt.contains(DIRECTION_POSE_V7_MARKER);
        let direction_motion_video = request.prompt.contains(DIRECTION_MOTION_V8_MARKER);
        let cycle_video = request.prompt.contains(VIDEO_CYCLE_V6_MARKER)
            || topdown_cycle_v10
            || direction_pose_video
            || direction_motion_video;
        let video_input_path = match &request.mode {
            VideoGenerationMode::ImageToVideo { image } => Some(image.clone()),
            VideoGenerationMode::Text | VideoGenerationMode::ReferenceToVideo { .. } => None,
        };
        let video_input_dimensions = video_input_path
            .as_deref()
            .and_then(|path| image::image_dimensions(path).ok());
        let video_input_sha256 = video_input_path
            .as_deref()
            .map(forge_core::asset_project::hash_file)
            .transpose()
            .ok()
            .flatten();
        self.observe(FixtureRequestObservation {
            kind: FixtureRequestKind::GenerateVideo,
            prompt: request.prompt.clone(),
            resolution: Some(request.resolution.clone()),
            authorization_target: request.authorization_target.clone(),
            reference_roles: Vec::new(),
            video_input_path,
            video_input_sha256,
            video_input_dimensions,
        });
        let mut next_ticket = self.next_ticket.lock().unwrap();
        let id = format!("fixture-{}", *next_ticket);
        *next_ticket = next_ticket.saturating_add(1);
        drop(next_ticket);
        self.tickets.lock().unwrap().insert(
            id.clone(),
            FixtureTicket {
                polls: 0,
                edited: false,
                back_facing: request.prompt.contains("facing up")
                    || request.prompt.contains("rear/up-facing"),
                right_facing: request.prompt.contains("facing right")
                    || request.prompt.contains("right-facing"),
                left_facing: request.prompt.contains("facing left")
                    || request.prompt.contains("left-facing"),
                detached_effect: self.detached_walk_up_effect
                    && request.prompt.contains("facing up"),
                idle: request.prompt.contains("idle loop")
                    || (cycle_video && request.prompt.contains("breathing-idle")),
                locked_video: request.prompt.contains("supplied DirectionLock sprite"),
                cycle_video,
                direction_pose_video: direction_pose_video || direction_motion_video,
                topdown_cycle_video: topdown_cycle_v10,
            },
        );
        self.usage.lock().unwrap().requests += 1;
        Ok(ProviderTicket {
            provider_id: self.id().into(),
            request_id: id,
        })
    }

    fn edit_video(&self, request: &EditVideoRequest) -> Result<ProviderTicket, ProviderError> {
        let direction_pose_video = request.prompt.contains(DIRECTION_POSE_V7_MARKER);
        let cycle_video = request.prompt.contains(VIDEO_CYCLE_V6_MARKER) || direction_pose_video;
        self.observe(FixtureRequestObservation {
            kind: FixtureRequestKind::EditVideo,
            prompt: request.prompt.clone(),
            resolution: None,
            authorization_target: request.authorization_target.clone(),
            reference_roles: Vec::new(),
            video_input_path: Some(request.video.path.clone()),
            video_input_sha256: Some(request.video.sha256.clone()),
            video_input_dimensions: image::image_dimensions(&request.video.path).ok(),
        });
        if !self.supports_video_edit {
            return Err(ProviderError::Unavailable(
                "fixture video editing is disabled".into(),
            ));
        }
        let mut next_ticket = self.next_ticket.lock().unwrap();
        let id = format!("fixture-edit-{}", *next_ticket);
        *next_ticket = next_ticket.saturating_add(1);
        drop(next_ticket);
        self.tickets.lock().unwrap().insert(
            id.clone(),
            FixtureTicket {
                polls: 0,
                edited: true,
                back_facing: request.prompt.contains("facing up"),
                right_facing: request.prompt.contains("facing right"),
                left_facing: request.prompt.contains("facing left"),
                detached_effect: self.detached_walk_up_effect
                    && request.prompt.contains("facing up"),
                idle: request.prompt.contains("idle loop")
                    || (cycle_video && request.prompt.contains("breathing-idle")),
                locked_video: request.prompt.contains("supplied DirectionLock sprite"),
                cycle_video,
                direction_pose_video,
                topdown_cycle_video: false,
            },
        );
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
        let Some(fixture_ticket) = tickets.get_mut(&ticket.request_id) else {
            return Ok(ProviderPoll::Failed {
                code: "unknown_fixture_ticket".into(),
                message: "fixture ticket was not found".into(),
            });
        };
        if std::env::var("FORGE_FIXTURE_POLL_PENDING_ONCE").as_deref() == Ok("1")
            && fixture_ticket.polls == 0
        {
            fixture_ticket.polls += 1;
            return Ok(ProviderPoll::Pending { progress: Some(50) });
        }
        let fixture_ticket = tickets
            .remove(&ticket.request_id)
            .expect("fixture ticket existed before removal");
        drop(tickets);
        self.usage.lock().unwrap().generated_videos += 1;
        Ok(ProviderPoll::Succeeded(
            self.write_video(output_path, fixture_ticket)?,
        ))
    }

    fn cancel(&self, ticket: &ProviderTicket) -> Result<(), ProviderError> {
        self.tickets.lock().unwrap().remove(&ticket.request_id);
        Ok(())
    }

    fn usage(&self) -> ProviderUsage {
        self.usage.lock().unwrap().clone()
    }
}
