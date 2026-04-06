//! Error classification functions for the streaming loop.
//!
//! Pure functions that classify error strings into categories. No side effects, no state.
//! Used by the stream loop to determine which recovery strategy to apply.

/// Check if an error indicates the prompt/context is too long
/// PROV-010: Exclude thinking budget configuration errors (budget_tokens)
///
/// This function is public for testing. Tests MUST import and test the
/// real function, NOT a copy. See: codelet/cli/tests/prompt_too_long_recovery_test.rs
pub fn is_prompt_too_long_error(error_str: &str) -> bool {
    let error_lower = error_str.to_lowercase();
    
    // PROV-010: Exclude thinking budget configuration errors
    // These contain "budget_tokens" and should NOT trigger compaction
    if error_lower.contains("budget_tokens") {
        return false;
    }
    
    error_lower.contains("prompt is too long")
        || error_lower.contains("maximum context length")
        || error_lower.contains("context_length_exceeded")
        || error_lower.contains("too many tokens")
        || error_lower.contains("exceeds the model")
        || (error_lower.contains("invalid_request_error")
            && (error_lower.contains("token") || error_lower.contains("maximum")))
}

/// EXT-016: Check if an error indicates image content was rejected by the API.
///
/// Detects 400 errors related to image dimensions, image size, or image processing.
/// This is used to trigger image content sanitization in the error recovery path.
///
/// This function is public for testing.
pub fn is_image_content_error(error_str: &str) -> bool {
    let error_lower = error_str.to_lowercase();

    // Must mention "image" in conjunction with dimension/size-related terms
    if error_lower.contains("image") {
        return error_lower.contains("dimension")
            || error_lower.contains("exceed")
            || error_lower.contains("too large")
            || error_lower.contains("max allowed size")
            || error_lower.contains("size");
    }

    false
}

/// PROV-040: Check if an error indicates a truncated tool call due to output token limit.
///
/// Detects the enriched error message emitted by PROV-039 in the Anthropic streaming
/// handler when `stop_reason == "max_tokens"` and a pending tool call was never closed.
///
/// This function is public for testing. Tests MUST import and test the
/// real function, NOT a copy. See: codelet/cli/tests/truncation_recovery_test.rs
pub fn is_truncated_tool_call_error(error_str: &str) -> bool {
    error_str.contains("Tool call truncated due to output token limit")
}

/// Check if an error is a transient network/connection error that can be retried.
///
/// Detects HTTP transport failures, DNS errors, connection resets, and similar
/// transient issues that may resolve on retry. These errors originate from the
/// reqwest HTTP client or the SSE transport layer.
///
/// This function is public for testing. Tests MUST import and test the
/// real function, NOT a copy.
pub fn is_transient_network_error(error_str: &str) -> bool {
    let lower = error_str.to_lowercase();

    // reqwest-level transport errors
    lower.contains("error sending request")
        || lower.contains("connection reset")
        || lower.contains("connection refused")
        || lower.contains("connection closed")
        || lower.contains("broken pipe")
        || lower.contains("network is unreachable")
        || lower.contains("dns error")
        || lower.contains("connection aborted")
        || lower.contains("connection was not ready")
        // Timeout errors (network-level, not API-level)
        || (lower.contains("timed out") && !lower.contains("context_length"))
        || lower.contains("operation timed out")
        // hyper-level errors
        || lower.contains("hyper::error")
        || lower.contains("stream closed before completion")
        // SSE transport errors (wrapping the above)
        || (lower.contains("sse error") && lower.contains("http client error"))
        // reqwest TLS errors
        || lower.contains("ssl routines")
        || lower.contains("certificate")
        // Generic I/O errors during streaming
        || (lower.contains("sse error") && lower.contains("instance"))
        // EOF during streaming
        || lower.contains("unexpected eof")
        || lower.contains("incomplete message")
}

/// AMGR-016: Check if an error indicates a stall timeout (no streaming data received).
///
/// Stall timeouts are TERMINAL errors that must NOT be retried by any error classifier.
/// They bypass the entire error classifier cascade — Rule [5], Rule [6].
///
/// Uses the canonical prefix from `recovery_stall::STALL_TIMEOUT_ERROR_PREFIX` to ensure
/// the identifier string is always in sync between creation and detection.
///
/// This function is public for testing.
pub fn is_stall_timeout_error(error_str: &str) -> bool {
    error_str.contains(super::recovery_stall::STALL_TIMEOUT_ERROR_PREFIX)
}

/// CMPCT-002: Check if an error indicates compaction was cancelled by the hook
/// This is used to detect when the CompactionHook cancels a request due to token threshold
pub(super) fn is_compaction_cancelled(error: &anyhow::Error) -> bool {
    error.to_string().contains("PromptCancelled")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Feature: spec/features/agent-stall-detection.feature

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
        let stall_msg = "Generation stalled: no streaming data received for 120s. \
                         The LLM connection is alive but not producing tokens. \
                         This may indicate an API-side hang or an overloaded endpoint.";

        assert!(
            is_stall_timeout_error(stall_msg),
            "is_stall_timeout_error must identify stall timeout errors"
        );
    }

    #[test]
    fn stall_timeout_error_not_caught_by_network_classifier() {
        let stall_msg = "Generation stalled: no streaming data received for 120s. \
                         The LLM connection is alive but not producing tokens.";

        assert!(
            !is_transient_network_error(stall_msg),
            "is_transient_network_error must NOT catch stall timeout errors"
        );
    }

    #[test]
    fn stall_timeout_error_not_caught_by_truncation_classifier() {
        let stall_msg = "Generation stalled: no streaming data received for 120s.";

        assert!(
            !is_truncated_tool_call_error(stall_msg),
            "is_truncated_tool_call_error must NOT catch stall timeout errors"
        );
    }

    #[test]
    fn stall_timeout_error_not_caught_by_prompt_too_long_classifier() {
        let stall_msg = "Generation stalled: no streaming data received for 120s.";

        assert!(
            !is_prompt_too_long_error(stall_msg),
            "is_prompt_too_long_error must NOT catch stall timeout errors"
        );
    }

    #[test]
    fn stall_timeout_error_not_caught_by_image_content_classifier() {
        let stall_msg = "Generation stalled: no streaming data received for 120s.";

        assert!(
            !is_image_content_error(stall_msg),
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
