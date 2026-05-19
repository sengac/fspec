# RPC-028 — AST Research: Scroll/Mouse/Wrap Parity Audit

This document captures concrete AST-grep evidence supporting the audit in `scroll-parity-audit.md`. Each finding is a verbatim grep result paired with the architectural rule it violates.

## 1. `move_up` definitions across the codebase

Pattern: `fn move_up($$$ARGS) { $$$BODY }` (Rust)

```
codelet/fspec-tui/src/components/model_selector_dialog.rs:99:5      fn move_up(&mut self)
codelet/fspec-tui/src/components/thinking_level_dialog.rs:81:5      fn move_up(&mut self)
codelet/fspec-tui/src/views/agent/slash_command_popup.rs:85:5       fn move_up(&mut self)
codelet/fspec-tui/src/views/agent/resume_session_view.rs:114:5      fn move_up(&mut self, visible_rows: usize)
codelet/fspec-tui/src/views/agent/search_history_view.rs:120:5      fn move_up(&mut self, visible_rows: usize)
codelet/fspec-tui/src/views/agent/file_search_popup.rs:109:5        fn move_up(&mut self)
```

**Finding:** six bespoke `move_up` implementations duplicate wrap-around logic. Per architecture rule [5]/[6] in the audit, these MUST be replaced with calls to `scroll_viewport::wrap_index(...)` + `scroll_viewport::ensure_visible(...)`.

## 2. Hard `.take(10)` cap (the reported defect)

Pattern: `$ITER.take(10)`

```
codelet/fspec-tui/src/views/agent/slash_command_popup.rs:167:25  self.matches.iter().take(10)
codelet/fspec-tui/src/views/agent/file_search_popup.rs:186:26    self.matches.iter().take(10)
```

**Finding:** SlashCommandPopup and FileSearchPopup hard-clip to the first 10 matches. Selection can advance past index 9 via wrap-around, but the row is invisible — exactly the user-reported defect. Architecture rule [0] mandates `iter().skip(scroll_offset).take(visible_rows)`.

## 3. Mouse-wheel handling

Pattern: `MouseEventKind::ScrollUp`

```
codelet/fspec-tui/src/views/board/mouse.rs:55:9  MouseEventKind::ScrollUp
```

**Finding:** BoardView is the ONLY view in the entire crate that responds to wheel events. Every other selectable view (Help, Disconnect, ThinkingLevel, ModelSelector, Confirm, SlashCommandPopup, FileSearchPopup, ResumeSessionView, SearchHistoryView) ignores `Event::Mouse`. Architecture rule [2] mandates `handle_mouse(MouseEvent, Rect, vr)` on every selectable view.

## 4. Reference patterns to mirror

BoardView precedents documented in `scroll-parity-audit.md` §7 (citations):

- `store/board_viewport.rs:42-44` — `proposed.rem_euclid(len_i)` (wrap-around primitive).
- `store/board_viewport.rs:118-174` — `adjust_scroll_offset` (ensure-visible primitive; popups need the simpler variant without the two-pass arrow correction).
- `views/board.rs:58-95` — `last_viewport_height: Cell<u16>` (geometry caching idiom).
- `views/board/mouse.rs:40-74` — wheel-event dispatch + hit-test + delegation to `Action::SelectPrev`/`SelectNext`.

## 5. Anti-patterns to delete

- Six duplicated `move_up`/`move_down` blocks with bespoke wrap arithmetic.
- Two `iter().take(10)` calls hiding everything past row 9.
- Zero mouse handlers in dialog/popup/picker views.

## 6. Search tools used

- `AstGrep` Rust patterns: `fn move_up($$$ARGS) { $$$BODY }`, `$ITER.take(10)`, `MouseEventKind::ScrollUp`.
- `Grep` cross-checks were performed by the DeepSearch sub-agent during the audit phase.
- File contents read in full: `slash_command_popup.rs`, `file_search_popup.rs` (build_rows + handlers), `resume_session_view.rs`, `dialog_theme.rs`.
