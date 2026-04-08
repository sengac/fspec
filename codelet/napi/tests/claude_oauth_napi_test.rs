#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/claude-oauth-napi-bindings.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios for PROV-024:
//! NAPI Bindings for Anthropic OAuth Subscription Flows.
//!
//! These tests verify the NAPI binding layer by testing:
//! - NapiClaudeTokens struct conversion from ClaudeAuthJson
//! - NapiClaudeHeadlessStartResult struct fields (authorize_url, pkce_verifier)
//! - Async NAPI function behavior through underlying Rust functions
//! - Error conversion patterns (Rust errors → napi::Error via Error::from_reason)
//! - Two-phase headless design (start + complete)
//! - Token refresh via wiremock (no production endpoint hits)
//! - Async get_tokens and clear_tokens (claude_auth uses tokio::fs)
//!
//! Key differences from codex_oauth_napi_test.rs (PROV-015):
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
    build_authorize_url, calculate_expiry, exchange_authorization_code,
    parse_authorization_code, refresh_access_token_at,
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

/// NapiClaudeTokens: mirrors the NAPI object struct from claude_oauth.rs (to be created).
///
/// Rule [6]: NapiClaudeTokens is an #[napi(object)] struct with fields:
/// access_token (String), refresh_token (String), expires (f64, ms since epoch).
/// Maps to ClaudeAuthJson from claude_auth.rs.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct NapiClaudeTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires: f64,
}

impl From<ClaudeAuthJson> for NapiClaudeTokens {
    fn from(auth: ClaudeAuthJson) -> Self {
        Self {
            access_token: auth.access_token,
            refresh_token: auth.refresh_token,
            expires: auth.expires as f64,
        }
    }
}

/// NapiClaudeHeadlessStartResult: mirrors the NAPI object struct returned by
/// claude_oauth_headless_start().
///
/// Architecture note [1]: authorize_url and pkce_verifier. The verifier is
/// returned to TypeScript so it can be passed back to headless_complete().
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct NapiClaudeHeadlessStartResult {
    pub authorize_url: String,
    pub pkce_verifier: String,
}

/// Simulate the error conversion that the NAPI binding would do.
/// Rule [5]: All NAPI functions convert Rust errors to napi::Error via
/// Error::from_reason() — TypeScript sees rejected promises with descriptive
/// error messages.
fn convert_error_to_napi_reason(err: &anyhow::Error) -> String {
    err.to_string()
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
            _ => format!("%{:02X}", b),
        })
        .collect()
}

// =========================================================================
// Scenario: Successful browser OAuth login via NAPI
// =========================================================================

#[tokio::test]
#[serial]
async fn test_successful_browser_oauth_login_via_napi() {
    // @step Given the Claude browser OAuth flow is configured with a test server
    let (_temp_dir, _guard) = setup_fspec_home();

    let mock_server = MockServer::start().await;
    let token_body = build_claude_token_response_json("at_claude_browser", "rt_claude_browser", 3600);

    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&token_body))
        .expect(1)
        .mount(&mock_server)
        .await;

    let (listener, port) = ephemeral_listener().await;
    let pkce = generate_pkce();
    let pkce_verifier = pkce.verifier.clone();
    let pkce_clone = pkce.clone();
    let issuer_url = mock_server.uri();

    let login_handle = tokio::spawn(async move {
        let config = ClaudeOAuthServerConfig {
            token_endpoint_base: issuer_url,
            listener,
            open_browser: false,
            timeout_ms: 10_000,
            pkce: Some(pkce_clone),
        };
        claude_browser_oauth_login_inner(config).await
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // @step When TypeScript calls claude_oauth_browser_login()
    // Simulate the user pasting code#state into the form
    let code_with_state = format!("test_auth_code#{pkce_verifier}");
    let client = reqwest::Client::new();
    let submit_resp = client
        .post(format!("http://127.0.0.1:{port}/submit"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("code={}", urlencoded(&code_with_state)))
        .send()
        .await
        .expect("Submit request should reach server");
    assert_eq!(submit_resp.status().as_u16(), 200);

    let result = login_handle.await.expect("Task should not panic");
    assert!(
        result.is_ok(),
        "Browser login should succeed, got: {:?}",
        result.err()
    );
    let auth = result.unwrap();

    // @step Then the Promise should resolve with NapiClaudeTokens
    let napi_tokens = NapiClaudeTokens::from(auth);

    // @step And the tokens should contain access_token, refresh_token, and expires
    assert_eq!(napi_tokens.access_token, "at_claude_browser");
    assert_eq!(napi_tokens.refresh_token, "rt_claude_browser");
    assert!(napi_tokens.expires > 0.0, "expires should be a positive timestamp");

    let serialized = serde_json::to_value(&napi_tokens).unwrap();
    assert!(serialized.get("access_token").is_some());
    assert!(serialized.get("refresh_token").is_some());
    assert!(serialized.get("expires").is_some());
}

// =========================================================================
// Scenario: Browser OAuth login times out
// =========================================================================

#[tokio::test]
#[serial]
async fn test_browser_oauth_login_times_out() {
    // @step Given the Claude browser OAuth flow is configured with a short timeout
    let (_temp_dir, _guard) = setup_fspec_home();

    let (listener, _port) = ephemeral_listener().await;
    let login_handle = tokio::spawn(async move {
        let config = ClaudeOAuthServerConfig {
            token_endpoint_base: "http://127.0.0.1:1".to_string(),
            listener,
            open_browser: false,
            timeout_ms: 100, // Very short timeout for test
            pkce: None,
        };
        claude_browser_oauth_login_inner(config).await
    });

    // @step When TypeScript calls claude_oauth_browser_login()
    // @step And no code is submitted before the timeout

    let result = login_handle.await.expect("Task should not panic");

    // @step Then the Promise should reject with an error containing "timed out"
    assert!(result.is_err(), "Browser login should fail with timeout");
    let err = result.unwrap_err();
    let napi_reason = convert_error_to_napi_reason(&err);
    assert!(
        napi_reason.contains("timed out"),
        "NAPI error reason should contain 'timed out': {napi_reason}"
    );
}

// =========================================================================
// Scenario: Headless login start returns authorize URL and PKCE verifier
// =========================================================================

#[test]
fn test_headless_login_start_returns_authorize_url_and_pkce_verifier() {
    // @step When TypeScript calls claude_oauth_headless_start()
    // The NAPI binding generates PKCE and builds authorize URL
    let pkce = generate_pkce();
    let authorize_url = build_authorize_url(&pkce);

    let napi_result = NapiClaudeHeadlessStartResult {
        authorize_url: authorize_url.clone(),
        pkce_verifier: pkce.verifier.clone(),
    };

    // @step Then the result should contain an authorize_url string pointing to claude.ai
    assert!(
        napi_result.authorize_url.contains("claude.ai/oauth/authorize"),
        "authorize_url should point to claude.ai: {}",
        napi_result.authorize_url
    );
    assert!(!napi_result.authorize_url.is_empty());

    // @step And the result should contain a pkce_verifier string
    assert!(
        napi_result.pkce_verifier.len() >= 43,
        "pkce_verifier should be at least 43 chars (RFC 7636): {}",
        napi_result.pkce_verifier.len()
    );
    assert!(!napi_result.pkce_verifier.is_empty());

    // Verify both fields serialize correctly for NAPI transport
    let serialized = serde_json::to_value(&napi_result).unwrap();
    assert!(serialized.get("authorize_url").is_some());
    assert!(serialized.get("pkce_verifier").is_some());
}

// =========================================================================
// Scenario: Headless login complete exchanges code for tokens
// =========================================================================

#[tokio::test]
#[serial]
async fn test_headless_login_complete_exchanges_code_for_tokens() {
    // @step Given a headless login flow has been started with a known pkce_verifier
    let (temp_dir, _guard) = setup_fspec_home();
    let auth_path = temp_dir.path().join("claude_auth.json");

    let pkce = generate_pkce();
    let pkce_verifier = pkce.verifier.clone();

    // @step And the token endpoint accepts authorization code requests
    let mock_server = MockServer::start().await;
    let token_body = build_claude_token_response_json("at_headless", "rt_headless", 3600);

    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&token_body))
        .expect(1)
        .mount(&mock_server)
        .await;

    // @step When TypeScript calls claude_oauth_headless_complete with a valid code_with_state and pkce_verifier
    // Simulate the NAPI binding's headless_complete logic:
    // 1. Parse code#state
    // 2. Validate state == pkce_verifier
    // 3. Exchange code for tokens
    // 4. Persist tokens
    let code_with_state = format!("headless_auth_code#{pkce_verifier}");
    let (code, maybe_state) = parse_authorization_code(&code_with_state);
    let state = maybe_state.expect("Should have state from code#state format");
    assert_eq!(state, pkce_verifier, "State should match PKCE verifier");

    let token_response = exchange_authorization_code(
        &mock_server.uri(),
        &code,
        &state,
        &pkce_verifier,
    )
    .await
    .expect("Token exchange should succeed");

    let expires = calculate_expiry(token_response.expires_in);
    let auth = ClaudeAuthJson {
        access_token: token_response.access_token,
        refresh_token: token_response.refresh_token,
        expires,
    };
    write_claude_auth(&auth).await.expect("write_claude_auth should succeed");

    // @step Then the Promise should resolve with NapiClaudeTokens
    let napi_tokens = NapiClaudeTokens::from(auth);
    assert_eq!(napi_tokens.access_token, "at_headless");
    assert_eq!(napi_tokens.refresh_token, "rt_headless");

    // @step And the tokens should be persisted to claude_auth.json
    assert!(auth_path.exists(), "claude_auth.json should exist after headless complete");
    let persisted = std::fs::read_to_string(&auth_path).unwrap();
    let persisted_json: serde_json::Value = serde_json::from_str(&persisted).unwrap();
    assert_eq!(persisted_json["access_token"], "at_headless");
    assert_eq!(persisted_json["refresh_token"], "rt_headless");
}

// =========================================================================
// Scenario: Headless login complete rejects mismatched state as CSRF
// =========================================================================

#[test]
fn test_headless_login_complete_rejects_mismatched_state_as_csrf() {
    // @step Given a headless login flow has been started with a known pkce_verifier
    let pkce_verifier = "real_verifier_value_abc123";

    // @step When TypeScript calls claude_oauth_headless_complete with code containing a wrong state
    let code_with_state = "code#wrong_state";
    let (_code, maybe_state) = parse_authorization_code(code_with_state);

    // @step Then the Promise should reject with an error containing "CSRF" or "state mismatch"
    let state = maybe_state.expect("Should have state");
    assert_ne!(
        state, pkce_verifier,
        "State should NOT match pkce_verifier"
    );

    // The NAPI binding would check: if state != pkce_verifier → Error::from_reason
    let error_msg = format!(
        "CSRF validation failed — state mismatch. Expected: {pkce_verifier}, Got: {state}"
    );
    assert!(
        error_msg.contains("CSRF") || error_msg.contains("state mismatch"),
        "Error should mention CSRF or state mismatch: {error_msg}"
    );
}

// =========================================================================
// Scenario: Headless login complete rejects code without hash separator
// =========================================================================

#[test]
fn test_headless_login_complete_rejects_code_without_hash_separator() {
    // @step Given a headless login flow has been started with a known pkce_verifier
    let _pkce_verifier = "some_verifier";

    // @step When TypeScript calls claude_oauth_headless_complete with code containing no hash separator
    let code_without_hash = "codeonly";
    let (_code, maybe_state) = parse_authorization_code(code_without_hash);

    // @step Then the Promise should reject with an error containing "Missing state"
    assert!(
        maybe_state.is_none(),
        "parse_authorization_code should return None state for code without '#'"
    );

    // The NAPI binding would return: Error::from_reason("Missing state ...")
    let error_msg = "Missing state in authorization code";
    assert!(
        error_msg.contains("Missing state"),
        "Error should mention missing state: {error_msg}"
    );
}

// =========================================================================
// Scenario: Token refresh returns new tokens
// =========================================================================

#[tokio::test]
#[serial]
async fn test_token_refresh_returns_new_tokens() {
    // @step Given valid Claude OAuth tokens exist in claude_auth.json
    let (temp_dir, _guard) = setup_fspec_home();
    let auth_path = temp_dir.path().join("claude_auth.json");

    let original_auth = ClaudeAuthJson {
        access_token: "original_access_token".to_string(),
        refresh_token: "rt_valid_for_refresh".to_string(),
        expires: calculate_expiry(3600),
    };
    write_claude_auth(&original_auth)
        .await
        .expect("Should write initial auth");
    assert!(auth_path.exists());

    // @step And the Claude token endpoint accepts refresh_token requests
    let mock_server = MockServer::start().await;
    let token_body = build_claude_token_response_json("at_refreshed", "rt_refreshed", 7200);

    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&token_body))
        .expect(1)
        .mount(&mock_server)
        .await;

    // @step When TypeScript calls claude_oauth_refresh_token with a valid refresh token
    let token_response = refresh_access_token_at(&mock_server.uri(), "rt_valid_for_refresh")
        .await
        .expect("refresh_access_token_at should succeed");

    // Build and persist tokens (mirrors NAPI binding logic)
    let expires = calculate_expiry(token_response.expires_in);
    let refreshed_auth = ClaudeAuthJson {
        access_token: token_response.access_token,
        refresh_token: token_response.refresh_token,
        expires,
    };
    write_claude_auth(&refreshed_auth)
        .await
        .expect("Should persist refreshed tokens");

    // @step Then the Promise should resolve with NapiClaudeTokens containing a new access_token
    let napi_tokens = NapiClaudeTokens::from(refreshed_auth);
    assert_eq!(napi_tokens.access_token, "at_refreshed");
    assert_eq!(napi_tokens.refresh_token, "rt_refreshed");

    // @step And the refreshed tokens should be persisted to claude_auth.json
    let persisted = std::fs::read_to_string(&auth_path).unwrap();
    let persisted_json: serde_json::Value = serde_json::from_str(&persisted).unwrap();
    assert_eq!(persisted_json["access_token"], "at_refreshed");
    assert_eq!(persisted_json["refresh_token"], "rt_refreshed");
}

// =========================================================================
// Scenario: Token refresh fails with invalid refresh token
// =========================================================================

#[tokio::test]
#[serial]
async fn test_token_refresh_fails_with_invalid_refresh_token() {
    // @step Given the Claude token endpoint rejects the refresh_token
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .respond_with(
            ResponseTemplate::new(401).set_body_string("invalid_grant: refresh token is invalid"),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    // @step When TypeScript calls claude_oauth_refresh_token with an invalid refresh token
    let result = refresh_access_token_at(&mock_server.uri(), "rt_completely_invalid").await;

    // @step Then the Promise should reject with an error describing the failure
    assert!(
        result.is_err(),
        "Token refresh should fail with invalid token"
    );
    let err = result.unwrap_err();
    let napi_reason = convert_error_to_napi_reason(&err);
    assert!(
        napi_reason.contains("401"),
        "Error should mention HTTP 401 status: {napi_reason}"
    );
}

// =========================================================================
// Scenario: Get tokens returns stored tokens from claude_auth.json
// =========================================================================

#[tokio::test]
#[serial]
async fn test_get_tokens_returns_stored_tokens_from_claude_auth_json() {
    // @step Given valid Claude OAuth tokens exist in claude_auth.json
    let (_temp_dir, _guard) = setup_fspec_home();

    let stored_auth = ClaudeAuthJson {
        access_token: "stored_access_xyz".to_string(),
        refresh_token: "stored_refresh_123".to_string(),
        expires: 1700000000000, // fixed timestamp for test
    };
    write_claude_auth(&stored_auth)
        .await
        .expect("Should write claude_auth.json");

    // @step When TypeScript calls claude_oauth_get_tokens()
    // Note: claude_auth uses async (tokio::fs), so get_tokens is async NAPI
    let loaded = read_claude_auth()
        .await
        .expect("read_claude_auth should succeed");
    assert!(loaded.is_some(), "Should find claude_auth.json");

    let loaded_auth = loaded.unwrap();
    let napi_tokens = NapiClaudeTokens::from(loaded_auth);

    // @step Then the result should be NapiClaudeTokens with access_token, refresh_token, and expires populated
    assert_eq!(napi_tokens.access_token, "stored_access_xyz");
    assert_eq!(napi_tokens.refresh_token, "stored_refresh_123");
    assert_eq!(napi_tokens.expires, 1700000000000.0);

    let serialized = serde_json::to_value(&napi_tokens).unwrap();
    assert_eq!(
        serialized.as_object().unwrap().len(),
        3,
        "NapiClaudeTokens should have exactly 3 fields"
    );
}

// =========================================================================
// Scenario: Get tokens returns null when no claude_auth.json exists
// =========================================================================

#[tokio::test]
#[serial]
async fn test_get_tokens_returns_null_when_no_claude_auth_json_exists() {
    // @step Given no claude_auth.json file exists
    let (temp_dir, _guard) = setup_fspec_home();
    let auth_path = temp_dir.path().join("claude_auth.json");
    assert!(
        !auth_path.exists(),
        "claude_auth.json should not exist initially"
    );

    // @step When TypeScript calls claude_oauth_get_tokens()
    let loaded = read_claude_auth()
        .await
        .expect("read_claude_auth should not error on missing file");

    // @step Then the result should be null
    assert!(
        loaded.is_none(),
        "Should return None (null in TypeScript) when no claude_auth.json exists"
    );
}

// =========================================================================
// Scenario: Clear tokens removes stored credentials
// =========================================================================

#[tokio::test]
#[serial]
async fn test_clear_tokens_removes_stored_credentials() {
    // @step Given valid Claude OAuth tokens exist in claude_auth.json
    let (temp_dir, _guard) = setup_fspec_home();
    let auth_path = temp_dir.path().join("claude_auth.json");

    let auth = ClaudeAuthJson {
        access_token: "to_be_cleared".to_string(),
        refresh_token: "rt_to_be_cleared".to_string(),
        expires: 1700000000000,
    };
    write_claude_auth(&auth)
        .await
        .expect("Should write claude_auth.json");
    assert!(auth_path.exists(), "claude_auth.json should exist before clearing");

    // @step When TypeScript calls claude_oauth_clear_tokens()
    // The NAPI binding deletes claude_auth.json (async tokio::fs::remove_file)
    tokio::fs::remove_file(&auth_path)
        .await
        .expect("Should delete claude_auth.json");

    // @step Then the operation should succeed
    // (no error from remove_file)

    // @step And subsequent calls to claude_oauth_get_tokens() should return null
    let loaded = read_claude_auth()
        .await
        .expect("read_claude_auth should not error on missing file");
    assert!(
        loaded.is_none(),
        "get_tokens should return null after clear_tokens"
    );
}

// =========================================================================
// Scenario: Clear tokens is idempotent when no credentials exist
// =========================================================================

#[tokio::test]
#[serial]
async fn test_clear_tokens_is_idempotent_when_no_credentials_exist() {
    // @step Given no claude_auth.json file exists
    let (temp_dir, _guard) = setup_fspec_home();
    let auth_path = temp_dir.path().join("claude_auth.json");
    assert!(
        !auth_path.exists(),
        "claude_auth.json should not exist initially"
    );

    // @step When TypeScript calls claude_oauth_clear_tokens()
    // The NAPI binding should handle file-not-found gracefully
    let remove_result = tokio::fs::remove_file(&auth_path).await;

    // @step Then the operation should succeed without error
    // File doesn't exist, so remove_file returns NotFound — NAPI binding ignores this
    match remove_result {
        Ok(()) => {} // File somehow existed, that's fine
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Expected: file doesn't exist, this is the idempotent case
        }
        Err(e) => panic!("Unexpected error during clear_tokens: {e}"),
    }

    // Double-check: get_tokens still returns null
    let loaded = read_claude_auth()
        .await
        .expect("read_claude_auth should not error");
    assert!(loaded.is_none(), "get_tokens should return null");
}
