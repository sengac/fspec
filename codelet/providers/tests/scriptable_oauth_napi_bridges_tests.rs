#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::await_holding_lock)]
//! Feature: spec/features/scriptable-oauth-napi-bridges-browser-loopback-pkce-flow.feature
//!
//! Integration tests for PROV-087.
//!
//! The NAPI bindings themselves (`custom_oauth_authorize` etc.) are thin
//! wrappers over pure-Rust helpers that live in `codelet_providers`; this
//! file exercises those helpers directly. Specifically it covers:
//!
//! * the new `auth_start` / `auth_exchange` / `auth_needs_refresh` /
//!   `auth_refresh` function names on `ScriptedOAuthProvider`, with
//!   graceful fallback to the PROV-060 legacy names
//!   (`build_authorization_request` / `exchange_code` / `needs_refresh` /
//!   `refresh_token`);
//! * the CredentialStore round-trip performed by
//!   `custom_oauth_exchange`, `custom_oauth_refresh`, and
//!   `custom_oauth_clear` for provider-scoped tokens stored as JSON maps;
//! * the dispatcher rule that built-in provider names with no shadow
//!   config keep resolving to the existing `claude_oauth` /
//!   `codex_oauth` / `copilot_oauth` NAPI bindings.
//!
//! These tests are RED until PROV-087 lands — they reference functions
//! that do not yet exist (`auth_start_or_legacy`, `auth_exchange_or_legacy`,
//! `auth_needs_refresh_or_legacy`, `auth_refresh_or_legacy`,
//! `custom_oauth_store_path`, `resolve_login_implementation`).

use std::sync::{Mutex, MutexGuard, OnceLock};

use codelet_providers::oauth::script_provider::{
    ScriptProviderConfig, ScriptedOAuthProvider,
};
use codelet_providers::oauth::script_provider_aliases::{
    auth_exchange_or_legacy, auth_needs_refresh_or_legacy,
    auth_refresh_or_legacy, auth_start_or_legacy,
};
use codelet_providers::oauth::custom_oauth::{
    custom_oauth_store_path, resolve_login_implementation, LoginImplementation,
};
use rhai::Map;

/// Serialise tests that mutate `FSPEC_HOME`.
fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

struct FspecHomeGuard {
    original: Option<String>,
}

impl Drop for FspecHomeGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(v) => std::env::set_var("FSPEC_HOME", v),
            None => std::env::remove_var("FSPEC_HOME"),
        }
    }
}

fn with_temp_fspec_home() -> (tempfile::TempDir, FspecHomeGuard) {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let guard = FspecHomeGuard {
        original: std::env::var("FSPEC_HOME").ok(),
    };
    std::env::set_var("FSPEC_HOME", dir.path());
    (dir, guard)
}

fn config(name: &str) -> ScriptProviderConfig {
    ScriptProviderConfig {
        name: name.to_string(),
        display_name: format!("{name} Display"),
        script: format!("{name}.rhai"),
        auth_url: Some("https://auth.example.com/authorize".to_string()),
        token_url: Some("https://auth.example.com/token".to_string()),
        client_id: Some("client-id".to_string()),
        redirect_uri: Some("http://127.0.0.1:0/callback".to_string()),
        scopes: Some("read write".to_string()),
        flow: Some("authorization_code".to_string()),
        credential_file: Some(format!("{name}.json")),
    }
}

const NEW_STYLE_SCRIPT: &str = r#"
fn auth_start(config) {
    #{
        url: config.auth_url + "?client_id=" + config.client_id,
        pkce_verifier: "new-verifier",
        state: "new-state",
    }
}
fn auth_exchange(config, code, verifier) {
    #{
        access_token: "new_at_" + code,
        refresh_token: "new_rt_" + code,
        expires_in: 3600,
    }
}
fn auth_needs_refresh(tokens) { true }
fn auth_refresh(config, tokens) {
    #{
        access_token: "refreshed_" + tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_in: 3600,
    }
}
"#;

const LEGACY_SCRIPT: &str = r#"
fn build_authorization_request(config) {
    #{
        url: config.auth_url + "?legacy=1",
        pkce_verifier: "legacy-verifier",
        state: "legacy-state",
    }
}
fn exchange_code(config, code, verifier) {
    #{
        access_token: "legacy_at_" + code,
        refresh_token: "legacy_rt_" + code,
        expires_in: 3600,
    }
}
fn needs_refresh(tokens) { false }
fn refresh_token(config, tokens) {
    #{
        access_token: "legacy_refreshed",
        refresh_token: tokens.refresh_token,
        expires_in: 3600,
    }
}
"#;

// =========================================================================
// Scenario: User logs in with custom shadow provider using auth_start and auth_exchange
// =========================================================================
#[tokio::test]
async fn user_logs_in_with_custom_shadow_using_new_names() {
    let _lock = env_lock();
    // @step Given a Rhai script my-custom.rhai defining auth_start and auth_exchange is registered with provider name "my-custom"
    let (_dir, _env) = with_temp_fspec_home();
    let provider =
        ScriptedOAuthProvider::from_script(NEW_STYLE_SCRIPT, config("my-custom"))
            .expect("load script");

    // @step When I invoke custom_oauth_authorize("my-custom") and then custom_oauth_exchange("my-custom", code, verifier) with the values returned from the loopback callback
    let start: Map = auth_start_or_legacy(&provider).await.expect("auth_start");
    let verifier = start
        .get("pkce_verifier")
        .expect("verifier")
        .clone()
        .into_string()
        .expect("string");
    let url = start
        .get("url")
        .expect("url")
        .clone()
        .into_string()
        .expect("string");
    let tokens: Map =
        auth_exchange_or_legacy(&provider, "AUTHCODE", &verifier)
            .await
            .expect("exchange");

    // @step Then the script's auth_start is called to produce the authorization URL and pkce_verifier
    assert!(url.contains("client_id="), "url must contain client_id");
    assert_eq!(verifier, "new-verifier");

    // @step Then the script's auth_exchange is called with the returned code and verifier and produces tokens
    let at = tokens
        .get("access_token")
        .expect("access_token")
        .clone()
        .into_string()
        .expect("string");
    assert_eq!(at, "new_at_AUTHCODE");

    // @step Then the resulting tokens are persisted in CredentialStore under provider_name "my-custom"
    let path = custom_oauth_store_path("my-custom");
    let json = serde_json::to_string(
        &codelet_providers::oauth::json_convert::dynamic_to_json_value(
            &rhai::Dynamic::from_map(tokens),
        ),
    )
    .unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, &json).unwrap();
    assert!(path.exists(), "store path must be writable");
    let read = std::fs::read_to_string(&path).unwrap();
    assert!(read.contains("new_at_AUTHCODE"));
}

// =========================================================================
// Scenario: Legacy script using deprecated function aliases still authenticates successfully
// =========================================================================
#[tokio::test]
async fn legacy_script_using_deprecated_aliases_still_authenticates() {
    let _lock = env_lock();
    let (_dir, _env) = with_temp_fspec_home();

    // @step Given a Rhai script registered as "legacy-custom" defines only build_authorization_request and exchange_code (the PROV-060 names)
    let provider = ScriptedOAuthProvider::from_script(LEGACY_SCRIPT, config("legacy-custom"))
        .expect("load legacy script");

    // @step When custom_oauth_authorize("legacy-custom") and custom_oauth_exchange("legacy-custom", code, verifier) are invoked
    let start: Map = auth_start_or_legacy(&provider).await.expect("start");
    let verifier = start
        .get("pkce_verifier")
        .expect("verifier")
        .clone()
        .into_string()
        .expect("string");
    let tokens: Map =
        auth_exchange_or_legacy(&provider, "CODE42", &verifier)
            .await
            .expect("exchange");

    // @step Then the NAPI layer falls back to build_authorization_request when auth_start is not defined
    assert_eq!(verifier, "legacy-verifier");

    // @step Then the NAPI layer falls back to exchange_code when auth_exchange is not defined
    let at = tokens
        .get("access_token")
        .expect("access_token")
        .clone()
        .into_string()
        .expect("string");
    assert_eq!(at, "legacy_at_CODE42");

    // @step Then the login completes and tokens are stored under provider_name "legacy-custom"
    let path = custom_oauth_store_path("legacy-custom");
    assert!(
        path.to_string_lossy().contains("legacy-custom"),
        "path must be scoped to provider"
    );
}

// =========================================================================
// Scenario: Expired tokens are refreshed silently via auth_needs_refresh and auth_refresh
// =========================================================================
#[tokio::test]
async fn expired_tokens_are_refreshed_silently() {
    let _lock = env_lock();
    let (_dir, _env) = with_temp_fspec_home();

    // @step Given tokens for provider "my-custom" exist in CredentialStore but are expired
    let provider = ScriptedOAuthProvider::from_script(NEW_STYLE_SCRIPT, config("my-custom"))
        .expect("load");
    let mut existing = Map::new();
    existing.insert("access_token".into(), rhai::Dynamic::from("old_at".to_string()));
    existing.insert("refresh_token".into(), rhai::Dynamic::from("rt".to_string()));
    existing.insert("expires_at".into(), rhai::Dynamic::from(0_i64));

    // @step When custom_oauth_needs_refresh("my-custom") is called
    let needs = auth_needs_refresh_or_legacy(&provider, existing.clone())
        .await
        .expect("needs_refresh");

    // @step Then the script's auth_needs_refresh is invoked and returns true
    assert!(needs, "expired tokens must need refresh");

    // @step When custom_oauth_refresh("my-custom") is subsequently called
    let refreshed: Map = auth_refresh_or_legacy(&provider, existing)
        .await
        .expect("refresh");

    // @step Then the script's auth_refresh is invoked with the current tokens and returns new tokens
    let new_at = refreshed
        .get("access_token")
        .expect("access_token")
        .clone()
        .into_string()
        .expect("string");
    assert_eq!(new_at, "refreshed_old_at");

    // @step Then the refreshed tokens replace the stored credentials in CredentialStore under "my-custom"
    let path = custom_oauth_store_path("my-custom");
    assert!(
        path.to_string_lossy().ends_with("my-custom.json"),
        "credential path must end with my-custom.json"
    );
}

// =========================================================================
// Scenario: Built-in providers are used unchanged when no shadow script is registered
// =========================================================================
#[test]
fn builtin_providers_used_unchanged_when_no_shadow() {
    let _lock = env_lock();
    let (_dir, _env) = with_temp_fspec_home();

    // @step Given no Rhai shadow config exists for the provider name "claude"
    // (fresh FSPEC_HOME has no providers/ directory)

    // @step When the dispatcher resolves the OAuth implementation for "claude"
    let impl_ = resolve_login_implementation("claude");

    // @step Then it selects the built-in claude_oauth NAPI binding and not custom_oauth_authorize
    assert_eq!(impl_, LoginImplementation::BuiltIn("claude".to_string()));
}

// =========================================================================
// Scenario: custom_oauth_clear removes stored tokens for a provider
// =========================================================================
#[test]
fn custom_oauth_clear_removes_stored_tokens() {
    let _lock = env_lock();
    let (_dir, _env) = with_temp_fspec_home();

    // @step Given tokens for provider "my-custom" are present in CredentialStore
    let path = custom_oauth_store_path("my-custom");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, r#"{"access_token":"abc"}"#).unwrap();
    assert!(path.exists());

    // @step When custom_oauth_clear("my-custom") is called
    codelet_providers::oauth::custom_oauth::custom_oauth_clear_sync("my-custom")
        .expect("clear");

    // @step Then the stored tokens for "my-custom" are removed from CredentialStore
    assert!(!path.exists(), "tokens file must be removed by clear");
}
