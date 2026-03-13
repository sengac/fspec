use super::*;
use rig::agent::MultiTurnStreamItem;

#[derive(Clone)]
struct TestStreamResponse;

impl rig::completion::GetTokenUsage for TestStreamResponse {
    fn token_usage(&self) -> Option<rig::completion::Usage> {
        None
    }
}

// Scenario: Codex DeepSearch uses a streaming-compatible execution path

// @step Given a DeepSearch sub-agent is constructed for provider "codex"
// @step When the sub-agent executes the query
#[test]
fn codex_requires_streaming_execution_path() {
    // @step Then the execution path consumes a streaming response to completion
    // @step And the final synthesized answer is returned as one String result
    assert!(provider_uses_streaming_execution("codex"));
}

// Scenario: Non-Codex providers keep the existing non-streaming execution path

// @step Given a DeepSearch sub-agent is constructed for provider "claude"
// @step When the sub-agent executes the query
#[test]
fn non_codex_providers_remain_non_streaming() {
    // @step Then the execution path remains non-streaming
    // @step And the final synthesized answer contract remains unchanged
    for provider in ["claude", "openai", "gemini", "zai"] {
        assert!(!provider_uses_streaming_execution(provider));
    }
}

// Scenario: Streaming collection returns only the final DeepSearch answer contract

// @step Given a DeepSearch streaming response contains intermediate assistant chunks
#[tokio::test]
async fn collect_final_response_from_stream_uses_final_response_only() {
    let stream = futures::stream::iter(vec![
        Ok(MultiTurnStreamItem::StreamAssistantItem(
            rig::streaming::StreamedAssistantContent::<TestStreamResponse>::text("intermediate chunk"),
        )),
        Ok(MultiTurnStreamItem::final_response(
            "final synthesized answer",
            rig::completion::Usage::new(),
        )),
    ]);

    // @step When the streaming collection completes
    let collected = collect_final_response_from_stream(stream)
        .await
        .expect("stream should produce final response");

    // @step Then DeepSearch returns only the final synthesized answer text to the caller
    // @step And raw streaming chunks are not returned from the DeepSearch tool call
    assert_eq!(collected, "final synthesized answer");
}

#[tokio::test]
async fn collect_final_response_from_stream_errors_when_final_missing() {
    let stream = futures::stream::iter(vec![Ok(MultiTurnStreamItem::StreamAssistantItem(
        rig::streaming::StreamedAssistantContent::<TestStreamResponse>::text("chunk without final"),
    ))]);

    let result = collect_final_response_from_stream(stream).await;
    assert!(result.is_err());
    assert!(
        result
            .expect_err("expected missing final response error")
            .contains("missing final response")
    );
}

#[tokio::test]
async fn collect_final_response_from_stream_propagates_stream_errors() {
    let stream = futures::stream::iter(vec![Err::<MultiTurnStreamItem<TestStreamResponse>, anyhow::Error>(
        anyhow::anyhow!("stream blew up"),
    )]);

    let result = collect_final_response_from_stream(stream).await;
    assert!(result.is_err());
    assert!(
        result
            .expect_err("expected stream error")
            .contains("stream blew up")
    );
}
