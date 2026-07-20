# RPC-431 — Unicode Display Width Fix: Implementation Guide

## Problem

The Rust ratatui TUI uses `.chars().count()` for all text width measurement (wrapping, centering, column sizing, truncation). This treats every Unicode scalar as 1 terminal column, causing misalignment for wide characters (emojis, CJK, etc.).

## Root Cause

`.chars().count()` returns the number of Unicode scalar values, not the number of terminal columns. Wide characters (CJK, emoji, etc.) occupy 2 terminal columns but count as 1 char.

## Solution

Replace `.chars().count()` with `unicode_width::UnicodeWidthStr::width()` for all **display-width** measurements. The `unicode-width` crate is already in the workspace.

## Files to Modify

### Display Width Measurements (MUST change to `.width()`)

| File | Function | Purpose | Change |
|------|----------|---------|--------|
| `store/agent_view/markdown_table_render.rs` | `pad_text()` | Column padding | `.chars().count()` → `.width()` |
| `store/agent_view/markdown_table_render.rs` | `push_table_block()` | Column width calc | `.chars().count()` → `.width()` |
| `views/agent/text_wrap.rs` | `wrap_to_width()` | Text wrapping | `.chars().count()` → `.width()` |
| `views/agent/text_wrap.rs` | `wrap_paragraph()` | Word wrapping | `.chars().count()` → `.width()` |
| `components/dialog_theme.rs` | `span_width()` | Dialog sizing | `.chars().count()` → `.width()` |
| `components/dialog_theme.rs` | `line_width()` | Dialog sizing | `.chars().count()` → `.width()` |
| `components/exit_confirmation_dialog.rs` | `render()` | Button centering | `.chars().count()` → `.width()` |
| `components/board_exit_confirmation_dialog.rs` | `render()` | Button centering | `.chars().count()` → `.width()` |
| `components/create_session_dialog.rs` | `render()` | Button centering | `.chars().count()` → `.width()` |
| `components/hello.rs` | `render()` | Text centering | `.chars().count()` → `.width()` |
| `views/board/details_strip.rs` | `render_placeholder()` | Centering | `.chars().count()` → `.width()` |
| `views/board/details_strip.rs` | `truncate_to()` | Truncation | `.chars().count()` → `.width()` |
| `views/board/details_strip.rs` | `build_attachments_line()` | Column sizing | `.chars().count()` → `.width()` |
| `views/board/footer.rs` | `render()` | Footer centering | `.chars().count()` → `.width()` |
| `views/board/columns.rs` | `pad_to_width()` | Column padding | `.chars().count()` → `.width()` |
| `views/board/viewport.rs` | `center_glyph()` | Glyph centering | `.chars().count()` → `.width()` |
| `views/diff_common/row.rs` | `truncate_path()` | Path truncation | `.chars().count()` → `.width()` |
| `views/agent/header.rs` | `truncate_line()` | Line truncation | `.chars().count()` → `.width()` |
| `views/agent/chrome.rs` | `line_width()` | Line width | `.chars().count()` → `.width()` |
| `store/agent_view/diff_decode.rs` | `changed_bar_row()` | Gutter width | `.chars().count()` → `.width()` |
| `store/agent_view/diff_decode.rs` | `changed_lines()` | Gutter width | `.chars().count()` → `.width()` |
| `store/agent_view/diff_decode.rs` | `context_lines()` | Gutter width | `.chars().count()` → `.width()` |
| `store/agent_view/diff_decode.rs` | `pad_to_width()` | Padding | `.chars().count()` → `.width()` |
| `store/agent_view/chunk_wrap.rs` | `wrap_header()` | Header wrap | `.chars().count()` → `.width()` |
| `store/agent_view/tool_args.rs` | `cap_value()` | Value truncation | `.chars().count()` → `.width()` |

### Intentionally UNCHANGED (character count is correct)

| File | Function | Why unchanged |
|------|----------|---------------|
| `views/board/details_select.rs` | `slice_chars()` | Clipboard is char-based |
| `views/agent/scrollback_copy.rs` | `slice_chars()` | Clipboard is char-based |
| `views/agent/turn_modal_select.rs` | `slice_chars()` | Clipboard is char-based |
| `views/agent/multiline_input_select.rs` | `slice_chars()` | Clipboard is char-based |
| `views/agent/input_transition.rs` | `advance()` | Animation is char-based |
| `views/agent/input_transition.rs` | `transition_on_idle()` | Animation is char-based |
| `views/provider_settings/copy.rs` | `mask_secret()` | Masking is char-based |

## Implementation Pattern

```rust
use unicode_width::UnicodeWidthStr;

// Before:
let width = text.chars().count();

// After:
let width = text.width();
```

For spans:
```rust
// Before:
let width = span.content.chars().count();

// After:
let width = span.content.width();
```

## Test Strategy

1. Write unit tests for `wrap_to_width`, `pad_text`, `truncate_to`, `inner_content_width` with wide Unicode input
2. Verify existing tests still pass (ASCII input should behave identically)
3. Add snapshot tests for table rendering with emoji content

## Cargo.toml

The `unicode-width` crate is already in the workspace. Add the import to each file that needs it:

```rust
use unicode_width::UnicodeWidthStr;
```

## Estimation

- **3 points**: Moderate complexity. Many files to touch but the change is mechanical and uniform. The main risk is missing a usage or breaking existing tests.
