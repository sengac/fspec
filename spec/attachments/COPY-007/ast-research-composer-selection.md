# AST Research — COPY-007 Wire selection + copy into the AgentView input composer

All paths under `codelet/fspec-tui/`.

## Composer structure
- `MultiLineInput` struct — multiline_input.rs:54. Fields include `scroll_top: usize` (59). `handle_event_gated` (265) currently drops mouse via `_ => InputEventOutcome::Ignored` (274) — the seam to change.
- Wrap geometry: `multiline_wrap::wrap_lines(lines: &[String], wrap_width: u16) -> Vec<VisualRow>` (multiline_wrap.rs:70). Windowed by `scroll_top`.
- Layout constants: `INPUT_PAD_X: u16 = 1`, `PROMPT_WIDTH: u16 = 2` (multiline_input_render.rs:26,28). `input_body_width(area_width) = area_width - (2*INPUT_PAD_X + PROMPT_WIDTH)` (36). Body x offset = area.x + INPUT_PAD_X + PROMPT_WIDTH.
- Render: multiline_input_render.rs::render (124) paints prompt at PROMPT_WIDTH then body at body_x = area.x + PROMPT_WIDTH (143). visible_rows() skips scroll_top (104).
- `is_edit_keystroke(code, mods) -> bool` — multiline_input_enter.rs:61 (used to detect editing keystrokes for clear trigger).

## Reusable primitives (all DONE)
- crate::mouse::selection::{Cell, Selection, RowSpan}; Selection::spans(width).
- crate::mouse::gesture::{SelectionRecognizer, SelectionGesture}; on_mouse/tick -> Vec<SelectionGesture>.
- crate::mouse::clipboard::Osc52Clipboard.
- paint_selection_highlight (scrollback_highlight.rs, pub(in crate::views::agent)) — reusable to paint REVERSED cells over the composer body rows.

## Design (testable, decoupled) 
- Add `selection: Option<Selection>` + `recognizer: SelectionRecognizer` fields to MultiLineInput.
- `pub fn handle_mouse(&mut self, ev: MouseEvent, area: Rect) -> Option<String>`: convert (ev.column,ev.row) → composer visual (row,col) by subtracting area.x + INPUT_PAD_X + PROMPT_WIDTH (col) and area.y (row), + scroll_top for the logical wrapped-row index; feed recognizer.on_mouse; on Begin set selection anchor/cursor, Extend move cursor, Commit reconstruct prompt-free text from wrap_lines(value, input_body_width) windowed by scroll_top and return Some(text); else None. Return None for wheel/non-left.
- Text reconstruction reuses COPY-004's approach adapted to wrap_lines rows, excluding prompt/pad columns (they were already subtracted, so spans are body-relative).
- Clear triggers: in handle_key_gated / is_edit_keystroke path, set_value, reset → selection = None; scroll_top change → selection = None.
- Highlight: multiline_input_render.rs::render paints REVERSED cells (paint_selection_highlight) for visible selection rows at body_x, before/around hardware cursor paint.

## Wiring into AgentView + copy
- AgentView routes Event::Mouse over the input rect to input.handle_mouse (in views/agent/dispatch.rs handle_event, or a new mouse arm). On Some(text) → copy via the App-held Osc52Clipboard. Since clipboard is on App (COPY-006), AgentView emits a new Action carrying the text, e.g. `Action::CopyToClipboard(String)`, reduced in app/dispatch.rs → self.clipboard.copy(&text). (Reuse this generic action; simpler than composer-specific Selection* actions.)
- Esc precedence: in views/agent/dispatch.rs, BEFORE the composer's own Esc/submit handling, if the composer has an active selection → clear it + consume (no submit).

## Testing (per feature doc)
- unit MultiLineInput: seed a multi-line value, render once (sets scroll_top/geometry), call handle_mouse Down+Drag+Up, get Some(text); feed text into an Osc52Clipboard<Vec<u8>> and assert prompt-free bytes. Also: Begin/Extend produce a live selection; quick click (Down+Up) returns None + no selection.
- clear: active selection + edit keystroke → selection None + char inserted; + Esc → selection None, not submitted; + scroll_top change → None.
- render: buffer shows REVERSED cells over body rows, NOT over the `> ` prompt columns.

## Ceilings
multiline_input.rs is 277 lines — adding fields+handle_mouse will exceed 300. Put handle_mouse + selection methods in a NEW sibling `multiline_input_select.rs` wired via `#[path] mod` from multiline_input.rs (follow the multiline_input_enter.rs / _paste.rs / _render.rs `#[path] mod` convention). Keep every file <300 (rpc026 uses strict `< 300`).
