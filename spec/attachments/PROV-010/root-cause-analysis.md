# PROV-010: Root Cause Analysis

## Problem Statement

When using Claude Opus 4.6, the first message in a new session fails with:
```
Stream error: Compaction failed: Cannot compact empty turn history
```

This happens even though:
1. The context is nearly empty (only system prompts)
2. The CompactionHook correctly reports "compaction NOT triggered"
3. There are no user/assistant turns to compact

## Bug Chain Analysis

### Step 1: Opus 4.6 Configuration Error

Opus 4.6 returns a 400 error due to thinking budget misconfiguration:

```json
{
  "type": "error",
  "error": {
    "type": "invalid_request_error",
    "message": "`max_tokens` must be greater than `thinking.budget_tokens`"
  }
}
```

This is a **configuration issue** - `max_tokens=8192` is not greater than the thinking budget being requested.

### Step 2: False Positive Error Classification

The `is_prompt_too_long_error` function in `stream_loop.rs` (lines 45-54):

```rust
fn is_prompt_too_long_error(error_str: &str) -> bool {
    let error_lower = error_str.to_lowercase();
    error_lower.contains("prompt is too long")
        || error_lower.contains("maximum context length")
        || error_lower.contains("context_length_exceeded")
        || error_lower.contains("too many tokens")
        || error_lower.contains("exceeds the model")
        || (error_lower.contains("invalid_request_error")
            && (error_lower.contains("token") || error_lower.contains("maximum")))
}
```

The error message contains:
- `"invalid_request_error"` ✓
- `"token"` ✓ (from `max_tokens` and `thinking.budget_tokens`)

This matches the over-broad condition on lines 52-53, causing **false positive** detection.

### Step 3: Compaction Triggered on Empty History

In the error handler (lines 1173-1193):

```rust
if is_prompt_too_long && !session.messages.is_empty() {
    // ... setup ...
    signal_compaction_needed(&token_state);
    break;
}
```

`session.messages` contains 2 system prompt messages (not empty), so compaction is triggered.

### Step 4: Compaction Fails

The compaction logic partitions messages:
```
partition: system_reminders=2, compactable=0
convert_messages_to_turns: turns_count=0
```

With 0 compactable turns, the compactor fails:
```
[Compactor::compact] FAILING: turns is empty - WHO CALLED THIS WITH EMPTY TURNS?
```

## Log Evidence

From `~/.fspec/fspec.log`:

```
Line 7:  session_set_model: model_id=claude-opus-4-6
Line 14: pre-prompt check: has_turns=false, will_compact=false
Line 18: compaction NOT triggered: 2880 tokens <= 191808 threshold
Line 22: SSE ERROR: Invalid status code 400 Bad Request with message: 
         {"type":"error","error":{"type":"invalid_request_error",
         "message":"`max_tokens` must be greater than `thinking.budget_tokens`"}}
Line 25: [signal_compaction_needed] CALLED - setting compaction_needed=true
Line 29: partition: system_reminders=2, compactable=0
Line 34: [Compactor::compact] FAILING: turns is empty
Line 36: Compaction failed: Cannot compact empty turn history
```

## Two Bugs Identified

### Bug 1: Over-broad `is_prompt_too_long_error` Function

**Location:** `codelet/cli/src/interactive/stream_loop.rs:45-54`

**Problem:** The condition `invalid_request_error && token` matches ANY error containing these strings, including:
- Thinking budget configuration errors
- Token limit errors unrelated to context length
- Any future error mentioning "token"

**Fix Required:** Exclude known false positives like `thinking.budget_tokens`

### Bug 2: Missing Guard for Empty Turn History

**Location:** `codelet/cli/src/interactive/stream_loop.rs:1173-1193`

**Problem:** Even for legitimate prompt-too-long errors, the code triggers compaction without verifying there are actual conversation turns to compact.

**Fix Required:** Check for compactable turns before triggering compaction

## Correct Behavior

For the thinking budget error:
1. Error should NOT be classified as "prompt too long"
2. Error should propagate up to user with clear message
3. Configuration should be fixed (separate issue)

For actual prompt-too-long errors:
1. Should only trigger compaction if there are turns to compact
2. If no turns, should propagate error (nothing to compact anyway)

## Test Strategy

1. Unit tests for `is_prompt_too_long_error`:
   - True positives: actual context length errors
   - False negatives: configuration errors containing "token"
   
2. Integration tests for error handler:
   - Prompt too long with turns → triggers compaction
   - Prompt too long without turns → propagates error
   - Configuration error → propagates error

## Related Work Units

- **PROV-009**: Server-side compaction API support (may affect how we handle Opus 4.6)
- **PROV-005**: Current work unit context
