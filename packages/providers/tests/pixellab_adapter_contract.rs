use forge_core::provider::{
    EditImageRequest, MediaGenerationProvider, ProviderCapability, ProviderImageReference,
    ReferenceRole,
};
use forge_providers::pixellab::{PixelLabLoopbackProvider, PIXELLAB_LOOPBACK_ID};

#[test]
fn pixellab_loopback_adapter_satisfies_the_existing_provider_boundary() {
    let temp = tempfile::tempdir().unwrap();
    let reference_path = temp.path().join("reference.png");
    image::RgbaImage::from_pixel(16, 16, image::Rgba([40, 80, 120, 255]))
        .save(&reference_path)
        .unwrap();
    let provider = PixelLabLoopbackProvider::default();
    assert_eq!(provider.id(), PIXELLAB_LOOPBACK_ID);
    assert!(provider
        .capabilities()
        .contains(&ProviderCapability::EditImage));
    let health = provider.health_check();
    assert!(health.available);
    assert!(health.authenticated);
    assert_eq!(
        health
            .constraints
            .as_ref()
            .and_then(|constraints| constraints.max_image_references),
        Some(3)
    );
    assert!(health
        .constraints
        .as_ref()
        .is_some_and(|constraints| constraints.native_alpha));

    let output = temp.path().join("grid.png");
    let media = provider
        .edit_image(
            &EditImageRequest {
                prompt: "2x2 four-direction character grid".into(),
                model: None,
                references: vec![
                    ProviderImageReference::from_path(
                        ReferenceRole::SubjectIdentity,
                        reference_path.clone(),
                    )
                    .unwrap(),
                    ProviderImageReference::from_path(ReferenceRole::Style, reference_path.clone())
                        .unwrap(),
                    ProviderImageReference::from_path(ReferenceRole::PoseStructure, reference_path)
                        .unwrap(),
                ],
                aspect_ratio: "1:1".into(),
                resolution: "1k".into(),
                authorization_target: Some("direction_grid".into()),
            },
            &output,
        )
        .unwrap();
    assert_eq!(media.mime_type, "image/png");
    let image = image::open(&media.path).unwrap().to_rgba8();
    assert_eq!(image.dimensions(), (192, 192));
    assert_eq!(provider.usage().requests, 1);
    assert_eq!(provider.usage().generated_images, 1);
    assert_eq!(provider.usage().generated_videos, 0);
}
