# Token Inflation Bug Analysis (15.7x Overcounting)

## Executive Summary

The token tracking system displays and persists token counts that are **15.7 times higher** than the actual current context size. This is caused by summing Anthropic's per-request `input_tokens` (which represents the full context size) as if they were incremental additions.

## Root Cause

### The Semantic Misunderstanding

**What Anthropic's API reports:**
- `input_tokens`: Total tokens in the prompt for THIS API call (full context size)
- Each API call re-sends the full conversation history
- `input_tokens` = entire context window at that moment, NOT incremental tokens

**What fspec is doing:**
- SUMMING all `input_tokens` across all API calls
- Treating them as incremental additions

### Example Scenario

```
Turn 1 with 3 API calls (user message + 2 tool uses):

API Call 1: Context = system + history + user = 10,000 tokens
  → usage.input_tokens = 10,000 (full context)

API Call 2: Context = previous + response + tool result = 12,000 tokens
  → usage.input_tokens = 12,000 (full context, NOT +2,000)

API Call 3: Context = previous + more results = 14,000 tokens
  → usage.input_tokens = 14,000 (full context, NOT +2,000)

CURRENT BEHAVIOR:
  turn_accumulated_input = 10,000 + 12,000 + 14,000 = 36,000
  session.token_tracker.input_tokens += 36,000

CORRECT BEHAVIOR (for context tracking):
  current_context_size = 14,000 (just the latest value)
```

### The 15.7x Calculation

From the session `09d7e4ea-0806-4814-9003-8fa10932009e.json`:
- `total_input_tokens`: 1,658,550
- Actual current context: ~105,000 tokens
- Ratio: 1,658,550 / 105,000 = **15.79x**

This means there were approximately 15-16 API calls, each reporting the full context (~105k), and fspec summed them all.

## Affected Code

### 1. stream_loop.rs - Accumulation Logic

**File:** `cli/src/interactive/stream_loop.rs`

**Lines 401-404 (MessageStart handling):**
```rust
if usage.output_tokens == 0 {
    // MessageStart - new API call starting
    turn_accumulated_input += current_api_input;  // ← WRONG: treats absolute as incremental
    turn_accumulated_output += current_api_output;
    current_api_input = usage.input_tokens;
    current_api_output = 0;
}
```

**Lines 447-449 (FinalResponse handling):**
```rust
// Commit final API call to accumulated totals
turn_accumulated_input += current_api_input;  // ← WRONG: adds full context again
turn_accumulated_output += current_api_output;
```

**Line 811 (Session update):**
```rust
session.token_tracker.input_tokens += turn_accumulated_input;  // ← Persists wrong sum
```

### 2. Display Logic

**Lines 417-420:**
```rust
output.emit_tokens(&TokenInfo {
    input_tokens: prev_input_tokens           // Previous cumulative
                + turn_accumulated_input      // This turn's sum
                + current_api_input,          // Current API call
    // ...
});
```

This displays a cumulative billing total (1.6M) instead of current context size (105k).

### 3. Persistence

**File:** `napi/src/persistence/types.rs`

**Line 188:**
```rust
self.token_usage.total_input_tokens += input;  // Accumulates the wrong sum
```

## What Works Correctly

The `CompactionHook` correctly handles per-request tokens:

**File:** `core/src/compaction_hook.rs`

**Line 134:**
```rust
// on_stream_completion_response_finish OVERWRITES (not adds):
state.input_tokens = usage.input_tokens;  // Correct: uses per-request value
```

This is why compaction threshold checking works correctly - it uses the actual per-request context size, not the cumulative sum.

## Impact

| Area | Impact |
|------|--------|
| **Display** | Users see 1.6M tokens instead of 105k - highly misleading |
| **Session Persistence** | Wrong values stored, confuses cost analysis |
| **Compaction** | Works correctly (hook overwrites with per-request) |
| **User Trust** | Users think system is consuming excessive tokens |

## Proposed Fix

### Option 1: Separate Billing vs Context Tracking

Track two distinct metrics:

```rust
pub struct TokenTracker {
    // Current context size (for display and context management)
    pub current_context_tokens: u64,

    // Cumulative billing (for cost analysis)
    pub cumulative_billed_input: u64,
    pub cumulative_billed_output: u64,

    // Cache metrics (per-request, not cumulative)
    pub cache_read_input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
}
```

### Option 2: Fix Display to Show Current Context

Keep the cumulative tracking for billing but fix the display:

```rust
// In stream_loop.rs - emit current context, not cumulative
output.emit_tokens(&TokenInfo {
    input_tokens: current_api_input,  // Just the current context size
    output_tokens: current_api_output,
    cache_read_input_tokens: Some(turn_cache_read),
    cache_creation_input_tokens: Some(turn_cache_creation),
});
```

### Option 3: Complete Overhaul (Recommended)

1. **Rename fields for clarity:**
   - `total_input_tokens` → `cumulative_billed_input_tokens`
   - Add `current_context_tokens` for actual context size

2. **Update stream_loop.rs:**
   - Track `current_context_tokens = latest input_tokens` (overwrite, not add)
   - Track `cumulative_billed_input += input_tokens` separately

3. **Update display:**
   - Show current context size prominently
   - Show cumulative billing in a separate/secondary display

4. **Update session persistence:**
   - Store both metrics clearly labeled

## Testing Strategy

1. **Unit Tests:**
   - Verify `current_context_tokens` equals latest API's `input_tokens`
   - Verify `cumulative_billed_input` equals sum of all API `input_tokens`

2. **Integration Tests:**
   - Multi-turn conversation with tool use
   - Verify displayed tokens match actual context size
   - Verify cumulative billing is correct for cost analysis

3. **Manual Verification:**
   - Compare displayed tokens with Anthropic dashboard
   - Verify compaction triggers at correct thresholds

## Relationship to tokens.md

The existing `tokens.md` document describes a "double-counting" bug in the streaming display logic. That is a **secondary issue** on top of this primary bug. The double-counting happens because:

1. `turn_accumulated_input += current_api_input` (on MessageStart)
2. Then display shows `turn_accumulated_input + current_api_input` (double-counts current)

However, the **primary bug** is the semantic confusion: treating absolute context sizes as incremental additions. Even fixing the double-counting won't fix the 15.7x inflation - it would only reduce it to ~15x.

## Priority

**HIGH** - This bug:
- Misleads users about token consumption
- Makes cost analysis unreliable
- Undermines trust in the system
- Has been present since the multi-API-call tracking was implemented
