#![allow(clippy::unwrap_used, clippy::expect_used)]
#![cfg(feature = "test-support")]
//! Z.AI Provider Integration Tests
//!
//! Feature: spec/features/z-ai-glm-provider-integration.feature
//!
//! Tests for PROV-004: Z.AI GLM Provider Integration. BUG-517: the endpoint,
//! authentication and streaming scenarios check the real client: the base URL
//! each key selects, and (through the test-support base-URL seam
//! `ZAIProvider::new_with_model_at_test_endpoint`) the Bearer key, model and
//! streamed tool calls a loopback server receives and sends.
//!
//! The tests change the process environment, so they all run under
//! serial_test's default `#[serial]` key.

use codelet_common::Message;
use codelet_providers::{LlmProvider, ZAIProvider};
use futures::StreamExt;
use rig::streaming::StreamedAssistantContent;
use rig::streaming::StreamingCompletion;
use serde_json::{json, Value};
use std::env;
use wiremock::{matchers::method, Mock, MockServer, Request, ResponseTemplate};

const NORMAL_ENDPOINT: &str = "https://api.z.ai/api/paas/v4";
const PLAN_ENDPOINT: &str = "https://api.z.ai/api/coding/paas/v4";

/// Set exactly the given Z.AI keys (the others removed).
fn zai_keys(api_key: Option<&str>, plan_key: Option<&str>) {
    env::remove_var("ZAI_MODEL");
    for (name, value) in [("ZAI_API_KEY", api_key), ("ZAI_PLAN_API_KEY", plan_key)] {
        match value {
            Some(value) => env::set_var(name, value),
            None => env::remove_var(name),
        }
    }
}

fn clear_zai_keys() {
    zai_keys(None, None);
}

/// A loopback Z.AI server answering every chat completion with "hello".
async fn completion_server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "bug517", "object": "chat.completion", "created": 1, "model": "glm-4.7",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "hello"},
                "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 3, "completion_tokens": 1, "total_tokens": 4}
        })))
        .mount(&server)
        .await;
    server
}

/// The one request the server received: its Authorization header and body.
async fn received(server: &MockServer) -> (String, Value) {
    let requests = server.received_requests().await.expect("recorded requests");
    assert_eq!(requests.len(), 1, "expected one request");
    let request = &requests[0];
    let authorization = request
        .headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    (authorization, request.body_json().expect("JSON body"))
}

/// Test: Select Z.AI GLM model with thinking mode
///
/// Scenario: Select Z.AI GLM model with thinking mode
#[test]
#[serial_test::serial]
fn test_select_zai_glm_model_with_thinking_mode() {
    // @step Given the ZAI_API_KEY or ZAI_PLAN_API_KEY environment variable is set
    zai_keys(Some("test-api-key-for-zai"), None);

    // @step And the user opens the fspec TUI model selector
    // TUI model selector would show zai provider when credentials available

    // @step When the user selects zai/glm-4.7 with thinking mode enabled
    let provider = ZAIProvider::new_with_model(Some("glm-4.7")).unwrap();

    // @step Then the agent should be configured with the Z.AI provider
    assert_eq!(provider.name(), "zai");
    assert_eq!(provider.model(), "glm-4.7");

    // @step And streaming responses should include reasoning_content when available
    // Verify thinking/reasoning is supported for glm-4.7
    assert!(provider.supports_reasoning());

    clear_zai_keys();
}

/// Scenario: Error when both ZAI API keys are missing
#[test]
#[serial_test::serial]
fn test_error_when_both_zai_api_keys_are_missing() {
    // @step Given neither ZAI_API_KEY nor ZAI_PLAN_API_KEY environment variable is set
    clear_zai_keys();

    // @step When the user attempts to select the Z.AI provider
    let result = ZAIProvider::new_with_model(Some("glm-4.7"));

    // @step Then the system should display an error message about missing credentials
    let error = format!("{}", result.expect_err("no Z.AI key must fail"));
    assert!(error.contains("Missing"), "{error}");

    // @step And the error should suggest setting ZAI_API_KEY or ZAI_PLAN_API_KEY
    assert!(
        error.contains("ZAI_API_KEY") && error.contains("ZAI_PLAN_API_KEY"),
        "{error}"
    );
}

/// Scenario: ZAI provider uses normal API endpoint
#[tokio::test]
#[serial_test::serial]
async fn test_zai_provider_uses_normal_api_endpoint() {
    // @step Given the ZAI_API_KEY environment variable is set
    // @step And ZAI_PLAN_API_KEY is not set
    zai_keys(Some("test-api-key-for-zai"), None);

    // @step When a ZAIProvider instance is created
    let provider = ZAIProvider::new_with_model(Some("glm-4.7")).unwrap();
    let server = completion_server().await;
    let routed =
        ZAIProvider::new_with_model_at_test_endpoint(Some("glm-4.7"), &server.uri()).unwrap();
    routed.complete(&[Message::user("hi")]).await.unwrap();
    let (authorization, body) = received(&server).await;

    // @step Then it should use the base URL https://api.z.ai/api/paas/v4
    assert_eq!(provider.base_url(), NORMAL_ENDPOINT);
    assert!(!provider.is_plan_endpoint());
    assert_eq!(routed.base_url(), NORMAL_ENDPOINT);

    // @step And it should authenticate using Bearer token
    assert_eq!(authorization, "Bearer test-api-key-for-zai");

    // @step And it should request the model glm-4.7 it was created with
    assert_eq!(body["model"], "glm-4.7");

    clear_zai_keys();
}

/// Scenario: ZAI provider uses coding plan API endpoint
#[tokio::test]
#[serial_test::serial]
async fn test_zai_provider_uses_coding_plan_api_endpoint() {
    // @step Given the ZAI_PLAN_API_KEY environment variable is set
    zai_keys(None, Some("test-plan-key-for-zai"));

    // @step When a ZAIProvider instance is created
    let provider = ZAIProvider::new_with_model(Some("glm-4.7")).unwrap();
    let server = completion_server().await;
    let routed =
        ZAIProvider::new_with_model_at_test_endpoint(Some("glm-4.7"), &server.uri()).unwrap();
    routed.complete(&[Message::user("hi")]).await.unwrap();
    let (authorization, body) = received(&server).await;

    // @step Then it should use the base URL https://api.z.ai/api/coding/paas/v4
    assert_eq!(provider.base_url(), PLAN_ENDPOINT);
    assert!(provider.is_plan_endpoint());
    assert_eq!(routed.base_url(), PLAN_ENDPOINT);

    // @step And it should authenticate using Bearer token
    assert_eq!(authorization, "Bearer test-plan-key-for-zai");

    // @step And it should request the model glm-4.7 it was created with
    assert_eq!(body["model"], "glm-4.7");

    clear_zai_keys();
}

/// Scenario: ZAI_PLAN_API_KEY takes precedence over ZAI_API_KEY
#[tokio::test]
#[serial_test::serial]
async fn test_zai_plan_api_key_takes_precedence_over_zai_api_key() {
    // @step Given both ZAI_API_KEY and ZAI_PLAN_API_KEY environment variables are set
    zai_keys(Some("test-api-key-for-zai"), Some("test-plan-key-for-zai"));

    // @step When a ZAIProvider instance is created
    let provider = ZAIProvider::new_with_model(Some("glm-4.7")).unwrap();
    let server = completion_server().await;
    let routed =
        ZAIProvider::new_with_model_at_test_endpoint(Some("glm-4.7"), &server.uri()).unwrap();
    routed.complete(&[Message::user("hi")]).await.unwrap();
    let (authorization, _) = received(&server).await;

    // @step Then it should use the coding plan endpoint https://api.z.ai/api/coding/paas/v4
    assert_eq!(provider.base_url(), PLAN_ENDPOINT);

    // @step And it should use the ZAI_PLAN_API_KEY for authentication
    assert_eq!(authorization, "Bearer test-plan-key-for-zai");

    clear_zai_keys();
}

/// An OpenAI-style SSE stream: two text deltas, then a `read_file` tool call
/// whose arguments arrive in two pieces.
fn tool_call_stream(request: &Request) -> ResponseTemplate {
    let body: Value = request.body_json().unwrap_or_default();
    assert_eq!(body["stream"], true, "the request must stream");
    let chunk = |delta: Value, finish: Value| {
        json!({"id": "bug517", "object": "chat.completion.chunk", "created": 1,
            "model": "glm-4.7", "choices": [{"index": 0, "delta": delta, "finish_reason": finish}]})
    };
    let call = |id: Option<&str>, name: Option<&str>, arguments: &str| {
        let mut function = json!({"arguments": arguments});
        if let Some(name) = name {
            function["name"] = json!(name);
        }
        let mut tool_call = json!({"index": 0, "type": "function", "function": function});
        if let Some(id) = id {
            tool_call["id"] = json!(id);
        }
        json!({"tool_calls": [tool_call]})
    };
    let events = [
        chunk(
            json!({"role": "assistant", "content": "Let me "}),
            Value::Null,
        ),
        chunk(json!({"content": "read it."}), Value::Null),
        chunk(
            call(Some("call_bug517"), Some("read_file"), "{\"path\":"),
            Value::Null,
        ),
        chunk(call(None, None, "\"/tmp/notes.txt\"}"), Value::Null),
        chunk(json!({}), json!("tool_calls")),
        json!({"id": "bug517", "object": "chat.completion.chunk", "created": 1,
            "model": "glm-4.7", "choices": [],
            "usage": {"prompt_tokens": 12, "completion_tokens": 9, "total_tokens": 21}}),
    ];
    let mut sse: String = events
        .iter()
        .map(|event| format!("data: {event}\n\n"))
        .collect();
    sse.push_str("data: [DONE]\n\n");
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(sse)
}

/// Scenario: Streaming response with tool calls
#[tokio::test]
#[serial_test::serial]
async fn test_streaming_response_with_tool_calls() {
    // @step Given the ZAI_API_KEY or ZAI_PLAN_API_KEY environment variable is set
    zai_keys(Some("test-api-key-for-zai"), None);
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(tool_call_stream)
        .mount(&server)
        .await;

    // @step And a ZAIProvider is configured with tools
    let provider =
        ZAIProvider::new_with_model_at_test_endpoint(Some("glm-4.7"), &server.uri()).unwrap();
    let agent = provider.create_rig_agent(uuid::Uuid::new_v4(), None, None);

    // @step When the user sends a message that requires tool use
    let mut stream = agent
        .stream_completion("Read /tmp/notes.txt", Vec::<rig::message::Message>::new())
        .await
        .unwrap()
        .stream()
        .await
        .unwrap();
    let mut texts = Vec::new();
    let mut tool_calls = Vec::new();
    while let Some(item) = stream.next().await {
        match item.unwrap() {
            StreamedAssistantContent::Text(text) => texts.push(text.text),
            StreamedAssistantContent::ToolCall(tool_call) => tool_calls.push(tool_call),
            _ => {}
        }
    }
    let (_, request) = received(&server).await;
    assert!(
        request["tools"]
            .as_array()
            .is_some_and(|tools| !tools.is_empty()),
        "the request must offer the agent's tools"
    );

    // @step Then the response should stream incrementally
    assert_eq!(texts, vec!["Let me ".to_string(), "read it.".to_string()]);

    // @step And tool calls should be properly parsed from the stream
    assert_eq!(tool_calls.len(), 1, "{tool_calls:?}");
    assert_eq!(tool_calls[0].function.name, "read_file");
    assert_eq!(
        tool_calls[0].function.arguments,
        json!({"path": "/tmp/notes.txt"})
    );

    clear_zai_keys();
}

/// Test: Provider name is correct
#[test]
#[serial_test::serial]
fn test_provider_name() {
    zai_keys(Some("test-key"), None);
    // MODEL-001: new_with_model requires a model
    let provider = ZAIProvider::new_with_model(Some("glm-4.7")).unwrap();

    assert_eq!(provider.name(), "zai");

    clear_zai_keys();
}

/// Test: Reasoning models detection
#[test]
#[serial_test::serial]
fn test_reasoning_models_supported() {
    zai_keys(Some("test-key-for-reasoning-test"), None);

    // GLM-4.7 supports reasoning
    let provider_47 = ZAIProvider::new_with_model(Some("glm-4.7")).unwrap();
    assert!(provider_47.supports_reasoning());

    // GLM-4.6 supports reasoning
    let provider_46 = ZAIProvider::new_with_model(Some("glm-4.6")).unwrap();
    assert!(provider_46.supports_reasoning());

    // GLM-4.5 supports reasoning
    let provider_45 = ZAIProvider::new_with_model(Some("glm-4.5")).unwrap();
    assert!(provider_45.supports_reasoning());

    // Vision models don't support reasoning
    let provider_v = ZAIProvider::new_with_model(Some("glm-4.6v")).unwrap();
    assert!(!provider_v.supports_reasoning());

    clear_zai_keys();
}
