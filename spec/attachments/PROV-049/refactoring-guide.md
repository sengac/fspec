# PROV-049: Refactoring Guide — Retry-After Parsing + Error Enum Extension

## Refactoring Opportunity

PROV-049 extends the error classification (PROV-045) and retry system (PROV-043) with server-guided retry timing. This card demonstrates that the refactored architecture is **truly open for extension** — it adds a new error recovery path without modifying the stream processor core.

## The Type Erasure Problem

The root cause: `rig` converts all HTTP errors to `anyhow::Error` before they reach the stream loop. Typed information (HTTP status code, Retry-After header) is lost. We must parse it back from error message strings.

### What Provider Error Messages Look Like

From the rig source and provider implementations:

```
// Anthropic (via reqwest)
"Request failed with status 429 Too Many Requests: {\"error\":{\"type\":\"rate_limit_error\",\"message\":\"Rate limit exceeded. Retry after 30 seconds.\"}}"

// OpenAI
"Error: 429 - {\"error\":{\"message\":\"Rate limit reached. Please retry after 25s.\",\"type\":\"rate_limit\",\"code\":\"rate_limit_exceeded\"}}"

// Generic
"rate limit exceeded"
"too many requests"
"retry after 15 seconds"
"Retry-After: 30"
```

## What to Add to `stream_errors.rs` (PROV-045's module)

### Rate Limit Detection

```rust
/// Detect rate limiting from error strings.
fn is_rate_limit_error(error_str: &str) -> bool {
    let lower = error_str.to_lowercase();
    lower.contains("rate limit")
        || lower.contains("rate_limit")
        || lower.contains("too many requests")
        || lower.contains("429")
        || lower.contains("quota exceeded")
        || lower.contains("throttl")
}
```

### Retry-After Parsing

```rust
use regex::Regex;
use std::sync::LazyLock;

static RETRY_AFTER_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| vec![
    // "retry-after: 30" (HTTP header style)
    Regex::new(r"(?i)retry-after:\s*(\d+\.?\d*)").unwrap(),
    // "retry after 30 seconds" (prose style)
    Regex::new(r"(?i)retry\s+after\s+(\d+\.?\d*)\s*(?:s|sec|seconds)?").unwrap(),
    // "wait 15 seconds" / "wait 15s"
    Regex::new(r"(?i)wait\s+(\d+\.?\d*)\s*(?:s|sec|seconds)?").unwrap(),
    // "try again in 30 seconds"
    Regex::new(r"(?i)try\s+again\s+in\s+(\d+\.?\d*)\s*(?:s|sec|seconds)?").unwrap(),
    // "Please retry after 25s" (OpenAI style)
    Regex::new(r"(?i)please\s+retry\s+after\s+(\d+\.?\d*)\s*s?").unwrap(),
]);

const MIN_RETRY_AFTER: f64 = 0.5;  // Floor: never retry faster than 500ms

/// Parse a Retry-After value from an error message string.
/// Returns seconds to wait, or None if no parseable value found.
pub fn parse_retry_after_from_error(error_str: &str) -> Option<f64> {
    for pattern in RETRY_AFTER_PATTERNS.iter() {
        if let Some(captures) = pattern.captures(error_str) {
            if let Some(value) = captures.get(1) {
                if let Ok(secs) = value.as_str().parse::<f64>() {
                    return Some(secs.max(MIN_RETRY_AFTER));
                }
            }
        }
    }
    None
}
```

### Integration in classify_stream_error()

This is already stubbed in the PROV-045 refactoring guide. The full implementation:

```rust
pub fn classify_stream_error(error: &anyhow::Error) -> StreamErrorKind {
    let error_str = error.to_string();

    // ... existing checks (compaction, prompt_too_long, image, truncation) ...

    // PROV-049: Rate limit detection with Retry-After extraction
    if is_rate_limit_error(&error_str) {
        let retry_after = parse_retry_after_from_error(&error_str);
        return StreamErrorKind::RateLimit {
            retry_after_secs: retry_after,
            raw_message: error_str,
        };
    }

    // ... remaining checks (network, auth, unknown) ...
}
```

## Integration in stream_loop.rs (or stream_processor.rs after PROV-043)

The rate limit handler in the match arm:

```rust
StreamOutcome::Error(kind) => {
    match kind {
        // ... other arms ...

        StreamErrorKind::RateLimit { retry_after_secs, raw_message } => {
            let delay = retry_after_secs
                .map(Duration::from_secs_f64)
                .unwrap_or_else(|| retry.policy.delay_for_attempt(1)); // PROV-043 fallback

            output.emit_status(&format!(
                "Rate limited. Waiting {:.1}s before retry...",
                delay.as_secs_f64()
            ));
            tokio::time::sleep(delay).await;

            // Record as API failure for circuit breaker (PROV-044)
            session.record_api_failure();

            // Retry with same prompt
            let (ts, hook) = retry.create_retry_hook(session, threshold);
            stream = agent.prompt_streaming_with_history_and_hook(
                prompt, &mut session.messages, hook
            ).await;
            ctx.reset_for_retry(session);
            continue;
        }

        // ... fallback ...
    }
}
```

## Dependency Chain Demonstrated

```
PROV-045 provides: StreamErrorKind::RateLimit variant + classify_stream_error()
PROV-043 provides: RetryOrchestrator.policy.delay_for_attempt() as fallback
PROV-044 provides: session.record_api_failure() for cross-turn tracking
PROV-049 adds:     parse_retry_after_from_error() + rate limit handler
```

This is the **OCP in action** — three existing modules extended, none modified in their core logic.

## Tests

```rust
#[test]
fn parse_retry_after_header_style() {
    assert_eq!(parse_retry_after_from_error("Retry-After: 30"), Some(30.0));
}

#[test]
fn parse_retry_after_prose_style() {
    assert_eq!(parse_retry_after_from_error("Rate limit exceeded. Retry after 25 seconds."), Some(25.0));
}

#[test]
fn parse_retry_after_openai_style() {
    assert_eq!(parse_retry_after_from_error("Please retry after 25s."), Some(25.0));
}

#[test]
fn parse_retry_after_fractional() {
    assert_eq!(parse_retry_after_from_error("retry-after: 1.5"), Some(1.5));
}

#[test]
fn parse_retry_after_minimum_floor() {
    assert_eq!(parse_retry_after_from_error("retry-after: 0.1"), Some(0.5)); // floor
}

#[test]
fn parse_retry_after_missing() {
    assert_eq!(parse_retry_after_from_error("rate limit exceeded"), None);
}

#[test]
fn classify_rate_limit_with_retry_after() {
    let err = anyhow!("429 Too Many Requests: retry after 30 seconds");
    match classify_stream_error(&err) {
        StreamErrorKind::RateLimit { retry_after_secs, .. } => {
            assert_eq!(retry_after_secs, Some(30.0));
        }
        other => panic!("Expected RateLimit, got {:?}", other),
    }
}

#[test]
fn classify_rate_limit_without_retry_after() {
    let err = anyhow!("rate limit exceeded");
    match classify_stream_error(&err) {
        StreamErrorKind::RateLimit { retry_after_secs, .. } => {
            assert_eq!(retry_after_secs, None);
        }
        other => panic!("Expected RateLimit, got {:?}", other),
    }
}
```

## Estimated Impact

- **Lines added to stream_errors.rs**: ~60 (detection + parsing + tests)
- **Lines added to stream_loop.rs**: ~20 (match arm for RateLimit)
- **New dependencies**: `regex` (already in Cargo.toml)
- **No files modified in their core logic** — pure extension
