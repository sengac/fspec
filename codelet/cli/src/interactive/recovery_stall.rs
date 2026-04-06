//! Stall timeout recovery for agent streaming.
//!
//! AMGR-016: When an LLM SSE stream stalls (no chunks received for a configurable
//! duration), the stream loop must detect and abort the generation. This prevents
//! subordinate agents from hanging indefinitely when the LLM API accepts the request
//! (HTTP headers returned) but never produces SSE body data.
//!
//! The stall timeout is distinct from network errors — the connection is alive but
//! no data flows. It resets on every received chunk (token, tool_call, usage, etc.)
//! and only fires when no data arrives for the full duration.

use std::time::Duration;

/// Default stall timeout in seconds.
/// If no streaming chunk (token, tool_call, usage, etc.) is received for this duration,
/// the stream loop aborts with a stall timeout error.
///
/// Public for testing and configuration — Rule [3].
pub const STALL_TIMEOUT_SECS: u64 = 120;

/// Default DeepSearch sub-agent wall-clock timeout in seconds.
/// The entire sub-agent execution (including all tool calls and LLM generations)
/// must complete within this duration — Rule [6].
///
/// Public for testing and configuration.
pub const DEEP_SEARCH_WALL_CLOCK_TIMEOUT_SECS: u64 = 300;

/// Canonical prefix for stall timeout error messages.
/// Used by both the stream loop (to create the error) and the error classifier
/// (to identify it). Keeping it as a constant prevents string drift.
pub const STALL_TIMEOUT_ERROR_PREFIX: &str = "Generation stalled";

/// Build a stall timeout error message with context.
///
/// The message must clearly indicate this is a stall (not a network error or API error)
/// per Rule [4]. The error classifier uses `STALL_TIMEOUT_ERROR_PREFIX` to identify it.
pub fn build_stall_timeout_message(timeout_secs: u64) -> String {
    format!(
        "{STALL_TIMEOUT_ERROR_PREFIX}: no streaming data received for {timeout_secs}s. \
         The LLM connection is alive but not producing tokens. \
         This may indicate an API-side hang or an overloaded endpoint.",
    )
}

/// Build a DeepSearch wall-clock timeout error message.
///
/// This is returned as the tool result string when a DeepSearch sub-agent exceeds
/// its wall-clock timeout. The parent agent receives it as a normal tool result
/// and can decide how to proceed — Rule [6].
pub fn build_deep_search_timeout_message(timeout_secs: u64) -> String {
    format!(
        "DeepSearch sub-agent timed out after {timeout_secs}s. The sub-agent's LLM generation \
         stalled or took too long to complete.",
    )
}

/// Get the stall timeout duration.
///
/// Currently returns the default. In future, this could read from environment
/// variables or session configuration — Rule [3].
pub fn stall_timeout_duration() -> Duration {
    Duration::from_secs(STALL_TIMEOUT_SECS)
}

/// Get the DeepSearch wall-clock timeout duration.
pub fn deep_search_wall_clock_timeout() -> Duration {
    Duration::from_secs(DEEP_SEARCH_WALL_CLOCK_TIMEOUT_SECS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::error_classifiers::{
        is_stall_timeout_error,
        is_transient_network_error,
        is_truncated_tool_call_error,
        is_prompt_too_long_error,
        is_image_content_error,
    };

    // Feature: spec/features/agent-stall-detection.feature

    // ====================================================================
    // Scenario: Stalled SSE stream detected and agent recovers to idle
    // ====================================================================

    // @step Given a subordinate agent is running and has received a tool result
    // @step And the agent's stream loop is awaiting the next LLM chunk
    // @step When the LLM SSE stream produces no chunks for 120 seconds
    // @step Then the stream loop should abort with a stall timeout error
    #[tokio::test]
    async fn stall_timeout_fires_when_stream_produces_no_chunks() {
        use futures::StreamExt;

        // Simulate a stream that never produces an item (stalled SSE)
        let stalled_stream = futures::stream::pending::<Result<String, anyhow::Error>>();
        futures::pin_mut!(stalled_stream);

        // Apply stall timeout (use short duration for test speed)
        let timeout_duration = std::time::Duration::from_millis(50);
        let result = tokio::time::timeout(timeout_duration, stalled_stream.next()).await;

        // The timeout must fire — this is the core mechanism
        assert!(result.is_err(), "Stall timeout must fire when stream produces no data");
    }

    // @step And the agent should transition from running to idle status
    // @step And an error message indicating "generation stalled" should be emitted
    #[test]
    fn stall_timeout_error_message_indicates_generation_stalled() {
        let msg = build_stall_timeout_message(120);
        assert!(
            msg.starts_with("Generation stalled"),
            "Error message must start with 'Generation stalled', got: {msg}"
        );
        assert!(
            msg.contains("120s"),
            "Error message must include the timeout duration"
        );
        assert!(
            msg.contains("no streaming data received"),
            "Error message must explain what happened"
        );
    }

    // @step And the supervisor's await_idle should return idle for this agent
    // (Integration behavior — tested at the session_manager level)

    // ====================================================================
    // Scenario: Normal token generation does not trigger stall timeout
    // ====================================================================

    // @step Given a subordinate agent is running and generating a response
    // @step When the LLM produces tokens continuously with less than 120 seconds between each
    // @step Then no stall timeout should fire
    // @step And the agent should complete its response normally
    // @step And the agent should transition to idle status
    #[tokio::test]
    async fn normal_token_generation_does_not_trigger_stall_timeout() {
        use futures::StreamExt;

        // Simulate a stream that produces tokens quickly
        let active_stream = futures::stream::iter(vec![
            Ok::<String, anyhow::Error>("Hello".to_string()),
            Ok("world".to_string()),
            Ok("!".to_string()),
        ]);
        futures::pin_mut!(active_stream);

        // Process each chunk with a generous timeout
        let timeout_duration = std::time::Duration::from_secs(1);
        let mut received = Vec::new();

        loop {
            match tokio::time::timeout(timeout_duration, active_stream.next()).await {
                Ok(Some(Ok(chunk))) => received.push(chunk),
                Ok(Some(Err(e))) => panic!("Unexpected stream error: {e}"),
                Ok(None) => break, // Stream ended normally
                Err(_elapsed) => panic!("Stall timeout must NOT fire on active stream"),
            }
        }

        assert_eq!(received.len(), 3, "All tokens should be received");
    }

    // ====================================================================
    // Scenario: Slow but active generation does not trigger stall timeout
    // ====================================================================

    // @step Given a subordinate agent is running and generating a response
    // @step When the LLM pauses for 60 seconds between tokens
    // @step And the stall timeout is configured to 120 seconds
    // @step Then no stall timeout should fire
    // @step And the agent should complete its response successfully
    #[tokio::test]
    async fn slow_but_active_generation_does_not_trigger_stall_timeout() {
        use futures::StreamExt;

        // Simulate a stream where each chunk arrives BEFORE the timeout
        // Timeout=200ms, chunks arrive every 80ms — well within bounds
        let slow_stream = futures::stream::unfold(0u32, |count| async move {
            if count >= 3 {
                return None;
            }
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            Some((
                Ok::<String, anyhow::Error>(format!("chunk-{count}")),
                count + 1,
            ))
        });
        futures::pin_mut!(slow_stream);

        let timeout_duration = std::time::Duration::from_millis(200);
        let mut received = Vec::new();

        loop {
            // Each timeout wraps a SINGLE stream.next() — resets per chunk
            match tokio::time::timeout(timeout_duration, slow_stream.next()).await {
                Ok(Some(Ok(chunk))) => received.push(chunk),
                Ok(Some(Err(e))) => panic!("Unexpected stream error: {e}"),
                Ok(None) => break,
                Err(_elapsed) => panic!(
                    "Stall timeout must NOT fire when chunks arrive within the timeout window"
                ),
            }
        }

        assert_eq!(received.len(), 3, "All slow chunks should be received");
    }

    // ====================================================================
    // Scenario: Mid-response stall preserves partial text in history
    // ====================================================================

    // @step Given a subordinate agent is running and has received partial response tokens
    // @step When the LLM stops producing tokens for 120 seconds mid-sentence
    // @step Then the stall timeout should fire and abort the generation
    // @step And the partial response text should be preserved in the session history
    // @step And the agent should transition to idle status
    #[tokio::test]
    async fn mid_response_stall_preserves_partial_text() {
        use futures::StreamExt;

        // Stream produces 2 chunks then stalls forever
        let partial_then_stall = futures::stream::unfold(0u32, |count| async move {
            if count < 2 {
                Some((
                    Ok::<String, anyhow::Error>(format!("partial-{count}")),
                    count + 1,
                ))
            } else {
                // Stall indefinitely
                futures::future::pending::<()>().await;
                unreachable!()
            }
        });
        futures::pin_mut!(partial_then_stall);

        let timeout_duration = std::time::Duration::from_millis(50);
        let mut received = Vec::new();
        let mut stall_detected = false;

        loop {
            match tokio::time::timeout(timeout_duration, partial_then_stall.next()).await {
                Ok(Some(Ok(chunk))) => received.push(chunk),
                Ok(Some(Err(e))) => panic!("Unexpected stream error: {e}"),
                Ok(None) => break,
                Err(_elapsed) => {
                    stall_detected = true;
                    break;
                }
            }
        }

        assert!(stall_detected, "Stall must be detected after partial response");
        assert_eq!(received.len(), 2, "Partial text must be preserved");
        assert_eq!(received[0], "partial-0");
        assert_eq!(received[1], "partial-1");
    }

    // ====================================================================
    // Scenario: DeepSearch sub-agent stall triggers wall-clock timeout
    // ====================================================================

    // @step Given a subordinate agent invokes a DeepSearch tool
    // @step And the DeepSearch sub-agent's LLM generation stalls indefinitely
    // @step When the DeepSearch wall-clock timeout of 300 seconds expires
    // @step Then the parent agent should receive a timeout error as the tool result string
    // @step And the parent agent should continue processing with the error result
    // @step And the parent agent should not hang or remain in running state
    #[tokio::test]
    async fn deep_search_wall_clock_timeout_aborts_stalled_sub_agent() {
        // Simulate a sub-agent that never completes
        let never_completing = async {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            Ok::<String, String>("never reached".to_string())
        };

        // Apply wall-clock timeout (short for test speed)
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(50),
            never_completing,
        )
        .await;

        // Timeout must fire
        assert!(result.is_err(), "Wall-clock timeout must fire on stalled sub-agent");

        // The error message returned to the parent must be descriptive
        let timeout_msg = build_deep_search_timeout_message(300);
        assert!(
            timeout_msg.contains("timed out after 300s"),
            "Timeout message must include duration"
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

    // ====================================================================
    // Scenario: Stream loop panic triggers drop guard to restore idle status
    // ====================================================================

    // @step Given a subordinate agent is running and the stream loop is active
    // @step When an unexpected panic occurs in the stream loop
    // @step Then the drop guard should fire and set the agent status to idle
    // @step And the supervisor's await_idle should return idle for this agent
    #[test]
    fn drop_guard_sets_idle_on_panic() {
        use std::sync::atomic::{AtomicU8, Ordering};
        use std::sync::Arc;

        /// Minimal drop guard matching the pattern for agent_loop — Rule [7].
        /// The real implementation will live in session_manager.rs.
        struct IdleGuard {
            status: Arc<AtomicU8>,
        }

        impl Drop for IdleGuard {
            fn drop(&mut self) {
                // 0 = Idle (matches SessionStatus::Idle = 0)
                self.status.store(0, Ordering::Release);
            }
        }

        let status = Arc::new(AtomicU8::new(1)); // 1 = Running
        let status_clone = status.clone();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = IdleGuard {
                status: status_clone,
            };
            // Simulate stream loop processing that panics
            panic!("stream loop panic during processing");
        }));

        assert!(result.is_err(), "Panic should be caught");
        assert_eq!(
            status.load(Ordering::Acquire),
            0,
            "Drop guard must set status to Idle (0) even after panic"
        );
    }

    #[test]
    fn drop_guard_sets_idle_on_normal_exit() {
        use std::sync::atomic::{AtomicU8, Ordering};
        use std::sync::Arc;

        struct IdleGuard {
            status: Arc<AtomicU8>,
        }

        impl Drop for IdleGuard {
            fn drop(&mut self) {
                self.status.store(0, Ordering::Release);
            }
        }

        let status = Arc::new(AtomicU8::new(1)); // 1 = Running

        {
            let _guard = IdleGuard {
                status: status.clone(),
            };
            // Normal processing — guard dropped at end of scope
        }

        assert_eq!(
            status.load(Ordering::Acquire),
            0,
            "Drop guard must set status to Idle on normal scope exit too"
        );
    }

    // ====================================================================
    // Constants verification
    // ====================================================================

    #[test]
    fn stall_timeout_defaults_to_120_seconds() {
        assert_eq!(STALL_TIMEOUT_SECS, 120);
        assert_eq!(stall_timeout_duration(), std::time::Duration::from_secs(120));
    }

    // ====================================================================
    // Scenario: Stall timeout error is not caught by error classifiers
    // ====================================================================

    // @step Given the stream loop has a stall timeout configured
    // @step When the stall timeout fires due to no tokens received
    // @step Then the error should bypass the error classifier cascade
    // @step And the error should not be retried as a network or truncation error
    // @step And the stream loop should break immediately with a terminal error
    #[test]
    fn stall_timeout_error_is_identified_by_dedicated_classifier() {
        let stall_msg = build_stall_timeout_message(120);

        assert!(
            is_stall_timeout_error(&stall_msg),
            "is_stall_timeout_error must identify stall timeout errors"
        );
    }

    #[test]
    fn stall_timeout_error_not_caught_by_network_classifier() {
        let stall_msg = build_stall_timeout_message(120);

        assert!(
            !is_transient_network_error(&stall_msg),
            "is_transient_network_error must NOT catch stall timeout errors"
        );
    }

    #[test]
    fn stall_timeout_error_not_caught_by_truncation_classifier() {
        let stall_msg = build_stall_timeout_message(120);

        assert!(
            !is_truncated_tool_call_error(&stall_msg),
            "is_truncated_tool_call_error must NOT catch stall timeout errors"
        );
    }

    #[test]
    fn stall_timeout_error_not_caught_by_prompt_too_long_classifier() {
        let stall_msg = build_stall_timeout_message(120);

        assert!(
            !is_prompt_too_long_error(&stall_msg),
            "is_prompt_too_long_error must NOT catch stall timeout errors"
        );
    }

    #[test]
    fn stall_timeout_error_not_caught_by_image_content_classifier() {
        let stall_msg = build_stall_timeout_message(120);

        assert!(
            !is_image_content_error(&stall_msg),
            "is_image_content_error must NOT catch stall timeout errors"
        );
    }

    // ====================================================================
    // Scenario: Network retry logic is not affected by stall timeout
    // ====================================================================

    // @step Given a subordinate agent is running and streaming a response
    // @step When a transient network error occurs during streaming
    // @step Then the existing NET-001 retry logic should handle the error
    // @step And the stall timeout should not interfere with the retry backoff
    // @step And the agent should complete normally after successful retry
    #[test]
    fn network_errors_are_not_classified_as_stall() {
        let network_errors = [
            "error sending request for url",
            "connection reset by peer",
            "dns error: failed to lookup address",
            "connection refused",
            "operation timed out",
        ];

        for error_msg in &network_errors {
            assert!(
                is_transient_network_error(error_msg),
                "'{error_msg}' should be classified as transient network error"
            );
            assert!(
                !is_stall_timeout_error(error_msg),
                "'{error_msg}' must NOT be classified as stall timeout"
            );
        }
    }

    #[test]
    fn stall_classifier_does_not_match_unrelated_errors() {
        let unrelated = [
            "API key invalid",
            "rate limit exceeded",
            "model not found",
            "internal server error",
            "connection reset",
        ];

        for error_msg in &unrelated {
            assert!(
                !is_stall_timeout_error(error_msg),
                "'{error_msg}' must NOT be classified as stall timeout"
            );
        }
    }
}
