# AST Research — RPC-374 Wire A key → attachment picker

## Target surfaces in codelet/fspec-tui (confirmed)

- `src/views/board.rs::handle_event` — add `Char('a') | Char('A')` arm beside the
  RPC-373 `d`/`D` arm (now ~L184-188). Always `EventResult::consumed()`; emit
  `Action::OpenAttachmentPicker` only when `store.selected_work_unit()` has a
  non-empty `attachments`. `store.selected_work_unit() -> Option<&WorkUnitInfo>`;
  `WorkUnitInfo.attachments: Vec<String>` (codelet/rpc-types/src/lib.rs:50).

- `src/components/mod.rs` `enum Action` — add `OpenAttachmentPicker` and
  `OpenAttachment(String)`.

## Dialog component pattern (model the picker on these)

- `src/components/create_session_dialog.rs` — Priority::Foreground modal: a
  `Component` with `id`, `selected`, `action_tx`, `render()` via
  `dialog_theme::render_dialog`, key handling, emits Actions, pops itself.
  `CREATE_SESSION_DIALOG_ID` const id; `.with_action_tx(tx)`.
- `src/components/checkpoint_restore_dialog.rs` — a LIST-style dialog (closest to
  a vertical attachment list with Up/Down + Enter + Esc).
- `src/app/dispatch_create_session_dialog.rs::handle_open_create_session_dialog`
  shows the push pattern: guard `self.compositor.contains(ID)`, build the dialog
  with `.with_action_tx(self.action_tx.clone())`, `self.compositor.push(Box::new(dialog))`.
- `dialog_theme::{render_dialog, Accent, DialogRow, FspecDialog}` — shared chrome.

## Reuse from RPC-373 (already in tree)

- `src/app/dispatch_viewer.rs` — already has `try_dispatch_viewer`,
  `foundation_url`, `foundation_target`, `viewer_port`, `set_viewer_port_for_test`,
  and the `open::that` spawn pattern behind the `Some` branch. Extend it:
  - `attachment_url(port, path) -> String` (percent-encode path; spaces→%20).
  - `App::attachment_target(&self, path) -> Option<String>` = viewer_port.map(...).
  - `OpenAttachmentPicker` → build + push `AttachmentPickerDialog`.
  - `OpenAttachment(path)` → `if let Some(url) = self.attachment_target(&path) { spawn open::that(url) }`.

## Percent-encoding

`urlencoding` is now a workspace dep (added in RPC-372). Use
`urlencoding::encode` per path segment OR encode the whole relative path while
preserving `/` separators (TS `encodeURI` keeps `/`). Simplest parity: split on
`/`, `urlencoding::encode` each segment, rejoin with `/`. Verify
`a b.md` → `a%20b.md`.

## Board key-test harness

`tests/view_board_unit_rpc012.rs` (`wu()` builds WorkUnitInfo with attachments;
set it to a non-empty Vec for the picker test). `tests/board_open_foundation_rpc373.rs`
shows the App-level seam test (`set_viewer_port_for_test`, MockBackend via
`mod common`). The picker-rows scenario unit-tests `AttachmentPickerDialog`
row/items accessor directly.

## Testability seam (no real browser in tests)

Mirror RPC-373: pure `attachment_url` + `App::attachment_target` are unit-tested;
`open::that` only inside the `Some` branch of `OpenAttachment` dispatch.
