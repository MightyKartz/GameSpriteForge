use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use forge_core::provider::{CredentialKind, CredentialProvider, ProviderError};
use reqwest::blocking::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use url::Url;

const XAI_OAUTH_ISSUER: &str = "https://auth.x.ai";
const XAI_OAUTH_DISCOVERY_URL: &str = "https://auth.x.ai/.well-known/openid-configuration";
const XAI_OAUTH_CLIENT_ID: &str = "b1a00492-073a-47ea-816f-4c329264a828";
const XAI_OAUTH_SCOPE: &str = "openid profile email offline_access grok-cli:access api:access";
const XAI_DEVICE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";
const KEYRING_SERVICE: &str = "dev.gamespriteforge.xai.oauth";
const API_KEY_KEYRING_SERVICE: &str = "dev.gamespriteforge.xai.api-key";
const RESPONSE_LIMIT_BYTES: usize = 1024 * 1024;
const REFRESH_SKEW_SECONDS: i64 = 3600;
const AUTH_PREFERENCE_SCHEMA_VERSION: &str = "1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum XaiAuthMethod {
    ApiKey,
    OAuthDeviceCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialStorageKind {
    Keychain,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct XaiAuthPreference {
    schema_version: String,
    method: XaiAuthMethod,
    storage: CredentialStorageKind,
}

#[derive(Clone)]
pub struct CredentialStore {
    profile_id: String,
    file_path: Option<PathBuf>,
    allow_file_fallback: bool,
    force_file_storage: bool,
}

impl CredentialStore {
    pub fn system(profile_id: impl Into<String>) -> Self {
        let profile_id = profile_id.into();
        Self {
            file_path: default_fallback_path(&profile_id),
            profile_id,
            allow_file_fallback: env_flag("FORGE_ALLOW_FILE_TOKEN_STORAGE"),
            force_file_storage: env_flag("FORGE_FORCE_FILE_TOKEN_STORAGE"),
        }
    }

    pub fn with_file_fallback(mut self, allow: bool) -> Self {
        self.allow_file_fallback = allow;
        self
    }

    pub fn with_file_storage(mut self, force: bool) -> Self {
        self.force_file_storage = force;
        if force {
            self.allow_file_fallback = true;
        }
        self
    }

    fn configured_storage(&self) -> CredentialStorageKind {
        if self.force_file_storage {
            CredentialStorageKind::File
        } else {
            CredentialStorageKind::Keychain
        }
    }

    pub fn file(profile_id: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            profile_id: profile_id.into(),
            file_path: Some(path.into()),
            allow_file_fallback: true,
            force_file_storage: true,
        }
    }

    fn load(&self) -> Result<Option<OAuthTokenState>, ProviderError> {
        if self.force_file_storage {
            return self.load_fallback();
        }
        match keyring_entry(&self.profile_id).and_then(|entry| entry.get_password()) {
            Ok(value) => decode_token_state(&value).map(Some),
            Err(keyring::Error::NoEntry) => self.load_fallback(),
            Err(error) if self.allow_file_fallback => self.load_fallback().map_err(|fallback| {
                ProviderError::Request(format!(
                    "system credential store failed ({error}); fallback failed ({fallback})"
                ))
            }),
            Err(error) => Err(ProviderError::Unavailable(format!(
                "system credential store failed: {error}; Preview OAuth development may opt into an owner-only file with --credential-store file"
            ))),
        }
    }

    fn save(&self, state: &OAuthTokenState) -> Result<CredentialStorageKind, ProviderError> {
        let encoded = serde_json::to_string(state)
            .map_err(|error| ProviderError::InvalidOutput(error.to_string()))?;
        if self.force_file_storage {
            self.save_fallback(&encoded)?;
            return Ok(CredentialStorageKind::File);
        }
        match keyring_entry(&self.profile_id).and_then(|entry| entry.set_password(&encoded)) {
            Ok(()) => Ok(CredentialStorageKind::Keychain),
            Err(error) if self.allow_file_fallback => {
                self.save_fallback(&encoded).map_err(|fallback| {
                    ProviderError::Request(format!(
                        "system credential store failed ({error}); fallback failed ({fallback})"
                    ))
                })?;
                Ok(CredentialStorageKind::File)
            }
            Err(error) => Err(ProviderError::Unavailable(format!(
                "system credential store failed: {error}; Preview OAuth development may opt into an owner-only file with --credential-store file"
            ))),
        }
    }

    fn delete(&self) -> Result<(), ProviderError> {
        let mut keyring_failure = None;
        if !self.force_file_storage {
            match keyring_entry(&self.profile_id).and_then(|entry| entry.delete_credential()) {
                Ok(()) | Err(keyring::Error::NoEntry) => {}
                Err(error) => keyring_failure = Some(error),
            }
        }
        let mut removed_fallback = false;
        if let Some(path) = &self.file_path {
            match fs::remove_file(path) {
                Ok(()) => removed_fallback = true,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        if let Some(error) = keyring_failure {
            if !removed_fallback {
                return Err(ProviderError::Request(format!(
                    "system credential delete failed: {error}"
                )));
            }
        }
        Ok(())
    }

    fn load_fallback(&self) -> Result<Option<OAuthTokenState>, ProviderError> {
        if !self.allow_file_fallback {
            return Ok(None);
        }
        let Some(path) = &self.file_path else {
            return Ok(None);
        };
        match fs::read_to_string(path) {
            Ok(value) => decode_token_state(&value).map(Some),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn save_fallback(&self, encoded: &str) -> Result<(), ProviderError> {
        let path = self.file_path.as_ref().ok_or_else(|| {
            ProviderError::Unavailable("credential fallback path is unavailable".into())
        })?;
        write_owner_only(path, encoded)
    }
}

fn keyring_entry(profile_id: &str) -> Result<keyring::Entry, keyring::Error> {
    keyring::Entry::new(KEYRING_SERVICE, profile_id)
}

fn default_fallback_path(profile_id: &str) -> Option<PathBuf> {
    if let Some(value) = env::var_os("FORGE_PROVIDER_TOKEN_STORE") {
        return Some(PathBuf::from(value));
    }
    credential_directory().map(|path| path.join(format!("{}.json", safe_profile_id(profile_id))))
}

fn auth_preference_path(profile_id: &str) -> Option<PathBuf> {
    credential_directory()
        .map(|path| path.join(format!("{}.auth-profile.json", safe_profile_id(profile_id))))
}

fn credential_directory() -> Option<PathBuf> {
    dirs_next::config_dir().map(|path| path.join("Game Sprite Forge/provider-auth"))
}

fn safe_profile_id(profile_id: &str) -> String {
    if !profile_id.is_empty()
        && profile_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        profile_id.to_string()
    } else {
        format!("profile-{:x}", Sha256::digest(profile_id.as_bytes()))
    }
}

fn write_owner_only(path: &Path, encoded: &str) -> Result<(), ProviderError> {
    let parent = path.parent().ok_or_else(|| {
        ProviderError::Unavailable("credential metadata path has no parent".into())
    })?;
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(encoded.as_bytes())?;
    temporary.as_file().sync_all()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temporary
            .as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    temporary
        .persist(path)
        .map_err(|error| ProviderError::Io(error.error))?;
    Ok(())
}

fn load_auth_preference(profile_id: &str) -> Result<Option<XaiAuthPreference>, ProviderError> {
    let Some(path) = auth_preference_path(profile_id) else {
        return Ok(None);
    };
    load_auth_preference_from_path(&path)
}

fn load_auth_preference_from_path(path: &Path) -> Result<Option<XaiAuthPreference>, ProviderError> {
    match fs::read_to_string(path) {
        Ok(value) => {
            let preference: XaiAuthPreference = serde_json::from_str(&value).map_err(|error| {
                ProviderError::InvalidOutput(format!(
                    "invalid xAI auth profile {}: {error}",
                    path.display()
                ))
            })?;
            if preference.schema_version != AUTH_PREFERENCE_SCHEMA_VERSION {
                return Err(ProviderError::InvalidOutput(format!(
                    "unsupported xAI auth profile schema {}",
                    preference.schema_version
                )));
            }
            if preference.method == XaiAuthMethod::ApiKey
                && preference.storage != CredentialStorageKind::Keychain
            {
                return Err(ProviderError::InvalidOutput(
                    "xAI API key auth profile selected unsupported file storage".into(),
                ));
            }
            Ok(Some(preference))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn save_xai_auth_preference(
    profile_id: &str,
    method: XaiAuthMethod,
    storage: CredentialStorageKind,
) -> Result<(), ProviderError> {
    if method == XaiAuthMethod::ApiKey && storage != CredentialStorageKind::Keychain {
        return Err(ProviderError::InvalidOutput(
            "xAI API key auth profiles require Keychain storage".into(),
        ));
    }
    let path = auth_preference_path(profile_id).ok_or_else(|| {
        ProviderError::Unavailable("xAI auth profile location is unavailable".into())
    })?;
    let preference = XaiAuthPreference {
        schema_version: AUTH_PREFERENCE_SCHEMA_VERSION.into(),
        method,
        storage,
    };
    save_auth_preference_to_path(&path, &preference)
}

fn save_auth_preference_to_path(
    path: &Path,
    preference: &XaiAuthPreference,
) -> Result<(), ProviderError> {
    write_owner_only(
        path,
        &serde_json::to_string_pretty(preference)
            .map_err(|error| ProviderError::InvalidOutput(error.to_string()))?,
    )
}

fn delete_xai_auth_preference(profile_id: &str) -> Result<(), ProviderError> {
    let Some(path) = auth_preference_path(profile_id) else {
        return Ok(());
    };
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn env_flag(name: &str) -> bool {
    env::var(name)
        .ok()
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OAuthTokenState {
    access_token: String,
    refresh_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id_token: Option<String>,
    expires_at: i64,
    token_endpoint: String,
    issuer: String,
}

fn decode_token_state(value: &str) -> Result<OAuthTokenState, ProviderError> {
    serde_json::from_str(value)
        .map_err(|error| ProviderError::InvalidOutput(format!("invalid credential state: {error}")))
}

pub struct XaiCredentials {
    kind: XaiCredentialKind,
}

enum XaiCredentialKind {
    ApiKey(String),
    OAuth {
        store: CredentialStore,
        state: Mutex<OAuthTokenState>,
    },
}

impl XaiCredentials {
    fn oauth(store: CredentialStore, state: OAuthTokenState) -> Self {
        Self {
            kind: XaiCredentialKind::OAuth {
                store,
                state: Mutex::new(state),
            },
        }
    }

    fn api_key(value: String) -> Self {
        Self {
            kind: XaiCredentialKind::ApiKey(value),
        }
    }
}

impl CredentialProvider for XaiCredentials {
    fn kind(&self) -> CredentialKind {
        match self.kind {
            XaiCredentialKind::ApiKey(_) => CredentialKind::ApiKey,
            XaiCredentialKind::OAuth { .. } => CredentialKind::OAuthDeviceCode,
        }
    }

    fn bearer(&self) -> Result<String, ProviderError> {
        match &self.kind {
            XaiCredentialKind::ApiKey(value) => Ok(value.clone()),
            XaiCredentialKind::OAuth { store, state } => {
                let mut state = state
                    .lock()
                    .map_err(|_| ProviderError::Request("credential lock was poisoned".into()))?;
                if state.expires_at <= now_seconds() + REFRESH_SKEW_SECONDS {
                    let refreshed = refresh_oauth_token(&state)?;
                    store.save(&refreshed)?;
                    *state = refreshed;
                }
                Ok(state.access_token.clone())
            }
        }
    }

    fn refresh(&self) -> Result<String, ProviderError> {
        match &self.kind {
            XaiCredentialKind::ApiKey(value) => Ok(value.clone()),
            XaiCredentialKind::OAuth { store, state } => {
                let mut state = state
                    .lock()
                    .map_err(|_| ProviderError::Request("credential lock was poisoned".into()))?;
                let refreshed = refresh_oauth_token(&state)?;
                let bearer = refreshed.access_token.clone();
                store.save(&refreshed)?;
                *state = refreshed;
                Ok(bearer)
            }
        }
    }

    fn logout(&self) -> Result<(), ProviderError> {
        if let XaiCredentialKind::OAuth { store, .. } = &self.kind {
            store.delete()?;
        }
        Ok(())
    }
}

pub fn resolve_xai_credentials(
    store: CredentialStore,
) -> Result<Arc<dyn CredentialProvider>, ProviderError> {
    if let Ok(value) = env::var("XAI_API_KEY") {
        if !value.trim().is_empty() {
            return Ok(Arc::new(XaiCredentials::api_key(value)));
        }
    }
    if let Some(value) = load_xai_api_key(&store.profile_id)? {
        return Ok(Arc::new(XaiCredentials::api_key(value)));
    }
    let state = store.load()?.ok_or_else(|| {
        ProviderError::AuthenticationRequired(
            "run `forge provider login --provider xai --method api-key`, use Preview OAuth, or set XAI_API_KEY"
                .into(),
        )
    })?;
    Ok(Arc::new(XaiCredentials::oauth(store, state)))
}

pub fn resolve_xai_credentials_for_profile(
    profile_id: &str,
) -> Result<Arc<dyn CredentialProvider>, ProviderError> {
    if let Ok(value) = env::var("XAI_API_KEY") {
        if !value.trim().is_empty() {
            return Ok(Arc::new(XaiCredentials::api_key(value)));
        }
    }
    match load_auth_preference(profile_id)? {
        Some(XaiAuthPreference {
            method: XaiAuthMethod::ApiKey,
            ..
        }) => {
            let value = load_xai_api_key(profile_id)?.ok_or_else(|| {
                ProviderError::AuthenticationRequired(format!(
                    "xAI profile {profile_id} selects api-key but its Keychain item is missing; rerun provider login"
                ))
            })?;
            Ok(Arc::new(XaiCredentials::api_key(value)))
        }
        Some(XaiAuthPreference {
            method: XaiAuthMethod::OAuthDeviceCode,
            storage,
            ..
        }) => resolve_oauth_credentials(
            CredentialStore::system(profile_id)
                .with_file_storage(storage == CredentialStorageKind::File),
        ),
        None => {
            let store = CredentialStore::system(profile_id);
            if store.configured_storage() == CredentialStorageKind::Keychain {
                if let Some(value) = load_xai_api_key(profile_id)? {
                    save_xai_auth_preference(
                        profile_id,
                        XaiAuthMethod::ApiKey,
                        CredentialStorageKind::Keychain,
                    )?;
                    return Ok(Arc::new(XaiCredentials::api_key(value)));
                }
            }
            let storage = store.configured_storage();
            let credentials = resolve_oauth_credentials(store)?;
            save_xai_auth_preference(profile_id, XaiAuthMethod::OAuthDeviceCode, storage)?;
            Ok(credentials)
        }
    }
}

fn resolve_oauth_credentials(
    store: CredentialStore,
) -> Result<Arc<dyn CredentialProvider>, ProviderError> {
    let state = store.load()?.ok_or_else(|| {
        ProviderError::AuthenticationRequired(
            "run `forge provider login --provider xai --method oauth`".into(),
        )
    })?;
    Ok(Arc::new(XaiCredentials::oauth(store, state)))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceAuthorization {
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub user_code: String,
    pub expires_in_seconds: u64,
}

pub fn login_xai_device_code(
    store: CredentialStore,
    announce: impl FnOnce(&DeviceAuthorization),
    cancelled: impl Fn() -> bool,
) -> Result<CredentialStorageKind, ProviderError> {
    let client = oauth_client()?;
    let discovery = discover(&client)?;
    let device = request_device_code(&client, &discovery.device_authorization_endpoint)?;
    let authorization = DeviceAuthorization {
        verification_uri: device.verification_uri.clone(),
        verification_uri_complete: device.verification_uri_complete.clone(),
        user_code: device.user_code.clone(),
        expires_in_seconds: device.expires_in,
    };
    announce(&authorization);
    let state = poll_device_token(&client, &discovery.token_endpoint, &device, cancelled)?;
    store.save(&state)
}

pub fn logout_xai_profile(profile_id: &str) -> Result<(), ProviderError> {
    let preference = load_auth_preference(profile_id)?;
    let file_selected = matches!(
        preference,
        Some(XaiAuthPreference {
            storage: CredentialStorageKind::File,
            ..
        })
    );
    if file_selected {
        CredentialStore::system(profile_id)
            .with_file_storage(true)
            .delete()?;
    }
    CredentialStore::system(profile_id).delete()?;
    delete_xai_api_key(profile_id)?;
    delete_xai_auth_preference(profile_id)
}

pub fn save_xai_api_key(profile_id: &str, value: &str) -> Result<(), ProviderError> {
    let value = value.trim();
    if value.is_empty() || value.chars().any(char::is_control) {
        return Err(ProviderError::AuthenticationRequired(
            "xAI API key must be non-empty printable text".into(),
        ));
    }
    api_key_entry(profile_id)
        .and_then(|entry| entry.set_password(value))
        .map_err(|error| {
            ProviderError::Unavailable(format!("system credential store failed: {error}"))
        })
}

fn load_xai_api_key(profile_id: &str) -> Result<Option<String>, ProviderError> {
    match api_key_entry(profile_id).and_then(|entry| entry.get_password()) {
        Ok(value) if !value.trim().is_empty() => Ok(Some(value)),
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(ProviderError::Unavailable(format!(
            "system credential store failed: {error}"
        ))),
    }
}

fn delete_xai_api_key(profile_id: &str) -> Result<(), ProviderError> {
    match api_key_entry(profile_id).and_then(|entry| entry.delete_credential()) {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(ProviderError::Request(format!(
            "system credential delete failed: {error}"
        ))),
    }
}

fn api_key_entry(profile_id: &str) -> Result<keyring::Entry, keyring::Error> {
    keyring::Entry::new(API_KEY_KEYRING_SERVICE, profile_id)
}

#[derive(Deserialize)]
struct DiscoveryDocument {
    issuer: String,
    device_authorization_endpoint: String,
    token_endpoint: String,
}

#[derive(Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    #[serde(default)]
    verification_uri_complete: Option<String>,
    expires_in: u64,
    #[serde(default = "default_poll_interval")]
    interval: u64,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
    expires_in: Option<i64>,
}

fn default_poll_interval() -> u64 {
    5
}

fn oauth_client() -> Result<Client, ProviderError> {
    Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("Game-Sprite-Forge/0.1")
        .build()
        .map_err(|error| ProviderError::Request(error.to_string()))
}

fn discover(client: &Client) -> Result<DiscoveryDocument, ProviderError> {
    let response = client
        .get(XAI_OAUTH_DISCOVERY_URL)
        .header("Accept", "application/json")
        .send()
        .map_err(|error| ProviderError::Request(format!("OAuth discovery failed: {error}")))?;
    let document: DiscoveryDocument = parse_json_response(response, "OAuth discovery")?;
    if document.issuer.trim_end_matches('/') != XAI_OAUTH_ISSUER {
        return Err(ProviderError::InvalidOutput(
            "xAI discovery returned an unexpected issuer".into(),
        ));
    }
    require_trusted_xai_url(&document.device_authorization_endpoint)?;
    require_trusted_xai_url(&document.token_endpoint)?;
    Ok(document)
}

fn request_device_code(
    client: &Client,
    endpoint: &str,
) -> Result<DeviceCodeResponse, ProviderError> {
    require_trusted_xai_url(endpoint)?;
    let response = client
        .post(endpoint)
        .form(&[
            ("client_id", oauth_client_id()),
            ("scope", XAI_OAUTH_SCOPE.to_string()),
        ])
        .send()
        .map_err(|error| ProviderError::Request(format!("device code request failed: {error}")))?;
    let device: DeviceCodeResponse = parse_json_response(response, "device code request")?;
    require_trusted_xai_url(&device.verification_uri)?;
    if let Some(url) = &device.verification_uri_complete {
        require_trusted_xai_url(url)?;
    }
    if device.device_code.is_empty() || device.user_code.is_empty() {
        return Err(ProviderError::InvalidOutput(
            "device code response was incomplete".into(),
        ));
    }
    Ok(device)
}

fn poll_device_token(
    client: &Client,
    endpoint: &str,
    device: &DeviceCodeResponse,
    cancelled: impl Fn() -> bool,
) -> Result<OAuthTokenState, ProviderError> {
    poll_device_token_with(client, endpoint, device, cancelled, true, thread::sleep)
}

fn poll_device_token_with(
    client: &Client,
    endpoint: &str,
    device: &DeviceCodeResponse,
    cancelled: impl Fn() -> bool,
    require_trusted_endpoint: bool,
    wait: impl Fn(Duration),
) -> Result<OAuthTokenState, ProviderError> {
    if require_trusted_endpoint {
        require_trusted_xai_url(endpoint)?;
    }
    let deadline = now_seconds() + device.expires_in as i64;
    let mut interval = device.interval.clamp(1, 30);
    while now_seconds() < deadline {
        if cancelled() {
            return Err(ProviderError::Cancelled);
        }
        let response = client
            .post(endpoint)
            .form(&[
                ("grant_type", XAI_DEVICE_GRANT.to_string()),
                ("client_id", oauth_client_id()),
                ("device_code", device.device_code.clone()),
            ])
            .send()
            .map_err(|error| {
                ProviderError::Request(format!("device token poll failed: {error}"))
            })?;
        let status = response.status();
        let body = limited_body(response)?;
        if status.is_success() {
            let token: TokenResponse = serde_json::from_slice(&body).map_err(|error| {
                ProviderError::InvalidOutput(format!("invalid token response: {error}"))
            })?;
            let refresh_token = token.refresh_token.ok_or_else(|| {
                ProviderError::InvalidOutput("OAuth response omitted refresh_token".into())
            })?;
            return Ok(OAuthTokenState {
                access_token: token.access_token,
                refresh_token,
                id_token: token.id_token,
                expires_at: now_seconds() + token.expires_in.unwrap_or(6 * 60 * 60),
                token_endpoint: endpoint.to_string(),
                issuer: XAI_OAUTH_ISSUER.into(),
            });
        }
        let error = serde_json::from_slice::<Value>(&body)
            .ok()
            .and_then(|value| {
                value
                    .get("error")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_else(|| format!("http_{}", status.as_u16()));
        match error.as_str() {
            "authorization_pending" => {}
            "slow_down" => interval = (interval + 5).min(30),
            "access_denied" => {
                return Err(ProviderError::AuthenticationRequired(
                    "xAI authorization was denied".into(),
                ))
            }
            "expired_token" => {
                return Err(ProviderError::AuthenticationRequired(
                    "xAI device code expired".into(),
                ))
            }
            _ if status.as_u16() == 403 => {
                return Err(ProviderError::Entitlement(
                    "this Grok subscription is not entitled to OAuth API access".into(),
                ))
            }
            _ => {
                return Err(ProviderError::Request(format!(
                    "device token poll failed: {error}"
                )))
            }
        }
        wait(Duration::from_secs(interval));
    }
    Err(ProviderError::AuthenticationRequired(
        "xAI device authorization timed out".into(),
    ))
}

fn refresh_oauth_token(state: &OAuthTokenState) -> Result<OAuthTokenState, ProviderError> {
    refresh_oauth_token_with(state, true)
}

fn refresh_oauth_token_with(
    state: &OAuthTokenState,
    require_trusted_endpoint: bool,
) -> Result<OAuthTokenState, ProviderError> {
    if require_trusted_endpoint {
        require_trusted_xai_url(&state.token_endpoint)?;
    }
    let client = oauth_client()?;
    let response = client
        .post(&state.token_endpoint)
        .form(&[
            ("grant_type", "refresh_token".to_string()),
            ("client_id", oauth_client_id()),
            ("refresh_token", state.refresh_token.clone()),
        ])
        .send()
        .map_err(|error| {
            ProviderError::Request(format!(
                "OAuth refresh transport failed; the rotating token was not retried: {error}"
            ))
        })?;
    let status = response.status();
    if status.as_u16() == 403 {
        return Err(ProviderError::Entitlement(
            "this Grok subscription is not entitled to OAuth API access".into(),
        ));
    }
    let token: TokenResponse = parse_json_response(response, "OAuth refresh")?;
    Ok(OAuthTokenState {
        access_token: token.access_token,
        refresh_token: token
            .refresh_token
            .unwrap_or_else(|| state.refresh_token.clone()),
        id_token: token.id_token.or_else(|| state.id_token.clone()),
        expires_at: now_seconds() + token.expires_in.unwrap_or(6 * 60 * 60),
        token_endpoint: state.token_endpoint.clone(),
        issuer: state.issuer.clone(),
    })
}

fn parse_json_response<T: for<'de> Deserialize<'de>>(
    response: Response,
    context: &str,
) -> Result<T, ProviderError> {
    let status = response.status();
    let body = limited_body(response)?;
    if !status.is_success() {
        if status.as_u16() == 403 {
            return Err(ProviderError::Entitlement(format!(
                "{context} was rejected by the subscription entitlement"
            )));
        }
        return Err(ProviderError::Request(format!(
            "{context} failed with HTTP {}",
            status.as_u16()
        )));
    }
    serde_json::from_slice(&body)
        .map_err(|error| ProviderError::InvalidOutput(format!("{context}: {error}")))
}

fn limited_body(response: Response) -> Result<Vec<u8>, ProviderError> {
    let bytes = response
        .bytes()
        .map_err(|error| ProviderError::Request(error.to_string()))?;
    if bytes.len() > RESPONSE_LIMIT_BYTES {
        return Err(ProviderError::InvalidOutput(format!(
            "response exceeded {RESPONSE_LIMIT_BYTES} bytes"
        )));
    }
    Ok(bytes.to_vec())
}

fn require_trusted_xai_url(value: &str) -> Result<(), ProviderError> {
    let url = Url::parse(value)
        .map_err(|_| ProviderError::InvalidOutput("xAI returned an invalid endpoint".into()))?;
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    if url.scheme() != "https" || (host != "x.ai" && !host.ends_with(".x.ai")) {
        return Err(ProviderError::InvalidOutput(
            "xAI returned an untrusted OAuth endpoint".into(),
        ));
    }
    Ok(())
}

fn oauth_client_id() -> String {
    env::var("FORGE_XAI_OAUTH_CLIENT_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| XAI_OAUTH_CLIENT_ID.into())
}

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread::JoinHandle;

    #[test]
    fn rejects_untrusted_oauth_endpoints() {
        assert!(require_trusted_xai_url("https://auth.x.ai/oauth2/token").is_ok());
        assert!(require_trusted_xai_url("http://auth.x.ai/oauth2/token").is_err());
        assert!(require_trusted_xai_url("https://x.ai.attacker.example/token").is_err());
    }

    #[test]
    fn owner_only_file_store_round_trips_without_exposing_tokens() {
        let temp = tempfile::tempdir().unwrap();
        let store = CredentialStore::file("test", temp.path().join("auth.json"));
        let state = OAuthTokenState {
            access_token: "access-secret".into(),
            refresh_token: "refresh-secret".into(),
            id_token: None,
            expires_at: now_seconds() + 7200,
            token_endpoint: "https://auth.x.ai/oauth2/token".into(),
            issuer: XAI_OAUTH_ISSUER.into(),
        };
        assert_eq!(store.save(&state).unwrap(), CredentialStorageKind::File);
        let loaded = store.load().unwrap().unwrap();
        assert_eq!(loaded.access_token, "access-secret");
        assert!(
            !format!("{}", ProviderError::AuthenticationRequired("login".into()))
                .contains("access-secret")
        );
    }

    #[test]
    fn auth_preference_is_non_secret_owner_only_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("default.auth-profile.json");
        let preference = XaiAuthPreference {
            schema_version: AUTH_PREFERENCE_SCHEMA_VERSION.into(),
            method: XaiAuthMethod::OAuthDeviceCode,
            storage: CredentialStorageKind::File,
        };
        save_auth_preference_to_path(&path, &preference).unwrap();
        assert_eq!(
            load_auth_preference_from_path(&path).unwrap(),
            Some(preference)
        );
        let encoded = fs::read_to_string(&path).unwrap();
        assert!(!encoded.contains("token"));
        assert!(!encoded.contains("authorization"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    struct FakeResponse {
        status: u16,
        body: Vec<u8>,
    }

    fn fake_oauth_server<F>(expected_requests: usize, handler: F) -> (String, JoinHandle<()>)
    where
        F: Fn(usize, &str) -> FakeResponse + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/token", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            for index in 0..expected_requests {
                let (mut stream, _) = listener.accept().unwrap();
                let mut bytes = [0_u8; 16 * 1024];
                let count = std::io::Read::read(&mut stream, &mut bytes).unwrap_or(0);
                let request = String::from_utf8_lossy(&bytes[..count]);
                let response = handler(index, &request);
                let reason = if response.status == 200 {
                    "OK"
                } else {
                    "Bad Request"
                };
                write!(
                    stream,
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    response.status,
                    reason,
                    response.body.len()
                )
                .unwrap();
                std::io::Write::write_all(&mut stream, &response.body).unwrap();
            }
        });
        (endpoint, handle)
    }

    fn device(expires_in: u64) -> DeviceCodeResponse {
        DeviceCodeResponse {
            device_code: "device-secret".into(),
            user_code: "ABCD-EFGH".into(),
            verification_uri: "https://auth.x.ai/device".into(),
            verification_uri_complete: None,
            expires_in,
            interval: 1,
        }
    }

    #[test]
    fn fake_oidc_handles_pending_slow_down_and_refresh_rotation() {
        let (endpoint, server) = fake_oauth_server(3, |index, request| {
            assert!(!request.contains("access-secret"));
            match index {
                0 => FakeResponse {
                    status: 400,
                    body: br#"{"error":"authorization_pending"}"#.to_vec(),
                },
                1 => FakeResponse {
                    status: 400,
                    body: br#"{"error":"slow_down"}"#.to_vec(),
                },
                _ => FakeResponse {
                    status: 200,
                    body: br#"{"access_token":"access-secret","refresh_token":"refresh-one","expires_in":7200}"#.to_vec(),
                },
            }
        });
        let state = poll_device_token_with(
            &oauth_client().unwrap(),
            &endpoint,
            &device(120),
            || false,
            false,
            |_| {},
        )
        .unwrap();
        assert_eq!(state.refresh_token, "refresh-one");
        server.join().unwrap();

        let (refresh_endpoint, server) = fake_oauth_server(1, |_, request| {
            assert!(request.contains("refresh_token=refresh-one"));
            FakeResponse {
                status: 200,
                body: br#"{"access_token":"access-two","refresh_token":"refresh-two","expires_in":7200}"#.to_vec(),
            }
        });
        let rotated = refresh_oauth_token_with(
            &OAuthTokenState {
                token_endpoint: refresh_endpoint,
                ..state
            },
            false,
        )
        .unwrap();
        assert_eq!(rotated.refresh_token, "refresh-two");
        assert_eq!(rotated.access_token, "access-two");
        server.join().unwrap();
    }

    #[test]
    fn fake_oidc_reports_denial_expiry_and_cancellation_without_token_leaks() {
        let (endpoint, server) = fake_oauth_server(1, |_, _| FakeResponse {
            status: 400,
            body: br#"{"error":"access_denied","access_token":"must-not-leak"}"#.to_vec(),
        });
        let error = match poll_device_token_with(
            &oauth_client().unwrap(),
            &endpoint,
            &device(120),
            || false,
            false,
            |_| {},
        ) {
            Ok(_) => panic!("denied authorization unexpectedly succeeded"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("denied"));
        assert!(!error.to_string().contains("must-not-leak"));
        server.join().unwrap();

        let error = match poll_device_token_with(
            &oauth_client().unwrap(),
            "http://127.0.0.1:9/token",
            &device(0),
            || false,
            false,
            |_| {},
        ) {
            Ok(_) => panic!("expired authorization unexpectedly succeeded"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("timed out"));

        let error = match poll_device_token_with(
            &oauth_client().unwrap(),
            "http://127.0.0.1:9/token",
            &device(120),
            || true,
            false,
            |_| {},
        ) {
            Ok(_) => panic!("cancelled authorization unexpectedly succeeded"),
            Err(error) => error,
        };
        assert!(matches!(error, ProviderError::Cancelled));
    }
}
