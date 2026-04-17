# CMPCT-027 — Complete Error Cascade in `run_retry_stream`

**Parent:** CMPCT-022
**Bug:** BUG 5

## The Problem

`codelet/cli/src/interactive/compaction_retry.rs:323-359` — the error-handling block in `run_retry_stream` is a pale imitation of the primary stream loop's error cascade:

```rust
Some(Err(e)) => {
    let error_str = e.to_string();
    if is_transient_network_error(&error_str) {
        // Network retry with backoff (NET-001)
        // ...
    }
    output.emit_error(&error_str);
    return Err(anyhow::anyhow!("Retry error after compaction: {e}"));
}
```

### Missing Handlers

Compare with the PRIMARY stream loop (`stream_loop.rs:1131-1450`), which handles:

| Error Class                   | Primary Loop | Retry Stream |
|-------------------------------|:------------:|:------------:|
| `is_compaction_cancelled`     | ✅ (break)   | ❌           |
| `is_stall_timeout_error`      | ✅ (abort)   | ❌           |
| `is_prompt_too_long_error`    | ✅ (compact) | ❌           |
| `is_image_content_error`      | ✅ (sanitize)| ❌           |
| `is_truncated_tool_call_error`| ✅ (recover) | ❌           |
| `is_transient_network_error`  | ✅ (retry)   | ✅ (retry)   |

## The Real-World Scenarios This Creates

### Scenario 1 — Compaction didn't shrink enough
1. User sends message → hook cancels at threshold → compaction runs.
2. `execute_compaction` builds DAG, but the compacted context + system prompt + tool schemas STILL exceeds the API's hard limit (e.g., `200000 tokens > 200000 maximum`).
3. Retry stream fires request → API returns `"prompt is too long"`.
4. `run_retry_stream` has no `is_prompt_too_long` handler → falls through to generic `return Err(...)`.
5. User sees "Retry error after compaction: prompt is too long: 209834 tokens > 200000 maximum".
6. **Session is dead.** No second compaction attempt, no escalation to emergency threshold.

### Scenario 2 — Retry itself blows the budget
1. Compaction completes → retry stream starts with "Continue".
2. LLM produces a very long response + tool result.
3. On the NEXT `on_completion_call` within the retry stream, hook fires AGAIN → `PromptCancelled`.
4. `run_retry_stream` has no `is_compaction_cancelled` handler → falls through to generic `return Err(...)`.
5. Session dies instead of running another compaction round.

### Scenario 3 — Truncation in retry
1. Compacted context is borderline-large; LLM response is truncated mid-tool-call.
2. Primary loop would have triggered truncation recovery (`PROV-040`).
3. `run_retry_stream` has no `is_truncated_tool_call_error` handler → dies.

### Scenario 4 — Stall timeout during retry
1. Provider hangs during retry stream.
2. Primary loop would return a stall timeout (`AMGR-016`) and terminate cleanly.
3. `run_retry_stream` routes stall timeout through `is_transient_network_error` incorrectly, gets misclassified (or not caught at all — the stall timeout string check at line 1169 is absent from retry).

## The Fix

### Option A — Share the cascade

Extract the primary loop's error handler into a helper:

```rust
enum StreamRecoveryAction {
    BreakForCompaction,
    RetryTruncated(String),      // new prompt
    RetryNetworkAfterDelay(u32), // retry count
    SanitizeImages,
    TerminateStall,
    TerminateTerminal(anyhow::Error),
}

fn classify_and_recover(
    e: anyhow::Error,
    session: &mut Session,
    // ...
) -> StreamRecoveryAction;
```

Then both primary loop and retry loop call it.

### Option B — Don't use a separate retry stream

Restructure so compaction-and-retry happens WITHIN the primary loop's error handler (by changing what `stream` points to), rather than returning from the primary loop and restarting in a new module. This would make BUG 5 impossible by construction.

This is essentially what the truncation retry does at `stream_loop.rs:1231-1302`:
```rust
if is_truncated_tool_call_error(&error_str) {
    // Build recovery prompt
    // Reset per-turn state
    stream = agent.prompt_streaming_with_history_and_hook(...).await;
    continue;   // ← stay in the same loop
}
```

### Recommendation: Option B

- Avoids code duplication entirely.
- Makes future error handling additions automatically work in both primary and post-compaction contexts.
- Aligns with the existing truncation-retry pattern.

Draft:

```rust
// In stream_loop.rs, when compaction is needed:
if compaction_needed {
    execute_compaction(session, compaction_in_progress, Some(prompt)).await?;
    
    // Reset display + token tracker for retry
    session.token_tracker.reset_after_compaction();
    streaming_display = StreamingTokenDisplay::new(...);
    
    // Recreate hook + token_state (use fresh arc)
    token_state = Arc::new(Mutex::new(TokenState { 
        compaction_needed: false, 
        // ...
    }));
    let new_hook = CompactionHook::new(Arc::clone(&token_state), threshold);
    
    // Restart stream in-place (SAME loop; error cascade continues to apply)
    stream = agent
        .prompt_streaming_with_history_and_hook("Continue", &mut session.messages, new_hook)
        .await;
    
    // Reset per-turn state
    tool_calls_buffer.clear();
    last_tool_name = None;
    assistant_text.clear();
    
    // Optional: increment a compaction_retry_count to enforce circuit breaker
    compaction_retry_count += 1;
    if compaction_retry_count > MAX_COMPACTION_RETRIES {
        return Err(anyhow::anyhow!(
            "Compaction retry budget exhausted after {} attempts",
            MAX_COMPACTION_RETRIES
        ));
    }
    
    continue;
}
```

This requires removing the post-loop compaction handler and the separate `compaction_retry.rs` module (or at least demoting it to a helper for the in-loop restart).

### Add a Circuit Breaker

Regardless of Option A vs B, introduce a bounded retry count (follow PROV-042 space):

```rust
const MAX_COMPACTION_RETRIES: u32 = 3;
let mut compaction_retry_count: u32 = 0;
```

If compaction fires 3 times in a single user turn, abort with a clear error. This prevents infinite cascades if compaction somehow produces a context that still exceeds the threshold.

## Acceptance Criteria

1. When the post-compaction stream encounters `prompt is too long` → a second compaction round runs (up to `MAX_COMPACTION_RETRIES`).
2. When the post-compaction stream encounters `PromptCancelled` → recovery runs again (up to `MAX_COMPACTION_RETRIES`).
3. When the post-compaction stream encounters truncation or image errors → the primary recovery handlers run.
4. After `MAX_COMPACTION_RETRIES` exhaustion, a clear budget-exhausted error is returned.
5. Network retry within post-compaction still works (preserve existing NET-001 behavior).

## Files to Modify

- `codelet/cli/src/interactive/stream_loop.rs` (post-loop handler around line 1500-1527)
- `codelet/cli/src/interactive/compaction_retry.rs` — either delete or demote to helper
- Add a `compaction_retry_count` to the state tracked in `run_agent_stream_internal`

## Testing

- Integration test: mock a stream that yields PromptCancelled → after compaction, yield "prompt is too long" → verify second compaction attempt runs.
- Integration test: mock a stream that yields PromptCancelled 4 times in a row → verify circuit breaker trips after 3.
- Integration test: mock post-compaction stream that yields a truncation error → verify truncation recovery runs.
