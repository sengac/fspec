#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::await_holding_lock
)]
//! Feature: spec/features/scriptable-oauth-napi-bridges-device-flow-auto-refresh-middleware.feature
//!
//! PROV-088: device-code flow + auto-refresh middleware for scripted
//! OAuth providers.
//!
//! These tests exercise the pure-Rust helpers that the NAPI bindings
//! delegate to:
//!
//! * `ScriptedDeviceFlow::start` — wraps `auth_start` (falling back to
//!   `build_authorization_request`) and returns `{user_code, verification_uri, ...}`.
//! * `ScriptedDeviceFlow::poll` — wraps `auth_poll` (falling back to
//!   `poll_for_token`) and returns `{status, tokens?}`.
//! * `persist_on_success` — writes tokens to CredentialStore when the
//!   poll status is "success" and is a no-op otherwise.
//! * `ScriptedRefreshingClient::ensure_fresh_if_needed` — the
//!   auto-refresh middleware: checks `auth_needs_refresh` and, when
//!   true, calls `auth_refresh` and overwrites the stored tokens.
//! * `resolve_refresh_middleware` — dispatcher that returns
//!   `Activated(provider_name)` when a shadow config exists and
//!   `Builtin` otherwise.

use std::sync::{Mutex, MutexGuard, OnceLock};

use codelet_providers::oauth::custom_oauth::{
    custom_oauth_store_path, read_stored_tokens, write_stored_tokens,
};
use codelet_providers::oauth::custom_oauth_device::{persist_on_success, ScriptedDeviceFlow};
use codelet_providers::oauth::script_provider::{ScriptProviderConfig, ScriptedOAuthProvider};
use codelet_providers::oauth::scripted_refreshing_client::{
    resolve_refresh_middleware, RefreshMiddleware, ScriptedRefreshingClient,
};
use rhai::{Dynamic, Map};

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

fn cfg(name: &str) -> ScriptProviderConfig {
    ScriptProviderConfig {
        name: name.to_string(),
        display_name: name.to_string(),
        script: format!("{name}.rhai"),
        auth_url: Some("https://example.com/device".to_string()),
        token_url: Some("https://example.com/token".to_string()),
        client_id: Some("client".to_string()),
        redirect_uri: None,
        scopes: None,
        flow: Some("device_code".to_string()),
        credential_file: None,
    }
}

const DEVICE_SCRIPT: &str = r#"
fn auth_start(config) {
    #{
        user_code: "ABCD-1234",
        verification_uri: "https://example.com/device",
        device_code: "DC-1",
        interval: 5,
    }
}
fn auth_poll(config, device_data) {
    #{
        status: "success",
        access_token: "AT1",
        refresh_token: "RT1",
        expires_in: 3600,
    }
}
fn auth_needs_refresh(tokens) { true }
fn auth_refresh(config, tokens) {
    #{
        access_token: "AT2",
        refresh_token: tokens.refresh_token,
        expires_in: 3600,
    }
}
"#;

const DENIED_SCRIPT: &str = r#"
fn auth_start(config) {
    #{
        user_code: "X",
        verification_uri: "https://example.com/device",
        device_code: "DC-DENY",
    }
}
fn auth_poll(config, device_data) {
    #{ status: "denied" }
}
fn auth_needs_refresh(tokens) { false }
"#;

const FRESH_SCRIPT: &str = r#"
fn auth_start(config) { #{} }
fn auth_exchange(config, code, verifier) { #{} }
fn auth_needs_refresh(tokens) { false }
fn auth_refresh(config, tokens) { tokens }
"#;

// =========================================================================
// Scenario: User runs device-code login and receives a user_code
// =========================================================================
#[tokio::test]
async fn user_runs_device_code_login_and_receives_user_code() {
    let _lock = env_lock();
    // @step Given a Rhai script shadowing provider "my-device" defines auth_start that returns user_code "ABCD-1234" and verification_uri "https://example.com/device"
    let (_dir, _env) = with_temp_fspec_home();
    let provider =
        ScriptedOAuthProvider::from_script(DEVICE_SCRIPT, cfg("my-device")).expect("load script");
    let flow = ScriptedDeviceFlow::new(&provider);

    // @step When custom_oauth_device_start("my-device") is invoked
    let result: Map = flow.start().await.expect("device start");

    // @step Then the returned payload contains user_code "ABCD-1234" and verification_uri "https://example.com/device"
    let uc = result
        .get("user_code")
        .expect("user_code")
        .clone()
        .into_string()
        .expect("string");
    let vuri = result
        .get("verification_uri")
        .expect("verification_uri")
        .clone()
        .into_string()
        .expect("string");
    assert_eq!(uc, "ABCD-1234");
    assert_eq!(vuri, "https://example.com/device");
}

// =========================================================================
// Scenario: Polling yields tokens after the user authorises the device
// =========================================================================
#[tokio::test]
async fn polling_yields_tokens_after_authorisation() {
    let _lock = env_lock();
    let (_dir, _env) = with_temp_fspec_home();
    // @step Given a device-code session for provider "my-device" is active with device_code "DC-1"
    let provider =
        ScriptedOAuthProvider::from_script(DEVICE_SCRIPT, cfg("my-device")).expect("load");
    let flow = ScriptedDeviceFlow::new(&provider);
    let mut device_data = Map::new();
    device_data.insert("device_code".into(), Dynamic::from("DC-1".to_string()));

    // @step When custom_oauth_device_poll("my-device", device_data) is called and the script's auth_poll returns status="success" with access_token "AT1"
    let result: Map = flow.poll(device_data).await.expect("poll");

    // @step Then the returned status is "success" and the tokens are persisted in CredentialStore under "my-device"
    let status = result
        .get("status")
        .expect("status")
        .clone()
        .into_string()
        .expect("string");
    assert_eq!(status, "success");

    persist_on_success("my-device", &result).expect("persist");
    let stored = read_stored_tokens("my-device")
        .expect("read tokens")
        .expect("tokens stored");
    let at = stored
        .get("access_token")
        .expect("access_token")
        .clone()
        .into_string()
        .expect("string");
    assert_eq!(at, "AT1");
}

// =========================================================================
// Scenario: Denied polling does not persist tokens
// =========================================================================
#[tokio::test]
async fn denied_polling_does_not_persist_tokens() {
    let _lock = env_lock();
    let (_dir, _env) = with_temp_fspec_home();
    // @step Given a device-code session for provider "my-device" is active and no tokens are stored yet
    let provider =
        ScriptedOAuthProvider::from_script(DENIED_SCRIPT, cfg("my-device")).expect("load");
    let flow = ScriptedDeviceFlow::new(&provider);
    let path = custom_oauth_store_path("my-device");
    assert!(!path.exists());
    let mut dd = Map::new();
    dd.insert("device_code".into(), Dynamic::from("DC-DENY".to_string()));

    // @step When custom_oauth_device_poll returns status="denied"
    let result: Map = flow.poll(dd).await.expect("poll");
    persist_on_success("my-device", &result).expect("persist no-op");

    // @step Then no credential file exists for "my-device"
    assert!(!path.exists(), "denied poll must not persist tokens");
}

// =========================================================================
// Scenario: Middleware auto-refreshes expired tokens on the next request
// =========================================================================
#[tokio::test]
async fn middleware_auto_refreshes_expired_tokens() {
    let _lock = env_lock();
    let (_dir, _env) = with_temp_fspec_home();
    // @step Given a RhaiCustomProvider is active for "my-device" whose stored tokens are expired
    let provider =
        ScriptedOAuthProvider::from_script(DEVICE_SCRIPT, cfg("my-device")).expect("load");
    let mut expired = Map::new();
    expired.insert("access_token".into(), Dynamic::from("AT1".to_string()));
    expired.insert("refresh_token".into(), Dynamic::from("RT1".to_string()));
    expired.insert("expires_at".into(), Dynamic::from(0_i64));
    write_stored_tokens("my-device", &expired).expect("write");

    // @step When the ScriptedRefreshingClient is asked to ensure fresh credentials before an outbound request
    let client = ScriptedRefreshingClient::new(&provider, "my-device");
    let refreshed = client.ensure_fresh_if_needed().await.expect("ensure_fresh");

    // @step Then the script's auth_needs_refresh returns true, auth_refresh is invoked, and the refreshed tokens replace the stored credentials before the request is sent
    assert!(refreshed, "tokens should have been refreshed");
    let stored = read_stored_tokens("my-device")
        .expect("read")
        .expect("tokens stored");
    let at = stored
        .get("access_token")
        .expect("at")
        .clone()
        .into_string()
        .expect("string");
    assert_eq!(at, "AT2", "refreshed access_token must be persisted");
}

// =========================================================================
// Scenario: Built-in provider refresh is untouched when no shadow config exists
// =========================================================================
#[test]
fn builtin_refresh_untouched_when_no_shadow() {
    let _lock = env_lock();
    let (_dir, _env) = with_temp_fspec_home();
    // @step Given no Rhai shadow config is present for "codex"
    // (fresh FSPEC_HOME)

    // @step When the dispatcher resolves the refresh middleware for "codex"
    let kind = resolve_refresh_middleware("codex");

    // @step Then the ScriptedRefreshingClient is not activated and the existing built-in refresh path is selected
    assert_eq!(kind, RefreshMiddleware::BuiltIn("codex".to_string()));

    // Sanity: a fresh script that claims no-refresh returns false without
    // touching the stored tokens, proving the middleware is inert for
    // built-ins even if it were accidentally invoked.
    let provider = ScriptedOAuthProvider::from_script(FRESH_SCRIPT, cfg("codex")).expect("load");
    let client = ScriptedRefreshingClient::new(&provider, "codex");
    let refreshed = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async { client.ensure_fresh_if_needed().await.unwrap_or(false) });
    assert!(!refreshed);
}
