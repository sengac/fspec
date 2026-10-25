//! Feature: spec/features/copilot-request-time-refresh.feature
#![cfg(feature = "test-support")]

#[path = "../../agent-loop/tests/support/bug268_environment.rs"]
mod environment;

use codelet_providers::{
    copilot::{
        test_support::TestIssuerGuard, CopilotAuthJson, CopilotDeploymentType, CopilotProvider,
    },
    session::ProviderSession,
    LlmProvider, ProviderAdapter,
};
use environment::{Environment, TestResult};
use rig::{
    client::CompletionClient,
    completion::{CompletionModel, CompletionRequestBuilder},
};
use serde_json::json;
use std::time::Duration;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

fn provider(api: &MockServer) -> TestResult<CopilotProvider> {
    assert!(api.uri().starts_with("http://127.0.0.1:"));
    Ok(CopilotProvider::from_auth(
        CopilotDeploymentType::GitHubCom,
        CopilotAuthJson {
            github_oauth_token: "synthetic-refresh-account".into(),
            copilot_token: Some("synthetic-expired".into()),
            copilot_token_expires_at: Some(0),
            endpoints_api: Some(api.uri()),
            enterprise_url: None,
        },
        "gpt-5",
    )?)
}

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
            "token":"synthetic-fresh","expires_at":u64::MAX,"endpoints":{"api":api.uri()}
        })))
        .mount(&server)
        .await;
    server
}

async fn assert_fresh(api: &MockServer, expected: usize) -> TestResult {
    let requests = api.received_requests().await.ok_or("missing capture")?;
    assert_eq!(requests.len(), expected);
    for request in requests {
        assert_eq!(request.url.path(), "/responses");
        assert_eq!(request.headers["authorization"], "Bearer synthetic-fresh");
        let body: serde_json::Value = request.body_json()?;
        assert!(body.get("input").is_some() && body.get("messages").is_none());
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn concurrent_independent_requests_share_existing_token_refresh_coordination() -> TestResult {
    // @step Given independent Copilot sessions share expired synthetic authentication and a validated local issuer
    let environment = Environment::new()?;
    let api = inference().await;
    let issuer = issuer(&api).await;
    let _guard = TestIssuerGuard::install(environment.directory.path(), &issuer.uri())?;
    let provider = provider(&api)?;
    let first = provider
        .clone()
        .with_session(&ProviderSession::for_provider("github-copilot"));
    let second = provider
        .clone()
        .with_session(&ProviderSession::for_provider("github-copilot"));
    let selected = second.client().completion_model("gpt-5");
    let messages = [codelet_common::Message::user("first independent task")];
    // @step When both real nonstreaming front doors request completion concurrently
    let (first, second) = tokio::join!(
        first.complete(&messages),
        selected.completion(
            CompletionRequestBuilder::new(selected.clone(), "second independent task").build()
        )
    );
    assert_eq!(first?, "synthetic answer");
    assert!(second?.choice.iter().any(|content| matches!(content,
        rig::message::AssistantContent::Text(text) if text.text == "synthetic answer")));
    // @step Then one token exchange precedes both freshly authorized inference requests and persists synthetic credentials
    assert_fresh(&api, 2).await?;
    assert_eq!(
        issuer
            .received_requests()
            .await
            .ok_or("missing issuer capture")?
            .len(),
        1
    );
    let persisted = codelet_providers::copilot::read_copilot_auth()
        .await?
        .ok_or("missing persisted auth")?;
    assert_eq!(persisted.copilot_token.as_deref(), Some("synthetic-fresh"));
    assert_eq!(persisted.github_oauth_token, "synthetic-refresh-account");
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn refreshed_effective_endpoints_reach_the_actual_nonstreaming_request() -> TestResult {
    // @step Given a Copilot model was built before a local issuer refresh changes its effective API origin
    let environment = Environment::new()?;
    let former = inference().await;
    let current = inference().await;
    let issuer = issuer(&current).await;
    let _guard = TestIssuerGuard::install(environment.directory.path(), &issuer.uri())?;
    let provider = provider(&former)?;
    let model = provider.client().completion_model("gpt-5");
    assert!(provider.ensure_fresh_copilot_token().await?);
    // @step When that public completion model sends its real request
    model
        .completion(CompletionRequestBuilder::new(model.clone(), "task after refresh").build())
        .await?;
    // @step Then only the refreshed loopback origin receives fresh Authorization and the selected Responses dialect
    assert!(former
        .received_requests()
        .await
        .ok_or("missing former capture")?
        .is_empty());
    assert_fresh(&current, 1).await?;
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn rejected_or_cancelled_exchange_cannot_send_expired_authorization() -> TestResult {
    // @step Given expired synthetic authentication and a rejecting or delayed local issuer
    let environment = Environment::new()?;
    for delayed in [false, true] {
        let api = inference().await;
        let issuer = MockServer::start().await;
        let response = if delayed {
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(2))
                .set_body_json(json!({"token":"synthetic-fresh","expires_at":u64::MAX}))
        } else {
            ResponseTemplate::new(403).set_body_string("synthetic exchange denied")
        };
        Mock::given(method("GET"))
            .respond_with(response)
            .mount(&issuer)
            .await;
        let _guard = TestIssuerGuard::install(environment.directory.path(), &issuer.uri())?;
        let provider = provider(&api)?;
        let model = provider.client().completion_model("gpt-5");
        // @step When a real Copilot completion is rejected or cancelled during token exchange
        let request = CompletionRequestBuilder::new(model.clone(), "must not infer").build();
        let diagnostic = if delayed {
            let task = tokio::spawn(async move { model.completion(request).await });
            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    if issuer
                        .received_requests()
                        .await
                        .is_some_and(|requests| !requests.is_empty())
                    {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            })
            .await?;
            task.abort();
            let error = task
                .await
                .err()
                .ok_or("exchange settled before cancellation")?;
            assert!(error.is_cancelled());
            error.to_string()
        } else {
            let result =
                tokio::time::timeout(Duration::from_secs(5), model.completion(request)).await?;
            let diagnostic = result
                .err()
                .ok_or("rejected exchange unexpectedly succeeded")?
                .to_string();
            assert!(diagnostic.contains("403"));
            diagnostic
        };
        // @step Then inference is never sent and no credential value reaches the returned diagnostic
        assert!(api
            .received_requests()
            .await
            .ok_or("missing inference capture")?
            .is_empty());
        assert!(
            !diagnostic.contains("synthetic-expired")
                && !diagnostic.contains("synthetic-refresh-account")
        );
        assert_eq!(
            provider.auth_snapshot().await.copilot_token_expires_at,
            Some(0)
        );
    }
    Ok(())
}
