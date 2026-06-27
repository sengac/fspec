# RPC-365 AST Research — Restore wiring reuse points

Research performed with AstGrep over the existing fspec-tui crate to confirm
the integration points the restore feature must reuse rather than reinvent.

## Existing modal dialog pattern (model for the confirmation/status modal)
- `components/disconnect_dialog.rs` — `Component` modal that delegates rendering
  to `dialog_theme::render_dialog` (rounded accent border, body rows, footer).
- `components/status_dialog.rs` — `StatusKind::{Restoring, Complete, Error}`
  state machine, the closest analogue to the restore status dialog. Confirms
  the restoring → complete | error phasing and the cyan/red accent convention.
- `components/dialog_theme.rs` — `FspecDialog { accent, title, rows, footer,
  min_width }` + `render_dialog(area, buf, &dialog)`. RPC-079 source-shape test
  requires the `FspecDialog {` literal to live under `components/`, so the new
  modal renderer was placed at `components/checkpoint_restore_dialog.rs`.

## Transport methods reused (RPC-362, no git logic re-implemented)
- `transport/mod.rs:150` `async fn restore_checkpoint_file(work_unit_id, name, path) -> Result<()>`
- `transport/mod.rs:161` `async fn restore_checkpoint_all(work_unit_id, name) -> Result<()>`
- `transport/mod.rs:99`  `async fn checkpoint_counts() -> Result<CheckpointCounts>` (post-restore refresh)
These delegate (embedded.rs / websocket.rs) to `FspecService` →
`codelet_git::ghost_commit::restore_ghost_commit_file` / `restore_ghost_commit`.

## Dispatch flow reused (RPC-356 / RPC-364)
- `app/dispatch_checkpoints.rs::try_dispatch_checkpoints` — catch-all chain entry
  from `app/dispatch.rs` (`|| self.try_dispatch_checkpoints(&action)`).
- Lazy `tokio::spawn` + `action_tx.send(...)` fold-back pattern (3 existing
  stages: list → files → diff). The restore actions add two spawns + a
  result-folding handler, split into `app/dispatch_checkpoint_restore.rs` to
  keep both files < 300 lines.

## View key handling reused (RPC-364)
- `views/checkpoints/mod.rs` three-pane `Pane::{Checkpoints, Files, Diff}` focus
  state machine + `handle_key`. The restore keys (`r`/`R`, `t`/`T`) and the
  modal-capture branch were added to `views/checkpoints/keys.rs` (split out for
  the 300-line ceiling). `selected_checkpoint_info()` / `selected_file_path()` /
  file count drive the confirmation copy.
