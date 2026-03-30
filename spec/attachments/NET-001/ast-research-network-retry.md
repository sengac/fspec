# AST Research: Network Retry Implementation

## Core Functions

### `is_transient_network_error()` — Error Classifier
- **File:** `codelet/cli/src/interactive/error_classifiers.rs:69`
- **Signature:** `pub fn is_transient_network_error(error_str: &str) -> bool`
- **Purpose:** Detects 17+ transient network error patterns (connection reset, DNS timeout, broken pipe, SSL errors, unexpected EOF, SSE HTTP client errors)
- **Distinguishes from:** rate limits, auth errors, content policy violations, prompt-too-long, truncated tool calls

### `network_retry_delay()` — Exponential Backoff
- **File:** `codelet/cli/src/interactive/recovery_network.rs:29`
- **Signature:** `pub fn network_retry_delay(attempt: u32) -> Duration`
- **Backoff:** 1s → 2s → 4s (base_delay * 2^(attempt-1))

### `MAX_NETWORK_RETRIES` — Budget Constant
- **File:** `codelet/cli/src/interactive/recovery_network.rs:19`
- **Value:** `3`

## Integration Points

### `stream_loop.rs` — Main Stream Loop
- **Import:** Lines 74, 91
- **Retry counter:** Line 496 (`network_retry_count: u32 = 0`)
- **Counter reset on success:** Lines 609, 621, 689, 721 (on Text, ToolCall, Usage, FinalResponse)
- **Retry logic:** Lines 1191-1279 (detection → increment → backoff → Continue prompt → fresh CompactionHook/TokenState)
- **Exhaustion:** Line 1276-1279 (fatal after MAX_NETWORK_RETRIES)

### `compaction_retry.rs` — Post-Compaction Retry Stream
- **Import:** Lines 9-10
- **Retry counter:** Line 190
- **Counter reset:** Lines 217, 226
- **Retry logic:** Lines 309-337

### `deep_search_handler.rs` — DeepSearch Sub-Agent
- Retry logic in `collect_final_response_from_stream()`

### `mod.rs` — Module Exports
- Lines 25, 43-44: Re-exports `is_transient_network_error`, `MAX_NETWORK_RETRIES`, `network_retry_delay`

## Test File
- **File:** `codelet/cli/tests/network_retry_test.rs` (28 tests)
- Tests error classification, backoff delays, budget limits, real-world error strings
