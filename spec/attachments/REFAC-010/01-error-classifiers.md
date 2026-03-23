# Module 1: `error_classifiers.rs`

**Path**: `codelet/cli/src/interactive/error_classifiers.rs`  
**Estimated lines**: ~80  
**Responsibility**: Pure error-string classification functions. Zero side effects, zero state.

---

## Functions to Extract

### From `stream_loop.rs`:

| Function | Current Lines | Signature |
|----------|--------------|-----------|
| `is_prompt_too_long_error` | 78–94 | `pub fn is_prompt_too_long_error(error_str: &str) -> bool` |
| `is_image_content_error` | 102–115 | `pub fn is_image_content_error(error_str: &str) -> bool` |
| `is_truncated_tool_call_error` | 235–237 | `pub fn is_truncated_tool_call_error(error_str: &str) -> bool` |
| `is_compaction_cancelled` | 415–417 | `fn is_compaction_cancelled(error: &anyhow::Error) -> bool` |

---

## Dependencies

```rust
use anyhow;  // Only for is_compaction_cancelled
```

No other crate dependencies. These are pure string-matching functions.

---

## Visibility Changes

- `is_compaction_cancelled` stays `pub(super)` — only used by `stream_loop.rs`
- The other three stay `pub` — used by external tests

---

## Re-exports in `mod.rs`

```rust
pub use error_classifiers::{
    is_prompt_too_long_error,
    is_image_content_error,
    is_truncated_tool_call_error,
};
// is_compaction_cancelled is pub(super), not re-exported
```

---

## Test Coverage

- `prompt_too_long_recovery_test.rs` — imports `is_prompt_too_long_error`
- `image_content_recovery_test.rs` — imports `is_image_content_error`
- `truncation_recovery_test.rs` — imports `is_truncated_tool_call_error`

All tests import via `codelet_cli::interactive::*` — re-exports preserve compatibility.
