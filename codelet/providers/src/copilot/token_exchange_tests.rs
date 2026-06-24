//! Unit tests for [`token_exchange`] (PROV-057 L2).
//!
//! Feature: spec/features/github-copilot-end-to-end-integration.feature
//!
//! These tests stand up a wiremock server in place of
//! `api.github.com/copilot_internal/v2/token` so the exchange function
//! can be exercised without any real HTTP traffic.

use super::*;
use wiremock::matchers::{header, header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn build_token_exchange_url_defaults_to_github_com() {
    // @step Given a github.com deployment (no enterprise host)
    let url = build_token_exchange_url(None);
    // @step Then the URL points at api.github.com
    assert_eq!(url, "https://api.github.com/copilot_internal/v2/token");
}

#[test]
fn build_token_exchange_url_uses_enterprise_api_v3_path() {
    // @step Given an enterprise host "ghe.example.com"
    let url = build_token_exchange_url(Some("ghe.example.com"));
    // @step Then the URL is the GHE /api/v3 variant
    assert_eq!(
        url,
        "https://ghe.example.com/api/v3/copilot_internal/v2/token"
    );
}

#[test]
fn empty_github_oauth_token_is_rejected_before_http() {
    // Smoke test: tokio runtime unnecessary because the guard is sync.
    let fut = exchange_github_token_for_copilot_token_at("https://ignored.example.com/", "");
    let rt = tokio::runtime::Runtime::new().unwrap();
    let res = rt.block_on(fut);
    assert!(res.is_err());
}

#[tokio::test]
async fn exchange_sends_correct_headers_and_parses_response() {
    // Scenario: GitHub OAuth token is exchanged for a short-lived Copilot token
    //           before any API call
    // @step Given a valid github_oauth_token is stored in copilot_auth.json
    let mock_server = MockServer::start().await;

    // @step And no copilot_token is currently cached
    // (verified at the provider layer; this test covers the wire contract)

    // @step When CopilotProvider issues a chat completion request
    // @step Then a GET request is sent to "https://api.github.com/copilot_internal/v2/token"
    // @step And the request uses header "Authorization: token <gho_*>"
    // @step And the request includes headers "Editor-Version",
    //       "Editor-Plugin-Version", "User-Agent" and "Accept: application/json"
    Mock::given(method("GET"))
        .and(path("/copilot_internal/v2/token"))
        .and(header("Authorization", "token gho_abc123"))
        .and(header("Accept", "application/json"))
        .and(header_exists("Editor-Version"))
        .and(header_exists("Editor-Plugin-Version"))
        .and(header_exists("User-Agent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "token": "tid=abc;exp=2000000000;:sig",
            "expires_at": 2_000_000_000u64,
            "refresh_in": 1500,
            "endpoints": {
                "api": "https://api.githubcopilot.com"
            },
            "tracking_id": "trk",
            "sku": "copilot-pro"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let url = format!("{}/copilot_internal/v2/token", mock_server.uri());

    // @step And the response is parsed for "token", "expires_at", and "endpoints.api"
    let response = exchange_github_token_for_copilot_token_at(&url, "gho_abc123")
        .await
        .expect("exchange should succeed against mock server");

    assert_eq!(response.token, "tid=abc;exp=2000000000;:sig");
    assert_eq!(response.expires_at, 2_000_000_000);
    assert_eq!(response.endpoints_api, "https://api.githubcopilot.com");

    // @step And the parsed values are persisted to copilot_auth.json before the chat request is sent
    // (persistence is exercised by provider::ensure_fresh_copilot_token — the unit-level
    // tests in provider_tests.rs prove apply_exchange_response + needs_copilot_token_refresh
    // drive the persistence path; this test validates the wire contract only.)
}

#[tokio::test]
async fn exchange_returns_endpoints_api_for_enterprise_deployment() {
    // Scenario: Chat requests honour the endpoints.api URL from the token
    //           exchange response
    // @step Given an enterprise-style token exchange returns a copilot-api host
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/copilot_internal/v2/token"))
        .and(header("Authorization", "token gho_ghe_xyz"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "token": "tid=ghe;exp=2000000000;:sig",
            "expires_at": 2_000_000_000u64,
            "endpoints": {
                "api": "https://copilot-api.ghe.example.com"
            }
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let url = format!("{}/copilot_internal/v2/token", mock_server.uri());
    let response = exchange_github_token_for_copilot_token_at(&url, "gho_ghe_xyz")
        .await
        .expect("enterprise exchange should succeed");

    // @step Then the parsed endpoints_api is the enterprise copilot-api URL
    assert_eq!(
        response.endpoints_api,
        "https://copilot-api.ghe.example.com"
    );
}

#[tokio::test]
async fn exchange_propagates_non_2xx_as_api_error() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/copilot_internal/v2/token"))
        .respond_with(ResponseTemplate::new(401).set_body_string("bad token"))
        .mount(&mock_server)
        .await;

    let url = format!("{}/copilot_internal/v2/token", mock_server.uri());
    let err = exchange_github_token_for_copilot_token_at(&url, "gho_bad")
        .await
        .expect_err("401 must surface as error");
    assert!(err.to_string().contains("401"));
}
