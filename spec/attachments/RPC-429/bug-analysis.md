# RPC-429 — MultiLineInput Up/Down Arrow Keys Skip Wrapped Visual Rows

## Problem Statement

When a user types a very long string in the MultiLineInput widget that wraps across multiple visual rows, the Up/Down arrow keys do not navigate between the visual rows. Instead, they always return `InputEventOutcome::Ignored`, which triggers scrollback navigation in the AgentView dispatch layer.

## Root Cause

The boundary check in `handle_key_gated()` (lines 237-245 of `multiline_input.rs`) uses **logical line count** (`self.line_count()`) and **logical cursor row** (`self.cursor()`) to determine whether the cursor is at the top or bottom boundary:

```rust
if matches!(code, KeyCode::Up | KeyCode::Down) {
    let (row, _col) = self.cursor();
    let line_count = self.line_count();
    let at_top = code == KeyCode::Up && row == 0;
    let at_bottom = code == KeyCode::Down && row + 1 >= line_count;
    if at_top || at_bottom {
        return InputEventOutcome::Ignored;
    }
}
```

For a single logical line that wraps into 5 visual rows:
- `line_count()` returns **1**
- `cursor()` returns `(0, col)` — always row 0
- `at_top` is `true` for Up, `at_bottom` is `true` for Down
- Both arrows return `Ignored`, triggering `Action::ScrollbackLineUp`/`ScrollbackLineDown`

## Fix Approach

Replace the logical-line boundary check with a **visual-row boundary check** using the wrap-aware geometry functions already available in `multiline_wrap.rs`:

1. **Cache the last known `body_width`** — The `MultiLineInput` struct needs to track the body width from the last render call (via `sync_viewport` or `render`) so the boundary check can compute visual rows.

2. **Use `total_visual_rows()`** instead of `line_count()` — Returns the total number of visual rows at the cached body width.

3. **Use `cursor_visual_position()`** instead of `cursor()` — Maps the logical cursor position to the visual row index.

4. **Only return `Ignored`** when the visual cursor is at the true visual top/bottom boundary.

## Files Affected

- `codelet/fspec-tui/src/views/agent/multiline_input.rs` — Add `last_body_width` field, replace boundary check
- `codelet/fspec-tui/src/views/agent/multiline_input_render.rs` — Update `sync_viewport`/`render` to cache body width
- `codelet/fspec-tui/src/views/agent/multiline_wrap.rs` — Already has the functions needed

## Integration Points

- **AgentView dispatch** (`dispatch.rs`): The `Ignored` outcome is mapped to `Action::ScrollbackLineUp`/`ScrollbackLineDown`. This mapping is correct — it should only fire when at the visual boundary.
- **Render path**: `sync_viewport()` already receives `body_width` and `height`. The cached body width enables the boundary check.
- **Tests**: Existing tests in `multiline_input.rs` and `multiline_wrap.rs` need updating for the new field and behavior.
