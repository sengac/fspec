# TUI-097 — Resume View Proportional Scrollbar

## Problem

The `/resume` view renders session rows using the full body area width with no scrollbar. When there are more sessions than fit on screen, the user has no visual indication of their position in the list.

## Current State

`mode_view_render.rs:render_session_rows` renders session rows across the full `body_area` width. No scrollbar gutter is reserved.

## Design

### Pattern to Follow

The CheckpointsView (`views/checkpoints/render.rs:125-157`) demonstrates the correct pattern:

```rust
let overflow = checkpoints.len() > visible;
let list_width = if overflow {
    content.width.saturating_sub(1)  // Reserve 1 col for scrollbar
} else {
    content.width
};
// ... render list at list_width ...
if overflow {
    render_pane_scrollbar(content, buf, list_width, scroll, visible, total);
}
```

### Implementation

Modify `render_session_rows` to:

1. Compute `overflow = sessions.len() > visible_rows`
2. If overflow, reserve 1 column: `list_width = area.width.saturating_sub(1)`
3. Render session rows at `list_width`
4. If overflow, call `render_pane_scrollbar(area, buf, list_width, scroll_offset, visible_rows, sessions.len())`

### Shared Component

Use `crate::views::diff_common::render_pane_scrollbar` which:
- Creates a 1-col gutter at the right edge
- Delegates to `crate::components::list_scrollbar::render_list_scrollbar`
- Draws proportional `■` thumb over `│` track using DIM style

### Files to Modify

1. **`codelet/fspec-tui/src/views/agent/mode_view_render.rs`** — Update `render_session_rows` to reserve scrollbar gutter and call `render_pane_scrollbar` when overflow

### Testing

- Verify scrollbar appears when sessions exceed visible rows
- Verify scrollbar is hidden when sessions fit on screen
- Verify thumb position is proportional to scroll offset
- Verify list width is reduced by 1 when scrollbar is shown
