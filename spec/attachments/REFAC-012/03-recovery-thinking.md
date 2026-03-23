# Module 3: `recovery_thinking.rs`

**Path**: `codelet/cli/src/interactive/recovery_thinking.rs`  
**Estimated lines**: ~120  
**Responsibility**: PROV-041 — Thinking/reasoning token exhaustion detection, recovery strategies, progressive degradation.

---

## Items to Extract

### Constants:

| Constant | Current Line | Value |
|----------|-------------|-------|
| `MAX_THINKING_EXHAUSTION_RETRIES` | 294 | `2` |
| `THINKING_EXHAUSTION_OUTPUT_THRESHOLD` | 300 | `50` |
| `THINKING_EXHAUSTION_CROSS_TURN_THRESHOLD` | 306 | `3` |

### Functions:

| Function | Current Lines | Signature |
|----------|--------------|-----------|
| `is_thinking_exhaustion` | 322–348 | `pub fn is_thinking_exhaustion(stop_reason: Option<&str>, reasoning_tokens: u64, output_tokens: u64, threshold: u64) -> bool` |
| `build_thinking_exhaustion_recovery_message` | 358–384 | `pub fn build_thinking_exhaustion_recovery_message(reasoning_tokens: u64, output_tokens: u64, captured_reasoning: Option<&str>) -> String` |
| `build_thinking_budget_exhausted_message` | 389–395 | `pub fn build_thinking_budget_exhausted_message(max_retries: u32) -> String` |
| `downgrade_thinking_level` | 403–411 | `pub fn downgrade_thinking_level(level: ThinkingLevel) -> ThinkingLevel` |

---

## Dependencies

```rust
use codelet_tools::facade::ThinkingLevel;  // For downgrade_thinking_level
```

Single external dependency. All functions are pure — no I/O, no state mutation.

---

## Re-exports in `mod.rs`

```rust
pub use recovery_thinking::{
    MAX_THINKING_EXHAUSTION_RETRIES,
    THINKING_EXHAUSTION_OUTPUT_THRESHOLD,
    THINKING_EXHAUSTION_CROSS_TURN_THRESHOLD,
    is_thinking_exhaustion,
    build_thinking_exhaustion_recovery_message,
    build_thinking_budget_exhausted_message,
    downgrade_thinking_level,
};
```

---

## Test Coverage

- `thinking_exhaustion_recovery_test.rs` (529 lines) — imports all 4 functions + all 3 constants
