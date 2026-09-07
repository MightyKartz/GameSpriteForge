pub mod auth;
pub mod authorization;
pub mod fixture;
pub mod pixellab;
pub mod xai;

use std::sync::Arc;

use forge_core::provider::{
    CredentialKind, EditImageRequest, EditVideoRequest, GenerateImageRequest, GenerateVideoRequest,
    MediaGenerationProvider, ProviderCapability, ProviderConstraints, ProviderError,
    ProviderHealth, ProviderMedia, ProviderPoll, ProviderTicket, ProviderUsage,
};
use serde::Serialize;

use crate::auth::resolve_xai_credentials_for_profile;
use crate::authorization::ProviderAuthorizationConfig;
use crate::fixture::FixtureProvider;
use crate::xai::XaiProvider;

pub const XAI_PROVIDER_ID: &str = "xai";
pub const FIXTURE_PROVIDER_ID: &str = "fixture";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderModelMediaKind {
    Image,
    Video,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderModelRouteStatus {
    Incumbent,
    Candidate,
    EditOnly,
}

/// Static product routing, not a live account entitlement response. The
/// separate xAI model-catalog endpoint remains the authority for whether an
/// authenticated team can currently call a model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderModelRoute {
    pub provider_id: String,
    pub model_id: String,
    pub media_kind: ProviderModelMediaKind,
    pub route_status: ProviderModelRouteStatus,
    pub default_for_kind: bool,
    pub requires_explicit_opt_in: bool,
    pub capabilities: Vec<String>,
    pub note: String,
}

pub fn list_provider_model_routes(
    provider_id: &str,
) -> Result<Vec<ProviderModelRoute>, ProviderError> {
    let route = |model_id: &str,
                 media_kind,
                 route_status,
                 default_for_kind,
                 requires_explicit_opt_in,
                 capabilities: &[&str],
                 note: &str| ProviderModelRoute {
        provider_id: provider_id.into(),
        model_id: model_id.into(),
        media_kind,
        route_status,
        default_for_kind,
        requires_explicit_opt_in,
        capabilities: capabilities.iter().map(|value| (*value).into()).collect(),
        note: note.into(),
    };
    match provider_id {
        XAI_PROVIDER_ID => Ok(vec![
            route(
                xai::DEFAULT_IMAGE_MODEL,
                ProviderModelMediaKind::Image,
                ProviderModelRouteStatus::Incumbent,
                true,
                false,
                &["generate_image", "edit_image"],
                "current production image default",
            ),
            route(
                xai::IMAGE_2_CANDIDATE_MODEL,
                ProviderModelMediaKind::Image,
                ProviderModelRouteStatus::Candidate,
                false,
                true,
                &["generate_image", "edit_image"],
                "evaluation-only: approved V9 DirectionGrid comparison, exact 1/1 authorization",
            ),
            route(
                xai::DEFAULT_VIDEO_MODEL,
                ProviderModelMediaKind::Video,
                ProviderModelRouteStatus::Incumbent,
                true,
                false,
                &["generate_video", "image_to_video", "reference_to_video"],
                "current production video generation default",
            ),
            route(
                xai::DEFAULT_VIDEO_EDIT_MODEL,
                ProviderModelMediaKind::Video,
                ProviderModelRouteStatus::EditOnly,
                false,
                false,
                &["edit_video"],
                "video editing route; not a generation fallback",
            ),
        ]),
        FIXTURE_PROVIDER_ID => Ok(vec![
            route(
                "fixture-image",
                ProviderModelMediaKind::Image,
                ProviderModelRouteStatus::Incumbent,
                true,
                false,
                &["generate_image", "edit_image"],
                "deterministic offline image model",
            ),
            route(
                "fixture-video",
                ProviderModelMediaKind::Video,
                ProviderModelRouteStatus::Incumbent,
                true,
                false,
                &["generate_video", "image_to_video", "reference_to_video"],
                "deterministic offline video model",
            ),
        ]),
        other => Err(ProviderError::Unavailable(format!(
            "unknown provider: {other}"
        ))),
    }
}

/// A no-network Provider used only when a retry replays retained character
/// media through local processing stages. It preserves the source Provider ID
/// for provenance while making an accidental paid request fail closed.
pub struct LocalReplayProvider {
    provider_id: &'static str,
}

impl LocalReplayProvider {
    pub fn new(provider_id: &str) -> Result<Self, ProviderError> {
        let provider_id = match provider_id {
            XAI_PROVIDER_ID => XAI_PROVIDER_ID,
            FIXTURE_PROVIDER_ID => FIXTURE_PROVIDER_ID,
            other => {
                return Err(ProviderError::Unavailable(format!(
                    "local replay does not recognize provider: {other}"
                )))
            }
        };
        Ok(Self { provider_id })
    }

    fn network_disabled() -> ProviderError {
        ProviderError::Unavailable("local-only replay attempted a Provider media request".into())
    }
}

impl MediaGenerationProvider for LocalReplayProvider {
    fn id(&self) -> &'static str {
        self.provider_id
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
            ProviderCapability::Cancel,
            ProviderCapability::Usage,
        ]
    }

    fn health_check(&self) -> ProviderHealth {
        ProviderHealth {
            provider_id: self.provider_id.into(),
            available: true,
            authenticated: true,
            auth_kind: CredentialKind::None,
            capabilities: self.capabilities(),
            constraints: Some(ProviderConstraints {
                max_image_references: Some(3),
                max_video_references: Some(1),
                native_alpha: false,
                video_edit: true,
                end_frame: false,
                private_file_input: true,
            }),
            message: Some("local-only replay; network media operations are disabled".into()),
        }
    }

    fn generate_image(
        &self,
        _request: &GenerateImageRequest,
        _output_path: &std::path::Path,
    ) -> Result<ProviderMedia, ProviderError> {
        Err(Self::network_disabled())
    }

    fn edit_image(
        &self,
        _request: &EditImageRequest,
        _output_path: &std::path::Path,
    ) -> Result<ProviderMedia, ProviderError> {
        Err(Self::network_disabled())
    }

    fn generate_video(
        &self,
        _request: &GenerateVideoRequest,
    ) -> Result<ProviderTicket, ProviderError> {
        Err(Self::network_disabled())
    }

    fn edit_video(&self, _request: &EditVideoRequest) -> Result<ProviderTicket, ProviderError> {
        Err(Self::network_disabled())
    }

    fn poll(
        &self,
        _ticket: &ProviderTicket,
        _output_path: &std::path::Path,
    ) -> Result<ProviderPoll, ProviderError> {
        Err(Self::network_disabled())
    }

    fn cancel(&self, _ticket: &ProviderTicket) -> Result<(), ProviderError> {
        Err(Self::network_disabled())
    }

    fn usage(&self) -> ProviderUsage {
        ProviderUsage::default()
    }
}

pub fn resolve_local_replay_provider(
    provider_id: &str,
) -> Result<Arc<dyn MediaGenerationProvider>, ProviderError> {
    Ok(Arc::new(LocalReplayProvider::new(provider_id)?))
}

pub fn resolve_image_model(
    provider_id: &str,
    requested: Option<&str>,
) -> Result<String, ProviderError> {
    match provider_id {
        XAI_PROVIDER_ID => Ok(requested
            .map(str::to_owned)
            .unwrap_or_else(|| XaiProvider::default_image_model().to_string())),
        FIXTURE_PROVIDER_ID => Ok(requested.unwrap_or("fixture-image").to_string()),
        other => Err(ProviderError::Unavailable(format!(
            "unknown provider: {other}"
        ))),
    }
}

pub fn resolve_video_model(
    provider_id: &str,
    requested: Option<&str>,
) -> Result<String, ProviderError> {
    match provider_id {
        XAI_PROVIDER_ID => Ok(requested
            .map(str::to_owned)
            .unwrap_or_else(|| XaiProvider::default_video_model().to_string())),
        FIXTURE_PROVIDER_ID => Ok(requested.unwrap_or("fixture-video").to_string()),
        other => Err(ProviderError::Unavailable(format!(
            "unknown provider: {other}"
        ))),
    }
}

pub fn list_provider_health() -> Vec<ProviderHealth> {
    let xai = resolve_xai_credentials_for_profile("default")
        .map(XaiProvider::new)
        .map(|provider| provider.health_check())
        .unwrap_or_else(|error| ProviderHealth {
            provider_id: XAI_PROVIDER_ID.into(),
            available: true,
            authenticated: false,
            auth_kind: CredentialKind::None,
            capabilities: XaiProvider::capability_list(),
            constraints: Some(XaiProvider::constraints()),
            message: Some(error.to_string()),
        });
    vec![xai, FixtureProvider::default().health_check()]
}

pub fn list_provider_health_noninteractive() -> Vec<ProviderHealth> {
    let api_key_available = std::env::var("XAI_API_KEY")
        .ok()
        .is_some_and(|value| !value.trim().is_empty());
    vec![
        ProviderHealth {
            provider_id: XAI_PROVIDER_ID.into(),
            available: true,
            authenticated: api_key_available,
            auth_kind: if api_key_available {
                CredentialKind::ApiKey
            } else {
                CredentialKind::None
            },
            capabilities: XaiProvider::capability_list(),
            constraints: Some(XaiProvider::constraints()),
            message: (!api_key_available).then(|| {
                "credential status is intentionally not read during list/doctor; run `forge provider doctor --provider xai` for an explicit Keychain check".into()
            }),
        },
        FixtureProvider::default().health_check(),
    ]
}

pub fn resolve_provider(
    provider_id: &str,
    profile_id: &str,
) -> Result<Arc<dyn MediaGenerationProvider>, ProviderError> {
    match provider_id {
        XAI_PROVIDER_ID => {
            let credentials = resolve_xai_credentials_for_profile(profile_id)?;
            Ok(Arc::new(XaiProvider::new(credentials)))
        }
        FIXTURE_PROVIDER_ID => Ok(Arc::new(FixtureProvider::default())),
        other => Err(ProviderError::Unavailable(format!(
            "unknown provider: {other}"
        ))),
    }
}

/// Resolve a Provider whose real network requests are charged against one
/// durable, caller-created authorization ledger. Fixture remains offline and
/// ignores the authorization config so existing deterministic tests keep the
/// same behavior.
pub fn resolve_provider_with_authorization(
    provider_id: &str,
    profile_id: &str,
    authorization: ProviderAuthorizationConfig,
) -> Result<Arc<dyn MediaGenerationProvider>, ProviderError> {
    match provider_id {
        XAI_PROVIDER_ID => {
            let credentials = resolve_xai_credentials_for_profile(profile_id)?;
            Ok(Arc::new(XaiProvider::new_with_authorization(
                credentials,
                profile_id,
                authorization,
            )?))
        }
        FIXTURE_PROVIDER_ID => Ok(Arc::new(FixtureProvider::default())),
        other => Err(ProviderError::Unavailable(format!(
            "unknown provider: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use forge_core::provider::{GenerateImageRequest, ProviderError};

    use super::*;

    #[test]
    fn xai_model_routes_keep_image2_candidate_opt_in_and_quality_as_default() {
        let routes = list_provider_model_routes(XAI_PROVIDER_ID).unwrap();
        let incumbent = routes
            .iter()
            .find(|route| route.model_id == xai::DEFAULT_IMAGE_MODEL)
            .unwrap();
        assert_eq!(incumbent.media_kind, ProviderModelMediaKind::Image);
        assert_eq!(incumbent.route_status, ProviderModelRouteStatus::Incumbent);
        assert!(incumbent.default_for_kind);
        assert!(!incumbent.requires_explicit_opt_in);

        let candidate = routes
            .iter()
            .find(|route| route.model_id == xai::IMAGE_2_CANDIDATE_MODEL)
            .unwrap();
        assert_eq!(candidate.media_kind, ProviderModelMediaKind::Image);
        assert_eq!(candidate.route_status, ProviderModelRouteStatus::Candidate);
        assert!(!candidate.default_for_kind);
        assert!(candidate.requires_explicit_opt_in);
        assert_eq!(XaiProvider::default_image_model(), xai::DEFAULT_IMAGE_MODEL);
    }

    #[test]
    fn xai_local_replay_is_authenticated_without_credentials_and_fails_closed() {
        let provider = resolve_local_replay_provider(XAI_PROVIDER_ID).unwrap();
        let health = provider.health_check();
        assert_eq!(health.provider_id, XAI_PROVIDER_ID);
        assert!(health.available);
        assert!(health.authenticated);
        assert_eq!(health.auth_kind, CredentialKind::None);
        assert_eq!(provider.usage(), ProviderUsage::default());

        let error = provider
            .generate_image(
                &GenerateImageRequest {
                    prompt: "must not execute".into(),
                    model: None,
                    aspect_ratio: "1:1".into(),
                    resolution: "1k".into(),
                    authorization_target: None,
                },
                Path::new("unused.png"),
            )
            .unwrap_err();
        assert!(matches!(error, ProviderError::Unavailable(_)));
    }
}
