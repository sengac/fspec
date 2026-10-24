//! Feature: spec/features/responses-api-streaming-usage-events.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios.
//!
//! Tests verify that Responses API streams normalize inclusive terminal usage
//! and reject missing terminal usage without publishing successful completion.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rig::completion::{CompletionError, GetTokenUsage};
use rig::providers::openai::responses_api::streaming::StreamingCompletionResponse as ResponsesStreamingCompletionResponse;

#[path = "prov_038_responses_api_usage_events/support.rs"]
mod support;
use support::{
    MockHttpClient, make_usage_with_details, make_usage_without_details, mock_http_client,
};

// =============================================================================
// Scenario: Streaming with inclusive usage emits normalized Usage before FinalResponse
// =============================================================================

#[test]
fn test_streaming_response_completed_with_usage_normalizes_inclusive_counts() {
    // @step Given a Responses API streaming session receives text deltas

    // @step And the response.completed event contains usage with input_tokens 1000, output_tokens 500, total_tokens 1500, cached_tokens 200, and reasoning_tokens 300
    let usage = make_usage_with_details(1000, 500, 1500, 200, 300);

    // @step When the streaming response is processed
    let streaming_resp = ResponsesStreamingCompletionResponse {
        usage: usage.clone(),
        request_context: None,
        request_usage: Vec::new(),
    };
    let crate_usage = streaming_resp.token_usage().expect("should return usage");

    // @step Then a RawStreamingChoice::Usage event should be yielded with input_tokens 800
    assert_eq!(crate_usage.input_tokens, 800);

    // @step And the Usage event should have output_tokens 200 and total_tokens 1500
    assert_eq!(crate_usage.output_tokens, 200);
    assert_eq!(crate_usage.total_tokens, 1500);

    // @step And the Usage event should have cache_read_input_tokens 200 and reasoning_tokens 300
    assert_eq!(crate_usage.cache_read_input_tokens, Some(200));
    assert_eq!(crate_usage.reasoning_tokens, Some(300));
}

// =============================================================================
// Scenario: Streaming without terminal usage reports an error without terminal success
// =============================================================================

#[tokio::test]
async fn test_streaming_response_completed_without_usage_rejects_terminal_success() {
    use bytes::Bytes;
    use futures::StreamExt;
    use rig::client::CompletionClient;
    use rig::completion::CompletionModel;

    // @step Given a Responses API streaming session receives text deltas

    // @step And the response.completed event contains no usage data
    let sse = concat!(
        "event: response.created\ndata: {\"type\":\"response.created\",\"response\":{\"id\":\"resp-002\",\"object\":\"response\",\"created_at\":1234567890,\"status\":\"in_progress\",\"error\":null,\"incomplete_details\":null,\"instructions\":null,\"max_output_tokens\":null,\"model\":\"gpt-5.3-codex\",\"usage\":null,\"output\":[],\"tools\":[]},\"sequence_number\":0}\n\n",
        "event: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"item_id\":\"item-001\",\"output_index\":0,\"content_index\":0,\"sequence_number\":1,\"delta\":\"Hello\"}\n\n",
        "event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp-002\",\"object\":\"response\",\"created_at\":1234567890,\"status\":\"completed\",\"error\":null,\"incomplete_details\":null,\"instructions\":null,\"max_output_tokens\":null,\"model\":\"gpt-5.3-codex\",\"usage\":null,\"output\":[{\"type\":\"message\",\"id\":\"msg-001\",\"role\":\"assistant\",\"status\":\"completed\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hello\"}]}],\"tools\":[]},\"sequence_number\":2}\n\n",
    );

    let mock = mock_http_client(Bytes::from(sse));

    let client = rig::providers::openai::Client::<MockHttpClient>::builder()
        .api_key("test-key")
        .base_url("http://test.local/v1")
        .http_client(mock)
        .build()
        .expect("should build client");

    let model = client.completion_model("gpt-5.3-codex");

    let request = rig::completion::CompletionRequest {
        preamble: Some("Test".to_string()),
        chat_history: rig::OneOrMany::one(rig::completion::Message::user("Hello")),
        tools: vec![],
        documents: vec![],
        temperature: None,
        max_tokens: None,
        additional_params: None,
        tool_choice: None,
    };

    // @step When the streaming response is processed
    let mut stream: rig::streaming::StreamingCompletionResponse<
        rig::providers::openai::responses_api::streaming::StreamingCompletionResponse,
    > = model.stream(request).await.expect("stream should start");

    let mut saw_usage = false;
    let mut saw_final_response = false;
    let mut errors = Vec::new();

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(rig::streaming::StreamedAssistantContent::Usage(_)) => {
                saw_usage = true;
            }
            Ok(rig::streaming::StreamedAssistantContent::Final(_)) => {
                saw_final_response = true;
            }
            Err(error) => errors.push(error),
            _ => {}
        }
    }

    // @step Then a ResponseError with message "missing Responses usage" should be returned
    assert_eq!(
        errors.len(),
        1,
        "Missing usage must return exactly one error"
    );
    assert!(matches!(
        &errors[0],
        CompletionError::ResponseError(message) if message == "missing Responses usage"
    ));
    // @step And no RawStreamingChoice::Usage event should be yielded
    assert!(
        !saw_usage,
        "Stream should NOT emit a Usage event when response.completed has no usage data"
    );

    // @step And no FinalResponse should be yielded
    assert!(
        !saw_final_response,
        "Stream must not emit FinalResponse without terminal usage evidence"
    );
}

// =============================================================================
// Scenario: Streaming with usage but no input_tokens_details handles missing optional fields
// =============================================================================

#[test]
fn test_streaming_response_completed_with_usage_no_details_handles_optionals() {
    // @step Given a Responses API streaming session receives text deltas

    // @step And the response.completed event contains usage with input_tokens 5000, output_tokens 2000, total_tokens 7000 but no input_tokens_details
    let usage = make_usage_without_details(5000, 2000, 7000);

    // @step When the streaming response is processed
    let streaming_resp = ResponsesStreamingCompletionResponse {
        usage,
        request_context: None,
        request_usage: Vec::new(),
    };
    let crate_usage = streaming_resp.token_usage().expect("should return usage");

    // @step Then a RawStreamingChoice::Usage event should be yielded with input_tokens 5000
    assert_eq!(crate_usage.input_tokens, 5000);
    assert_eq!(crate_usage.output_tokens, 2000);
    assert_eq!(crate_usage.total_tokens, 7000);

    // @step And the Usage event should have cache_read_input_tokens None and reasoning_tokens 0
    assert_eq!(crate_usage.cache_read_input_tokens, None);
    assert_eq!(crate_usage.reasoning_tokens, Some(0));
}

// =============================================================================
// Integration test: Responses API SSE stream yields Usage before FinalResponse
// =============================================================================

#[tokio::test]
async fn test_responses_api_sse_stream_yields_usage_event() {
    use bytes::Bytes;
    use futures::StreamExt;
    use rig::client::CompletionClient;
    use rig::completion::CompletionModel;

    // SSE events in Responses API format:
    // 1. response.created
    // 2. response.output_text.delta
    // 3. response.completed with usage
    let sse = concat!(
        "event: response.created\ndata: {\"type\":\"response.created\",\"response\":{\"id\":\"resp-001\",\"object\":\"response\",\"created_at\":1234567890,\"status\":\"in_progress\",\"error\":null,\"incomplete_details\":null,\"instructions\":null,\"max_output_tokens\":null,\"model\":\"gpt-5.3-codex\",\"usage\":null,\"output\":[],\"tools\":[]},\"sequence_number\":0}\n\n",
        "event: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"item_id\":\"item-001\",\"output_index\":0,\"content_index\":0,\"sequence_number\":1,\"delta\":\"Hello\"}\n\n",
        "event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp-001\",\"object\":\"response\",\"created_at\":1234567890,\"status\":\"completed\",\"error\":null,\"incomplete_details\":null,\"instructions\":null,\"max_output_tokens\":null,\"model\":\"gpt-5.3-codex\",\"usage\":{\"input_tokens\":1000,\"input_tokens_details\":{\"cached_tokens\":200},\"output_tokens\":500,\"output_tokens_details\":{\"reasoning_tokens\":300},\"total_tokens\":1500},\"output\":[{\"type\":\"message\",\"id\":\"msg-001\",\"role\":\"assistant\",\"status\":\"completed\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hello\"}]}],\"tools\":[]},\"sequence_number\":2}\n\n",
    );

    let mock = mock_http_client(Bytes::from(sse));

    // Build an OpenAI Responses API client with mock HTTP
    let client = rig::providers::openai::Client::<MockHttpClient>::builder()
        .api_key("test-key")
        .base_url("http://test.local/v1")
        .http_client(mock)
        .build()
        .expect("should build client");

    let model = client.completion_model("gpt-5.3-codex");

    // Build a minimal completion request
    let request = rig::completion::CompletionRequest {
        preamble: Some("Test".to_string()),
        chat_history: rig::OneOrMany::one(rig::completion::Message::user("Hello")),
        tools: vec![],
        documents: vec![],
        temperature: None,
        max_tokens: None,
        additional_params: None,
        tool_choice: None,
    };

    // Execute the stream
    let mut stream: rig::streaming::StreamingCompletionResponse<
        rig::providers::openai::responses_api::streaming::StreamingCompletionResponse,
    > = model.stream(request).await.expect("stream should start");

    // Collect all events from the stream
    let mut observed_usage = None;
    let mut final_usage = None;
    let mut terminal_events = Vec::new();

    while let Some(chunk) = stream.next().await {
        match chunk.expect("valid terminal usage must not return a stream error") {
            rig::streaming::StreamedAssistantContent::Usage(usage) => {
                terminal_events.push("usage");
                observed_usage = Some(usage);
            }
            rig::streaming::StreamedAssistantContent::Final(response) => {
                terminal_events.push("final");
                final_usage = response.token_usage();
            }
            _ => {}
        }
    }

    // @step And the Usage event should be yielded before the FinalResponse
    assert_eq!(terminal_events, vec!["usage", "final"]);

    // @step And exactly one FinalResponse should contain the same normalized usage
    let usage = observed_usage.expect("response.completed must emit measured usage");
    assert_eq!(final_usage, Some(usage));

    // Verify the Usage values normalize inclusive response.completed data
    assert_eq!(usage.input_tokens, 800, "input excludes the cached subset");
    assert_eq!(usage.output_tokens, 200, "output excludes reasoning tokens");
    assert_eq!(usage.total_tokens, 1500, "inclusive total is preserved");
    assert_eq!(usage.cache_read_input_tokens, Some(200));
    assert_eq!(usage.reasoning_tokens, Some(300));
}
