# Module 4: `recovery_image.rs`

**Path**: `codelet/cli/src/interactive/recovery_image.rs`  
**Estimated lines**: ~120  
**Responsibility**: EXT-016 — Image content sanitization from conversation history after API dimension/size errors.

---

## Items to Extract

### Functions:

| Function | Current Lines | Signature |
|----------|--------------|-----------|
| `sanitize_image_content` | 125–221 | `pub fn sanitize_image_content(messages: &mut [Message]) -> bool` |

Note: `is_image_content_error` goes in `error_classifiers.rs` (classifier, not recovery logic).

---

## Dependencies

```rust
use rig::message::{Message, UserContent, ToolResultContent};
use rig::one_or_many::OneOrMany;
```

Depends on rig message types for walking and mutating conversation history.

---

## Notes

This is the largest single pure function (96 lines) being extracted. It walks messages in reverse, identifies `UserContent::Image` and `ToolResultContent::Image` within tool results, and replaces them with text placeholders. The function preserves `call_id` on tool results (OpenAI provider path).

The logic is self-contained — it takes `&mut [Message]` and returns whether any replacement occurred. No coupling to the stream loop state.

---

## Re-exports in `mod.rs`

```rust
pub use recovery_image::sanitize_image_content;
```

---

## Test Coverage

- `image_content_recovery_test.rs` (456 lines) — imports `sanitize_image_content`
