//! Feature: spec/features/codex-refreshing-client.feature

pub mod prov159_support;
#[path = "codex_refreshing_client/support.rs"]
mod support;

use codelet_providers::codex::codex_auth::read_codex_auth;
use codelet_providers::codex::codex_oauth::{rewrite_codex_url, CODEX_API_ENDPOINT};
use codelet_providers::codex::refreshing_client::{DEFAULT_EXPIRY_SECS, EXPIRY_BUFFER_SECS};
use codelet_providers::codex::{CodexAuthMode, CodexProvider};
use codelet_providers::LlmProvider;
use futures::StreamExt;
use prov159_support::TestResult;
use rig::http_client::HttpClientExt;
use serial_test::serial;
use support::{request, Case};

#[tokio::test]
#[serial]
async fn test_request_with_valid_token_passes_through_with_correct_headers() -> TestResult {
    let case = Case::new(Some(1800)).await?;
    case.send().await?;
    case.assert_headers("access-A").await?;
    assert!(case.issuer_requests().await?.is_empty());
    assert!(!case.client.is_token_expired().await);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_expired_token_is_automatically_refreshed_before_request() -> TestResult {
    let case = Case::new(Some(0)).await?;
    assert!(case.client.is_token_expired().await);
    case.refresh(200, Some(3600)).await;
    case.send().await?;
    case.assert_headers("access-A-rotated").await?;
    assert_eq!(case.issuer_requests().await?.len(), 1);
    let tokens = read_codex_auth()?
        .ok_or("persisted auth missing")?
        .tokens
        .ok_or("persisted tokens missing")?;
    assert_eq!(tokens.access_token, "access-A-rotated");
    assert_eq!(tokens.refresh_token, "refresh-A-rotated");
    assert!(!case.client.is_token_expired().await);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_token_refresh_failure_propagates_error() -> TestResult {
    let case = Case::new(Some(0)).await?;
    case.refresh(401, None).await;
    assert!(case.send().await.is_err());
    assert_eq!(case.issuer_requests().await?.len(), 1);
    assert!(case.backend_requests().await?.is_empty());
    assert_eq!(
        read_codex_auth()?
            .ok_or("persisted auth missing")?
            .tokens
            .ok_or("persisted tokens missing")?
            .access_token,
        "access-A"
    );
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_url_rewrite_for_v1_responses_path() -> TestResult {
    assert_eq!(
        rewrite_codex_url("https://api.openai.com/v1/responses"),
        CODEX_API_ENDPOINT
    );
    let case = Case::new(Some(1800)).await?;
    case.send().await?;
    case.assert_headers("access-A").await
}

#[tokio::test]
#[serial]
async fn test_url_rewrite_for_chat_completions_path() -> TestResult {
    assert_eq!(
        rewrite_codex_url("https://api.openai.com/v1/chat/completions"),
        CODEX_API_ENDPOINT
    );
    let case = Case::new(Some(1800)).await?;
    assert!(!case.client.is_token_expired().await);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_non_api_urls_pass_through_without_rewrite() -> TestResult {
    let case = Case::new(Some(1800)).await?;
    assert_eq!(rewrite_codex_url(&case.url()), case.url());
    case.send().await?;
    case.assert_headers("access-A").await?;
    assert_eq!(case.backend_requests().await?[0].url.path(), "/v1/models");
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_streaming_request_with_expired_token_refreshes_before_streaming() -> TestResult {
    let case = Case::new(Some(0)).await?;
    case.refresh(200, Some(3600)).await;
    let mut response = case.client.send_streaming(request(&case.url())?).await?;
    let mut body = Vec::new();
    while let Some(chunk) = response.body_mut().next().await {
        body.extend_from_slice(&chunk?);
    }
    assert!(String::from_utf8(body)?.contains("[DONE]"));
    case.assert_headers("access-A-rotated").await?;
    assert_eq!(case.issuer_requests().await?.len(), 1);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_existing_authorization_header_is_replaced() -> TestResult {
    let case = Case::new(Some(1800)).await?;
    let mut request = request(&case.url())?;
    request
        .headers_mut()
        .insert("authorization", "Bearer dummy-api-key".parse()?);
    let response: http::Response<rig::http_client::LazyBody<Vec<u8>>> =
        case.client.send(request).await?;
    assert_eq!(response.status(), 200);
    case.assert_headers("access-A").await
}

#[tokio::test]
#[serial]
async fn test_codex_provider_uses_refreshing_client_for_oauth_mode() -> TestResult {
    let case = Case::new(Some(3600)).await?;
    let provider = CodexProvider::from_oauth_tokens(
        "access-A",
        "refresh-A",
        "A",
        Some(3600),
        &case.issuer.uri(),
        "gpt-5.1-codex",
    )?;
    let _agent = provider.create_rig_agent(uuid::Uuid::new_v4(), None, None);
    assert_eq!(provider.model(), "gpt-5.1-codex");
    assert!(matches!(
        provider.auth_mode(),
        CodexAuthMode::OAuthDirect { .. }
    ));
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_token_refresh_within_expiry_buffer_triggers_proactive_refresh() -> TestResult {
    let case = Case::new(Some(20)).await?;
    assert_eq!(EXPIRY_BUFFER_SECS, 30);
    assert!(case.client.is_token_expired().await);
    case.refresh(200, Some(3600)).await;
    case.send().await?;
    case.assert_headers("access-A-rotated").await?;
    assert_eq!(case.issuer_requests().await?.len(), 1);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_api_key_mode_passes_requests_through_unchanged() -> TestResult {
    let case = Case::new(Some(0)).await?;
    let client = codelet_providers::codex::refreshing_client::RefreshingCodexClient::new_api_key();
    let url = format!("{}/v1/chat/completions", case.backend.uri());
    let mut request = request(&url)?;
    request
        .headers_mut()
        .insert("authorization", "Bearer independent-key".parse()?);
    let response: http::Response<rig::http_client::LazyBody<Vec<u8>>> =
        client.send(request).await?;
    assert_eq!(response.status(), 200);
    assert!(!client.is_token_expired().await);
    let requests = case.backend_requests().await?;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url.path(), "/v1/chat/completions");
    assert_eq!(
        requests[0].headers["authorization"],
        "Bearer independent-key"
    );
    assert!(!requests[0].headers.contains_key("chatgpt-account-id"));
    assert!(!requests[0].headers.contains_key("originator"));
    assert!(case.issuer_requests().await?.is_empty());
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_default_expiry_when_expires_in_not_provided() -> TestResult {
    let case = Case::new(None).await?;
    assert_eq!(DEFAULT_EXPIRY_SECS, 3600);
    assert_eq!(DEFAULT_EXPIRY_SECS - EXPIRY_BUFFER_SECS, 3570);
    assert!(!case.client.is_token_expired().await);
    let expired = Case::new(Some(29)).await?;
    expired.refresh(200, None).await;
    expired.send().await?;
    assert!(!expired.client.is_token_expired().await);
    assert_eq!(expired.issuer_requests().await?.len(), 1);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_explicit_zero_lifetime_triggers_immediate_refresh() -> TestResult {
    let case = Case::new(Some(0)).await?;
    assert!(case.client.is_token_expired().await);
    case.refresh(200, Some(3600)).await;
    case.send().await?;
    case.assert_headers("access-A-rotated").await?;
    assert_eq!(case.issuer_requests().await?.len(), 1);
    Ok(())
}
