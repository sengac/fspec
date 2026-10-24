#![cfg(feature = "test-support")]

use codelet_providers::stub_model::StubModel;
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder, ToolDefinition};
use rig::message::Message;
use rig::streaming::StreamedAssistantContent;
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
#[serial_test::serial]
async fn stub_preparation_preserves_complete_context_without_invoking_the_adapter() -> TestResult {
    // @step Given a stub request with mandatory instructions tools and structured history
    let model = StubModel::new();
    let captured_before = codelet_providers::stub_model::last_request_bytes();
    let request = CompletionRequestBuilder::new(model.clone(), "next suffix")
        .preamble("mandatory instructions".into())
        .messages(vec![Message::tool_result_with_call_id(
            "fc_prior",
            Some("prior".into()),
            "recorded outcome",
        )])
        .tools(vec![ToolDefinition {
            name: "inspect".into(),
            description: "mandatory schema".into(),
            parameters: json!({"type":"object"}),
        }])
        .additional_params(json!({"thinking":{"budget":123}}))
        .temperature(0.25)
        .max_tokens(321)
        .build();
    // @step When the selected adapter prepares and streams the request
    let bytes = model.prepare_request(request.clone())?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    assert_eq!(
        codelet_providers::stub_model::last_request_bytes(),
        captured_before
    );
    // @step Then prepared bytes preserve every request field and match the invoked adapter bytes
    assert_eq!(value["temperature"], 0.25);
    assert_eq!(value["max_tokens"], 321);
    for required in [
        "mandatory instructions",
        "next suffix",
        "recorded outcome",
        "mandatory schema",
        "123",
    ] {
        assert!(value.to_string().contains(required), "missing {required}");
    }
    let mut stream = model.stream(request).await?;
    while let Some(event) = stream.next().await {
        event?;
    }
    assert_eq!(codelet_providers::stub_model::last_request_bytes(), bytes);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn explicit_stub_tool_submission_uses_the_real_streaming_tool_path() -> TestResult {
    // @step Given a stub request explicitly configuring a tool submission
    let model = StubModel::new();
    let request = CompletionRequestBuilder::new(model.clone(), "submit")
        .additional_params(json!({"stub_tool_call":{
            "id":"stub_call", "name":"inject_summary", "arguments":{"summary":"candidate DAG"}
        }}))
        .build();
    // @step When the streaming adapter consumes the request and its recorded tool outcome
    let mut stream = model.stream(request.clone()).await?;
    let mut tool_calls = 0;
    let mut final_reason = None;
    while let Some(event) = stream.next().await {
        match event? {
            StreamedAssistantContent::ToolCall(call) => {
                assert_eq!(call.id, "stub_call");
                assert_eq!(call.function.name, "inject_summary");
                assert_eq!(call.function.arguments["summary"], "candidate DAG");
                tool_calls += 1;
            }
            StreamedAssistantContent::Final(response) => final_reason = Some(response.stop_reason),
            _ => {}
        }
    }
    // @step Then it emits the configured tool call exactly once and ordinary output afterward
    assert_eq!(tool_calls, 1);
    assert_eq!(final_reason.as_deref(), Some("tool_use"));
    let mut continued = request;
    continued
        .chat_history
        .push(Message::tool_result_with_call_id(
            "stub_call",
            Some("stub_call".into()),
            "recorded outcome",
        ));
    let mut stream = model.stream(continued).await?;
    let mut text = String::new();
    while let Some(event) = stream.next().await {
        match event? {
            StreamedAssistantContent::ToolCall(_) => return Err("tool submission repeated".into()),
            StreamedAssistantContent::Text(delta) => text.push_str(&delta.text),
            _ => {}
        }
    }
    assert_eq!(text, "hi back");
    Ok(())
}
