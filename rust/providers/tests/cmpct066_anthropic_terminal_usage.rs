//! Feature: spec/features/anthropic-terminal-cache-usage.feature

use codelet_core::compaction_hook::{CompactionHook, TokenState};
use futures::StreamExt;
use rig::agent::{CancelSignal, StreamingPromptHook};
use rig::client::CompletionClient;
use rig::completion::{CompletionModel, CompletionRequestBuilder, GetTokenUsage};
use rig::providers::anthropic;
use rig::streaming::StreamedAssistantContent;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn body(cache: Option<u64>) -> String {
    let mut usage = json!({"input_tokens":600,"output_tokens":0});
    if let Some(cache) = cache {
        usage["cache_read_input_tokens"] = json!(cache);
        usage["cache_creation_input_tokens"] = json!(cache / 8);
    }
    [
        json!({"type":"message_start","message":{
            "id":"msg_test","role":"assistant","model":"test-model",
            "content":[],"stop_reason":null,"stop_sequence":null,"usage":usage
        }}),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"done"}}),
        json!({"type":"content_block_stop","index":0}),
        json!({"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":100}}),
        json!({"type":"message_stop"}),
    ].into_iter().map(|event| format!("event: {}\ndata: {event}\n\n", event["type"].as_str().unwrap_or("message"))).collect()
}

#[tokio::test]
async fn terminal_usage_without_a_request_start_is_rejected() -> TestResult {
    // @step Given an Anthropic stream has no measured message start usage
    let server = MockServer::start().await;
    let model = model(&server)?;
    let script = body(None);
    let (_, without_start) = script.split_once("\n\n").ok_or("SSE start missing")?;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(without_start.to_string()),
        )
        .mount(&server)
        .await;
    // @step When a terminal delta reports output without input measurements
    let mut stream = model
        .stream(
            CompletionRequestBuilder::new(model.clone(), "hello")
                .max_tokens(200)
                .build(),
        )
        .await?;
    let mut errors = Vec::new();
    while let Some(event) = stream.next().await {
        if let Err(error) = event {
            errors.push(error.to_string());
        }
    }
    // @step Then the adapter reports incomplete usage instead of a measured zero input
    assert!(
        errors
            .iter()
            .any(|error| error.contains("incomplete Anthropic usage")),
        "{errors:?}"
    );
    Ok(())
}

#[tokio::test]
async fn overflowing_cache_occupancy_rejects_successful_terminal_response() -> TestResult {
    // @step Given an Anthropic stream reports cache usage that overflows total occupancy
    let server = MockServer::start().await;
    let model = model(&server)?;
    let script = body(Some(400)).replace(
        "\"cache_read_input_tokens\":400",
        &format!("\"cache_read_input_tokens\":{}", u64::MAX - 700),
    );
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(script),
        )
        .expect(1)
        .mount(&server)
        .await;
    // @step When the terminal delta completes the request
    let mut stream = model
        .stream(
            CompletionRequestBuilder::new(model.clone(), "hello")
                .max_tokens(200)
                .build(),
        )
        .await?;
    let mut errors = Vec::new();
    let mut finals = 0;
    while let Some(event) = stream.next().await {
        match event {
            Err(error) => errors.push(error.to_string()),
            Ok(StreamedAssistantContent::Final(_)) => finals += 1,
            _ => {}
        }
    }
    server.verify().await;
    // @step Then the adapter reports invalid usage without publishing a successful terminal response
    assert_eq!(finals, 0, "invalid usage must not finish successfully");
    assert!(
        errors
            .iter()
            .any(|error| error.contains("invalid Anthropic usage")),
        "{errors:?}"
    );
    let raw: anthropic::completion::Usage = serde_json::from_value(json!({
        "input_tokens":600,"output_tokens":100,
        "cache_read_input_tokens":u64::MAX - 700,"cache_creation_input_tokens":50
    }))?;
    assert!(raw.token_usage().is_none());
    Ok(())
}

async fn capture(
    model: &anthropic::completion::CompletionModel,
    server: &MockServer,
    cache: Option<u64>,
) -> Result<
    (
        Vec<rig::completion::Usage>,
        anthropic::streaming::StreamingCompletionResponse,
    ),
    Box<dyn std::error::Error>,
> {
    server.reset().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(body(cache)),
        )
        .expect(1)
        .mount(server)
        .await;
    let request = CompletionRequestBuilder::new(model.clone(), "hello")
        .max_tokens(200)
        .build();
    let mut stream = model.stream(request).await?;
    let mut live = Vec::new();
    let mut terminal = None;
    while let Some(event) = stream.next().await {
        match event? {
            StreamedAssistantContent::Usage(usage) => live.push(usage),
            StreamedAssistantContent::Final(response) => terminal = Some(response),
            _ => {}
        }
    }
    server.verify().await;
    Ok((live, terminal.ok_or("terminal response missing")?))
}

fn model(
    server: &MockServer,
) -> Result<anthropic::completion::CompletionModel, Box<dyn std::error::Error>> {
    Ok(anthropic::Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()?
        .completion_model("test-model"))
}

#[tokio::test]
async fn terminal_cache_measurements_survive_the_stream() -> TestResult {
    // @step Given an Anthropic stream starts with fresh input 600 cache read 400 and cache creation 50
    let server = MockServer::start().await;
    let model = model(&server)?;
    // @step When the terminal delta reports output 100 without repeating cache fields
    let (live, terminal) = capture(&model, &server, Some(400)).await?;
    // @step Then live and final usage retain cache read 400 and cache creation 50
    assert!(!live.is_empty(), "missing live usage");
    let final_usage = terminal.token_usage().ok_or("missing terminal usage")?;
    for usage in live.into_iter().chain([final_usage]) {
        assert_eq!(usage.cache_read_input_tokens, Some(400));
        assert_eq!(usage.cache_creation_input_tokens, Some(50));
    }
    let state = Arc::new(Mutex::new(TokenState::default()));
    let hook = CompactionHook::new(state.clone(), 2000);
    <CompactionHook as StreamingPromptHook<anthropic::completion::CompletionModel>>::on_stream_completion_response_finish(
        &hook, &rig::message::Message::user("hello"), &terminal, CancelSignal::new(),
    ).await;
    // @step And the finish hook records occupancy 1150
    assert_eq!(
        state.lock().map_err(|_| "hook lock poisoned")?.total(),
        1150
    );
    Ok(())
}

#[tokio::test]
async fn adjacent_requests_do_not_share_cache_evidence() -> TestResult {
    // @step Given a completed cached Anthropic request
    let server = MockServer::start().await;
    let model = model(&server)?;
    capture(&model, &server, Some(400)).await?;
    // @step When the next request omits cache measurements
    let (_, terminal) = capture(&model, &server, None).await?;
    // @step Then the next final usage preserves absent cache fields without borrowing prior measurements
    let usage = terminal.token_usage().ok_or("terminal usage missing")?;
    assert_eq!(usage.cache_read_input_tokens, None);
    assert_eq!(usage.cache_creation_input_tokens, None);
    Ok(())
}

#[tokio::test]
async fn measured_zero_cache_remains_distinct_from_absent_cache() -> TestResult {
    // @step Given an Anthropic request explicitly reports zero cache read and creation
    let server = MockServer::start().await;
    let model = model(&server)?;
    // @step When its terminal delta completes the request
    let (_, terminal) = capture(&model, &server, Some(0)).await?;
    let usage = terminal.token_usage().ok_or("terminal usage missing")?;
    // @step Then the final usage retains measured zero cache fields
    assert_eq!(usage.cache_read_input_tokens, Some(0));
    assert_eq!(usage.cache_creation_input_tokens, Some(0));
    let encoded: Value = serde_json::to_value(usage)?;
    assert_eq!(encoded["cache_read_input_tokens"], 0);
    Ok(())
}

#[tokio::test]
async fn nonstreaming_cache_usage_agrees_with_terminal_measurements() -> TestResult {
    // @step Given an Anthropic nonstreaming response reports fresh input 600 cache read 400 cache creation 50 and output 100
    let server = MockServer::start().await;
    let model = model(&server)?;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id":"msg_test","type":"message","role":"assistant","model":"test-model",
            "content":[{"type":"text","text":"done"}],
            "stop_reason":"end_turn","stop_sequence":null,
            "usage":{"input_tokens":600,"output_tokens":100,
                "cache_read_input_tokens":400,"cache_creation_input_tokens":50}
        })))
        .expect(1)
        .mount(&server)
        .await;
    // @step When the real adapter normalizes the completion and its raw usage
    let response = model
        .completion(
            CompletionRequestBuilder::new(model.clone(), "hello")
                .max_tokens(200)
                .build(),
        )
        .await?;
    let raw_usage = response
        .raw_response
        .usage
        .token_usage()
        .ok_or("raw usage missing")?;
    server.verify().await;
    // @step Then both usages retain occupancy 1150 with the same disjoint cache categories
    for usage in [response.usage, raw_usage] {
        assert_eq!(usage.total_tokens, 1150);
        assert_eq!(usage.input_tokens, 600);
        assert_eq!(usage.output_tokens, 100);
        assert_eq!(usage.cache_read_input_tokens, Some(400));
        assert_eq!(usage.cache_creation_input_tokens, Some(50));
    }
    Ok(())
}
