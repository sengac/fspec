// Feature: spec/features/codex-models-fetch-never-leaves-a-test-build.feature
//! BUG-544 — a test build never sends the Codex /models request to
//! chatgpt.com: it goes only to a redirect the test installed, and without
//! one it fails before any request.

use codelet_providers::codex::models_catalog::{fetch_for_selection, test_support};
use serde_json::json;
use serial_test::serial;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Sign Codex in with (fake) ChatGPT tokens in a temporary Codex home.
fn signed_in_codex_home() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let home = tempfile::tempdir()?;
    let document = json!({
        "tokens": {
            "id_token": "id-acc_bug544",
            "access_token": "access-acc_bug544",
            "refresh_token": "refresh-acc_bug544",
            "account_id": "acc_bug544"
        },
        "last_refresh": chrono::Utc::now().to_rfc3339()
    });
    std::fs::write(home.path().join("auth.json"), document.to_string())?;
    std::env::set_var("CODEX_HOME", home.path());
    Ok(home)
}

// Scenario: Without a redirect a test build sends no Codex models request
#[tokio::test]
#[serial]
async fn without_a_redirect_a_test_build_sends_no_codex_models_request() -> TestResult {
    // @step Given Codex is signed in with ChatGPT in a temporary Codex home
    let _home = signed_in_codex_home()?;
    // @step And no test redirect is installed
    // (no test_support::redirect guard is alive in this serial test)
    // @step When the Codex models are fetched for a model selection
    let fetched = fetch_for_selection().await;
    // @step Then the fetch fails at once with a reason naming the test redirect
    let reason = fetched
        .err()
        .ok_or("the fetch must fail without a redirect")?;
    assert!(
        reason.contains("chatgpt.com") && reason.contains("test_support::redirect"),
        "{reason}"
    );
    std::env::remove_var("CODEX_HOME");
    Ok(())
}

// Scenario: With a redirect the Codex models request goes to the local mock
#[tokio::test]
#[serial]
async fn with_a_redirect_the_codex_models_request_goes_to_the_local_mock() -> TestResult {
    // @step Given Codex is signed in with ChatGPT in a temporary Codex home
    let _home = signed_in_codex_home()?;
    // @step And a test redirect to a local mock is installed
    let backend = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/models"))
        .and(query_param("client_version", "0.165.0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"models": []})))
        .expect(1)
        .mount(&backend)
        .await;
    let _redirect = test_support::redirect(&backend.uri());
    // @step When the Codex models are fetched for a model selection
    let _ = fetch_for_selection().await;
    // @step Then the local mock receives the models request with the pinned client version
    let received = backend.received_requests().await.unwrap_or_default();
    assert_eq!(received.len(), 1, "{received:?}");
    assert_eq!(received[0].url.path(), "/models");
    std::env::remove_var("CODEX_HOME");
    Ok(())
}
