# AST Research — COPY-006 Wire selection + copy into AgentView scrollback end-to-end

All paths under `codelet/fspec-tui/`.

## Primitives (all done, reuse unchanged)
- `crate::mouse::clipboard::Osc52Clipboard<W: Write + Send>` (clipboard.rs:30) — `with_stdout()` (36), `copy(&mut self, &str) -> io::Result<()>` (48).
- `crate::mouse::selection::{Cell, Selection, RowSpan}` (selection.rs). `Selection::spans(row_width) -> Vec<RowSpan>` (45).
- `crate::mouse::gesture::{SelectionRecognizer, SelectionGesture}` (gesture.rs). `on_mouse(ev, now) -> Vec<SelectionGesture>` (68), `tick(now) -> Vec<SelectionGesture>` (106). Gestures: Begin(Cell)/Extend(Cell)/Commit/Cancel.
- `ScrollbackList::selected_text(&self, &[RowSpan]) -> String` (scrollback_copy.rs:31); `set_selection_highlight_spans(&mut self, Vec<RowSpan>)` pub(crate) (scrollback_copy.rs:75); painted at scrollback.rs:247.

## Wiring points (from DeepSearch)
1. `AgentView` struct — views/agent.rs:92. Fields: `action_tx` (94), `last_scrollback_area: Option<Rect>` (113, set in render_with_store:280), `scrollback_wheel` (114), `turn_select_mode` (105). `emit(&self, Action)` at agent.rs:205 (fire-and-forget).
2. Mouse routing — views/agent/mouse_dispatch.rs `handle_scrollback_mouse` (99): hit-tests `last_scrollback_area`, currently only ScrollUp/ScrollDown → Action::ScrollbackMouseWheel{Up,Down}. Down/Drag/Up must feed the recognizer BEFORE the wheel branch. Convert mouse (column,row) → scrollback (row,col) by subtracting rect.x/rect.y.
3. Focused ScrollbackList access in reducer — app/dispatch_scroll.rs `scroll_focused` (10): `self.agent_view_store.current_session_context_mut()?.scrollback`. New Selection* reducers go here as helpers.
4. Reducer match arm — app/dispatch.rs near lines 248-249 (ScrollbackMouseWheelUp/Down). Add Action::SelectionBegin/Extend/Commit/Clear arms delegating to dispatch_scroll helpers.
5. content_width — computed locally in ScrollbackList::render_count_visited (scrollback.rs:222-230: `reserve_gutter = total_visual_rows() > vh && area.width>=4; content_width = if reserve_gutter { area.width-2 } else { area.width }`). NOT stored. For commit, cache it on ScrollbackList during render (new field) or recompute from last_rect()+total_visual_rows(). PREFER caching a `content_width: u16` field set in render_count_visited.
6. Action enum — components/mod.rs:109. Scrollback variants near 309-317. Add SelectionBegin(Cell)/SelectionExtend(Cell)/SelectionCommit/SelectionClear. Cell must be importable there (crate::mouse::selection::Cell).
7. Tick arm — app/events.rs:238 (`_ = tick.tick() => {...}`, 16ms RENDER_TICK). Poll recognizer.tick(Instant::now()) here for long-press Begin.
8. Hold Osc52Clipboard + SelectionRecognizer + live Selection on `App` (app/state.rs:33, init in with_action_bus 107-127) — commit reducer runs on App with &mut self + store access + stdout. MouseTrackingToggle (toggle.rs:44) is the analogous stdout helper but is NOT instantiated anywhere — no view-side precedent; put clipboard on App.
9. Esc cascade — views/agent/dispatch.rs: insert SelectionClear-first level between popup routing (after line 121) and the Tab/SELECT/Esc block (126-141). Guard on a mirrored `selection_active` flag on AgentView. Order becomes: selection-clear → turn-modal/SELECT exit → AgentEscPressed.
10. Selection clears on scroll (wheel/line/page — the scroll_focused helper or the reducers), input submit path, and Esc.

## Testing strategy (per feature doc)
- unit reducer: SelectionCommit with seeded ScrollbackList + Selection → expected clipboard bytes via INJECTED Vec<u8> Osc52Clipboard (make the App/reducer hold `Osc52Clipboard<W>` generic OR test the copy path at a seam that accepts an injected writer). SelectionClear on scroll/Esc empties highlight spans.
- dispatch: Down+Drag+Up through handle_scrollback_mouse → Begin/Extend/Commit emitted; wheel still yields ScrollbackMouseWheel + no selection.
- render: buffer shows REVERSED cells for an active selection (already covered structurally by COPY-005; here assert end-to-end an active selection paints).
- mouse capture: assert no DisableMouseCapture issued in the flow.

## Ceilings
dispatch.rs is 236 lines; mouse_dispatch.rs 124; app/state.rs, events.rs, dispatch_scroll.rs — keep all < 300. Put new reducer helpers in dispatch_scroll.rs (or a new sibling) and recognizer-holding fields on App/AgentView minimally.
