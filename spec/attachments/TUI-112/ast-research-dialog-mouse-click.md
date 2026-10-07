# TUI-112 — AST research: dialog mouse-click activation

Feature: spec/features/left-click-activates-buttons-rows-in-all-enter-selectable-modal-dialogs-new-agent-exit-session-etc.feature
Card: TUI-112 (researched 2026-10-07).

## 1. Event flow (where mouse events reach dialogs)

`App::handle_event` (rust/fspec-tui/src/app/events.rs:50) dispatches in stages:

1. DisconnectDialog (Critical, stage 1)
2. **Compositor** (stage 2) — `compositor.handle_event`
   (rust/fspec-tui/src/compositor.rs:132) walks layers from highest
   priority + newest registration down, short-circuits on `Consumed`.
   Every dialog `Component` receives the SAME `Event` (key AND mouse),
   so a dialog-level mouse branch needs no routing changes.
3. Navigator (BoardView / AgentView / Mux) — stage 3
4. App-level shortcuts — stage 4

**Consequence:** mouse events already flow to every topmost dialog.
Work needed is (a) per-dialog rect capture + hit-test, (b) click routing
to the right control, (c) emitting the SAME action/outcome as the
Enter key handler.

## 2. AstGrep survey — `fn handle_event` in components/

```
role_dialog.rs:133            handle_event (Key + Paste; NO mouse branch)
board_exit_confirmation_dialog.rs:157  (Key; mouse = wheel-only)
help_dialog.rs:138            (Key; mouse = scroll)
notification_dialog.rs:239    (no activatable controls)
create_session_dialog.rs:165  (Key; mouse = wheel-only)
menu_bar_help.rs:191          (Key; mouse = scroll)
thinking_level_dialog.rs:122  (Key; mouse = wheel-only)
work_unit_search_dialog.rs:138 (Key; mouse in sibling
  work_unit_search_dialog_mouse.rs — wheel + scrollbar drag ONLY)
status_dialog.rs:210          (no activatable controls)
disconnect_dialog.rs:88       (no activatable controls)
exit_confirmation_dialog.rs:175 (Key; mouse = wheel-only)
mux_config_dialog.rs:222      (Key; mouse = wheel-only)
attachment_picker_dialog.rs:109 (Key ONLY — no mouse branch at all)
error_dialog.rs:63            (no activatable controls)
```

## 3. views/agent shared-button-row dialogs

- `views/agent/confirm_dialog.rs` — `ConfirmDialog::handle_key`
  (line 140) returns `ConfirmDialogOutcome` (Primary/Secondary/Cancel/
  Continued/Ignored). NO `Component` impl, NO mouse handling; the
  caller (resume_session_view.rs) routes keys only.
- `views/agent/merge_confirm_dialog.rs` — `MergeConfirmDialog::handle_key`
  (line 115) → `MergeConfirmDialogOutcome` (Merge/Discard/Cancel/…).
  `Component` impl (line 227) has id/priority/render but the default
  `handle_event` (components/mod.rs:1433) ignores everything.
- `views/agent/session_worktrees_dialog.rs` — `SessionWorktreesDialog`
  with `handle_key` + `Component::handle_event` (line 247) that
  re-wraps `handle_key`.

All three paint the button row via `build_button_row` with
`FOOTER_SEPARATOR = " │ "` between buttons and a focused-button
inverse highlight (accent bg, black fg, bold).

## 4. Three-button dialog button-row geometry (R1 hit-test input)

`create_session_dialog.rs` / `exit_confirmation_dialog.rs` /
`board_exit_confirmation_dialog.rs` build the button row identically:

- spans: `" "` + for each option `" <label> "` (selected = bg Blue /
  fg White / bold; unselected fg Gray) + `"  "` gaps between options
  + trailing `" "`, then LEFT-PADDED with `pad = (body_w - raw_w)/2`
  spaces inserted at span index 0.
- `body_w = max(title, description, raw_w, FOOTER, MIN_WIDTH)`.
- Row is painted by `dialog_theme::render_dialog_at` at
  `y = body.y + 2` (spacious layout, rows after title+gap) where
  `body = inner(rect) inset 1`.

⇒ Per-button rect derivable from: `dialog_rect(area, &dialog)` (the
exact rect `render_dialog` computes), the padding, and the
per-option label widths. No buffer scanning needed.

## 5. Existing click precedents (patterns to mirror)

- `views/checkpoints/keys.rs:171` (RPC-369): `if let
  MouseEventKind::Down(_) = ev.kind { return self.handle_click(col, row); }`
  — click-to-select row hit-test.
- `views/agent/slash_command_popup_mouse.rs:59` — popup click handling.
- `components/work_unit_search_dialog_mouse.rs:71` — `handle_mouse`
  gated on `last_dialog_rect` + `mouse::hit_test::rect_contains`.
- `crate::mouse::rect_contains` (mouse/hit_test.rs:29) — half-open
  containment helper used by every click consumer.

## 6. Rect capture

No in-scope dialog stores its last-rendered rect today.
`dialog_theme::dialog_rect(area, &dialog)` (dialog_theme.rs:120) is
the pure function that computes the exact centered rect — capture
`Some(rect)` during `render()` into a new `last_rect: Option<Rect>`
field; mouse handler gates on `rect_contains`.

## 7. 300-LoC constraint

`components/dialog_theme.rs` is 296 lines (ceiling). Shared
button-row geometry math (button rects from the span layout) should
live in a NEW sibling module (e.g. `components/dialog_button_hits.rs`)
or be inlined per-dialog (each file has headroom; the three-button
dialogs are 200–300 lines — check per file). Preference: one shared
helper module since all 6 button-row dialogs share the exact layout.

## 8. IMPLEMENTATION PLAN (decided 2026-10-07)

**Design: capture control rects at render time, hit-test on click.**
Each in-scope dialog gains a `last_controls: Vec<Rect>` field (buttons
for button-row dialogs, rows for row-list dialogs), set during `render()`
from the SAME span data that paints the row. `handle_event`'s new mouse
branch: on `Down(Left)`, `hit_button(&self.last_controls, col, row)` →
commit that control (identical to Enter on it); no hit → `Ignored`.
Because the rects are derived from the painted spans at the same moment,
hit-test == paint by construction, and the acceptance tests (which click
the computed coords) validate the equivalence end-to-end.

**Shared helper** `components/dialog_button_hits.rs`:
- `content_row_rect(area, dialog, row_index) -> Option<Rect>` — the painted
  origin of the i-th content row, mirroring `render_dialog_at`'s
  spacious/compact/footer + query-row math exactly (reuses `dialog_rect`).
- `spaced_button_rects(start_x, row_y, labels, gap) -> Vec<Rect>` — per-button
  rects for a row painted as ` <label> ` buttons separated by `gap` cols
  (three-button: start_x = body.x + pad + 1 leading space, gap = 2;
  separator dialogs: start_x = body.x, gap = FOOTER_SEPARATOR.width()=3).
- `hit_button(rects, col, row) -> Option<usize>` — `mouse::rect_contains`.

**Per-dialog wiring** (button-row → `last_controls` = button rects;
row-list → `last_controls` = row rects):
- R1 CreateSessionDialog / ExitConfirmationDialog / BoardExitConfirmation:
  button row index = 1 (rows = [description, button_row]); extract a
  `commit(option) -> EventResult` shared by the Enter arm and the click.
- R2 ConfirmDialog / MergeConfirmDialog / SessionWorktreesDialog:
  button row index = 2 (rows = [body, spacer, button_row]); a
  `handle_mouse(ev) -> Option<Outcome>` returns the same outcome variant
  as `handle_key(Enter on that button)`.
- R3 ThinkingLevel (rows 0..4) / AttachmentPicker (rows 0..n) /
  MuxConfig (rows 0..row_count) / Role (single draft row idx 0) /
  WorkUnitSearch (visible match rows, fixed-frame geometry): a click on
  row i commits the Enter action for row i (selects + emits).

**Routing notes (verified):**
- Compositor stage 2 (app/events.rs:80) delivers the SAME `Event`
  (key AND mouse) to the topmost active layer → all `Component`
  dialogs (Create/Exit/BoardExit/Thinking/Mux/Role/Attachment/
  WorkUnitSearch + Merge/Worktrees which DO override `handle_event`)
  get mouse events with NO routing change.
- MergeConfirmDialog's `Component` impl overrides only `render`
  (default `handle_event` is Ignored) but the worktrees sibling
  overrides `handle_event`; add `handle_event` mouse branches to both.
- ConfirmDialog (ResumeSessionView.delete_confirm) is NOT a compositor
  layer — it lives inside the full-screen ResumeSessionView mode view
  (stage 3, routed via views/agent/mouse_dispatch.rs
  `handle_mode_view_mouse` → `resume_view.handle_mouse`). Wire its click
  through that path (ConfirmDialog needs `last_controls` + a
  `handle_mouse(ev) -> Option<ConfirmDialogOutcome>`).

**Test plan** (`tests/tui112_dialog_click.rs`): component-level —
construct each dialog, `render` into a TestBackend (80x24) so
`last_controls` is populated, then feed a `Event::Mouse`
`Down(Left)` at a button/row coordinate and assert the emitted
Action/outcome + `Ignored` for off-control clicks; plus the R4/R5 edge
cases (outside rect, gap, title/footer) and wheel-unchanged.
