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

/// CMPCT-002: Check if an error indicates compaction was cancelled by the hook
/// This is used to detect when the CompactionHook cancels a request due to token threshold
pub(super) fn is_compaction_cancelled(error: &anyhow::Error) -> bool {
    error.to_string().contains("PromptCancelled")
}
