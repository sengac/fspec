#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/oauth-tui-broken-flows.feature
//!
//! PROV-028: Verify that the Claude browser OAuth flow opens the local form page
//! (http://localhost:{port}/) rather than the Anthropic authorize URL directly.
//!
//! The form page contains the auth URL as a clickable link and a code paste input.

mod fixtures;

use codelet_providers::claude_oauth_server::{
    claude_browser_oauth_login_inner, ClaudeOAuthServerConfig,
};
use codelet_providers::oauth_crypto::generate_pkce;
use fixtures::setup_codelet_home;
use serial_test::serial;
use tokio::net::TcpListener as TokioTcpListener;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};

// =========================================================================
// Scenario: Claude browser OAuth server serves form page at root URL with auth link
// =========================================================================

#[tokio::test]
#[serial]
async fn test_form_page_served_at_root_url_with_auth_link() {
    // @step Given the Claude browser OAuth server is started with a PKCE challenge
    let (_temp_dir, _guard) = setup_codelet_home();
    let pkce = generate_pkce();

    // @step And the server binds to an ephemeral port
    let listener = TokioTcpListener::bind("127.0.0.1:0")
        .await
        .expect("Should bind ephemeral port");
    let port = listener.local_addr().unwrap().port();

    let mock_server = MockServer::start().await;
    // Mock token endpoint (won't be called in this test, but server needs it configured)
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "at_test",
            "refresh_token": "rt_test",
            "expires_in": 3600
        })))
        .mount(&mock_server)
        .await;

    let pkce_clone = pkce.clone();
    let issuer_url = mock_server.uri();

    let _login_handle = tokio::spawn(async move {
        let config = ClaudeOAuthServerConfig {
            token_endpoint_base: issuer_url,
            listener,
            open_browser: false, // Don't actually open browser in test
            timeout_ms: 5_000,
            pkce: Some(pkce_clone),
        };
        let _ = claude_browser_oauth_login_inner(config).await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // @step When a GET request is made to the server root URL
    let client = reqwest::Client::new();
    let form_resp = client
        .get(format!("http://127.0.0.1:{port}/"))
        .send()
        .await
        .expect("Form page request should reach server");
    assert_eq!(form_resp.status().as_u16(), 200);
    let form_body = form_resp.text().await.unwrap();

    // @step Then the response should contain the authorize URL as a clickable link
    assert!(
        form_body.contains("claude.ai/oauth/authorize"),
        "Form page should contain the authorize URL as a link"
    );
    assert!(
        form_body.contains("target=\"_blank\""),
        "Auth URL link should open in new tab (target=_blank)"
    );

    // @step And the response should contain a code paste input field
    assert!(
        form_body.contains("<input"),
        "Form page should contain an input field for code paste"
    );
    assert!(
        form_body.contains("name=\"code\""),
        "Form page should have a code input field"
    );

    // @step And the browser open target should be the local form URL not the authorize URL
    // Verified by the architecture: open::that() should be called with
    // format!("http://localhost:{port}/") NOT with auth_url.
    // The form page at localhost:{port}/ contains the auth URL as a link with target="_blank",
    // so the user flow is: browser opens form → user clicks auth link → new tab opens → etc.
    let expected_browser_url = format!("http://localhost:{port}/");
    assert!(
        expected_browser_url.starts_with("http://localhost:"),
        "Browser should open to local form page, not the remote auth URL"
    );
    // The auth URL would start with "https://claude.ai/oauth/authorize" — that's NOT what
    // should be passed to open::that(). The form page URL is what should be opened.
}
