//! Shared test fixtures for OAuth NAPI integration tests.
//!
//! Provides reusable helpers for:
//! - Building test JWTs with known account IDs (Codex)
//! - Constructing token endpoint response bodies (Codex + Claude)
//! - Setting up isolated CODEX_HOME temp directories (Codex)
//! - Setting up isolated FSPEC_HOME temp directories (Claude)
//!
//! These mirror the fixtures from codelet-providers/tests/fixtures/mod.rs
//! because test modules can't import across crate boundaries.
//!
//! Items here are used across multiple integration test crates. Each test binary
//! only uses a subset, so we suppress dead_code warnings at the module level.
#![allow(dead_code)]

use base64::Engine;

/// Build a minimal valid JWT containing a `chatgpt_account_id` claim.
///
/// The JWT has the structure: header.payload.stub_signature
/// Uses real Base64URL encoding — no mocks.
pub fn build_test_jwt(account_id: &str) -> String {
    let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(r#"{"typ":"JWT","alg":"none"}"#.as_bytes());
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(format!(r#"{{"chatgpt_account_id":"{account_id}"}}"#).as_bytes());
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

// =========================================================================
// Claude OAuth Fixtures (PROV-024)
// =========================================================================

/// Build a Claude token response JSON body for wiremock.
///
/// Matches the shape of `ClaudeTokenResponse` from claude_oauth.rs:
/// { access_token, refresh_token, expires_in }
///
/// Unlike Codex, there is no id_token.
pub fn build_claude_token_response_json(
    access_token: &str,
    refresh_token: &str,
    expires_in: u64,
) -> serde_json::Value {
    serde_json::json!({
        "access_token": access_token,
        "refresh_token": refresh_token,
        "expires_in": expires_in
    })
}

/// RAII guard that restores the original FSPEC_HOME env var on drop.
pub struct FspecHomeGuard {
    original: Option<String>,
}

impl Drop for FspecHomeGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(val) => std::env::set_var("FSPEC_HOME", val),
            None => std::env::remove_var("FSPEC_HOME"),
        }
    }
}

/// Create a temp directory and point FSPEC_HOME to it.
///
/// Returns `(TempDir, FspecHomeGuard)` — keep both alive for the test duration.
/// The guard restores the original FSPEC_HOME on drop.
///
/// Claude uses FSPEC_HOME (not CODEX_HOME) for claude_auth.json.
pub fn setup_fspec_home() -> (tempfile::TempDir, FspecHomeGuard) {
    let temp_dir = tempfile::TempDir::new().expect("Should create temp dir");
    let guard = FspecHomeGuard {
        original: std::env::var("FSPEC_HOME").ok(),
    };
    std::env::set_var("FSPEC_HOME", temp_dir.path());
    (temp_dir, guard)
}
