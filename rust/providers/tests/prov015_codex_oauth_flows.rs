#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/codex-oauth-napi-bindings.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios for PROV-015:
//! Codex OAuth flows (browser login, two-phase device auth start + poll,
//! token refresh, get tokens).
//!
//! Relocated from the retired codelet-napi crate: the binding layer is gone,
//! so these tests drive the same codelet-providers functions the bindings
//! wrapped and assert on the real `CodexTokens` / `DeviceCodeResponse` shapes.
//!
//! Tests use:
//! - The real underlying Rust functions from codelet-providers
//! - wiremock for HTTP endpoint simulation
//! - Real JWT construction and account ID extraction
//! - Real auth.json persistence via write_codex_auth (with temp dirs)

mod fixtures;

use codelet_providers::codex::codex_auth::{
    read_codex_auth, write_codex_auth, CodexAuthJson, CodexTokens,
};
use codelet_providers::codex::codex_device_auth::{
    poll_device_token, request_device_code, DeviceCodeResponse, PollConfig, PollResult,
};
use codelet_providers::codex::codex_oauth::{
    exchange_authorization_code, extract_account_id, refresh_access_token_at, TokenRefreshResponse,
    CODEX_CLIENT_ID,
};
use codelet_providers::codex::codex_oauth_server::{browser_oauth_login_inner, OAuthServerConfig};
use fixtures::{build_test_jwt, build_token_response_json, setup_codex_home};
use serial_test::serial;
use tokio::net::TcpListener as TokioTcpListener;
use wiremock::matchers::{body_partial_json, body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "prov015_codex_oauth_flows/browser.rs"]
mod browser;
#[path = "prov015_codex_oauth_flows/device.rs"]
mod device;
#[path = "prov015_codex_oauth_flows/tokens.rs"]
mod tokens;

/// Isolated CODEX_HOME for the duration of a test (keep both alive).
fn codex_home() -> (tempfile::TempDir, fixtures::CodexHomeGuard) {
    setup_codex_home().expect("Should create isolated CODEX_HOME")
}

/// Helper: bind a tokio TcpListener to port 0 (OS-assigned) and return it with its port.
async fn ephemeral_listener() -> (TokioTcpListener, u16) {
    let listener = TokioTcpListener::bind("127.0.0.1:0")
        .await
        .expect("Should bind ephemeral port");
    let port = listener.local_addr().unwrap().port();
    (listener, port)
}

/// Build CodexTokens from a token response and persist to auth.json.
///
/// The extract → assemble → persist flow the device-poll and refresh
/// entry points run after a successful token response.
fn build_and_persist_tokens(token_response: &TokenRefreshResponse) -> CodexTokens {
    let account_id = extract_account_id(
        Some(&token_response.id_token),
        Some(&token_response.access_token),
    )
    .expect("failed to extract account_id from token response");

    let tokens = CodexTokens {
        id_token: token_response.id_token.clone(),
        access_token: token_response.access_token.clone(),
        refresh_token: token_response.refresh_token.clone(),
        account_id,
    };

    let auth = CodexAuthJson {
        openai_api_key: None,
        tokens: Some(tokens.clone()),
        last_refresh: None,
    };
    write_codex_auth(&auth).expect("failed to persist tokens to auth.json");

    tokens
}
