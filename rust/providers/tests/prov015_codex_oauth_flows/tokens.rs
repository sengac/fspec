//! Feature: spec/features/codex-oauth-napi-bindings.feature
use super::*;

// =========================================================================
// Scenario: Token refresh returns new tokens
// =========================================================================

#[tokio::test]
#[serial]
async fn test_token_refresh_returns_new_tokens() {
    // @step Given valid OAuth tokens exist in auth.json
    let (temp_dir, _guard) = codex_home();
    let auth_path = temp_dir.path().join("auth.json");

    let original_tokens = CodexTokens {
        id_token: "original_id_token".to_string(),
        access_token: "original_access_token".to_string(),
        refresh_token: "rt_valid_for_refresh".to_string(),
        account_id: "acct_original".to_string(),
    };
    let original_auth = CodexAuthJson {
        openai_api_key: None,
        tokens: Some(original_tokens),
        last_refresh: None,
    };
    write_codex_auth(&original_auth).expect("Should write initial auth");
    assert!(auth_path.exists());

    // @step And the token endpoint accepts refresh_token requests
    let mock_server = MockServer::start().await;
    let account_id = "acct_refreshed";
    let new_id_token = build_test_jwt(account_id);
    let token_body = build_token_response_json(&new_id_token, "at_refreshed", "rt_refreshed");

    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=refresh_token"))
        .and(body_string_contains("refresh_token=rt_valid_for_refresh"))
        .and(body_string_contains(format!("client_id={CODEX_CLIENT_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(&token_body))
        .expect(1)
        .mount(&mock_server)
        .await;

    // @step When TypeScript calls codex_oauth_refresh_token with a valid refresh token
    let token_response = refresh_access_token_at(&mock_server.uri(), "rt_valid_for_refresh")
        .await
        .expect("refresh_access_token_at should succeed");

    // @step Then the Promise should resolve with NapiCodexTokens containing a new access_token
    let tokens = build_and_persist_tokens(&token_response);
    assert_eq!(tokens.access_token, "at_refreshed");
    assert_eq!(tokens.refresh_token, "rt_refreshed");
    assert_eq!(tokens.account_id, account_id);

    // @step And the refreshed tokens should be persisted to auth.json
    let persisted = std::fs::read_to_string(&auth_path).unwrap();
    let persisted_json: serde_json::Value = serde_json::from_str(&persisted).unwrap();
    assert_eq!(persisted_json["tokens"]["access_token"], "at_refreshed");
    assert_eq!(persisted_json["tokens"]["refresh_token"], "rt_refreshed");
    assert_eq!(persisted_json["tokens"]["account_id"], account_id);
}

// =========================================================================
// Scenario: Token refresh fails with invalid refresh token
// =========================================================================

#[tokio::test]
#[serial]
async fn test_token_refresh_fails_with_invalid_refresh_token() {
    // @step Given the token endpoint rejects the refresh_token
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=refresh_token"))
        .respond_with(
            ResponseTemplate::new(401).set_body_string("invalid_grant: refresh token is invalid"),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    // @step When TypeScript calls codex_oauth_refresh_token with an invalid refresh token
    let result = refresh_access_token_at(&mock_server.uri(), "rt_completely_invalid").await;

    // @step Then the Promise should reject with an error describing the failure
    assert!(
        result.is_err(),
        "Token refresh should fail with invalid token"
    );
    let reason = result.unwrap_err().to_string();
    assert!(
        reason.contains("401"),
        "Error should mention HTTP 401 status: {reason}"
    );
    assert!(
        reason.contains("invalid_grant"),
        "Error should contain server error body: {reason}"
    );
}

// =========================================================================
// Scenario: Get tokens returns stored tokens from auth.json
// =========================================================================

#[test]
#[serial]
fn test_get_tokens_returns_stored_tokens_from_auth_json() {
    // @step Given valid OAuth tokens exist in auth.json
    let (_temp_dir, _guard) = codex_home();

    let stored_tokens = CodexTokens {
        id_token: "stored_id_token_abc".to_string(),
        access_token: "stored_access_token_xyz".to_string(),
        refresh_token: "stored_refresh_token_123".to_string(),
        account_id: "acct_stored_test".to_string(),
    };
    let auth = CodexAuthJson {
        openai_api_key: None,
        tokens: Some(stored_tokens),
        last_refresh: None,
    };
    write_codex_auth(&auth).expect("Should write auth.json");

    // @step When TypeScript calls codex_oauth_get_tokens()
    let loaded = read_codex_auth().expect("read_codex_auth should succeed");
    let loaded_auth = loaded.expect("Should find auth.json");
    let tokens = loaded_auth.tokens.expect("Should have tokens");

    // @step Then the result should be NapiCodexTokens with all 4 fields populated
    assert_eq!(tokens.id_token, "stored_id_token_abc");
    assert_eq!(tokens.access_token, "stored_access_token_xyz");
    assert_eq!(tokens.refresh_token, "stored_refresh_token_123");
    assert_eq!(tokens.account_id, "acct_stored_test");

    let serialized = serde_json::to_value(&tokens).unwrap();
    assert_eq!(
        serialized.as_object().unwrap().len(),
        4,
        "CodexTokens should have exactly 4 fields"
    );
}

// =========================================================================
// Scenario: Get tokens returns null when no auth.json exists
// =========================================================================

#[test]
#[serial]
fn test_get_tokens_returns_null_when_no_auth_json_exists() {
    // @step Given no auth.json file exists
    let (temp_dir, _guard) = codex_home();
    let auth_path = temp_dir.path().join("auth.json");
    assert!(!auth_path.exists(), "auth.json should not exist initially");

    // @step When TypeScript calls codex_oauth_get_tokens()
    let loaded = read_codex_auth().expect("read_codex_auth should not error on missing file");

    // @step Then the result should be null
    assert!(
        loaded.is_none(),
        "Should return None when no auth.json exists"
    );
}
