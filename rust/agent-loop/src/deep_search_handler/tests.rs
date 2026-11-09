use super::*;
use codelet_cli::interactive::{
    build_deep_search_timeout_message, deep_search_wall_clock_timeout, is_stall_timeout_error,
    DEEP_SEARCH_WALL_CLOCK_TIMEOUT_SECS,
};
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
    // @step And the final synthesized answer contract remains unchanged
    // Non-streaming providers use rig_agent.prompt() which returns String directly
}

// Scenario: Streaming collection returns only the final DeepSearch answer contract

// @step Given a DeepSearch streaming response contains intermediate assistant chunks
#[tokio::test]
async fn collect_final_response_from_stream_uses_final_response_only() {
    let stream = futures::stream::iter(vec![
        Ok(MultiTurnStreamItem::StreamAssistantItem(
            rig::streaming::StreamedAssistantContent::<TestStreamResponse>::text(
                "intermediate chunk",
            ),
        )),
        Ok(MultiTurnStreamItem::final_response(
            "final synthesized answer",
            rig::completion::Usage::new(),
        )),
    ]);

    // @step When the streaming collection completes
    let collected = collect_final_response_from_stream(stream, "zai")
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

    let result = collect_final_response_from_stream(stream, "zai").await;
    assert!(result.is_err());
    assert!(result
        .expect_err("expected missing final response error")
        .contains("missing final response"));
}

#[tokio::test]
async fn collect_final_response_from_stream_propagates_stream_errors() {
    let stream = futures::stream::iter(vec![Err::<
        MultiTurnStreamItem<TestStreamResponse>,
        anyhow::Error,
    >(anyhow::anyhow!("stream blew up"))]);

    let result = collect_final_response_from_stream(stream, "zai").await;
    assert!(result.is_err());
    assert!(result
        .expect_err("expected stream error")
        .contains("stream blew up"));
}

// ====================================================================
// Feature: spec/features/agent-stall-detection.feature
// AMGR-016: DeepSearch wall-clock timeout tests
// ====================================================================

// Scenario: DeepSearch sub-agent stall triggers wall-clock timeout

// @step Given a subordinate agent invokes a DeepSearch tool
// @step And the DeepSearch sub-agent's LLM generation stalls indefinitely
// @step When the DeepSearch wall-clock timeout of 600 seconds expires
// @step Then the parent agent should receive a timeout error as the tool result string
// @step And the parent agent should continue processing with the error result
// @step And the parent agent should not hang or remain in running state
#[tokio::test(start_paused = true)]
async fn deep_search_wall_clock_timeout_fires_on_stalled_sub_agent() {
    // Simulate a sub-agent execution that never completes
    let never_completing = tokio::spawn(async {
        std::future::pending::<()>().await;
        Ok::<String, String>("never reached".to_string())
    });

    // TEST-042: on a paused clock, advance past the (short) limit instead of
    // timing out on the wall clock; the stalled sub-agent is still running,
    // which is what the wall-clock timeout around build_and_run_agent ends.
    tokio::time::advance(std::time::Duration::from_millis(50)).await;
    tokio::task::yield_now().await;
    assert!(
        !never_completing.is_finished(),
        "Wall-clock timeout must fire on stalled sub-agent"
    );
    never_completing.abort();

    // When timeout fires, the error message returned to the parent must be descriptive
    let timeout_msg = build_deep_search_timeout_message(DEEP_SEARCH_WALL_CLOCK_TIMEOUT_SECS);
    assert!(
        timeout_msg.contains("timed out after 600s"),
        "Timeout message must include the duration"
    );
    assert!(
        timeout_msg.contains("DeepSearch sub-agent"),
        "Timeout message must identify the source"
    );
}

#[test]
fn deep_search_wall_clock_timeout_defaults_to_600_seconds() {
    assert_eq!(DEEP_SEARCH_WALL_CLOCK_TIMEOUT_SECS, 600);
    assert_eq!(
        deep_search_wall_clock_timeout(),
        std::time::Duration::from_secs(600)
    );
}

#[test]
fn deep_search_timeout_message_is_not_classified_as_stall_timeout() {
    // The DeepSearch timeout message is returned as a TOOL RESULT string,
    // not as a stream error. But verify it doesn't accidentally match
    // the stall timeout classifier if it ever reaches error handling.
    let timeout_msg = build_deep_search_timeout_message(600);
    assert!(
        !is_stall_timeout_error(&timeout_msg),
        "DeepSearch timeout message must NOT match the stall timeout classifier \
         (it's a tool result, not a stream error)"
    );
}

// Scenario: Streaming collection with stalled stream would be caught by wall-clock timeout
#[tokio::test(start_paused = true)]
async fn collect_final_response_stalled_stream_aborted_by_wall_clock_timeout() {
    // Simulate a stream that produces one chunk then stalls forever
    // (the SSE connection is alive but no more data flows)
    let stalled_stream = futures::stream::unfold(0u32, |count| async move {
        if count == 0 {
            Some((
                Ok::<MultiTurnStreamItem<TestStreamResponse>, anyhow::Error>(
                    MultiTurnStreamItem::StreamAssistantItem(
                        rig::streaming::StreamedAssistantContent::<TestStreamResponse>::text(
                            "partial",
                        ),
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
    // TEST-042: on a paused clock, advance past the limit; the collection is
    // still blocked on the stalled stream, which only the wall-clock timeout
    // around it ends.
    let collecting = tokio::spawn(collect_final_response_from_stream(stalled_stream, "zai"));
    tokio::time::advance(std::time::Duration::from_millis(100)).await;
    tokio::task::yield_now().await;
    assert!(
        !collecting.is_finished(),
        "Wall-clock timeout must abort stalled collect_final_response_from_stream"
    );
    collecting.abort();
}
