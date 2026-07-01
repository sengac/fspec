# AST Research — RPC-396 Scrollable, space-filling help dialog

## Target component
`codelet/fspec-tui/src/components/help_dialog.rs` — `HelpDialog`.

### Current state (verbatim)
- Struct is `{ id: String }` (help_dialog.rs:35-37). No scroll state.
- `handle_event` (help_dialog.rs:63-74): only `KeyCode::Esc` handled → removes dialog by id. No arrow/page/wheel handling.
- `render` (help_dialog.rs:76-93): maps `HELP_LINES` to `DialogRow`s, calls `render_dialog` (shrink-to-content, centered). No slicing, no scrollbar.
- `Component::render` signature is `&mut self` (mod.rs:1115) — so measured `visible_rows` can be stored on `self` directly (no `Cell` needed).
- `Component::handle_event` receives the whole `Event` (mod.rs:1104) — so `Event::Mouse(m)` wheel arms can be matched here.

## Reusable APIs (all already `pub`)
### scroll math — `src/components/scroll_viewport.rs`
- `ensure_visible(&mut scroll_offset, selected, visible_rows, total)` (lines 46-66) — clamps window.
- `WheelVelocity` + `WheelDirection` (lines 68-128) — `.step(dir) -> i32` acceleration ramp.

### scrollbar — `src/components/list_scrollbar.rs`
- `render_list_scrollbar(area, buf, scroll_offset, visible, total)` (lines 23-51) — `■` thumb over `│` track, DIM; paints nothing when `total == 0`.

## Reference implementations to mirror
- **SlashCommandPopup** `src/views/agent/slash_command_popup.rs`:
  - key handling Up/Down/PageUp/PageDown/Home/End at lines 188-233.
  - mouse wheel `handle_mouse` matching `MouseEventKind::ScrollUp/ScrollDown` at 163-184.
- **ThinkingLevelDialog** `src/components/thinking_level_dialog.rs:167-179` — a Component that
  matches `Event::Mouse(m)` wheel directly in `handle_event` without hit-testing (dialog is
  topmost/centered). This is the exact pattern for HelpDialog wheel handling.
- **ModelSelector** `src/views/model_selector/rows_render.rs:92-129` — reserves a 1-col gutter
  and calls `render_list_scrollbar` only on overflow (`total > visible_rows`).

## Sizing
- `dialog_theme::dialog_rect` (dialog_theme.rs:107-125) = shrink-to-content, centered, clamped.
- `dialog_theme::render_dialog_at(rect, buf, dialog)` (lines 144-299) paints at an EXPLICIT rect —
  already used by the full-screen TurnContentModal. RPC-396 computes a space-filling rect
  (area minus a small margin) and calls `render_dialog_at` instead of `render_dialog`.

## Call sites (unchanged by RPC-396 but relevant)
- `?` push: `src/app/events.rs:127` → `HelpDialog::new()`.
- `/help` push: `src/app/dispatch_slash_commands.rs:33` → `HelpDialog::new()`.
- Compositor dispatches events to topmost Critical layer first (`app/events.rs:64-76`,
  `compositor.rs:132-161`), so HelpDialog's `handle_event` receives keys/mouse while open.

## Existing tests to update
- `help_dialog.rs` inline tests + insta snapshot `help_dialog__centered_popup_80x24`
  (`src/components/snapshots/...`). Size change → snapshot regenerates.
