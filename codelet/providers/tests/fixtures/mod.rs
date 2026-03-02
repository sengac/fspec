//! Shared test fixtures for OAuth server integration tests.
//!
//! Provides reusable helpers for:
//! - Building test JWTs with known account IDs (Codex)
//! - Constructing token endpoint response bodies (Codex)
//! - Setting up isolated CODEX_HOME temp directories (Codex)
//! - Setting up isolated CODELET_HOME temp directories (Claude)
//!
//! These use REAL code from codex_oauth.rs / codex_auth.rs / claude_auth.rs — no mocks.
//!
//! Note: Each integration test binary compiles this module independently,
//! so some items may appear unused in a given binary but are used by others.

#![allow(dead_code)]

use base64::Engine;

/// Build a minimal valid JWT containing a `chatgpt_account_id` claim.
///
/// The JWT has the structure: header.payload.stub_signature
/// Uses real Base64URL encoding — no mocks.
pub fn build_test_jwt(account_id: &str) -> String {
    let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(r#"{"typ":"JWT","alg":"none"}"#.as_bytes());
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
        format!(r#"{{"chatgpt_account_id":"{account_id}"}}"#).as_bytes(),
    );
    format!("{header}.{payload}.stub_signature")
}

/// Build a JSON body matching the token endpoint response shape
/// (same struct as `TokenRefreshResponse`).
pub fn build_token_response_json(
    id_token: &str,
    access_token: &str,
    refresh_token: &str,
) -> serde_json::Value {
    serde_json::json!({
        "id_token": id_token,
        "access_token": access_token,
        "refresh_token": refresh_token,
        "expires_in": 3600
    })
}

/// RAII guard that restores the original CODEX_HOME env var on drop.
pub struct CodexHomeGuard {
    original: Option<String>,
}

impl Drop for CodexHomeGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(val) => std::env::set_var("CODEX_HOME", val),
            None => std::env::remove_var("CODEX_HOME"),
        }
    }
}

/// Create a temp directory and point CODEX_HOME to it.
///
/// Returns `(TempDir, CodexHomeGuard)` — keep both alive for the test duration.
/// The guard restores the original CODEX_HOME on drop.
pub fn setup_codex_home() -> (tempfile::TempDir, CodexHomeGuard) {
    let temp_dir = tempfile::TempDir::new().expect("Should create temp dir");
    let guard = CodexHomeGuard {
        original: std::env::var("CODEX_HOME").ok(),
    };
    std::env::set_var("CODEX_HOME", temp_dir.path());
    (temp_dir, guard)
}

/// RAII guard that restores the original CODELET_HOME env var on drop.
pub struct CodeletHomeGuard {
    original: Option<String>,
}

impl Drop for CodeletHomeGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(val) => std::env::set_var("CODELET_HOME", val),
            None => std::env::remove_var("CODELET_HOME"),
        }
    }
}

/// Create a temp directory and point CODELET_HOME to it.
///
/// Returns `(TempDir, CodeletHomeGuard)` — keep both alive for the test duration.
/// The guard restores the original CODELET_HOME on drop.
pub fn setup_codelet_home() -> (tempfile::TempDir, CodeletHomeGuard) {
    let temp_dir = tempfile::TempDir::new().expect("Should create temp dir");
    let guard = CodeletHomeGuard {
        original: std::env::var("CODELET_HOME").ok(),
    };
    std::env::set_var("CODELET_HOME", temp_dir.path());
    (temp_dir, guard)
}
