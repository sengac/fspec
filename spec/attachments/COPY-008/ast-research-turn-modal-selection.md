# AST Research — COPY-008 Wire selection + copy into the turn-content modal

All paths under `codelet/fspec-tui/`.

## Wiring points
- `handle_turn_modal_mouse` — mouse_dispatch.rs:79. Gated by `self.turn_modal_seq?` (81); currently only ScrollUp/ScrollDown → Action::TurnModalScrollUp/Down (84,88). Feed recognizer BEFORE the wheel branch when a left Down/Drag/Up arrives.
- Modal state on AgentView: `turn_modal_seq: Option<u64>` (agent.rs:108), `turn_modal_offset: usize` (agent.rs:112). Add `turn_modal_selection: Option<Selection>` here too.
- Geometry: `turn_modal_geometry(area, body) -> TurnModalGeometry` (dialog_theme_rows.rs:47), `fixed_dialog_rect(area)` (193). The modal is a fixed centered rect; body inner origin = rect + border + padding. Body width from TurnModalGeometry.
- Rows: `TurnContentModal::styled_rows(width)` (turn_modal.rs:155) via `diff_decode::style_modal_lines` — windowed by `turn_modal_offset`. Same windowing shared by highlight + copy.
- Render: `render_turn_modal` / `TurnContentModal::render` (turn_modal.rs:104+). Paint REVERSED cells (paint_selection_highlight, reusable) for visible selection rows AFTER body build, never over the scrollbar column.
- Scroll reducers: app/dispatch_scroll.rs handle_turn_modal scroll/page/jump (~106-147) — clear selection there (rule [6]).
- Esc: dispatch_select.rs:72-74 — when turn_modal_seq.is_some(), sets None + emits CloseTurnModal. Insert a FIRST-Esc level BEFORE this: if turn_modal_selection is Some → clear it + consume; second Esc then closes the modal.
- Wheel clears selection: in handle_turn_modal_mouse, a wheel event clears any active selection then scrolls (rule [6]).

## Reusable primitives (DONE): selection/gesture/clipboard + paint_selection_highlight. App holds Osc52Clipboard (COPY-006); reuse Action::CopyToClipboard(String) (added in COPY-007) to route the copy through the App clipboard.

## Design
- AgentView.turn_modal_selection: Option<Selection> + reuse the AgentView.recognizer (COPY-006, already on AgentView).
- handle_turn_modal_mouse: convert mouse (col,row) → modal body (row,col) by subtracting body inner origin; feed recognizer; Begin/Extend update turn_modal_selection; Commit → reconstruct gutter-free text from styled_rows windowed by turn_modal_offset (reuse COPY-004-style char-slice + content-width clamp against modal body width) → emit Action::CopyToClipboard(text); keep selection. Wheel → clear selection + emit TurnModalScroll. Quick click → None.
- Render highlight in turn_modal.rs.
- Clears: scroll reducers + first-Esc.

## Ceilings
turn_modal.rs is 285 lines, mouse_dispatch.rs 208. Put new selection/reconstruction logic in a NEW sibling module (e.g. turn_modal_select.rs) wired via `mod`/`#[path]` from agent.rs or turn_modal.rs. Keep every file < 300 (strict).

## Tests (per feature doc)
Integration test tests/turn_content_modal_copy008.rs (or unit at the AgentView/TurnContentModal level): open modal (seed a turn), Down+Drag+Up over body → injected Vec<u8> Osc52 receives gutter-free text; non-zero turn_modal_offset maps correctly; wide-line abutting scrollbar excludes glyph; wheel clears selection + scrolls; first Esc clears selection (modal stays open), second Esc closes (turn_modal_seq None); quick click → nothing. 6 scenarios.
