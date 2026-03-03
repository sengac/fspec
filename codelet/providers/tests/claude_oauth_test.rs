//! Feature: spec/features/claude-oauth-core.feature
//!
//! This test file validates the acceptance criteria for PROV-020:
//! Claude OAuth core flow for Anthropic subscriptions.
//!
//! Tests map directly to Gherkin scenarios in the feature file.
//! All pure-function tests use #[test], async HTTP tests use wiremock.

// NOTE: This module `claude_oauth` does not yet exist — these tests will
// fail to compile until the implementation is written.  That's exactly
// what we want in the "red" phase.
use codelet_providers::claude_oauth::{
    build_authorize_url, build_oauth_headers, calculate_expiry, exchange_authorization_code,
    parse_authorization_code, prefix_tool_name, refresh_access_token_at, rewrite_claude_url,
    strip_tool_name_prefix, CLAUDE_CLIENT_ID, CLAUDE_REDIRECT_URI,
    CLAUDE_SCOPE, CLAUDE_TOKEN_ENDPOINT, CLAUDE_USER_AGENT, REQUIRED_BETA_HEADERS, TOOL_NAME_PREFIX,
};
use codelet_providers::oauth_crypto::{generate_pkce, PkceCodes};

// =========================================================================
// Scenario: PKCE code verifier meets RFC 7636 requirements
// =========================================================================

#[test]
fn test_pkce_code_verifier_meets_rfc_7636_requirements() {
    // @step Given the Anthropic OAuth module is available
    // Module imported above — compile-time proof

    // @step When I generate a PKCE code challenge pair
    let pkce = generate_pkce();

    // @step Then the verifier should be at least 43 characters long
    assert!(
        pkce.verifier.len() >= 43,
        "Verifier too short: {}",
        pkce.verifier.len()
    );

    // @step And the verifier should contain only unreserved URI characters
    let allowed = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    for c in pkce.verifier.chars() {
        assert!(allowed.contains(c), "Invalid character in verifier: {}", c);
    }

    // @step And the challenge method should be "S256"
    assert_eq!(pkce.challenge_method, "S256");

    // @step And the challenge should be the Base64URL-encoded SHA-256 hash of the verifier
    let pkce2 = PkceCodes::from_verifier(pkce.verifier.clone());
    assert_eq!(pkce.challenge, pkce2.challenge);
}

// =========================================================================
// Scenario: PKCE challenge is deterministic for a given verifier
// =========================================================================

#[test]
fn test_pkce_challenge_is_deterministic_for_a_given_verifier() {
    // @step Given a known PKCE verifier string "test_verifier_abc"
    let verifier = "test_verifier_abc".to_string();

    // @step When I compute the S256 challenge twice
    let pkce1 = PkceCodes::from_verifier(verifier.clone());
    let pkce2 = PkceCodes::from_verifier(verifier);

    // @step Then both challenges should be identical
    assert_eq!(pkce1.challenge, pkce2.challenge);
}

// =========================================================================
// Scenario: Authorize URL contains all required parameters for Max mode
// =========================================================================

#[test]
fn test_authorize_url_contains_all_required_parameters_for_max_mode() {
    // @step Given a PKCE challenge pair has been generated
    let pkce = generate_pkce();

    // @step When I build the authorize URL for "max" mode
    let url = build_authorize_url(&pkce);

    // @step Then the URL base should be "https://claude.ai/oauth/authorize"
    assert!(
        url.starts_with("https://claude.ai/oauth/authorize?"),
        "URL base wrong: {}",
        url
    );

    // @step And the URL should contain parameter "code" with value "true"
    assert!(url.contains("code=true"), "Missing code=true: {}", url);

    // @step And the URL should contain parameter "client_id" with value "9d1c250a-e61b-44d9-88ed-5944d1962f5e"
    assert!(
        url.contains(&format!("client_id={}", CLAUDE_CLIENT_ID)),
        "Missing client_id: {}",
        url
    );

    // @step And the URL should contain parameter "response_type" with value "code"
    assert!(
        url.contains("response_type=code"),
        "Missing response_type: {}",
        url
    );

    // @step And the URL should contain parameter "redirect_uri" with value "https://console.anthropic.com/oauth/code/callback"
    assert!(
        url.contains("redirect_uri="),
        "Missing redirect_uri: {}",
        url
    );

    // @step And the URL should contain parameter "scope" with value "org:create_api_key user:profile user:inference"
    assert!(url.contains("scope="), "Missing scope: {}", url);

    // @step And the URL should contain parameter "code_challenge" matching the PKCE challenge
    assert!(
        url.contains(&format!("code_challenge={}", pkce.challenge)),
        "Missing code_challenge: {}",
        url
    );

    // @step And the URL should contain parameter "code_challenge_method" with value "S256"
    assert!(
        url.contains("code_challenge_method=S256"),
        "Missing code_challenge_method: {}",
        url
    );

    // @step And the URL should contain parameter "state" matching the PKCE verifier
    assert!(
        url.contains(&format!("state={}", pkce.verifier)),
        "Missing state=verifier: {}",
        url
    );
}

// =========================================================================
// Scenario: Authorization code in code-hash-state format is parsed correctly
// =========================================================================

#[test]
fn test_authorization_code_in_code_hash_state_format_is_parsed_correctly() {
    // @step Given an authorization response "l0pnTslNFOmT#FgE6g_6khGKF"
    let raw = "l0pnTslNFOmT#FgE6g_6khGKF";

    // @step When the code is parsed
    let (code, state) = parse_authorization_code(raw);

    // @step Then the extracted code should be "l0pnTslNFOmT"
    assert_eq!(code, "l0pnTslNFOmT");

    // @step And the extracted state should be "FgE6g_6khGKF"
    assert_eq!(state, Some("FgE6g_6khGKF".to_string()));
}

// =========================================================================
// Scenario: Authorization code without hash separator is used as-is
// =========================================================================

#[test]
fn test_authorization_code_without_hash_separator_is_used_as_is() {
    // @step Given an authorization response "abc123"
    let raw = "abc123";

    // @step When the code is parsed
    let (code, state) = parse_authorization_code(raw);

    // @step Then the extracted code should be "abc123"
    assert_eq!(code, "abc123");

    // @step And the extracted state should be empty
    assert_eq!(state, None);
}

// =========================================================================
// Scenario: Authorization code exchanged for tokens at token endpoint
// =========================================================================

#[tokio::test]
async fn test_authorization_code_exchanged_for_tokens_at_token_endpoint() {
    // @step Given a valid authorization code "test_code" with state "test_state"
    let code = "test_code";
    let state = "test_state";

    // @step And a PKCE verifier "test_verifier"
    let code_verifier = "test_verifier";

    // Start wiremock server to simulate the token endpoint
    let mock_server = wiremock::MockServer::start().await;

    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, ResponseTemplate};

    let expected_body = serde_json::json!({
        "grant_type": "authorization_code",
        "code": code,
        "state": state,
        "client_id": CLAUDE_CLIENT_ID,
        "redirect_uri": CLAUDE_REDIRECT_URI,
        "code_verifier": code_verifier,
    });

    // @step When the code is exchanged for tokens at the token endpoint
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .and(header("Content-Type", "application/json"))
        .and(body_json(&expected_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "new_access_token",
            "refresh_token": "new_refresh_token",
            "expires_in": 3600
        })))
        .mount(&mock_server)
        .await;

    let result =
        exchange_authorization_code(&mock_server.uri(), code, state, code_verifier).await;

    // @step Then the exchange request should be a JSON POST to "https://console.anthropic.com/v1/oauth/token"
    // Verified by wiremock mock matching above

    // @step And the request Content-Type should be "application/json"
    // Verified by wiremock header matcher above

    // @step And the request body should contain "grant_type" as "authorization_code"
    // @step And the request body should contain "code" as "test_code"
    // @step And the request body should contain "state" as "test_state"
    // @step And the request body should contain "client_id" as "9d1c250a-e61b-44d9-88ed-5944d1962f5e"
    // @step And the request body should contain "redirect_uri" as "https://console.anthropic.com/oauth/code/callback"
    // @step And the request body should contain "code_verifier" as "test_verifier"
    // All verified by body_json matcher above

    // @step And the response should contain access_token, refresh_token, and expires_in
    let token_response = result.expect("Token exchange should succeed");
    assert_eq!(token_response.access_token, "new_access_token");
    assert_eq!(token_response.refresh_token, "new_refresh_token");
    assert_eq!(token_response.expires_in, 3600);
}

// =========================================================================
// Scenario: Code exchange fails with invalid authorization code
// =========================================================================

#[tokio::test]
async fn test_code_exchange_fails_with_invalid_authorization_code() {
    // @step Given an invalid authorization code "bad_code"
    let code = "bad_code";

    let mock_server = wiremock::MockServer::start().await;

    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"error":"invalid_grant","error_description":"Invalid code"}"#),
        )
        .mount(&mock_server)
        .await;

    // @step When the code is exchanged for tokens at the token endpoint
    let result = exchange_authorization_code(&mock_server.uri(), code, "", "verifier").await;

    // @step Then the exchange should fail with an error containing the HTTP status
    assert!(result.is_err(), "Exchange should fail with invalid code");
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("400"),
        "Error should contain HTTP status: {}",
        error_msg
    );

    // @step And the error should contain the response body
    assert!(
        error_msg.contains("invalid_grant") || error_msg.contains("Invalid code"),
        "Error should contain response body: {}",
        error_msg
    );
}

// =========================================================================
// Scenario: Token refresh using refresh_token grant
// =========================================================================

#[tokio::test]
async fn test_token_refresh_using_refresh_token_grant() {
    // @step Given a valid refresh token "existing_refresh_token"
    let refresh_token = "existing_refresh_token";

    let mock_server = wiremock::MockServer::start().await;

    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, ResponseTemplate};

    let expected_body = serde_json::json!({
        "grant_type": "refresh_token",
        "refresh_token": refresh_token,
        "client_id": CLAUDE_CLIENT_ID,
    });

    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .and(header("Content-Type", "application/json"))
        .and(body_json(&expected_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "refreshed_access_token",
            "refresh_token": "new_refresh_token",
            "expires_in": 7200
        })))
        .mount(&mock_server)
        .await;

    // @step When the token is refreshed
    let result = refresh_access_token_at(&mock_server.uri(), refresh_token).await;

    // @step Then the refresh request should be a JSON POST to "https://console.anthropic.com/v1/oauth/token"
    // Verified by wiremock mock matching

    // @step And the request body should contain "grant_type" as "refresh_token"
    // @step And the request body should contain "refresh_token" as "existing_refresh_token"
    // @step And the request body should contain "client_id" as "9d1c250a-e61b-44d9-88ed-5944d1962f5e"
    // All verified by body_json matcher above

    // @step And the response should contain a new access_token, refresh_token, and expires_in
    let token_response = result.expect("Token refresh should succeed");
    assert_eq!(token_response.access_token, "refreshed_access_token");
    assert_eq!(token_response.refresh_token, "new_refresh_token");
    assert_eq!(token_response.expires_in, 7200);
}

// =========================================================================
// Scenario: OAuth headers built with required beta headers
// =========================================================================

#[test]
fn test_oauth_headers_built_with_required_beta_headers() {
    // @step Given an access token "test_access_token"
    let access_token = "test_access_token";

    // @step When OAuth headers are built for an API request
    let headers = build_oauth_headers(access_token, None);

    // @step Then the Authorization header should be "Bearer test_access_token"
    assert_eq!(
        headers.get("authorization").unwrap(),
        "Bearer test_access_token"
    );

    // @step And the anthropic-beta header should contain "oauth-2025-04-20"
    let beta = headers.get("anthropic-beta").unwrap();
    assert!(
        beta.contains("oauth-2025-04-20"),
        "Missing oauth beta: {}",
        beta
    );

    // @step And the anthropic-beta header should contain "interleaved-thinking-2025-05-14"
    assert!(
        beta.contains("interleaved-thinking-2025-05-14"),
        "Missing thinking beta: {}",
        beta
    );

    // @step And the user-agent header should be "claude-cli/2.1.3 (external, cli)"
    assert_eq!(
        headers.get("user-agent").unwrap(),
        CLAUDE_USER_AGENT
    );

    // @step And the x-api-key header should be removed
    // build_oauth_headers does not include x-api-key; callers must strip it
    assert!(
        !headers.contains_key("x-api-key"),
        "x-api-key should not be present"
    );
}

// =========================================================================
// Scenario: OAuth headers preserve existing beta headers
// =========================================================================

#[test]
fn test_oauth_headers_preserve_existing_beta_headers() {
    // @step Given an access token "test_access_token"
    let access_token = "test_access_token";

    // @step And existing beta headers "prompt-caching-2024-07-31"
    let existing_beta = Some("prompt-caching-2024-07-31");

    // @step When OAuth headers are built for an API request
    let headers = build_oauth_headers(access_token, existing_beta);

    let beta = headers.get("anthropic-beta").unwrap();

    // @step Then the anthropic-beta header should contain "oauth-2025-04-20"
    assert!(beta.contains("oauth-2025-04-20"), "Missing oauth beta: {}", beta);

    // @step And the anthropic-beta header should contain "interleaved-thinking-2025-05-14"
    assert!(
        beta.contains("interleaved-thinking-2025-05-14"),
        "Missing thinking beta: {}",
        beta
    );

    // @step And the anthropic-beta header should contain "prompt-caching-2024-07-31"
    assert!(
        beta.contains("prompt-caching-2024-07-31"),
        "Missing existing beta: {}",
        beta
    );
}

// =========================================================================
// Scenario: Tool names prefixed with mcp_ in OAuth mode
// =========================================================================

#[test]
fn test_tool_names_prefixed_with_mcp_in_oauth_mode() {
    // @step Given a tool named "Bash"
    let name = "Bash";

    // @step When the tool name is prefixed for OAuth mode
    let prefixed = prefix_tool_name(name);

    // @step Then the prefixed name should be "mcp_Bash"
    assert_eq!(prefixed, "mcp_Bash");
}

// =========================================================================
// Scenario: Tool names stripped of mcp_ prefix from response
// =========================================================================

#[test]
fn test_tool_names_stripped_of_mcp_prefix_from_response() {
    // @step Given a response tool name "mcp_Bash"
    let name = "mcp_Bash";

    // @step When the prefix is stripped from the response
    let stripped = strip_tool_name_prefix(name);

    // @step Then the resulting name should be "Bash"
    assert_eq!(stripped, "Bash");
}

// =========================================================================
// Scenario: Messages URL rewritten with beta query parameter
// =========================================================================

#[test]
fn test_messages_url_rewritten_with_beta_query_parameter() {
    // @step Given a request URL "https://api.anthropic.com/v1/messages"
    let url = "https://api.anthropic.com/v1/messages";

    // @step When the URL is rewritten for OAuth mode
    let rewritten = rewrite_claude_url(url);

    // @step Then the URL should be "https://api.anthropic.com/v1/messages?beta=true"
    assert_eq!(rewritten, "https://api.anthropic.com/v1/messages?beta=true");
}

// =========================================================================
// Scenario: Messages URL with existing query parameters gets beta appended
// =========================================================================

#[test]
fn test_messages_url_with_existing_query_parameters_gets_beta_appended() {
    // @step Given a request URL "https://api.anthropic.com/v1/messages?stream=true"
    let url = "https://api.anthropic.com/v1/messages?stream=true";

    // @step When the URL is rewritten for OAuth mode
    let rewritten = rewrite_claude_url(url);

    // @step Then the URL should contain "beta=true"
    assert!(
        rewritten.contains("beta=true"),
        "Missing beta=true: {}",
        rewritten
    );

    // @step And the URL should preserve "stream=true"
    assert!(
        rewritten.contains("stream=true"),
        "Missing stream=true: {}",
        rewritten
    );
}

// =========================================================================
// Scenario: Non-messages URL is not rewritten
// =========================================================================

#[test]
fn test_non_messages_url_is_not_rewritten() {
    // @step Given a request URL "https://api.anthropic.com/v1/models"
    let url = "https://api.anthropic.com/v1/models";

    // @step When the URL is checked for OAuth rewriting
    let result = rewrite_claude_url(url);

    // @step Then the URL should remain unchanged
    assert_eq!(result, url);
}

// =========================================================================
// Scenario: Token expiry calculated from expires_in seconds
// =========================================================================

#[test]
fn test_token_expiry_calculated_from_expires_in_seconds() {
    // @step Given a token response with expires_in of 3600
    let expires_in: u64 = 3600;

    // @step When the expiry timestamp is calculated
    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let expiry = calculate_expiry(expires_in);

    let after = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    // @step Then the expiry should be approximately current time plus 3600 seconds
    let expected_min = before + expires_in * 1000;
    let expected_max = after + expires_in * 1000;
    assert!(
        expiry >= expected_min && expiry <= expected_max,
        "Expiry {} should be between {} and {}",
        expiry,
        expected_min,
        expected_max
    );
}

// =========================================================================
// Additional: Verify constants are correct
// =========================================================================

#[test]
fn test_claude_oauth_constants() {
    assert_eq!(CLAUDE_CLIENT_ID, "9d1c250a-e61b-44d9-88ed-5944d1962f5e");
    assert_eq!(
        CLAUDE_TOKEN_ENDPOINT,
        "https://console.anthropic.com/v1/oauth/token"
    );
    assert_eq!(
        CLAUDE_REDIRECT_URI,
        "https://console.anthropic.com/oauth/code/callback"
    );
    assert_eq!(CLAUDE_SCOPE, "org:create_api_key user:profile user:inference");
    assert_eq!(CLAUDE_USER_AGENT, "claude-cli/2.1.3 (external, cli)");
    assert_eq!(TOOL_NAME_PREFIX, "mcp_");
    assert_eq!(REQUIRED_BETA_HEADERS.len(), 2);
    assert!(REQUIRED_BETA_HEADERS.contains(&"oauth-2025-04-20"));
    assert!(REQUIRED_BETA_HEADERS.contains(&"interleaved-thinking-2025-05-14"));
}
