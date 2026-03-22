# PROV-045: Stream Error Classification Enum — Implementation Guide

## Problem

Error classification in stream_loop.rs is done via 5 separate string-matching functions:
- `is_prompt_too_long_error()` — 6 string patterns
- `is_truncated_tool_call_error()` — 1 string pattern
- `is_thinking_exhaustion()` — checks stop_reason + token counts
- `is_image_content_error()` — 5 string patterns
- `is_compaction_cancelled()` — 1 string pattern

Each new error type adds another function. Provider-specific error messages break detection. The `Some(Err(e))` branch is a chain of if/else-if blocks that's growing unwieldy.

## VTCode Reference

### ErrorCategory enum (`vtcode-commons`, re-exported in `vtcode-core/src/error.rs` line 12)
```rust
pub use vtcode_commons::{BackoffStrategy, ErrorCategory, Retryability};
```

Categories include: Network, Timeout, Authentication, PolicyViolation, ResourceExhausted, InvalidInput, etc.

### ErrorCode enum (`vtcode-core/src/error.rs` lines 49–96)
```rust
pub enum ErrorCode {
    InvalidArgument, ValidationFailed, ParseError,
    CommandFailed, ToolExecutionFailed, Timeout,
    ConnectionFailed, RequestFailed, RateLimited, ServiceUnavailable,
    AuthenticationFailed, LLMProviderError, TokenLimitExceeded, ContextTooLong,
    ConfigInvalid, ConfigMissing, ConfigParseFailed,
    PermissionDenied, PolicyViolation, PlanModeViolation, SandboxViolation, DotfileProtection,
    IoError, OutOfMemory, ResourceUnavailable, ResourceNotFound,
    ToolNotFound, CircuitOpen, Cancelled, Unexpected, NotImplemented,
}
```

### Typed error downcasting (`vtcode-core/src/retry.rs` lines 111–141)
```rust
pub fn decision_for_anyhow(&self, error: &anyhow::Error, attempt_index: u32, tool_name: Option<&str>) -> RetryDecision {
    // Priority: typed VtCodeError → LLMError → UnifiedToolError → string fallback
    if let Some(vtcode_error) = error.downcast_ref::<VtCodeError>() {
        return self.decision_for_vtcode_error(vtcode_error, attempt_index, tool_name);
    }
    if let Some(llm_error) = error.downcast_ref::<LLMError>() {
        return self.decision_for_llm_error(llm_error, attempt_index);
    }
    if let Some(tool_error) = error.downcast_ref::<UnifiedToolError>() { ... }
    // Fallback: classify by error message text
    let category = vtcode_commons::classify_anyhow_error(error);
    self.decision_for_category_with_tool(category, attempt_index, None, tool_name)
}
```

### FinishReason parsing (`vtcode-core/src/llm/providers/anthropic/response_parser.rs` lines 150–154)
```rust
pub fn parse_finish_reason(stop_reason: &str) -> FinishReason {
    match stop_reason {
        "end_turn" | "stop" => FinishReason::Stop,
        "max_tokens" => FinishReason::Length,
        "tool_use" => FinishReason::ToolCalls,
        "refusal" => FinishReason::Refusal,
        _ => FinishReason::Stop,
    }
}
```

## Proposed Implementation for fspec

### 1. Define StreamErrorKind enum

```rust
// codelet/cli/src/interactive/stream_errors.rs (new file)
use std::time::Duration;

/// Classified stream error for routing to the appropriate recovery handler.
/// Replaces the chain of is_*() string-matching functions.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamErrorKind {
    /// Context exceeds model maximum — trigger emergency compaction
    PromptTooLong,

    /// Tool call JSON truncated by output token limit (PROV-039/040)
    TruncatedToolCall {
        tool_name: String,
        partial_args: String,
    },

    /// Thinking consumed entire budget, near-empty output (PROV-041)
    /// Note: This is detected from FinalResponse, not from the error branch
    ThinkingExhaustion {
        reasoning_tokens: u64,
        output_tokens: u64,
    },

    /// Image dimensions/size rejected by provider (EXT-016)
    ImageContent,

    /// CompactionHook cancelled the request (CMPCT-002)
    CompactionCancelled,

    /// Rate limited — may include Retry-After duration (PROV-049)
    RateLimit {
        retry_after: Option<Duration>,
    },

    /// Transient network error — retryable
    NetworkTransient,

    /// Authentication/authorization failure — NOT retryable
    AuthFailure,

    /// Unrecognized error — report to user
    Unknown,
}

impl StreamErrorKind {
    /// Whether this error type is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::PromptTooLong
                | Self::TruncatedToolCall { .. }
                | Self::ImageContent
                | Self::CompactionCancelled
                | Self::RateLimit { .. }
                | Self::NetworkTransient
        )
    }
}

/// Classify an anyhow::Error into a StreamErrorKind.
/// Centralizes all string-matching detection in one place.
pub fn classify_stream_error(error: &anyhow::Error) -> StreamErrorKind {
    let error_str = error.to_string();
    let error_lower = error_str.to_lowercase();

    // Check compaction first (most specific)
    if error_str.contains("PromptCancelled") {
        return StreamErrorKind::CompactionCancelled;
    }

    // Check truncated tool call (PROV-039/040)
    if error_str.contains("Tool call truncated due to output token limit") {
        let tool_name = extract_tool_name(&error_str);
        let partial_args = extract_partial_args(&error_str);
        return StreamErrorKind::TruncatedToolCall { tool_name, partial_args };
    }

    // Check prompt too long (exclude budget_tokens errors per PROV-010)
    if !error_lower.contains("budget_tokens") && is_context_length_error(&error_lower) {
        return StreamErrorKind::PromptTooLong;
    }

    // Check image content error (EXT-016)
    if error_lower.contains("image")
        && (error_lower.contains("dimension")
            || error_lower.contains("exceed")
            || error_lower.contains("too large")
            || error_lower.contains("size"))
    {
        return StreamErrorKind::ImageContent;
    }

    // Check rate limit
    if error_lower.contains("rate_limit") || error_lower.contains("429") {
        let retry_after = parse_retry_after(&error_str);
        return StreamErrorKind::RateLimit { retry_after };
    }

    // Check auth failures
    if error_lower.contains("authentication")
        || error_lower.contains("unauthorized")
        || error_lower.contains("401")
        || error_lower.contains("invalid_api_key")
    {
        return StreamErrorKind::AuthFailure;
    }

    // Check transient network errors
    if error_lower.contains("connection")
        || error_lower.contains("timeout")
        || error_lower.contains("503")
        || error_lower.contains("502")
    {
        return StreamErrorKind::NetworkTransient;
    }

    StreamErrorKind::Unknown
}

fn is_context_length_error(error_lower: &str) -> bool {
    error_lower.contains("prompt is too long")
        || error_lower.contains("maximum context length")
        || error_lower.contains("context_length_exceeded")
        || error_lower.contains("too many tokens")
        || error_lower.contains("exceeds the model")
        || (error_lower.contains("invalid_request_error")
            && (error_lower.contains("token") || error_lower.contains("maximum")))
}

fn extract_tool_name(error_str: &str) -> String {
    error_str.split("Tool '").nth(1)
        .and_then(|s| s.split('\'').next())
        .unwrap_or("unknown")
        .to_string()
}

fn extract_partial_args(error_str: &str) -> String {
    error_str.split("Partial arguments: ").nth(1)
        .unwrap_or("(not available)")
        .to_string()
}

fn parse_retry_after(error_str: &str) -> Option<Duration> {
    // Try to find "retry-after: N" or "Retry-After: N" in error
    // This will be enhanced in PROV-049
    None
}
```

### 2. Refactor stream_loop.rs error branch

**Before (current — chain of if/else):**
```rust
Some(Err(e)) => {
    let is_compaction_cancel = is_compaction_cancelled(&e);
    let compaction_triggered = token_state.lock()...;
    if is_compaction_cancel && compaction_triggered { ... break; }
    let error_str = e.to_string();
    let is_prompt_too_long = is_prompt_too_long_error(&error_str);
    if is_prompt_too_long && has_compactable_turns { ... }
    if is_image_content_error(&error_str) { ... }
    if is_truncated_tool_call_error(&error_str) { ... }
    // Fall through to generic error
}
```

**After (with classification):**
```rust
Some(Err(e)) => {
    let error_kind = classify_stream_error(&e);
    session.api_failure_tracker.record_failure(); // PROV-044

    match error_kind {
        StreamErrorKind::CompactionCancelled if compaction_triggered => {
            debug!("Breaking due to compaction cancellation");
            break;
        }
        StreamErrorKind::PromptTooLong if has_compactable_turns => {
            // Emergency compaction path
        }
        StreamErrorKind::ImageContent => {
            // Image sanitization path
        }
        StreamErrorKind::TruncatedToolCall { tool_name, partial_args } => {
            // Truncation recovery path (PROV-040)
        }
        StreamErrorKind::RateLimit { retry_after } => {
            // Rate limit handling (PROV-049)
        }
        StreamErrorKind::NetworkTransient => {
            // Could retry with backoff (PROV-043)
        }
        StreamErrorKind::AuthFailure => {
            // Non-retryable — report to user immediately
        }
        _ => {
            // Generic error — report to user
        }
    }
}
```

### 3. Keep existing functions as deprecated aliases (migration path)

```rust
#[deprecated(note = "Use classify_stream_error() instead")]
pub fn is_prompt_too_long_error(error_str: &str) -> bool { ... }
```

This allows tests to migrate gradually.

## Estimated Effort: 5 story points
