# AST Research: TUI-102 AgentView Scrollbar Integration

## Key Files for Integration

| File | Lines | Purpose |
|------|-------|---------|
| `src/views/agent.rs` | 300 | AgentView struct — needs `scrollbar_drag` field |
| `src/views/agent/mouse_dispatch.rs` | 222 | Mouse routing — needs scrollbar column hit-test |
| `src/views/agent/scrollback.rs` | 299 | ScrollbackList — needs `scrollbar_geometry()` method |
| `src/components/mod.rs` | 1227 | Action enum — needs `ScrollbackJumpToOffset` variant |
| `src/app/dispatch.rs` | 300+ | Dispatch handler — needs offset jump handler |
| `src/mouse/scrollbar_drag.rs` | 132 | State machine — already implemented, ready to wire |

## Scrollbar Column Position

- Scrollbar is painted at `area.x + area.width - 1` (rightmost column)
- Gutter is reserved when `total_rows > vh && area.width >= 4`
- Content width becomes `area.width - 2` when gutter is reserved
- The scrollbar column is the single rightmost column of the scrollback area

## Integration Pattern

```
handle_scrollback_mouse(ev)
  → hit-test: is ev.column == scrollbar_col?
    → YES: feed to scrollbar_drag.on_mouse(ev, geom)
      → Some(offset): emit Action::ScrollbackJumpToOffset(offset)
    → NO: feed to selection_recognizer (existing text selection)
```

## AgentView Fields to Add

```rust
pub(crate) scrollbar_drag: crate::mouse::ScrollbarDrag,
pub(crate) last_scrollback_total_rows: usize,
```

## Action Enum Addition

```rust
/// TUI-102: emitted by scrollbar click/drag on scrollback.
ScrollbackJumpToOffset(usize),
```

## Dispatch Handler

```rust
Action::ScrollbackJumpToOffset(offset) => {
    if let Some(ctx) = self.agent_view_store.current_session_context_mut() {
        ctx.scrollback.scroll_state.offset = *offset;
        ctx.scrollback.scroll_state.stick_to_bottom = false;
    }
}
```

## Source-Shape Constraint

Every file must be < 300 lines. `dispatch.rs` is near the limit — new handler should be minimal or extracted.
