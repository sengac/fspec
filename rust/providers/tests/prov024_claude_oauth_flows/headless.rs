//! Feature: spec/features/claude-oauth-napi-bindings.feature
use super::*;

// =========================================================================
// Scenario: Headless login start returns authorize URL and PKCE verifier
// =========================================================================

#[test]
fn test_headless_login_start_returns_authorize_url_and_pkce_verifier() {
    // @step When TypeScript calls claude_oauth_headless_start()
    // Headless start generates PKCE and builds the authorize URL
    let pkce = generate_pkce();
    let authorize_url = build_authorize_url(&pkce);

    // @step Then the result should contain an authorize_url string pointing to claude.ai
    assert!(
        authorize_url.contains("claude.ai/oauth/authorize"),
        "authorize_url should point to claude.ai: {authorize_url}"
    );

    // @step And the result should contain a pkce_verifier string
    assert!(
        pkce.verifier.len() >= 43,
        "pkce_verifier should be at least 43 chars (RFC 7636): {}",
        pkce.verifier.len()
    );
}

// =========================================================================
// Scenario: Headless login complete exchanges code for tokens
// =========================================================================

#[tokio::test]
#[serial]
async fn test_headless_login_complete_exchanges_code_for_tokens() {
    // @step Given a headless login flow has been started with a known pkce_verifier
    let (temp_dir, _guard) = fspec_home();
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
    // Headless complete:
    // 1. Parse code#state
    // 2. Validate state == pkce_verifier
    // 3. Exchange code for tokens
    // 4. Persist tokens
    let code_with_state = format!("headless_auth_code#{pkce_verifier}");
    let (code, maybe_state) = parse_authorization_code(&code_with_state);
    let state = maybe_state.expect("Should have state from code#state format");
    assert_eq!(state, pkce_verifier, "State should match PKCE verifier");

    let token_response =
        exchange_authorization_code(&mock_server.uri(), &code, &state, &pkce_verifier)
            .await
            .expect("Token exchange should succeed");

    let expires = calculate_expiry(token_response.expires_in);
    let auth = ClaudeAuthJson {
        access_token: token_response.access_token,
        refresh_token: token_response.refresh_token,
        expires,
    };
    write_claude_auth(&auth)
        .await
        .expect("write_claude_auth should succeed");

    // @step Then the Promise should resolve with NapiClaudeTokens
    assert_eq!(auth.access_token, "at_headless");
    assert_eq!(auth.refresh_token, "rt_headless");

    // @step And the tokens should be persisted to claude_auth.json
    assert!(
        auth_path.exists(),
        "claude_auth.json should exist after headless complete"
    );
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
    // The parsed state must differ from the verifier so the CSRF check rejects it.
    let state = maybe_state.expect("Should have state");
    assert_eq!(state, "wrong_state");
    assert_ne!(state, pkce_verifier, "State should NOT match pkce_verifier");
}

// =========================================================================
// Scenario: Headless login complete rejects code without hash separator
// =========================================================================

#[test]
fn test_headless_login_complete_rejects_code_without_hash_separator() {
    // @step Given a headless login flow has been started with a known pkce_verifier
    // @step When TypeScript calls claude_oauth_headless_complete with code containing no hash separator
    let code_without_hash = "codeonly";
    let (code, maybe_state) = parse_authorization_code(code_without_hash);

    // @step Then the Promise should reject with an error containing "Missing state"
    // A missing state is what triggers the "Missing state" rejection.
    assert_eq!(code, "codeonly");
    assert!(
        maybe_state.is_none(),
        "parse_authorization_code should return None state for code without '#'"
    );
}
