//! Feature: spec/features/anthropic-oauth-wire-contract.feature
#![cfg(feature = "test-support")]

#[path = "support/prov168.rs"]
mod support;

use codelet_providers::claude::{AuthMode, ClaudeProvider};
use futures::StreamExt;
use rig::completion::{Completion, CompletionModel, CompletionRequestBuilder, GetTokenUsage};
use rig::providers::anthropic;
use rig::streaming::StreamedAssistantContent;
use serde_json::json;
use support::{oauth, refreshing, Environment, TestResult};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The anthropic wire form of the effort medium (MODEL-010 wire table).
fn adaptive_medium() -> serde_json::Value {
    json!({
        "thinking": {"type": "adaptive", "display": "summarized"},
        "output_config": {"effort": "medium"}
    })
}

fn local_provider(server: &MockServer, provider: &ClaudeProvider) -> TestResult<ClaudeProvider> {
    let client = anthropic::Client::<reqwest::Client>::builder()
        .api_key("sk-ant-oat01-prov168")
        .base_url(server.uri())
        .http_headers(provider.client().headers().clone())
        .http_client(provider.client().http_client().clone())
        .build()?;
    Ok(ClaudeProvider::from_test_client(
        client,
        "claude-opus-5-5",
        AuthMode::OAuth,
    )?)
}

#[tokio::test]
#[serial_test::serial]
async fn current_model_requests_preserve_the_oauth_contract() -> TestResult {
    // @step Given a subscription session selects Opus 5.5 and medium thinking
    let environment = Environment::new()?;
    environment.install("2.1.316")?;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id":"msg_prov168", "type":"message", "role":"assistant",
            "model":"claude-opus-5-5", "content":[{"type":"text","text":"5"}],
            "stop_reason":"end_turn", "stop_sequence":null,
            "usage":{"input_tokens":10,"output_tokens":1}
        })))
        .expect(2)
        .mount(&server)
        .await;

    // @step When a request is sent through the Anthropic provider
    for source in [oauth()?, refreshing()?] {
        let provider = local_provider(&server, &source)?;
        let agent = provider.create_rig_agent(
            uuid::Uuid::new_v4(),
            Some("Be concise."),
            Some(adaptive_medium()),
        );
        let response = agent
            .completion("what is 3 + 2", vec![])
            .await?
            .send()
            .await?;
        assert_eq!(response.usage.output_tokens, 1);
    }

    // @step Then the request preserves bearer authentication OAuth headers Claude identity and adaptive effort without unrelated feature betas
    let requests = server.received_requests().await.ok_or("missing requests")?;
    assert_eq!(requests.len(), 2);
    for request in requests {
        let body: serde_json::Value = serde_json::from_slice(&request.body)?;
        assert_eq!(request.url.path(), "/v1/messages");
        assert_eq!(request.url.query(), Some("beta=true"));
        assert_eq!(
            request.headers["authorization"],
            "Bearer sk-ant-oat01-prov168"
        );
        assert!(!request.headers.contains_key("x-api-key"));
        assert_eq!(
            request.headers["user-agent"],
            "claude-cli/2.1.316 (external, cli)"
        );
        assert_eq!(request.headers["x-app"], "cli");
        assert_eq!(request.headers["anthropic-version"], "2023-06-01");
        let betas = request.headers["anthropic-beta"].to_str()?;
        assert!(betas.contains("oauth-2025-04-20"));
        assert!(betas.contains("claude-code-20250219"));
        for unsupported in [
            "context-management-",
            "context-1m-",
            "per-turn-control-",
            "extended-cache-ttl-",
            "interleaved-thinking-",
        ] {
            assert!(!betas.contains(unsupported), "{betas}");
        }
        assert!(body["system"][0]["text"]
            .as_str()
            .ok_or("missing identity")?
            .starts_with("You are Claude Code, Anthropic's official CLI for Claude."));
        assert_eq!(body["model"], "claude-opus-5-5");
        assert_eq!(
            body["thinking"],
            json!({"type":"adaptive","display":"summarized"})
        );
        assert_eq!(body["output_config"]["effort"], "medium");
        assert!(body["thinking"]["budget_tokens"].is_null());
    }
    drop(environment);
    if std::env::var("FSPEC_PROV168_LIVE").as_deref() == Ok("1") {
        verify_authorized_request().await?;
    }
    Ok(())
}

async fn verify_authorized_request() -> TestResult {
    let auth = codelet_providers::claude_auth::read_claude_auth()
        .await?
        .ok_or("live verification requires stored fspec Claude OAuth credentials")?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis();
    let remaining = u64::try_from(u128::from(auth.expires).saturating_sub(now) / 1000)?;
    let provider = ClaudeProvider::from_oauth_tokens(
        &auth.access_token,
        &auth.refresh_token,
        Some(remaining),
        codelet_providers::claude_oauth::CLAUDE_TOKEN_ENDPOINT_BASE,
        "claude-opus-5-5",
    )?;
    let agent = provider.create_rig_agent(
        uuid::Uuid::new_v4(),
        Some("Answer with only the numeral. Do not use tools."),
        Some(adaptive_medium()),
    );
    let mut stream = agent
        .completion("what is 3 + 2", vec![])
        .await?
        .max_tokens(1024)
        .stream()
        .await?;
    let mut answer = String::new();
    let mut terminal_usage = None;
    while let Some(event) = stream.next().await {
        match event? {
            StreamedAssistantContent::Text(text) => answer.push_str(&text.text),
            StreamedAssistantContent::Final(response) => terminal_usage = response.token_usage(),
            StreamedAssistantContent::ToolCall(_) => {
                return Err("live arithmetic request used a tool".into())
            }
            _ => {}
        }
    }
    assert_eq!(answer.trim(), "5");
    let usage = terminal_usage.ok_or("live request did not return complete usage")?;
    assert!(usage.input_tokens > 0);
    assert!(usage.output_tokens > 0);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn rejected_client_versions_provide_actionable_errors() -> TestResult {
    // @step Given Anthropic rejects the client version with a higher minimum and a request identifier
    let environment = Environment::new()?;
    environment.install("2.1.316")?;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "type":"error", "error":{"type":"invalid_request_error",
            "message":"Claude Code 2.1.316 does not support this model; version 2.1.999 or newer is required.",
            "details":{"error_code":"claude_code_version_too_old"}},
            "request_id":"req_prov168"
        })))
        .expect(4)
        .mount(&server).await;

    // @step When a subscription request fails before returning usage
    for source in [oauth()?, refreshing()?] {
        let provider = local_provider(&server, &source)?;
        let agent = provider.create_rig_agent(uuid::Uuid::new_v4(), None, None);
        let request = || {
            CompletionRequestBuilder::new((*agent.model).clone(), "what is 3 + 2")
                .max_tokens(64)
                .build()
        };
        let mut stream = agent.model.stream(request()).await?;
        let mut errors = Vec::new();
        while let Some(event) = stream.next().await {
            match event {
                Err(error) => errors.push(error.to_string()),
                Ok(StreamedAssistantContent::Final(_) | StreamedAssistantContent::Usage(_)) => {
                    return Err("rejection fabricated successful usage".into())
                }
                _ => {}
            }
        }
        let completion = agent.model.completion(request()).await;
        let Err(error) = completion else {
            return Err("rejection completed successfully".into());
        };
        errors.push(error.to_string());

        // @step Then the error preserves the server details and explains updating the version source without retrying
        assert_eq!(errors.len(), 2);
        for message in errors {
            for required in [
                "claude_code_version_too_old",
                "2.1.999",
                "req_prov168",
                "claude update",
                "FSPEC_CLAUDE_CODE_VERSION",
            ] {
                assert!(message.contains(required), "missing {required}: {message}");
            }
        }
    }
    server.verify().await;
    Ok(())
}
