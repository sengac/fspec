# PROV-045: Refactoring Guide — Stream Error Classification Extraction

## This Card Is the Foundation

PROV-045 is the **first card to implement** because every other card depends on the clean error classification it introduces. It extracts ~250 lines from `stream_loop.rs` into a new `stream_errors.rs` module.

## What to Extract FROM `stream_loop.rs`

### Functions to Move (lines 78–417)

| Function | Current Line | Purpose |
|----------|-------------|---------|
| `is_prompt_too_long_error()` | 78–94 | String-matching for context length errors |
| `is_image_content_error()` | 102–115 | String-matching for image rejection |
| `sanitize_image_content()` | 125–218 | Walks messages, replaces images with placeholders |
| `is_truncated_tool_call_error()` | 235–237 | String-matching for PROV-040 truncation |
| `build_truncation_recovery_message()` | 245–274 | Recovery prompt for truncated tool calls |
| `build_truncation_budget_exhausted_message()` | 277–287 | Budget-exhausted user message |
| `is_thinking_exhaustion()` | 322–348 | Usage-based detection of thinking budget depletion |
| `build_thinking_exhaustion_recovery_message()` | 358–381 | Recovery prompt with preserved reasoning context |
| `build_thinking_budget_exhausted_message()` | 389–397 | Budget-exhausted user message |
| `downgrade_thinking_level()` | 403–410 | ThinkingLevel state transition |
| `is_compaction_cancelled()` | 415–417 | PromptCancelled detection |

### Constants to Move

| Constant | Current Line |
|----------|-------------|
| `MAX_TRUNCATION_RETRIES` | 226 |
| `MAX_THINKING_EXHAUSTION_RETRIES` | 294 |
| `THINKING_EXHAUSTION_OUTPUT_THRESHOLD` | 300 |
| `THINKING_EXHAUSTION_CROSS_TURN_THRESHOLD` | 306 |

## What to Create in `stream_errors.rs`

### The StreamErrorKind Enum

```rust
/// Classified stream error — replaces 5 separate `is_*()` string-matching functions.
/// Every stream error passes through `classify_stream_error()` exactly once.
#[derive(Debug, Clone)]
pub enum StreamErrorKind {
    /// API rejected the prompt as too long. Recovery: emergency compaction.
    PromptTooLong {
        raw_message: String,
    },

    /// Tool call was truncated by output token limit (PROV-040).
    /// Recovery: inject alternative-strategy prompt, retry up to MAX_TRUNCATION_RETRIES.
    TruncatedToolCall {
        raw_message: String,
    },

    /// Model spent all tokens on reasoning with near-empty output (PROV-041).
    /// Recovery: inject recovery prompt with preserved reasoning, reduce thinking budget.
    ThinkingExhaustion {
        reasoning_tokens: u64,
        output_tokens: u64,
        stop_reason: Option<String>,
    },

    /// API rejected image content (dimensions, size, format).
    /// Recovery: sanitize images from history.
    ImageContent {
        raw_message: String,
    },

    /// CompactionHook cancelled the request (expected, not a real error).
    CompactionCancelled,

    /// HTTP 429 or provider rate limit. Recovery: wait Retry-After then retry.
    /// (PROV-049 populates `retry_after_secs`)
    RateLimit {
        retry_after_secs: Option<f64>,
        raw_message: String,
    },

    /// Transient network error (DNS, TCP reset, TLS handshake).
    /// Recovery: retry with backoff.
    NetworkTransient {
        raw_message: String,
    },

    /// Authentication failure (401, invalid API key).
    /// Recovery: none — user must fix credentials.
    AuthFailure {
        raw_message: String,
    },

    /// Unclassified error. Recovery: emit to user and return Err.
    Unknown {
        raw_message: String,
    },
}
```

### The Classifier Function

```rust
/// Classifies an anyhow::Error into a StreamErrorKind.
///
/// Called exactly once per `Some(Err(e))` in the stream loop.
/// Priority order matches current if/else chain:
///   1. CompactionCancelled (checked by caller via is_compaction_cancelled)
///   2. PromptTooLong
///   3. ImageContent
///   4. TruncatedToolCall
///   5. ThinkingExhaustion (separate path — detected on FinalResponse, not on Err)
///   6. RateLimit (NEW — PROV-049)
///   7. NetworkTransient (NEW)
///   8. AuthFailure (NEW)
///   9. Unknown (fallback)
pub fn classify_stream_error(error: &anyhow::Error) -> StreamErrorKind {
    let error_str = error.to_string();

    if is_compaction_cancelled(error) {
        return StreamErrorKind::CompactionCancelled;
    }

    if is_prompt_too_long_error(&error_str) {
        return StreamErrorKind::PromptTooLong { raw_message: error_str };
    }

    if is_image_content_error(&error_str) {
        return StreamErrorKind::ImageContent { raw_message: error_str };
    }

    if is_truncated_tool_call_error(&error_str) {
        return StreamErrorKind::TruncatedToolCall { raw_message: error_str };
    }

    if is_rate_limit_error(&error_str) {
        let retry_after = parse_retry_after_from_error(&error_str); // PROV-049 adds this
        return StreamErrorKind::RateLimit { retry_after_secs: retry_after, raw_message: error_str };
    }

    if is_network_transient(&error_str) {
        return StreamErrorKind::NetworkTransient { raw_message: error_str };
    }

    if is_auth_failure(&error_str) {
        return StreamErrorKind::AuthFailure { raw_message: error_str };
    }

    StreamErrorKind::Unknown { raw_message: error_str }
}
```

### Methods on StreamErrorKind

```rust
impl StreamErrorKind {
    /// Whether this error type is potentially recoverable via retry.
    pub fn is_retryable(&self) -> bool {
        matches!(self,
            Self::PromptTooLong { .. } |
            Self::TruncatedToolCall { .. } |
            Self::ThinkingExhaustion { .. } |
            Self::ImageContent { .. } |
            Self::RateLimit { .. } |
            Self::NetworkTransient { .. }
        )
    }

    /// The raw error message string (for logging/display).
    pub fn raw_message(&self) -> &str { ... }
}
```

### Recovery Message Builders (moved here)

All `build_*_recovery_message()` and `build_*_budget_exhausted_message()` functions move into this module, potentially as methods on `StreamErrorKind` or as associated functions.

## How `stream_loop.rs` Changes

### Before (current `Some(Err(e))` arm — lines 1755–1962)

```rust
Some(Err(e)) => {
    let is_compaction_cancel = is_compaction_cancelled(&e);
    let compaction_triggered = token_state.lock()...;

    if is_compaction_cancel && compaction_triggered { break; }

    let error_str = e.to_string();
    let is_prompt_too_long = is_prompt_too_long_error(&error_str);
    // ... 15 more lines of setup ...
    if is_prompt_too_long && has_compactable_turns { ... }
    if is_image_content_error(&error_str) { ... }
    if is_truncated_tool_call_error(&error_str) { ... }
    // ... 200 lines of nested recovery logic ...
    output.emit_error(&error_str);
    return Err(anyhow!("Agent error: {e}"));
}
```

### After

```rust
Some(Err(e)) => {
    let kind = classify_stream_error(&e);

    // CompactionCancelled is expected — not a real error
    if matches!(&kind, StreamErrorKind::CompactionCancelled) && compaction_triggered {
        break;
    }

    match kind {
        StreamErrorKind::PromptTooLong { .. } if has_compactable_turns => {
            output.emit_compaction_started();
            pop_last_user_message(session);
            signal_compaction_needed(&token_state);
            break;
        }
        StreamErrorKind::ImageContent { raw_message } => {
            pop_last_user_message(session);
            if sanitize_image_content(&mut session.messages) {
                output.emit_error(&format!("{raw_message}\n\n[Images removed]"));
                break;
            }
            // fall through to unknown
            output.emit_error(&raw_message);
            return Err(anyhow!("Agent error: {}", raw_message));
        }
        StreamErrorKind::TruncatedToolCall { raw_message } => {
            return retry_orchestrator.handle_truncation(
                &raw_message, session, &agent, &mut stream, &mut ctx, output, threshold
            ).await;
        }
        StreamErrorKind::RateLimit { retry_after_secs, .. } => {
            // PROV-049: handled by retry orchestrator
            return retry_orchestrator.handle_rate_limit(
                retry_after_secs, session, &agent, &mut stream, &mut ctx, output, threshold
            ).await;
        }
        other => {
            output.emit_error(other.raw_message());
            return Err(anyhow!("Agent error: {e}"));
        }
    }
}
```

## SOLID Alignment

| Principle | How This Card Addresses It |
|-----------|---------------------------|
| **SRP** | Error classification is one responsibility, extracted into one module |
| **OCP** | Adding a new error type = adding a variant to the enum, not modifying the god function |
| **DIP** | `stream_loop.rs` depends on `StreamErrorKind` abstraction, not string-matching details |
| **DRY** | `error.to_string()` called once in `classify_stream_error()`, not 5 times |

## Migration Strategy

1. Create `stream_errors.rs` with the enum and classifier
2. Move all `is_*()` functions and constants into it (keep `pub` temporarily for tests)
3. Move all `build_*()` recovery message functions into it
4. In `stream_loop.rs`, replace the if/else chain with `match classify_stream_error(&e)`
5. Update `mod.rs` re-exports
6. Deprecate direct `is_*()` function calls (they become internal to the classifier)
7. Run all 19 tests + add new tests for the classifier

## New Tests

```rust
#[test]
fn classify_prompt_too_long() {
    let err = anyhow!("prompt is too long for this model");
    assert!(matches!(classify_stream_error(&err), StreamErrorKind::PromptTooLong { .. }));
}

#[test]
fn classify_rate_limit_with_retry_after() {
    let err = anyhow!("rate limit exceeded, retry-after: 30");
    match classify_stream_error(&err) {
        StreamErrorKind::RateLimit { retry_after_secs, .. } => {
            assert_eq!(retry_after_secs, Some(30.0));
        }
        other => panic!("Expected RateLimit, got {:?}", other),
    }
}

#[test]
fn classify_unknown_falls_through() {
    let err = anyhow!("something completely unexpected");
    assert!(matches!(classify_stream_error(&err), StreamErrorKind::Unknown { .. }));
}

#[test]
fn thinking_exhaustion_not_classified_from_error() {
    // ThinkingExhaustion is detected on FinalResponse, not on Err
    // Ensure the classifier doesn't accidentally match it
    let err = anyhow!("budget_tokens exceeded");
    assert!(matches!(classify_stream_error(&err), StreamErrorKind::Unknown { .. }));
}
```

## Estimated Impact

- **Lines removed from `stream_loop.rs`**: ~250 (functions + constants + comments)
- **Lines added to `stream_errors.rs`**: ~200 (enum + classifier + recovery messages + new error types)
- **Net complexity reduction in god function**: if/else chain → clean match statement
- **Prep for PROV-043**: The match statement structure directly feeds into `StreamOutcome::Error(kind)` pattern
