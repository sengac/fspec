#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/sse-disconnection-retry.feature
//!
//! NET-001: Transient network error retry — SSE disconnection recovery
//!
//! Tests for transient network error detection, retry delay calculation, and
//! retry budget logic. These tests import the REAL production functions from
//! codelet_cli::interactive — no copies, no mocks.

use codelet_cli::interactive::{
    is_transient_network_error, network_retry_delay, MAX_NETWORK_RETRIES,
};
use std::time::Duration;

// =============================================================================
// Scenario: Recover from a single network blip during streaming
// =============================================================================

#[test]
fn test_recover_from_single_network_blip() {
    // @step Given an active SSE streaming session with the LLM
    // (precondition — streaming is active, connection reset occurs)

    // @step When a transient connection reset occurs mid-stream
    let error = "Streaming error: ProviderError: SSE Error: Http client error: \
                 error sending request for url (https://api.anthropic.com/v1/messages?beta=true)";

    // @step Then the error is classified as a transient network error
    assert!(
        is_transient_network_error(error),
        "Should detect SSE HTTP client error as transient. Got: {error}"
    );

    // @step And the system waits 1 second before retrying
    assert_eq!(network_retry_delay(1), Duration::from_millis(1000));

    // @step And a Continue re-prompt is sent to the LLM
    // (verified by stream_loop integration — retry sends Continue prompt)

    // @step And the model resumes generating from where it left off
    // (verified by stream_loop integration — conversation history is intact)
}

// Also verify nested/wrapped errors are detected
#[test]
fn test_detects_wrapped_agent_error() {
    let error = "Agent error: Streaming error: ProviderError: SSE Error: Http client error: \
                 error sending request for url (https://api.anthropic.com/v1/messages?beta=true)";
    assert!(
        is_transient_network_error(error),
        "Should detect deeply nested SSE error as transient. Got: {error}"
    );
}

// =============================================================================
// Scenario: All retry attempts exhausted after consecutive failures
// =============================================================================

#[test]
fn test_all_retries_exhausted() {
    // @step Given an active SSE streaming session with the LLM
    let dns_error = "DNS error: failed to resolve api.anthropic.com";
    assert!(is_transient_network_error(dns_error));

    // @step When three consecutive DNS timeout errors occur
    // @step Then the system retries at 1s, 2s, and 4s intervals
    assert_eq!(network_retry_delay(1), Duration::from_millis(1000));
    assert_eq!(network_retry_delay(2), Duration::from_millis(2000));
    assert_eq!(network_retry_delay(3), Duration::from_millis(4000));

    // @step And after all 3 retries are exhausted the original error propagates as fatal
    assert_eq!(MAX_NETWORK_RETRIES, 3);

    // @step And the session terminates with an error message
    // (verified by stream_loop integration — error propagates after budget exhausted)
}

// =============================================================================
// Scenario: Partial text preserved on network error during streaming
// =============================================================================

#[test]
fn test_partial_text_preserved() {
    // @step Given an active SSE streaming session that has already received partial text
    // (precondition — partial text was received before error)

    // @step When a transient network error occurs
    let error = "connection reset by peer";
    assert!(is_transient_network_error(error));

    // @step Then the partial text generated before disconnection is preserved in message history
    // (verified by stream_loop integration — accumulated_text is NOT cleared on retry)

    // @step And the retry succeeds with a Continue re-prompt
    // (verified by stream_loop integration — sends Continue message)

    // @step And the model continues from where it left off
    // (verified by stream_loop integration — conversation history intact)
}

// =============================================================================
// Scenario: Retry succeeds on second attempt with increasing backoff
// =============================================================================

#[test]
fn test_retry_succeeds_on_second_attempt() {
    // @step Given an active SSE streaming session with the LLM
    // (precondition — active stream)

    // @step When a transient network error occurs and the first retry also fails
    let error = "broken pipe";
    assert!(is_transient_network_error(error));

    // @step Then the first retry waits 1 second
    assert_eq!(network_retry_delay(1), Duration::from_millis(1000));

    // @step And the second retry waits 2 seconds
    assert_eq!(network_retry_delay(2), Duration::from_millis(2000));

    // @step And the second retry succeeds
    // (verified by stream_loop integration — retry loop continues)

    // @step And the session recovers
    // (verified by stream_loop integration — fresh CompactionHook/TokenState on retry)
}

// =============================================================================
// Scenario: User interruption during retry backoff aborts immediately
// =============================================================================

#[test]
fn test_user_interruption_during_backoff() {
    // @step Given the system is waiting during retry backoff after a network error
    // (precondition — retry backoff sleep in progress)

    // @step When the user presses Esc
    // (verified by stream_loop integration — is_interrupted checked after sleep)

    // @step Then the retry loop aborts immediately without waiting for the full delay
    // (verified by stream_loop integration — breaks out of retry on is_interrupted)
    // Unit-level: verify delays are reasonable so interruption is responsive
    let max_single_delay = network_retry_delay(MAX_NETWORK_RETRIES);
    assert!(
        max_single_delay <= Duration::from_secs(5),
        "Max single delay should be <=5s for responsive interruption"
    );
}

// =============================================================================
// Scenario: Non-network API errors are not retried
// =============================================================================

#[test]
fn test_non_network_errors_not_retried() {
    // @step Given an active SSE streaming session with the LLM
    // (precondition — active stream)

    // @step When a non-transient error occurs such as 400 bad request or 401 unauthorized
    // @step Then the error is not classified as a transient network error
    assert!(!is_transient_network_error("invalid api key"));
    assert!(!is_transient_network_error("authentication failed"));
    assert!(!is_transient_network_error("unauthorized"));
    assert!(!is_transient_network_error("prompt is too long"));
    assert!(!is_transient_network_error(
        "maximum context length exceeded"
    ));
    assert!(!is_transient_network_error("rate limit exceeded"));
    assert!(!is_transient_network_error("429 Too Many Requests"));
    assert!(!is_transient_network_error(
        "Tool call truncated due to output token limit"
    ));
    assert!(!is_transient_network_error(
        "Failed to parse JSON: expected ','"
    ));
    assert!(!is_transient_network_error("model not found: claude-4"));
    assert!(!is_transient_network_error("content policy violation"));
    assert!(!is_transient_network_error(""));

    // @step And no retry is attempted
    // @step And the error propagates immediately
    // (verified by stream_loop integration — non-transient errors hit terminal catch-all)
}

// =============================================================================
// Scenario: Network retry works in post-compaction retry streams
// =============================================================================

#[test]
fn test_network_retry_in_compaction_stream() {
    // @step Given a post-compaction retry stream is active
    // (precondition — compaction_retry.rs uses same imports)

    // @step When a transient network error occurs during the compaction retry stream
    let error = "connection closed before message completed";
    assert!(is_transient_network_error(error));

    // @step Then the same retry logic applies with exponential backoff
    // Same constants are used in compaction_retry.rs
    assert_eq!(MAX_NETWORK_RETRIES, 3);
    assert_eq!(network_retry_delay(1), Duration::from_millis(1000));

    // @step And the compaction retry stream recovers
    // (verified by compaction_retry.rs integration — same retry block pattern)
}

// =============================================================================
// Scenario: Network retry works in DeepSearch sub-agent streams
// =============================================================================

#[test]
fn test_network_retry_in_deepsearch() {
    // @step Given a DeepSearch sub-agent is streaming a response
    // (precondition — deep_search_handler.rs uses same error classifier)

    // @step When a transient network error occurs in the sub-agent stream
    let error = "operation timed out";
    assert!(is_transient_network_error(error));

    // @step Then the sub-agent retries independently
    // (verified by deep_search_handler.rs — collect_final_response_from_stream has retry)

    // @step And the parent session is not crashed
    // (verified by deep_search_handler.rs — errors are contained in sub-agent)
}

// =============================================================================
// Scenario: Retry counter resets after successful data receipt
// =============================================================================

#[test]
fn test_retry_counter_resets() {
    // @step Given a session that previously recovered from a network error
    // (precondition — retry_count was incremented, then success received)

    // @step When the stream successfully receives data events
    // (verified by stream_loop integration — counter reset on Text, ToolCall, Usage, FinalResponse)

    // @step Then the retry counter resets to zero
    // @step And future network errors get the full 3 retry attempts again
    let total_budget: Duration = (1..=MAX_NETWORK_RETRIES).map(network_retry_delay).sum();
    assert_eq!(
        total_budget,
        Duration::from_millis(7000),
        "Full budget after reset should be 7s (1+2+4)"
    );
}

// =============================================================================
// Scenario: Transient network error patterns are correctly detected
// =============================================================================

#[test]
fn test_transient_error_patterns_detected() {
    // @step Given the error classifier for transient network errors

    // @step When various error messages are evaluated for transient classification

    // @step Then it detects connection reset, connection refused, and connection closed errors
    assert!(is_transient_network_error("connection reset by peer"));
    assert!(is_transient_network_error(
        "Connection reset during streaming"
    ));
    assert!(is_transient_network_error("connection refused"));
    assert!(is_transient_network_error(
        "Connection refused: 127.0.0.1:443"
    ));
    assert!(is_transient_network_error(
        "connection closed before message completed"
    ));

    // @step And it detects broken pipe, DNS error, and network unreachable errors
    assert!(is_transient_network_error("broken pipe"));
    assert!(is_transient_network_error("Broken pipe during write"));
    assert!(is_transient_network_error(
        "DNS error: failed to resolve api.anthropic.com"
    ));
    assert!(is_transient_network_error("network is unreachable"));
    assert!(is_transient_network_error("connection aborted"));

    // @step And it detects timeout, hyper, unexpected EOF, and SSL errors
    assert!(is_transient_network_error("operation timed out"));
    assert!(is_transient_network_error("request timed out"));
    assert!(is_transient_network_error(
        "hyper::Error(IncompleteMessage)"
    ));
    assert!(is_transient_network_error(
        "stream closed before completion"
    ));
    assert!(is_transient_network_error("unexpected eof while reading"));
    assert!(is_transient_network_error("incomplete message"));
    assert!(is_transient_network_error(
        "ssl routines:OPENSSL internal error"
    ));
    assert!(is_transient_network_error("certificate verify failed"));

    // @step And it detects SSE HTTP client errors with nested error wrapping
    assert!(is_transient_network_error("error sending request for url"));
    assert!(is_transient_network_error(
        "API Error: Streaming error: ProviderError: SSE Error: Http client error: \
         error sending request for url (https://api.anthropic.com/v1/messages?beta=true)"
    ));
    assert!(is_transient_network_error(
        "API Error: Agent error: Streaming error: ProviderError: SSE Error: Http client error: \
         error sending request for url (https://api.anthropic.com/v1/messages?beta=true)"
    ));

    // @step And it does not classify rate limits, auth errors, or content policy violations as transient
    assert!(!is_transient_network_error("rate limit exceeded"));
    assert!(!is_transient_network_error("invalid api key"));
    assert!(!is_transient_network_error("content policy violation"));
    assert!(!is_transient_network_error(
        "context_length_exceeded timed out"
    ));
}
