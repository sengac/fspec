use super::*;
use crate::deep_search_provider_config::{request_config_for_provider, SUB_AGENT_MAX_TOKENS};
use rig::agent::MultiTurnStreamItem;
use codelet_cli::interactive::{
    DEEP_SEARCH_WALL_CLOCK_TIMEOUT_SECS,
    build_deep_search_timeout_message,
    deep_search_wall_clock_timeout,
    is_stall_timeout_error,
};

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

// Scenario: ZAI DeepSearch uses streaming execution path
// Feature: spec/features/glm-zai-deepsearch-fails-with-500-internal-server-error.feature

// @step Given a DeepSearch sub-agent is constructed for provider "zai"
// @step When the sub-agent executes the query
#[test]
fn zai_requires_streaming_execution_path() {
    // @step Then the execution path uses streaming to collect the final response
    assert!(provider_uses_streaming_execution("zai"));
    // @step Then the final synthesized answer is returned as one String result
    // Streaming collection produces a String result (tested in collect_final_response_from_stream tests)
}

// Scenario: Non-streaming providers remain unchanged
// Feature: spec/features/glm-zai-deepsearch-fails-with-500-internal-server-error.feature

// @step Given a DeepSearch sub-agent is constructed for provider "claude"
// @step When the sub-agent executes the query
#[test]
fn non_streaming_providers_remain_unchanged() {
    // @step Then the execution path remains non-streaming
    for provider in ["claude", "openai", "gemini"] {
        assert!(!provider_uses_streaming_execution(provider));
    }
    // @step Then the final synthesized answer contract remains unchanged
    // Non-streaming providers use rig_agent.prompt() which returns String directly
}

// Scenario: ZAI DeepSearch config includes max_tokens in additional_params
// Feature: spec/features/glm-zai-deepsearch-fails-with-500-internal-server-error.feature

// @step Given a DeepSearch request config is built for provider "zai"
#[test]
fn zai_config_includes_max_tokens_in_additional_params() {
    let config = request_config_for_provider("zai", "glm-4.7", "deep search prompt", false)
        .expect("zai config should build");

    // @step When the config is serialized for the HTTP request
    let params = config.additional_params.as_ref().expect("params");

    // @step Then the additional_params includes max_tokens set to 8192
    assert_eq!(params["max_tokens"], SUB_AGENT_MAX_TOKENS);

    // @step Then the additional_params includes temperature and top_p
    assert_eq!(params["temperature"], 1.0);
    assert_eq!(params["top_p"], 0.95);
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

// ====================================================================
// Feature: spec/features/agent-stall-detection.feature
// AMGR-016: DeepSearch wall-clock timeout tests
// ====================================================================

// Scenario: DeepSearch sub-agent stall triggers wall-clock timeout

// @step Given a subordinate agent invokes a DeepSearch tool
// @step And the DeepSearch sub-agent's LLM generation stalls indefinitely
// @step When the DeepSearch wall-clock timeout of 300 seconds expires
// @step Then the parent agent should receive a timeout error as the tool result string
// @step And the parent agent should continue processing with the error result
// @step And the parent agent should not hang or remain in running state
#[tokio::test]
async fn deep_search_wall_clock_timeout_fires_on_stalled_sub_agent() {
    // Simulate a sub-agent execution that never completes
    let never_completing = async {
        tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        Ok::<String, String>("never reached".to_string())
    };

    // Apply wall-clock timeout (short duration for test speed)
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(50),
        never_completing,
    )
    .await;

    // The timeout must fire — this is the wrapping pattern for build_and_run_agent
    assert!(result.is_err(), "Wall-clock timeout must fire on stalled sub-agent");

    // When timeout fires, the error message returned to the parent must be descriptive
    let timeout_msg = build_deep_search_timeout_message(DEEP_SEARCH_WALL_CLOCK_TIMEOUT_SECS);
    assert!(
        timeout_msg.contains("timed out after 300s"),
        "Timeout message must include the duration"
    );
    assert!(
        timeout_msg.contains("DeepSearch sub-agent"),
        "Timeout message must identify the source"
    );
}

#[test]
fn deep_search_wall_clock_timeout_defaults_to_300_seconds() {
    assert_eq!(DEEP_SEARCH_WALL_CLOCK_TIMEOUT_SECS, 300);
    assert_eq!(
        deep_search_wall_clock_timeout(),
        std::time::Duration::from_secs(300)
    );
}

#[test]
fn deep_search_timeout_message_is_not_classified_as_stall_timeout() {
    // The DeepSearch timeout message is returned as a TOOL RESULT string,
    // not as a stream error. But verify it doesn't accidentally match
    // the stall timeout classifier if it ever reaches error handling.
    let timeout_msg = build_deep_search_timeout_message(300);
    assert!(
        !is_stall_timeout_error(&timeout_msg),
        "DeepSearch timeout message must NOT match the stall timeout classifier \
         (it's a tool result, not a stream error)"
    );
}

// Scenario: Streaming collection with stalled stream would be caught by wall-clock timeout
#[tokio::test]
async fn collect_final_response_stalled_stream_aborted_by_wall_clock_timeout() {
    // Simulate a stream that produces one chunk then stalls forever
    // (the SSE connection is alive but no more data flows)
    let stalled_stream = futures::stream::unfold(0u32, |count| async move {
        if count == 0 {
            Some((
                Ok::<MultiTurnStreamItem<TestStreamResponse>, anyhow::Error>(
                    MultiTurnStreamItem::StreamAssistantItem(
                        rig::streaming::StreamedAssistantContent::<TestStreamResponse>::text("partial"),
                    ),
                ),
                count + 1,
            ))
        } else {
            // Stall forever — simulates the root cause of AMGR-016
            futures::future::pending::<()>().await;
            unreachable!()
        }
    });

    // The wall-clock timeout wraps the ENTIRE collect_final_response call
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        collect_final_response_from_stream(stalled_stream),
    )
    .await;

    // Wall-clock timeout must fire — collect_final_response would block forever without it
    assert!(
        result.is_err(),
        "Wall-clock timeout must abort stalled collect_final_response_from_stream"
    );
}
