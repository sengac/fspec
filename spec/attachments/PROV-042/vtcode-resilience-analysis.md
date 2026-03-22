# VTCode Stream Loop Resilience Analysis

## Context

This document captures a detailed comparison between fspec/codelet's stream loop (`codelet/cli/src/interactive/stream_loop.rs`) and VTCode's agent runner / provider response handling system. The goal is to identify operational resilience patterns in VTCode that fspec/codelet can adopt.

**VTCode codebase location:** `/tmp/VTCode` (analyzed 2026-03-22)
**VTCode repo:** https://github.com/vinhnx/VTCode (third-party, analyzed locally)

---

## Executive Summary

fspec/codelet excels at **semantic error recovery** — detecting thinking token exhaustion (PROV-041), truncated tool calls (PROV-040), prompt-too-long errors, and image content errors. VTCode excels at **operational resilience** — structured retries with backoff, circuit breakers, streaming fallback, error classification, and history persistence. These are complementary; fspec should adopt VTCode's operational patterns without losing its semantic recovery capabilities.

---

## 1. Structured Retry with Exponential Backoff

### VTCode Implementation

**Files:**
- `vtcode-core/src/retry.rs` (lines 1–259) — `RetryPolicy` struct, `RetryDecision`, error classification
- `vtcode-core/src/core/agent/runner/retry.rs` (lines 1–135) — `execute_task_with_retry()`

**Key types:**
```rust
// vtcode-core/src/retry.rs:12-20
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
    pub jitter: f64,
}

// vtcode-core/src/retry.rs:193-199
pub struct RetryDecision {
    pub category: ErrorCategory,
    pub retryable: bool,
    pub delay: Option<Duration>,
    pub retry_after: Option<Duration>,
}
```

**How it works:**
1. Each attempt gets a delay: `initial_delay * multiplier^attempt_index`, capped at `max_delay`
2. Jitter adds randomness to prevent thundering herd
3. `Retry-After` header from 429 responses overrides computed delay
4. `RetryPolicy::decision_for_category()` checks both retryability and remaining attempts

**Default policy:**
```rust
// vtcode-core/src/retry.rs:186-190
impl Default for RetryPolicy {
    fn default() -> Self {
        Self::from_retries(2, Duration::from_secs(1), Duration::from_secs(60), 2.0)
    }
}
```
→ 3 total attempts, 1s → 2s → 4s delays, capped at 60s

### fspec Gap

fspec retries PROV-040 and PROV-041 **immediately** with zero delay:
- `stream_loop.rs` line ~1701: creates new stream instantly after thinking exhaustion
- `stream_loop.rs` line ~1887: creates new stream instantly after truncation

No backoff between retries. If the API is rate-limited or overloaded, immediate retry worsens the problem.

### Recommendation

Add a small `RetryPolicy` struct (or even just a `delay_before_retry()` function) and insert a `tokio::time::sleep()` before creating the retry stream. Even a fixed 2-second delay would be better than nothing.

---

## 2. Circuit Breaker / API Failure Tracker

### VTCode Implementation

**Files:**
- `vtcode-core/src/core/agent/state.rs` (lines 111–151) — `ApiFailureTracker`
- `vtcode-core/src/core/agent/runner/provider_response.rs` (lines 211–219) — circuit break check
- `vtcode-core/src/core/agent/runner.rs` (lines 109–115) — tracker on AgentRunner

```rust
// vtcode-core/src/core/agent/state.rs:111-151
pub struct ApiFailureTracker {
    pub consecutive_failures: u32,
    pub last_failure: Option<std::time::Instant>,
}

impl ApiFailureTracker {
    pub fn should_circuit_break(&self) -> bool {
        self.consecutive_failures >= 3
    }

    pub fn backoff_duration(&self) -> Duration {
        let base_ms = 1000;
        let max_ms = 30000;
        let backoff_ms = base_ms * 2_u64.pow(self.consecutive_failures.saturating_sub(1));
        Duration::from_millis(backoff_ms.min(max_ms))
    }
}
```

**Usage in provider_response.rs (lines 211-219):**
```rust
if self.failure_tracker.lock().should_circuit_break() {
    let backoff = self.failure_tracker.lock().backoff_duration();
    warn!("Circuit breaker active after {} consecutive failures. Waiting {:?} before retry.",
        self.failure_tracker.lock().consecutive_failures, backoff);
    tokio::time::sleep(backoff).await;
}
```

**Reset on success (line 282):**
```rust
self.failure_tracker.lock().reset();
```

### fspec Gap

No failure tracking whatsoever. If the API returns errors on 3 consecutive turns, fspec keeps trying at full speed. Each turn independently encounters the same error with no cross-turn memory.

### Recommendation

Add an `ApiFailureTracker` (or equivalent) to `Session`. Increment on API errors in the `Some(Err(e))` branch of the stream loop. Check before starting a new stream. Reset on successful FinalResponse. ~30 lines of code for significant improvement.

---

## 3. Streaming Fallback to Non-Streaming

### VTCode Implementation

**File:** `vtcode-core/src/core/agent/runner/provider_response.rs` (lines 36–293)

```rust
// Tracks streaming failures separately
streaming_disabled = *self.streaming_failures.lock() >= super::constants::MAX_STREAMING_FAILURES;

// If streaming fails, falls back to non-streaming generate()
let fallback_request = LLMRequest {
    stream: false,
    ..request.clone()
};
let response = self.provider_client.generate(fallback_request).await;
```

**Cooldown mechanism (lines 37-45):**
```rust
if let Some(last_failure) = *self.streaming_last_failure.lock()
    && last_failure.elapsed().as_secs() >= STREAMING_COOLDOWN_SECS
{
    *self.streaming_failures.lock() = 0;  // Re-enable streaming after cooldown
    self.streaming_last_failure.lock().take();
}
```

### fspec Gap

Streaming is all-or-nothing. If `stream.next()` returns an error, it propagates to the user. No fallback path.

### Recommendation

This is harder to adopt because rig's abstraction wraps streaming tightly. However, **tracking streaming reliability** is still valuable — emit warnings like "Streaming has failed N times in this session" to help diagnose provider issues.

---

## 4. History Persistence Before Compaction

### VTCode Implementation

**File:** `vtcode-core/src/core/agent/runner/summarize.rs` (lines 99–141)
**File:** `vtcode-core/src/context/history_files.rs` (full file — JSONL history persistence)

```rust
// summarize.rs:104-141 — persist_history_before_summarization()
pub(super) fn persist_history_before_summarization(
    &self,
    conversation: &[Content],
    session_id: &str,
    turn_number: usize,
    modified_files: &[String],
    executed_commands: &[String],
) -> Option<std::path::PathBuf> {
    // Writes full conversation to .vtcode/history/session_{id}_{turn}.jsonl
    // Returns path for inclusion in summary message
}
```

**Summary message includes file reference (lines 74-79):**
```rust
let summary = if let Some(path) = history_file_path {
    format!(
        "{}\n\nFull conversation history saved to: {}\nUse grep_file to search for specific details if needed.",
        base_summary, path.display()
    )
} else {
    base_summary
};
```

### fspec Gap

Compaction in `execute_compaction()` is lossy. The original conversation is replaced by the summary. If the summary loses critical context (file paths, specific error messages, decisions), it's gone permanently. This is the root cause of the PromptCancelled recovery sessions we've experienced.

### Recommendation

Before compaction, write the full conversation to `.codelet/history/session_{id}.jsonl`. Include the path in the compaction summary. Agents can then use SessionSearch/Read to recover lost details. Cost: one file write. Benefit: prevents the "compaction ate my context" problem.

---

## 5. Response Loop Detection

### VTCode Implementation

**File:** `vtcode-core/src/core/agent/completion.rs` (lines 68–108)

```rust
// completion.rs:70-108
pub fn check_for_response_loop(response_text: &str, session_state: &mut AgentSessionState) -> bool {
    if response_text.len() < 10 {
        return false;
    }

    let normalized_current = response_text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let repeated = session_state
        .messages
        .iter()
        .rev()
        .filter(|m| m.role == MessageRole::Assistant)
        .skip(1)  // Skip current (just added)
        .take(2)  // Check last 2 assistant messages
        .any(|m| {
            let normalized_prev = m.content.as_text()
                .split_whitespace().collect::<Vec<_>>().join(" ");
            normalized_prev == normalized_current
        });

    if repeated {
        // Emit warning, increment idle counter, break loop
    }
    repeated
}
```

**Usage in execute.rs (lines 519-534):**
```rust
if check_for_response_loop(response.content_text(), &mut controller.state) {
    controller.state.outcome = TaskOutcome::LoopDetected;
    break;
}
```

### fspec Gap

No loop detection. If the model enters a repetitive pattern (common when confused or stuck), fspec will loop indefinitely until the user interrupts or context fills up.

### Recommendation

After each `FinalResponse`, normalize and compare `assistant_text` against the previous 1-2 assistant messages in `session.messages`. If it's a duplicate, emit a warning and break. This is ~20 lines in the FinalResponse handler.

---

## 6. Structured Error Classification

### VTCode Implementation

**File:** `vtcode-core/src/error.rs` (lines 1–96)

```rust
// error.rs:23-45
pub struct VtCodeError {
    pub category: ErrorCategory,     // Network, Timeout, Auth, Policy, etc.
    pub code: ErrorCode,             // 30+ machine-readable codes
    pub message: String,
    pub context: Option<String>,
    pub retry_after_ms: Option<u64>,
}

// ErrorCode enum (lines 49-96):
pub enum ErrorCode {
    InvalidArgument, ValidationFailed, ParseError,
    CommandFailed, ToolExecutionFailed, Timeout,
    ConnectionFailed, RequestFailed, RateLimited, ServiceUnavailable,
    AuthenticationFailed, LLMProviderError, TokenLimitExceeded, ContextTooLong,
    // ... 16 more codes
}
```

**Error handling in retry.rs (lines 117-141):**
```rust
// Tries to downcast anyhow::Error to typed errors in priority order:
if let Some(vtcode_error) = error.downcast_ref::<VtCodeError>() { ... }
if let Some(llm_error) = error.downcast_ref::<LLMError>() { ... }
if let Some(tool_error) = error.downcast_ref::<UnifiedToolError>() { ... }
// Falls back to string-based classification
let category = vtcode_commons::classify_anyhow_error(error);
```

**Provider-specific error handling:**
- `vtcode-core/src/llm/providers/error_handling.rs` — lists retryable error codes including "resource_exhausted"
- `vtcode-core/src/llm/providers/anthropic/response_parser.rs:150-154` — `parse_finish_reason()` maps stop_reason strings to typed enum

### fspec Gap

All error classification is via string matching:
- `is_prompt_too_long_error()` — 6 string patterns
- `is_truncated_tool_call_error()` — 1 string pattern
- `is_thinking_exhaustion()` — checks stop_reason strings
- `is_image_content_error()` — 5 string patterns
- `is_compaction_cancelled()` — 1 string pattern

Each new error type adds another function. No shared classification. Provider-specific error messages break detection.

### Recommendation

Create a `StreamErrorKind` enum and a single `classify_stream_error()` function:

```rust
enum StreamErrorKind {
    PromptTooLong,
    TruncatedToolCall { tool_name: String, partial_args: String },
    ThinkingExhaustion { reasoning_tokens: u64, output_tokens: u64 },
    ImageContent,
    CompactionCancelled,
    RateLimit { retry_after: Option<Duration> },
    NetworkTransient,
    AuthFailure,
    Unknown,
}
```

This centralizes error classification, makes the stream loop's error branch a simple match, and makes it easy to add new providers.

---

## 7. Error Recovery State (Circuit Events + Patterns)

### VTCode Implementation

**File:** `vtcode-core/src/core/agent/error_recovery.rs` (full file — 301 lines)

```rust
// error_recovery.rs:8-14
pub struct ErrorRecoveryState {
    pub recent_errors: VecDeque<RecentError>,
    pub circuit_events: Vec<CircuitEvent>,
    pub pause_threshold: usize,
    pub last_recovery_prompt: Option<Instant>,
    pub recovery_cooldown: std::time::Duration,
}
```

**Key capabilities:**
- Tracks last N errors (default 10) with tool name, timestamp, error type, category
- Detects error patterns (same tool failing repeatedly)
- Circuit events track when tools enter/exit circuit-breaker state
- `RecoveryDiagnostics` provides a summary: open circuits, patterns, whether to pause
- Cooldown prevents spamming recovery prompts (60s minimum between prompts)

### fspec Gap

No error tracking across the stream loop. Each error is handled independently. No way to detect "the same tool has failed 5 times" or "3 different tools are broken."

### Recommendation

This is lower priority than the circuit breaker (item 2), but when implementing the circuit breaker, consider also tracking which specific errors/tools are failing. This enables smarter recovery messages like "Bash tool has failed 3 times with permission errors — check your workspace permissions."

---

## 8. Rate Limit Handling with Retry-After

### VTCode Implementation

**File:** `vtcode-core/src/retry.rs` (lines 211-219)

```rust
// retry.rs:211-219
fn parse_retry_after_header(metadata: &LLMErrorMetadata) -> Option<Duration> {
    let raw = metadata.retry_after.as_deref()?.trim();
    if let Ok(seconds) = raw.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    if let Ok(seconds) = raw.parse::<f64>() {
        return Some(Duration::from_secs_f64(seconds.max(0.0)));
    }
    None
}
```

**Usage in decision_for_llm_error (lines 143-151):**
```rust
pub fn decision_for_llm_error(&self, error: &LLMError, attempt_index: u32) -> RetryDecision {
    let retry_after = llm_metadata(error).and_then(parse_retry_after_header);
    self.decision_for_category_with_tool(
        ErrorCategory::from(error),
        attempt_index,
        retry_after,  // Overrides computed backoff
        None,
    )
}
```

### fspec Gap

Rate limit errors are treated like any other API error — reported to the user, stream terminates. Anthropic returns precise `Retry-After` values that could be used to transparently retry.

### Recommendation

When the error string contains "rate_limit" or similar, parse the Retry-After value (if available from the provider error metadata) and sleep before retrying. This converts a user-visible error into a transparent pause.

---

## 9. Tool Call/Output Pairing Validation

### VTCode Implementation

**File:** `vtcode-core/src/core/agent/state.rs` (lines 39-88)

```rust
// state.rs:62-68
pub struct HistoryValidationReport {
    pub missing_outputs: Vec<MissingOutput>,  // Calls without results
    pub orphan_outputs: Vec<ToolCallId>,       // Results without calls
}
```

**Used in summarization to find safe split points:**
```rust
// summarize.rs:30
let split_at = session_state.find_safe_split_point(preferred_split_at);
```

### fspec Gap

Compaction can split a tool call from its tool result, leaving an orphaned call or result in the compacted history. This can confuse the model in subsequent turns.

### Recommendation

Before compaction, validate that the split point doesn't break a tool call/result pair. Walk backward from the proposed split point until you find a boundary where all calls have matching results.

---

## 10. Completion Detection (Lower Priority)

### VTCode Implementation

**File:** `vtcode-core/src/core/agent/completion.rs` (lines 5-66)

```rust
const COMPLETION_SENTENCES: &[&str] = &[
    "the task is complete",
    "task has been completed",
    "i have successfully completed the task",
    "work is finished",
    // ... 9 more patterns
];
```

Combined with `ContinuationController` (file: `vtcode-core/src/core/agent/runner/continuation.rs`) which:
1. Checks a task tracker for incomplete items
2. Runs verification commands (`cargo check`, `npm test`)
3. Injects continuation prompts if work is incomplete

### fspec Gap

No completion detection. The stream runs until FinalResponse or error. In autonomous mode (NAPI/agent sessions), the agent can declare victory prematurely.

### Recommendation

Lower priority for now. Revisit when codelet's autonomous agent sessions become a primary use case.

---

## Priority Summary

| # | Feature | Impact | Effort | Child Card |
|---|---------|--------|--------|------------|
| 1 | Structured retry with backoff | HIGH | Small | PROV-043 |
| 2 | Circuit breaker / failure tracker | HIGH | Small | PROV-044 |
| 3 | Error classification enum | MEDIUM | Medium | PROV-045 |
| 4 | History persistence before compaction | MEDIUM | Small | PROV-046 |
| 5 | Response loop detection | MEDIUM | Small | PROV-047 |
| 6 | Streaming failure tracking | MEDIUM | Small | PROV-048 |
| 7 | Rate limit Retry-After parsing | LOW | Small | PROV-049 |
| 8 | Tool call/output split-safe compaction | LOW→HIGH | Medium | PROV-050 |
| 9 | Completion detection (future) | LOW | Large | Deferred |
