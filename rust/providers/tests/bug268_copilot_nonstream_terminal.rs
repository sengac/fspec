//! Feature: spec/features/copilot-nonstreaming-terminal-evidence.feature

#[path = "../../agent-loop/tests/support/bug268_environment.rs"]
mod environment;

use codelet_providers::{
    copilot::{CopilotAuthJson, CopilotDeploymentType, CopilotProvider},
    LlmProvider, StopReason,
};
use environment::{Environment, TestResult};
use rig::{
    client::CompletionClient,
    completion::{CompletionModel, CompletionRequestBuilder},
    message::AssistantContent,
};
use serde_json::{json, Value};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

async fn provider(status: &str, reason: Option<&str>) -> TestResult<(CopilotProvider, MockServer)> {
    let server = MockServer::start().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(200).set_body_json(json!({
        "id":"synthetic-terminal","object":"response","created_at":1,"model":"gpt-5","status":status,
        "error":if status == "failed" {json!({"code":"synthetic_backend_failure","message":"synthetic backend failed"})} else {Value::Null},
        "incomplete_details":reason.map(|reason| json!({"reason":reason})),
        "output":[{"type":"message","role":"assistant","id":"synthetic-message","status":"completed",
            "content":[{"type":"output_text","text":"synthetic partial answer","annotations":[]}]},
            {"type":"function_call","id":"synthetic-item","call_id":"synthetic-call","name":"Done","arguments":"{}","status":"incomplete"}],
        "usage":{"input_tokens":4,"output_tokens":2,"total_tokens":6}
    }))).mount(&server).await;
    let result = CopilotProvider::from_auth(
        CopilotDeploymentType::GitHubCom,
        CopilotAuthJson {
            github_oauth_token: "synthetic-account".into(),
            copilot_token: Some("synthetic-short-token".into()),
            copilot_token_expires_at: Some(u64::MAX),
            endpoints_api: Some(server.uri()),
            enterprise_url: None,
        },
        "gpt-5",
    )?;
    Ok((result, server))
}

#[tokio::test]
#[serial_test::serial]
async fn nonstreaming_responses_failures_cannot_authorize_tools_or_completion() -> TestResult {
    // @step Given a local Copilot Responses endpoint returns failed or unsettled output containing a tool call
    let _environment = Environment::new()?;
    for status in ["failed", "in_progress", "queued", "cancelled"] {
        let (provider, _server) = provider(status, None).await?;
        let model = provider.client().completion_model("gpt-5");
        // @step When both public nonstreaming front doors consume that response
        let direct = model
            .completion(CompletionRequestBuilder::new(model.clone(), "task").build())
            .await;
        let public = provider
            .complete_with_tools(&[codelet_common::Message::user("task")], &[])
            .await;
        // @step Then each returns an error instead of a successful completion or tool request
        assert!(direct.is_err(), "client accepted {status}");
        assert!(public.is_err(), "provider accepted {status}");
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn nonstreaming_responses_truncation_preserves_its_terminal_semantics() -> TestResult {
    // @step Given a local Copilot Responses endpoint returns token limited text and an unfinished tool call
    let _environment = Environment::new()?;
    let (provider, _server) = provider("incomplete", Some("max_output_tokens")).await?;
    let model = provider.client().completion_model("gpt-5");
    // @step When the actual client model and provider consume that response
    let response = model
        .completion(CompletionRequestBuilder::new(model.clone(), "task").build())
        .await?;
    let public = provider
        .complete_with_tools(&[codelet_common::Message::user("task")], &[])
        .await?;
    // @step Then text remains available with a maximum token stop and no executable tool call
    assert!(!response
        .choice
        .iter()
        .any(|part| matches!(part, AssistantContent::ToolCall(_))));
    assert!(response.choice.iter().any(|part| matches!(part, AssistantContent::Text(text) if text.text == "synthetic partial answer")));
    assert_eq!(public.stop_reason, StopReason::MaxTokens);
    assert!(!serde_json::to_string(&public.content)?.contains("synthetic-call"));
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn nonstreaming_responses_safety_stops_cannot_authorize_tools() -> TestResult {
    // @step Given a local Copilot Responses endpoint returns content filtered text alongside an unfinished tool call
    let _environment = Environment::new()?;
    let (provider, _server) = provider("incomplete", Some("content_filter")).await?;
    let model = provider.client().completion_model("gpt-5");
    // @step When the actual client model and provider consume that response
    let response = model
        .completion(CompletionRequestBuilder::new(model.clone(), "task").build())
        .await?;
    let public = provider
        .complete_with_tools(&[codelet_common::Message::user("task")], &[])
        .await;
    // @step Then the client retains the safety stop without executable tools and the provider cannot report ordinary completion
    assert!(!response
        .choice
        .iter()
        .any(|part| matches!(part, AssistantContent::ToolCall(_))));
    assert!(serde_json::to_string(&response.raw_response)?.contains("content_filter"));
    assert!(public.is_err());
    Ok(())
}
