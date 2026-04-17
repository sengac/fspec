# AST Research: parse_tool_result_content text-branch bounding

## Goal

Find the precise injection point for `bound_tool_result_text` inside
`codelet/patches/rig-core/src/agent/prompt_request/streaming.rs` and
confirm that adding a text-branch byte bound does not disturb the
existing image/PDF rejection paths.

## Queries run

### 1. Locate `parse_tool_result_content`

```
pattern: fn parse_tool_result_content($$$ARGS) -> $RET { $$$BODY }
lang:    rust
path:    codelet/patches/rig-core/src/agent/prompt_request/streaming.rs
```

Hit:

```
codelet/patches/rig-core/src/agent/prompt_request/streaming.rs:33:
  fn parse_tool_result_content(result: &str) -> Vec<ToolResultContent>
```

### 2. Locate sibling helper `check_image_dimensions`

```
pattern: fn check_image_dimensions($$$ARGS) -> $RET { $$$BODY }
lang:    rust
path:    codelet/patches/rig-core/src/agent/prompt_request/streaming.rs
```

Hit:

```
codelet/patches/rig-core/src/agent/prompt_request/streaming.rs:30:
  fn check_image_dimensions(base64_data: &str) -> Option<String>
```

This is the pattern to mirror for our new helper: a small private fn
declared directly above `parse_tool_result_content`, not exported,
easy to audit and to carry across upstream rig rebases.

### 3. Enumerate every call site producing `ToolResultContent::text`

```
pattern: ToolResultContent::text($ARG)
lang:    rust
path:    codelet/patches/rig-core/src/agent/prompt_request/streaming.rs
```

Hits (in call order inside `parse_tool_result_content`):

| Line | Call                               | Context                                                              |
|-----:|------------------------------------|----------------------------------------------------------------------|
|  43  | `ToolResultContent::text(content_str)` | Early-return when `content` field is a string but not valid JSON. Plain text branch — **must be bounded**. |
|  53  | `ToolResultContent::text(inner_str)`   | Double-serialization fallback when inner JSON parse fails. Plain text branch — **must be bounded**. |
|  73  | `ToolResultContent::text(error_msg)`   | Oversized PDF-page rejection message. **Leave alone.** |
|  99  | `ToolResultContent::text(error_msg)`   | Oversized top-level image rejection message. **Leave alone.** |
| 129  | `ToolResultContent::text(error_msg)`   | Oversized nested PDF-page rejection message. **Leave alone.** |
| 144  | `ToolResultContent::text(content)`     | `{"type":"text","content":"..."}` fall-through (Read tool text form). Plain text branch — **must be bounded**. |
| 149  | `ToolResultContent::text(result)`      | Final default fall-through for raw text tool output. Plain text branch — **must be bounded**. |

Call site at `vec_to_one_or_many` (line 155, `ToolResultContent::text("")`)
is an empty-vec fallback and does not need bounding.

## Decision

Introduce:

```rust
const MAX_TOOL_RESULT_TEXT_BYTES: usize = 64 * 1024;
fn bound_tool_result_text(original: String) -> String { … }
```

Call `bound_tool_result_text` at every plain-text fall-through inside
`parse_tool_result_content` (lines 43, 53, 144, 149 in the pre-change
file). Do NOT call it at the three oversized-image/PDF error-message
sites (73, 99, 129): those are already short, self-describing error
strings whose byte length is bounded by construction.

This keeps the bound centralized in one helper, invoked only from the
text branch, so the image/PDF rejection behavior is untouched.

## UTF-8 safety

`MAX_TOOL_RESULT_TEXT_BYTES = 65536`, preview = first 2048 bytes,
suffix = last 512 bytes. Byte slicing would corrupt multi-byte UTF-8
characters, so the helper must walk `str::char_indices()` to find the
largest valid byte boundary `≤ preview_bytes` and the smallest valid
byte boundary `≥ len - suffix_bytes`. `str::floor_char_boundary` is
nightly-only, hence the manual walk.

## Marker shape

```json
{
  "status": "truncated",
  "original_bytes": <usize>,
  "max_bytes": <usize>,
  "preview": "<first ≤ 2048 bytes, UTF-8-safe>",
  "suffix":  "<last  ≤ 512 bytes,  UTF-8-safe>",
  "hint":    "tool output exceeded MAX_TOOL_RESULT_TEXT_BYTES; re-run with a narrower query or --summary/--failures-only flags"
}
```

Constructed via `serde_json::json!` so preview/suffix strings with
quotes, backslashes, or control characters are properly escaped.

## Test strategy

Six `#[test]` fns inside `#[cfg(test)] mod tests` — one per feature
scenario. The helper is pure (`String -> String`), so tests call it
directly. Scenarios that observe `parse_tool_result_content` end-to-end
(e.g. the 500 KiB verbose-tool cascade) invoke
`parse_tool_result_content` and inspect the resulting
`Vec<ToolResultContent>`.
