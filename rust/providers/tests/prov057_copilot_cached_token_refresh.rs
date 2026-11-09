//! Feature: spec/features/copilot-provider-token-refresh-and-endpoints.feature
//!
//! PROV-057: the cached Copilot token in copilot_auth.json is refreshed through
//! the token exchange when it is within 60 seconds of expiry, and reused
//! otherwise. Drives a real completion request against loopback servers.
#![cfg(feature = "test-support")]

#[path = "../../agent-loop/tests/support/bug268_environment.rs"]
mod environment;

use codelet_providers::copilot::{
    read_copilot_auth, test_support::TestIssuerGuard, write_copilot_auth, CopilotAuthJson,
    CopilotDeploymentType, CopilotProvider,
};
use environment::{Environment, TestResult};
use rig::{
    client::CompletionClient,
    completion::{CompletionModel, CompletionRequestBuilder},
};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

const REFRESHED_EXPIRES_AT: u64 = 4_102_444_800;

async fn inference() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path("/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id":"synthetic-response","object":"response","created_at":1,"model":"gpt-5","status":"completed",
            "output":[{"type":"message","id":"synthetic-message","role":"assistant","status":"completed",
                "content":[{"type":"output_text","text":"synthetic answer","annotations":[]}]}],
            "usage":{"input_tokens":4,"output_tokens":2,"total_tokens":6}
        }))).mount(&server).await;
    server
}

async fn issuer(api: &MockServer) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "token":"synthetic-refreshed","expires_at":REFRESHED_EXPIRES_AT,"endpoints":{"api":api.uri()}
        })))
        .mount(&server)
        .await;
    server
}

/// Write copilot_auth.json with a cached Copilot token expiring `lifetime`
/// seconds from now, then build the provider from what was persisted.
async fn provider_from_cached_token(
    api: &MockServer,
    lifetime: u64,
) -> TestResult<CopilotProvider> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    write_copilot_auth(&CopilotAuthJson {
        github_oauth_token: "synthetic-account".into(),
        copilot_token: Some("synthetic-cached".into()),
        copilot_token_expires_at: Some(now + lifetime),
        endpoints_api: Some(api.uri()),
        enterprise_url: None,
    })
    .await?;
    let auth = read_copilot_auth()
        .await?
        .ok_or("missing copilot_auth.json")?;
    Ok(CopilotProvider::from_auth(
        CopilotDeploymentType::GitHubCom,
        auth,
        "gpt-5",
    )?)
}

async fn complete(provider: &CopilotProvider) -> TestResult {
    let model = provider.client().completion_model("gpt-5");
    model
        .completion(CompletionRequestBuilder::new(model.clone(), "cached token task").build())
        .await?;
    Ok(())
}

async fn chat_authorizations(api: &MockServer) -> TestResult<Vec<String>> {
    let requests = api.received_requests().await.ok_or("missing capture")?;
    Ok(requests
        .iter()
        .map(|request| {
            assert_eq!(request.url.path(), "/responses");
            request.headers["authorization"].to_str().map(str::to_owned)
        })
        .collect::<Result<_, _>>()?)
}

async fn exchange_count(issuer: &MockServer) -> TestResult<usize> {
    Ok(issuer
        .received_requests()
        .await
        .ok_or("missing issuer capture")?
        .len())
}

#[tokio::test]
#[serial_test::serial]
async fn cached_copilot_token_is_refreshed_when_within_60_seconds_of_expiry() -> TestResult {
    let environment = Environment::new()?;
    let api = inference().await;
    let issuer = issuer(&api).await;
    let _guard = TestIssuerGuard::install(environment.directory.path(), &issuer.uri())?;
    // @step Given copilot_auth.json contains a copilot_token with copilot_token_expires_at set to 30 seconds from now
    let provider = provider_from_cached_token(&api, 30).await?;

    // @step When CopilotProvider issues a chat completion request
    complete(&provider).await?;

    // @step Then the provider re-calls the token exchange endpoint before sending the chat request
    assert_eq!(exchange_count(&issuer).await?, 1);
    assert_eq!(
        chat_authorizations(&api).await?,
        ["Bearer synthetic-refreshed"]
    );

    // @step And copilot_auth.json is updated with the new copilot_token and expires_at
    let persisted = read_copilot_auth().await?.ok_or("missing persisted auth")?;
    assert_eq!(
        persisted.copilot_token.as_deref(),
        Some("synthetic-refreshed")
    );
    assert_eq!(
        persisted.copilot_token_expires_at,
        Some(REFRESHED_EXPIRES_AT)
    );
    assert_eq!(persisted.github_oauth_token, "synthetic-account");
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn cached_copilot_token_is_reused_when_not_near_expiry() -> TestResult {
    let environment = Environment::new()?;
    let api = inference().await;
    let issuer = issuer(&api).await;
    let _guard = TestIssuerGuard::install(environment.directory.path(), &issuer.uri())?;
    // @step Given copilot_auth.json contains a copilot_token with copilot_token_expires_at set to 20 minutes from now
    let provider = provider_from_cached_token(&api, 20 * 60).await?;

    // @step When CopilotProvider issues a chat completion request
    complete(&provider).await?;

    // @step Then the provider does NOT re-call the token exchange endpoint
    assert_eq!(exchange_count(&issuer).await?, 0);

    // @step And the cached copilot_token is used as the Bearer token for the request
    assert_eq!(
        chat_authorizations(&api).await?,
        ["Bearer synthetic-cached"]
    );
    Ok(())
}
