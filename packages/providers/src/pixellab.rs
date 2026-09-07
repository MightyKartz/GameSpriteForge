use std::fs;
use std::path::Path;
use std::sync::Mutex;

use forge_core::provider::{
    CredentialKind, EditImageRequest, EditVideoRequest, GenerateImageRequest, GenerateVideoRequest,
    MediaGenerationProvider, ProviderCapability, ProviderConstraints, ProviderError,
    ProviderHealth, ProviderMedia, ProviderPoll, ProviderTicket, ProviderUsage,
};
use image::{ImageBuffer, Rgba, RgbaImage};

pub const PIXELLAB_LOOPBACK_ID: &str = "pixellab-loopback";

/// Interface alignment for a PixelLab-style grid Provider. This adapter is
/// deliberately loopback-only: it never opens a socket, reads a key, or sends
/// a real request. It exists to prove that Forge can adopt a grid-specialized
/// Provider through the existing `MediaGenerationProvider` boundary without
/// changing core code.
#[derive(Default)]
pub struct PixelLabLoopbackProvider {
    usage: Mutex<ProviderUsage>,
}

impl PixelLabLoopbackProvider {
    fn write_grid(
        &self,
        output_path: &Path,
        prompt: &str,
        reference_count: usize,
    ) -> Result<ProviderMedia, ProviderError> {
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let seed = prompt.bytes().fold(0_u32, |seed, byte| {
            seed.wrapping_mul(33).wrapping_add(u32::from(byte))
        });
        let mut sheet: RgbaImage = ImageBuffer::from_pixel(192, 192, Rgba([0, 0, 0, 0]));
        for frame in 0..4_u32 {
            let base_x = (frame % 2) * 96;
            let base_y = (frame / 2) * 96;
            let color = [
                (64 + seed.wrapping_add(frame * 37) % 128) as u8,
                (96 + reference_count as u32 * 32) as u8,
                (72 + frame * 29) as u8,
                255,
            ];
            for y in 20..76 {
                for x in 30..66 {
                    sheet.put_pixel(base_x + x, base_y + y, Rgba(color));
                }
            }
            for y in 68..90 {
                for x in (36 + frame * 2)..(50 + frame * 2) {
                    sheet.put_pixel(base_x + x, base_y + y, Rgba(color));
                }
                for x in (58 - frame * 2)..(72 - frame * 2) {
                    sheet.put_pixel(base_x + x, base_y + y, Rgba(color));
                }
            }
        }
        sheet
            .save(output_path)
            .map_err(|error| ProviderError::InvalidOutput(error.to_string()))?;
        Ok(ProviderMedia {
            path: output_path.to_path_buf(),
            mime_type: "image/png".into(),
            provider_asset_id: Some("pixellab-loopback-grid".into()),
            revised_prompt: None,
        })
    }
}

impl MediaGenerationProvider for PixelLabLoopbackProvider {
    fn id(&self) -> &'static str {
        PIXELLAB_LOOPBACK_ID
    }

    fn capabilities(&self) -> Vec<ProviderCapability> {
        vec![
            ProviderCapability::GenerateImage,
            ProviderCapability::EditImage,
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
            constraints: Some(ProviderConstraints {
                max_image_references: Some(3),
                max_video_references: Some(0),
                native_alpha: true,
                video_edit: false,
                end_frame: false,
                private_file_input: true,
            }),
            message: Some("PixelLab loopback adapter; no real Provider request".into()),
        }
    }

    fn resolved_image_model(&self, requested: Option<&str>) -> Option<String> {
        Some(
            requested
                .unwrap_or("pixellab-grid-fixture@1.0.0")
                .to_string(),
        )
    }

    fn generate_image(
        &self,
        request: &GenerateImageRequest,
        output_path: &Path,
    ) -> Result<ProviderMedia, ProviderError> {
        self.usage.lock().unwrap().requests += 1;
        self.usage.lock().unwrap().generated_images += 1;
        self.write_grid(output_path, &request.prompt, 0)
    }

    fn edit_image(
        &self,
        request: &EditImageRequest,
        output_path: &Path,
    ) -> Result<ProviderMedia, ProviderError> {
        if request.references.len() > 3 {
            return Err(ProviderError::InvalidOutput(
                "PixelLab loopback accepts at most three image references".into(),
            ));
        }
        self.usage.lock().unwrap().requests += 1;
        self.usage.lock().unwrap().generated_images += 1;
        self.write_grid(output_path, &request.prompt, request.references.len())
    }

    fn generate_video(
        &self,
        _request: &GenerateVideoRequest,
    ) -> Result<ProviderTicket, ProviderError> {
        Err(ProviderError::Unavailable(
            "PixelLab loopback adapter does not implement video".into(),
        ))
    }

    fn edit_video(&self, _request: &EditVideoRequest) -> Result<ProviderTicket, ProviderError> {
        Err(ProviderError::Unavailable(
            "PixelLab loopback adapter does not implement video editing".into(),
        ))
    }

    fn poll(
        &self,
        _ticket: &ProviderTicket,
        _output_path: &Path,
    ) -> Result<ProviderPoll, ProviderError> {
        Err(ProviderError::Unavailable(
            "PixelLab loopback adapter has no asynchronous jobs".into(),
        ))
    }

    fn cancel(&self, _ticket: &ProviderTicket) -> Result<(), ProviderError> {
        Ok(())
    }

    fn usage(&self) -> ProviderUsage {
        self.usage.lock().unwrap().clone()
    }
}
