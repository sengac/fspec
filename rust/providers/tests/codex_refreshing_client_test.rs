//! Feature: spec/features/codex-refreshing-client.feature
//!
//! Requests reach loopback mock servers, so the Codex URL rewrite is checked
//! through `rewrite_codex_url` (the rewrite the client applies to real
//! OpenAI URLs) while headers and refresh behaviour are checked on the
//! requests the client actually sends.

pub mod prov159_support;
#[path = "codex_refreshing_client/support.rs"]
mod support;

use codelet_providers::codex::codex_auth::read_codex_auth;
use codelet_providers::codex::codex_oauth::rewrite_codex_url;
use codelet_providers::codex::refreshing_client::{
    RefreshingCodexClient, DEFAULT_EXPIRY_SECS, EXPIRY_BUFFER_SECS,
};
use codelet_providers::codex::{CodexAuthMode, CodexProvider};
use codelet_providers::LlmProvider;
use futures::StreamExt;
use prov159_support::TestResult;
use rig::http_client::HttpClientExt;
use serial_test::serial;
use support::{request, Case, ACCESS, ACCOUNT, ROTATED_ACCESS, ROTATED_REFRESH};

const CODEX_RESPONSES_URL: &str = "https://chatgpt.com/backend-api/codex/responses";

type Sent = Result<http::Response<rig::http_client::LazyBody<Vec<u8>>>, rig::http_client::Error>;

#[tokio::test]
#[serial]
async fn test_request_with_valid_token_passes_through_with_correct_headers() -> TestResult {
    // @step Given a RefreshingCodexClient in OAuth mode with a valid access token expiring in 30 minutes
    // @step And an account ID "acc_12345"
    let case = Case::new(Some(30 * 60)).await?;
    assert!(!case.client.is_token_expired().await);
    // @step When the client sends a request to "https://api.openai.com/v1/chat/completions"
    case.send().await?;
    // @step Then the request URL should be rewritten to "https://chatgpt.com/backend-api/codex/responses"
    assert_eq!(
        rewrite_codex_url("https://api.openai.com/v1/chat/completions"),
        CODEX_RESPONSES_URL
    );
    // @step And the Authorization header should be "Bearer {access_token}"
    // @step And the ChatGPT-Account-Id header should be "acc_12345"
    // @step And the originator header should be "rust"
    case.assert_headers(ACCESS).await?;
    // @step And no token refresh should occur
    assert!(case.issuer_requests().await?.is_empty());
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_expired_token_is_automatically_refreshed_before_request() -> TestResult {
    // @step Given a RefreshingCodexClient in OAuth mode with an expired access token
    let case = Case::new(Some(0)).await?;
    assert!(case.client.is_token_expired().await);
    // @step And a valid refresh token bound to the currently persisted OAuth credentials
    case.refresh(200, Some(3600)).await;
    // @step When the client sends a request to "https://api.openai.com/v1/chat/completions"
    // @step And the response should be returned successfully
    case.send().await?;
    // @step Then the client should refresh the access token via the OAuth token endpoint
    assert_eq!(case.issuer_requests().await?.len(), 1);
    // @step And the refreshed tokens should be persisted to auth.json
    let tokens = read_codex_auth()?
        .ok_or("persisted auth missing")?
        .tokens
        .ok_or("persisted tokens missing")?;
    assert_eq!(tokens.access_token, ROTATED_ACCESS);
    assert_eq!(tokens.refresh_token, ROTATED_REFRESH);
    // @step And the request should proceed with the new access token
    case.assert_headers(ROTATED_ACCESS).await?;
    assert!(!case.client.is_token_expired().await);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_token_refresh_failure_propagates_error() -> TestResult {
    // @step Given a RefreshingCodexClient in OAuth mode with an expired access token
    let case = Case::new(Some(0)).await?;
    // @step And an invalid refresh token that returns a 401 error
    case.refresh(401, None).await;
    // @step When the client sends a request to "https://api.openai.com/v1/chat/completions"
    let sent: Sent = case.client.send(request(&case.url())?).await;
    // @step Then the client should attempt to refresh the access token
    assert_eq!(case.issuer_requests().await?.len(), 1);
    // @step And the refresh should fail with an authentication error
    // @step And the error should propagate to the caller
    let error = sent
        .err()
        .ok_or("request succeeded despite the failed refresh")?;
    assert!(
        matches!(
            &error,
            rig::http_client::Error::InvalidStatusCodeWithMessage(status, message)
                if status.as_u16() == 401 && message.contains("sign in again")
        ),
        "expected the rejected refresh to surface as an authentication error, got {error:?}"
    );
    // @step And the original API request should NOT be sent
    assert!(case.backend_requests().await?.is_empty());
    assert_eq!(
        read_codex_auth()?
            .ok_or("persisted auth missing")?
            .tokens
            .ok_or("persisted tokens missing")?
            .access_token,
        ACCESS
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_url_rewrite_for_v1_responses_path() -> TestResult {
    // @step Given a RefreshingCodexClient in OAuth mode with a valid access token
    let case = Case::new(Some(1800)).await?;
    // @step When the client sends a request to "https://api.openai.com/v1/responses"
    case.send().await?;
    // @step Then the request URL should be rewritten to "https://chatgpt.com/backend-api/codex/responses"
    assert_eq!(
        rewrite_codex_url("https://api.openai.com/v1/responses"),
        CODEX_RESPONSES_URL
    );
    case.assert_headers(ACCESS).await
}

#[tokio::test]
#[serial]
async fn test_url_rewrite_for_chat_completions_path() -> TestResult {
    // @step Given a RefreshingCodexClient in OAuth mode with a valid access token
    let case = Case::new(Some(1800)).await?;
    assert!(!case.client.is_token_expired().await);
    // @step When the client sends a request to "https://api.openai.com/v1/chat/completions"
    case.send().await?;
    // @step Then the request URL should be rewritten to "https://chatgpt.com/backend-api/codex/responses"
    assert_eq!(
        rewrite_codex_url("https://api.openai.com/v1/chat/completions"),
        CODEX_RESPONSES_URL
    );
    case.assert_headers(ACCESS).await
}

#[tokio::test]
#[serial]
async fn test_non_api_urls_pass_through_without_rewrite() -> TestResult {
    // @step Given a RefreshingCodexClient in OAuth mode with a valid access token
    let case = Case::new(Some(1800)).await?;
    // @step When the client sends a request to "https://api.openai.com/v1/models"
    case.send().await?;
    // @step Then the request URL should NOT be rewritten
    assert_eq!(
        rewrite_codex_url("https://api.openai.com/v1/models"),
        "https://api.openai.com/v1/models"
    );
    assert_eq!(rewrite_codex_url(&case.url()), case.url());
    assert_eq!(case.backend_requests().await?[0].url.path(), "/v1/models");
    // @step And the auth headers should still be set correctly
    case.assert_headers(ACCESS).await
}

#[tokio::test]
#[serial]
async fn test_streaming_request_with_expired_token_refreshes_before_streaming() -> TestResult {
    // @step Given a RefreshingCodexClient in OAuth mode with an expired access token
    let case = Case::new(Some(0)).await?;
    // @step And a valid refresh token
    case.refresh(200, Some(3600)).await;
    // @step When the client sends a streaming request to "https://api.openai.com/v1/chat/completions"
    let mut response = case.client.send_streaming(request(&case.url())?).await?;
    // @step Then the client should refresh the access token before streaming
    assert_eq!(case.issuer_requests().await?.len(), 1);
    // @step And the streaming response should use the refreshed credentials
    case.assert_headers(ROTATED_ACCESS).await?;
    // @step And the SSE stream should be returned successfully
    let mut body = Vec::new();
    while let Some(chunk) = response.body_mut().next().await {
        body.extend_from_slice(&chunk?);
    }
    assert!(String::from_utf8(body)?.contains("[DONE]"));
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_existing_authorization_header_is_replaced() -> TestResult {
    // @step Given a RefreshingCodexClient in OAuth mode with a valid access token
    let case = Case::new(Some(1800)).await?;
    // @step And the original request has a dummy Authorization header "Bearer dummy-api-key" set by rig
    let mut request = request(&case.url())?;
    request
        .headers_mut()
        .insert("authorization", "Bearer dummy-api-key".parse()?);
    // @step When the client sends the request
    let response: http::Response<rig::http_client::LazyBody<Vec<u8>>> =
        case.client.send(request).await?;
    assert_eq!(response.status(), 200);
    // @step Then the dummy Authorization header should be stripped
    let requests = case.backend_requests().await?;
    let authorizations = requests[0]
        .headers
        .get_all("authorization")
        .iter()
        .map(|value| value.to_str())
        .collect::<Result<Vec<_>, _>>()?;
    assert!(!authorizations.contains(&"Bearer dummy-api-key"));
    // @step And replaced with "Bearer {current_access_token}"
    assert_eq!(authorizations, [format!("Bearer {ACCESS}")]);
    case.assert_headers(ACCESS).await
}

#[tokio::test]
#[serial]
async fn test_codex_provider_uses_refreshing_client_for_oauth_mode() -> TestResult {
    let case = Case::new(Some(3600)).await?;
    // @step Given OAuth tokens with access_token, refresh_token, and account_id
    let refresh = format!("refresh-{ACCOUNT}");
    // @step When CodexProvider::from_oauth_tokens() is called
    let provider = CodexProvider::from_oauth_tokens(
        ACCESS,
        &refresh,
        ACCOUNT,
        Some(3600),
        &case.issuer.uri(),
        "gpt-5.1-codex",
    )?;
    // @step Then a RefreshingCodexClient should be created with OAuth TokenMode
    assert!(matches!(
        provider.auth_mode(),
        CodexAuthMode::OAuthDirect { .. }
    ));
    // @step And it should be passed as the HTTP client to rig CompletionsClient<RefreshingCodexClient>
    let _client: &rig::providers::openai::Client<RefreshingCodexClient> = provider.client();
    // @step And the provider should be able to construct a rig Agent
    let _agent = provider.create_rig_agent(uuid::Uuid::new_v4(), None, None);
    assert_eq!(provider.model(), "gpt-5.1-codex");
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_token_refresh_within_expiry_buffer_triggers_proactive_refresh() -> TestResult {
    // @step Given a RefreshingCodexClient in OAuth mode with a token expiring in 20 seconds
    let case = Case::new(Some(20)).await?;
    // @step And the expiry buffer is 30 seconds
    assert_eq!(EXPIRY_BUFFER_SECS, 30);
    assert!(case.client.is_token_expired().await);
    case.refresh(200, Some(3600)).await;
    // @step When the client sends a request
    case.send().await?;
    // @step Then the client should proactively refresh the token
    assert_eq!(case.issuer_requests().await?.len(), 1);
    // @step And the request should use the refreshed token
    case.assert_headers(ROTATED_ACCESS).await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_api_key_mode_passes_requests_through_unchanged() -> TestResult {
    let case = Case::new(Some(0)).await?;
    // @step Given a RefreshingCodexClient in ApiKey mode
    let client = RefreshingCodexClient::new_api_key();
    let url = format!("{}/v1/chat/completions", case.backend.uri());
    let mut request = request(&url)?;
    request
        .headers_mut()
        .insert("authorization", "Bearer independent-key".parse()?);
    // @step When the client sends a request to "https://api.openai.com/v1/chat/completions"
    let response: http::Response<rig::http_client::LazyBody<Vec<u8>>> =
        client.send(request).await?;
    assert_eq!(response.status(), 200);
    let requests = case.backend_requests().await?;
    assert_eq!(requests.len(), 1);
    // @step Then the request URL should NOT be rewritten
    assert_eq!(requests[0].url.path(), "/v1/chat/completions");
    // @step And no token refresh should occur
    assert!(!client.is_token_expired().await);
    assert!(case.issuer_requests().await?.is_empty());
    // @step And the original headers from rig should be preserved
    assert_eq!(
        requests[0].headers["authorization"],
        "Bearer independent-key"
    );
    // @step And the request should be forwarded to reqwest as-is
    assert!(!requests[0].headers.contains_key("chatgpt-account-id"));
    assert!(!requests[0].headers.contains_key("originator"));
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_default_expiry_when_expires_in_not_provided() -> TestResult {
    // @step Given a RefreshingCodexClient in OAuth mode
    let case = Case::new(Some(0)).await?;
    // @step And a token refresh response with no expires_in field
    case.refresh(200, None).await;
    // @step When the token is refreshed
    case.send().await?;
    assert_eq!(case.issuer_requests().await?.len(), 1);
    // @step Then the expiry should default to 3600 seconds from now
    assert_eq!(DEFAULT_EXPIRY_SECS, 3600);
    assert!(!case.client.is_token_expired().await);
    assert!(!Case::new(None).await?.client.is_token_expired().await);
    // @step And the 30-second buffer should still apply
    assert_eq!(DEFAULT_EXPIRY_SECS - EXPIRY_BUFFER_SECS, 3570);
    // @step And a request sent 3571 seconds later should trigger a refresh
    let later = Case::new(Some(DEFAULT_EXPIRY_SECS - 3571)).await?;
    assert!(later.client.is_token_expired().await);
    later.refresh(200, None).await;
    later.send().await?;
    assert_eq!(later.issuer_requests().await?.len(), 1);
    later.assert_headers(ROTATED_ACCESS).await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_explicit_zero_lifetime_triggers_immediate_refresh() -> TestResult {
    // @step Given OAuth tokens exist in ~/.codex/auth.json from a previous session
    // @step When a caller explicitly passes Some(0) for expires_in_secs to RefreshingCodexClient
    let case = Case::new(Some(0)).await?;
    // @step Then the token is immediately considered expired
    assert!(case.client.is_token_expired().await);
    // @step Given the access token may be expired
    case.refresh(200, Some(3600)).await;
    case.send().await?;
    // @step Then the first API request triggers a token refresh before sending
    assert_eq!(case.issuer_requests().await?.len(), 1);
    case.assert_headers(ROTATED_ACCESS).await?;
    Ok(())
}
