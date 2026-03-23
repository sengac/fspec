# Module 2: `recovery_truncation.rs`

**Path**: `codelet/cli/src/interactive/recovery_truncation.rs`  
**Estimated lines**: ~80  
**Responsibility**: PROV-040 — Truncated tool call detection, recovery message building, and retry budget.

---

## Items to Extract

### Constants:

| Constant | Current Line | Value |
|----------|-------------|-------|
| `MAX_TRUNCATION_RETRIES` | 226 | `2` |

### Functions:

| Function | Current Lines | Signature |
|----------|--------------|-----------|
| `build_truncation_recovery_message` | 245–272 | `pub fn build_truncation_recovery_message(error_str: &str) -> String` |
| `build_truncation_budget_exhausted_message` | 277–285 | `pub fn build_truncation_budget_exhausted_message(max_retries: u32) -> String` |

Note: `is_truncated_tool_call_error` goes in `error_classifiers.rs` (classifier, not recovery logic).

---

## Dependencies

```rust
// None — pure string formatting functions
```

---

## Re-exports in `mod.rs`

```rust
pub use recovery_truncation::{
    MAX_TRUNCATION_RETRIES,
    build_truncation_recovery_message,
    build_truncation_budget_exhausted_message,
};
```

---

## Test Coverage

- `truncation_recovery_test.rs` — imports `build_truncation_recovery_message`, `build_truncation_budget_exhausted_message`, `MAX_TRUNCATION_RETRIES`
