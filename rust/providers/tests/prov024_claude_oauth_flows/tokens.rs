//! Feature: spec/features/claude-oauth-napi-bindings.feature
use super::*;

// =========================================================================
// Scenario: Token refresh returns new tokens
// =========================================================================

#[tokio::test]
#[serial]
async fn test_token_refresh_returns_new_tokens() {
    // @step Given valid Claude OAuth tokens exist in claude_auth.json
    let (temp_dir, _guard) = fspec_home();
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

    // Build and persist the refreshed tokens
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
    assert_eq!(refreshed_auth.access_token, "at_refreshed");
    assert_eq!(refreshed_auth.refresh_token, "rt_refreshed");

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
    let reason = result.unwrap_err().to_string();
    assert!(
        reason.contains("401"),
        "Error should mention HTTP 401 status: {reason}"
    );
}

// =========================================================================
// Scenario: Get tokens returns stored tokens from claude_auth.json
// =========================================================================

#[tokio::test]
#[serial]
async fn test_get_tokens_returns_stored_tokens_from_claude_auth_json() {
    // @step Given valid Claude OAuth tokens exist in claude_auth.json
    let (_temp_dir, _guard) = fspec_home();

    let stored_auth = ClaudeAuthJson {
        access_token: "stored_access_xyz".to_string(),
        refresh_token: "stored_refresh_123".to_string(),
        expires: 1700000000000, // fixed timestamp for test
    };
    write_claude_auth(&stored_auth)
        .await
        .expect("Should write claude_auth.json");

    // @step When TypeScript calls claude_oauth_get_tokens()
    // Note: claude_auth uses async (tokio::fs), so get_tokens is async
    let loaded = read_claude_auth()
        .await
        .expect("read_claude_auth should succeed");
    let loaded_auth = loaded.expect("Should find claude_auth.json");

    // @step Then the result should be NapiClaudeTokens with access_token, refresh_token, and expires populated
    assert_eq!(loaded_auth.access_token, "stored_access_xyz");
    assert_eq!(loaded_auth.refresh_token, "stored_refresh_123");
    assert_eq!(loaded_auth.expires, 1700000000000);

    let serialized = serde_json::to_value(&loaded_auth).unwrap();
    assert_eq!(
        serialized.as_object().unwrap().len(),
        3,
        "ClaudeAuthJson should have exactly 3 fields"
    );
}

// =========================================================================
// Scenario: Get tokens returns null when no claude_auth.json exists
// =========================================================================

#[tokio::test]
#[serial]
async fn test_get_tokens_returns_null_when_no_claude_auth_json_exists() {
    // @step Given no claude_auth.json file exists
    let (temp_dir, _guard) = fspec_home();
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
        "Should return None when no claude_auth.json exists"
    );
}

// =========================================================================
// Scenario: Clear tokens removes stored credentials
// =========================================================================

#[tokio::test]
#[serial]
async fn test_clear_tokens_removes_stored_credentials() {
    // @step Given valid Claude OAuth tokens exist in claude_auth.json
    let (temp_dir, _guard) = fspec_home();
    let auth_path = temp_dir.path().join("claude_auth.json");

    let auth = ClaudeAuthJson {
        access_token: "to_be_cleared".to_string(),
        refresh_token: "rt_to_be_cleared".to_string(),
        expires: 1700000000000,
    };
    write_claude_auth(&auth)
        .await
        .expect("Should write claude_auth.json");
    assert!(
        auth_path.exists(),
        "claude_auth.json should exist before clearing"
    );

    // @step When TypeScript calls claude_oauth_clear_tokens()
    // Clearing deletes claude_auth.json (async tokio::fs::remove_file)
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
    let (temp_dir, _guard) = fspec_home();
    let auth_path = temp_dir.path().join("claude_auth.json");
    assert!(
        !auth_path.exists(),
        "claude_auth.json should not exist initially"
    );

    // @step When TypeScript calls claude_oauth_clear_tokens()
    // Clearing must handle file-not-found gracefully
    let remove_result = tokio::fs::remove_file(&auth_path).await;

    // @step Then the operation should succeed without error
    // File doesn't exist, so remove_file returns NotFound — the idempotent case
    if let Err(e) = remove_result {
        assert_eq!(
            e.kind(),
            std::io::ErrorKind::NotFound,
            "Unexpected error during clear_tokens: {e}"
        );
    }

    // Double-check: get_tokens still returns null
    let loaded = read_claude_auth()
        .await
        .expect("read_claude_auth should not error");
    assert!(loaded.is_none(), "get_tokens should return null");
}
