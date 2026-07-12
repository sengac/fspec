# AST Research: Scrollbar pattern for TUI-097

## render_pane_scrollbar (codelet/fspec-tui/src/views/diff_common/mod.rs:28-47)
```rust
pub fn render_pane_scrollbar(
    content: Rect,
    buf: &mut Buffer,
    list_width: u16,
    scroll: usize,
    visible: usize,
    total: usize,
) {
    crate::components::list_scrollbar::render_list_scrollbar(
        Rect {
            x: content.x + list_width,
            y: content.y,
            width: 1,
            height: content.height,
        },
        buf,
        scroll,
        visible,
        total,
    );
}
```

## render_list_scrollbar (codelet/fspec-tui/src/components/list_scrollbar.rs:23-50)
```rust
pub fn render_list_scrollbar(
    area: Rect,
    buf: &mut Buffer,
    scroll_offset: usize,
    visible: usize,
    total: usize,
) {
    let h = area.height as usize;
    if h == 0 || total == 0 { return; }
    let thumb_h = ((visible * h) / total).max(1);
    let thumb_pos = (scroll_offset * h) / total;
    // Glyphs: ■ (thumb) / │ (track), both Modifier::DIM
}
```

## CheckpointsView pattern (codelet/fspec-tui/src/views/checkpoints/render.rs:125-157)
```rust
fn render_checkpoints_pane(...) -> Rect {
    let content = pane_header(area, buf, "Checkpoints", focused == Pane::Checkpoints);
    let visible = content.height as usize;
    let overflow = checkpoints.len() > visible;
    let list_width = if overflow {
        content.width.saturating_sub(1)
    } else {
        content.width
    };
    // ... render list at list_width ...
    if overflow {
        render_pane_scrollbar(content, buf, list_width, scroll, visible, checkpoints.len());
    }
    content
}
```

## Current render_session_rows (codelet/fspec-tui/src/views/agent/mode_view_render.rs:85-132)
```rust
pub(super) fn render_session_rows(
    area: Rect,
    buf: &mut Buffer,
    sessions: &[SessionInfo],
    selected_index: usize,
    scroll_offset: usize,
) {
    // Uses full area.width - no scrollbar gutter reserved
    // No overflow check
}
```

## Integration point
render_session_rows needs to:
1. Check `sessions.len() > visible_rows` for overflow
2. Reserve 1 column: `list_width = area.width.saturating_sub(1)` when overflow
3. Render list at `list_width`
4. Call `render_pane_scrollbar(area, buf, list_width, scroll_offset, visible_rows, sessions.len())` when overflow
