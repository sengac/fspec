# AST Research: Token Tracking Code Analysis

## Research Summary

Analysis of token tracking code paths in codelet to understand the token inflation bug.

## Files Analyzed

### 1. cli/src/interactive/stream_loop.rs

**Key Function:** `run_agent_stream_internal` (line 113)

**Token Tracking Variables (lines 269-281):**
```rust
let prev_input_tokens = session.token_tracker.input_tokens;       // line 270
let prev_output_tokens = session.token_tracker.output_tokens;     // line 271
let mut turn_accumulated_input: u64 = 0;                          // line 275
let mut turn_accumulated_output: u64 = 0;                         // line 276
let mut current_api_input: u64 = 0;                               // line 280
let mut current_api_output: u64 = 0;                              // line 281
```

**BUG LOCATION 1 - MessageStart Handler (lines 397-409):**
```rust
if usage.output_tokens == 0 {
    // MessageStart - new API call starting
    turn_accumulated_input += current_api_input;   // BUG: Adds previous call's tokens
    turn_accumulated_output += current_api_output;
    current_api_input = usage.input_tokens;        // Then sets current to new value
    current_api_output = 0;
}
```

**BUG LOCATION 2 - FinalResponse Handler (lines 447-449):**
```rust
// Commit final API call to accumulated totals
turn_accumulated_input += current_api_input;   // BUG: Same value added again
turn_accumulated_output += current_api_output;
```

**BUG LOCATION 3 - Display Emission (lines 417-426):**
```rust
output.emit_tokens(&TokenInfo {
    input_tokens: prev_input_tokens
        + turn_accumulated_input
        + current_api_input,           // Shows cumulative sum, not current context
    output_tokens: prev_output_tokens
        + turn_accumulated_output
        + current_api_output,
    cache_read_input_tokens: Some(turn_cache_read),
    cache_creation_input_tokens: Some(turn_cache_creation),
});
```

**BUG LOCATION 4 - Session Update (lines 810-812):**
```rust
session.token_tracker.input_tokens += turn_accumulated_input;   // Persists buggy sum
session.token_tracker.output_tokens += turn_accumulated_output;
```

### 2. napi/src/persistence/types.rs

**Key Function:** `update_token_usage` (line 181)

```rust
pub fn update_token_usage(
    &mut self,
    input: u64,
    output: u64,
    cache_read: u64,
    cache_create: u64,
) {
    self.token_usage.total_input_tokens += input;    // Accumulates the buggy sum
    self.token_usage.total_output_tokens += output;
    self.token_usage.cache_read_tokens += cache_read;
    self.token_usage.cache_creation_tokens += cache_create;
}
```

### 3. core/src/compaction_hook.rs (WORKING CORRECTLY)

**Key Function:** `on_stream_completion_response_finish` (line 122)

```rust
async fn on_stream_completion_response_finish(
    &self,
    _prompt: &Message,
    response: &<M as CompletionModel>::StreamingResponse,
    _cancel_sig: CancelSignal,
) {
    if let Some(usage) = response.token_usage() {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.input_tokens = usage.input_tokens;   // OVERWRITES - correct behavior!
        state.cache_read_input_tokens = usage.cache_read_input_tokens.unwrap_or(0);
        state.cache_creation_input_tokens = usage.cache_creation_input_tokens.unwrap_or(0);
        state.output_tokens = usage.output_tokens;
    }
}
```

## Bug Flow Trace

For 3 API calls with input_tokens of 50k, 55k, 60k:

1. **API Call 1 MessageStart**: `turn_accumulated += 0`, `current = 50k`
2. **API Call 1 FinalResponse**: `turn_accumulated += 50k` = **50k**
3. **API Call 2 MessageStart**: `turn_accumulated += 50k` = **100k** (DOUBLE!), `current = 55k`
4. **API Call 2 FinalResponse**: `turn_accumulated += 55k` = **155k**
5. **API Call 3 MessageStart**: `turn_accumulated += 55k` = **210k** (DOUBLE!), `current = 60k`
6. **API Call 3 FinalResponse**: `turn_accumulated += 60k` = **270k**

**Result:** Session stores 270k instead of correct 60k (4.5x inflation for 3 calls)

## Required Changes

### stream_loop.rs
1. Track `current_context_tokens` that overwrites (not accumulates)
2. Display should emit `current_context_tokens` only
3. Remove double-counting in MessageStart handler

### types.rs
1. Add `current_context_tokens` field to `TokenUsage`
2. Separate `cumulative_billed_input` for billing analytics
3. Update `update_token_usage` to handle both metrics

### TokenTracker (core/src/common/mod.rs)
1. Add `current_context_tokens: u64` field
2. Keep `input_tokens` for cumulative billing if needed

## Test Coverage Needed

1. Single API call - verify 1:1 ratio
2. Multi-API turn - verify only latest context shown
3. Double-counting prevention - verify tokens not added twice
4. Session persistence - verify both metrics stored correctly
5. Context fill percentage - verify uses current context
