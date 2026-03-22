# PROV-049: Parse Retry-After for Rate Limit Recovery — Implementation Guide

## Problem

Rate limit errors (HTTP 429) terminate the stream and are reported to the user. Anthropic returns precise `Retry-After` values that could be used to transparently retry instead of failing.

## VTCode Reference

### parse_retry_after_header (`vtcode-core/src/retry.rs` lines 211–219)

```rust
fn parse_retry_after_header(metadata: &LLMErrorMetadata) -> Option<Duration> {
    let raw = metadata.retry_after.as_deref()?.trim();
    // Try integer seconds first
    if let Ok(seconds) = raw.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    // Try floating-point seconds
    if let Ok(seconds) = raw.parse::<f64>() {
        return Some(Duration::from_secs_f64(seconds.max(0.0)));
    }
    None
}
```

### LLMErrorMetadata (`vtcode-commons` crate)

```rust
pub struct LLMErrorMetadata {
    pub provider: String,
    pub status_code: Option<u16>,
    pub error_code: Option<String>,
    pub request_id: Option<String>,
    pub organization_id: Option<String>,
    pub retry_after: Option<String>,   // ← The raw Retry-After header value
    pub raw_message: Option<String>,
}
```

### Usage in retry decision (`vtcode-core/src/retry.rs` lines 143–151)

```rust
pub fn decision_for_llm_error(&self, error: &LLMError, attempt_index: u32) -> RetryDecision {
    let retry_after = llm_metadata(error).and_then(parse_retry_after_header);
    self.decision_for_category_with_tool(
        ErrorCategory::from(error),
        attempt_index,
        retry_after,   // ← Overrides computed backoff if present
        None,
    )
}
```

### Retry-After overrides backoff (`retry.rs` lines 86–95)

```rust
// If retry_after is provided by the server, use it instead of computed backoff
let delay = retry_after.unwrap_or_else(|| self.delay_for_attempt(attempt_index));
RetryDecision {
    category,
    retryable: true,
    delay: Some(delay),
    retry_after,
}
```

### Test (`retry.rs` lines 278–297)

```rust
#[test]
fn retry_after_header_overrides_backoff_delay() {
    let policy = RetryPolicy::from_retries(3, Duration::from_secs(1), Duration::from_secs(8), 2.0);
    let err = LLMError::RateLimit {
        metadata: Some(LLMErrorMetadata::new(
            "Anthropic", Some(429),
            Some("rate_limit_error".to_string()),
            None, None,
            Some("7".to_string()),   // Retry-After: 7 seconds
            Some("too many requests".to_string()),
        )),
    };

    let decision = policy.decision_for_llm_error(&err, 0);
    assert!(decision.retryable);
    assert_eq!(decision.retry_after, Some(Duration::from_secs(7)));
    assert_eq!(decision.delay, Some(Duration::from_secs(7)));  // Uses 7s, not computed 1s
}
```

## Proposed Implementation for fspec

### 1. Parse Retry-After from error message

Since fspec uses `anyhow::Error` without typed metadata, we need to extract Retry-After from the error string:

```rust
/// PROV-049: Extract Retry-After duration from rate limit error messages.
///
/// Anthropic format: "rate_limit_error ... retry after 7 seconds"
/// Some providers include: "Retry-After: 7"
pub fn parse_retry_after_from_error(error_str: &str) -> Option<Duration> {
    let error_lower = error_str.to_lowercase();

    // Pattern 1: "retry-after: N" or "retry after N"
    for pattern in ["retry-after: ", "retry after ", "retry_after: "] {
        if let Some(idx) = error_lower.find(pattern) {
            let after = &error_str[idx + pattern.len()..];
            let num_str: String = after.chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if let Ok(secs) = num_str.parse::<f64>() {
                return Some(Duration::from_secs_f64(secs.max(0.5)));
            }
        }
    }

    // Pattern 2: "wait N seconds" or "try again in N seconds"
    for pattern in ["wait ", "try again in "] {
        if let Some(idx) = error_lower.find(pattern) {
            let after = &error_str[idx + pattern.len()..];
            let num_str: String = after.chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if let Ok(secs) = num_str.parse::<f64>() {
                if error_lower[idx..].contains("second") {
                    return Some(Duration::from_secs_f64(secs.max(0.5)));
                }
            }
        }
    }

    None
}
```

### 2. Integrate with StreamErrorKind (PROV-045)

```rust
// In classify_stream_error():
if error_lower.contains("rate_limit") || error_lower.contains("429")
    || error_lower.contains("too many requests")
{
    let retry_after = parse_retry_after_from_error(&error_str);
    return StreamErrorKind::RateLimit { retry_after };
}
```

### 3. Handle in stream_loop.rs error branch

```rust
StreamErrorKind::RateLimit { retry_after } => {
    let delay = retry_after.unwrap_or_else(|| {
        // Use RetryPolicy backoff as fallback (PROV-043)
        StreamRetryPolicy::default().delay_for_attempt(0)
    });

    info!("PROV-049: Rate limited, waiting {:?} before retry", delay);
    output.emit_status(&format!(
        "Rate limited. Waiting {:.0}s before retrying...",
        delay.as_secs_f64()
    ));

    tokio::time::sleep(delay).await;

    // Retry by creating a new stream (same pattern as PROV-040/041)
    // ... create retry_token_state, retry_hook ...
    stream = agent
        .prompt_streaming_with_history_and_hook(
            effective_prompt,
            &mut session.messages,
            retry_hook,
        )
        .await;

    continue;
}
```

### 4. Tests

```rust
#[test]
fn parse_retry_after_integer() {
    let error = "rate_limit_error: Too many requests. Retry-After: 7";
    assert_eq!(
        parse_retry_after_from_error(error),
        Some(Duration::from_secs(7))
    );
}

#[test]
fn parse_retry_after_float() {
    let error = "Rate limited. retry-after: 3.5 seconds";
    assert_eq!(
        parse_retry_after_from_error(error),
        Some(Duration::from_secs_f64(3.5))
    );
}

#[test]
fn parse_retry_after_natural_language() {
    let error = "Too many requests. Please try again in 10 seconds.";
    assert_eq!(
        parse_retry_after_from_error(error),
        Some(Duration::from_secs(10))
    );
}

#[test]
fn parse_retry_after_no_match() {
    let error = "Unknown error occurred";
    assert_eq!(parse_retry_after_from_error(error), None);
}

#[test]
fn parse_retry_after_minimum_half_second() {
    let error = "retry-after: 0";
    assert_eq!(
        parse_retry_after_from_error(error),
        Some(Duration::from_secs_f64(0.5))
    );
}
```

## Dependencies

- PROV-043 (RetryPolicy) — shares backoff calculation as fallback
- PROV-045 (StreamErrorKind) — RateLimit variant with retry_after field

## Estimated Effort: 3 story points
