# AST Research: Compaction Trigger Implementation

## Overview
This document captures AST analysis of the existing codebase to understand the current implementation and plan the CTX-002 changes.

## Files Analyzed

### 1. codelet/core/src/compaction_hook.rs

**Key Functions:**

| Function | Line | Purpose |
|----------|------|---------|
| `calculate_effective_tokens` | 53 | **BROKEN** - Current: `input - (cache * 0.9)`. Must change to: `input + cache_read + output` |
| `check_compaction` | 61 | Compares effective tokens against threshold. Must update to use new calculation |
| `on_completion_call` | 85 | Hook called before completion. Uses calculate_effective_tokens |
| `on_stream_completion_response_finish` | 114 | Updates token state from API response. **NOTE**: Currently doesn't capture output_tokens |

**Current Algorithm (BROKEN):**
```rust
fn calculate_effective_tokens(&self, input_tokens: u64, cache_read_tokens: u64) -> u64 {
    let cache_discount = (cache_read_tokens as f64 * 0.9) as u64;
    input_tokens.saturating_sub(cache_discount)
}
```

**Required Changes:**
1. Add `output_tokens` to TokenState struct
2. Change calculation to simple sum: `input + cache_read + output`
3. Update `on_stream_completion_response_finish` to capture output_tokens
4. Remove cache discount logic entirely

### 2. codelet/cli/src/compaction_threshold.rs

**Key Functions:**

| Function | Line | Purpose |
|----------|------|---------|
| `calculate_compaction_threshold` | 85 | **SUBOPTIMAL** - Current: `context * 0.9`. Must change to: `context - min(max_output, 32k)` |
| `calculate_summarization_budget` | 118 | Budget for summary generation. Not affected by CTX-002 |

**Current Algorithm (SUBOPTIMAL):**
```rust
pub fn calculate_compaction_threshold(context_window: u64) -> u64 {
    (context_window as f64 * COMPACTION_THRESHOLD_RATIO) as u64
}
```

**Required Changes:**
1. Add `SESSION_OUTPUT_TOKEN_MAX = 32_000` constant
2. Add new `should_trigger_compaction()` function with optimized algorithm
3. Deprecate `calculate_compaction_threshold()` (keep for backwards compat)
4. Add `calculate_usable_context()` helper

### 3. codelet/providers/src/lib.rs

**Key Functions:** None found (likely defines traits)

**Required Changes:**
1. Add `max_output_tokens() -> u64` to Provider trait
2. Return model-specific values (0 if unknown)

## Existing Tests That Will Need Updates

### compaction_hook.rs Tests:
- `test_effective_tokens_calculation` (line 140) - Tests old cache discount logic
- `test_cache_discount_prevents_compaction` (line 206) - Tests cache discount behavior
- `test_tool_call_context_growth_scenario` (line 274) - Uses old effective token calculation

### compaction_threshold.rs Tests:
- `test_calculate_threshold_claude` (line 137) - Tests old 90% threshold
- `test_calculate_threshold_openai` (line 144) - Tests old 90% threshold

## New Tests Required

1. Token counting: `input + cache_read + output = total`
2. Usable context: `context - min(max_output, SESSION_OUTPUT_MAX)`
3. Zero max_output fallback: Uses SESSION_OUTPUT_MAX (32k)
4. Zero context bypass: Returns false immediately
5. Disable flag bypass: Returns false when flag set
6. Boundary condition: `total == usable` should NOT trigger (uses >)

## Integration Points

1. `check_compaction()` in compaction_hook.rs calls the threshold logic
2. Provider must expose max_output_tokens for usable context calculation
3. Disable flag must be configurable (environment variable or config)
