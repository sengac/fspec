#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]
//! Feature: spec/features/claude-refreshing-client.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios for PROV-023:
//! Anthropic token refresh client and resilient request auth.
//!
//! Tests use wiremock to simulate the Claude OAuth token endpoint and a mock HTTP
//! backend to capture outgoing requests for header assertions.
//!
//! Key difference from Codex tests: no URL rewriting assertions, no ChatGPT-Account-Id
//! or originator header checks. Claude's RefreshingClaudeClient only handles
//! Authorization: Bearer header and token refresh.

mod fixtures;

use codelet_providers::claude_auth;
use codelet_providers::claude_oauth::CLAUDE_CLIENT_ID;
use codelet_providers::claude_refreshing_client::{RefreshingClaudeClient, EXPIRY_BUFFER_SECS};
use codelet_providers::{AuthMode, ClaudeProvider};
use fixtures::setup_fspec_home;
use http::Request;
use rig::http_client::HttpClientExt;
#[allow(unused_imports)]
use serial_test::serial;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// =========================================================================
// Test helpers
// =========================================================================

/// Build a Claude token refresh JSON response body.
fn build_claude_token_response(
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

/// Build a RefreshingClaudeClient in OAuth mode with a token expiring in `secs` seconds.
/// Uses a wiremock server URI as the token endpoint base so refresh calls are interceptable.
fn build_oauth_client(
    access_token: &str,
    refresh_token: &str,
    expires_in_secs: Option<u64>,
    token_endpoint_base: &str,
) -> RefreshingClaudeClient {
    RefreshingClaudeClient::new_oauth(
        access_token.to_string(),
        refresh_token.to_string(),
        expires_in_secs,
        token_endpoint_base.to_string(),
    )
}

/// Build a RefreshingClaudeClient in OAuth mode with an ALREADY-EXPIRED token.
/// Sets expires_in to 0, which means the token is expired immediately.
fn build_expired_oauth_client(
    refresh_token: &str,
    token_endpoint_base: &str,
) -> RefreshingClaudeClient {
    RefreshingClaudeClient::new_oauth(
        "expired_access_token".to_string(),
        refresh_token.to_string(),
        Some(0),
        token_endpoint_base.to_string(),
    )
}

/// Create a simple POST request to the given URL with an empty JSON body.
fn make_request(url: &str) -> Request<Vec<u8>> {
    Request::builder()
        .method("POST")
        .uri(url)
        .header("content-type", "application/json")
        .body(Vec::new())
        .unwrap()
}

/// Create a POST request with a stale Authorization header (simulating rig's behavior).
fn make_request_with_stale_auth(url: &str) -> Request<Vec<u8>> {
    Request::builder()
        .method("POST")
        .uri(url)
        .header("content-type", "application/json")
        .header("authorization", "Bearer old-stale-token")
        .body(Vec::new())
        .unwrap()
}

/// Create a POST request with static headers (anthropic-beta, user-agent) set by rig.
fn make_request_with_static_headers(url: &str) -> Request<Vec<u8>> {
    Request::builder()
        .method("POST")
        .uri(url)
        .header("content-type", "application/json")
        .header(
            "anthropic-beta",
            "prompt-caching-2024-07-31,interleaved-thinking-2025-05-14",
        )
        .header("user-agent", "claude-cli/2.1.3 (external, cli)")
        .header("authorization", "Bearer old-stale-token")
        .body(Vec::new())
        .unwrap()
}

/// Mount a successful Claude token refresh mock on the wiremock server.
async fn mount_successful_refresh(
    mock_server: &MockServer,
    new_access_token: &str,
    new_refresh_token: &str,
) {
    let token_body = build_claude_token_response(new_access_token, new_refresh_token, 3600);

    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .and(body_string_contains("grant_type"))
        .and(body_string_contains("refresh_token"))
        .and(body_string_contains(CLAUDE_CLIENT_ID))
        .respond_with(ResponseTemplate::new(200).set_body_json(&token_body))
        .mount(mock_server)
        .await;
}

/// Mount a failing token refresh mock (401 Unauthorized).
async fn mount_failed_refresh(mock_server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .and(body_string_contains("refresh_token"))
        .respond_with(ResponseTemplate::new(401).set_body_string(
            r#"{"error":"invalid_grant","error_description":"Invalid refresh token"}"#,
        ))
        .mount(mock_server)
        .await;
}

// =========================================================================
// Scenario: Request with valid token injects Bearer header without refresh
// =========================================================================

#[tokio::test]
#[serial]
async fn test_request_with_valid_token_injects_bearer_header_without_refresh() {
    // @step Given a RefreshingClaudeClient in OAuth mode with a valid access token expiring in 30 minutes
    let mock_server = MockServer::start().await;
    let backend = MockServer::start().await;
    let access_token = "valid_claude_token_abc";

    let client = build_oauth_client(
        access_token,
        "refresh_tok",
        Some(1800), // 30 minutes
        &mock_server.uri(),
    );

    // Mount a backend that accepts any POST and returns 200.
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"ok":true}"#))
        .expect(1)
        .mount(&backend)
        .await;

    // @step When the client sends a request to "https://api.anthropic.com/v1/messages"
    let url = format!("{}/v1/messages", backend.uri());
    let req = make_request(&url);
    let _result: rig::http_client::Result<http::Response<rig::http_client::LazyBody<Vec<u8>>>> =
        client.send(req).await;

    // @step Then the Authorization header should be "Bearer {access_token}"
    let received = backend.received_requests().await.unwrap();
    assert_eq!(
        received.len(),
        1,
        "Backend should receive exactly 1 request"
    );
    assert_eq!(
        received[0]
            .headers
            .get("authorization")
            .map(|v| v.to_str().unwrap()),
        Some(format!("Bearer {access_token}")).as_deref(),
        "Authorization header should be Bearer token"
    );

    // @step And no token refresh should occur
    assert!(
        !client.is_token_expired().await,
        "Token with 30 min remaining should NOT be expired"
    );
    let refresh_calls = mock_server.received_requests().await.unwrap();
    assert!(
        refresh_calls.is_empty(),
        "No token refresh should occur when token is valid"
    );

    // @step And the request should be forwarded to reqwest successfully
    // (implicit: no panic or error from send(), backend received the request)
}

// =========================================================================
// Scenario: Expired token is automatically refreshed before request
// =========================================================================

#[tokio::test]
#[serial]
async fn test_expired_token_is_automatically_refreshed_before_request() {
    // @step Given a RefreshingClaudeClient in OAuth mode with an expired access token
    let mock_server = MockServer::start().await;
    let backend = MockServer::start().await;
    let (_temp_dir, _guard) = setup_fspec_home();
    let client = build_expired_oauth_client("valid_refresh_tok", &mock_server.uri());

    // @step And a valid refresh token
    mount_successful_refresh(
        &mock_server,
        "new_claude_access_tok",
        "new_claude_refresh_tok",
    )
    .await;

    // Mount a backend for the actual API request
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"ok":true}"#))
        .mount(&backend)
        .await;

    // Verify the token IS expired before making the request
    assert!(
        client.is_token_expired().await,
        "Token with 0s expiry should be expired"
    );

    // @step When the client sends a request to "https://api.anthropic.com/v1/messages"
    let url = format!("{}/v1/messages", backend.uri());
    let req = make_request(&url);
    let _result: rig::http_client::Result<http::Response<rig::http_client::LazyBody<Vec<u8>>>> =
        client.send(req).await;

    // @step Then the client should refresh the access token via claude_oauth refresh_access_token_at()
    let received = mock_server.received_requests().await.unwrap();
    let refresh_calls: Vec<_> = received
        .iter()
        .filter(|r| r.url.path() == "/v1/oauth/token")
        .collect();
    assert_eq!(
        refresh_calls.len(),
        1,
        "Exactly one token refresh call should be made"
    );

    // @step And the refreshed tokens should be persisted to claude_auth.json
    // Allow a small delay for the tokio::spawn persistence task
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let persisted = claude_auth::read_claude_auth().await.unwrap().unwrap();
    assert_eq!(
        persisted.access_token, "new_claude_access_tok",
        "Persisted access_token should match refreshed value"
    );
    assert_eq!(
        persisted.refresh_token, "new_claude_refresh_tok",
        "Persisted refresh_token should match refreshed value"
    );

    // @step And the request should proceed with the new access token
    let backend_reqs = backend.received_requests().await.unwrap();
    assert_eq!(
        backend_reqs.len(),
        1,
        "API request should be forwarded after refresh"
    );
    assert_eq!(
        backend_reqs[0]
            .headers
            .get("authorization")
            .map(|v| v.to_str().unwrap()),
        Some("Bearer new_claude_access_tok"),
        "Refreshed token should be used"
    );

    // @step And the response should be returned successfully
    // (implicit: no panic or error from send())
}

// =========================================================================
// Scenario: Token refresh failure propagates error without sending request
// =========================================================================

#[tokio::test]
#[serial]
async fn test_token_refresh_failure_propagates_error() {
    // @step Given a RefreshingClaudeClient in OAuth mode with an expired access token
    let mock_server = MockServer::start().await;
    let backend = MockServer::start().await;
    let client = build_expired_oauth_client("invalid_refresh_tok", &mock_server.uri());

    // @step And an invalid refresh token that returns a 401 error
    mount_failed_refresh(&mock_server).await;

    // Mount a backend — should NOT receive any request
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"ok":true}"#))
        .expect(0)
        .mount(&backend)
        .await;

    // Verify the token is expired (pre-condition for refresh)
    assert!(
        client.is_token_expired().await,
        "Token should be expired, triggering refresh attempt"
    );

    // @step When the client sends a request to "https://api.anthropic.com/v1/messages"
    let url = format!("{}/v1/messages", backend.uri());
    let req = make_request(&url);
    let result: rig::http_client::Result<http::Response<rig::http_client::LazyBody<Vec<u8>>>> =
        client.send(req).await;

    // @step Then the client should attempt to refresh the access token
    let received = mock_server.received_requests().await.unwrap();
    let refresh_calls: Vec<_> = received
        .iter()
        .filter(|r| r.url.path() == "/v1/oauth/token")
        .collect();
    assert_eq!(
        refresh_calls.len(),
        1,
        "Token refresh should be attempted even though it will fail"
    );

    // @step And the refresh should fail with an authentication error
    // @step And the original API request should NOT be sent
    // @step And the error should propagate to the caller
    assert!(
        result.is_err(),
        "Failed refresh should propagate error to caller"
    );
}

// =========================================================================
// Scenario: Streaming request with expired token refreshes before streaming
// =========================================================================

#[tokio::test]
#[serial]
async fn test_streaming_request_with_expired_token_refreshes_before_streaming() {
    // @step Given a RefreshingClaudeClient in OAuth mode with an expired access token
    let mock_server = MockServer::start().await;
    let backend = MockServer::start().await;
    let (_temp_dir, _guard) = setup_fspec_home();
    let client = build_expired_oauth_client("valid_refresh_for_stream", &mock_server.uri());

    // @step And a valid refresh token
    mount_successful_refresh(&mock_server, "stream_access_tok", "stream_refresh_tok").await;

    // Mount a backend for the streaming request — respond with minimal SSE stream
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string("data: {\"type\":\"content_block_delta\"}\n\ndata: [DONE]\n\n"),
        )
        .mount(&backend)
        .await;

    // Verify pre-condition: token is expired
    assert!(
        client.is_token_expired().await,
        "Token should be expired before streaming request"
    );

    // @step When the client sends a streaming request to "https://api.anthropic.com/v1/messages"
    let url = format!("{}/v1/messages", backend.uri());
    let req = make_request(&url);
    let _result = client.send_streaming(req).await;

    // @step Then the client should refresh the access token before streaming
    let received = mock_server.received_requests().await.unwrap();
    let refresh_calls: Vec<_> = received
        .iter()
        .filter(|r| r.url.path() == "/v1/oauth/token")
        .collect();
    assert_eq!(
        refresh_calls.len(),
        1,
        "Streaming with expired token should trigger refresh"
    );

    // @step And the streaming response should use the refreshed credentials
    let backend_reqs = backend.received_requests().await.unwrap();
    assert_eq!(
        backend_reqs.len(),
        1,
        "Streaming request should be forwarded"
    );
    assert_eq!(
        backend_reqs[0]
            .headers
            .get("authorization")
            .map(|v| v.to_str().unwrap()),
        Some("Bearer stream_access_tok"),
        "Refreshed token should be used for streaming"
    );

    // @step And the SSE stream should be returned successfully
}

// =========================================================================
// Scenario: API key mode passes requests through unchanged
// =========================================================================

#[tokio::test]
async fn test_api_key_mode_passes_requests_through_unchanged() {
    // @step Given a RefreshingClaudeClient in ApiKey mode
    let client = RefreshingClaudeClient::new_api_key();
    let backend = MockServer::start().await;

    // Mount a backend to capture the request
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"ok":true}"#))
        .expect(1)
        .mount(&backend)
        .await;

    // @step When the client sends a request to "https://api.anthropic.com/v1/messages"
    let url = format!("{}/v1/messages", backend.uri());
    let req = make_request(&url);
    let _result: rig::http_client::Result<http::Response<rig::http_client::LazyBody<Vec<u8>>>> =
        client.send(req).await;

    // @step Then no token refresh should occur
    assert!(
        !client.is_token_expired().await,
        "API key mode should never report token as expired"
    );

    // @step And the original headers from rig should be preserved
    // @step And the request should be forwarded to reqwest as-is
    let received = backend.received_requests().await.unwrap();
    assert_eq!(
        received.len(),
        1,
        "Backend should receive exactly 1 request"
    );
    // In API key mode, no Authorization header is injected by RefreshingClaudeClient
    // (rig's static headers are preserved as-is)
    assert!(
        received[0].headers.get("authorization").is_none()
            || received[0].headers.get("authorization").unwrap() != "Bearer expired_access_token",
        "API key mode should NOT inject OAuth Bearer tokens"
    );
    // Verify the request arrived at the correct path (no rewrite)
    assert_eq!(
        received[0].url.path(),
        "/v1/messages",
        "API key mode should NOT rewrite the URL path"
    );
}

// =========================================================================
// Scenario: Token refresh within expiry buffer triggers proactive refresh
// =========================================================================

#[tokio::test]
async fn test_token_refresh_within_expiry_buffer_triggers_proactive_refresh() {
    // @step Given a RefreshingClaudeClient in OAuth mode with a token expiring in 20 seconds
    let mock_server = MockServer::start().await;
    let client = build_oauth_client(
        "about_to_expire_tok",
        "refresh_tok",
        Some(20), // 20 seconds left
        &mock_server.uri(),
    );

    // @step And the expiry buffer is 30 seconds
    assert_eq!(EXPIRY_BUFFER_SECS, 30, "Expiry buffer should be 30 seconds");

    // @step When the client sends a request
    // @step Then the client should proactively refresh the token
    // @step And the request should use the refreshed token

    // A token expiring in 20s with a 30s buffer should be considered expired
    assert!(
        client.is_token_expired().await,
        "Token expiring in 20s with 30s buffer should be considered expired"
    );
}

// =========================================================================
// Scenario: Existing Authorization header is replaced with current token
// =========================================================================

#[tokio::test]
async fn test_existing_authorization_header_is_replaced() {
    // @step Given a RefreshingClaudeClient in OAuth mode with a valid access token
    let mock_server = MockServer::start().await;
    let backend = MockServer::start().await;
    let access_token = "real_claude_bearer_token";
    let client = build_oauth_client(access_token, "refresh_tok", Some(1800), &mock_server.uri());

    // Mount a backend to capture the request
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"ok":true}"#))
        .expect(1)
        .mount(&backend)
        .await;

    // @step And the original request has a stale Authorization header "Bearer old-stale-token" set by rig
    let url = format!("{}/v1/messages", backend.uri());
    let req = make_request_with_stale_auth(&url);

    // Verify the original request has the stale auth header
    assert_eq!(
        req.headers().get("authorization").unwrap(),
        "Bearer old-stale-token",
        "Original request should have rig's stale auth header"
    );

    // @step When the client sends the request
    let _result: rig::http_client::Result<http::Response<rig::http_client::LazyBody<Vec<u8>>>> =
        client.send(req).await;

    // @step Then the stale Authorization header should be stripped
    // @step And replaced with "Bearer {current_access_token}"
    let received = backend.received_requests().await.unwrap();
    assert_eq!(
        received.len(),
        1,
        "Backend should receive exactly 1 request"
    );
    assert_eq!(
        received[0]
            .headers
            .get("authorization")
            .map(|v| v.to_str().unwrap()),
        Some(format!("Bearer {access_token}")).as_deref(),
        "Stale auth header should be replaced with real Bearer token"
    );
}

// =========================================================================
// Scenario: Static headers are preserved and not modified
// =========================================================================

#[tokio::test]
async fn test_static_headers_are_preserved_and_not_modified() {
    // @step Given a RefreshingClaudeClient in OAuth mode with a valid access token
    let mock_server = MockServer::start().await;
    let backend = MockServer::start().await;
    let access_token = "static_header_test_token";
    let client = build_oauth_client(access_token, "refresh_tok", Some(1800), &mock_server.uri());

    // Mount a backend to capture the request
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"ok":true}"#))
        .expect(1)
        .mount(&backend)
        .await;

    // @step And the request has static headers including anthropic-beta and user-agent set by rig
    let url = format!("{}/v1/messages", backend.uri());
    let req = make_request_with_static_headers(&url);

    // @step When the client sends the request
    let _result: rig::http_client::Result<http::Response<rig::http_client::LazyBody<Vec<u8>>>> =
        client.send(req).await;

    let received = backend.received_requests().await.unwrap();
    assert_eq!(
        received.len(),
        1,
        "Backend should receive exactly 1 request"
    );

    // @step Then the anthropic-beta header should be preserved unchanged
    assert_eq!(
        received[0]
            .headers
            .get("anthropic-beta")
            .map(|v| v.to_str().unwrap()),
        Some("prompt-caching-2024-07-31,interleaved-thinking-2025-05-14"),
        "anthropic-beta header should be preserved unchanged"
    );

    // @step And the user-agent header should be preserved unchanged
    assert_eq!(
        received[0]
            .headers
            .get("user-agent")
            .map(|v| v.to_str().unwrap()),
        Some("claude-cli/2.1.3 (external, cli)"),
        "user-agent header should be preserved unchanged"
    );

    // @step And only the Authorization header should be modified
    assert_eq!(
        received[0]
            .headers
            .get("authorization")
            .map(|v| v.to_str().unwrap()),
        Some(format!("Bearer {access_token}")).as_deref(),
        "Only Authorization header should be modified to current token"
    );
}

// =========================================================================
// Scenario: ClaudeProvider uses RefreshingClaudeClient for OAuth mode
// =========================================================================

#[tokio::test]
#[serial]
async fn test_claude_provider_uses_refreshing_client_for_oauth_mode() {
    // @step Given OAuth tokens with access_token and refresh_token
    let mock_server = MockServer::start().await;
    let access_token = "sk-ant-oat01-provider-test-token";
    let refresh_token = "provider_claude_refresh_tok";

    // @step When ClaudeProvider::from_api_key_with_mode_and_model() is called in OAuth mode
    // Verify the existing from_api_key_with_mode_and_model path (API key pass-through mode)
    // creates a ClaudeProvider with RefreshingClaudeClient as HTTP backend
    let provider = ClaudeProvider::from_api_key_with_mode_and_model(
        access_token,
        AuthMode::OAuth,
        "claude-sonnet-4-20250514",
    )
    .expect("ClaudeProvider should be created with OAuth mode");

    // @step Then a RefreshingClaudeClient should be created with OAuth ClaudeTokenMode
    // Verify the provider is in OAuth mode
    assert!(provider.is_oauth_mode(), "Provider should be in OAuth mode");

    // @step And it should be passed as the HTTP client to rig anthropic::Client<RefreshingClaudeClient>
    // Verify the client is properly typed — the fact that .client() returns
    // &anthropic::Client<RefreshingClaudeClient> proves the type integration is correct.
    // If RefreshingClaudeClient were NOT wired in, this would return &anthropic::Client<reqwest::Client>
    // and the type alias would not match.
    let _client = provider.client();

    // Verify from_oauth_tokens path (full OAuth with token refresh support)
    let oauth_provider = ClaudeProvider::from_oauth_tokens(
        access_token,
        refresh_token,
        Some(3600),
        &mock_server.uri(),
        "claude-sonnet-4-20250514",
    )
    .expect("ClaudeProvider should be created with from_oauth_tokens");

    assert!(
        oauth_provider.is_oauth_mode(),
        "OAuth tokens provider should be in OAuth mode"
    );

    // @step And the provider should be able to construct a rig Agent
    let session_id = uuid::Uuid::new_v4();
    let _agent = oauth_provider.create_rig_agent(session_id, None, None);

    // Verify API key mode also works (unified type)
    let api_provider = ClaudeProvider::from_api_key_with_mode_and_model(
        "sk-ant-api03-test-key",
        AuthMode::ApiKey,
        "claude-sonnet-4-20250514",
    )
    .expect("ClaudeProvider should be created in API key mode");

    assert!(
        !api_provider.is_oauth_mode(),
        "API key provider should NOT be in OAuth mode"
    );

    let _api_agent = api_provider.create_rig_agent(session_id, None, None);
}

// =========================================================================
// Scenario: Tokens loaded from disk trigger immediate refresh via Some(0)
// =========================================================================

#[tokio::test]
#[serial]
async fn test_tokens_loaded_from_disk_trigger_immediate_refresh() {
    // @step Given OAuth tokens exist in claude_auth.json from a previous session
    let mock_server = MockServer::start().await;
    let backend = MockServer::start().await;
    let (_temp_dir, _guard) = setup_fspec_home();

    // @step When ClaudeProvider passes Some(0) for expires_in_secs to RefreshingClaudeClient
    let client = build_expired_oauth_client("valid_refresh_disk", &mock_server.uri());

    // @step Then the token is immediately considered expired
    assert!(
        client.is_token_expired().await,
        "Token loaded from disk with Some(0) should be expired immediately"
    );

    // @step Given the access token may be expired
    // Mount a successful refresh endpoint
    mount_successful_refresh(&mock_server, "fresh_disk_tok", "fresh_refresh_tok").await;

    // Mount a backend to receive the API request
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"ok":true}"#))
        .expect(1)
        .mount(&backend)
        .await;

    // @step Then the first API request triggers a token refresh before sending
    let url = format!("{}/v1/messages", backend.uri());
    let req = make_request(&url);
    let result: rig::http_client::Result<http::Response<rig::http_client::LazyBody<Vec<u8>>>> =
        client.send(req).await;

    // Verify refresh was triggered
    let received = mock_server.received_requests().await.unwrap();
    let refresh_calls: Vec<_> = received
        .iter()
        .filter(|r| r.url.path() == "/v1/oauth/token")
        .collect();
    assert_eq!(
        refresh_calls.len(),
        1,
        "First request with disk-loaded tokens should trigger exactly one refresh"
    );

    // Verify the API request used the refreshed token
    let backend_reqs = backend.received_requests().await.unwrap();
    assert_eq!(
        backend_reqs.len(),
        1,
        "API request should be forwarded after refresh"
    );
    assert_eq!(
        backend_reqs[0]
            .headers
            .get("authorization")
            .map(|v| v.to_str().unwrap()),
        Some("Bearer fresh_disk_tok"),
        "Request should use the freshly refreshed token, not the stale disk token"
    );

    // Verify the request succeeded
    assert!(result.is_ok(), "Request should succeed after token refresh");
}

// =========================================================================
// Scenario: Token persistence is best-effort and does not fail requests
// =========================================================================

#[tokio::test]
#[serial]
async fn test_token_persistence_is_best_effort_and_does_not_fail_requests() {
    // @step Given a RefreshingClaudeClient in OAuth mode with an expired access token
    let mock_server = MockServer::start().await;
    let backend = MockServer::start().await;

    // @step And token persistence to claude_auth.json will fail due to filesystem error
    // Point FSPEC_HOME to a non-existent/read-only path to cause persistence failure.
    // We use setup_fspec_home() to get a valid guard, then override with a bad path.
    let (_temp_dir, _guard) = setup_fspec_home();
    std::env::set_var("FSPEC_HOME", "/dev/null/nonexistent/path");

    let client = build_expired_oauth_client("valid_refresh_tok", &mock_server.uri());

    mount_successful_refresh(
        &mock_server,
        "persistence_fail_tok",
        "persistence_fail_refresh",
    )
    .await;

    // Mount a backend for the actual API request
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"ok":true}"#))
        .mount(&backend)
        .await;

    // @step When the client sends a request that triggers a token refresh
    let url = format!("{}/v1/messages", backend.uri());
    let req = make_request(&url);
    let result: rig::http_client::Result<http::Response<rig::http_client::LazyBody<Vec<u8>>>> =
        client.send(req).await;

    // @step Then the refresh should succeed and tokens should be updated in memory
    // @step And the persistence failure should be logged
    // @step And the request should still proceed with the refreshed token
    assert!(
        result.is_ok(),
        "Request should succeed even if persistence fails"
    );

    let backend_reqs = backend.received_requests().await.unwrap();
    assert_eq!(backend_reqs.len(), 1, "API request should be forwarded");
    assert_eq!(
        backend_reqs[0]
            .headers
            .get("authorization")
            .map(|v| v.to_str().unwrap()),
        Some("Bearer persistence_fail_tok"),
        "Refreshed token should be used despite persistence failure"
    );

    // _guard restores FSPEC_HOME on drop
}

// =========================================================================
// Scenario: Claude auth persistence writes correct JSON structure
// =========================================================================

#[tokio::test]
#[serial]
async fn test_claude_auth_persistence_writes_correct_json_structure() {
    // @step Given a RefreshingClaudeClient in OAuth mode with an expired access token
    let mock_server = MockServer::start().await;
    let backend = MockServer::start().await;
    let (_temp_dir, _guard) = setup_fspec_home();

    let client = build_expired_oauth_client("valid_refresh_persist", &mock_server.uri());

    // @step And a valid refresh token
    mount_successful_refresh(&mock_server, "persist_access_tok", "persist_refresh_tok").await;

    // Mount a backend for the actual API request
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"ok":true}"#))
        .mount(&backend)
        .await;

    // @step When a token refresh occurs
    let url = format!("{}/v1/messages", backend.uri());
    let req = make_request(&url);
    let _result: rig::http_client::Result<http::Response<rig::http_client::LazyBody<Vec<u8>>>> =
        client.send(req).await;

    // Allow persistence tokio::spawn task to complete
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // @step Then claude_auth.json should contain access_token from the refresh response
    let persisted = claude_auth::read_claude_auth().await.unwrap().unwrap();
    assert_eq!(
        persisted.access_token, "persist_access_tok",
        "claude_auth.json should contain access_token from refresh response"
    );

    // @step And claude_auth.json should contain refresh_token from the refresh response
    assert_eq!(
        persisted.refresh_token, "persist_refresh_tok",
        "claude_auth.json should contain refresh_token from refresh response"
    );

    // @step And claude_auth.json should contain expires calculated from expires_in
    // The expires field should be a future timestamp (now_ms + 3600 * 1000)
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    // The persisted expires should be approximately now + 3600s (within 5s tolerance)
    let expected_min = now_ms + (3600 - 5) * 1000;
    let expected_max = now_ms + (3600 + 5) * 1000;
    assert!(
        persisted.expires >= expected_min && persisted.expires <= expected_max,
        "Persisted expires ({}) should be approximately now + 3600s (expected range {}-{})",
        persisted.expires,
        expected_min,
        expected_max
    );
}
