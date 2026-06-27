# RPC-366 AST Research — Checkpoint delete actions

Reuses the RPC-365 restore dialog infrastructure. AstGrep findings over
`codelet/fspec-tui/src/views/checkpoints/`:

## Existing dialog sub-state (template for DeleteDialog)
- `dialog.rs:50` — `impl RestoreDialog { ... }` with `confirm(target)`,
  `title()`, `body_lines()`. The new `DeleteDialog` (delete_dialog.rs)
  mirrors this shape with `DeletePhase::{ConfirmSingle, ConfirmAll{input},
  Deleting, Error}`.

## Existing key/transition handlers (template for delete.rs)
- `restore.rs:21` — `pub(super) fn open_restore_single(&mut self) -> CheckpointsEvent`
- `restore.rs` — `open_restore_all`, `handle_dialog_key`, `confirm_restore`,
  `on_restore_result` → mirrored by `open_delete_single`, `open_delete_all`,
  `handle_delete_dialog_key`, `confirm_delete_single`,
  `confirm_delete_all_if_ready`, `on_delete_result`.

## Wiring points found
- `keys.rs` — `handle_key` dispatches `r/R`, `t/T`; delete adds `d/D`, `a/A`
  and routes to `handle_delete_dialog_key` while the delete dialog is active.
- `mod.rs` — `restore_dialog: Option<RestoreDialog>` field + `dialog()`
  accessor; delete adds `delete_dialog: Option<DeleteDialog>` + `delete_dialog()`.
- `app/dispatch_checkpoints.rs:201-226` — restore Action arms;
  delete adds `DeleteCheckpoint`, `DeleteAllCheckpoints`,
  `DeleteCheckpointResult` arms (spawns in new `dispatch_checkpoint_delete.rs`).
- `transport/mod.rs:167,173` — `delete_checkpoint`, `delete_all_checkpoints`
  already exist (RPC-362). Delegate; do NOT reimplement.
- `tests/checkpoint_restore_dispatch_rpc365.rs` + `tests/common/mod.rs`
  MockBackend — template for the delete dispatch integration test (records
  `delete_checkpoint` / `delete_all_checkpoints`).
