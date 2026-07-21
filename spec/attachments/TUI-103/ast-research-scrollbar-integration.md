# AST Research: ScrollbarDrag Integration Points for TUI-103

## Research Date
2026-07-21

## Scope
Analyzed `codelet/fspec-tui/src/views/agent/` to identify all views that need ScrollbarDrag integration.

## Findings

### Views Already Integrated (TUI-101/TUI-102)
- `ResumeSessionView` (resume_session_view.rs): Has `scrollbar_drag: ScrollbarDrag` field, `last_scrollbar_rect: Option<Rect>`, full `handle_mouse` wiring
- `AgentView` scrollback (mouse_dispatch.rs): Has `scrollback_scrollbar_drag: ScrollbarDrag`, full `handle_scrollback_mouse` wiring

### Views Missing Integration (TUI-103 Targets)

#### 1. SlashCommandPopup (slash_command_popup.rs)
- Struct fields: `filter`, `matches`, `selected_index`, `scroll_offset`, `last_visible_rows`, `wheel`
- `handle_mouse`: Only handles `ScrollUp`/`ScrollDown` wheel events
- Missing: `scrollbar_drag`, `last_scrollbar_rect`, scrollbar hit-testing in `handle_mouse`

#### 2. FileSearchPopup (file_search_popup.rs)
- Struct fields: `filter`, `anchor_offset`, `matches`, `selected_index`, `scroll_offset`, `last_visible_rows`, `wheel`
- `handle_mouse`: Only handles `ScrollUp`/`ScrollDown` wheel events
- Missing: `scrollbar_drag`, `last_scrollbar_rect`, scrollbar hit-testing in `handle_mouse`

#### 3. SearchHistoryView (search_history_view.rs)
- Struct fields: `query`, `matches`, `selected_index`, `scroll_offset`, `wheel`
- `handle_mouse`: Only handles `ScrollUp`/`ScrollDown` wheel events
- Missing: `scrollbar_drag`, `last_scrollbar_rect`, scrollbar hit-testing in `handle_mouse`

#### 4. TurnContentModal (turn_modal.rs + mouse_dispatch.rs)
- Modal has no `handle_mouse` method — mouse handling is in `mouse_dispatch.rs`
- `handle_turn_modal_mouse` routes left-button events to text selection (`feed_turn_modal_selection`)
- Missing: Scrollbar gutter hit-testing, `ScrollbarDrag` state in AgentView, `TurnModalJumpToOffset` action

### Integration Points
- `AgentView::render_with_store` calls `p.render(area, buf)` for popups — needs `&mut` reference for scrollbar rect caching
- `mouse_dispatch.rs::handle_turn_modal_mouse` needs scrollbar gutter hit-testing before text selection
- `Action` enum needs `TurnModalJumpToOffset(usize)` variant
- `dispatch_scroll.rs` needs handler for `TurnModalJumpToOffset`

### Pattern to Follow
ResumeSessionView pattern:
1. Add `scrollbar_drag: ScrollbarDrag` and `last_scrollbar_rect: Option<Rect>` fields
2. Initialize in constructor
3. Reset drag state in `set_matches`/`set_query`/`set_filter`
4. In `handle_mouse`: hit-test left-button events against scrollbar rect, route through `ScrollbarDrag`
5. In `render`: cache scrollbar rect geometry
