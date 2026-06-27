# RPC-368 — AST research for click-to-select in ChangedFilesView

Tool: AstGrep (language: rust)
Target file: codelet/fspec-tui/src/views/changed_files/mod.rs

## Functions located (the click handler will reuse these)

| Pattern | Location | Role |
|---|---|---|
| `fn handle_mouse(&mut self, ev: MouseEvent) -> ChangedFilesEvent { $$$BODY }` | mod.rs:190 | Mouse entry point. Currently only handles ScrollUp/ScrollDown; `_ => Ignored` drops `Down(_)`. A `MouseEventKind::Down(_)` arm must be added BEFORE the wheel match. |
| `fn pane_at(&self, col: u16, row: u16) -> Option<Pane> { $$$BODY }` | mod.rs:211 | Hit-tests cursor against `last_diff_rect` then `last_files_rect`. Returns `Some(Pane::Files/Diff)` or `None`. Reused by the click handler. |
| `fn move_selection(&mut self, delta: i32) -> ChangedFilesEvent { $$$BODY }` | mod.rs:238 | Clamps selection, resets diff_scroll, ensure_visible, emits `Emit(Action::LoadFileDiff)` or `Consumed` when index unchanged. Reused so the already-selected no-op comes for free. |

## Findings
- `last_files_rect` is the CONTENT rect (header + underline already excluded by `pane_header`), so `clicked_index = file_scroll + (row - rect.y)`.
- No App/Navigator change needed: `Event::Mouse` already flows into `handle_mouse` and `Emit` is relayed by navigator_events.rs.
- Existing wheel unit tests in tests.rs construct the view, render to a `TestBackend` (caching rects), then dispatch a synthetic `MouseEvent`. The new click tests mirror that.
