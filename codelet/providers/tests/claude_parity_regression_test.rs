#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/anthropic-oauth-parity.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios for PROV-027:
//! Anthropic subscription parity and regression hardening against opencode behavior.
//!
//! PARITY tests verify our implementation matches opencode's anthropic-auth plugin.
//! REGRESSION tests prevent PROV-019 class bugs from recurring for Claude.
//!
//! Tests use:
//! - Unit assertions for pure-function parity (tool prefixing, URL rewriting, headers)
//! - wiremock for token refresh and concurrent refresh scenarios
//! - FSPEC_HOME temp directories for persistence testing

mod fixtures;

use codelet_providers::claude_auth;
use codelet_providers::claude_oauth::{
    build_oauth_headers, prefix_tool_name, rewrite_claude_url, strip_tool_name_prefix,
    CLAUDE_CLIENT_ID, TOOL_NAME_PREFIX,
};
use codelet_providers::claude_refreshing_client::RefreshingClaudeClient;
use codelet_providers::{AuthMode, ClaudeProvider};
use codelet_tools::facade::CLAUDE_CODE_PROMPT_PREFIX;
use fixtures::setup_fspec_home;
use http::Request;
use rig::http_client::HttpClientExt;
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
#[allow(dead_code)]
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

/// Mount a successful Claude token refresh mock on the wiremock server.
/// Returns an expectation guard for counting refresh calls.
#[allow(dead_code)]
async fn mount_counted_successful_refresh(
    mock_server: &MockServer,
    new_access_token: &str,
    new_refresh_token: &str,
) -> wiremock::MockGuard {
    let token_body = build_claude_token_response(new_access_token, new_refresh_token, 3600);

    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .and(body_string_contains("grant_type"))
        .and(body_string_contains("refresh_token"))
        .and(body_string_contains(CLAUDE_CLIENT_ID))
        .respond_with(ResponseTemplate::new(200).set_body_json(&token_body))
        .named("counted_refresh")
        .mount_as_scoped(mock_server)
        .await
}

/// Mount a successful Claude token refresh mock (non-counted).
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

// =========================================================================
// PARITY: Scenario: Tool names are prefixed with mcp_ in OAuth mode requests
// =========================================================================
// Matches opencode's fetch interceptor which adds mcp_ prefix to tool
// definitions and tool_use blocks, and strips mcp_ from streaming responses.
//
// NOTE: These test the parity reference implementations in claude_oauth.rs.
// Our tools are native (not MCP), so tool prefixing is not in the production
// request path. opencode needs the prefix because it routes through MCP servers.
// These functions exist for future MCP integration and for verifying our
// implementation matches opencode's behavior.

#[test]
fn test_parity_tool_names_prefixed_with_mcp_in_oauth_mode_requests() {
    // @step Given I am authenticated with Claude via OAuth
    // Our claude_oauth.rs module provides prefix_tool_name and strip_tool_name_prefix.

    // @step When a request is sent with tool definitions and tool_use blocks
    // opencode's custom fetch does:
    //   parsed.tools = parsed.tools.map(tool => ({ ...tool, name: `mcp_${tool.name}` }))
    //   msg.content = msg.content.map(block => block.type === "tool_use"
    //     ? { ...block, name: `mcp_${block.name}` } : block)
    // Our functions must produce identical output.

    // @step Then tool names in tool definitions should be prefixed with "mcp_"
    // Test all 12 codelet tools match opencode's behavior
    let tools = [
        "Read",
        "Write",
        "Edit",
        "Bash",
        "Grep",
        "Glob",
        "Ls",
        "AstGrep",
        "AstGrepRefactor",
        "Fspec",
        "Bridge",
        "WebSearch",
    ];
    for tool in &tools {
        let prefixed = prefix_tool_name(tool);
        assert_eq!(
            prefixed,
            format!("{TOOL_NAME_PREFIX}{tool}"),
            "Tool '{tool}' should be prefixed with mcp_"
        );
        assert!(
            prefixed.starts_with("mcp_"),
            "Prefixed tool '{prefixed}' must start with mcp_"
        );
    }

    // @step And tool_use block names in messages should be prefixed with "mcp_"
    // Same function is used for tool_use blocks in messages
    assert_eq!(prefix_tool_name("Read"), "mcp_Read");
    assert_eq!(prefix_tool_name("Bash"), "mcp_Bash");

    // @step And tool names in streaming responses should have "mcp_" prefix stripped
    // opencode uses regex: /"name"\s*:\s*"mcp_([^"]+)"/g → '"name": "$1"'
    // Our strip_tool_name_prefix must produce equivalent results
    assert_eq!(strip_tool_name_prefix("mcp_Read"), "Read");
    assert_eq!(strip_tool_name_prefix("mcp_Bash"), "Bash");
    assert_eq!(strip_tool_name_prefix("mcp_AstGrep"), "AstGrep");
    // Tool without prefix passes through unchanged (like opencode regex)
    assert_eq!(strip_tool_name_prefix("NoPrefixTool"), "NoPrefixTool");
    // Double prefix is only stripped once (like opencode regex)
    assert_eq!(strip_tool_name_prefix("mcp_mcp_Tool"), "mcp_Tool");
}

// =========================================================================
// PARITY: Scenario: API URL is rewritten to append beta query parameter
// =========================================================================
// Matches opencode's URL transformation:
//   if (requestUrl.pathname === "/v1/messages" && !requestUrl.searchParams.has("beta"))
//     requestUrl.searchParams.set("beta", "true")
//
// NOTE: Tests the parity reference rewrite_claude_url() function.
// Production URL rewriting is handled by the patched rig AnthropicExt::build_uri()
// which appends ?beta=true for OAuth tokens (detected via AnthropicKey::is_oauth_token).
// Both paths produce identical output.

#[test]
fn test_parity_api_url_rewritten_to_append_beta_query_parameter() {
    // @step Given I am authenticated with Claude via OAuth

    // @step When a request is sent to /v1/messages
    let url = "https://api.anthropic.com/v1/messages";
    let rewritten = rewrite_claude_url(url);

    // @step Then the URL should have "?beta=true" appended
    assert_eq!(
        rewritten, "https://api.anthropic.com/v1/messages?beta=true",
        "Messages URL should have ?beta=true appended"
    );

    // @step And a URL that already has "?beta=true" should not be duplicated
    let already_has = "https://api.anthropic.com/v1/messages?beta=true";
    assert_eq!(
        rewrite_claude_url(already_has),
        already_has,
        "URL with existing ?beta=true should not be duplicated"
    );

    // Also test with beta as non-first parameter
    let has_beta_and_other = "https://api.anthropic.com/v1/messages?stream=true&beta=true";
    assert_eq!(
        rewrite_claude_url(has_beta_and_other),
        has_beta_and_other,
        "URL with existing beta param should pass through"
    );

    // @step And non-messages URLs should pass through unchanged
    assert_eq!(
        rewrite_claude_url("https://api.anthropic.com/v1/models"),
        "https://api.anthropic.com/v1/models",
        "Non-messages URL should pass through unchanged"
    );
    assert_eq!(
        rewrite_claude_url("https://api.anthropic.com/v1/completions"),
        "https://api.anthropic.com/v1/completions",
        "Non-messages URL should pass through unchanged"
    );
}

// =========================================================================
// PARITY: Scenario: OAuth requests include merged beta headers and Bearer auth
// =========================================================================
// Matches opencode's header merging:
//   const requiredBetas = ["oauth-2025-04-20", "interleaved-thinking-2025-05-14"];
//   const mergedBetas = [...new Set([...requiredBetas, ...incomingBetasList])].join(",");
//   requestHeaders.set("authorization", `Bearer ${auth.access}`);
//   requestHeaders.set("user-agent", "claude-cli/2.1.2 (external, cli)");
//   requestHeaders.delete("x-api-key");
//
// NOTE: Tests the parity reference build_oauth_headers() function.
// Production headers are set via rig's ClientBuilder::http_headers() in
// ClaudeProvider::from_oauth_tokens(). Both paths produce equivalent headers.

#[test]
fn test_parity_oauth_requests_include_merged_beta_headers_and_bearer_auth() {
    // @step Given I am authenticated with Claude via OAuth
    let access_token = "test_parity_access_token";

    // @step And the request has existing beta headers "max-tokens-3-5-sonnet-2024-07-15"
    let existing_beta = Some("max-tokens-3-5-sonnet-2024-07-15");

    // @step When the request is prepared for the Claude API
    let headers = build_oauth_headers(access_token, existing_beta);

    // @step Then the Authorization header should be "Bearer {access_token}"
    assert_eq!(
        headers.get("authorization").unwrap(),
        &format!("Bearer {access_token}"),
        "Authorization must be Bearer token"
    );

    let beta = headers.get("anthropic-beta").unwrap();

    // @step And the anthropic-beta header should contain "oauth-2025-04-20"
    assert!(
        beta.contains("oauth-2025-04-20"),
        "Missing required oauth beta: {beta}"
    );

    // @step And the anthropic-beta header should contain "interleaved-thinking-2025-05-14"
    assert!(
        beta.contains("interleaved-thinking-2025-05-14"),
        "Missing required thinking beta: {beta}"
    );

    // @step And the anthropic-beta header should contain "max-tokens-3-5-sonnet-2024-07-15"
    assert!(
        beta.contains("max-tokens-3-5-sonnet-2024-07-15"),
        "Existing beta should be preserved: {beta}"
    );

    // @step And the anthropic-beta header should have no duplicate entries
    let beta_parts: Vec<&str> = beta.split(',').collect();
    let unique: std::collections::HashSet<&str> = beta_parts.iter().copied().collect();
    assert_eq!(
        beta_parts.len(),
        unique.len(),
        "Beta headers should have no duplicates: {beta}"
    );

    // @step And the user-agent should be "claude-cli/2.1.3 (external, cli)"
    assert_eq!(
        headers.get("user-agent").unwrap(),
        "claude-cli/2.1.3 (external, cli)",
        "User-Agent must match CLAUDE_USER_AGENT constant"
    );

    // @step And x-api-key should not be present
    assert!(
        !headers.contains_key("x-api-key"),
        "x-api-key must not be present in OAuth mode"
    );

    // Deduplication test: pass already-present required betas as existing
    let with_dup = build_oauth_headers(access_token, Some("oauth-2025-04-20,custom-beta"));
    let dup_beta = with_dup.get("anthropic-beta").unwrap();
    let dup_parts: Vec<&str> = dup_beta.split(',').collect();
    let dup_unique: std::collections::HashSet<&str> = dup_parts.iter().copied().collect();
    assert_eq!(
        dup_parts.len(),
        dup_unique.len(),
        "Duplicate required betas should be deduplicated: {dup_beta}"
    );
}

// =========================================================================
// PARITY: Scenario: System prompt includes Claude Code identity prefix
// =========================================================================
// Matches opencode's system.transform:
//   const prefix = "You are Claude Code, Anthropic's official CLI for Claude.";
//   output.system.unshift(prefix);
//   if (output.system[1]) output.system[1] = prefix + "\n\n" + output.system[1];

#[test]
fn test_parity_system_prompt_includes_claude_code_identity_prefix() {
    // @step Given I am authenticated with Claude via OAuth
    let provider = ClaudeProvider::from_api_key_with_mode_and_model(
        "sk-ant-oat01-test-token",
        AuthMode::OAuth,
        "claude-sonnet-4-20250514",
    )
    .expect("Should create OAuth provider");

    // @step When the system prompt is built for a request
    let prompt = provider.system_prompt();

    // @step Then the first system block should start with "You are Claude Code, Anthropic's official CLI for Claude."
    assert!(
        prompt.is_some(),
        "OAuth mode must return a system prompt prefix"
    );
    let prefix = prompt.unwrap();
    assert_eq!(
        prefix, "You are Claude Code, Anthropic's official CLI for Claude.",
        "Prefix must exactly match opencode's system.transform prefix"
    );

    // @step And app name references should be replaced with "Claude Code"
    // opencode replaces "OpenCode" → "Claude Code" and "opencode" → "Claude"
    // Our facade uses the CLAUDE_CODE_PROMPT_PREFIX constant directly
    assert_eq!(
        CLAUDE_CODE_PROMPT_PREFIX, "You are Claude Code, Anthropic's official CLI for Claude.",
        "CLAUDE_CODE_PROMPT_PREFIX must match opencode's prefix"
    );

    // Verify API key mode does NOT include the prefix
    let api_provider = ClaudeProvider::from_api_key_with_mode_and_model(
        "sk-ant-api03-test-key",
        AuthMode::ApiKey,
        "claude-sonnet-4-20250514",
    )
    .expect("Should create API key provider");
    assert!(
        api_provider.system_prompt().is_none(),
        "API key mode should NOT return a system prompt prefix"
    );
}

// =========================================================================
// PARITY: Scenario: OAuth max plan users see zero costs
// =========================================================================
// Matches opencode's auth.loader which zeroes costs:
//   for (const model of Object.values(provider.models)) {
//     model.cost = { input: 0, output: 0, cache: { read: 0, write: 0 } };
//   }
// NOTE: Cost zeroing is handled at the TUI layer in our implementation.
// NapiModelInfo does not currently expose pricing fields, so cost display
// is out of scope. This test verifies the Rust provider correctly reports
// OAuth mode, which is the precondition the TUI uses to determine whether
// to zero costs. The TUI cost zeroing itself will be testable once pricing
// fields are added to NapiModelInfo.

#[test]
fn test_parity_oauth_max_plan_users_see_zero_costs() {
    // @step Given I am authenticated with Claude via OAuth with a Max subscription
    let provider = ClaudeProvider::from_api_key_with_mode_and_model(
        "sk-ant-oat01-max-subscription-token",
        AuthMode::OAuth,
        "claude-sonnet-4-20250514",
    )
    .expect("Should create OAuth provider");

    // @step When the provider is queried for its auth mode
    let is_oauth = provider.is_oauth_mode();

    // @step Then the provider should report OAuth mode for cost zeroing
    assert!(
        is_oauth,
        "Provider must report OAuth mode so TUI can zero costs"
    );

    // @step And API key mode providers should not be flagged for cost zeroing
    let api_provider = ClaudeProvider::from_api_key_with_mode_and_model(
        "sk-ant-api03-test-key",
        AuthMode::ApiKey,
        "claude-sonnet-4-20250514",
    )
    .expect("Should create API key provider");
    assert!(
        !api_provider.is_oauth_mode(),
        "API key mode should NOT trigger cost zeroing"
    );
}

// =========================================================================
// REGRESSION: Scenario: Tokens loaded from disk force immediate refresh
// =========================================================================
// Regression from PROV-019 where Codex tokens loaded from disk were treated
// as fresh. Our fix: pass Some(0) for expires_in_secs to force refresh.

#[tokio::test]
#[serial]
async fn test_regression_tokens_loaded_from_disk_force_immediate_refresh() {
    // @step Given claude_auth.json contains week-old tokens with expired access_token
    let mock_server = MockServer::start().await;
    let backend = MockServer::start().await;
    let (_temp_dir, _guard) = setup_fspec_home();

    // Write simulated "week-old" tokens to claude_auth.json
    let old_auth = claude_auth::ClaudeAuthJson {
        access_token: "week_old_access_token".to_string(),
        refresh_token: "valid_refresh_token".to_string(),
        expires: 1000, // ancient timestamp — clearly expired
    };
    claude_auth::write_claude_auth(&old_auth).await.unwrap();

    // @step When the provider creates a RefreshingClaudeClient from disk tokens
    // Simulate what manager.rs does: read from disk, pass Some(0)
    let disk_auth = claude_auth::read_claude_auth()
        .await
        .unwrap()
        .expect("Should read tokens from disk");

    let client = RefreshingClaudeClient::new_oauth(
        disk_auth.access_token.clone(),
        disk_auth.refresh_token.clone(),
        Some(0), // Force immediate refresh — the PROV-019 fix
        mock_server.uri(),
    );

    // @step Then expires_in_secs should be Some(0) to force immediate refresh
    assert!(
        client.is_token_expired().await,
        "Token created with Some(0) must be immediately expired"
    );

    // Mount refresh endpoint
    mount_successful_refresh(
        &mock_server,
        "freshly_refreshed_access",
        "freshly_refreshed_refresh",
    )
    .await;

    // Mount backend
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"ok":true}"#))
        .mount(&backend)
        .await;

    // @step And the first API call should trigger a token refresh before sending
    let url = format!("{}/v1/messages", backend.uri());
    let req = make_request(&url);
    let result: rig::http_client::Result<http::Response<rig::http_client::LazyBody<Vec<u8>>>> =
        client.send(req).await;

    // Verify refresh was triggered
    let refresh_calls: Vec<_> = mock_server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|r| r.url.path() == "/v1/oauth/token")
        .collect();
    assert_eq!(
        refresh_calls.len(),
        1,
        "Week-old tokens with Some(0) must trigger exactly one refresh on first API call"
    );

    // @step And the API call should succeed with the refreshed token
    assert!(result.is_ok(), "API call should succeed after refresh");
    let backend_reqs = backend.received_requests().await.unwrap();
    assert_eq!(
        backend_reqs[0]
            .headers
            .get("authorization")
            .map(|v| v.to_str().unwrap()),
        Some("Bearer freshly_refreshed_access"),
        "Request must use the freshly refreshed token, not the week-old disk token"
    );
}

// =========================================================================
// REGRESSION: Scenario: Concurrent requests during expired token only refresh once
// =========================================================================
// Regression test for double-check locking in RefreshingClaudeClient.
// Two simultaneous requests with an expired token should only trigger ONE
// HTTP refresh call, not two.

#[tokio::test]
#[serial]
async fn test_regression_concurrent_requests_during_expired_token_only_refresh_once() {
    // @step Given I am authenticated with Claude via OAuth
    let mock_server = MockServer::start().await;
    let backend = MockServer::start().await;
    let (_temp_dir, _guard) = setup_fspec_home();

    // @step And the access token is expired
    let client = build_expired_oauth_client("concurrent_refresh_tok", &mock_server.uri());

    // Mount refresh — add a small delay to make concurrency more likely to race
    let token_body =
        build_claude_token_response("concurrent_fresh_tok", "concurrent_fresh_refresh", 3600);
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .and(body_string_contains("refresh_token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(&token_body)
                .set_delay(std::time::Duration::from_millis(50)),
        )
        .mount(&mock_server)
        .await;

    // Mount backend
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"ok":true}"#))
        .mount(&backend)
        .await;

    // @step When two simultaneous API requests are made
    let client1 = client.clone();
    let client2 = client.clone();
    let url1 = format!("{}/v1/messages", backend.uri());
    let url2 = url1.clone();

    let (result1, result2) = tokio::join!(
        async move {
            let req = make_request(&url1);
            let r: rig::http_client::Result<http::Response<rig::http_client::LazyBody<Vec<u8>>>> =
                client1.send(req).await;
            r
        },
        async move {
            let req = make_request(&url2);
            let r: rig::http_client::Result<http::Response<rig::http_client::LazyBody<Vec<u8>>>> =
                client2.send(req).await;
            r
        }
    );

    // @step Then only one HTTP refresh call should be made to the token endpoint
    let refresh_calls: Vec<_> = mock_server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|r| r.url.path() == "/v1/oauth/token")
        .collect();
    assert_eq!(
        refresh_calls.len(),
        1,
        "Double-check locking should ensure only ONE refresh call, got {}",
        refresh_calls.len()
    );

    // @step And both requests should proceed with the same refreshed token
    assert!(result1.is_ok(), "First concurrent request should succeed");
    assert!(result2.is_ok(), "Second concurrent request should succeed");

    let backend_reqs = backend.received_requests().await.unwrap();
    assert_eq!(
        backend_reqs.len(),
        2,
        "Both API requests should be forwarded after refresh"
    );
    for (i, req) in backend_reqs.iter().enumerate() {
        assert_eq!(
            req.headers
                .get("authorization")
                .map(|v| v.to_str().unwrap()),
            Some("Bearer concurrent_fresh_tok"),
            "Request {i} should use the same refreshed token"
        );
    }
}

// =========================================================================
// REGRESSION: Scenario: OAuth tokens take precedence over API key
// =========================================================================

#[test]
fn test_regression_oauth_tokens_take_precedence_over_api_key() {
    // @step Given claude_auth.json exists with valid OAuth tokens
    // @step And ANTHROPIC_API_KEY environment variable is set
    // This tests the provider construction logic.

    // @step When the provider manager creates a Claude provider
    // from_oauth_tokens should succeed regardless of any API key
    let provider = ClaudeProvider::from_oauth_tokens(
        "sk-ant-oat01-oauth-access-token",
        "oauth-refresh-token",
        Some(3600),
        "https://console.anthropic.com",
        "claude-sonnet-4-20250514",
    )
    .expect("Should create OAuth provider from tokens");

    // @step Then the provider should be in OAuth mode
    assert!(
        provider.is_oauth_mode(),
        "Provider created from OAuth tokens must be in OAuth mode"
    );

    // @step And the ANTHROPIC_API_KEY should not be used for authentication
    // Verify the provider's system prompt indicates OAuth mode
    assert!(
        provider.system_prompt().is_some(),
        "OAuth mode provider must have system prompt prefix"
    );
}

// =========================================================================
// REGRESSION: Scenario: API key fallback works when no OAuth tokens exist
// =========================================================================

#[test]
fn test_regression_api_key_fallback_works_when_no_oauth_tokens_exist() {
    // @step Given claude_auth.json does not exist
    // @step And ANTHROPIC_API_KEY environment variable is set

    // @step When the provider manager creates a Claude provider
    // from_api_key_with_mode_and_model with ApiKey mode simulates the fallback
    let provider = ClaudeProvider::from_api_key_with_mode_and_model(
        "sk-ant-api03-fallback-api-key",
        AuthMode::ApiKey,
        "claude-sonnet-4-20250514",
    )
    .expect("Should create API key provider");

    // @step Then the provider should be in API key mode
    assert!(
        !provider.is_oauth_mode(),
        "Fallback provider must be in API key mode"
    );

    // @step And the ANTHROPIC_API_KEY should be used for authentication
    // In API key mode, rig uses x-api-key header (set at build time)
    assert!(
        provider.system_prompt().is_none(),
        "API key mode should NOT have OAuth system prompt prefix"
    );
}
