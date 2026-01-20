# Compaction System Issues Analysis

## Summary

The compaction system is **functionally correct** but has significant code quality issues that increase maintenance burden and risk of future bugs.

---

## Issue 1: Dead Code

### Location
- `codelet/cli/src/compaction_threshold.rs` lines 84-97, 139-161

### Problem
`TokenUsage` struct and `should_trigger_compaction()` function are not used in production code. They only appear in unit tests and documentation.

### Evidence
```rust
// TokenUsage struct (lines 84-88) - DEAD CODE
pub struct TokenUsage {
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub output_tokens: u64,  // Missing cache_creation_tokens!
}

// should_trigger_compaction() (lines 139-161) - DEAD CODE
pub fn should_trigger_compaction(...) -> bool { ... }
```

Production code uses `CompactionHook::on_completion_call()` in `compaction_hook.rs:161-199` instead.

### Impact
- Confusing for maintainers
- `TokenUsage.total()` is wrong (missing `cache_creation_tokens`)
- Risk of someone using dead code and introducing bugs

---

## Issue 2: Code Duplication

### Location
- `codelet/cli/src/interactive/stream_loop.rs`

### Problem
Nearly identical token tracking logic duplicated across 4 code paths:

| Path | Token State | Cumulative Output | Lines |
|------|-------------|-------------------|-------|
| Main loop | `token_state` | `turn_cumulative_output` | 388-784 |
| Gemini continuation | `continuation_token_state` | `continuation_cumulative_output` | 914-1173 |
| Nested continuation | `nested_token_state` | (reuses continuation) | 1092-1118 |
| Retry after compaction | `retry_token_state` | `retry_cumulative_output` | 1368-1621 |

### Evidence
Main loop update (lines 1660-1683):
```rust
session.token_tracker.input_tokens = turn_usage.total_input();
session.token_tracker.output_tokens = turn_cumulative_output;
session.token_tracker.cumulative_billed_input += turn_usage.input_tokens;
session.token_tracker.cumulative_billed_output += turn_usage.output_tokens;
session.token_tracker.cache_read_input_tokens = Some(turn_usage.cache_read_input_tokens);
session.token_tracker.cache_creation_input_tokens = Some(turn_usage.cache_creation_input_tokens);
```

Retry path update (lines 1610-1621) - **identical pattern**:
```rust
session.token_tracker.input_tokens = retry_usage.total_input();
session.token_tracker.output_tokens = retry_cumulative_output;
session.token_tracker.cumulative_billed_input += retry_usage.input_tokens;
session.token_tracker.cumulative_billed_output += retry_usage.output_tokens;
session.token_tracker.cache_read_input_tokens = Some(retry_usage.cache_read_input_tokens);
session.token_tracker.cache_creation_input_tokens = Some(retry_usage.cache_creation_input_tokens);
```

### Impact
- Bug fixes must be applied to all 4 locations
- Easy to miss one location when making changes
- Increases maintenance burden

---

## Issue 3: Missing Architecture Documentation

### Problem
Token tracking architecture is undocumented and confusing:

1. `session.token_tracker.input_tokens` stores `total_input()` (includes cache) - not obvious
2. Cache fields stored separately despite being included in `input_tokens` - redundant but intentional
3. Multiple token state objects with unclear ownership relationships

### What Needs Documentation

```
Token Tracking Architecture:

1. session.token_tracker (Persistent - stored in session)
   - input_tokens: TOTAL context (includes cache) - for display/thresholds
   - output_tokens: Cumulative session output
   - cache_read/creation: Latest API values (display only)

2. TokenState (Per-request - in CompactionHook)
   - Updated by on_stream_completion_response_finish
   - Checked by on_completion_call
   - NOT used for display

3. turn_usage / ApiTokenUsage (Per-turn - temporary)
   - Accumulates across Usage events in multi-turn tools
   - Final value saved to session.token_tracker
```

---

## Issue 4: Known Limitation - Gemini Continuation Compaction

### Location
- `stream_loop.rs` line 1143-1146

### Problem
Compaction during Gemini continuation is explicitly not supported:

```rust
error!(
    "Compaction triggered during Gemini continuation - not yet supported"
);
```

### Impact
- If context fills during Gemini continuation, operation fails
- Error message but no recovery path

---

## Recommended Actions

### High Priority
1. **Remove dead code**: Delete `TokenUsage` struct and `should_trigger_compaction()` from `compaction_threshold.rs`
2. **Add architecture documentation**: Document token tracking in a README or code comments

### Medium Priority  
3. **Refactor to reduce duplication**: Extract common token update logic into helper function
4. **Add integration tests**: Test provider-specific flows (Anthropic, OpenAI, Gemini)

### Low Priority
5. **Consider Gemini continuation compaction support**: Add recovery path when compaction needed during continuation
