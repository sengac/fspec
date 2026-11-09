#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/claude-oauth-napi-bindings.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios for PROV-024:
//! Anthropic OAuth subscription flows (browser login, two-phase headless
//! start + complete, token refresh, get/clear tokens).
//!
//! Relocated from the retired codelet-napi crate: the binding layer is gone,
//! so these tests drive the same codelet-providers functions the bindings
//! wrapped and assert on the real `ClaudeAuthJson` shape.
//!
//! Key differences from prov015_codex_oauth_flows.rs:
//! - No id_token, no account_id, no JWT extraction — simpler token struct
//! - get_tokens and clear_tokens are async (tokio::fs) not sync
//! - Headless start+complete instead of device auth start+poll
//! - Token exchange uses JSON POST, not form-encoded
//!
//! Tests use:
//! - The real underlying Rust functions from codelet-providers
//! - wiremock for HTTP endpoint simulation
//! - Real auth persistence via write_claude_auth (with temp dirs)

mod fixtures;

use codelet_providers::claude_auth::{read_claude_auth, write_claude_auth, ClaudeAuthJson};
use codelet_providers::claude_oauth::{
    build_authorize_url, calculate_expiry, exchange_authorization_code, parse_authorization_code,
    refresh_access_token_at,
};
use codelet_providers::claude_oauth_server::{
    claude_browser_oauth_login_inner, ClaudeOAuthServerConfig,
};
use codelet_providers::oauth_crypto::generate_pkce;
use fixtures::{build_claude_token_response_json, setup_fspec_home};
use serial_test::serial;
use tokio::net::TcpListener as TokioTcpListener;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "prov024_claude_oauth_flows/browser.rs"]
mod browser;
#[path = "prov024_claude_oauth_flows/headless.rs"]
mod headless;
#[path = "prov024_claude_oauth_flows/tokens.rs"]
mod tokens;

/// Isolated FSPEC_HOME for the duration of a test (keep both alive).
fn fspec_home() -> (tempfile::TempDir, fixtures::FspecHomeGuard) {
    setup_fspec_home().expect("Should create isolated FSPEC_HOME")
}

/// Helper: bind a tokio TcpListener to port 0 (OS-assigned) and return it with its port.
async fn ephemeral_listener() -> (TokioTcpListener, u16) {
    let listener = TokioTcpListener::bind("127.0.0.1:0")
        .await
        .expect("Should bind ephemeral port");
    let port = listener.local_addr().unwrap().port();
    (listener, port)
}

/// Simple percent-encoding for form data values
fn urlencoded(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                format!("{}", b as char)
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}
