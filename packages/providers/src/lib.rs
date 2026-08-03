pub mod auth;
pub mod fixture;
pub mod xai;

use std::sync::Arc;

use forge_core::provider::{
    CredentialKind, MediaGenerationProvider, ProviderError, ProviderHealth,
};

use crate::auth::resolve_xai_credentials_for_profile;
use crate::fixture::FixtureProvider;
use crate::xai::XaiProvider;

pub const XAI_PROVIDER_ID: &str = "xai";
pub const FIXTURE_PROVIDER_ID: &str = "fixture";

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
