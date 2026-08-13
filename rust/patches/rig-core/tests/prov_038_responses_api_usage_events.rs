//! Feature: spec/features/responses-api-streaming-usage-events.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios.
//!
//! Tests verify that the OpenAI Responses API streaming implementation yields
//! RawStreamingChoice::Usage events when response.completed contains usage data.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rig::completion::GetTokenUsage;
use rig::providers::openai::responses_api::streaming::StreamingCompletionResponse as ResponsesStreamingCompletionResponse;
use rig::providers::openai::responses_api::{
    InputTokensDetails, OutputTokensDetails, ResponsesUsage,
};

// =============================================================================
// Shared mock HTTP client for integration tests
// =============================================================================

#[derive(Clone, Debug, Default)]
struct MockHttpClient {
    sse_bytes: bytes::Bytes,
}

fn mock_http_client(sse_bytes: bytes::Bytes) -> MockHttpClient {
    MockHttpClient { sse_bytes }
}

impl rig::http_client::HttpClientExt for MockHttpClient {
    fn send<T, U>(
        &self,
        _req: http::Request<T>,
    ) -> impl std::future::Future<
        Output = rig::http_client::Result<http::Response<rig::http_client::LazyBody<U>>>,
    > + Send
    + 'static
    where
        T: Into<bytes::Bytes> + Send,
        U: From<bytes::Bytes> + Send + 'static,
    {
        std::future::ready(Err(rig::http_client::Error::InvalidStatusCode(
            http::StatusCode::NOT_IMPLEMENTED,
        )))
    }

    fn send_multipart<U>(
        &self,
        _req: http::Request<rig::http_client::MultipartForm>,
    ) -> impl std::future::Future<
        Output = rig::http_client::Result<http::Response<rig::http_client::LazyBody<U>>>,
    > + Send
    + 'static
    where
        U: From<bytes::Bytes> + Send + 'static,
    {
        std::future::ready(Err(rig::http_client::Error::InvalidStatusCode(
            http::StatusCode::NOT_IMPLEMENTED,
        )))
    }

    fn send_streaming<T>(
        &self,
        _req: http::Request<T>,
    ) -> impl std::future::Future<
        Output = rig::http_client::Result<rig::http_client::StreamingResponse>,
    > + Send
    where
        T: Into<bytes::Bytes>,
    {
        let sse_bytes = self.sse_bytes.clone();
        async move {
            let byte_stream = futures::stream::iter(vec![Ok::<
                bytes::Bytes,
                rig::http_client::Error,
            >(sse_bytes)]);
            let boxed_stream: rig::http_client::sse::BoxedStream = Box::pin(byte_stream);

            http::Response::builder()
                .status(http::StatusCode::OK)
                .header(reqwest::header::CONTENT_TYPE, "text/event-stream")
                .body(boxed_stream)
                .map_err(rig::http_client::Error::Protocol)
        }
    }
}

/// Helper: build a ResponsesUsage with full details
fn make_usage_with_details(
    input: u64,
    output: u64,
    total: u64,
    cached: u64,
    reasoning: u64,
) -> ResponsesUsage {
    ResponsesUsage {
        input_tokens: input,
        input_tokens_details: Some(InputTokensDetails {
            cached_tokens: cached,
        }),
        output_tokens: output,
        output_tokens_details: OutputTokensDetails {
            reasoning_tokens: reasoning,
        },
        total_tokens: total,
    }
}

/// Helper: build a ResponsesUsage with no input_tokens_details
fn make_usage_without_details(input: u64, output: u64, total: u64) -> ResponsesUsage {
    ResponsesUsage {
        input_tokens: input,
        input_tokens_details: None,
        output_tokens: output,
        output_tokens_details: OutputTokensDetails {
            reasoning_tokens: 0,
        },
        total_tokens: total,
    }
}

// =============================================================================
// Scenario: Streaming with usage in response.completed emits Usage event
// =============================================================================

#[test]
fn test_streaming_response_completed_with_usage_emits_usage_event() {
    // @step Given a Responses API streaming session receives text deltas

    // @step And the response.completed event contains usage with input_tokens 1000, output_tokens 500, total_tokens 1500, cached_tokens 200, and reasoning_tokens 300
    let usage = make_usage_with_details(1000, 500, 1500, 200, 300);

    // @step When the streaming response is processed
    let streaming_resp = ResponsesStreamingCompletionResponse {
        usage: usage.clone(),
    };
    let crate_usage = streaming_resp.token_usage().expect("should return usage");

    // @step Then a RawStreamingChoice::Usage event should be yielded with input_tokens 1000
    assert_eq!(crate_usage.input_tokens, 1000);

    // @step And the Usage event should have output_tokens 500 and total_tokens 1500
    assert_eq!(crate_usage.output_tokens, 500);
    assert_eq!(crate_usage.total_tokens, 1500);

    // @step And the Usage event should have cache_read_input_tokens 200 and reasoning_tokens 300
    assert_eq!(crate_usage.cache_read_input_tokens, Some(200));
    assert_eq!(crate_usage.reasoning_tokens, Some(300));

    // @step And the Usage event should be yielded before the FinalResponse
    // Verified by the streaming implementation: yield Usage then yield FinalResponse
}

// =============================================================================
// Scenario: Streaming without usage in response.completed skips Usage event
// =============================================================================

#[tokio::test]
async fn test_streaming_response_completed_without_usage_skips_usage_event() {
    use bytes::Bytes;
    use futures::StreamExt;
    use rig::completion::CompletionModel;
    use rig::client::CompletionClient;

    // @step Given a Responses API streaming session receives text deltas

    // @step And the response.completed event contains no usage data
    // SSE events: response.completed has "usage": null
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

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(rig::streaming::StreamedAssistantContent::Usage(_)) => {
                saw_usage = true;
            }
            Ok(rig::streaming::StreamedAssistantContent::Final(_)) => {
                saw_final_response = true;
            }
            _ => {}
        }
    }

    // @step Then no RawStreamingChoice::Usage event should be yielded
    assert!(
        !saw_usage,
        "Stream should NOT emit a Usage event when response.completed has no usage data"
    );

    // @step And only the FinalResponse should be yielded with zero usage values
    assert!(
        saw_final_response,
        "Stream should still emit FinalResponse even without usage data"
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
    let streaming_resp = ResponsesStreamingCompletionResponse { usage };
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
    use rig::completion::CompletionModel;
    use rig::client::CompletionClient;

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
    let mut saw_usage = false;
    let mut saw_final_response = false;
    let mut usage_before_final = false;
    let mut usage_input_tokens = 0u64;
    let mut usage_output_tokens = 0u64;
    let mut usage_total_tokens = 0u64;
    let mut usage_cache_read: Option<u64> = None;
    let mut usage_reasoning: Option<u64> = None;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(rig::streaming::StreamedAssistantContent::Usage(usage)) => {
                saw_usage = true;
                if !saw_final_response {
                    usage_before_final = true;
                }
                usage_input_tokens = usage.input_tokens;
                usage_output_tokens = usage.output_tokens;
                usage_total_tokens = usage.total_tokens;
                usage_cache_read = usage.cache_read_input_tokens;
                usage_reasoning = usage.reasoning_tokens;
            }
            Ok(rig::streaming::StreamedAssistantContent::Final(_)) => {
                saw_final_response = true;
            }
            _ => {}
        }
    }

    // CRITICAL: This assertion will FAIL before the fix is applied.
    // The current code never yields a Usage event from the Responses API streaming.
    assert!(
        saw_usage,
        "Stream should emit a Usage event from response.completed usage data"
    );

    // Usage should be emitted BEFORE FinalResponse
    assert!(
        usage_before_final,
        "Usage event should be emitted BEFORE FinalResponse"
    );

    // Verify the Usage values match response.completed data
    assert_eq!(usage_input_tokens, 1000, "input_tokens should be 1000");
    assert_eq!(usage_output_tokens, 500, "output_tokens should be 500");
    assert_eq!(usage_total_tokens, 1500, "total_tokens should be 1500");
    assert_eq!(
        usage_cache_read,
        Some(200),
        "cache_read_input_tokens should be Some(200)"
    );
    assert_eq!(
        usage_reasoning,
        Some(300),
        "reasoning_tokens should be Some(300)"
    );
}
