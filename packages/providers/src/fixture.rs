use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use forge_core::provider::{
    CredentialKind, EditImageRequest, EditVideoRequest, GenerateImageRequest, GenerateVideoRequest,
    MediaGenerationProvider, ProviderCapability, ProviderConstraints, ProviderError,
    ProviderHealth, ProviderMedia, ProviderPoll, ProviderTicket, ProviderUsage,
};
use gif::{Encoder, Frame, Repeat};
use image::{ImageBuffer, Rgba, RgbaImage};

struct FixtureTicket {
    polls: u8,
    edited: bool,
}

pub struct FixtureProvider {
    tickets: Mutex<HashMap<String, FixtureTicket>>,
    usage: Mutex<ProviderUsage>,
    bad_loop_before_edit: bool,
    supports_video_edit: bool,
    review_keyframes: bool,
}

impl Default for FixtureProvider {
    fn default() -> Self {
        Self {
            tickets: Mutex::new(HashMap::new()),
            usage: Mutex::new(ProviderUsage::default()),
            bad_loop_before_edit: false,
            supports_video_edit: true,
            review_keyframes: false,
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

    fn write_image(
        &self,
        output_path: &Path,
        prompt: &str,
    ) -> Result<ProviderMedia, ProviderError> {
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut image: RgbaImage = ImageBuffer::from_pixel(96, 96, Rgba([0, 255, 0, 255]));
        let keyframe_phase = prompt
            .split("Forge frame phase ")
            .nth(1)
            .and_then(|value| value.split('/').next())
            .and_then(|value| value.parse::<u8>().ok());
        let color = if prompt.contains("[fixture:palette_drift]") {
            Rgba([220, 90, 60, 255])
        } else if prompt.contains("[fixture:review]")
            || (self.review_keyframes && keyframe_phase.is_some())
        {
            Rgba([180, 80, 120, 255])
        } else {
            Rgba([130, 60, 210, 255])
        };
        let articulated = keyframe_phase.is_some() || prompt.contains("canonical identity image");
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
        } else if articulated {
            for y in 28..68 {
                for x in 34..62 {
                    image.put_pixel(x, y, color);
                }
            }
            let phase = keyframe_phase.unwrap_or(0) % 8;
            let swing = (if phase <= 4 { phase } else { 8 - phase } as u32).min(3);
            let left = 30 + swing;
            let right = 54_u32.saturating_sub(swing);
            for y in 68..84 {
                for x in left..(left + 8) {
                    image.put_pixel(x, y, color);
                }
                for x in right..(right + 8) {
                    image.put_pixel(x, y, color);
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

    fn write_video(
        &self,
        output_path: &Path,
        edited: bool,
    ) -> Result<ProviderMedia, ProviderError> {
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
            let swing = if !edited && self.bad_loop_before_edit {
                0
            } else if phase <= 4 {
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
            authenticated: true,
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
        self.tickets.lock().unwrap().insert(
            id.clone(),
            FixtureTicket {
                polls: 0,
                edited: false,
            },
        );
        self.usage.lock().unwrap().requests += 1;
        Ok(ProviderTicket {
            provider_id: self.id().into(),
            request_id: id,
        })
    }

    fn edit_video(&self, _request: &EditVideoRequest) -> Result<ProviderTicket, ProviderError> {
        if !self.supports_video_edit {
            return Err(ProviderError::Unavailable(
                "fixture video editing is disabled".into(),
            ));
        }
        let id = format!("fixture-edit-{}", self.tickets.lock().unwrap().len() + 1);
        self.tickets.lock().unwrap().insert(
            id.clone(),
            FixtureTicket {
                polls: 0,
                edited: true,
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
        let edited = fixture_ticket.edited;
        tickets.remove(&ticket.request_id);
        drop(tickets);
        self.usage.lock().unwrap().generated_videos += 1;
        Ok(ProviderPoll::Succeeded(
            self.write_video(output_path, edited)?,
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
